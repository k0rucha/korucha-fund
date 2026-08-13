use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;

use crate::db::snapshots;
use crate::handlers::AppState;
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

pub async fn composition_json(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let current = portfolio::current_for_display(&state.db)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to load portfolio composition");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entries: Vec<(String, String, f64)> = current
        .portfolio
        .holdings
        .into_iter()
        .filter_map(|h| {
            if h.current_value_jpy > 0.0 {
                let full_name = current
                    .symbol_names
                    .get(&h.symbol)
                    .cloned()
                    .unwrap_or_default();
                Some((
                    h.symbol,
                    full_name,
                    (h.current_value_jpy * 100.0).round() / 100.0,
                ))
            } else {
                None
            }
        })
        .collect();

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
        .map(|(sym, name, _)| {
            if name.is_empty() {
                sym.clone()
            } else {
                name.clone()
            }
        })
        .collect();
    let values = entries.iter().map(|(_, _, v)| *v).collect();

    Ok(Json(CompositionResponse {
        labels,
        full_names,
        values,
    }))
}

#[derive(Serialize)]
pub struct TimeseriesResponse {
    pub dates: Vec<String>,
    pub values: Vec<f64>,
    pub costs: Vec<f64>,
    pub pnls: Vec<f64>,
}

pub async fn timeseries_json(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let snaps = snapshots::list_snapshots(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch snapshots: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dates: Vec<String> = snaps.iter().map(|s| s.date.to_string()).collect();
    let values: Vec<f64> = snaps.iter().map(|s| s.total_value_jpy).collect();
    let costs: Vec<f64> = snaps.iter().map(|s| s.total_cost_jpy).collect();
    let pnls: Vec<f64> = snaps.iter().map(|s| s.unrealized_pnl_jpy).collect();

    Ok(Json(TimeseriesResponse {
        dates,
        values,
        costs,
        pnls,
    }))
}
