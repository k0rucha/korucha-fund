use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_SYMBOL_CHARS: usize = 32;
const MAX_NOTES_CHARS: usize = 1_000;

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

#[derive(Debug, Error, PartialEq)]
pub enum TransactionValidationError {
    #[error("銘柄シンボルを入力してください")]
    EmptySymbol,
    #[error("銘柄シンボルは{MAX_SYMBOL_CHARS}文字以内で入力してください")]
    SymbolTooLong,
    #[error("銘柄シンボルに使用できない文字が含まれています")]
    InvalidSymbol,
    #[error("取引タイプはBUYまたはSELLを指定してください")]
    InvalidTransactionType,
    #[error("数量は0より大きい有限の数値を指定してください")]
    InvalidQuantity,
    #[error("単価は0より大きい有限の数値を指定してください")]
    InvalidPrice,
    #[error("通貨はJPYまたはUSDを指定してください")]
    InvalidCurrency,
    #[error("手数料は0以上の有限の数値を指定してください")]
    InvalidFee,
    #[error("USD取引には0より大きいUSD/JPY換算レートが必要です")]
    InvalidFxRate,
    #[error("メモは{MAX_NOTES_CHARS}文字以内で入力してください")]
    NotesTooLong,
}

#[derive(Debug, Error, PartialEq)]
pub enum LedgerValidationError {
    #[error("取引ID {id} の種別が不正です")]
    InvalidTransactionType { id: i64 },
    #[error("{symbol} の取引通貨が混在しています（{expected} と {actual}）")]
    CurrencyMismatch {
        symbol: String,
        expected: String,
        actual: String,
    },
    #[error("{symbol} の売却数 {requested} が約定日時点の保有数 {available} を超えています")]
    InsufficientQuantity {
        symbol: String,
        available: f64,
        requested: f64,
    },
}

impl NewTransaction {
    /// Normalize user/imported input and enforce the invariants expected by
    /// portfolio calculations. Keeping this at the domain boundary ensures
    /// every write path applies the same rules.
    pub fn normalized(mut self) -> Result<Self, TransactionValidationError> {
        self.symbol = normalize_symbol(&self.symbol)?;
        self.txn_type = self.txn_type.trim().to_ascii_uppercase();
        self.currency = self.currency.trim().to_ascii_uppercase();
        self.notes = self
            .notes
            .take()
            .map(|notes| notes.trim().to_string())
            .filter(|notes| !notes.is_empty());

        if !matches!(self.txn_type.as_str(), "BUY" | "SELL") {
            return Err(TransactionValidationError::InvalidTransactionType);
        }
        if !self.quantity.is_finite() || self.quantity <= 0.0 {
            return Err(TransactionValidationError::InvalidQuantity);
        }
        if !self.price.is_finite() || self.price <= 0.0 {
            return Err(TransactionValidationError::InvalidPrice);
        }
        if !self.fee.is_finite() || self.fee < 0.0 {
            return Err(TransactionValidationError::InvalidFee);
        }
        if !matches!(self.currency.as_str(), "JPY" | "USD") {
            return Err(TransactionValidationError::InvalidCurrency);
        }
        if self.currency == "USD"
            && !self
                .fx_rate_to_jpy
                .is_some_and(|rate| rate.is_finite() && rate > 0.0)
        {
            return Err(TransactionValidationError::InvalidFxRate);
        }
        // An FX rate on a JPY transaction is meaningless and previously made
        // the calculation multiply yen values by that rate. Canonicalize it.
        if self.currency == "JPY" {
            self.fx_rate_to_jpy = None;
        }
        if self
            .notes
            .as_ref()
            .is_some_and(|notes| notes.chars().count() > MAX_NOTES_CHARS)
        {
            return Err(TransactionValidationError::NotesTooLong);
        }

        Ok(self)
    }
}

pub fn normalize_symbol(symbol: &str) -> Result<String, TransactionValidationError> {
    let symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty() {
        return Err(TransactionValidationError::EmptySymbol);
    }
    if symbol.chars().count() > MAX_SYMBOL_CHARS {
        return Err(TransactionValidationError::SymbolTooLong);
    }
    if !symbol.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '^' | '=' | '_')
    }) {
        return Err(TransactionValidationError::InvalidSymbol);
    }
    Ok(symbol)
}

