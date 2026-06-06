use axum::{extract::State, response::IntoResponse, http::StatusCode, Json};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Instant;
use serde_json::json;

use crate::AppState;
use crate::db::{transactions, prices, fx, snapshots, api_stats};
use crate::services::{portfolio, yfinance};
use crate::util::jst_today;

/// Compute total value/cost from holdings + cached prices.
/// Returns `Some((value, cost))` only when we have enough data to be honest:
/// USD holdings require an FX rate; without one we'd write fabricated 150 JPY/USD
/// into the snapshot, corrupting historical data. Spec §6 says snapshots are
/// time-series-canonical, so we'd rather skip than lie.
fn compute_totals(
    holdings: &[portfolio::Holding],
    price_map: &HashMap<String, f64>,
    usdjpy: Option<f64>,
) -> Option<(f64, f64)> {
    let needs_fx = holdings.iter().any(|h| h.currency == "USD");
    if needs_fx && usdjpy.is_none() {
        return None;
    }
    let fx = usdjpy.unwrap_or(1.0);
    let mut total_value = 0.0;
    let mut total_cost = 0.0;
    for h in holdings {
        let current_price = price_map.get(&h.symbol).copied().unwrap_or(0.0);
        let rate = if h.currency == "USD" { fx } else { 1.0 };
        total_value += h.quantity * current_price * rate;
        total_cost += h.total_cost_jpy;
    }
    Some((total_value, total_cost))
}

pub async fn refresh_prices(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    // Concurrency guard: only one refresh at a time. Multiple in-flight
    // refreshes would double-count the rate limiter and over-fire yfinance.
    if state
        .refresh_lock
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    // RAII guard so the lock is released on every exit path.
    struct Release<'a>(&'a std::sync::atomic::AtomicBool);
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _guard = Release(&state.refresh_lock);

    let start = Instant::now();
    tracing::info!("Manual refresh triggered");

    // ========================================
    // STAGE 1: snapshot from existing cache (no external calls).
    // ========================================
    let txs = transactions::list_transactions(&state.db).await.map_err(|e| {
        tracing::error!("Failed to fetch transactions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let holdings = portfolio::calculate_holdings(&txs);
    let symbols: Vec<String> = holdings.iter().map(|h| h.symbol.clone()).collect();
    tracing::info!(
        "Calculated holdings: {} entries, {} unique symbols",
        holdings.len(),
        symbols.len()
    );

    let today = jst_today();

    // Stage 2 first: try to refresh from external API. If it succeeds we use
    // the fresh data for the snapshot; otherwise we fall back to existing
    // cache. This avoids the previous behavior of writing the snapshot twice.
    let mut updated_from_api = false;
    match api_stats::can_request_api(&state.db).await {
        Ok((can_request, minutes_remaining)) => {
            if can_request {
                tracing::info!("API request allowed, updating from external API...");
                let (api_success, api_updated) =
                    yfinance::try_update_prices_from_api(&state.db, &symbols).await;
                if api_success {
                    updated_from_api = api_updated;
                } else {
                    tracing::error!(
                        "API update failed, falling back to cache for snapshot"
                    );
                }
            } else {
                tracing::info!(
                    "API rate limit in effect. {} minutes until next allowed request.",
                    minutes_remaining
                );
            }
        }
        Err(e) => tracing::error!("Failed to check API rate limit: {}", e),
    }

    // ========================================
    // Snapshot — write exactly once, after the API attempt has settled.
    // ========================================
    let latest_prices = prices::get_latest_prices(&state.db).await.unwrap_or_default();
    let price_map: HashMap<String, f64> = latest_prices
        .into_iter()
        .map(|p| (p.symbol, p.close_price))
        .collect();
    let usdjpy = fx::get_latest_usdjpy(&state.db).await.unwrap_or(None);

    if let Some((total_value, total_cost)) = compute_totals(&holdings, &price_map, usdjpy) {
        let pnl = total_value - total_cost;
        match snapshots::upsert_snapshot(&state.db, today, total_value, total_cost, pnl).await {
            Ok(()) => tracing::info!(
                "Today's snapshot upserted: value={} cost={} pnl={}",
                total_value,
                total_cost,
                pnl
            ),
            Err(e) => tracing::error!("Failed to upsert snapshot: {}", e),
        }
    } else {
        tracing::warn!(
            "Skipped snapshot upsert: USD holdings present but no USDJPY rate cached"
        );
    }

    let remaining_requests = api_stats::requests_remaining(&state.db).await.unwrap_or(0);

    tracing::info!(
        "Manual refresh completed in {} ms (updated_from_api={})",
        start.elapsed().as_millis(),
        updated_from_api
    );

    // HX-Trigger asks the listener in base.html to reload the dashboard.
    let response = json!({
        "ok": true,
        "updated_from_api": updated_from_api,
        "remaining_api_requests": remaining_requests,
    });

    Ok((
        StatusCode::OK,
        [("HX-Trigger", "refreshAll")],
        Json(response),
    ))
}
