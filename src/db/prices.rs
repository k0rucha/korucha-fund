use chrono::NaiveDate;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, FromRow)]
pub struct PriceCache {
    pub symbol: String,
    pub date: NaiveDate,
    pub close_price: f64,
}

pub async fn get_latest_prices(pool: &SqlitePool) -> Result<Vec<PriceCache>, sqlx::Error> {
    sqlx::query_as::<_, PriceCache>(
        r#"
        SELECT symbol, date, close_price
        FROM price_cache
        WHERE date = (SELECT MAX(date) FROM price_cache p2 WHERE p2.symbol = price_cache.symbol)
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn upsert_price(
    pool: &SqlitePool,
    symbol: &str,
    date: NaiveDate,
    close_price: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO price_cache (symbol, date, close_price)
        VALUES (?, ?, ?)
        ON CONFLICT(symbol, date) DO UPDATE SET close_price = excluded.close_price
        "#,
    )
    .bind(symbol)
    .bind(date)
    .bind(close_price)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn bulk_insert_prices(
    pool: &SqlitePool,
    symbol: &str,
    rows: &[(NaiveDate, f64)],
) -> Result<usize, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }
    // One transaction for the whole batch — backfills can run hundreds of
    // rows; without this each INSERT pays its own fsync.
    let mut tx = pool.begin().await?;
    let mut inserted = 0usize;
    for (date, close) in rows {
        let res = sqlx::query(
            "INSERT OR IGNORE INTO price_cache (symbol, date, close_price) VALUES (?, ?, ?)",
        )
        .bind(symbol)
        .bind(date)
        .bind(close)
        .execute(&mut *tx)
        .await?;
        inserted += res.rows_affected() as usize;
    }
    tx.commit().await?;
    Ok(inserted)
}

pub async fn get_price_on_or_before(
    pool: &SqlitePool,
    symbol: &str,
    date: NaiveDate,
) -> Result<Option<f64>, sqlx::Error> {
    let row: Option<(f64,)> = sqlx::query_as(
        "SELECT close_price FROM price_cache WHERE symbol = ? AND date <= ? ORDER BY date DESC LIMIT 1"
    )
    .bind(symbol)
    .bind(date)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// Count cached price rows for `symbol` whose date is on or after `since`.
/// Used to decide whether a fresh history backfill is worth a yfinance call.
pub async fn count_history_since(
    pool: &SqlitePool,
    symbol: &str,
    since: NaiveDate,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM price_cache WHERE symbol = ? AND date >= ?")
            .bind(symbol)
            .bind(since)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Fetch all (date, close) pairs for `symbol` on or before `until`,
/// ordered ascending — used for charting a ticker's history.
pub async fn list_history(
    pool: &SqlitePool,
    symbol: &str,
    until: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>, sqlx::Error> {
    let rows: Vec<(NaiveDate, f64)> = sqlx::query_as(
        r#"
        SELECT date, close_price FROM price_cache
        WHERE symbol = ? AND date <= ?
        ORDER BY date ASC
        "#,
    )
    .bind(symbol)
    .bind(until)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