/// Validate constraints that depend on the complete chronological ledger.
/// This is intentionally separate from per-row validation: inserting a
/// backdated sale or deleting an old buy can invalidate later transactions.
pub fn validate_transaction_ledger(
    transactions: &[Transaction],
) -> Result<(), LedgerValidationError> {
    let mut ordered: Vec<_> = transactions.iter().collect();
    ordered.sort_by_key(|transaction| (transaction.txn_date, transaction.id));
    let mut balances = HashMap::<String, (String, f64)>::new();

    for transaction in ordered {
        if !matches!(transaction.txn_type.as_str(), "BUY" | "SELL") {
            return Err(LedgerValidationError::InvalidTransactionType { id: transaction.id });
        }
        let entry = balances
            .entry(transaction.symbol.clone())
            .or_insert_with(|| (transaction.currency.clone(), 0.0));
        if entry.0 != transaction.currency {
            return Err(LedgerValidationError::CurrencyMismatch {
                symbol: transaction.symbol.clone(),
                expected: entry.0.clone(),
                actual: transaction.currency.clone(),
            });
        }

        if transaction.txn_type == "BUY" {
            entry.1 += transaction.quantity;
        } else {
            let tolerance = f64::EPSILON * entry.1.abs().max(transaction.quantity.abs()) * 8.0;
            if transaction.quantity - entry.1 > tolerance {
                return Err(LedgerValidationError::InsufficientQuantity {
                    symbol: transaction.symbol.clone(),
                    available: entry.1,
                    requested: transaction.quantity,
                });
            }
            entry.1 = (entry.1 - transaction.quantity).max(0.0);
        }
    }

    Ok(())
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
    if analysis.holdings.iter().any(|holding| {
        !prices
            .get(&holding.symbol)
            .is_some_and(|price| price.is_finite() && *price > 0.0)
    }) {
        return None;
    }
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
        let current_price_native = prices[&holding.symbol];
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

    #[test]
    fn valuation_requires_a_valid_price_for_every_holding() {
        let analysis = analyze_transactions(&[transaction(1, "BUY", 10.0, 100.0)]);

        assert!(value_portfolio(analysis.clone(), &HashMap::new(), Some(150.0)).is_none());
        assert!(
            value_portfolio(
                analysis,
                &HashMap::from([("AAPL".into(), f64::NAN)]),
                Some(150.0)
            )
            .is_none()
        );
    }

    #[test]
    fn normalizes_and_validates_transaction_input() {
        let normalized = NewTransaction {
            symbol: " aapl ".into(),
            txn_type: " buy ".into(),
            quantity: 1.0,
            price: 100.0,
            currency: " usd ".into(),
            fee: 0.0,
            txn_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fx_rate_to_jpy: Some(150.0),
            notes: Some(" memo ".into()),
        }
        .normalized()
        .unwrap();

        assert_eq!(normalized.symbol, "AAPL");
        assert_eq!(normalized.txn_type, "BUY");
        assert_eq!(normalized.currency, "USD");
        assert_eq!(normalized.notes.as_deref(), Some("memo"));

        let mut invalid = normalized;
        invalid.quantity = f64::NAN;
        assert_eq!(
            invalid.normalized().unwrap_err(),
            TransactionValidationError::InvalidQuantity
        );
    }

    #[test]
    fn ignores_fx_rate_for_jpy_transactions() {
        let transaction = NewTransaction {
            symbol: "7203.t".into(),
            txn_type: "BUY".into(),
            quantity: 1.0,
            price: 3_000.0,
            currency: "JPY".into(),
            fee: 0.0,
            txn_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fx_rate_to_jpy: Some(150.0),
            notes: None,
        }
        .normalized()
        .unwrap();

        assert_eq!(transaction.symbol, "7203.T");
        assert_eq!(transaction.fx_rate_to_jpy, None);
    }

    #[test]
    fn ledger_rejects_overselling_and_mixed_currencies() {
        let buy = transaction(1, "BUY", 10.0, 100.0);
        let oversell = transaction(2, "SELL", 11.0, 110.0);
        assert!(matches!(
            validate_transaction_ledger(&[buy.clone(), oversell]),
            Err(LedgerValidationError::InsufficientQuantity { .. })
        ));

        let mut mixed_currency = transaction(2, "BUY", 1.0, 110.0);
        mixed_currency.currency = "JPY".into();
        assert!(matches!(
            validate_transaction_ledger(&[buy, mixed_currency]),
            Err(LedgerValidationError::CurrencyMismatch { .. })
        ));
    }
}
