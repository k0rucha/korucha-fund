use sqlx::{SqlitePool, FromRow};
use chrono::NaiveDateTime;

#[derive(Debug, FromRow, Clone)]
pub struct TickerShareCard {
    pub id: String,
    pub created_at: NaiveDateTime,
    pub symbol: String,
    pub display_name: Option<String>,
    pub currency: String,
    pub issue_price_native: f64,
    pub fx_rate_at_issue: Option<f64>,
    pub quantity: Option<f64>,
    pub avg_cost_native: Option<f64>,
    pub issue_value_jpy: Option<f64>,
    pub issue_pnl_jpy: Option<f64>,
    pub default_span: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_ticker_share_card(
    pool: &SqlitePool,
    id: &str,
    symbol: &str,
    display_name: Option<&str>,
    currency: &str,
    issue_price_native: f64,
    fx_rate_at_issue: Option<f64>,
    quantity: Option<f64>,
    avg_cost_native: Option<f64>,
    issue_value_jpy: Option<f64>,
    issue_pnl_jpy: Option<f64>,
    default_span: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO ticker_share_cards (
            id, symbol, display_name, currency,
            issue_price_native, fx_rate_at_issue,
            quantity, avg_cost_native, issue_value_jpy, issue_pnl_jpy,
            default_span
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(symbol)
    .bind(display_name)
    .bind(currency)
    .bind(issue_price_native)
    .bind(fx_rate_at_issue)
    .bind(quantity)
    .bind(avg_cost_native)
    .bind(issue_value_jpy)
    .bind(issue_pnl_jpy)
    .bind(default_span)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_ticker_share_card(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<TickerShareCard>, sqlx::Error> {
    sqlx::query_as::<_, TickerShareCard>(
        r#"
        SELECT id, created_at, symbol, display_name, currency,
               issue_price_native, fx_rate_at_issue,
               quantity, avg_cost_native, issue_value_jpy, issue_pnl_jpy,
               default_span
        FROM ticker_share_cards WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
