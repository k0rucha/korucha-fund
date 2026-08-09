//! Ticker share cards — analogous to fund share cards (`share.rs`) but for
//! a single ticker. The owner picks a symbol + chart span, the server
//! captures a snapshot of the price (and the owner's position, if held),
//! and returns a permanent URL.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use askama::Template;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::db::{prices, fx, symbols, transactions, ticker_share_cards, api_stats};
use crate::error::{AppError, AppResult};
use crate::services::{portfolio, yfinance};
use crate::template_response::TemplateResponse;
use crate::util::{format_with_commas, signed_pct, jst, jst_today};
use crate::handlers::share::{base_url, generate_id, normalize_span_str};

/// Threshold of in-window rows below which we'll attempt a one-shot backfill.
/// Tuned so that a fresh symbol (0 rows) and a "just one latest" symbol (1
/// row) both trigger a backfill, but a previously-fetched ticker with
/// ~weekly+ data does not.
const BACKFILL_ROW_THRESHOLD: i64 = 5;
/// How far back to backfill on first encounter. 35 covers the 30d chart
/// option even with weekends/holidays trimmed.
const BACKFILL_DAYS: i64 = 35;

#[derive(Deserialize)]
pub struct CreateTickerShareQuery {
    pub symbol: String,
    pub span: Option<String>,
}

#[derive(Serialize)]
pub struct CreateTickerShareResponse {
    pub id: String,
    pub url: String,
}

/// `POST /ticker-share?symbol=AAPL&span=30d`
///
/// Capture the current state of `symbol` plus (if held) the owner's position,
/// and persist a snapshot keyed by a fresh ID. Returns the new URL.
pub async fn create_ticker_share_card(
    State(state): State<AppState>,
    Query(q): Query<CreateTickerShareQuery>,
) -> AppResult<impl IntoResponse> {
    let raw_symbol = q.symbol.trim().to_uppercase();
    if raw_symbol.is_empty() {
        return Err(AppError::Other(anyhow::anyhow!("symbol is required")));
    }
    let span = normalize_span_str(q.span.as_deref());

    let today = jst_today();

    // If we have little or no recent history for this symbol, do a single
    // 30-day backfill in one yfinance call so the chart has content. This
    // is opt-in: only fires when below threshold to keep API usage low.
    // Counted against the rate-limit stats (forced — owner-initiated, rare).
    let since = today - chrono::Duration::days(BACKFILL_DAYS);
    let recent = prices::count_history_since(&state.db, &raw_symbol, since)
        .await
        .unwrap_or(0);
    if recent < BACKFILL_ROW_THRESHOLD {
        match yfinance::backfill_price_history(&state.db, &raw_symbol, since).await {
            Ok(n) => {
                tracing::info!(
                    "ticker-share: backfilled {} rows for {} (had {} in window)",
                    n, raw_symbol, recent
                );
                if let Err(e) = api_stats::record_api_request_forced(&state.db).await {
                    tracing::warn!("ticker-share: failed to record api request: {}", e);
                }
            }
            Err(e) => tracing::warn!(
                "ticker-share: backfill failed for {}: {}",
                raw_symbol, e
            ),
        }
    } else {
        tracing::info!(
            "ticker-share: skipping backfill for {} ({} rows already cached in window)",
            raw_symbol, recent
        );
    }

    let issue_price = prices::get_price_on_or_before(&state.db, &raw_symbol, today)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::Other(anyhow::anyhow!(
                "銘柄 {} の価格データがまだありません。管理画面から取引を追加するか、しばらく待ってから再試行してください。",
                raw_symbol
            ))
        })?;

    // Currency: prefer the symbols table (populated when first seen), then
    // fall back to the simple `.T = JPY, else USD` heuristic.
    let symbol_rows = symbols::get_symbol_names(&state.db).await.unwrap_or_default();
    let symbol_meta = symbol_rows.iter().find(|s| s.symbol == raw_symbol);
    let currency = symbol_meta
        .map(|s| s.currency.clone())
        .unwrap_or_else(|| if raw_symbol.ends_with(".T") { "JPY".into() } else { "USD".into() });
    let mut display_name = symbol_meta.and_then(|s| s.name.clone());

    // If name is missing (e.g. symbol not held), try fetching from Yahoo Finance.
    if display_name.is_none() {
        match yfinance::fetch_symbol_name(&raw_symbol).await {
            Ok(Some(name)) => {
                tracing::info!("ticker-share: fetched missing name for {}: {}", raw_symbol, name);
                display_name = Some(name);
            }
            Ok(None) => tracing::info!("ticker-share: name not found for {} on Yahoo", raw_symbol),
            Err(e) => tracing::warn!("ticker-share: failed to fetch name for {}: {}", raw_symbol, e),
        }
    }

    // Upsert symbol meta so we have it for future cards/transactions.
    if let Err(e) = symbols::upsert_symbol(&state.db, &raw_symbol, display_name.as_deref(), &currency).await {
        tracing::warn!("ticker-share: failed to upsert symbol meta for {}: {}", raw_symbol, e);
    }

    // FX at issue (only relevant for USD-denominated tickers).
    let fx_rate_at_issue = if currency == "USD" {
        fx::get_latest_usdjpy(&state.db).await.unwrap_or(None)
    } else {
        None
    };

    // Owner's position, if any. Filter transactions to this symbol and run
    // the same cost-basis math as the dashboard.
    let txs = transactions::list_transactions(&state.db).await.map_err(AppError::Database)?;
    let symbol_txs: Vec<_> = txs.into_iter().filter(|t| t.symbol == raw_symbol).collect();
    let holding = portfolio::calculate_holdings(&symbol_txs).into_iter().next();

    let (quantity, avg_cost, issue_value_jpy, issue_pnl_jpy) = if let Some(h) = holding.as_ref() {
        let fx = if h.currency == "USD" { fx_rate_at_issue.unwrap_or(150.0) } else { 1.0 };
        let value = h.quantity * issue_price * fx;
        let pnl = value - h.total_cost_jpy;
        (Some(h.quantity), Some(h.average_cost_native), Some(value), Some(pnl))
    } else {
        (None, None, None, None)
    };

    // Allocate a unique ID with retries (collision is astronomically unlikely
    // but the DB enforces uniqueness, so we don't want a stray failure to
    // bubble up).
    let mut id = String::new();
    let mut last_err: Option<sqlx::Error> = None;
    for attempt in 0..3 {
        let candidate = generate_id();
        match ticker_share_cards::insert_ticker_share_card(
            &state.db,
            &candidate,
            &raw_symbol,
            display_name.as_deref(),
            &currency,
            issue_price,
            fx_rate_at_issue,
            quantity,
            avg_cost,
            issue_value_jpy,
            issue_pnl_jpy,
            &span,
        )
        .await
        {
            Ok(()) => {
                id = candidate;
                last_err = None;
                break;
            }
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                tracing::warn!("ticker-share: id collision on attempt {}, retrying", attempt);
                last_err = Some(sqlx::Error::Database(db_err));
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                continue;
            }
            Err(e) => return Err(AppError::Database(e)),
        }
    }
    if let Some(e) = last_err {
        return Err(AppError::Database(e));
    }

    let url = format!("/ticker/{}", id);
    Ok(Json(CreateTickerShareResponse { id, url }))
}

