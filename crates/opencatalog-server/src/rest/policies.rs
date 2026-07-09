use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path, Query},
    http::StatusCode,
};
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub policy_type: String,
    pub dataset_pattern: String,
    pub action: String,
    pub roles: Vec<String>,
}

#[derive(Deserialize)]
pub struct PolicyListParams {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn list_policies(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<PolicyListParams>,
) -> Result<Json<PaginatedResponse<Policy>>, StatusCode> {
    let pagination = PaginationParams {
        offset: params.offset.unwrap_or(0),
        limit: params.limit.unwrap_or(50),
    };
    let policies = state.store.list_policies(&pagination).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(policies))
}

pub async fn create_policy(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<(StatusCode, Json<Policy>), StatusCode> {
    let ptype = match req.policy_type.as_str() {
        "masking" => PolicyType::Masking,
        "row_filter" => PolicyType::RowFilter,
        "access" => PolicyType::Access,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let action = match req.action.as_str() {
        "redact" => PolicyAction::Mask(MaskMethod::Redact),
        "hash" => PolicyAction::Mask(MaskMethod::Hash),
        "nullify" => PolicyAction::Mask(MaskMethod::Nullify),
        "deny" => PolicyAction::Deny,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let policy = Policy {
        id: Uuid::nil(),
        name: req.name,
        description: None,
        policy_type: ptype,
        rules: vec![PolicyRule {
            dataset_pattern: req.dataset_pattern,
            column_pattern: None,
            condition: None,
            action,
            roles: req.roles,
        }],
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = state.store.create_policy(policy).await.map_err(|_| StatusCode::CONFLICT)?;
    Ok((StatusCode::CREATED, Json(created)))
}
