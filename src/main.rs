mod clients;
mod config;
pub mod db;
mod domain;
pub mod handlers;
pub mod services;
pub mod util;

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::io::BufRead;
use std::str::FromStr;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
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

    let state = handlers::AppState {
        config: std::sync::Arc::new(config),
        db,
        refresh_lock: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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

    let app = handlers::router(state);

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
    use tokio_cron_scheduler::{Job, JobScheduler};

    let sched = JobScheduler::new()
        .await
        .context("failed to create scheduler")?;

    let pool = pool.clone();
    let job = Job::new_async(cron_expr, move |_uuid, _lock| {
        let pool = pool.clone();
        Box::pin(async move {
            services::scheduler::run_daily_batch(&pool).await;
        })
    })
    .context("failed to create cron job")?;

    sched
        .add(job)
        .await
        .context("failed to add job to scheduler")?;

    sched.start().await.context("failed to start scheduler")?;

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
