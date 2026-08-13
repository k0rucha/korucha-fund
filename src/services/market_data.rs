use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::clients::yahoo;
use crate::db::{api_stats, fx, prices, symbols};

pub async fn update_price_cache(pool: &SqlitePool, symbol: &str) -> anyhow::Result<()> {
    let (price, date) = yahoo::latest_close(symbol).await?;
    prices::upsert_price(pool, symbol, date, price).await?;

    let currency = if symbol.ends_with(".T") { "JPY" } else { "USD" };
    symbols::upsert_symbol(pool, symbol, None, currency).await?;
    if symbols::get_symbol_name(pool, symbol).await?.is_none() {
        match yahoo::symbol_name(symbol).await {
            Ok(Some(name)) => {
                symbols::update_symbol_name(pool, symbol, &name, None).await?;
                tracing::info!(%symbol, %name, "updated symbol name");
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(%symbol, %error, "symbol lookup failed"),
        }
    }
    Ok(())
}

pub async fn lookup_symbol_name(symbol: &str) -> anyhow::Result<Option<String>> {
    yahoo::symbol_name(symbol).await
}

pub async fn backfill_price_history(
    pool: &SqlitePool,
    symbol: &str,
    start: NaiveDate,
) -> anyhow::Result<usize> {
    let rows = yahoo::daily_closes(symbol, start).await?;
    Ok(prices::bulk_insert_prices(pool, symbol, &rows).await?)
}

pub async fn backfill_fx_history(pool: &SqlitePool, start: NaiveDate) -> anyhow::Result<usize> {
    let rows = yahoo::daily_closes("USDJPY=X", start).await?;
    Ok(fx::bulk_insert_usdjpy(pool, &rows).await?)
}

pub async fn update_fx_cache(pool: &SqlitePool) -> anyhow::Result<()> {
    let (rate, date) = yahoo::latest_close("USDJPY=X").await?;
    fx::upsert_usdjpy(pool, date, rate).await?;
    Ok(())
}

pub async fn try_update_prices_from_api(
    pool: &SqlitePool,
    symbols_to_update: &[String],
) -> (bool, bool) {
    let claimed = match api_stats::try_record_api_request(pool).await {
        Ok(claimed) => claimed,
        Err(error) => {
            tracing::error!(%error, "failed to claim API slot");
            return (false, false);
        }
    };
    if !claimed {
        tracing::info!("API rate limit blocked the request");
        return (true, false);
    }

    let mut success = true;
    for symbol in symbols_to_update {
        if let Err(error) = update_price_cache(pool, symbol).await {
            tracing::warn!(%symbol, %error, "price update failed");
            success = false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    if let Err(error) = update_fx_cache(pool).await {
        tracing::warn!(%error, "FX update failed");
        success = false;
    }

    (success, true)
}
