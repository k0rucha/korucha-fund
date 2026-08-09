use axum::{
    extract::{State, Path, Form, Multipart},
    response::{IntoResponse, Redirect, Response},
    body::Body,
};
use askama::Template;
use serde::Deserialize;
use chrono::NaiveDate;
use std::collections::HashMap;

use crate::AppState;
use crate::db::{transactions::{self, Transaction, CreateTransaction}, symbols};
use crate::template_response::TemplateResponse;

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {
    pub transactions: Vec<Transaction>,
    pub symbol_names: HashMap<String, String>,
}

 
pub async fn admin_index(State(state): State<AppState>) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let txs = transactions::list_transactions(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list transactions: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let symbol_list = symbols::get_symbol_names(&state.db).await.unwrap_or_default();
    let symbol_names: HashMap<String, String> = symbol_list
        .into_iter()
        .filter_map(|s| s.name.map(|n| (s.symbol, n)))
        .collect();
    Ok(TemplateResponse(AdminTemplate { transactions: txs, symbol_names }))
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

impl TryFrom<TransactionForm> for CreateTransaction {
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

        Ok(CreateTransaction {
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
    let data = CreateTransaction::try_from(form)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    transactions::create_transaction(&state.db, data.clone())
        .await
        .map_err(|e| {
            tracing::error!("Failed to create transaction: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to create transaction".to_string())
        })?;

    // Symbols-table auto-population happens in the background so the admin
    // form returns immediately even if yfinance is slow / failing.
    let pool = state.db.clone();
    let symbol = data.symbol.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::services::yfinance::update_price_cache(&pool, &symbol).await {
            tracing::warn!("Background price cache update failed for {}: {}", symbol, e);
        }
    });

    Ok(Redirect::to("/admin"))
}

pub async fn delete_transaction(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    transactions::delete_transaction(&state.db, id).await.map_err(|e| {
        tracing::error!("Failed to delete transaction: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(axum::http::StatusCode::OK)
}

pub async fn export_transactions(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let txs = transactions::list_transactions(&state.db).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let json = serde_json::to_string_pretty(&txs).map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let response = Response::builder()
        .header("Content-Type", "application/json")
        .header("Content-Disposition", "attachment; filename=\"transactions.json\"")
        .body(Body::from(json))
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)
}

pub async fn import_transactions(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let mut data = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))? {
        if let Some(name) = field.name() {
            if name == "file" {
                let bytes = field.bytes().await.map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;
                data = Some(bytes);
                break;
            }
        }
    }

    let bytes = data.ok_or((axum::http::StatusCode::BAD_REQUEST, "No file uploaded".to_string()))?;
    let imported_txs: Vec<Transaction> = serde_json::from_slice(&bytes)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, format!("Invalid JSON format: {}", e)))?;

    // Build a fingerprint set of existing transactions so we can skip
    // duplicates. Spec §6 says `transactions` is the single source of truth,
    // so re-importing the same export twice must be idempotent.
    let existing = transactions::list_transactions(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list existing transactions for dedup: {}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DB read failed".to_string())
    })?;
    let fingerprint = |t: &Transaction| -> String {
        format!(
            "{}|{}|{}|{:.6}|{:.6}|{:.6}",
            t.symbol, t.txn_type, t.txn_date, t.quantity, t.price, t.fee
        )
    };
    let existing_keys: std::collections::HashSet<String> =
        existing.iter().map(fingerprint).collect();

    let mut symbols_to_update = std::collections::HashSet::new();
    let mut imported = 0usize;
    let mut skipped = 0usize;
    for tx in imported_txs {
        if existing_keys.contains(&fingerprint(&tx)) {
            skipped += 1;
            continue;
        }
        let create_data = CreateTransaction {
            symbol: tx.symbol.clone(),
            txn_type: tx.txn_type,
            quantity: tx.quantity,
            price: tx.price,
            currency: tx.currency,
            fee: tx.fee,
            txn_date: tx.txn_date,
            fx_rate_to_jpy: tx.fx_rate_to_jpy,
            notes: tx.notes,
        };

        transactions::create_transaction(&state.db, create_data.clone()).await.map_err(|e| {
            tracing::error!("Failed to import transaction: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed during import".to_string())
        })?;

        symbols_to_update.insert(tx.symbol);
        imported += 1;
    }

    tracing::info!("Import complete: {} new, {} skipped (duplicate)", imported, skipped);

    // Background-fetch prices for newly seen symbols.
    let pool = state.db.clone();
    tokio::spawn(async move {
        for symbol in symbols_to_update {
            if let Err(e) = crate::services::yfinance::update_price_cache(&pool, &symbol).await {
                tracing::warn!("Background price cache update failed for {}: {}", symbol, e);
            }
        }
    });

    Ok(Redirect::to("/admin"))
}
