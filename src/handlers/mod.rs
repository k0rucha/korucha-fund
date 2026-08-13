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
    http::{Request, StatusCode},
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
}

pub fn router(state: AppState) -> Router {
    let admin_routes = Router::new()
        .route("/", get(admin::admin_index))
        .route("/transactions", post(admin::add_transaction))
        .route("/transactions/:id", delete(admin::delete_transaction))
        .route("/export", get(admin::export_transactions))
        .route("/import", post(admin::import_transactions))
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
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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
