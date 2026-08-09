use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

/// Renders an Askama template as an Axum HTML response.
///
/// Askama 0.13 removed the framework-specific integration crates, so the
/// application owns this small adapter instead of depending on `askama_axum`.
pub struct TemplateResponse<T>(pub T);

impl<T: Template> IntoResponse for TemplateResponse<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(error) => {
                tracing::error!(%error, "failed to render Askama template");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
