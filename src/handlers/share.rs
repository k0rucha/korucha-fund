use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    http::{StatusCode, HeaderMap},
    Json,
};
use askama::Template;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::AppState;
use crate::db::{transactions, prices, fx, symbols, snapshots, share_cards};
use crate::services::portfolio;
use crate::template_response::TemplateResponse;
use crate::util::{format_with_commas, signed_with_commas, signed_pct};

#[derive(Serialize, Deserialize, Clone)]
pub struct CardHolding {
    pub symbol: String,
    pub name: String,
    pub current_value_jpy: f64,
    pub unrealized_pnl_jpy: f64,
    pub unrealized_pnl_pct: f64,
}

/// Generate a short, time-ordered hex ID for a share card.
/// We mix in the OS thread id to defend against (very unlikely) nanosecond
/// collisions when two share cards are created in the same instant.
/// Shared with the ticker-share handler.
pub(crate) fn generate_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
    let mut hasher = DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    let salt = hasher.finish() & 0xFFFF;
    format!("{:x}{:04x}", nanos, salt)
}

fn normalize_span(s: Option<&String>) -> String {
    normalize_span_str(s.map(|v| v.as_str()))
}

pub(crate) fn normalize_span_str(s: Option<&str>) -> String {
    match s.unwrap_or("all") {
        "7d" => "7d".into(),
        "30d" => "30d".into(),
        _ => "all".into(),
    }
}

pub(crate) fn base_url(headers: &HeaderMap) -> String {
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("fund.korucha.com");
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_else(|| if host.starts_with("localhost") || host.starts_with("127.") { "http" } else { "https" });
    format!("{}://{}", scheme, host)
}

