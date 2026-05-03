use sqlx::{SqlitePool, FromRow};
use chrono::NaiveDate;

#[derive(Debug, FromRow, Clone)]
pub struct Snapshot {
    pub date: NaiveDate,
    pub total_value_jpy: f64,
    pub total_cost_jpy: f64,
    pub unrealized_pnl_jpy: f64,
}

pub async fn list_snapshots(pool: &SqlitePool) -> Result<Vec<Snapshot>, sqlx::Error> {
    sqlx::query_as::<_, Snapshot>(
        "SELECT date, total_value_jpy, total_cost_jpy, unrealized_pnl_jpy FROM snapshots ORDER BY date ASC"
    )
    .fetch_all(pool)
    .await
}

pub async fn get_snapshot_on_or_before(
    pool: &SqlitePool,
    date: NaiveDate,
) -> Result<Option<Snapshot>, sqlx::Error> {
    sqlx::query_as::<_, Snapshot>(
        "SELECT date, total_value_jpy, total_cost_jpy, unrealized_pnl_jpy
         FROM snapshots WHERE date <= ? ORDER BY date DESC LIMIT 1"
    )
    .bind(date)
    .fetch_optional(pool)
    .await
}

pub async fn upsert_snapshot(pool: &SqlitePool, date: NaiveDate, total_value: f64, total_cost: f64, pnl: f64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO snapshots (date, total_value_jpy, total_cost_jpy, unrealized_pnl_jpy)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(date) DO UPDATE SET
            total_value_jpy = excluded.total_value_jpy,
            total_cost_jpy = excluded.total_cost_jpy,
            unrealized_pnl_jpy = excluded.unrealized_pnl_jpy
        "#
    )
    .bind(date)
    .bind(total_value)
    .bind(total_cost)
    .bind(pnl)
    .execute(pool)
    .await?;
    Ok(())
}
