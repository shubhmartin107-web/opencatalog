use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path, Query},
    http::StatusCode,
};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::{Dataset, LineageGraph, PaginatedResponse, PaginationParams};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct ListDatasetsParams {
    pub datasource_id: Option<Uuid>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

fn to_pagination(offset: Option<u64>, limit: Option<u64>) -> PaginationParams {
    PaginationParams {
        offset: offset.unwrap_or(0),
        limit: limit.unwrap_or(50),
    }
}

pub async fn list_datasets(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<ListDatasetsParams>,
) -> Result<Json<PaginatedResponse<Dataset>>, StatusCode> {
    let pagination = to_pagination(params.offset, params.limit);
    let datasets = state
        .store
        .list_datasets(params.datasource_id, &pagination)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(datasets))
}

pub async fn get_dataset(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Dataset>, StatusCode> {
    let ds = state
        .store
        .get_dataset(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(ds))
}

pub async fn search_datasets(
    Extension(state): Extension<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<PaginatedResponse<Dataset>>, StatusCode> {
    let pagination = to_pagination(query.offset, query.limit);
    let results = state
        .store
        .search_datasets(&query.q, &pagination)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(results))
}

pub async fn get_dataset_lineage(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LineageGraph>, StatusCode> {
    let lineage = state
        .store
        .get_lineage(id, opencatalog_core::traits::LineageDirection::Both)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(lineage))
}

pub async fn get_dataset_impact(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LineageGraph>, StatusCode> {
    let lineage = state
        .store
        .get_lineage(id, opencatalog_core::traits::LineageDirection::Downstream)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(lineage))
}
