use sqlx::SqlitePool;

use crate::db::{api_stats, snapshots};
use crate::services::{market_data, portfolio};
use crate::util::jst_today;

pub struct RefreshOutcome {
    pub updated_from_api: bool,
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

    let (can_request, minutes_remaining) = api_stats::can_request_api(pool).await?;
    let updated_from_api = if can_request {
        tracing::info!("API request allowed, updating from external API");
        let (success, updated) = market_data::try_update_prices_from_api(pool, &symbols).await;
        success && updated
    } else {
        tracing::info!(minutes_remaining, "API rate limit in effect");
        false
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
        remaining_api_requests: api_stats::requests_remaining(pool).await?,
    })
}
