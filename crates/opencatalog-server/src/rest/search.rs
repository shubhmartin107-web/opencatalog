use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Query},
    http::StatusCode,
};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::SearchResults;
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

pub async fn search(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResults>, StatusCode> {
    let results = state
        .store
        .search(&params.q, params.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(results))
}
