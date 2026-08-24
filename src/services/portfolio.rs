use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::db::{fx, prices, symbols, transactions};
use crate::domain::portfolio::{self, Portfolio, PortfolioAnalysis};

pub struct DisplayPortfolio {
    pub portfolio: Portfolio,
    pub symbol_names: HashMap<String, String>,
    pub fallback_price_symbols: Vec<String>,
    pub fallback_fx_rate: bool,
}

pub async fn current_for_display(pool: &SqlitePool) -> anyhow::Result<DisplayPortfolio> {
    let transaction_rows = transactions::list_transactions(pool).await?;
    let analysis = portfolio::analyze_transactions(&transaction_rows);
    let mut prices: HashMap<_, _> = prices::get_latest_prices(pool)
        .await?
        .into_iter()
        .filter(|price| price.close_price.is_finite() && price.close_price > 0.0)
        .map(|price| (price.symbol, price.close_price))
        .collect();
    let mut fallback_price_symbols = Vec::new();

    // A newly entered transaction can be displayed immediately while its
    // quote is fetched in the background. Use the latest real trade price,
    // never a fabricated zero, and expose that fact to the dashboard.
    for holding in &analysis.holdings {
        if !prices.contains_key(&holding.symbol)
            && let Some(transaction) = transaction_rows
                .iter()
                .filter(|transaction| {
                    transaction.symbol == holding.symbol
                        && transaction.price.is_finite()
                        && transaction.price > 0.0
                })
                .max_by_key(|transaction| (transaction.txn_date, transaction.id))
        {
            prices.insert(holding.symbol.clone(), transaction.price);
            fallback_price_symbols.push(holding.symbol.clone());
        }
    }

    let cached_usd_jpy = fx::get_latest_usdjpy(pool)
        .await?
        .filter(|rate| rate.is_finite() && *rate > 0.0);
    let transaction_usd_jpy = transaction_rows
        .iter()
        .filter(|transaction| transaction.currency == "USD")
        .filter_map(|transaction| {
            transaction
                .fx_rate_to_jpy
                .filter(|rate| rate.is_finite() && *rate > 0.0)
                .map(|rate| ((transaction.txn_date, transaction.id), rate))
        })
        .max_by_key(|(order, _)| *order)
        .map(|(_, rate)| rate);
    let fallback_fx_rate = cached_usd_jpy.is_none() && transaction_usd_jpy.is_some();
    let usd_jpy = cached_usd_jpy.or(transaction_usd_jpy);
    let portfolio = portfolio::value_portfolio(analysis, &prices, usd_jpy)
        .ok_or_else(|| anyhow::anyhow!("評価に必要な価格または為替データがありません"))?;
    let symbol_names = symbols::get_symbol_names(pool)
        .await?
        .into_iter()
        .filter_map(|symbol| symbol.name.map(|name| (symbol.symbol, name)))
        .collect();

    Ok(DisplayPortfolio {
        portfolio,
        symbol_names,
        fallback_price_symbols,
        fallback_fx_rate,
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
