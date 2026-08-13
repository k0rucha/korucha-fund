use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub symbol: String,
    pub txn_type: String,
    pub quantity: f64,
    pub price: f64,
    pub currency: String,
    pub fee: f64,
    pub txn_date: NaiveDate,
    pub fx_rate_to_jpy: Option<f64>,
    pub notes: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct NewTransaction {
    pub symbol: String,
    pub txn_type: String,
    pub quantity: f64,
    pub price: f64,
    pub currency: String,
    pub fee: f64,
    pub txn_date: NaiveDate,
    pub fx_rate_to_jpy: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Holding {
    pub symbol: String,
    pub currency: String,
    pub quantity: f64,
    pub average_cost_native: f64,
    pub total_cost_jpy: f64,
}

#[derive(Debug, Clone)]
pub struct PortfolioAnalysis {
    pub holdings: Vec<Holding>,
    pub realized_pnl_jpy: f64,
}

#[derive(Debug, Clone)]
pub struct ValuedHolding {
    pub symbol: String,
    pub currency: String,
    pub quantity: f64,
    pub average_cost_native: f64,
    pub total_cost_jpy: f64,
    pub current_price_native: f64,
    pub current_value_jpy: f64,
    pub unrealized_pnl_jpy: f64,
    pub unrealized_pnl_pct: f64,
}

#[derive(Debug, Clone)]
pub struct Portfolio {
    pub holdings: Vec<ValuedHolding>,
    pub total_cost_jpy: f64,
    pub total_value_jpy: f64,
    pub unrealized_pnl_jpy: f64,
    pub unrealized_pnl_pct: f64,
    pub realized_pnl_jpy: f64,
}

pub fn analyze_transactions(transactions: &[Transaction]) -> PortfolioAnalysis {
    analyze(transactions, None)
}

pub fn analyze_transactions_as_of(
    transactions: &[Transaction],
    date: NaiveDate,
) -> PortfolioAnalysis {
    analyze(transactions, Some(date))
}

fn analyze(transactions: &[Transaction], as_of: Option<NaiveDate>) -> PortfolioAnalysis {
    let mut sorted: Vec<&Transaction> = transactions
        .iter()
        .filter(|transaction| as_of.is_none_or(|date| transaction.txn_date <= date))
        .collect();
    sorted.sort_by_key(|transaction| (transaction.txn_date, transaction.id));

    let mut holdings = HashMap::<String, Holding>::new();
    let mut realized_pnl_jpy = 0.0;

    for transaction in sorted {
        let holding = holdings
            .entry(transaction.symbol.clone())
            .or_insert_with(|| Holding {
                symbol: transaction.symbol.clone(),
                currency: transaction.currency.clone(),
                quantity: 0.0,
                average_cost_native: 0.0,
                total_cost_jpy: 0.0,
            });
        let fx_rate = transaction.fx_rate_to_jpy.unwrap_or(1.0);

        match transaction.txn_type.as_str() {
            "BUY" => {
                let native_cost = transaction.price * transaction.quantity + transaction.fee;
                let new_quantity = holding.quantity + transaction.quantity;
                if new_quantity > 0.0 {
                    holding.average_cost_native = (holding.average_cost_native * holding.quantity
                        + native_cost)
                        / new_quantity;
                    holding.total_cost_jpy += native_cost * fx_rate;
                }
                holding.quantity = new_quantity;
            }
            "SELL" if holding.quantity > 0.0 => {
                let sold_quantity = transaction.quantity.min(holding.quantity);
                let allocated_cost = holding.total_cost_jpy * sold_quantity / holding.quantity;
                let allocated_fee = if transaction.quantity > 0.0 {
                    transaction.fee * sold_quantity / transaction.quantity
                } else {
                    0.0
                };
                let proceeds = (transaction.price * sold_quantity - allocated_fee) * fx_rate;
                realized_pnl_jpy += proceeds - allocated_cost;

                holding.quantity = (holding.quantity - sold_quantity).max(0.0);
                holding.total_cost_jpy -= allocated_cost;
                if holding.quantity == 0.0 {
                    holding.average_cost_native = 0.0;
                    holding.total_cost_jpy = 0.0;
                }
            }
            _ => {}
        }
    }

    let mut holdings: Vec<_> = holdings
        .into_values()
        .filter(|holding| holding.quantity > 0.0)
        .collect();
    holdings.sort_by(|left, right| left.symbol.cmp(&right.symbol));

    PortfolioAnalysis {
        holdings,
        realized_pnl_jpy,
    }
}

pub fn value_portfolio(
    analysis: PortfolioAnalysis,
    prices: &HashMap<String, f64>,
    usd_jpy: Option<f64>,
) -> Option<Portfolio> {
    if usd_jpy.is_none()
        && analysis
            .holdings
            .iter()
            .any(|holding| holding.currency == "USD")
    {
        return None;
    }

    let usd_jpy = usd_jpy.unwrap_or(1.0);
    let mut total_cost_jpy = 0.0;
    let mut total_value_jpy = 0.0;
    let mut holdings = Vec::with_capacity(analysis.holdings.len());

    for holding in analysis.holdings {
        let current_price_native = prices.get(&holding.symbol).copied().unwrap_or(0.0);
        let fx_rate = if holding.currency == "USD" {
            usd_jpy
        } else {
            1.0
        };
        let current_value_jpy = holding.quantity * current_price_native * fx_rate;
        let unrealized_pnl_jpy = current_value_jpy - holding.total_cost_jpy;
        let unrealized_pnl_pct = percentage(unrealized_pnl_jpy, holding.total_cost_jpy);

        total_cost_jpy += holding.total_cost_jpy;
        total_value_jpy += current_value_jpy;
        holdings.push(ValuedHolding {
            symbol: holding.symbol,
            currency: holding.currency,
            quantity: holding.quantity,
            average_cost_native: holding.average_cost_native,
            total_cost_jpy: holding.total_cost_jpy,
            current_price_native,
            current_value_jpy,
            unrealized_pnl_jpy,
            unrealized_pnl_pct,
        });
    }

    holdings.sort_by(|left, right| {
        right
            .current_value_jpy
            .total_cmp(&left.current_value_jpy)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    let unrealized_pnl_jpy = total_value_jpy - total_cost_jpy;

    Some(Portfolio {
        holdings,
        total_cost_jpy,
        total_value_jpy,
        unrealized_pnl_jpy,
        unrealized_pnl_pct: percentage(unrealized_pnl_jpy, total_cost_jpy),
        realized_pnl_jpy: analysis.realized_pnl_jpy,
    })
}

pub fn percentage(delta: f64, base: f64) -> f64 {
    if base > 0.0 {
        delta / base * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transaction(id: i64, txn_type: &str, quantity: f64, price: f64) -> Transaction {
        Transaction {
            id,
            symbol: "AAPL".into(),
            txn_type: txn_type.into(),
            quantity,
            price,
            currency: "USD".into(),
            fee: 0.0,
            txn_date: NaiveDate::from_ymd_opt(2026, 1, id as u32).unwrap(),
            fx_rate_to_jpy: Some(150.0),
            notes: None,
            created_at: None,
        }
    }

    #[test]
    fn analyzes_and_values_the_portfolio() {
        let transactions = vec![
            transaction(1, "BUY", 10.0, 100.0),
            transaction(2, "BUY", 10.0, 120.0),
            transaction(3, "SELL", 5.0, 130.0),
        ];

        let analysis = analyze_transactions(&transactions);
        assert_eq!(analysis.holdings[0].quantity, 15.0);
        assert_eq!(analysis.holdings[0].average_cost_native, 110.0);
        assert_eq!(analysis.holdings[0].total_cost_jpy, 247_500.0);
        assert_eq!(analysis.realized_pnl_jpy, 15_000.0);

        let prices = HashMap::from([("AAPL".into(), 140.0)]);
        let portfolio = value_portfolio(analysis.clone(), &prices, Some(150.0)).unwrap();
        assert_eq!(portfolio.total_value_jpy, 315_000.0);
        assert_eq!(portfolio.unrealized_pnl_jpy, 67_500.0);
        assert!(value_portfolio(analysis, &prices, None).is_none());
    }
}
