use std::collections::HashMap;
use chrono::NaiveDate;
use crate::db::transactions::Transaction;

#[derive(Debug, Clone)]
pub struct Holding {
    pub symbol: String,
    pub name: Option<String>,
    pub currency: String,
    pub quantity: f64,
    pub average_cost_native: f64,
    pub total_cost_jpy: f64,
}

pub fn calculate_holdings(transactions: &[Transaction]) -> Vec<Holding> {
    let mut holdings_map: HashMap<String, Holding> = HashMap::new();

    // Transactions are fetched in DESC order, we need to process them in ASC order
    // to calculate cost basis correctly.
    let mut sorted_txs = transactions.to_vec();
    sorted_txs.sort_by(|a, b| a.txn_date.cmp(&b.txn_date).then(a.id.cmp(&b.id)));

    for tx in &sorted_txs {
        let entry = holdings_map.entry(tx.symbol.clone()).or_insert(Holding {
            symbol: tx.symbol.clone(),
            name: None,
            currency: tx.currency.clone(),
            quantity: 0.0,
            average_cost_native: 0.0,
            total_cost_jpy: 0.0,
        });

        let fx_rate = tx.fx_rate_to_jpy.unwrap_or(1.0);
        
        // Fee handling: usually fee increases cost basis on BUY, decreases proceeds on SELL.
        let native_cost = tx.price * tx.quantity + tx.fee;
        let jpy_cost = native_cost * fx_rate;

        if tx.txn_type == "BUY" {
            let new_qty = entry.quantity + tx.quantity;
            if new_qty > 0.0 {
                let current_native_value = entry.average_cost_native * entry.quantity;
                entry.average_cost_native = (current_native_value + native_cost) / new_qty;
                
                entry.total_cost_jpy += jpy_cost;
            }
            entry.quantity = new_qty;
        } else if tx.txn_type == "SELL" {
            let new_qty = entry.quantity - tx.quantity;
            if new_qty <= 0.0 {
                entry.quantity = 0.0;
                entry.average_cost_native = 0.0;
                entry.total_cost_jpy = 0.0;
            } else {
                let proportion = new_qty / entry.quantity;
                entry.total_cost_jpy *= proportion;
                entry.quantity = new_qty;
            }
        }
    }

    let mut holdings: Vec<Holding> = holdings_map
        .into_values()
        .filter(|h| h.quantity > 0.0)
        .collect();
    // Stable, deterministic baseline order. Handlers may re-sort for display.
    holdings.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    holdings
}

pub fn calculate_holdings_as_of(transactions: &[Transaction], as_of: NaiveDate) -> Vec<Holding> {
    let filtered: Vec<Transaction> = transactions
        .iter()
        .filter(|t| t.txn_date <= as_of)
        .cloned()
        .collect();
    calculate_holdings(&filtered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_tx(id: i64, symbol: &str, txn_type: &str, qty: f64, price: f64, currency: &str, fee: f64, fx: Option<f64>) -> Transaction {
        Transaction {
            id,
            symbol: symbol.to_string(),
            txn_type: txn_type.to_string(),
            quantity: qty,
            price,
            currency: currency.to_string(),
            fee,
            txn_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fx_rate_to_jpy: fx,
            notes: None,
            created_at: None,
        }
    }

    #[test]
    fn test_calculate_holdings() {
        let txs = vec![
            make_tx(1, "AAPL", "BUY", 10.0, 150.0, "USD", 0.0, Some(140.0)),
            make_tx(2, "AAPL", "BUY", 10.0, 160.0, "USD", 0.0, Some(150.0)),
            make_tx(3, "7203.T", "BUY", 100.0, 2000.0, "JPY", 100.0, None),
        ];

        let holdings = calculate_holdings(&txs);
        assert_eq!(holdings.len(), 2);
        
        let aapl = holdings.iter().find(|h| h.symbol == "AAPL").unwrap();
        assert_eq!(aapl.quantity, 20.0);
        assert_eq!(aapl.average_cost_native, 155.0);
        assert_eq!(aapl.total_cost_jpy, 10.0 * 150.0 * 140.0 + 10.0 * 160.0 * 150.0);
        
        let toyota = holdings.iter().find(|h| h.symbol == "7203.T").unwrap();
        assert_eq!(toyota.quantity, 100.0);
        assert_eq!(toyota.average_cost_native, 2001.0); // (2000 * 100 + 100)/100
        assert_eq!(toyota.total_cost_jpy, 200100.0);
    }
}
