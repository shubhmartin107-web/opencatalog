use std::sync::Arc;

use axum::{Json, extract::Extension, http::StatusCode};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::MetricsSnapshot;

use crate::AppState;

pub async fn get_metrics(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<MetricsSnapshot>, StatusCode> {
    let metrics = state
        .store
        .get_metrics()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(metrics))
}
