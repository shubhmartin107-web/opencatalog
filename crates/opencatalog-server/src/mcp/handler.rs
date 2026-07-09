use std::sync::Arc;

use axum::{Json, extract::Extension, http::StatusCode};
use serde::{Deserialize, Serialize};
use opencatalog_core::traits::{CatalogStore, LineageDirection};
use opencatalog_core::types::*;

use crate::AppState;

#[derive(Deserialize)]
pub struct McpRequest {
    pub tool: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

#[derive(Serialize)]
pub struct McpResponse {
    pub success: bool,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn handle_mcp(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<McpRequest>,
) -> Result<Json<McpResponse>, StatusCode> {
    let result = match req.tool.as_str() {
        "catalog_search" => handle_search(&state, &req.arguments).await,
        "catalog_describe" => handle_describe(&state, &req.arguments).await,
        "catalog_lineage" => handle_lineage(&state, &req.arguments).await,
        "catalog_glossary_list" => handle_glossary_list(&state).await,
        "catalog_glossary_create" => handle_glossary_create(&state, &req.arguments).await,
        "catalog_policy_list" => handle_policy_list(&state).await,
        "catalog_policy_create" => handle_policy_create(&state, &req.arguments).await,
        "catalog_crawl" => handle_crawl(&state, &req.arguments).await,
        "catalog_doc_generate" => handle_doc_generate(&state, &req.arguments).await,
        "catalog_semantic_search" => handle_semantic_search(&state, &req.arguments).await,
        "list_tools" => {
            let tools = super::tools::all_tools();
            Ok(serde_json::to_value(tools).unwrap_or_default())
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match result {
        Ok(data) => Ok(Json(McpResponse {
            success: true,
            data,
            error: None,
        })),
        Err(e) => Ok(Json(McpResponse {
            success: false,
            data: serde_json::Value::Null,
            error: Some(e),
        })),
    }
}

fn get_id(params: &serde_json::Value, key: &str) -> Result<uuid::Uuid, String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or_else(|| format!("Missing or invalid '{key}'"))
}

async fn handle_search(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let query = params.get("query").and_then(|v| v.as_str()).ok_or("Missing 'query'")?;
    let results = state.store.search(query, 20).await.map_err(|e| format!("Search failed: {e}"))?;
    serde_json::to_value(results).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_describe(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_id(params, "dataset_id")?;
    let mut ds = state.store.get_dataset(id).await.map_err(|e| format!("Dataset not found: {e}"))?;
    let cols = state.store.get_columns(id).await.unwrap_or_default();
    ds.schema = cols;
    serde_json::to_value(ds).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_lineage(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_id(params, "dataset_id")?;
    let direction = params
        .get("direction")
        .and_then(|v| v.as_str())
        .unwrap_or("both");
    let dir = match direction {
        "upstream" => LineageDirection::Upstream,
        "downstream" => LineageDirection::Downstream,
        _ => LineageDirection::Both,
    };
    let lineage = state.store.get_lineage(id, dir).await.map_err(|e| format!("Lineage error: {e}"))?;
    serde_json::to_value(lineage).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_glossary_list(state: &AppState) -> Result<serde_json::Value, String> {
    let pagination = opencatalog_core::types::PaginationParams { offset: 0, limit: 1000 };
    let terms = state.store.list_glossary_terms(&pagination).await.map_err(|e| format!("Error: {e}"))?;
    serde_json::to_value(terms).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_glossary_create(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let name = params.get("name").and_then(|v| v.as_str()).ok_or("Missing 'name'")?;
    let desc = params.get("description").and_then(|v| v.as_str()).ok_or("Missing 'description'")?;
    let domain = params.get("domain").and_then(|v| v.as_str());

    let term = GlossaryTerm {
        id: uuid::Uuid::nil(),
        name: name.into(),
        description: desc.into(),
        short_description: None,
        domain: domain.map(String::from),
        synonyms: vec![],
        related_term_ids: vec![],
        custom_properties: std::collections::HashMap::new(),
        status: TermStatus::Draft,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = state.store.create_glossary_term(term).await.map_err(|e| format!("Error: {e}"))?;
    serde_json::to_value(created).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_policy_list(state: &AppState) -> Result<serde_json::Value, String> {
    let pagination = opencatalog_core::types::PaginationParams { offset: 0, limit: 1000 };
    let policies = state.store.list_policies(&pagination).await.map_err(|e| format!("Error: {e}"))?;
    serde_json::to_value(policies).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_policy_create(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let name = params.get("name").and_then(|v| v.as_str()).ok_or("Missing 'name'")?;
    let ptype = params.get("policy_type").and_then(|v| v.as_str()).ok_or("Missing 'policy_type'")?;
    let dataset_pattern = params.get("dataset_pattern").and_then(|v| v.as_str()).ok_or("Missing 'dataset_pattern'")?;
    let action_str = params.get("action").and_then(|v| v.as_str()).ok_or("Missing 'action'")?;
    let roles: Vec<String> = params
        .get("roles")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .ok_or("Missing 'roles'")?;

    let policy_type = match ptype {
        "masking" => PolicyType::Masking,
        "row_filter" => PolicyType::RowFilter,
        "access" => PolicyType::Access,
        _ => return Err("Invalid policy_type".into()),
    };

    let action = match action_str {
        "redact" => PolicyAction::Mask(MaskMethod::Redact),
        "hash" => PolicyAction::Mask(MaskMethod::Hash),
        "nullify" => PolicyAction::Mask(MaskMethod::Nullify),
        "deny" => PolicyAction::Deny,
        _ => return Err("Invalid action".into()),
    };

    let policy = Policy {
        id: uuid::Uuid::nil(),
        name: name.into(),
        description: None,
        policy_type,
        rules: vec![PolicyRule {
            dataset_pattern: dataset_pattern.into(),
            column_pattern: None,
            condition: None,
            action,
            roles,
        }],
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = state.store.create_policy(policy).await.map_err(|e| format!("Error: {e}"))?;
    serde_json::to_value(created).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_crawl(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_id(params, "datasource_id")?;
    let ds = state.store.get_datasource(id).await.map_err(|e| format!("Datasource not found: {e}"))?;

    let run = state.crawler_registry.crawl_and_persist(&ds, &state.store).await.map_err(|e| format!("Crawl failed: {e}"))?;
    serde_json::to_value(run).map_err(|e| format!("Serialize error: {e}"))
}

async fn handle_doc_generate(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let id = get_id(params, "dataset_id")?;
    let ds = state.store.get_dataset(id).await.map_err(|e| format!("Dataset not found: {e}"))?;

    if let Some(ref llm) = state.llm {
        let docs = llm.generate_documentation(&ds).await.map_err(|e| format!("LLM error: {e}"))?;
        serde_json::to_value(docs).map_err(|e| format!("Serialize error: {e}"))
    } else {
        Err("LLM not configured".into())
    }
}

async fn handle_semantic_search(state: &AppState, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let query = params.get("query").and_then(|v| v.as_str()).ok_or("Missing 'query'")?;

    let llm = state.llm.as_ref().ok_or("LLM not configured")?;
    let pagination = opencatalog_core::types::PaginationParams { offset: 0, limit: 1000 };
    let datasets = state.store.list_datasets(None, &pagination).await.map_err(|e| format!("Error: {e}"))?;
    let results = llm.semantic_search(query, &datasets.data).await.map_err(|e| format!("LLM error: {e}"))?;

    let json_results: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(score, ds)| {
            serde_json::json!({
                "score": score,
                "dataset": ds.name,
                "dataset_id": ds.id,
            })
        })
        .collect();

    Ok(serde_json::json!({ "results": json_results }))
}
