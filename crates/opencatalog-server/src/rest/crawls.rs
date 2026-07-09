use std::sync::Arc;

use axum::{Json, extract::{Extension, Path}, http::StatusCode};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::{CrawlRun, OpenLineageEvent};
use uuid::Uuid;

use crate::AppState;

pub async fn trigger_crawl(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrawlRun>, StatusCode> {
    let ds = state.store.get_datasource(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    let run = state
        .crawler_registry
        .crawl_and_persist(&ds, &state.store)
        .await
        .map_err(|e| {
            tracing::error!("Crawl failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(run))
}

/// POST /api/v1/lineage — Ingest an OpenLineage event from external systems.
pub async fn ingest_openlineage_event(
    Extension(state): Extension<Arc<AppState>>,
    Json(event): Json<OpenLineageEvent>,
) -> Result<StatusCode, StatusCode> {
    // Store the event
    state.store.ingest_openlineage_event(event).await.map_err(|e| {
        tracing::error!("Failed to ingest OpenLineage event: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::ACCEPTED)
}
