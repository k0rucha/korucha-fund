use axum::{extract::State, response::IntoResponse};
use croner::Cron;
use serde_json::json;
use std::str::FromStr;

use crate::db::api_stats;
use crate::handlers::AppState;
use crate::util::jst;

fn jst_now() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::Utc::now().with_timezone(&jst())
}

pub async fn get_status_json(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    // Get current API stats
    let stats = api_stats::get_stats(&state.db).await.map_err(|error| {
        tracing::error!(%error, "failed to load API stats");
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (can_request, minutes_remaining) =
        api_stats::can_request_api(&state.db)
            .await
            .map_err(|error| {
                tracing::error!(%error, "failed to check API availability");
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let remaining = api_stats::requests_remaining(&state.db)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to load remaining API requests");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Format last request time
    let last_request_time_str = stats
        .last_request_time
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

/// Calculate the next execution using the same cron engine and JST timezone as
/// the actual scheduler.
fn calculate_next_cron_execution(cron_expr: &str) -> (String, i64) {
    calculate_next_cron_execution_at(cron_expr, jst_now())
}

fn calculate_next_cron_execution_at(
    cron_expr: &str,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> (String, i64) {
    let Ok(cron) = Cron::from_str(cron_expr) else {
        return ("不明".to_string(), 0);
    };
    let Ok(next) = cron.find_next_occurrence(&now, false) else {
        return ("不明".to_string(), 0);
    };
    let seconds = next.signed_duration_since(now).num_seconds().max(0);
    let minutes = (seconds + 59) / 60;
    let formatted = if next.date_naive() == now.date_naive() {
        next.format("%H:%M").to_string()
    } else {
        next.format("%m月%d日 %H:%M").to_string()
    };
    (formatted, minutes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn calculates_full_cron_expression_in_jst() {
        let now = jst().with_ymd_and_hms(2026, 8, 24, 22, 30, 0).unwrap();
        assert_eq!(
            calculate_next_cron_execution_at("0 0 23 * * *", now),
            ("23:00".into(), 30)
        );
    }

    #[test]
    fn respects_day_of_week_constraints() {
        let monday = jst().with_ymd_and_hms(2026, 8, 24, 23, 30, 0).unwrap();
        assert_eq!(
            calculate_next_cron_execution_at("0 0 23 * * MON", monday),
            ("08月31日 23:00".into(), 10_050)
        );
    }
}
