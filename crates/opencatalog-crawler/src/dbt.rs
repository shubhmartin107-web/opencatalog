use std::collections::HashMap;

use opencatalog_core::error::{CatalogError, CatalogResult};
use opencatalog_core::traits::CrawlResult;
use opencatalog_core::types::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Serde types for dbt manifest.json
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct Manifest {
    nodes: HashMap<String, Node>,
}

#[derive(serde::Deserialize)]
struct Node {
    name: String,
    resource_type: String,
    database: Option<String>,
    schema: Option<String>,
    alias: Option<String>,
    #[serde(default)]
    columns: HashMap<String, ColumnInfo>,
    depends_on: Option<DependsOn>,
    config: Option<NodeConfig>,
}

#[derive(serde::Deserialize)]
struct ColumnInfo {
    name: Option<String>,
    description: Option<String>,
    data_type: Option<String>,
}

#[derive(serde::Deserialize)]
struct DependsOn {
    nodes: Vec<String>,
}

#[derive(serde::Deserialize)]
struct NodeConfig {
    materialized: Option<String>,
}

// ---------------------------------------------------------------------------
// DBT Crawler
// ---------------------------------------------------------------------------

pub struct DbtCrawler;

impl DbtCrawler {
    pub async fn crawl(&self, config: &HashMap<String, String>) -> CatalogResult<CrawlResult> {
        let manifest_path = config
            .get("manifest_path")
            .ok_or_else(|| CatalogError::InvalidInput("DBT crawler requires 'manifest_path' config".into()))?;

        let catalog_path = config.get("catalog_path");

        let content = std::fs::read_to_string(manifest_path)
            .map_err(|e| CatalogError::CrawlFailed(format!("Failed to read manifest file: {e}")))?;

        let manifest: Manifest = serde_json::from_str(&content)
            .map_err(|e| CatalogError::CrawlFailed(format!("Failed to parse manifest JSON: {e}")))?;

        // Optionally read catalog.json for additional metadata
        let catalog_columns: HashMap<String, HashMap<String, CatalogColumn>> = if let Some(cat_path) = catalog_path {
            let cat_content = std::fs::read_to_string(cat_path)
                .map_err(|e| CatalogError::CrawlFailed(format!("Failed to read catalog file: {e}")))?;
            let catalog: Catalog = serde_json::from_str(&cat_content)
                .map_err(|e| CatalogError::CrawlFailed(format!("Failed to parse catalog JSON: {e}")))?;
            catalog.nodes
        } else {
            HashMap::new()
        };

        let mut datasets = Vec::new();
        let mut lineage_edges = Vec::new();
        let mut openlineage_events = Vec::new();

        // First pass: collect all nodes keyed by their node name and full_name
        let mut node_datasets: HashMap<String, String> = HashMap::new(); // node_key -> dataset_name

        for (node_key, node) in &manifest.nodes {
            if node.resource_type != "model" && node.resource_type != "source" {
                continue;
            }

            let physical_name = node.alias.as_deref().unwrap_or(&node.name);
            let schema_name = node.schema.as_deref().unwrap_or("default");
            let db_name = node.database.as_deref().unwrap_or("default");
            let full_name = format!("{db_name}.{schema_name}.{physical_name}");

            let materialized = node.config.as_ref().and_then(|c| c.materialized.as_deref());
            let dataset_type = match materialized {
                Some("view") => DatasetType::View,
                _ => DatasetType::Table,
            };

            let mut tags = vec!["dbt".into(), node.resource_type.clone()];
            if let Some(mat) = materialized {
                tags.push(format!("materialized:{mat}"));
            }

            let mut columns: Vec<Column> = Vec::new();

            for (ordinal, (col_key, col_info)) in node.columns.iter().enumerate() {
                let ordinal = ordinal as i32;
                let data_type = col_info.data_type.clone().unwrap_or_else(|| "string".into());
                let col_tags: Vec<String> = Vec::new();

                // Enrich with catalog metadata if available
                let mut col_metadata: HashMap<String, String> = HashMap::new();
                if let Some(node_catalog) = catalog_columns.get(node_key)
                    && let Some(cat_col) = node_catalog.get(col_key)
                {
                    if let Some(ct) = &cat_col.data_type {
                        col_metadata.insert("catalog_data_type".into(), ct.clone());
                    }
                    if let Some(comment) = &cat_col.comment {
                        col_metadata.insert("catalog_comment".into(), comment.clone());
                    }
                    if let Some(stats) = &cat_col.stats {
                        if let Some(distinct) = stats.get("distinct_values")
                            && let Some(v) = distinct.as_str()
                        {
                            col_metadata.insert("distinct_values".into(), v.into());
                        }
                        if let Some(nulls) = stats.get("null_count")
                            && let Some(v) = nulls.as_str()
                        {
                            col_metadata.insert("null_count".into(), v.into());
                        }
                    }
                }

                columns.push(Column {
                    id: Uuid::nil(),
                    dataset_id: Uuid::nil(),
                    name: col_info.name.clone().unwrap_or_else(|| col_key.clone()),
                    column_type: data_type,
                    description: col_info.description.clone(),
                    is_nullable: true,
                    is_primary_key: false,
                    is_foreign_key: false,
                    ordinal_position: ordinal,
                    classification: None,
                    tags: col_tags,
                    glossary_term_id: None,
                    metadata: col_metadata,
                });
            }

            let now = chrono::Utc::now();
            let ds = Dataset {
                id: Uuid::nil(),
                data_source_id: Uuid::nil(),
                name: full_name.clone(),
                physical_name: physical_name.into(),
                dataset_type,
                schema: columns,
                description: None,
                tags,
                classification: Some("internal".into()),
                location: None,
                row_count: None,
                last_crawled_at: Some(now),
                version: 1,
                metadata: HashMap::new(),
                created_at: now,
                updated_at: now,
            };

            datasets.push(ds);
            node_datasets.insert(node_key.clone(), full_name);
        }

        // Second pass: create lineage edges from depends_on
        for (node_key, node) in &manifest.nodes {
            if node.resource_type != "model" {
                continue;
            }

            if !node_datasets.contains_key(node_key) {
                continue;
            }

            if let Some(depends) = &node.depends_on {
                for parent_key in &depends.nodes {
                    if node_datasets.contains_key(parent_key) {
                        let now = chrono::Utc::now();
                        lineage_edges.push(LineageEdge {
                            id: Uuid::nil(),
                            source_node_id: Uuid::nil(),
                            target_node_id: Uuid::nil(),
                            transformation_type: "DIRECT".into(),
                            transformation_subtype: "DBT_DEPENDS".into(),
                            transformation_sql: None,
                            openlineage_run_id: None,
                            job_name: Some(format!("dbt:{}", node.name)),
                            created_at: now,
                        });
                    }
                }
            }
        }

        // Create OpenLineage events for each model
        for (node_key, node) in &manifest.nodes {
            if node.resource_type != "model" {
                continue;
            }

            let child_name = match node_datasets.get(node_key) {
                Some(n) => n.clone(),
                None => continue,
            };

            let mut inputs = Vec::new();
            if let Some(depends) = &node.depends_on {
                for parent_key in &depends.nodes {
                    if let Some(parent_name) = node_datasets.get(parent_key) {
                        let parts: Vec<&str> = parent_name.splitn(3, '.').collect();
                        let ns = if parts.len() >= 3 { parts[..parts.len()-1].join(".") } else { "default".into() };
                        inputs.push(OpenLineageDatasetRef {
                            namespace: ns,
                            name: parent_name.clone(),
                            facets: None,
                        });
                    }
                }
            }

            let parts: Vec<&str> = child_name.splitn(3, '.').collect();
            let ns = if parts.len() >= 3 { parts[..parts.len()-1].join(".") } else { "default".into() };

            let now = chrono::Utc::now();
            let event = OpenLineageEvent {
                event_type: "COMPLETE".into(),
                event_time: now.to_rfc3339(),
                producer: "opencatalog-crawler-dbt".into(),
                schema_url: "https://openlineage.io/spec/1-0-5/OpenLineage.json".into(),
                job: OpenLineageJob {
                    namespace: "dbt".into(),
                    name: node.name.clone(),
                    facets: None,
                },
                run: OpenLineageRun {
                    run_id: Uuid::now_v7().to_string(),
                    facets: None,
                },
                inputs,
                outputs: vec![OpenLineageDatasetRef {
                    namespace: ns,
                    name: child_name,
                    facets: None,
                }],
            };

            openlineage_events.push(event);
        }

        Ok(CrawlResult {
            datasets,
            lineage_edges,
            openlineage_events,
        })
    }
}

/// Minimal catalog.json types for enrichment
#[derive(serde::Deserialize)]
struct Catalog {
    nodes: HashMap<String, HashMap<String, CatalogColumn>>,
}

#[derive(serde::Deserialize)]
struct CatalogColumn {
    #[serde(rename = "type")]
    data_type: Option<String>,
    comment: Option<String>,
    stats: Option<HashMap<String, serde_json::Value>>,
}
