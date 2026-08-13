use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db::{api_stats, fx, prices, share_cards as fund_cards, symbols, ticker_share_cards};
use crate::services::{market_data, portfolio};
use crate::util::jst_today;

const BACKFILL_ROW_THRESHOLD: i64 = 5;
const BACKFILL_DAYS: i64 = 35;

#[derive(Serialize, Deserialize, Clone)]
pub struct FundHoldingSnapshot {
    pub symbol: String,
    pub name: String,
    pub current_value_jpy: f64,
    pub unrealized_pnl_jpy: f64,
    pub unrealized_pnl_pct: f64,
}

pub struct CurrentFund {
    pub total_value_jpy: f64,
    pub total_cost_jpy: f64,
    pub unrealized_pnl_jpy: f64,
    pub holdings: Vec<FundHoldingSnapshot>,
}

pub async fn current_fund(pool: &SqlitePool) -> sqlx::Result<CurrentFund> {
    let current = portfolio::current_for_display(pool).await?;
    let holdings = current
        .portfolio
        .holdings
        .iter()
        .map(|holding| FundHoldingSnapshot {
            symbol: holding.symbol.clone(),
            name: current
                .symbol_names
                .get(&holding.symbol)
                .cloned()
                .unwrap_or_default(),
            current_value_jpy: holding.current_value_jpy,
            unrealized_pnl_jpy: holding.unrealized_pnl_jpy,
            unrealized_pnl_pct: holding.unrealized_pnl_pct,
        })
        .collect();

    Ok(CurrentFund {
        total_value_jpy: current.portfolio.total_value_jpy,
        total_cost_jpy: current.portfolio.total_cost_jpy,
        unrealized_pnl_jpy: current.portfolio.unrealized_pnl_jpy,
        holdings,
    })
}

pub async fn create_fund_card(pool: &SqlitePool, span: Option<&str>) -> anyhow::Result<String> {
    let fund = current_fund(pool).await?;
    let holdings_json = serde_json::to_string(&fund.holdings)?;
    let span = normalize_span(span);
    let mut last_collision = None;

    for attempt in 0..3 {
        let id = generate_id();
        match fund_cards::insert_share_card(
            pool,
            &id,
            fund.total_value_jpy,
            fund.total_cost_jpy,
            fund.unrealized_pnl_jpy,
            &holdings_json,
            &span,
        )
        .await
        {
            Ok(()) => return Ok(id),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                tracing::warn!(attempt, "fund share ID collision");
                last_collision = Some(sqlx::Error::Database(error));
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }

    Err(last_collision.expect("three collisions recorded").into())
}

pub async fn create_ticker_card(
    pool: &SqlitePool,
    symbol: &str,
    span: Option<&str>,
) -> anyhow::Result<String> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() {
        anyhow::bail!("symbol is required");
    }
    let span = normalize_span(span);
    let today = jst_today();
    let since = today - chrono::Duration::days(BACKFILL_DAYS);
    let recent = prices::count_history_since(pool, &symbol, since).await?;
    if recent < BACKFILL_ROW_THRESHOLD {
        match market_data::backfill_price_history(pool, &symbol, since).await {
            Ok(rows) => {
                tracing::info!(%symbol, rows, recent, "backfilled ticker history");
                if let Err(error) = api_stats::record_api_request_forced(pool).await {
                    tracing::warn!(%error, "failed to record ticker backfill API request");
                }
            }
            Err(error) => tracing::warn!(%symbol, %error, "ticker backfill failed"),
        }
    }

    let issue_price = prices::get_price_on_or_before(pool, &symbol, today)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "銘柄 {symbol} の価格データがまだありません。管理画面から取引を追加するか、しばらく待ってから再試行してください。"
            )
        })?;
    let symbol_rows = symbols::get_symbol_names(pool).await?;
    let symbol_meta = symbol_rows.iter().find(|row| row.symbol == symbol);
    let currency = symbol_meta
        .map(|row| row.currency.clone())
        .unwrap_or_else(|| {
            if symbol.ends_with(".T") {
                "JPY".into()
            } else {
                "USD".into()
            }
        });
    let mut display_name = symbol_meta.and_then(|row| row.name.clone());
    if display_name.is_none() {
        match market_data::lookup_symbol_name(&symbol).await {
            Ok(name) => display_name = name,
            Err(error) => tracing::warn!(%symbol, %error, "ticker name lookup failed"),
        }
    }
    symbols::upsert_symbol(pool, &symbol, display_name.as_deref(), &currency).await?;

    let fx_rate_at_issue = if currency == "USD" {
        fx::get_latest_usdjpy(pool).await?
    } else {
        None
    };
    let holding = portfolio::current_analysis(pool)
        .await?
        .holdings
        .into_iter()
        .find(|holding| holding.symbol == symbol);
    let (quantity, average_cost, issue_value, issue_pnl) = holding
        .map(|holding| {
            let rate = if holding.currency == "USD" {
                fx_rate_at_issue.unwrap_or(portfolio::DISPLAY_USD_JPY_FALLBACK)
            } else {
                1.0
            };
            let value = holding.quantity * issue_price * rate;
            (
                Some(holding.quantity),
                Some(holding.average_cost_native),
                Some(value),
                Some(value - holding.total_cost_jpy),
            )
        })
        .unwrap_or((None, None, None, None));

    let mut last_collision = None;
    for attempt in 0..3 {
        let id = generate_id();
        match ticker_share_cards::insert_ticker_share_card(
            pool,
            &id,
            &symbol,
            display_name.as_deref(),
            &currency,
            issue_price,
            fx_rate_at_issue,
            quantity,
            average_cost,
            issue_value,
            issue_pnl,
            &span,
        )
        .await
        {
            Ok(()) => return Ok(id),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                tracing::warn!(attempt, "ticker share ID collision");
                last_collision = Some(sqlx::Error::Database(error));
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }

    Err(last_collision.expect("three collisions recorded").into())
}

pub fn normalize_span(span: Option<&str>) -> String {
    match span.unwrap_or("all") {
        "7d" => "7d".into(),
        "30d" => "30d".into(),
        _ => "all".into(),
    }
}

fn generate_id() -> String {
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    format!("{:x}{:04x}", nanos, hasher.finish() & 0xFFFF)
}
