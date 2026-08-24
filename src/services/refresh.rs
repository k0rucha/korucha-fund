use sqlx::SqlitePool;

use crate::db::{api_stats, snapshots};
use crate::services::{market_data, portfolio};
use crate::util::jst_today;

pub struct RefreshOutcome {
    pub updated_from_api: bool,
    pub external_update_attempted: bool,
    pub remaining_api_requests: i64,
}

pub async fn run(pool: &SqlitePool) -> sqlx::Result<RefreshOutcome> {
    let analysis = portfolio::current_analysis(pool).await?;
    let symbols: Vec<_> = analysis
        .holdings
        .iter()
        .map(|holding| holding.symbol.clone())
        .collect();
    tracing::info!(
        holdings = analysis.holdings.len(),
        symbols = symbols.len(),
        "calculated holdings"
    );

    if symbols.is_empty() {
        return Ok(RefreshOutcome {
            updated_from_api: false,
            external_update_attempted: false,
            remaining_api_requests: api_stats::requests_remaining(pool).await?,
        });
    }

    let (can_request, minutes_remaining) = api_stats::can_request_api(pool).await?;
    let (updated_from_api, external_update_attempted) = if can_request {
        tracing::info!("API request allowed, updating from external API");
        let (success, updated) = market_data::try_update_prices_from_api(pool, &symbols).await;
        (success && updated, updated)
    } else {
        tracing::info!(minutes_remaining, "API rate limit in effect");
        (false, false)
    };

    if let Some(portfolio) = portfolio::current_for_snapshot(pool).await? {
        snapshots::upsert_snapshot(
            pool,
            jst_today(),
            portfolio.total_value_jpy,
            portfolio.total_cost_jpy,
            portfolio.unrealized_pnl_jpy,
        )
        .await?;
    } else {
        tracing::warn!("skipped snapshot: USD holdings present but USDJPY is unavailable");
    }

    Ok(RefreshOutcome {
        updated_from_api,
        external_update_attempted,
        remaining_api_requests: api_stats::requests_remaining(pool).await?,
    })
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn empty_portfolio_does_not_consume_api_quota_or_write_snapshot() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let outcome = run(&pool).await.unwrap();

        assert!(!outcome.external_update_attempted);
        assert_eq!(outcome.remaining_api_requests, 15);
        assert!(snapshots::list_snapshots(&pool).await.unwrap().is_empty());
    }
}
