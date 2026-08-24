use anyhow::Context;
use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::clients::yahoo;
use crate::db::{api_stats, fx, prices, symbols, transactions};

pub async fn update_price_cache(pool: &SqlitePool, symbol: &str) -> anyhow::Result<()> {
    let (price, date) = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        yahoo::latest_close(symbol),
    )
    .await
    .context("latest quote request timed out")??;
    prices::upsert_price(pool, symbol, date, price).await?;

    let currency = transactions::get_currency_for_symbol(pool, symbol)
        .await?
        .unwrap_or_else(|| {
            if symbol.ends_with(".T") {
                "JPY".into()
            } else {
                "USD".into()
            }
        });
    symbols::upsert_symbol(pool, symbol, None, &currency).await?;
    if symbols::get_symbol_name(pool, symbol).await?.is_none() {
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            yahoo::symbol_name(symbol),
        )
        .await
        {
            Err(_) => tracing::warn!(%symbol, "symbol lookup timed out"),
            Ok(result) => match result {
                Ok(Some(name)) => {
                    symbols::update_symbol_name(pool, symbol, &name, None).await?;
                    tracing::info!(%symbol, %name, "updated symbol name");
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(%symbol, %error, "symbol lookup failed"),
            },
        }
    }
    Ok(())
}

pub async fn backfill_price_history(
    pool: &SqlitePool,
    symbol: &str,
    start: NaiveDate,
) -> anyhow::Result<usize> {
    let rows = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        yahoo::daily_closes(symbol, start),
    )
    .await
    .context("price history request timed out")??;
    Ok(prices::bulk_insert_prices(pool, symbol, &rows).await?)
}

pub async fn backfill_fx_history(pool: &SqlitePool, start: NaiveDate) -> anyhow::Result<usize> {
    let rows = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        yahoo::daily_closes("USDJPY=X", start),
    )
    .await
    .context("FX history request timed out")??;
    Ok(fx::bulk_insert_usdjpy(pool, &rows).await?)
}

pub async fn update_fx_cache(pool: &SqlitePool) -> anyhow::Result<()> {
    let (rate, date) = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        yahoo::latest_close("USDJPY=X"),
    )
    .await
    .context("latest FX request timed out")??;
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
