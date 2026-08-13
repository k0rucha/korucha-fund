use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::db::{fx, prices, symbols, transactions};
use crate::domain::portfolio::{self, Portfolio, PortfolioAnalysis};

pub(crate) const DISPLAY_USD_JPY_FALLBACK: f64 = 150.0;

pub struct DisplayPortfolio {
    pub portfolio: Portfolio,
    pub symbol_names: HashMap<String, String>,
    pub usd_jpy: f64,
}

pub async fn current_for_display(pool: &SqlitePool) -> sqlx::Result<DisplayPortfolio> {
    let (analysis, prices, usd_jpy) = load_valuation_inputs(pool).await?;
    let usd_jpy = usd_jpy.unwrap_or(DISPLAY_USD_JPY_FALLBACK);
    let portfolio = portfolio::value_portfolio(analysis, &prices, Some(usd_jpy))
        .expect("display valuation always supplies USDJPY");
    let symbol_names = symbols::get_symbol_names(pool)
        .await?
        .into_iter()
        .filter_map(|symbol| symbol.name.map(|name| (symbol.symbol, name)))
        .collect();

    Ok(DisplayPortfolio {
        portfolio,
        symbol_names,
        usd_jpy,
    })
}

pub async fn current_for_snapshot(pool: &SqlitePool) -> sqlx::Result<Option<Portfolio>> {
    let (analysis, prices, usd_jpy) = load_valuation_inputs(pool).await?;
    Ok(portfolio::value_portfolio(analysis, &prices, usd_jpy))
}

pub async fn current_analysis(pool: &SqlitePool) -> sqlx::Result<PortfolioAnalysis> {
    let transactions = transactions::list_transactions(pool).await?;
    Ok(portfolio::analyze_transactions(&transactions))
}

async fn load_valuation_inputs(
    pool: &SqlitePool,
) -> sqlx::Result<(PortfolioAnalysis, HashMap<String, f64>, Option<f64>)> {
    let analysis = current_analysis(pool).await?;
    let prices = prices::get_latest_prices(pool)
        .await?
        .into_iter()
        .map(|price| (price.symbol, price.close_price))
        .collect();
    let usd_jpy = fx::get_latest_usdjpy(pool).await?;
    Ok((analysis, prices, usd_jpy))
}
