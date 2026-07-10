use std::collections::HashMap;

use opencatalog_core::error::{CatalogError, CatalogResult};
use opencatalog_core::traits::CrawlResult;
use opencatalog_core::types::*;
use uuid::Uuid;

/// Crawler for OpenIngest REST API.
/// Discovers pipelines, their source/target schemas, and PII masking rules.
pub struct OpenIngestCrawler;

impl OpenIngestCrawler {
    pub async fn crawl(&self, config: &HashMap<String, String>) -> CatalogResult<CrawlResult> {
        let base_url = config.get("url").ok_or_else(|| {
            CatalogError::InvalidInput("OpenIngest crawler requires 'url' config".into())
        })?;
        let base = base_url.trim_end_matches('/');

        let client = reqwest::Client::new();

        // 1. List pipelines
        let pipelines: Vec<serde_json::Value> =
            match client.get(format!("{base}/api/v1/pipelines")).send().await {
                Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
                _ => vec![],
            };

        let mut datasets = Vec::new();
        let mut lineage_edges = Vec::new();
        let openlineage_events = Vec::new();

        for pipeline in &pipelines {
            let pipeline_id = pipeline["id"].as_str().unwrap_or("unknown");
            let pipeline_name = pipeline["name"].as_str().unwrap_or("unknown");

            // 2. Get pipeline details
            let detail_url = format!("{base}/api/v1/pipelines/{pipeline_id}");
            let detail: serde_json::Value = match client.get(&detail_url).send().await {
                Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
                _ => continue,
            };

            // Extract source info
            let source_type = detail["source"]["type"].as_str().unwrap_or("source");
            let source_config = &detail["source"]["config"];
            let source_table = source_config["table"]
                .as_str()
                .or_else(|| source_config["stream"].as_str())
                .unwrap_or("unknown_source");

            // Extract target info
            let target_type = detail["target"]["type"].as_str().unwrap_or("lakehouse");
            let target_table = detail["target"]["table"]
                .as_str()
                .unwrap_or("unknown_target");

            // Extract schema
            let source_cols = extract_schema(&detail["source"]["columns"]);
            let target_cols_source = extract_schema(&detail["target"]["columns"]);

            // Extract PII masking rules
            let masking_rules: Vec<serde_json::Value> = detail["transforms"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter(|t| t["type"].as_str() == Some("pii_mask"))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();

            let mut target_cols = target_cols_source.clone();
            for rule in &masking_rules {
                let col_name = rule["column"].as_str().unwrap_or("unknown");
                let method = rule["method"].as_str().unwrap_or("hash");
                // Mark column as masked with classification
                for col in &mut target_cols {
                    if col.name == col_name {
                        col.classification = Some("confidential".into());
                        col.tags.push("pii".into());
                        col.tags.push(format!("masked:{method}"));
                    }
                }
            }

            // Create source dataset
            let src_ds = Dataset {
                id: Uuid::nil(),
                data_source_id: Uuid::nil(),
                name: format!("openingest.{source_type}.{source_table}"),
                physical_name: source_table.into(),
                dataset_type: DatasetType::Table,
                schema: source_cols,
                description: Some(format!("Source for pipeline '{pipeline_name}'")),
                tags: vec!["openingest".into(), format!("pipeline:{pipeline_name}")],
                classification: Some("internal".into()),
                location: None,
                row_count: None,
                last_crawled_at: Some(chrono::Utc::now()),
                version: 1,
                metadata: std::collections::HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            // Create target dataset
            let tgt_ds = Dataset {
                id: Uuid::nil(),
                data_source_id: Uuid::nil(),
                name: format!("openingest.{target_type}.{target_table}"),
                physical_name: target_table.into(),
                dataset_type: DatasetType::Table,
                schema: target_cols,
                description: Some(format!("Target for pipeline '{pipeline_name}'")),
                tags: vec!["openingest".into(), format!("pipeline:{pipeline_name}")],
                classification: Some("internal".into()),
                location: None,
                row_count: None,
                last_crawled_at: Some(chrono::Utc::now()),
                version: 1,
                metadata: std::collections::HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            // Record lineage edge (source → target)
            lineage_edges.push(LineageEdge {
                id: Uuid::nil(),
                source_node_id: Uuid::nil(),
                target_node_id: Uuid::nil(),
                transformation_type: "DIRECT".into(),
                transformation_subtype: "TRANSFORMATION".into(),
                transformation_sql: None,
                openlineage_run_id: None,
                job_name: Some(pipeline_name.into()),
                created_at: chrono::Utc::now(),
            });

            datasets.push(src_ds);
            datasets.push(tgt_ds);
        }

        Ok(CrawlResult {
            datasets,
            lineage_edges,
            openlineage_events,
        })
    }
}

fn extract_schema(columns_val: &serde_json::Value) -> Vec<Column> {
    columns_val
        .as_array()
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, col)| Column {
                    id: Uuid::nil(),
                    dataset_id: Uuid::nil(),
                    name: col["name"].as_str().unwrap_or("unknown").into(),
                    column_type: col["type"].as_str().unwrap_or("string").into(),
                    description: col["description"].as_str().map(String::from),
                    is_nullable: col["nullable"].as_bool().unwrap_or(true),
                    is_primary_key: col["primary_key"].as_bool().unwrap_or(false),
                    is_foreign_key: false,
                    ordinal_position: i as i32,
                    classification: col["classification"].as_str().map(String::from),
                    tags: col["tags"]
                        .as_array()
                        .map(|t| {
                            t.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    glossary_term_id: None,
                    metadata: std::collections::HashMap::new(),
                })
                .collect()
        })
        .unwrap_or_default()
}
