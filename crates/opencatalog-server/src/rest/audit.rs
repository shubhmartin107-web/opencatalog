use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Query},
    http::StatusCode,
};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::{AuditEntry, PaginatedResponse, PaginationParams};
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
pub struct AuditQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn list_audit_entries(
    Extension(state): Extension<Arc<AppState>>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<PaginatedResponse<AuditEntry>>, StatusCode> {
    let pagination = PaginationParams {
        offset: query.offset.unwrap_or(0),
        limit: query.limit.unwrap_or(50),
    };
    let entries = state
        .store
        .list_audit_entries(&pagination)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(entries))
}