#[derive(Template)]
#[template(path = "ticker_share.html")]
pub struct TickerShareTemplate {
    pub id: String,
    pub created_at: String,
    pub days_since: i64,

    pub symbol: String,
    pub display_name: String,
    pub currency: String,

    // ===== Main: at-issue values (the "headline") =====
    pub issue_price_native: String,
    pub issue_price_native_num: f64,
    pub issue_price_jpy: String,            // for USD tickers, ¥ form too

    // Optional position info (empty/zero strings when not held).
    pub has_position: bool,
    pub quantity: String,
    pub avg_cost_native: String,
    pub issue_value_jpy: String,
    pub issue_pnl_jpy: String,
    pub issue_pnl_jpy_num: f64,
    pub issue_pnl_pct: String,
    pub issue_pnl_pct_num: f64,

    // ===== Sub: change since issuance =====
    pub current_price_native: String,
    pub price_delta_native: String,
    pub price_delta_native_num: f64,
    pub price_delta_pct: String,

    // ===== Chart =====
    pub history_dates_json: String,
    pub history_prices_json: String,
    pub default_span: String,

    // ===== OGP =====
    pub og_url: String,
    pub og_image_url: String,
    pub og_title: String,
    pub og_description: String,
}

/// `GET /ticker/:id`
pub async fn view_ticker_share_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let card = ticker_share_cards::get_ticker_share_card(&state.db, &id)
        .await
        .map_err(|e| {
            tracing::error!("ticker-share: get: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // SQLite's CURRENT_TIMESTAMP is UTC; everything user-facing is JST.
    let jst = jst();
    let created_jst = chrono::TimeZone::from_utc_datetime(&jst, &card.created_at);
    let issue_date = created_jst.date_naive();
    let today = jst_today();
    let days_since = (today - issue_date).num_days().max(0);

    // Current price (today's most recent close on or before today).
    let current_price = prices::get_price_on_or_before(&state.db, &card.symbol, today)
        .await
        .unwrap_or(None)
        .unwrap_or(card.issue_price_native);

    let price_delta = current_price - card.issue_price_native;
    let price_delta_pct = if card.issue_price_native != 0.0 {
        (price_delta / card.issue_price_native) * 100.0
    } else {
        0.0
    };

    let issue_pnl_pct_num = match (card.issue_pnl_jpy, card.issue_value_jpy, card.quantity, card.avg_cost_native) {
        (Some(_), Some(_), Some(qty), Some(avg)) if avg > 0.0 && qty > 0.0 => {
            // Native PnL%: (current_price - avg_cost) / avg_cost * 100
            ((card.issue_price_native - avg) / avg) * 100.0
        }
        _ => 0.0,
    };

    // Issue price expressed in JPY (matches the JST display convention).
    let issue_price_jpy_num = if card.currency == "USD" {
        let fx = card.fx_rate_at_issue.unwrap_or(150.0);
        card.issue_price_native * fx
    } else {
        card.issue_price_native
    };

    // Chart data: every cached close for `symbol` up through the issue date.
    // JS slices by span on the client.
    let history = prices::list_history(&state.db, &card.symbol, issue_date)
        .await
        .unwrap_or_default();
    let history_dates: Vec<String> = history.iter().map(|(d, _)| d.to_string()).collect();
    let history_prices: Vec<f64> = history.iter().map(|(_, p)| *p).collect();

    let display_name = card.display_name.clone().unwrap_or_default();
    let display_name_for_title = if display_name.is_empty() {
        card.symbol.clone()
    } else {
        format!("{} ({})", display_name, card.symbol)
    };

    let base = base_url(&headers);
    let og_url = format!("{}/ticker/{}", base, card.id);
    let og_image_url = format!("{}/ticker/{}/ogp.png", base, card.id);
    let unit = if card.currency == "USD" { "$" } else { "¥" };
    let og_title = format!(
        "{} 戦績カード — 価格 {}{}",
        display_name_for_title,
        unit,
        format_with_commas(card.issue_price_native)
    );
    let position_suffix = if card.has_position_marker() {
        let pnl = card.issue_pnl_jpy.unwrap_or(0.0);
        let sign = if pnl >= 0.0 { "+" } else { "-" };
        format!(
            " ・ 保有数 {} ・ 含み損益 {}¥{}",
            format_with_commas(card.quantity.unwrap_or(0.0)),
            sign,
            format_with_commas(pnl.abs()),
        )
    } else {
        String::new()
    };
    let og_description = format!(
        "発行日 {} ・ {} の発行時価格は {}{}{}",
        created_jst.format("%Y-%m-%d"),
        card.symbol,
        unit,
        format_with_commas(card.issue_price_native),
        position_suffix,
    );

    let has_position = card.has_position_marker();
    let tmpl = TickerShareTemplate {
        id: card.id.clone(),
        created_at: created_jst.format("%Y-%m-%d %H:%M").to_string(),
        days_since,

        symbol: card.symbol.clone(),
        display_name,
        currency: card.currency.clone(),

        issue_price_native: format_with_commas(card.issue_price_native),
        issue_price_native_num: card.issue_price_native,
        issue_price_jpy: format_with_commas(issue_price_jpy_num),

        has_position,
        quantity: card.quantity.map(|q| format!("{:.2}", q)).unwrap_or_default(),
        avg_cost_native: card.avg_cost_native.map(|c| format!("{:.2}", c)).unwrap_or_default(),
        issue_value_jpy: card.issue_value_jpy.map(format_with_commas).unwrap_or_default(),
        issue_pnl_jpy: card
            .issue_pnl_jpy
            .map(|p| format_with_commas(p.abs()))
            .unwrap_or_default(),
        issue_pnl_jpy_num: card.issue_pnl_jpy.unwrap_or(0.0),
        issue_pnl_pct: signed_pct(issue_pnl_pct_num),
        issue_pnl_pct_num,

        current_price_native: format_with_commas(current_price),
        price_delta_native: format_with_commas(price_delta.abs()),
        price_delta_native_num: price_delta,
        price_delta_pct: signed_pct(price_delta_pct),

        history_dates_json: serde_json::to_string(&history_dates).unwrap_or_else(|_| "[]".into()),
        history_prices_json: serde_json::to_string(&history_prices).unwrap_or_else(|_| "[]".into()),
        default_span: card.default_span,

        og_url,
        og_image_url,
        og_title,
        og_description,
    };

    Ok(TemplateResponse(tmpl))
}

// Small extension trait to keep the conditional-format logic readable above.
trait HasPosition {
    fn has_position_marker(&self) -> bool;
}
impl HasPosition for ticker_share_cards::TickerShareCard {
    fn has_position_marker(&self) -> bool {
        self.quantity.unwrap_or(0.0) > 0.0
    }
}
