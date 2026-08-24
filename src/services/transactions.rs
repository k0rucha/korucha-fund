use std::collections::HashSet;

use sqlx::SqlitePool;
use thiserror::Error;

use crate::db::transactions;
use crate::domain::portfolio::{
    LedgerValidationError, NewTransaction, Transaction, TransactionValidationError,
    validate_transaction_ledger,
};

const MAX_IMPORT_TRANSACTIONS: usize = 10_000;

#[derive(Debug)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub symbols: HashSet<String>,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("取引件数が上限の{MAX_IMPORT_TRANSACTIONS}件を超えています")]
    TooManyTransactions,
    #[error("{index}件目の取引が不正です: {source}")]
    InvalidTransaction {
        index: usize,
        #[source]
        source: TransactionValidationError,
    },
    #[error("取引履歴が不正です: {0}")]
    InvalidLedger(#[from] LedgerValidationError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
pub enum MutationError {
    #[error("取引が見つかりません")]
    NotFound,
    #[error("取引履歴が不正です: {0}")]
    InvalidLedger(#[from] LedgerValidationError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Hash, Eq, PartialEq)]
struct TransactionFingerprint {
    symbol: String,
    txn_type: String,
    txn_date: chrono::NaiveDate,
    quantity: u64,
    price: u64,
    currency: String,
    fee: u64,
    fx_rate_to_jpy: Option<u64>,
    notes: Option<String>,
}

pub async fn import(
    pool: &SqlitePool,
    imported: Vec<Transaction>,
) -> Result<ImportResult, ImportError> {
    if imported.len() > MAX_IMPORT_TRANSACTIONS {
        return Err(ImportError::TooManyTransactions);
    }

    let existing_transactions = transactions::list_transactions(pool).await?;
    let mut fingerprints: HashSet<_> = existing_transactions
        .iter()
        .map(fingerprint_transaction)
        .collect();
    let mut new_transactions = Vec::new();
    let mut symbols = HashSet::new();
    let mut skipped = 0;

    for (position, transaction) in imported.into_iter().enumerate() {
        let transaction = NewTransaction {
            symbol: transaction.symbol,
            txn_type: transaction.txn_type,
            quantity: transaction.quantity,
            price: transaction.price,
            currency: transaction.currency,
            fee: transaction.fee,
            txn_date: transaction.txn_date,
            fx_rate_to_jpy: transaction.fx_rate_to_jpy,
            notes: transaction.notes,
        }
        .normalized()
        .map_err(|source| ImportError::InvalidTransaction {
            index: position + 1,
            source,
        })?;

        if !fingerprints.insert(fingerprint_new_transaction(&transaction)) {
            skipped += 1;
            continue;
        }
        symbols.insert(transaction.symbol.clone());
        new_transactions.push(transaction);
    }

    let imported = new_transactions.len();
    let mut resulting_ledger = existing_transactions;
    let mut next_id = resulting_ledger
        .iter()
        .map(|transaction| transaction.id)
        .max()
        .unwrap_or(0)
        + 1;
    resulting_ledger.extend(new_transactions.iter().map(|transaction| {
        let row = transaction_from_new(next_id, transaction);
        next_id += 1;
        row
    }));
    validate_transaction_ledger(&resulting_ledger)?;
    transactions::create_transactions(pool, new_transactions).await?;
    Ok(ImportResult {
        imported,
        skipped,
        symbols,
    })
}

pub async fn create(pool: &SqlitePool, data: NewTransaction) -> Result<i64, MutationError> {
    let mut ledger = transactions::list_transactions(pool).await?;
    let next_id = ledger
        .iter()
        .map(|transaction| transaction.id)
        .max()
        .unwrap_or(0)
        + 1;
    ledger.push(transaction_from_new(next_id, &data));
    validate_transaction_ledger(&ledger)?;
    Ok(transactions::create_transaction(pool, data).await?)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), MutationError> {
    let mut ledger = transactions::list_transactions(pool).await?;
    let original_len = ledger.len();
    ledger.retain(|transaction| transaction.id != id);
    if ledger.len() == original_len {
        return Err(MutationError::NotFound);
    }
    validate_transaction_ledger(&ledger)?;
    transactions::delete_transaction(pool, id).await?;
    Ok(())
}

fn transaction_from_new(id: i64, transaction: &NewTransaction) -> Transaction {
    Transaction {
        id,
        symbol: transaction.symbol.clone(),
        txn_type: transaction.txn_type.clone(),
        quantity: transaction.quantity,
        price: transaction.price,
        currency: transaction.currency.clone(),
        fee: transaction.fee,
        txn_date: transaction.txn_date,
        fx_rate_to_jpy: transaction.fx_rate_to_jpy,
        notes: transaction.notes.clone(),
        created_at: None,
    }
}

fn fingerprint_transaction(transaction: &Transaction) -> TransactionFingerprint {
    TransactionFingerprint {
        symbol: transaction.symbol.clone(),
        txn_type: transaction.txn_type.clone(),
        txn_date: transaction.txn_date,
        quantity: transaction.quantity.to_bits(),
        price: transaction.price.to_bits(),
        currency: transaction.currency.clone(),
        fee: transaction.fee.to_bits(),
        fx_rate_to_jpy: transaction.fx_rate_to_jpy.map(f64::to_bits),
        notes: transaction.notes.clone(),
    }
}

fn fingerprint_new_transaction(transaction: &NewTransaction) -> TransactionFingerprint {
    TransactionFingerprint {
        symbol: transaction.symbol.clone(),
        txn_type: transaction.txn_type.clone(),
        txn_date: transaction.txn_date,
        quantity: transaction.quantity.to_bits(),
        price: transaction.price.to_bits(),
        currency: transaction.currency.clone(),
        fee: transaction.fee.to_bits(),
        fx_rate_to_jpy: transaction.fx_rate_to_jpy.map(f64::to_bits),
        notes: transaction.notes.clone(),
    }
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

    #[tokio::test]
    async fn import_rejects_invalid_transactions_without_partial_writes() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let valid = Transaction {
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
        let mut invalid = valid.clone();
        invalid.price = f64::NAN;

        let error = import(&pool, vec![valid, invalid]).await.unwrap_err();
        assert!(matches!(
            error,
            ImportError::InvalidTransaction { index: 2, .. }
        ));
        assert!(
            transactions::list_transactions(&pool)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn mutation_preserves_a_valid_chronological_ledger() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let buy = NewTransaction {
            symbol: "AAPL".into(),
            txn_type: "BUY".into(),
            quantity: 10.0,
            price: 100.0,
            currency: "USD".into(),
            fee: 0.0,
            txn_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fx_rate_to_jpy: Some(150.0),
            notes: None,
        };
        let buy_id = create(&pool, buy.clone()).await.unwrap();
        let mut oversell = buy;
        oversell.txn_type = "SELL".into();
        oversell.quantity = 11.0;
        oversell.txn_date = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();

        assert!(matches!(
            create(&pool, oversell.clone()).await,
            Err(MutationError::InvalidLedger(_))
        ));
        oversell.quantity = 5.0;
        create(&pool, oversell).await.unwrap();
        assert!(matches!(
            delete(&pool, buy_id).await,
            Err(MutationError::InvalidLedger(_))
        ));
    }

    #[tokio::test]
    async fn database_rejects_invalid_direct_transaction_writes() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let result = sqlx::query(
            r#"
            INSERT INTO transactions
                (symbol, txn_type, quantity, price, currency, fee, txn_date, fx_rate_to_jpy)
            VALUES ('7203.T', 'BUY', 1, 3000, 'JPY', 0, '2026-01-01', 150)
            "#,
        )
        .execute(&pool)
        .await;

        assert!(result.is_err());
    }
}
