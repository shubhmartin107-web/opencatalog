use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path, Query},
    http::StatusCode,
};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::{GlossaryTerm, PaginatedResponse, PaginationParams, TermStatus};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct CreateGlossaryTermRequest {
    pub name: String,
    pub description: String,
    pub domain: Option<String>,
}

#[derive(Deserialize)]
pub struct GlossaryListParams {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn list_glossary(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<GlossaryListParams>,
) -> Result<Json<PaginatedResponse<GlossaryTerm>>, StatusCode> {
    let pagination = PaginationParams {
        offset: params.offset.unwrap_or(0),
        limit: params.limit.unwrap_or(50),
    };
    let terms = state.store.list_glossary_terms(&pagination).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(terms))
}

pub async fn create_glossary_term(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<CreateGlossaryTermRequest>,
) -> Result<(StatusCode, Json<GlossaryTerm>), StatusCode> {
    let term = GlossaryTerm {
        id: Uuid::nil(),
        name: req.name,
        description: req.description,
        short_description: None,
        domain: req.domain,
        synonyms: vec![],
        related_term_ids: vec![],
        custom_properties: std::collections::HashMap::new(),
        status: TermStatus::Draft,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = state.store.create_glossary_term(term).await.map_err(|_| StatusCode::CONFLICT)?;
    Ok((StatusCode::CREATED, Json(created)))
}
