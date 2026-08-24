use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone};
use sqlx::SqlitePool;

use crate::util::{jst, jst_today};

const MIN_INTERVAL_MINUTES: i64 = 96; // 1440 / 15 requests per day
const MAX_REQUESTS_PER_DAY: i64 = 15;

fn jst_now() -> DateTime<FixedOffset> {
    chrono::Utc::now().with_timezone(&jst())
}

#[derive(Debug, Clone)]
pub struct ApiRequestStats {
    pub last_request_time: Option<DateTime<FixedOffset>>,
    pub request_count: i64,
    pub reset_date: NaiveDate,
}

/// Initialize stats row for today if it doesn't already exist.
async fn ensure_stats_for_today(pool: &SqlitePool) -> sqlx::Result<()> {
    let today = jst_today();

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO api_request_stats (reset_date, last_request_time, request_count)
        VALUES (?, NULL, 0)
        "#,
    )
    .bind(today)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get current API request stats for today.
pub async fn get_stats(pool: &SqlitePool) -> sqlx::Result<ApiRequestStats> {
    ensure_stats_for_today(pool).await?;

    let today = jst_today();

    let row: (Option<String>, i64, String) = sqlx::query_as(
        r#"
        SELECT last_request_time, request_count, reset_date
        FROM api_request_stats
        WHERE reset_date = ?
        LIMIT 1
        "#,
    )
    .bind(today)
    .fetch_one(pool)
    .await?;

    let last_request_time = row.0.and_then(|ts| {
        NaiveDateTime::parse_from_str(&ts, "%Y-%m-%d %H:%M:%S")
            .ok()
            .and_then(|naive| jst().from_local_datetime(&naive).single())
    });

    Ok(ApiRequestStats {
        last_request_time,
        request_count: row.1,
        reset_date: NaiveDate::parse_from_str(&row.2, "%Y-%m-%d").unwrap_or(today),
    })
}

/// Speculative check: is an external API request allowed right now?
/// Returns (can_request, minutes_until_next_allowed).
///
/// **NOTE**: this is informational only. To actually claim a slot, call
/// [`try_record_api_request`] which performs the same check atomically.
pub async fn can_request_api(pool: &SqlitePool) -> sqlx::Result<(bool, i64)> {
    let stats = get_stats(pool).await?;

    if stats.request_count >= MAX_REQUESTS_PER_DAY {
        return Ok((false, 0));
    }

    let Some(last) = stats.last_request_time else {
        return Ok((true, 0));
    };

    let now = jst_now();
    let elapsed_minutes = now.signed_duration_since(last).num_seconds() / 60;
    let remaining_minutes = MIN_INTERVAL_MINUTES - elapsed_minutes;

    if remaining_minutes <= 0 {
        Ok((true, 0))
    } else {
        Ok((false, remaining_minutes))
    }
}

/// Atomically claim a request slot for today.
/// Returns `true` if the slot was claimed (caller may proceed with the API call).
/// Returns `false` if the rate limit forbade it (count maxed or interval not elapsed).
///
/// The single UPDATE statement performs check + increment atomically, so two
/// concurrent callers cannot both succeed.
pub async fn try_record_api_request(pool: &SqlitePool) -> sqlx::Result<bool> {
    ensure_stats_for_today(pool).await?;

    let today = jst_today();
    let now_str = jst_now().format("%Y-%m-%d %H:%M:%S").to_string();

    // The WHERE clause enforces both rate-limit conditions inside the UPDATE.
    // SQLite's `(julianday(a) - julianday(b)) * 1440` gives elapsed minutes.
    let result = sqlx::query(
        r#"
        UPDATE api_request_stats
        SET last_request_time = ?,
            request_count     = request_count + 1
        WHERE reset_date = ?
          AND request_count < ?
          AND (last_request_time IS NULL
               OR (julianday(?) - julianday(last_request_time)) * 1440 >= ?)
        "#,
    )
    .bind(&now_str)
    .bind(today)
    .bind(MAX_REQUESTS_PER_DAY)
    .bind(&now_str)
    .bind(MIN_INTERVAL_MINUTES)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn requests_remaining(pool: &SqlitePool) -> sqlx::Result<i64> {
    let stats = get_stats(pool).await?;
    Ok((MAX_REQUESTS_PER_DAY - stats.request_count).max(0))
}
