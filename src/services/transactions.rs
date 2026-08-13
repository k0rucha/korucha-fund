use std::collections::HashSet;

use sqlx::SqlitePool;

use crate::db::transactions;
use crate::domain::portfolio::{NewTransaction, Transaction};

pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub symbols: HashSet<String>,
}

pub async fn import(pool: &SqlitePool, imported: Vec<Transaction>) -> sqlx::Result<ImportResult> {
    let mut fingerprints: HashSet<_> = transactions::list_transactions(pool)
        .await?
        .iter()
        .map(fingerprint)
        .collect();
    let mut new_transactions = Vec::new();
    let mut symbols = HashSet::new();
    let mut skipped = 0;

    for transaction in imported {
        if !fingerprints.insert(fingerprint(&transaction)) {
            skipped += 1;
            continue;
        }
        symbols.insert(transaction.symbol.clone());
        new_transactions.push(NewTransaction {
            symbol: transaction.symbol,
            txn_type: transaction.txn_type,
            quantity: transaction.quantity,
            price: transaction.price,
            currency: transaction.currency,
            fee: transaction.fee,
            txn_date: transaction.txn_date,
            fx_rate_to_jpy: transaction.fx_rate_to_jpy,
            notes: transaction.notes,
        });
    }

    let imported = new_transactions.len();
    transactions::create_transactions(pool, new_transactions).await?;
    Ok(ImportResult {
        imported,
        skipped,
        symbols,
    })
}

fn fingerprint(transaction: &Transaction) -> String {
    format!(
        "{}|{}|{}|{:.6}|{:.6}|{:.6}",
        transaction.symbol,
        transaction.txn_type,
        transaction.txn_date,
        transaction.quantity,
        transaction.price,
        transaction.fee,
    )
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn import_is_idempotent() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let transaction = Transaction {
            id: 1,
            symbol: "7203.T".into(),
            txn_type: "BUY".into(),
            quantity: 10.0,
            price: 2_000.0,
            currency: "JPY".into(),
            fee: 0.0,
            txn_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fx_rate_to_jpy: None,
            notes: None,
            created_at: None,
        };

        let first = import(&pool, vec![transaction.clone(), transaction.clone()])
            .await
            .unwrap();
        assert_eq!((first.imported, first.skipped), (1, 1));

        let second = import(&pool, vec![transaction]).await.unwrap();
        assert_eq!((second.imported, second.skipped), (0, 1));
        assert_eq!(
            transactions::list_transactions(&pool).await.unwrap().len(),
            1
        );
    }
}
