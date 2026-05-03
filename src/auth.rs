use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::AppState;

pub async fn basic_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if check_credentials(&request, &state.config.admin_user, &state.config.admin_pass) {
        next.run(request).await
    } else {
        unauthorized()
    }
}

fn check_credentials(request: &Request, expected_user: &str, expected_pass: &str) -> bool {
    let Some(header_value) = request.headers().get(header::AUTHORIZATION) else {
        return false;
    };
    let Ok(value) = header_value.to_str() else {
        return false;
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded_bytes) = BASE64.decode(encoded.trim()) else {
        return false;
    };
    let Ok(decoded) = std::str::from_utf8(&decoded_bytes) else {
        return false;
    };
    let Some((user, pass)) = decoded.split_once(':') else {
        return false;
    };
    user == expected_user && pass == expected_pass
}

fn unauthorized() -> Response {
    let mut response = Response::new(axum::body::Body::from("Unauthorized"));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"korucha-fund admin\", charset=\"UTF-8\""),
    );
    response
}
