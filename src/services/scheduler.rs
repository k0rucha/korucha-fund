use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use sqlx::SqlitePool;

use crate::db::{fx, prices, snapshots, transactions};
use crate::domain::portfolio::{self as domain_portfolio, Transaction};
use crate::services::{market_data, portfolio};

pub async fn backfill_and_regenerate(pool: &SqlitePool) -> anyhow::Result<()> {
    let transactions = transactions::list_transactions(pool).await?;
    let Some(earliest) = transactions
        .iter()
        .map(|transaction| transaction.txn_date)
        .min()
    else {
        tracing::info!("Backfill: no transactions, skipping");
        return Ok(());
    };
    let today = crate::util::jst_today();
    let mut symbols: Vec<_> = transactions
        .iter()
        .map(|transaction| transaction.symbol.clone())
        .collect();
    symbols.sort();
    symbols.dedup();

    tracing::info!(
        %earliest,
        symbols = symbols.len(),
        "backfilling market history"
    );
    for symbol in &symbols {
        match market_data::backfill_price_history(pool, symbol, earliest).await {
            Ok(rows) => tracing::info!(%symbol, rows, "backfilled prices"),
            Err(error) => tracing::warn!(%symbol, %error, "price backfill failed"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    if transactions
        .iter()
        .any(|transaction| transaction.currency == "USD")
    {
        match market_data::backfill_fx_history(pool, earliest).await {
            Ok(rows) => tracing::info!(rows, "backfilled USDJPY"),
            Err(error) => tracing::warn!(%error, "USDJPY backfill failed"),
        }
    }

    regenerate_snapshots(pool, &transactions, earliest, today).await
}

async fn regenerate_snapshots(
    pool: &SqlitePool,
    transactions: &[Transaction],
    start: NaiveDate,
    end: NaiveDate,
) -> anyhow::Result<()> {
    let mut date = start;
    let mut written = 0usize;

    while date <= end {
        let analysis = domain_portfolio::analyze_transactions_as_of(transactions, date);
        if analysis.holdings.is_empty() {
            date += Duration::days(1);
            continue;
        }

        let mut price_map = HashMap::new();
        for holding in &analysis.holdings {
            if let Some(price) = prices::get_price_on_or_before(pool, &holding.symbol, date).await?
            {
                price_map.insert(holding.symbol.clone(), price);
            }
        }
        if price_map.is_empty() {
            date += Duration::days(1);
            continue;
        }

        let usd_jpy = fx::get_usdjpy_on_or_before(pool, date).await?;
        let Some(portfolio) = domain_portfolio::value_portfolio(analysis, &price_map, usd_jpy)
        else {
            date += Duration::days(1);
            continue;
        };
        snapshots::upsert_snapshot(
            pool,
            date,
            portfolio.total_value_jpy,
            portfolio.total_cost_jpy,
            portfolio.unrealized_pnl_jpy,
        )
        .await?;
        written += 1;
        date += Duration::days(1);
    }

    tracing::info!(written, "snapshots regenerated");
    Ok(())
}

pub async fn run_daily_batch(pool: &SqlitePool) {
    if let Err(error) = run_daily_batch_inner(pool).await {
        tracing::error!(%error, "daily batch failed");
    }
}

async fn run_daily_batch_inner(pool: &SqlitePool) -> anyhow::Result<()> {
    tracing::info!("starting daily batch");
    let analysis = portfolio::current_analysis(pool).await?;
    let symbols: Vec<_> = analysis
        .holdings
        .iter()
        .map(|holding| holding.symbol.clone())
        .collect();
    if symbols.is_empty() {
        tracing::info!("daily batch: no holdings, skipping");
        return Ok(());
    }

    let (success, claimed) = market_data::try_update_prices_from_api(pool, &symbols).await;
    if !claimed {
        tracing::info!("daily market update skipped: API rate limit in effect");
    } else if !success {
        tracing::warn!("daily market update was incomplete");
    }

    if let Some(portfolio) = portfolio::current_for_snapshot(pool).await? {
        let today = crate::util::jst_today();
        snapshots::upsert_snapshot(
            pool,
            today,
            portfolio.total_value_jpy,
            portfolio.total_cost_jpy,
            portfolio.unrealized_pnl_jpy,
        )
        .await?;
        tracing::info!(%today, "daily snapshot saved");
    } else {
        tracing::warn!("daily snapshot skipped: USDJPY is unavailable");
    }

    tracing::info!("daily batch completed");
    Ok(())
}
