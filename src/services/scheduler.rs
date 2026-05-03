use std::collections::HashMap;
use chrono::{Duration, NaiveDate};
use sqlx::SqlitePool;

use crate::db::{transactions, prices, fx, snapshots, api_stats};
use crate::services::{portfolio, yfinance};

/// Backfill historical prices/FX for all held symbols starting from the
/// earliest transaction date, then regenerate every daily snapshot from
/// that date through today. Idempotent — existing prices are preserved.
pub async fn backfill_and_regenerate(pool: &SqlitePool) -> anyhow::Result<()> {
    let txs = transactions::list_transactions(pool).await?;
    if txs.is_empty() {
        tracing::info!("Backfill: no transactions, skipping");
        return Ok(());
    }

    let earliest = txs
        .iter()
        .map(|t| t.txn_date)
        .min()
        .expect("non-empty checked above");
    let today = crate::util::jst_today();

    let mut symbols: Vec<String> = txs.iter().map(|t| t.symbol.clone()).collect();
    symbols.sort();
    symbols.dedup();

    tracing::info!("Backfill: fetching history from {} for {} symbols", earliest, symbols.len());

    for symbol in &symbols {
        match yfinance::backfill_price_history(pool, symbol, earliest).await {
            Ok(n) => tracing::info!("Backfill: {} → {} new rows", symbol, n),
            Err(e) => tracing::warn!("Backfill failed for {}: {}", symbol, e),
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    let needs_fx = txs.iter().any(|t| t.currency == "USD");
    if needs_fx {
        match yfinance::backfill_fx_history(pool, earliest).await {
            Ok(n) => tracing::info!("Backfill: USDJPY → {} new rows", n),
            Err(e) => tracing::warn!("Backfill USDJPY failed: {}", e),
        }
    }

    regenerate_snapshots(pool, &txs, earliest, today).await?;
    Ok(())
}

/// Walk every calendar day from `start` through `end` and (re)compute the
/// portfolio snapshot using the most recent close price on or before each day.
async fn regenerate_snapshots(
    pool: &SqlitePool,
    txs: &[transactions::Transaction],
    start: NaiveDate,
    end: NaiveDate,
) -> anyhow::Result<()> {
    let mut date = start;
    let mut written = 0usize;

    while date <= end {
        let holdings = portfolio::calculate_holdings_as_of(txs, date);

        if holdings.is_empty() {
            date += Duration::days(1);
            continue;
        }

        let usdjpy = fx::get_usdjpy_on_or_before(pool, date).await.unwrap_or(None);
        let needs_fx = holdings.iter().any(|h| h.currency == "USD");

        // If USD holdings exist on this date but we have no FX rate yet,
        // skip — fabricating 150 yen would corrupt the historical record.
        if needs_fx && usdjpy.is_none() {
            date += Duration::days(1);
            continue;
        }

        let mut total_value = 0.0;
        let mut total_cost = 0.0;
        let mut have_any_price = false;

        for h in &holdings {
            let price = match prices::get_price_on_or_before(pool, &h.symbol, date).await {
                Ok(Some(p)) => { have_any_price = true; p },
                _ => 0.0,
            };
            let fx_rate = if h.currency == "USD" {
                usdjpy.expect("checked above")
            } else {
                1.0
            };
            total_value += h.quantity * price * fx_rate;
            total_cost += h.total_cost_jpy;
        }

        // Skip days where we have no price data at all (avoids zero-value snapshots).
        if !have_any_price {
            date += Duration::days(1);
            continue;
        }

        let pnl = total_value - total_cost;
        if let Err(e) = snapshots::upsert_snapshot(pool, date, total_value, total_cost, pnl).await {
            tracing::warn!("Snapshot upsert failed for {}: {}", date, e);
        } else {
            written += 1;
        }

        date += Duration::days(1);
    }

    tracing::info!("Snapshots regenerated: {} days", written);
    Ok(())
}

/// Run the daily batch job:
/// 1. Fetch latest prices for all held symbols
/// 2. Update FX cache (USDJPY)
/// 3. Generate today's snapshot
pub async fn run_daily_batch(pool: &SqlitePool) {
    tracing::info!("Starting daily batch job...");

    // 1. Get all unique symbols from current holdings
    let txs = match transactions::list_transactions(pool).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Daily batch: failed to fetch transactions: {}", e);
            return;
        }
    };

    let holdings = portfolio::calculate_holdings(&txs);
    let symbols: Vec<String> = holdings.iter().map(|h| h.symbol.clone()).collect();

    if symbols.is_empty() {
        tracing::info!("Daily batch: no holdings found, skipping");
        return;
    }

    tracing::info!("Daily batch: updating prices for {} symbols", symbols.len());

    // 2. Update price cache for each symbol (with sleep between requests).
    //    The scheduler is intentionally NOT gated by the rate limit (it's
    //    expected to run on a schedule), but it DOES record the API session
    //    so the status page reflects scheduler runs as well as manual
    //    refreshes (status.html: "手動更新とスケジューラーの両方でカウント").
    for symbol in &symbols {
        match yfinance::update_price_cache(pool, symbol).await {
            Ok(()) => tracing::info!("Updated price for {}", symbol),
            Err(e) => tracing::warn!("Failed to update price for {}: {}", symbol, e),
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    // 3. Update FX cache
    match yfinance::update_fx_cache(pool).await {
        Ok(()) => tracing::info!("Updated FX cache (USDJPY)"),
        Err(e) => tracing::warn!("Failed to update FX cache: {}", e),
    }

    // 3b. Record this batch as one external API session for rate-limit stats.
    //     Forced because the scheduler is supposed to run regardless of limit.
    if let Err(e) = api_stats::record_api_request_forced(pool).await {
        tracing::warn!("Daily batch: failed to record api request: {}", e);
    }

    // 4. Generate today's snapshot.
    //    If USD holdings exist but USDJPY is missing, skip rather than
    //    fabricate a 150 yen rate that would corrupt historical snapshots.
    let latest_prices = prices::get_latest_prices(pool).await.unwrap_or_default();
    let price_map: HashMap<String, f64> = latest_prices
        .into_iter()
        .map(|p| (p.symbol, p.close_price))
        .collect();
    let usdjpy_opt = fx::get_latest_usdjpy(pool).await.unwrap_or(None);
    let needs_fx = holdings.iter().any(|h| h.currency == "USD");
    if needs_fx && usdjpy_opt.is_none() {
        tracing::warn!(
            "Daily batch: skipping snapshot — USD holdings present but no USDJPY rate cached"
        );
        return;
    }
    let usdjpy = usdjpy_opt.unwrap_or(1.0);

    let mut total_value = 0.0;
    let mut total_cost = 0.0;

    for h in &holdings {
        let current_price = price_map.get(&h.symbol).copied().unwrap_or(0.0);
        let fx_rate = if h.currency == "USD" { usdjpy } else { 1.0 };
        total_value += h.quantity * current_price * fx_rate;
        total_cost += h.total_cost_jpy;
    }

    let pnl = total_value - total_cost;
    let today = crate::util::jst_today();

    match snapshots::upsert_snapshot(pool, today, total_value, total_cost, pnl).await {
        Ok(()) => tracing::info!("Daily batch: snapshot saved for {}", today),
        Err(e) => tracing::error!("Daily batch: failed to save snapshot: {}", e),
    }

    tracing::info!("Daily batch job completed");
}
