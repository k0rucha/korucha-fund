use axum::{extract::State, response::IntoResponse};
use askama::Template;

use crate::AppState;

#[derive(Template)]
#[template(path = "status.html")]
pub struct StatusTemplate;

pub async fn get_status(State(_state): State<AppState>) -> impl IntoResponse {
    StatusTemplate
}
