use sqlx::{FromRow, SqlitePool};
use chrono::NaiveDate;
use serde::{Serialize, Deserialize};

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub symbol: String,
    pub txn_type: String, // 'BUY' | 'SELL'
    pub quantity: f64,
    pub price: f64,
    pub currency: String, // 'JPY' | 'USD'
    pub fee: f64,
    pub txn_date: NaiveDate,
    pub fx_rate_to_jpy: Option<f64>,
    pub notes: Option<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct CreateTransaction {
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

pub async fn list_transactions(pool: &SqlitePool) -> Result<Vec<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>("SELECT * FROM transactions ORDER BY txn_date DESC, id DESC")
        .fetch_all(pool)
        .await
}

pub async fn create_transaction(pool: &SqlitePool, data: CreateTransaction) -> Result<i64, sqlx::Error> {
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

pub async fn delete_transaction(pool: &SqlitePool, id: i64) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query!("DELETE FROM transactions WHERE id = ?", id)
        .execute(pool)
        .await?
        .rows_affected())
}
