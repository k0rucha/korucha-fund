use anyhow::Context;
use chrono::NaiveDate;
use yahoo::time::OffsetDateTime;
use yahoo_finance_api as yahoo;

use crate::util::jst;

pub async fn latest_close(symbol: &str) -> anyhow::Result<(f64, NaiveDate)> {
    let provider = yahoo::YahooConnector::new()?;
    let response = provider.get_latest_quotes(symbol, "1d").await?;
    let quote = response.last_quote()?;
    anyhow::ensure!(
        quote.close.is_finite() && quote.close > 0.0,
        "invalid close price for {symbol}: {}",
        quote.close
    );
    let date = timestamp_to_jst_date(quote.timestamp)
        .ok_or_else(|| anyhow::anyhow!("invalid timestamp: {}", quote.timestamp))?;
    Ok((quote.close, date))
}

pub async fn symbol_name(symbol: &str) -> anyhow::Result<Option<String>> {
    let provider = yahoo::YahooConnector::new()?;
    let result = provider.search_ticker(symbol).await?;
    let quote = result
        .quotes
        .iter()
        .find(|quote| quote.symbol == symbol)
        .or_else(|| result.quotes.first());

    Ok(quote.and_then(|quote| {
        if !quote.long_name.is_empty() {
            Some(quote.long_name.clone())
        } else if !quote.short_name.is_empty() {
            Some(quote.short_name.clone())
        } else {
            None
        }
    }))
}

pub async fn daily_closes(symbol: &str, start: NaiveDate) -> anyhow::Result<Vec<(NaiveDate, f64)>> {
    let provider = yahoo::YahooConnector::new()?;
    let response = provider
        .get_quote_history(
            symbol,
            date_to_offset_date_time(start),
            OffsetDateTime::now_utc(),
        )
        .await
        .with_context(|| format!("get_quote_history failed for {symbol}"))?;

    Ok(response
        .quotes()?
        .into_iter()
        .filter(|quote| quote.close.is_finite() && quote.close > 0.0)
        .filter_map(|quote| timestamp_to_jst_date(quote.timestamp).map(|date| (date, quote.close)))
        .collect())
}

fn timestamp_to_jst_date(timestamp: i64) -> Option<NaiveDate> {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|date_time| date_time.with_timezone(&jst()).date_naive())
}

fn date_to_offset_date_time(date: NaiveDate) -> OffsetDateTime {
    let timestamp = date
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight")
        .and_utc()
        .timestamp();
    OffsetDateTime::from_unix_timestamp(timestamp).unwrap_or(OffsetDateTime::UNIX_EPOCH)
}
