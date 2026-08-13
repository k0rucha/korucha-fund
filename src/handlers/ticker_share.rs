//! Ticker share cards — analogous to fund share cards (`share.rs`) but for
//! a single ticker. The owner picks a symbol + chart span, the server
//! captures a snapshot of the price (and the owner's position, if held),
//! and returns a permanent URL.

use askama::Template;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::db::{prices, ticker_share_cards};
use crate::handlers::AppState;
use crate::handlers::error::{AppError, AppResult};
use crate::handlers::share::base_url;
use crate::handlers::template_response::TemplateResponse;
use crate::services::portfolio::DISPLAY_USD_JPY_FALLBACK;
use crate::services::share_cards as share_service;
use crate::util::{format_with_commas, jst, jst_today, signed_pct};

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
    let id = share_service::create_ticker_card(&state.db, &q.symbol, q.span.as_deref())
        .await
        .map_err(AppError::Other)?;
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
    pub issue_price_jpy: String, // for USD tickers, ¥ form too

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

    let issue_pnl_pct_num = match (
        card.issue_pnl_jpy,
        card.issue_value_jpy,
        card.quantity,
        card.avg_cost_native,
    ) {
        (Some(_), Some(_), Some(qty), Some(avg)) if avg > 0.0 && qty > 0.0 => {
            // Native PnL%: (current_price - avg_cost) / avg_cost * 100
            ((card.issue_price_native - avg) / avg) * 100.0
        }
        _ => 0.0,
    };

    // Issue price expressed in JPY (matches the JST display convention).
    let issue_price_jpy_num = if card.currency == "USD" {
        let fx = card.fx_rate_at_issue.unwrap_or(DISPLAY_USD_JPY_FALLBACK);
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
    let has_position = card.quantity.is_some_and(|quantity| quantity > 0.0);
    let position_suffix = if has_position {
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
        quantity: card
            .quantity
            .map(|q| format!("{:.2}", q))
            .unwrap_or_default(),
        avg_cost_native: card
            .avg_cost_native
            .map(|c| format!("{:.2}", c))
            .unwrap_or_default(),
        issue_value_jpy: card
            .issue_value_jpy
            .map(format_with_commas)
            .unwrap_or_default(),
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
