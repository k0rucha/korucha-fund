use chrono::{Duration, NaiveDate};
use sqlx::SqlitePool;

use crate::db::{fx, prices, snapshots};
use crate::domain::portfolio::{self, Portfolio, ValuedHolding};
use crate::services::portfolio as portfolio_service;
use crate::util::jst_today;

pub struct HoldingPerformance {
    pub holding: ValuedHolding,
    pub name: String,
    pub day: Option<ValueChange>,
    pub month: Option<ValueChange>,
}

pub struct ValueChange {
    pub amount: f64,
    pub percent: f64,
}

pub struct PortfolioChange {
    pub reference_date: NaiveDate,
    pub value: ValueChange,
    pub cost: f64,
    pub pnl: f64,
    pub pnl_percent_points: f64,
}

pub struct Dashboard {
    pub portfolio: Portfolio,
    pub holdings: Vec<HoldingPerformance>,
    pub day: Option<PortfolioChange>,
    pub month: Option<PortfolioChange>,
    pub cumulative_pnl_jpy: f64,
    pub fallback_price_symbols: Vec<String>,
    pub fallback_fx_rate: bool,
}

pub async fn load(pool: &SqlitePool) -> anyhow::Result<Dashboard> {
    let current = portfolio_service::current_for_display(pool).await?;
    let today = jst_today();
    let yesterday = today - Duration::days(1);
    let month_ago = today - Duration::days(30);
    let mut holdings = Vec::with_capacity(current.portfolio.holdings.len());

    for holding in current.portfolio.holdings.iter().cloned() {
        let (day, month) = tokio::try_join!(
            holding_change(pool, &holding, yesterday),
            holding_change(pool, &holding, month_ago),
        )?;
        let name = current
            .symbol_names
            .get(&holding.symbol)
            .cloned()
            .unwrap_or_default();
        holdings.push(HoldingPerformance {
            holding,
            name,
            day,
            month,
        });
    }

    let day = snapshots::get_snapshot_on_or_before(pool, yesterday)
        .await?
        .filter(|snapshot| snapshot.date >= today - Duration::days(7))
        .map(|snapshot| portfolio_change(&current.portfolio, snapshot));
    let month = snapshots::get_snapshot_on_or_before(pool, month_ago)
        .await?
        .filter(|snapshot| snapshot.date >= today - Duration::days(60))
        .map(|snapshot| portfolio_change(&current.portfolio, snapshot));
    let cumulative_pnl_jpy =
        current.portfolio.realized_pnl_jpy + current.portfolio.unrealized_pnl_jpy;

    Ok(Dashboard {
        fallback_price_symbols: current.fallback_price_symbols,
        fallback_fx_rate: current.fallback_fx_rate,
        portfolio: current.portfolio,
        holdings,
        day,
        month,
        cumulative_pnl_jpy,
    })
}

async fn holding_change(
    pool: &SqlitePool,
    holding: &ValuedHolding,
    date: NaiveDate,
) -> sqlx::Result<Option<ValueChange>> {
    let Some(previous_price) = prices::get_price_on_or_before(pool, &holding.symbol, date).await?
    else {
        return Ok(None);
    };
    let fx_rate = if holding.currency == "USD" {
        let Some(rate) = fx::get_usdjpy_on_or_before(pool, date).await? else {
            return Ok(None);
        };
        rate
    } else {
        1.0
    };
    let previous_value = holding.quantity * previous_price * fx_rate;
    let amount = holding.current_value_jpy - previous_value;
    Ok(Some(ValueChange {
        amount,
        percent: portfolio::percentage(amount, previous_value),
    }))
}

fn portfolio_change(current: &Portfolio, previous: snapshots::Snapshot) -> PortfolioChange {
    let value = current.total_value_jpy - previous.total_value_jpy;
    let previous_pnl_percent =
        portfolio::percentage(previous.unrealized_pnl_jpy, previous.total_cost_jpy);

    PortfolioChange {
        reference_date: previous.date,
        value: ValueChange {
            amount: value,
            percent: portfolio::percentage(value, previous.total_value_jpy),
        },
        cost: current.total_cost_jpy - previous.total_cost_jpy,
        pnl: current.unrealized_pnl_jpy - previous.unrealized_pnl_jpy,
        pnl_percent_points: current.unrealized_pnl_pct - previous_pnl_percent,
    }
}