async fn compute_current(
    state: &AppState,
) -> Result<(f64, f64, f64, Vec<CardHolding>), StatusCode> {
    let txs = transactions::list_transactions(&state.db).await.map_err(|e| {
        tracing::error!("share: list_transactions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let holdings = portfolio::calculate_holdings(&txs);

    let latest_prices = prices::get_latest_prices(&state.db).await.unwrap_or_default();
    let price_map: HashMap<String, f64> = latest_prices
        .into_iter()
        .map(|p| (p.symbol, p.close_price))
        .collect();

    // Display-only fallback (see fragments.rs for rationale).
    let usdjpy = fx::get_latest_usdjpy(&state.db)
        .await
        .unwrap_or(None)
        .unwrap_or(150.0);

    let symbol_list = symbols::get_symbol_names(&state.db).await.unwrap_or_default();
    let name_map: HashMap<String, String> = symbol_list
        .into_iter()
        .filter_map(|s| s.name.map(|n| (s.symbol, n)))
        .collect();

    let mut total_value = 0.0;
    let mut total_cost = 0.0;
    let mut card_holdings = Vec::new();

    for h in holdings {
        let current_price = price_map.get(&h.symbol).copied().unwrap_or(0.0);
        let fx_rate = if h.currency == "USD" { usdjpy } else { 1.0 };
        let value_jpy = h.quantity * current_price * fx_rate;
        let pnl = value_jpy - h.total_cost_jpy;
        let pnl_pct = if h.total_cost_jpy > 0.0 {
            (pnl / h.total_cost_jpy) * 100.0
        } else {
            0.0
        };

        total_value += value_jpy;
        total_cost += h.total_cost_jpy;

        let display_name = name_map.get(&h.symbol).cloned().unwrap_or_default();

        card_holdings.push(CardHolding {
            symbol: h.symbol,
            name: display_name,
            current_value_jpy: value_jpy,
            unrealized_pnl_jpy: pnl,
            unrealized_pnl_pct: pnl_pct,
        });
    }

    card_holdings.sort_by(|a, b| {
        b.current_value_jpy
            .partial_cmp(&a.current_value_jpy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let pnl = total_value - total_cost;
    Ok((total_value, total_cost, pnl, card_holdings))
}

#[derive(Deserialize)]
pub struct CreateShareQuery {
    pub span: Option<String>,
}

#[derive(Serialize)]
pub struct CreateShareResponse {
    pub id: String,
    pub url: String,
}

use crate::error::{AppResult, AppError};

pub async fn create_share_card(
    State(state): State<AppState>,
    Query(q): Query<CreateShareQuery>,
) -> AppResult<impl IntoResponse> {
    let span = normalize_span(q.span.as_ref());

    let (total_value, total_cost, pnl, holdings) = compute_current(&state).await.map_err(|s| {
        if s == StatusCode::NOT_FOUND { AppError::NotFound }
        else { AppError::Other(anyhow::anyhow!("portfolio computation failed")) }
    })?;

    let holdings_json = serde_json::to_string(&holdings).map_err(|e| {
        tracing::error!("share: serialize holdings: {}", e);
        AppError::Other(e.into())
    })?;

    // Retry a few times on (extraordinarily unlikely) ID collision.
    let mut id = String::new();
    let mut last_err: Option<sqlx::Error> = None;
    for attempt in 0..3 {
        let candidate = generate_id();
        match share_cards::insert_share_card(
            &state.db,
            &candidate,
            total_value,
            total_cost,
            pnl,
            &holdings_json,
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
                tracing::warn!("share: id collision on attempt {}, retrying", attempt);
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

    let url = format!("/share/{}", id);
    Ok(Json(CreateShareResponse { id, url }))
}

#[derive(Clone)]
pub struct HoldingRow {
    pub symbol: String,
    pub name: String,
    pub at_issue_value: String,
    pub at_issue_pnl: String,
    pub at_issue_pnl_num: f64,
    pub at_issue_pnl_pct: String,
    pub at_issue_pnl_pct_num: f64,
}

#[derive(Template)]
#[template(path = "share.html")]
pub struct ShareTemplate {
    pub id: String,
    pub created_at: String,

    // === Main: at-issue values (the "headline") ===
    pub issue_value: String,
    pub issue_value_num: f64,
    pub issue_cost: String,
    pub issue_pnl: String,
    pub issue_pnl_num: f64,
    pub issue_pnl_pct: String,
    pub issue_pnl_pct_num: f64,

    // === Sub: how things have moved since issuance ===
    pub current_value: String,
    pub current_value_num: f64,
    pub value_delta: String,
    pub value_delta_num: f64,
    pub value_delta_pct: String,
    pub days_since: i64,

    pub holdings: Vec<HoldingRow>,

    // Chart history up to issuance moment, JSON-encoded.
    pub history_dates_json: String,
    pub history_values_json: String,
    pub history_pnls_json: String,
    pub default_span: String,

    // OGP / Twitter-card meta
    pub og_url: String,
    pub og_image_url: String,
    pub og_title: String,
    pub og_description: String,
}

pub async fn view_share_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    let card = share_cards::get_share_card(&state.db, &id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let issue_holdings: Vec<CardHolding> =
        serde_json::from_str(&card.holdings_json).unwrap_or_default();

    let (cur_value, _cur_cost, _cur_pnl, _cur_holdings) = compute_current(&state).await.map_err(|s| {
        if s == StatusCode::NOT_FOUND { AppError::NotFound }
        else { AppError::Other(anyhow::anyhow!("portfolio computation failed")) }
    })?;

    let issue_pnl_pct = if card.total_cost_jpy > 0.0 {
        (card.unrealized_pnl_jpy / card.total_cost_jpy) * 100.0
    } else {
        0.0
    };

    let value_delta = cur_value - card.total_value_jpy;
    let value_delta_pct_num = if card.total_value_jpy > 0.0 {
        (value_delta / card.total_value_jpy) * 100.0
    } else {
        0.0
    };

    // SQLite stores CURRENT_TIMESTAMP in UTC; display everything in JST.
    let jst = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    let created_jst = chrono::TimeZone::from_utc_datetime(&jst, &card.created_at);
    let issue_date = created_jst.date_naive();
    let today_jst = chrono::Utc::now().with_timezone(&jst).date_naive();
    let days_since = (today_jst - issue_date).num_days().max(0);

    // Holdings table — stored at-issue values are the canonical view.
    // Sort by current_value_jpy desc up front (instead of O(n²) sort_by + find).
    let mut sorted = issue_holdings.clone();
    sorted.sort_by(|a, b| {
        b.current_value_jpy
            .partial_cmp(&a.current_value_jpy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Templates render the sign before ¥, so ¥-amounts are sent as absolute strings.
    let rows: Vec<HoldingRow> = sorted
        .iter()
        .map(|h| HoldingRow {
            symbol: h.symbol.clone(),
            name: h.name.clone(),
            at_issue_value: format_with_commas(h.current_value_jpy),
            at_issue_pnl: format_with_commas(h.unrealized_pnl_jpy.abs()),
            at_issue_pnl_num: h.unrealized_pnl_jpy,
            at_issue_pnl_pct: signed_pct(h.unrealized_pnl_pct),
            at_issue_pnl_pct_num: h.unrealized_pnl_pct,
        })
        .collect();

    // Chart history: every snapshot on/before issuance date. JS filters by span.
    let snaps = snapshots::list_snapshots(&state.db).await.unwrap_or_default();
    let filtered: Vec<_> = snaps
        .into_iter()
        .filter(|s| s.date <= issue_date)
        .collect();
    let history_dates: Vec<String> = filtered.iter().map(|s| s.date.to_string()).collect();
    let history_values: Vec<f64> = filtered.iter().map(|s| s.total_value_jpy).collect();
    let history_pnls: Vec<f64> = filtered.iter().map(|s| s.unrealized_pnl_jpy).collect();

    let base = base_url(&headers);
    let og_url = format!("{}/share/{}", base, card.id);
    let og_image_url = format!("{}/share/{}/ogp.png", base, card.id);
    let og_title = format!(
        "こるちゃファンド 戦績カード — 評価額 ¥{}",
        format_with_commas(card.total_value_jpy)
    );
    let og_description = format!(
        "発行日 {} ・ 含み損益 ¥{} ({}) ・ 投資元本 ¥{}",
        created_jst.format("%Y-%m-%d"),
        signed_with_commas(card.unrealized_pnl_jpy),
        signed_pct(issue_pnl_pct),
        format_with_commas(card.total_cost_jpy)
    );

    let tmpl = ShareTemplate {
        id: card.id,
        created_at: created_jst.format("%Y-%m-%d %H:%M").to_string(),

        issue_value: format_with_commas(card.total_value_jpy),
        issue_value_num: card.total_value_jpy,
        issue_cost: format_with_commas(card.total_cost_jpy),
        // ¥-prefixed strings are absolute; templates render the sign.
        issue_pnl: format_with_commas(card.unrealized_pnl_jpy.abs()),
        issue_pnl_num: card.unrealized_pnl_jpy,
        issue_pnl_pct: signed_pct(issue_pnl_pct),
        issue_pnl_pct_num: issue_pnl_pct,

        current_value: format_with_commas(cur_value),
        current_value_num: cur_value,
        value_delta: format_with_commas(value_delta.abs()),
        value_delta_num: value_delta,
        value_delta_pct: signed_pct(value_delta_pct_num),
        days_since,

        holdings: rows,

        history_dates_json: serde_json::to_string(&history_dates).unwrap_or_else(|_| "[]".into()),
        history_values_json: serde_json::to_string(&history_values).unwrap_or_else(|_| "[]".into()),
        history_pnls_json: serde_json::to_string(&history_pnls).unwrap_or_else(|_| "[]".into()),
        default_span: card.default_span,

        og_url,
        og_image_url,
        og_title,
        og_description,
    };

    Ok(TemplateResponse(tmpl))
}
