use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use std::collections::HashMap;

use crate::AppState;
use crate::db::{transactions, prices, fx, snapshots, symbols};
use crate::services::portfolio;

#[derive(Serialize)]
pub struct CompositionItem {
    pub symbol: String,
    pub value_jpy: f64,
}

#[derive(Serialize)]
pub struct CompositionResponse {
    pub labels: Vec<String>,
    pub full_names: Vec<String>,
    pub values: Vec<f64>,
}

const NAME_TRUNCATE_CHARS: usize = 12;

fn truncate_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= NAME_TRUNCATE_CHARS {
        name.to_string()
    } else {
        let truncated: String = chars.into_iter().take(NAME_TRUNCATE_CHARS).collect();
        format!("{}…", truncated)
    }
}

pub async fn composition_json(State(state): State<AppState>) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let txs = transactions::list_transactions(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch transactions: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let holdings = portfolio::calculate_holdings(&txs);

    let latest_prices = prices::get_latest_prices(&state.db).await.unwrap_or_default();
    let price_map: HashMap<String, f64> = latest_prices.into_iter().map(|p| (p.symbol, p.close_price)).collect();

    // Display-only fallback: nominal 150 JPY/USD when no FX rate is cached.
    // Snapshot writers skip persistence rather than fabricate a rate.
    let usdjpy = fx::get_latest_usdjpy(&state.db).await.unwrap_or(None).unwrap_or(150.0);

    let symbol_list = symbols::get_symbol_names(&state.db).await.unwrap_or_default();
    let name_map: HashMap<String, String> = symbol_list
        .into_iter()
        .filter_map(|s| s.name.map(|n| (s.symbol, n)))
        .collect();

    let mut entries: Vec<(String, String, f64)> = holdings
        .into_iter()
        .filter_map(|h| {
            let current_price = price_map.get(&h.symbol).copied().unwrap_or(0.0);
            let fx_rate = if h.currency == "USD" { usdjpy } else { 1.0 };
            let value_jpy = h.quantity * current_price * fx_rate;
            if value_jpy > 0.0 {
                let full_name = name_map.get(&h.symbol).cloned().unwrap_or_default();
                Some((h.symbol, full_name, (value_jpy * 100.0).round() / 100.0))
            } else {
                None
            }
        })
        .collect();

    // Largest slice first — keeps legend order deterministic across reloads.
    entries.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let labels = entries
        .iter()
        .map(|(sym, name, _)| {
            if name.is_empty() {
                sym.clone()
            } else {
                format!("{} {}", sym, truncate_name(name))
            }
        })
        .collect();
    let full_names = entries
        .iter()
        .map(|(sym, name, _)| if name.is_empty() { sym.clone() } else { name.clone() })
        .collect();
    let values = entries.iter().map(|(_, _, v)| *v).collect();

    Ok(Json(CompositionResponse { labels, full_names, values }))
}

#[derive(Serialize)]
pub struct TimeseriesResponse {
    pub dates: Vec<String>,
    pub values: Vec<f64>,
    pub costs: Vec<f64>,
    pub pnls: Vec<f64>,
}

pub async fn timeseries_json(State(state): State<AppState>) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let snaps = snapshots::list_snapshots(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch snapshots: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dates: Vec<String> = snaps.iter().map(|s| s.date.to_string()).collect();
    let values: Vec<f64> = snaps.iter().map(|s| s.total_value_jpy).collect();
    let costs: Vec<f64> = snaps.iter().map(|s| s.total_cost_jpy).collect();
    let pnls: Vec<f64> = snaps.iter().map(|s| s.unrealized_pnl_jpy).collect();

    Ok(Json(TimeseriesResponse { dates, values, costs, pnls }))
}
