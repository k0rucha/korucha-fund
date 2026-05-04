mod auth;
mod config;
mod error;
pub mod db;
pub mod handlers;
pub mod services;
pub mod util;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::{Context, Result};
use axum::{
    Router,
    middleware::{self, Next},
    routing::{get, post, delete},
    http::{StatusCode, Request},
    response::IntoResponse,
};

use sqlx::SqlitePool;
use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions};
use std::str::FromStr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use std::io::BufRead;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    /// Set to true while a manual refresh is running, to prevent overlapping
    /// runs from blasting yfinance and double-counting the rate limiter.
    pub refresh_lock: Arc<AtomicBool>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let config = Config::from_env()?;
    let port = config.port;

    let connect_options = SqliteConnectOptions::from_str(&config.database_url)
        .context("invalid database url")?
        .create_if_missing(true);

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .context("failed to connect to database")?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("failed to run migrations")?;

    let state = AppState {
        config: Arc::new(config),
        db,
        refresh_lock: Arc::new(AtomicBool::new(false)),
    };

    // Phase 8: Start the daily scheduler
    let scheduler_db = state.db.clone();
    let scheduler_cron = state.config.scheduler_cron.clone();
    tokio::spawn(async move {
        if let Err(e) = start_scheduler(&scheduler_db, &scheduler_cron).await {
            tracing::error!("Scheduler failed to start: {}", e);
        }
    });

    // Phase 9: Start a simple server-console command listener (stdin).
    // This accepts commands only from the server console, e.g. `backfill`.
    let console_db = state.db.clone();
    tokio::spawn(async move {
        start_console_listener(console_db).await;
    });

    let admin_routes = Router::new()
        .route("/", get(handlers::admin::admin_index))
        .route("/transactions", post(handlers::admin::add_transaction))
        .route("/transactions/:id", delete(handlers::admin::delete_transaction))
        .route("/export", get(handlers::admin::export_transactions))
        .route("/import", post(handlers::admin::import_transactions))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::basic_auth));

    let app = Router::new()
        .route("/", get(handlers::dashboard::dashboard_index))
        .route("/status", get(handlers::status::get_status))
        .route("/api/status", get(handlers::status_api::get_status_json))
        .route("/fragment/composition", get(handlers::fragments::composition_json))
        .route("/fragment/timeseries", get(handlers::fragments::timeseries_json))
        .route("/refresh", post(handlers::refresh::refresh_prices))
        .route("/share", post(handlers::share::create_share_card))
        .route("/share/:id", get(handlers::share::view_share_card))
        .route("/share/:id/ogp.png", get(handlers::ogp::share_ogp))
        .route("/ticker-share", post(handlers::ticker_share::create_ticker_share_card))
        .route("/ticker/:id", get(handlers::ticker_share::view_ticker_share_card))
        .route("/ticker/:id/ogp.png", get(handlers::ogp::ticker_ogp))
        .nest("/admin", admin_routes)
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .fallback(handler_404)
        .layer(middleware::from_fn(error_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;
    tracing::info!("listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

/// Read lines from stdin and execute administrative commands.
/// Commands are intentionally only available via the server console (stdin).
async fn start_console_listener(pool: SqlitePool) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Spawn a blocking thread to read stdin lines and forward them to the async task.
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        let mut buf = String::new();
        loop {
            buf.clear();
            match handle.read_line(&mut buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let s = buf.trim_end().to_string();
                    if tx.send(s).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Console stdin read error: {}", e);
                    break;
                }
            }
        }
    });

    tracing::info!("Console command listener started. Type 'help' for commands.");

    while let Some(line) = rx.recv().await {
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }

        match cmd {
            "help" => {
                println!("available commands: backfill, help, quit");
            }
            "backfill" => {
                tracing::info!("Console: starting backfill_and_regenerate (console-triggered)");
                let pool_clone = pool.clone();
                tokio::spawn(async move {
                    match services::scheduler::backfill_and_regenerate(&pool_clone).await {
                        Ok(_) => tracing::info!("Console: backfill completed"),
                        Err(e) => tracing::error!("Console: backfill failed: {}", e),
                    }
                });
            }
            "quit" | "exit" => {
                tracing::info!("Console: exit requested — shutting down process");
                std::process::exit(0);
            }
            other => {
                tracing::warn!("Console: unknown command '{}'", other);
            }
        }
    }

    tracing::info!("Console command listener terminated");
}

async fn start_scheduler(pool: &SqlitePool, cron_expr: &str) -> Result<()> {
    use tokio_cron_scheduler::{JobScheduler, Job};

    let sched = JobScheduler::new().await
        .context("failed to create scheduler")?;

    let pool = pool.clone();
    let job = Job::new_async(cron_expr, move |_uuid, _lock| {
        let pool = pool.clone();
        Box::pin(async move {
            services::scheduler::run_daily_batch(&pool).await;
        })
    })
    .context("failed to create cron job")?;

    sched.add(job).await
        .context("failed to add job to scheduler")?;

    sched.start().await
        .context("failed to start scheduler")?;

    tracing::info!("Scheduler started with cron: {}", cron_expr);
    Ok(())
}


async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}

async fn handler_404() -> impl axum::response::IntoResponse {
    crate::error::AppError::NotFound
}

async fn error_handler(req: Request<axum::body::Body>, next: Next) -> axum::response::Response {
    let response = next.run(req).await;
    let status = response.status();

    // 404 is already rendered by handler_404 / AppError::NotFound::into_response().
    // Only intercept 405 here, which has no dedicated fallback handler.
    if status == StatusCode::METHOD_NOT_ALLOWED {
        crate::error::AppError::MethodNotAllowed.into_response()
    } else {
        response
    }
}
