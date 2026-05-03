use axum::{extract::State, response::IntoResponse};
use serde_json::json;
use chrono::{FixedOffset, TimeZone};

use crate::AppState;
use crate::db::api_stats;

fn jst() -> FixedOffset {
    FixedOffset::east_opt(9 * 3600).unwrap()
}

fn jst_now() -> chrono::DateTime<FixedOffset> {
    chrono::Utc::now().with_timezone(&jst())
}

pub async fn get_status_json(State(state): State<AppState>) -> Result<impl IntoResponse, axum::http::StatusCode> {
    // Get current API stats
    let stats = api_stats::get_stats(&state.db).await.map_err(|_| {
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (can_request, minutes_remaining) = api_stats::can_request_api(&state.db).await.map_err(|_| {
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let remaining = api_stats::requests_remaining(&state.db).await.map_err(|_| {
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Format last request time
    let last_request_time_str = stats.last_request_time
        .map(|dt| dt.format("%Y年%m月%d日 %H:%M:%S").to_string())
        .unwrap_or_else(|| "未実行".to_string());

    // Determine status message
    let status_message = if can_request {
        "外部APIリクエスト可能".to_string()
    } else if minutes_remaining > 0 {
        format!("次回リクエストまで {} 分待機中", minutes_remaining)
    } else {
        "本日のリクエスト上限に達しました".to_string()
    };

    // Parse cron expression and calculate next execution
    let cron_expr = &state.config.scheduler_cron;
    let (next_exec_time, minutes_until_next) = calculate_next_cron_execution(cron_expr);

    let response = json!({
        "today": jst_now().date_naive().to_string(),
        "api_stats": {
            "request_count": stats.request_count,
            "max_requests": 15,
            "remaining_requests": remaining,
            "can_request": can_request,
            "minutes_until_next": if can_request { 0 } else { minutes_remaining },
            "last_request_time": last_request_time_str,
            "status_message": status_message,
        },
        "scheduler": {
            "cron_expression": cron_expr,
            "next_execution_time": next_exec_time,
            "minutes_until_next_execution": minutes_until_next,
        }
    });

    Ok(axum::response::Json(response))
}

/// Calculate next execution time from a 6-field cron expression
/// (`second minute hour day month dayofweek`). Only handles plain numeric
/// hour/minute fields; wildcards/steps (`*`, `*/30`) yield "不明" rather than
/// silently falling back to a fixed time.
fn calculate_next_cron_execution(cron_expr: &str) -> (String, i64) {
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();

    if parts.len() < 6 {
        return ("不明".to_string(), 1440);
    }

    let (Some(target_minute), Some(target_hour)) = (parts[1].parse::<u32>().ok(), parts[2].parse::<u32>().ok()) else {
        return ("不明".to_string(), 1440);
    };

    let now = jst_now();

    // Try today at target time first.
    if let Some(today_target) = now.date_naive().and_hms_opt(target_hour, target_minute, 0)
        && let Some(next_dt) = jst().from_local_datetime(&today_target).single()
        && next_dt > now
    {
        let minutes = next_dt.signed_duration_since(now).num_minutes();
        let next_str = next_dt.format("%H:%M").to_string();
        return (next_str, minutes);
    }

    // Otherwise tomorrow at target time.
    let tomorrow = now.date_naive() + chrono::Duration::days(1);
    if let Some(tomorrow_target) = tomorrow.and_hms_opt(target_hour, target_minute, 0)
        && let Some(next_dt) = jst().from_local_datetime(&tomorrow_target).single()
    {
        let minutes = next_dt.signed_duration_since(now).num_minutes();
        let date_str = next_dt.format("%m月%d日 %H:%M").to_string();
        return (date_str, minutes);
    }

    ("不明".to_string(), 1440)
}
