use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path},
    http::StatusCode,
};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::{DataSource, PaginatedResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct CreateDatasourceRequest {
    pub name: String,
    pub source_type: String,
    pub description: Option<String>,
    pub connection_config: std::collections::HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct UpdateDatasourceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub connection_config: Option<std::collections::HashMap<String, String>>,
}

pub async fn list_datasources(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<PaginatedResponse<DataSource>>, StatusCode> {
    let sources = state.store.list_datasources().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sources))
}

pub async fn create_datasource(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<CreateDatasourceRequest>,
) -> Result<(StatusCode, Json<DataSource>), StatusCode> {
    let source_type = match req.source_type.as_str() {
        "openlake" => opencatalog_core::types::SourceType::OpenLake,
        "openingest" => opencatalog_core::types::SourceType::OpenIngest,
        "openpipe" => opencatalog_core::types::SourceType::OpenPipe,
        "openstream" => opencatalog_core::types::SourceType::OpenStream,
        custom => opencatalog_core::types::SourceType::Custom(custom.to_string()),
    };

    let ds = DataSource {
        id: Uuid::nil(),
        name: req.name,
        source_type,
        description: req.description,
        connection_config: req.connection_config,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = state.store.create_datasource(ds).await.map_err(|e| {
        tracing::error!("Failed to create datasource: {e}");
        StatusCode::CONFLICT
    })?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_datasource(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DataSource>, StatusCode> {
    let ds = state.store.get_datasource(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(ds))
}

pub async fn delete_datasource(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    state.store.delete_datasource(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(StatusCode::NO_CONTENT)
}
