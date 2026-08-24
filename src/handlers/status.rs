use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::handlers::AppState;
use crate::handlers::template_response::TemplateResponse;

#[derive(Template)]
#[template(path = "status.html")]
pub struct StatusTemplate;

pub async fn get_status(State(_state): State<AppState>) -> impl IntoResponse {
    TemplateResponse(StatusTemplate)
}
