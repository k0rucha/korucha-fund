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
        .unwrap_or_default();
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
        })
    }
}

pub async fn add_transaction(
    State(state): State<AppState>,
    Form(form): Form<TransactionForm>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let data = NewTransaction::try_from(form)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    transactions::create_transaction(&state.db, data.clone())
        .await
        .map_err(|e| {
            tracing::error!("Failed to create transaction: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create transaction".to_string(),
            )
        })?;

    // Symbols-table auto-population happens in the background so the admin
    // form returns immediately even if yfinance is slow / failing.
    let pool = state.db.clone();
    let symbol = data.symbol.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::services::market_data::update_price_cache(&pool, &symbol).await {
            tracing::warn!("Background price cache update failed for {}: {}", symbol, e);
        }
    });

    Ok(Redirect::to("/admin"))
}

pub async fn delete_transaction(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    transactions::delete_transaction(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete transaction: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(axum::http::StatusCode::OK)
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
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?
    {
        if field.name() == Some("file") {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;
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

    let result = transaction_service::import(&state.db, imported_txs)
        .await
        .map_err(|error| {
            tracing::error!(%error, "transaction import failed");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed during import".to_string(),
            )
        })?;

    tracing::info!(
        "Import complete: {} new, {} skipped (duplicate)",
        result.imported,
        result.skipped
    );

    // Background-fetch prices for newly seen symbols.
    let pool = state.db.clone();
    tokio::spawn(async move {
        for symbol in result.symbols {
            if let Err(e) = crate::services::market_data::update_price_cache(&pool, &symbol).await {
                tracing::warn!("Background price cache update failed for {}: {}", symbol, e);
            }
        }
    });

    Ok(Redirect::to("/admin"))
}
