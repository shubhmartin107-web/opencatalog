use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub fn all_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "catalog_search".into(),
            description: "Search datasets, columns, and glossary terms by keyword".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"}
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "catalog_describe".into(),
            description: "Get detailed metadata for a dataset including schema".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dataset_id": {"type": "string", "description": "Dataset UUID"}
                },
                "required": ["dataset_id"]
            }),
        },
        McpTool {
            name: "catalog_lineage".into(),
            description: "Get upstream/downstream lineage for a dataset".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dataset_id": {"type": "string", "description": "Dataset UUID"},
                    "direction": {"type": "string", "enum": ["upstream", "downstream", "both"], "default": "both"}
                },
                "required": ["dataset_id"]
            }),
        },
        McpTool {
            name: "catalog_glossary_list".into(),
            description: "List all business glossary terms".into(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "catalog_glossary_create".into(),
            description: "Create a new business glossary term".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "description": {"type": "string"},
                    "domain": {"type": "string"}
                },
                "required": ["name", "description"]
            }),
        },
        McpTool {
            name: "catalog_policy_list".into(),
            description: "List all governance policies".into(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "catalog_policy_create".into(),
            description: "Create a masking or access policy".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "policy_type": {"type": "string", "enum": ["masking", "row_filter", "access"]},
                    "dataset_pattern": {"type": "string"},
                    "action": {"type": "string", "enum": ["redact", "hash", "nullify", "deny"]},
                    "roles": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["name", "policy_type", "dataset_pattern", "action", "roles"]
            }),
        },
        McpTool {
            name: "catalog_crawl".into(),
            description: "Trigger a metadata crawl for a data source".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "datasource_id": {"type": "string", "description": "Data source UUID"}
                },
                "required": ["datasource_id"]
            }),
        },
        McpTool {
            name: "catalog_doc_generate".into(),
            description: "Generate documentation for a dataset via LLM".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dataset_id": {"type": "string", "description": "Dataset UUID"}
                },
                "required": ["dataset_id"]
            }),
        },
        McpTool {
            name: "catalog_semantic_search".into(),
            description: "Search datasets using natural language via LLM embeddings".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Natural language query"}
                },
                "required": ["query"]
            }),
        },
    ]
}
