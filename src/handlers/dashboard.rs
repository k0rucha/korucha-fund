use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::handlers::AppState;
use crate::handlers::template_response::TemplateResponse;
use crate::services::dashboard;
use crate::util::{format_with_commas, signed_pct};

pub struct HoldingView {
    pub symbol: String,
    pub name: String,
    pub quantity: String,
    pub average_cost_native: String,
    pub current_price_native: String,
    pub current_price_native_num: f64,
    pub quantity_num: f64,
    pub average_cost_native_num: f64,
    pub total_cost_jpy: String,
    pub current_value_jpy: String,
    pub current_value_jpy_num: f64,
    pub unrealized_pnl_jpy: String,
    pub unrealized_pnl_jpy_num: f64,
    pub unrealized_pnl_pct: String,
    pub unrealized_pnl_pct_num: f64,
    // Day-over-day deltas for this holding
    pub dod_available: bool,
    pub dod_pnl_jpy: String,
    pub dod_pnl_jpy_num: f64,
    pub dod_pnl_pct: String,
    pub dod_pnl_pct_num: f64,
    // Month-over-month deltas for this holding
    pub mom_available: bool,
    pub mom_pnl_jpy: String,
    pub mom_pnl_jpy_num: f64,
    pub mom_pnl_pct: String,
    pub mom_pnl_pct_num: f64,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub holdings: Vec<HoldingView>,
    pub total_cost_jpy: String,
    pub total_value_jpy: String,
    pub total_unrealized_pnl_jpy: String,
    pub total_unrealized_pnl_jpy_num: f64,
    pub total_unrealized_pnl_pct: String,
    pub total_unrealized_pnl_pct_num: f64,
    // Day-over-day deltas (vs. previous trading day snapshot).
    pub dod_available: bool,
    pub dod_ref_date: String,
    pub dod_value_delta: String,
    pub dod_value_delta_num: f64,
    pub dod_value_pct: String,
    pub dod_cost_delta: String,
    pub dod_cost_delta_num: f64,
    pub dod_pnl_delta: String,
    pub dod_pnl_delta_num: f64,
    pub dod_pnl_pct_delta: String,
    pub dod_pnl_pct_delta_num: f64,
    // Realized & cumulative P&L
    pub realized_pnl_jpy: String,
    pub realized_pnl_jpy_num: f64,
    pub cumulative_pnl_jpy: String,
    pub cumulative_pnl_jpy_num: f64,
    // Month-over-month deltas (vs. snapshot closest to ~30 days ago).
    pub mom_available: bool,
    pub mom_ref_date: String,
    pub mom_value_delta: String,
    pub mom_value_delta_num: f64,
    pub mom_value_pct: String,
    pub mom_cost_delta: String,
    pub mom_cost_delta_num: f64,
    pub mom_pnl_delta: String,
    pub mom_pnl_delta_num: f64,
    pub mom_pnl_pct_delta: String,
    pub mom_pnl_pct_delta_num: f64,
    pub fallback_price_symbols: String,
    pub fallback_fx_rate: bool,
}

