use std::sync::Arc;

use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::AppState;

/// Simple API key authentication middleware.
/// Reads `X-API-Key` header. If `api_keys` is empty in config, auth is disabled.
pub async fn auth_middleware(
    Extension(state): Extension<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_keys = &state.api_keys;

    // If no API keys configured, skip auth
    if api_keys.is_empty() {
        return Ok(next.run(req).await);
    }

    let api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !api_keys.contains(&api_key.to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}
