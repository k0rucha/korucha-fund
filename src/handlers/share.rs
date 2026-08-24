use askama::Template;
use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::db::{share_cards, snapshots};
use crate::handlers::AppState;
use crate::handlers::error::{AppError, AppResult};
use crate::handlers::template_response::TemplateResponse;
use crate::services::share_cards::{self as share_service, FundHoldingSnapshot};
use crate::util::{format_with_commas, signed_pct, signed_with_commas};

pub(crate) fn base_url(state: &AppState) -> &str {
    &state.config.public_base_url
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

pub async fn create_share_card(
    State(state): State<AppState>,
    Query(q): Query<CreateShareQuery>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    if !crate::handlers::claim_share_creation(&state).await {
        return Err((
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "共有カードの連続発行はできません。少し待ってから再試行してください".into(),
        ));
    }
    let id = share_service::create_fund_card(&state.db, q.span.as_deref())
        .await
        .map_err(create_card_error_response)?;
    let url = format!("/share/{}", id);
    Ok(Json(CreateShareResponse { id, url }))
}

pub(crate) fn create_card_error_response(
    error: share_service::CreateCardError,
) -> (axum::http::StatusCode, String) {
    match error {
        share_service::CreateCardError::InvalidInput(_) => {
            (axum::http::StatusCode::BAD_REQUEST, error.to_string())
        }
        share_service::CreateCardError::DataUnavailable(_) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            error.to_string(),
        ),
        share_service::CreateCardError::Database(_)
        | share_service::CreateCardError::Serialization(_)
        | share_service::CreateCardError::Other(_) => {
            tracing::error!(%error, "share card creation failed");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "共有カードの発行中にエラーが発生しました".into(),
            )
        }
    }
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
    pub fallback_price_symbols: String,
    pub fallback_fx_rate: bool,

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
) -> AppResult<impl IntoResponse> {
    let card = share_cards::get_share_card(&state.db, &id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let issue_holdings: Vec<FundHoldingSnapshot> =
        serde_json::from_str(&card.holdings_json).map_err(|error| AppError::Other(error.into()))?;

    let current_fund = share_service::current_fund(&state.db)
        .await
        .map_err(AppError::Other)?;
    let cur_value = current_fund.total_value_jpy;

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
    let jst = crate::util::jst();
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
    let snaps = snapshots::list_snapshots(&state.db)
        .await
        .map_err(AppError::Database)?;
    let filtered: Vec<_> = snaps.into_iter().filter(|s| s.date <= issue_date).collect();
    let history_dates: Vec<String> = filtered.iter().map(|s| s.date.to_string()).collect();
    let history_values: Vec<f64> = filtered.iter().map(|s| s.total_value_jpy).collect();
    let history_pnls: Vec<f64> = filtered.iter().map(|s| s.unrealized_pnl_jpy).collect();

    let base = base_url(&state);
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
        fallback_price_symbols: current_fund.fallback_price_symbols.join(", "),
        fallback_fx_rate: current_fund.fallback_fx_rate,

        holdings: rows,

        history_dates_json: serde_json::to_string(&history_dates)
            .map_err(|error| AppError::Other(error.into()))?,
        history_values_json: serde_json::to_string(&history_values)
            .map_err(|error| AppError::Other(error.into()))?,
        history_pnls_json: serde_json::to_string(&history_pnls)
            .map_err(|error| AppError::Other(error.into()))?,
        default_span: card.default_span,

        og_url,
        og_image_url,
        og_title,
        og_description,
    };

    Ok(TemplateResponse(tmpl))
}
