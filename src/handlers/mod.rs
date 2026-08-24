pub mod admin;
mod auth;
pub mod dashboard;
mod error;
pub mod fragments;
pub mod ogp;
pub mod refresh;
pub mod share;
pub mod status;
pub mod status_api;
mod template_response;
pub mod ticker_share;

use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{HeaderValue, Method, Request, StatusCode, header},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, post},
};
use tower_http::trace::TraceLayer;

use std::sync::{Arc, atomic::AtomicBool};

use sqlx::SqlitePool;

use crate::config::Config;

use error::AppError;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    pub refresh_lock: Arc<AtomicBool>,
    pub transaction_lock: Arc<tokio::sync::Mutex<()>>,
    pub share_rate_limit: Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
}

pub(crate) async fn claim_share_creation(state: &AppState) -> bool {
    const MIN_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
    let now = std::time::Instant::now();
    let mut last = state.share_rate_limit.lock().await;
    if last.is_some_and(|last| now.duration_since(last) < MIN_INTERVAL) {
        return false;
    }
    *last = Some(now);
    true
}

pub fn router(state: AppState) -> Router {
    let admin_routes = Router::new()
        .route("/", get(admin::admin_index))
        .route("/transactions", post(admin::add_transaction))
        .route("/transactions/:id", delete(admin::delete_transaction))
        .route("/export", get(admin::export_transactions))
        .route("/import", post(admin::import_transactions))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::basic_auth,
        ));

    Router::new()
        .route("/", get(dashboard::dashboard_index))
        .route("/status", get(status::get_status))
        .route("/api/status", get(status_api::get_status_json))
        .route("/fragment/composition", get(fragments::composition_json))
        .route("/fragment/timeseries", get(fragments::timeseries_json))
        .route("/refresh", post(refresh::refresh_prices))
        .route("/share", post(share::create_share_card))
        .route("/share/:id", get(share::view_share_card))
        .route("/share/:id/ogp.png", get(ogp::share_ogp))
        .route(
            "/ticker-share",
            post(ticker_share::create_ticker_share_card),
        )
        .route("/ticker/:id", get(ticker_share::view_ticker_share_card))
        .route("/ticker/:id/ogp.png", get(ogp::ticker_ogp))
        .nest("/admin", admin_routes)
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .fallback(not_found)
        .layer(middleware::from_fn(render_method_not_allowed))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            validate_write_origin,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            add_security_headers,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn validate_write_origin(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> axum::response::Response {
    if matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        return next.run(request).await;
    }

    if let Some(site) = request
        .headers()
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        && !matches!(site, "same-origin" | "none")
    {
        return StatusCode::FORBIDDEN.into_response();
    }

    if let Some(origin) = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        && origin.trim_end_matches('/') != state.config.public_base_url
    {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(request).await
}

async fn add_security_headers(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> axum::response::Response {
    let is_static = request.uri().path().starts_with("/static/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    if state.config.public_base_url.starts_with("https://") {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    if is_static {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=3600"),
        );
    }
    response
}

async fn not_found() -> impl IntoResponse {
    AppError::NotFound
}

async fn render_method_not_allowed(request: Request<Body>, next: Next) -> axum::response::Response {
    let response = next.run(request).await;
    if response.status() == StatusCode::METHOD_NOT_ALLOWED {
        AppError::MethodNotAllowed.into_response()
    } else {
        response
    }
}