pub async fn dashboard_index(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let data = dashboard::load(&state.db).await.map_err(|error| {
        tracing::error!(%error, "failed to load dashboard");
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let holding_views = data
        .holdings
        .into_iter()
        .map(|performance| {
            let h = performance.holding;
            let (dod_available, dod_pnl_jpy_num, dod_pnl_pct_num) = performance
                .day
                .map(|change| (true, change.amount, change.percent))
                .unwrap_or((false, 0.0, 0.0));
            let (mom_available, mom_pnl_jpy_num, mom_pnl_pct_num) = performance
                .month
                .map(|change| (true, change.amount, change.percent))
                .unwrap_or((false, 0.0, 0.0));
            HoldingView {
                symbol: h.symbol,
                name: performance.name,
                quantity: format!("{:.2}", h.quantity),
                quantity_num: h.quantity,
                average_cost_native: format!("{:.2}", h.average_cost_native),
                average_cost_native_num: h.average_cost_native,
                current_price_native: format!("{:.2}", h.current_price_native),
                current_price_native_num: h.current_price_native,
                total_cost_jpy: format_with_commas(h.total_cost_jpy),
                current_value_jpy: format_with_commas(h.current_value_jpy),
                current_value_jpy_num: h.current_value_jpy,
                unrealized_pnl_jpy: format_with_commas(h.unrealized_pnl_jpy.abs()),
                unrealized_pnl_jpy_num: h.unrealized_pnl_jpy,
                unrealized_pnl_pct: format!("{:.2}", h.unrealized_pnl_pct),
                unrealized_pnl_pct_num: h.unrealized_pnl_pct,
                dod_available,
                dod_pnl_jpy: format_with_commas(dod_pnl_jpy_num.abs()),
                dod_pnl_jpy_num,
                dod_pnl_pct: format!("{:.2}", dod_pnl_pct_num.abs()),
                dod_pnl_pct_num,
                mom_available,
                mom_pnl_jpy: format_with_commas(mom_pnl_jpy_num.abs()),
                mom_pnl_jpy_num,
                mom_pnl_pct: format!("{:.2}", mom_pnl_pct_num.abs()),
                mom_pnl_pct_num,
            }
        })
        .collect();

    let (
        dod_available,
        dod_ref_date,
        dod_value_delta_num,
        dod_value_pct_num,
        dod_cost_delta_num,
        dod_pnl_delta_num,
        dod_pnl_pct_delta_num,
    ) = unpack_change(data.day);
    let (
        mom_available,
        mom_ref_date,
        mom_value_delta_num,
        mom_value_pct_num,
        mom_cost_delta_num,
        mom_pnl_delta_num,
        mom_pnl_pct_delta_num,
    ) = unpack_change(data.month);
    let portfolio = data.portfolio;
    let fallback_price_symbols = data.fallback_price_symbols.join(", ");

    Ok(TemplateResponse(DashboardTemplate {
        holdings: holding_views,
        total_cost_jpy: format_with_commas(portfolio.total_cost_jpy),
        total_value_jpy: format_with_commas(portfolio.total_value_jpy),
        total_unrealized_pnl_jpy: format_with_commas(portfolio.unrealized_pnl_jpy.abs()),
        total_unrealized_pnl_jpy_num: portfolio.unrealized_pnl_jpy,
        total_unrealized_pnl_pct: format!("{:.2}", portfolio.unrealized_pnl_pct),
        total_unrealized_pnl_pct_num: portfolio.unrealized_pnl_pct,
        dod_available,
        dod_ref_date,
        dod_value_delta: format_with_commas(dod_value_delta_num.abs()),
        dod_value_delta_num,
        dod_value_pct: signed_pct(dod_value_pct_num),
        dod_cost_delta: format_with_commas(dod_cost_delta_num.abs()),
        dod_cost_delta_num,
        dod_pnl_delta: format_with_commas(dod_pnl_delta_num.abs()),
        dod_pnl_delta_num,
        dod_pnl_pct_delta: format!("{:.2}pt", dod_pnl_pct_delta_num.abs()),
        dod_pnl_pct_delta_num,
        mom_available,
        mom_ref_date,
        // Absolute strings — templates render the sign + ¥ in the right
        // order. (Was: signed_with_commas → produced "¥-12,345".)
        mom_value_delta: format_with_commas(mom_value_delta_num.abs()),
        mom_value_delta_num,
        mom_value_pct: signed_pct(mom_value_pct_num),
        mom_cost_delta: format_with_commas(mom_cost_delta_num.abs()),
        mom_cost_delta_num,
        mom_pnl_delta: format_with_commas(mom_pnl_delta_num.abs()),
        mom_pnl_delta_num,
        mom_pnl_pct_delta: format!("{:.2}pt", mom_pnl_pct_delta_num.abs()),
        mom_pnl_pct_delta_num,
        realized_pnl_jpy: format_with_commas(portfolio.realized_pnl_jpy.abs()),
        realized_pnl_jpy_num: portfolio.realized_pnl_jpy,
        cumulative_pnl_jpy: format_with_commas(data.cumulative_pnl_jpy.abs()),
        cumulative_pnl_jpy_num: data.cumulative_pnl_jpy,
        fallback_price_symbols,
        fallback_fx_rate: data.fallback_fx_rate,
    }))
}

fn unpack_change(
    change: Option<dashboard::PortfolioChange>,
) -> (bool, String, f64, f64, f64, f64, f64) {
    change
        .map(|change| {
            (
                true,
                change.reference_date.to_string(),
                change.value.amount,
                change.value.percent,
                change.cost,
                change.pnl,
                change.pnl_percent_points,
            )
        })
        .unwrap_or((false, String::new(), 0.0, 0.0, 0.0, 0.0, 0.0))
}
