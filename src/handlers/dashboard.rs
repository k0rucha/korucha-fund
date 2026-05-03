use axum::{extract::State, response::IntoResponse};
use askama::Template;
use std::collections::HashMap;

use crate::AppState;
use crate::db::{transactions, prices, fx, symbols, snapshots};
use crate::services::portfolio;
use crate::util::{format_with_commas, signed_pct, jst_today};

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
}

pub async fn dashboard_index(State(state): State<AppState>) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let txs = transactions::list_transactions(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch transactions: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let holdings = portfolio::calculate_holdings(&txs);

    let latest_prices = prices::get_latest_prices(&state.db).await.unwrap_or_default();
    let price_map: HashMap<String, f64> = latest_prices.into_iter().map(|p| (p.symbol, p.close_price)).collect();

    // For *display* only, fall back to the spec's nominal 150 JPY/USD when no
    // FX rate is cached — the dashboard is informational, and a working number
    // is more useful than blank cells. Snapshot writers (scheduler/refresh)
    // skip the write instead, so persisted history is never fabricated.
    let usdjpy = fx::get_latest_usdjpy(&state.db).await.unwrap_or(None).unwrap_or(150.0);

    // Fetch symbol names from DB
    let symbol_list = symbols::get_symbol_names(&state.db).await.unwrap_or_default();
    let name_map: HashMap<String, String> = symbol_list
        .into_iter()
        .filter_map(|s| s.name.map(|n| (s.symbol, n)))
        .collect();

    let mut holding_views = Vec::new();
    let mut total_cost_jpy = 0.0;
    let mut total_value_jpy = 0.0;

    for h in holdings {
        let current_price = price_map.get(&h.symbol).copied().unwrap_or(0.0);
        let fx_rate = if h.currency == "USD" { usdjpy } else { 1.0 };
        
        let current_value_jpy = h.quantity * current_price * fx_rate;
        let unrealized_pnl_jpy = current_value_jpy - h.total_cost_jpy;
        let unrealized_pnl_pct = if h.total_cost_jpy > 0.0 {
            (unrealized_pnl_jpy / h.total_cost_jpy) * 100.0
        } else {
            0.0
        };

        total_cost_jpy += h.total_cost_jpy;
        total_value_jpy += current_value_jpy;

        let display_name = name_map.get(&h.symbol).cloned().unwrap_or_default();

        holding_views.push(HoldingView {
            symbol: h.symbol,
            name: display_name,
            quantity: format!("{:.2}", h.quantity),
            quantity_num: h.quantity,
            average_cost_native: format!("{:.2}", h.average_cost_native),
            average_cost_native_num: h.average_cost_native,
            current_price_native: format!("{:.2}", current_price),
            current_price_native_num: current_price,
            total_cost_jpy: format_with_commas(h.total_cost_jpy),
            current_value_jpy: format_with_commas(current_value_jpy),
            current_value_jpy_num: current_value_jpy,
            // Templates render the sign before ¥, so we send absolute strings.
            unrealized_pnl_jpy: format_with_commas(unrealized_pnl_jpy.abs()),
            unrealized_pnl_jpy_num: unrealized_pnl_jpy,
            unrealized_pnl_pct: format!("{:.2}", unrealized_pnl_pct),
            unrealized_pnl_pct_num: unrealized_pnl_pct,
        });
    }

    // Sort by current value descending — largest holding first.
    holding_views.sort_by(|a, b| b.current_value_jpy_num.partial_cmp(&a.current_value_jpy_num).unwrap_or(std::cmp::Ordering::Equal));

    let total_unrealized_pnl_jpy = total_value_jpy - total_cost_jpy;
    let total_unrealized_pnl_pct = if total_cost_jpy > 0.0 {
        (total_unrealized_pnl_jpy / total_cost_jpy) * 100.0
    } else {
        0.0
    };

    // Month-over-month deltas: compare against the snapshot closest to ~30
    // days ago. If the closest available snapshot is much older than that
    // window (say, no data in the last 60 days), the comparison would be
    // misleading ("MoM" but actually a 6-month delta), so we suppress it.
    let today = jst_today();
    let one_month_ago = today - chrono::Duration::days(30);
    let mom_floor = today - chrono::Duration::days(60);
    let prev_snap = snapshots::get_snapshot_on_or_before(&state.db, one_month_ago)
        .await
        .unwrap_or(None)
        .filter(|s| s.date >= mom_floor);

    let mut mom_available = false;
    let mut mom_ref_date = String::new();
    let mut mom_value_delta_num = 0.0;
    let mut mom_value_pct_num = 0.0;
    let mut mom_cost_delta_num = 0.0;
    let mut mom_pnl_delta_num = 0.0;
    let mut mom_pnl_pct_delta_num = 0.0;

    if let Some(s) = prev_snap {
        mom_available = true;
        mom_ref_date = s.date.to_string();
        mom_value_delta_num = total_value_jpy - s.total_value_jpy;
        mom_value_pct_num = if s.total_value_jpy > 0.0 {
            (mom_value_delta_num / s.total_value_jpy) * 100.0
        } else {
            0.0
        };
        mom_cost_delta_num = total_cost_jpy - s.total_cost_jpy;
        mom_pnl_delta_num = total_unrealized_pnl_jpy - s.unrealized_pnl_jpy;
        let prev_pnl_pct = if s.total_cost_jpy > 0.0 {
            (s.unrealized_pnl_jpy / s.total_cost_jpy) * 100.0
        } else {
            0.0
        };
        mom_pnl_pct_delta_num = total_unrealized_pnl_pct - prev_pnl_pct;
    }

    Ok(DashboardTemplate {
        holdings: holding_views,
        total_cost_jpy: format_with_commas(total_cost_jpy),
        total_value_jpy: format_with_commas(total_value_jpy),
        total_unrealized_pnl_jpy: format_with_commas(total_unrealized_pnl_jpy.abs()),
        total_unrealized_pnl_jpy_num: total_unrealized_pnl_jpy,
        total_unrealized_pnl_pct: format!("{:.2}", total_unrealized_pnl_pct),
        total_unrealized_pnl_pct_num: total_unrealized_pnl_pct,
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
    })
}
