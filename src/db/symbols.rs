use sqlx::{FromRow, SqlitePool};

#[derive(Debug, FromRow, Clone)]
pub struct Symbol {
    pub symbol: String,
    pub name: Option<String>,
    pub currency: String,
    pub exchange: Option<String>,
}

pub async fn get_symbol_names(pool: &SqlitePool) -> Result<Vec<Symbol>, sqlx::Error> {
    sqlx::query_as::<_, Symbol>("SELECT symbol, name, currency, exchange FROM symbols")
        .fetch_all(pool)
        .await
}

pub async fn get_symbol_name(
    pool: &SqlitePool,
    symbol: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT name FROM symbols WHERE symbol = ?")
            .bind(symbol)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|r| r.0))
}

pub async fn update_symbol_name(
    pool: &SqlitePool,
    symbol: &str,
    name: &str,
    exchange: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE symbols SET name = ?, exchange = ?, updated_at = CURRENT_TIMESTAMP
        WHERE symbol = ?
        "#,
    )
    .bind(name)
    .bind(exchange)
    .bind(symbol)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_symbol(
    pool: &SqlitePool,
    symbol: &str,
    name: Option<&str>,
    currency: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO symbols (symbol, name, currency, updated_at)
        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(symbol) DO UPDATE SET
            name = COALESCE(excluded.name, symbols.name),
            currency = excluded.currency,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(symbol)
    .bind(name)
    .bind(currency)
    .execute(pool)
    .await?;
    Ok(())
}
