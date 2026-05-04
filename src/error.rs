use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response, Html},
};
use thiserror::Error;

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFoundTemplate {}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("not found")]
    NotFound,

    #[error("method not allowed")]
    MethodNotAllowed,

    #[error("unauthorized")]
    Unauthorized,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Database(_) | AppError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        match status {
            StatusCode::NOT_FOUND => {
                let template = NotFoundTemplate {};
                match template.render() {
                    Ok(html) => (status, Html(html)).into_response(),
                    Err(e) => {
                        tracing::error!(error = ?e, "failed to render 404 template");
                        (status, "Not Found").into_response()
                    }
                }
            }
            StatusCode::UNAUTHORIZED => (status, "Unauthorized").into_response(),
            _ => {
                if status.is_server_error() {
                    tracing::error!(error = ?self, "internal server error");
                }
                let template = ErrorTemplate {};
                match template.render() {
                    Ok(html) => (status, Html(html)).into_response(),
                    Err(e) => {
                        tracing::error!(error = ?e, "failed to render error template");
                        (status, "Internal Server Error").into_response()
                    }
                }
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
