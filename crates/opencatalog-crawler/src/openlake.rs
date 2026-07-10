use std::collections::HashMap;

use opencatalog_core::error::{CatalogError, CatalogResult};
use opencatalog_core::traits::CrawlResult;
use opencatalog_core::types::*;
use uuid::Uuid;

/// Crawler for OpenLake Iceberg REST API.
/// Discovers namespaces, tables, schemas, and snapshots.
pub struct OpenLakeCrawler;

impl OpenLakeCrawler {
    pub async fn crawl(&self, config: &HashMap<String, String>) -> CatalogResult<CrawlResult> {
        let base_url = config.get("url").ok_or_else(|| {
            CatalogError::InvalidInput("OpenLake crawler requires 'url' config".into())
        })?;

        let client = reqwest::Client::new();

        // 1. List namespaces via Iceberg REST API
        let namespaces_url = format!("{}/api/v1/namespaces", base_url.trim_end_matches('/'));
        let namespaces: Vec<String> = match client.get(&namespaces_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                json["namespaces"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|ns| ns["namespace"].as_array())
                            .flatten()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(|| vec!["default".into()])
            }
            _ => vec!["default".into()],
        };

        let mut datasets = Vec::new();
        let lineage_edges = Vec::new();

        // 2. For each namespace, list tables
        for namespace in &namespaces {
            let tables_url = format!(
                "{}/api/v1/namespaces/{}/tables",
                base_url.trim_end_matches('/'),
                namespace
            );
            let tables: Vec<serde_json::Value> = match client.get(&tables_url).send().await {
                Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
                _ => vec![],
            };

            for table_val in tables {
                let table_name = table_val["name"].as_str().unwrap_or("unknown").to_string();
                let full_name = format!("{namespace}.{table_name}");

                // 3. Get table details including schema
                let detail_url = format!(
                    "{}/api/v1/namespaces/{}/tables/{}",
                    base_url.trim_end_matches('/'),
                    namespace,
                    &table_name
                );

                let (schema_cols, location, _snapshot_id) = match client
                    .get(&detail_url)
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        let detail: serde_json::Value = resp.json().await.unwrap_or_default();
                        let loc = detail["metadata-location"].as_str().map(String::from);
                        let sid = detail["current-snapshot-id"].as_i64();

                        let cols = detail["schema"]["fields"]
                            .as_array()
                            .map(|fields| {
                                fields
                                    .iter()
                                    .enumerate()
                                    .map(|(i, f)| Column {
                                        id: Uuid::nil(),
                                        dataset_id: Uuid::nil(),
                                        name: f["name"].as_str().unwrap_or("unknown").into(),
                                        column_type: f["type"].as_str().unwrap_or("string").into(),
                                        description: f["doc"].as_str().map(String::from),
                                        is_nullable: f["required"].as_bool().unwrap_or(true),
                                        is_primary_key: false,
                                        is_foreign_key: false,
                                        ordinal_position: i as i32,
                                        classification: None,
                                        tags: vec![],
                                        glossary_term_id: None,
                                        metadata: HashMap::new(),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        (cols, loc, sid)
                    }
                    _ => (vec![], None, None),
                };

                let ds = Dataset {
                    id: Uuid::nil(),
                    data_source_id: Uuid::nil(),
                    name: full_name.clone(),
                    physical_name: full_name.clone(),
                    dataset_type: DatasetType::Table,
                    schema: schema_cols,
                    description: None,
                    tags: vec!["openlake".into()],
                    classification: Some("internal".into()),
                    location,
                    row_count: None,
                    last_crawled_at: Some(chrono::Utc::now()),
                    version: 1,
                    metadata: std::collections::HashMap::new(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                datasets.push(ds);
            }
        }

        Ok(CrawlResult {
            datasets,
            lineage_edges,
            openlineage_events: vec![],
        })
    }
}
