use chrono::{NaiveDate, NaiveDateTime};
use sqlx::{FromRow, SqlitePool};

use crate::domain::portfolio::{NewTransaction, Transaction};

#[derive(FromRow)]
struct TransactionRow {
    id: i64,
    symbol: String,
    txn_type: String,
    quantity: f64,
    price: f64,
    currency: String,
    fee: f64,
    txn_date: NaiveDate,
    fx_rate_to_jpy: Option<f64>,
    notes: Option<String>,
    created_at: Option<NaiveDateTime>,
}

impl From<TransactionRow> for Transaction {
    fn from(row: TransactionRow) -> Self {
        Self {
            id: row.id,
            symbol: row.symbol,
            txn_type: row.txn_type,
            quantity: row.quantity,
            price: row.price,
            currency: row.currency,
            fee: row.fee,
            txn_date: row.txn_date,
            fx_rate_to_jpy: row.fx_rate_to_jpy,
            notes: row.notes,
            created_at: row.created_at,
        }
    }
}

pub async fn list_transactions(pool: &SqlitePool) -> Result<Vec<Transaction>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TransactionRow>(
        r#"
        SELECT id, symbol, txn_type, quantity, price, currency, fee,
               txn_date, fx_rate_to_jpy, notes, created_at
        FROM transactions
        ORDER BY txn_date DESC, id DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Transaction::from).collect())
}

pub async fn get_currency_for_symbol(
    pool: &SqlitePool,
    symbol: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT currency FROM transactions WHERE symbol = ? ORDER BY txn_date DESC, id DESC LIMIT 1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| row.0))
}

pub async fn create_transaction(
    pool: &SqlitePool,
    data: NewTransaction,
) -> Result<i64, sqlx::Error> {
    Ok(sqlx::query!(
        r#"
        INSERT INTO transactions (symbol, txn_type, quantity, price, currency, fee, txn_date, fx_rate_to_jpy, notes)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        data.symbol,
        data.txn_type,
        data.quantity,
        data.price,
        data.currency,
        data.fee,
        data.txn_date,
        data.fx_rate_to_jpy,
        data.notes
    )
    .execute(pool)
    .await?
    .last_insert_rowid())
}

pub async fn create_transactions(
    pool: &SqlitePool,
    transactions: Vec<NewTransaction>,
) -> Result<(), sqlx::Error> {
    if transactions.is_empty() {
        return Ok(());
    }

    let mut db_transaction = pool.begin().await?;
    for data in transactions {
        sqlx::query(
            r#"
            INSERT INTO transactions
                (symbol, txn_type, quantity, price, currency, fee, txn_date, fx_rate_to_jpy, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(data.symbol)
        .bind(data.txn_type)
        .bind(data.quantity)
        .bind(data.price)
        .bind(data.currency)
        .bind(data.fee)
        .bind(data.txn_date)
        .bind(data.fx_rate_to_jpy)
        .bind(data.notes)
        .execute(&mut *db_transaction)
        .await?;
    }
    db_transaction.commit().await
}

pub async fn delete_transaction(pool: &SqlitePool, id: i64) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query!("DELETE FROM transactions WHERE id = ?", id)
        .execute(pool)
        .await?
        .rows_affected())
}
