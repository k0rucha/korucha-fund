use std::sync::atomic::Ordering;
use std::time::Instant;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

use crate::handlers::AppState;
use crate::services::refresh;

pub async fn refresh_prices(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    if state
        .refresh_lock
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    struct Release<'a>(&'a std::sync::atomic::AtomicBool);
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _guard = Release(&state.refresh_lock);
    let start = Instant::now();
    let outcome = refresh::run(&state.db).await.map_err(|error| {
        tracing::error!(%error, "manual refresh failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        elapsed_ms = start.elapsed().as_millis(),
        updated_from_api = outcome.updated_from_api,
        "manual refresh completed"
    );
    Ok((
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "updated_from_api": outcome.updated_from_api,
            "external_update_attempted": outcome.external_update_attempted,
            "remaining_api_requests": outcome.remaining_api_requests,
        })),
    ))
}
