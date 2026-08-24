use chrono::NaiveDateTime;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, FromRow, Clone)]
pub struct ShareCard {
    pub id: String,
    pub created_at: NaiveDateTime,
    pub total_value_jpy: f64,
    pub total_cost_jpy: f64,
    pub unrealized_pnl_jpy: f64,
    pub holdings_json: String,
    pub default_span: String,
}

pub async fn insert_share_card(
    pool: &SqlitePool,
    id: &str,
    total_value_jpy: f64,
    total_cost_jpy: f64,
    unrealized_pnl_jpy: f64,
    holdings_json: &str,
    default_span: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO share_cards (id, total_value_jpy, total_cost_jpy, unrealized_pnl_jpy, holdings_json, default_span)
        VALUES (?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(id)
    .bind(total_value_jpy)
    .bind(total_cost_jpy)
    .bind(unrealized_pnl_jpy)
    .bind(holdings_json)
    .bind(default_span)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_share_card(pool: &SqlitePool, id: &str) -> Result<Option<ShareCard>, sqlx::Error> {
    sqlx::query_as::<_, ShareCard>(
        "SELECT id, created_at, total_value_jpy, total_cost_jpy, unrealized_pnl_jpy, holdings_json, default_span
         FROM share_cards WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
