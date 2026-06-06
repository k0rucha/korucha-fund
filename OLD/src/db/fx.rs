use sqlx::SqlitePool;
use chrono::NaiveDate;

pub async fn get_latest_usdjpy(pool: &SqlitePool) -> Result<Option<f64>, sqlx::Error> {
    let row: Option<(f64,)> = sqlx::query_as(
        r#"
        SELECT rate FROM fx_cache WHERE pair = 'USDJPY' ORDER BY date DESC LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.0))
}

pub async fn bulk_insert_usdjpy(
    pool: &SqlitePool,
    rows: &[(NaiveDate, f64)],
) -> Result<usize, sqlx::Error> {
    if rows.is_empty() {
        return Ok(0);
    }
    let mut tx = pool.begin().await?;
    let mut inserted = 0usize;
    for (date, rate) in rows {
        let res = sqlx::query(
            "INSERT OR IGNORE INTO fx_cache (pair, date, rate) VALUES ('USDJPY', ?, ?)"
        )
        .bind(date)
        .bind(rate)
        .execute(&mut *tx)
        .await?;
        inserted += res.rows_affected() as usize;
    }
    tx.commit().await?;
    Ok(inserted)
}

pub async fn get_usdjpy_on_or_before(
    pool: &SqlitePool,
    date: NaiveDate,
) -> Result<Option<f64>, sqlx::Error> {
    let row: Option<(f64,)> = sqlx::query_as(
        "SELECT rate FROM fx_cache WHERE pair = 'USDJPY' AND date <= ? ORDER BY date DESC LIMIT 1"
    )
    .bind(date)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}
