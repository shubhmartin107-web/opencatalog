use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path},
    http::StatusCode,
};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::SetMetadataRequest;
use uuid::Uuid;

use crate::AppState;

pub async fn set_dataset_metadata(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<SetMetadataRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let entry = opencatalog_core::types::MetadataEntry {
        dataset_id: id,
        column_id: None,
        key: req.key,
        value: req.value,
    };
    state
        .store
        .set_metadata(entry)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, Json(serde_json::json!({"status": "ok"}))))
}

pub async fn get_dataset_metadata(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    let metadata = state
        .store
        .get_metadata(id, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(metadata))
}

pub async fn delete_dataset_metadata(
    Extension(state): Extension<Arc<AppState>>,
    Path((id, key)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    state
        .store
        .delete_metadata(id, &key, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
