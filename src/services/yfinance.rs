use chrono::{NaiveDate, FixedOffset};
use sqlx::SqlitePool;
use yahoo_finance_api as yahoo;
use yahoo::time::OffsetDateTime;

use crate::db::{symbols, prices, fx, api_stats};

/// Convert a Yahoo Finance Unix timestamp to a JST date.
/// Spec §11: all dates are JST.
fn timestamp_to_jst_date(ts: i64) -> Option<NaiveDate> {
    let jst = FixedOffset::east_opt(9 * 3600)?;
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.with_timezone(&jst).date_naive())
}

pub async fn get_latest_close_price(symbol: &str) -> Result<(f64, NaiveDate), anyhow::Error> {
    let provider = yahoo::YahooConnector::new()?;
    let response = provider.get_latest_quotes(symbol, "1d").await?;
    let quote = response.last_quote()?;

    let date = timestamp_to_jst_date(quote.timestamp as i64)
        .ok_or_else(|| anyhow::anyhow!("invalid timestamp"))?;

    Ok((quote.close, date))
}

/// Fetch the short/long name for a ticker via Yahoo Finance search API
pub async fn fetch_symbol_name(symbol: &str) -> Result<Option<String>, anyhow::Error> {
    let provider = yahoo::YahooConnector::new()?;
    let result = provider.search_ticker(symbol).await?;
    
    for item in &result.quotes {
        if item.symbol == symbol {
            if !item.long_name.is_empty() {
                return Ok(Some(item.long_name.clone()));
            }
            if !item.short_name.is_empty() {
                return Ok(Some(item.short_name.clone()));
            }
        }
    }
    
    // If exact match not found, use the first result
    if let Some(item) = result.quotes.first() {
        if !item.long_name.is_empty() {
            return Ok(Some(item.long_name.clone()));
        }
        if !item.short_name.is_empty() {
            return Ok(Some(item.short_name.clone()));
        }
    }
    
    Ok(None)
}

pub async fn update_price_cache(pool: &SqlitePool, symbol: &str) -> Result<(), anyhow::Error> {
    let (price, date) = get_latest_close_price(symbol).await?;
    
    sqlx::query!(
        r#"
        INSERT INTO price_cache (symbol, date, close_price)
        VALUES (?, ?, ?)
        ON CONFLICT(symbol, date) DO UPDATE SET close_price = excluded.close_price
        "#,
        symbol,
        date,
        price
    )
    .execute(pool)
    .await?;

    let currency = if symbol.ends_with(".T") { "JPY" } else { "USD" };

    sqlx::query!(
        r#"
        INSERT INTO symbols (symbol, currency, updated_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(symbol) DO UPDATE SET updated_at = CURRENT_TIMESTAMP
        "#,
        symbol,
        currency
    )
    .execute(pool)
    .await?;

    // Fetch and store company name if not already set
    let existing = symbols::get_symbol_names(pool).await.unwrap_or_default();
    let needs_name = existing.iter()
        .find(|s| s.symbol == symbol)
        .map(|s| s.name.is_none())
        .unwrap_or(true);

    if needs_name {
        if let Ok(Some(name)) = fetch_symbol_name(symbol).await {
            let _ = symbols::update_symbol_name(pool, symbol, &name, None).await;
            tracing::info!("Updated symbol name: {} -> {}", symbol, name);
        }
    }

    Ok(())
}

fn naive_date_to_odt(date: NaiveDate) -> OffsetDateTime {
    let ts = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    OffsetDateTime::from_unix_timestamp(ts).unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

/// Fetch daily close prices from `start` through today and insert them into price_cache.
/// Existing rows are preserved (INSERT OR IGNORE).
pub async fn backfill_price_history(
    pool: &SqlitePool,
    symbol: &str,
    start: NaiveDate,
) -> Result<usize, anyhow::Error> {
    let provider = yahoo::YahooConnector::new()?;
    let start_odt = naive_date_to_odt(start);
    let end_odt = OffsetDateTime::now_utc();
    let response = provider.get_quote_history(symbol, start_odt, end_odt).await?;
    let quotes = response.quotes()?;

    let rows: Vec<(NaiveDate, f64)> = quotes
        .into_iter()
        .filter_map(|q| {
            timestamp_to_jst_date(q.timestamp).map(|d| (d, q.close))
        })
        .collect();

    let inserted = prices::bulk_insert_prices(pool, symbol, &rows).await?;
    Ok(inserted)
}

/// Fetch USDJPY=X daily history from `start` through today.
pub async fn backfill_fx_history(
    pool: &SqlitePool,
    start: NaiveDate,
) -> Result<usize, anyhow::Error> {
    let provider = yahoo::YahooConnector::new()?;
    let start_odt = naive_date_to_odt(start);
    let end_odt = OffsetDateTime::now_utc();
    let response = provider.get_quote_history("USDJPY=X", start_odt, end_odt).await?;
    let quotes = response.quotes()?;

    let rows: Vec<(NaiveDate, f64)> = quotes
        .into_iter()
        .filter_map(|q| {
            timestamp_to_jst_date(q.timestamp).map(|d| (d, q.close))
        })
        .collect();

    let inserted = fx::bulk_insert_usdjpy(pool, &rows).await?;
    Ok(inserted)
}

pub async fn update_fx_cache(pool: &SqlitePool) -> Result<(), anyhow::Error> {
    let pair = "USDJPY=X";
    let (rate, date) = get_latest_close_price(pair).await?;
    
    sqlx::query!(
        r#"
        INSERT INTO fx_cache (pair, date, rate)
        VALUES ('USDJPY', ?, ?)
        ON CONFLICT(pair, date) DO UPDATE SET rate = excluded.rate
        "#,
        date,
        rate
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Attempt to update price cache from API if rate limit allows.
/// Returns `(success, updated_from_api)`:
/// - `success = false` only when rate-limit lookup itself errored.
/// - `updated_from_api = false` when the limiter blocked the request.
///
/// The slot is claimed atomically via `try_record_api_request` BEFORE any
/// network I/O — concurrent callers cannot both succeed.
pub async fn try_update_prices_from_api(
    pool: &SqlitePool,
    symbols_to_update: &[String],
) -> (bool, bool) {
    let claimed = match api_stats::try_record_api_request(pool).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to claim API slot: {}", e);
            return (false, false);
        }
    };

    if !claimed {
        tracing::info!("API rate limit blocked the request");
        return (true, false);
    }

    // From here, we've consumed our slot — actually do the work.
    let mut success = true;
    for symbol in symbols_to_update {
        match update_price_cache(pool, symbol).await {
            Ok(()) => tracing::info!("Updated price for {} from API", symbol),
            Err(e) => {
                tracing::warn!("Failed to update price for {} from API: {}", symbol, e);
                success = false;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    if let Err(e) = update_fx_cache(pool).await {
        tracing::warn!("Failed to update FX cache from API: {}", e);
        success = false;
    }

    (success, true)
}

