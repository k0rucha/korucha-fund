use askama::Template;
use axum::{
    body::Body,
    extract::{Form, Multipart, Path, State},
    response::{IntoResponse, Redirect, Response},
};
use chrono::NaiveDate;
use serde::Deserialize;
use std::collections::HashMap;

use crate::db::{symbols, transactions};
use crate::domain::portfolio::{NewTransaction, Transaction};
use crate::handlers::AppState;
use crate::handlers::template_response::TemplateResponse;
use crate::services::transactions as transaction_service;

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {
    pub transactions: Vec<Transaction>,
    pub symbol_names: HashMap<String, String>,
}

pub async fn admin_index(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let txs = transactions::list_transactions(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list transactions: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let symbol_list = symbols::get_symbol_names(&state.db)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to list symbol names");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let symbol_names: HashMap<String, String> = symbol_list
        .into_iter()
        .filter_map(|s| s.name.map(|n| (s.symbol, n)))
        .collect();
    Ok(TemplateResponse(AdminTemplate {
        transactions: txs,
        symbol_names,
    }))
}

#[derive(Deserialize)]
pub struct TransactionForm {
    pub symbol: String,
    pub txn_type: String,
    pub quantity: f64,
    pub price: f64,
    pub currency: String,
    #[serde(default)]
    pub fee: f64,
    pub txn_date: String,
    pub fx_rate_to_jpy: Option<String>,
    pub notes: Option<String>,
}

impl TryFrom<TransactionForm> for NewTransaction {
    type Error = anyhow::Error;

    fn try_from(form: TransactionForm) -> Result<Self, Self::Error> {
        let txn_date = NaiveDate::parse_from_str(&form.txn_date, "%Y-%m-%d")?;
        let fx_rate_to_jpy = match form.fx_rate_to_jpy {
            Some(ref s) if !s.trim().is_empty() => Some(s.parse::<f64>()?),
            _ => None,
        };
        let notes = match form.notes {
            Some(ref s) if !s.trim().is_empty() => Some(s.to_string()),
            _ => None,
        };

        Ok(NewTransaction {
            symbol: form.symbol,
            txn_type: form.txn_type,
            quantity: form.quantity,
            price: form.price,
            currency: form.currency,
            fee: form.fee,
            txn_date,
            fx_rate_to_jpy,
            notes,
        }
        .normalized()?)
    }
}

pub async fn add_transaction(
    State(state): State<AppState>,
    Form(form): Form<TransactionForm>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let data = NewTransaction::try_from(form)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let transaction_guard = state.transaction_lock.lock().await;
    transaction_service::create(&state.db, data.clone())
        .await
        .map_err(|error| match error {
            transaction_service::MutationError::InvalidLedger(_) => {
                (axum::http::StatusCode::BAD_REQUEST, error.to_string())
            }
            transaction_service::MutationError::NotFound => {
                (axum::http::StatusCode::NOT_FOUND, error.to_string())
            }
            transaction_service::MutationError::Database(_) => {
                tracing::error!(%error, "failed to create transaction");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "取引の保存中にデータベースエラーが発生しました".to_string(),
                )
            }
        })?;
    drop(transaction_guard);

    // Symbols-table auto-population happens in the background so the admin
    // form returns immediately even if yfinance is slow / failing.
    let pool = state.db.clone();
    let symbol = data.symbol.clone();
    tokio::spawn(async move {
        let (success, claimed) = crate::services::market_data::try_update_prices_from_api(
            &pool,
            std::slice::from_ref(&symbol),
        )
        .await;
        if claimed && !success {
            tracing::warn!(%symbol, "background market-data update was incomplete");
        }
    });

    Ok(Redirect::to("/admin"))
}

pub async fn delete_transaction(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let transaction_guard = state.transaction_lock.lock().await;
    transaction_service::delete(&state.db, id)
        .await
        .map_err(|error| match error {
            transaction_service::MutationError::NotFound => axum::http::StatusCode::NOT_FOUND,
            transaction_service::MutationError::InvalidLedger(_) => {
                axum::http::StatusCode::CONFLICT
            }
            transaction_service::MutationError::Database(_) => {
                tracing::error!(%error, "failed to delete transaction");
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    drop(transaction_guard);
    Ok(([("HX-Refresh", "true")], axum::http::StatusCode::OK))
}

pub async fn export_transactions(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let txs = transactions::list_transactions(&state.db)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let json = serde_json::to_string_pretty(&txs)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .header(
            "Content-Disposition",
            "attachment; filename=\"transactions.json\"",
        )
        .body(Body::from(json))
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

pub async fn import_transactions(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let mut data = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (e.status(), e.to_string()))?
    {
        if field.name() == Some("file") {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| (e.status(), e.to_string()))?;
            data = Some(bytes);
            break;
        }
    }

    let bytes = data.ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        "No file uploaded".to_string(),
    ))?;
    let imported_txs: Vec<Transaction> = serde_json::from_slice(&bytes).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid JSON format: {}", e),
        )
    })?;

    let transaction_guard = state.transaction_lock.lock().await;
    let result = transaction_service::import(&state.db, imported_txs)
        .await
        .map_err(|error| match error {
            transaction_service::ImportError::TooManyTransactions
            | transaction_service::ImportError::InvalidTransaction { .. }
            | transaction_service::ImportError::InvalidLedger(_) => {
                (axum::http::StatusCode::BAD_REQUEST, error.to_string())
            }
            transaction_service::ImportError::Database(_) => {
                tracing::error!(%error, "transaction import failed");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "インポート中にデータベースエラーが発生しました".to_string(),
                )
            }
        })?;
    drop(transaction_guard);

    tracing::info!(
        "Import complete: {} new, {} skipped (duplicate)",
        result.imported,
        result.skipped
    );

    // Background-fetch prices for newly seen symbols.
    let pool = state.db.clone();
    tokio::spawn(async move {
        let symbols: Vec<_> = result.symbols.into_iter().collect();
        let (success, claimed) =
            crate::services::market_data::try_update_prices_from_api(&pool, &symbols).await;
        if claimed && !success {
            tracing::warn!("background import market-data update was incomplete");
        }
    });

    Ok(Redirect::to("/admin"))
}
