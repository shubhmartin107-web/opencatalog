use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use opencatalog_core::error::{CatalogError, CatalogResult};
use opencatalog_core::traits::{CatalogStore, LineageDirection};
use opencatalog_core::types::*;
use parking_lot::RwLock;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use uuid::Uuid;

type Graph = DiGraph<LineageNode, LineageEdge>;



#[derive(Default)]
struct Inner {
    datasources: HashMap<Uuid, DataSource>,
    datasets: HashMap<Uuid, Dataset>,
    columns: HashMap<Uuid, Vec<Column>>,
    glossary_terms: HashMap<Uuid, GlossaryTerm>,
    term_mappings: HashMap<Uuid, Vec<TermMapping>>,
    policies: HashMap<Uuid, Policy>,
    lineage_graph: Graph,
    lineage_node_map: HashMap<Uuid, NodeIndex>,
    crawl_runs: HashMap<Uuid, Vec<CrawlRun>>,
    openlineage_events: Vec<OpenLineageEvent>,
    metadata: HashMap<(Uuid, Option<Uuid>), HashMap<String, String>>,
    audit_entries: Vec<AuditEntry>,
    schema_versions: HashMap<Uuid, Vec<SchemaVersion>>,
    started_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Default)]
pub struct MemoryCatalogStore {
    inner: Arc<RwLock<Inner>>,
}

impl MemoryCatalogStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                started_at: Utc::now(),
                ..Default::default()
            })),
        }
    }
}

#[async_trait]
impl CatalogStore for MemoryCatalogStore {
    // ---- DataSources ----
    async fn create_datasource(&self, mut ds: DataSource) -> CatalogResult<DataSource> {
        let now = Utc::now();
        ds.id = Uuid::now_v7();
        ds.created_at = now;
        ds.updated_at = now;
        let mut inner = self.inner.write();
        if inner.datasources.values().any(|d| d.name == ds.name) {
            return Err(CatalogError::AlreadyExists(format!("datasource '{}'", ds.name)));
        }
        inner.datasources.insert(ds.id, ds.clone());
        Ok(ds)
    }

    async fn get_datasource(&self, id: Uuid) -> CatalogResult<DataSource> {
        self.inner
            .read()
            .datasources
            .get(&id)
            .cloned()
            .ok_or_else(|| CatalogError::NotFound(format!("datasource {id}")))
    }

    async fn list_datasources(&self) -> CatalogResult<PaginatedResponse<DataSource>> {
        let items: Vec<DataSource> = self.inner.read().datasources.values().cloned().collect();
        let total = items.len() as u64;
        Ok(PaginatedResponse { data: items, total, offset: 0, limit: total.max(1) })
    }

    async fn update_datasource(&self, mut ds: DataSource) -> CatalogResult<DataSource> {
        ds.updated_at = Utc::now();
        let mut inner = self.inner.write();
        if !inner.datasources.contains_key(&ds.id) {
            return Err(CatalogError::NotFound(format!("datasource {}", ds.id)));
        }
        inner.datasources.insert(ds.id, ds.clone());
        Ok(ds)
    }

    async fn delete_datasource(&self, id: Uuid) -> CatalogResult<()> {
        let mut inner = self.inner.write();
        inner.datasources.remove(&id);
        inner.datasets.retain(|_, d| d.data_source_id != id);
        Ok(())
    }

    // ---- Datasets ----
    async fn create_dataset(&self, mut ds: Dataset) -> CatalogResult<Dataset> {
        let now = Utc::now();
        ds.id = Uuid::now_v7();
        ds.created_at = now;
        ds.updated_at = now;
        ds.version = 1;
        for col in &mut ds.schema {
            col.id = Uuid::now_v7();
            col.dataset_id = ds.id;
        }
        let mut inner = self.inner.write();
        if inner.datasets.values().any(|d| d.name == ds.name) {
            return Err(CatalogError::AlreadyExists(format!("dataset '{}'", ds.name)));
        }
        let cols = ds.schema.clone();
        inner.datasets.insert(ds.id, ds.clone());

        let schema_version = SchemaVersion {
            dataset_id: ds.id,
            version: ds.version,
            schema: cols.clone(),
            diff_from_previous: None,
            created_at: now,
        };
        inner.schema_versions.entry(ds.id).or_default().push(schema_version);

        inner.columns.insert(ds.id, cols);
        Ok(ds)
    }

    async fn get_dataset(&self, id: Uuid) -> CatalogResult<Dataset> {
        self.inner
            .read()
            .datasets
            .get(&id)
            .cloned()
            .ok_or_else(|| CatalogError::NotFound(format!("dataset {id}")))
    }

    async fn get_dataset_by_name(&self, name: &str) -> CatalogResult<Dataset> {
        self.inner
            .read()
            .datasets
            .values()
            .find(|d| d.name == name)
            .cloned()
            .ok_or_else(|| CatalogError::NotFound(format!("dataset '{name}'")))
    }

    async fn list_datasets(&self, datasource_id: Option<Uuid>, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<Dataset>> {
        let inner = self.inner.read();
        let items: Vec<Dataset> = match datasource_id {
            Some(id) => inner.datasets.values().filter(|d| d.data_source_id == id).cloned().collect(),
            None => inner.datasets.values().cloned().collect(),
        };
        Ok(apply_paginated(items, pagination))
    }

    async fn search_datasets(&self, query: &str, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<Dataset>> {
        let q = query.to_lowercase();
        let inner = self.inner.read();
        let items: Vec<Dataset> = inner
            .datasets
            .values()
            .filter(|d| {
                d.name.to_lowercase().contains(&q)
                    || d.description.as_deref().map_or(false, |s| s.to_lowercase().contains(&q))
                    || d.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();
        Ok(apply_paginated(items, pagination))
    }

    async fn update_dataset(&self, mut ds: Dataset) -> CatalogResult<Dataset> {
        ds.updated_at = Utc::now();
        let mut inner = self.inner.write();
        if !inner.datasets.contains_key(&ds.id) {
            return Err(CatalogError::NotFound(format!("dataset {}", ds.id)));
        }

        let existing = inner.datasets.get(&ds.id).cloned().unwrap();
        let prev_version = existing.version;
        ds.version = prev_version + 1;

        let cols = ds.schema.clone();
        inner.datasets.insert(ds.id, ds.clone());
        inner.columns.insert(ds.id, cols.clone());

        let diff = compute_schema_diff(&existing.schema, &cols);

        let schema_version = SchemaVersion {
            dataset_id: ds.id,
            version: ds.version,
            schema: cols,
            diff_from_previous: Some(diff),
            created_at: Utc::now(),
        };
        inner.schema_versions.entry(ds.id).or_default().push(schema_version);

        Ok(ds)
    }

    async fn delete_dataset(&self, id: Uuid) -> CatalogResult<()> {
        let mut inner = self.inner.write();
        inner.datasets.remove(&id);
        inner.columns.remove(&id);
        inner.schema_versions.remove(&id);
        inner.metadata.retain(|(ds_id, _), _| *ds_id != id);
        Ok(())
    }

    // ---- Columns ----
    async fn get_columns(&self, dataset_id: Uuid) -> CatalogResult<Vec<Column>> {
        Ok(self
            .inner
            .read()
            .columns
            .get(&dataset_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn update_column(&self, mut col: Column) -> CatalogResult<Column> {
        let mut inner = self.inner.write();
        if let Some(cols) = inner.columns.get_mut(&col.dataset_id) {
            if let Some(existing) = cols.iter_mut().find(|c| c.id == col.id) {
                col.ordinal_position = existing.ordinal_position;
                *existing = col.clone();
                return Ok(col);
            }
        }
        Err(CatalogError::NotFound(format!("column {}", col.id)))
    }

    // ---- Metadata ----
    async fn set_metadata(&self, entry: MetadataEntry) -> CatalogResult<()> {
        let mut inner = self.inner.write();
        let map = inner
            .metadata
            .entry((entry.dataset_id, entry.column_id))
            .or_default();
        map.insert(entry.key, entry.value);
        Ok(())
    }

    async fn get_metadata(&self, dataset_id: Uuid, column_id: Option<Uuid>) -> CatalogResult<HashMap<String, String>> {
        Ok(self
            .inner
            .read()
            .metadata
            .get(&(dataset_id, column_id))
            .cloned()
            .unwrap_or_default())
    }

    async fn delete_metadata(&self, dataset_id: Uuid, key: &str, column_id: Option<Uuid>) -> CatalogResult<()> {
        let mut inner = self.inner.write();
        if let Some(map) = inner.metadata.get_mut(&(dataset_id, column_id)) {
            map.remove(key);
            if map.is_empty() {
                inner.metadata.remove(&(dataset_id, column_id));
            }
        }
        Ok(())
    }

    // ---- Audit ----
    async fn append_audit_entry(&self, entry: AuditEntry) -> CatalogResult<()> {
        self.inner.write().audit_entries.push(entry);
        Ok(())
    }

    async fn list_audit_entries(&self, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<AuditEntry>> {
        let inner = self.inner.read();
        let items = inner.audit_entries.clone();
        Ok(apply_paginated(items, pagination))
    }

    // ---- Schema Versions ----
    async fn get_schema_versions(&self, dataset_id: Uuid) -> CatalogResult<Vec<SchemaVersion>> {
        Ok(self
            .inner
            .read()
            .schema_versions
            .get(&dataset_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn diff_schema(&self, dataset_id: Uuid, from_version: i32, to_version: i32) -> CatalogResult<String> {
        let inner = self.inner.read();
        let versions = inner
            .schema_versions
            .get(&dataset_id)
            .ok_or_else(|| CatalogError::NotFound(format!("schema versions for dataset {dataset_id}")))?;

        let from = versions
            .iter()
            .find(|v| v.version == from_version)
            .ok_or_else(|| CatalogError::NotFound(format!("schema version {from_version}")))?
            .schema
            .clone();
        let to = versions
            .iter()
            .find(|v| v.version == to_version)
            .ok_or_else(|| CatalogError::NotFound(format!("schema version {to_version}")))?
            .schema
            .clone();

        Ok(compute_schema_diff(&from, &to))
    }

    // ---- Lineage ----
    async fn add_lineage_node(&self, mut node: LineageNode) -> CatalogResult<LineageNode> {
        node.id = Uuid::now_v7();
        let mut inner = self.inner.write();
        let idx = inner.lineage_graph.add_node(node.clone());
        inner.lineage_node_map.insert(node.id, idx);
        Ok(node)
    }

    async fn add_lineage_edge(&self, mut edge: LineageEdge) -> CatalogResult<LineageEdge> {
        edge.id = Uuid::now_v7();
        edge.created_at = Utc::now();
        let mut inner = self.inner.write();
        let src = inner
            .lineage_node_map
            .get(&edge.source_node_id)
            .copied()
            .ok_or_else(|| CatalogError::NotFound(format!("lineage node {}", edge.source_node_id)))?;
        let dst = inner
            .lineage_node_map
            .get(&edge.target_node_id)
            .copied()
            .ok_or_else(|| CatalogError::NotFound(format!("lineage node {}", edge.target_node_id)))?;
        inner.lineage_graph.add_edge(src, dst, edge.clone());
        Ok(edge)
    }

    async fn get_lineage(
        &self,
        dataset_id: Uuid,
        direction: LineageDirection,
    ) -> CatalogResult<LineageGraph> {
        let inner = self.inner.read();
        let node_idx = inner
            .lineage_node_map
            .values()
            .find(|&&idx| inner.lineage_graph[idx].dataset_id == dataset_id)
            .copied();

        let Some(start) = node_idx else {
            return Ok(LineageGraph {
                nodes: vec![],
                edges: vec![],
            });
        };

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![start];
        visited.insert(start);

        while let Some(idx) = queue.pop() {
            nodes.push(inner.lineage_graph[idx].clone());

            let iter: Box<dyn Iterator<Item = _>> = match direction {
                LineageDirection::Upstream => {
                    Box::new(inner.lineage_graph.neighbors_directed(idx, Direction::Incoming))
                }
                LineageDirection::Downstream => {
                    Box::new(inner.lineage_graph.neighbors_directed(idx, Direction::Outgoing))
                }
                LineageDirection::Both => Box::new(
                    inner
                        .lineage_graph
                        .neighbors_directed(idx, Direction::Incoming)
                        .chain(
                            inner
                                .lineage_graph
                                .neighbors_directed(idx, Direction::Outgoing),
                        ),
                ),
            };

            for neighbor in iter {
                let edge_idx = inner
                    .lineage_graph
                    .find_edge(idx, neighbor)
                    .or_else(|| inner.lineage_graph.find_edge(neighbor, idx));
                if let Some(ei) = edge_idx {
                    edges.push(inner.lineage_graph[ei].clone());
                }
                if visited.insert(neighbor) {
                    queue.push(neighbor);
                }
            }
        }

        Ok(LineageGraph { nodes, edges })
    }

    async fn get_column_lineage(
        &self,
        dataset_id: Uuid,
        column_name: &str,
    ) -> CatalogResult<Vec<ColumnLineageInfo>> {
        let inner = self.inner.read();
        let mut results = Vec::new();
        for edge in inner.lineage_graph.edge_weights() {
            let Some(&src_idx) = inner.lineage_node_map.get(&edge.source_node_id) else { continue };
            let Some(&dst_idx) = inner.lineage_node_map.get(&edge.target_node_id) else { continue };
            let src = &inner.lineage_graph[src_idx];
            let dst = &inner.lineage_graph[dst_idx];
            if dst.dataset_id == dataset_id {
                if let Some(col_id) = dst.column_id {
                    if let Some(cols) = inner.columns.get(&dataset_id) {
                        if let Some(col) = cols.iter().find(|c| c.id == col_id) {
                            if col.name == column_name {
                                let src_name = inner
                                    .datasets
                                    .get(&src.dataset_id)
                                    .map(|d| d.name.clone())
                                    .unwrap_or_default();
                                results.push(ColumnLineageInfo {
                                    source_dataset: src_name.clone(),
                                    source_column: src.label.clone(),
                                    target_dataset: dst.label.clone(),
                                    target_column: column_name.to_string(),
                                    transformation_type: edge.transformation_type.clone(),
                                    transformation_subtype: edge.transformation_subtype.clone(),
                                    transformation_sql: edge.transformation_sql.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    // ---- OpenLineage ----
    async fn ingest_openlineage_event(&self, event: OpenLineageEvent) -> CatalogResult<()> {
        self.inner.write().openlineage_events.push(event);
        Ok(())
    }

    // ---- Glossary ----
    async fn create_glossary_term(&self, mut term: GlossaryTerm) -> CatalogResult<GlossaryTerm> {
        let now = Utc::now();
        term.id = Uuid::now_v7();
        term.created_at = now;
        term.updated_at = now;
        let mut inner = self.inner.write();
        if inner.glossary_terms.values().any(|t| t.name == term.name) {
            return Err(CatalogError::AlreadyExists(format!("glossary term '{}'", term.name)));
        }
        inner.glossary_terms.insert(term.id, term.clone());
        Ok(term)
    }

    async fn get_glossary_term(&self, id: Uuid) -> CatalogResult<GlossaryTerm> {
        self.inner
            .read()
            .glossary_terms
            .get(&id)
            .cloned()
            .ok_or_else(|| CatalogError::NotFound(format!("glossary term {id}")))
    }

    async fn list_glossary_terms(&self, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<GlossaryTerm>> {
        let inner = self.inner.read();
        let items: Vec<GlossaryTerm> = inner.glossary_terms.values().cloned().collect();
        Ok(apply_paginated(items, pagination))
    }

    async fn update_glossary_term(&self, mut term: GlossaryTerm) -> CatalogResult<GlossaryTerm> {
        term.updated_at = Utc::now();
        let mut inner = self.inner.write();
        if !inner.glossary_terms.contains_key(&term.id) {
            return Err(CatalogError::NotFound(format!("glossary term {}", term.id)));
        }
        inner.glossary_terms.insert(term.id, term.clone());
        Ok(term)
    }

    async fn delete_glossary_term(&self, id: Uuid) -> CatalogResult<()> {
        let mut inner = self.inner.write();
        inner.glossary_terms.remove(&id);
        inner.term_mappings.remove(&id);
        Ok(())
    }

    async fn create_term_mapping(&self, mut mapping: TermMapping) -> CatalogResult<TermMapping> {
        mapping.id = Uuid::now_v7();
        mapping.created_at = Utc::now();
        let mut inner = self.inner.write();
        inner
            .term_mappings
            .entry(mapping.term_id)
            .or_default()
            .push(mapping.clone());
        Ok(mapping)
    }

    async fn get_term_mappings(&self, term_id: Uuid) -> CatalogResult<Vec<TermMapping>> {
        Ok(self
            .inner
            .read()
            .term_mappings
            .get(&term_id)
            .cloned()
            .unwrap_or_default())
    }

    // ---- Policies ----
    async fn create_policy(&self, mut policy: Policy) -> CatalogResult<Policy> {
        let now = Utc::now();
        policy.id = Uuid::now_v7();
        policy.created_at = now;
        policy.updated_at = now;
        let mut inner = self.inner.write();
        if inner.policies.values().any(|p| p.name == policy.name) {
            return Err(CatalogError::AlreadyExists(format!("policy '{}'", policy.name)));
        }
        inner.policies.insert(policy.id, policy.clone());
        Ok(policy)
    }

    async fn get_policy(&self, id: Uuid) -> CatalogResult<Policy> {
        self.inner
            .read()
            .policies
            .get(&id)
            .cloned()
            .ok_or_else(|| CatalogError::NotFound(format!("policy {id}")))
    }

    async fn list_policies(&self, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<Policy>> {
        let inner = self.inner.read();
        let items: Vec<Policy> = inner.policies.values().cloned().collect();
        Ok(apply_paginated(items, pagination))
    }

    async fn update_policy(&self, mut policy: Policy) -> CatalogResult<Policy> {
        policy.updated_at = Utc::now();
        let mut inner = self.inner.write();
        if !inner.policies.contains_key(&policy.id) {
            return Err(CatalogError::NotFound(format!("policy {}", policy.id)));
        }
        inner.policies.insert(policy.id, policy.clone());
        Ok(policy)
    }

    async fn delete_policy(&self, id: Uuid) -> CatalogResult<()> {
        self.inner.write().policies.remove(&id);
        Ok(())
    }

    async fn get_active_policies(&self) -> CatalogResult<Vec<Policy>> {
        Ok(self
            .inner
            .read()
            .policies
            .values()
            .filter(|p| p.enabled)
            .cloned()
            .collect())
    }

    // ---- Crawls ----
    async fn create_crawl_run(&self, mut run: CrawlRun) -> CatalogResult<CrawlRun> {
        run.id = Uuid::now_v7();
        run.started_at = Utc::now();
        let mut inner = self.inner.write();
        inner
            .crawl_runs
            .entry(run.data_source_id)
            .or_default()
            .push(run.clone());
        Ok(run)
    }

    async fn update_crawl_run(&self, run: CrawlRun) -> CatalogResult<CrawlRun> {
        let mut inner = self.inner.write();
        if let Some(runs) = inner.crawl_runs.get_mut(&run.data_source_id) {
            if let Some(existing) = runs.iter_mut().find(|r| r.id == run.id) {
                *existing = run.clone();
                return Ok(run);
            }
        }
        Err(CatalogError::NotFound(format!("crawl run {}", run.id)))
    }

    async fn list_crawl_runs(&self, datasource_id: Uuid, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<CrawlRun>> {
        let inner = self.inner.read();
        let items = inner.crawl_runs.get(&datasource_id).cloned().unwrap_or_default();
        Ok(apply_paginated(items, pagination))
    }

    // ---- Search ----
    async fn search(&self, query: &str, limit: usize) -> CatalogResult<SearchResults> {
        let q = query.to_lowercase();
        let inner = self.inner.read();
        let mut results = Vec::new();

        for ds in inner.datasets.values() {
            let score = if ds.name.to_lowercase() == q {
                1.0
            } else if ds.name.to_lowercase().contains(&q) {
                0.8
            } else if ds.description.as_deref().map_or(false, |s| s.to_lowercase().contains(&q)) {
                0.6
            } else if ds.tags.iter().any(|t| t.to_lowercase().contains(&q)) {
                0.5
            } else {
                continue;
            };
            results.push(SearchResult {
                dataset_id: Some(ds.id),
                column_id: None,
                glossary_term_id: None,
                name: ds.name.clone(),
                description: ds.description.clone(),
                score,
                kind: "dataset".into(),
            });
        }

        for cols in inner.columns.values() {
            for col in cols {
                if col.name.to_lowercase().contains(&q)
                    || col.description.as_deref().map_or(false, |s| s.to_lowercase().contains(&q))
                {
                    if let Some(ds) = inner.datasets.get(&col.dataset_id) {
                        results.push(SearchResult {
                            dataset_id: Some(ds.id),
                            column_id: Some(col.id),
                            glossary_term_id: None,
                            name: format!("{}.{}", ds.name, col.name),
                            description: col.description.clone(),
                            score: 0.7,
                            kind: "column".into(),
                        });
                    }
                }
            }
        }

        for term in inner.glossary_terms.values() {
            if term.name.to_lowercase().contains(&q)
                || term.description.to_lowercase().contains(&q)
                || term.synonyms.iter().any(|s| s.to_lowercase().contains(&q))
            {
                results.push(SearchResult {
                    dataset_id: None,
                    column_id: None,
                    glossary_term_id: Some(term.id),
                    name: term.name.clone(),
                    description: Some(term.description.clone()),
                    score: 0.9,
                    kind: "glossary_term".into(),
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        let total = results.len();
        Ok(SearchResults { results, total })
    }

    // ---- Metrics ----
    async fn get_metrics(&self) -> CatalogResult<MetricsSnapshot> {
        let inner = self.inner.read();
        let column_count: usize = inner.columns.values().map(|v| v.len()).sum();
        let lineage_edge_count = inner.lineage_graph.edge_count();
        let crawl_run_count: usize = inner.crawl_runs.values().map(|v| v.len()).sum();
        let uptime = Utc::now()
            .signed_duration_since(inner.started_at)
            .num_seconds() as u64;
        Ok(MetricsSnapshot {
            datasource_count: inner.datasources.len(),
            dataset_count: inner.datasets.len(),
            column_count,
            glossary_term_count: inner.glossary_terms.len(),
            policy_count: inner.policies.len(),
            lineage_edge_count,
            crawl_run_count,
            total_api_calls: inner.audit_entries.len() as u64,
            uptime_seconds: uptime,
        })
    }
}

fn apply_paginated<T: Clone + serde::Serialize>(items: Vec<T>, pagination: &PaginationParams) -> PaginatedResponse<T> {
    let total = items.len() as u64;
    let offset = pagination.offset;
    let limit = pagination.limit;
    let data: Vec<T> = items
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();
    PaginatedResponse { data, total, offset, limit }
}

fn compute_schema_diff(from: &[Column], to: &[Column]) -> String {
    let mut lines = Vec::new();

    let from_map: HashMap<&str, (&str, bool, bool, bool, Option<&str>)> = from
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                (
                    c.column_type.as_str(),
                    c.is_nullable,
                    c.is_primary_key,
                    c.is_foreign_key,
                    c.description.as_deref(),
                ),
            )
        })
        .collect();
    let to_map: HashMap<&str, (&str, bool, bool, bool, Option<&str>)> = to
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                (
                    c.column_type.as_str(),
                    c.is_nullable,
                    c.is_primary_key,
                    c.is_foreign_key,
                    c.description.as_deref(),
                ),
            )
        })
        .collect();

    for col in to {
        if !from_map.contains_key(col.name.as_str()) {
            lines.push(format!("+ {} {} (added)", col.name, col.column_type));
        }
    }

    for col in from {
        if !to_map.contains_key(col.name.as_str()) {
            lines.push(format!("- {} {} (removed)", col.name, col.column_type));
        }
    }

    for col in to {
        if let Some(&(old_type, old_null, old_pk, old_fk, _old_desc)) = from_map.get(col.name.as_str())
        {
            let mut changed = false;
            let mut detail = String::new();

            if col.column_type != old_type {
                changed = true;
                detail.push_str(&format!("type: {} -> {}", old_type, col.column_type));
            }
            if col.is_nullable != old_null {
                changed = true;
                detail.push_str(&format!(
                    ", nullable: {} -> {}",
                    old_null, col.is_nullable
                ));
            }
            if col.is_primary_key != old_pk {
                changed = true;
                detail.push_str(&format!(
                    ", primary_key: {} -> {}",
                    old_pk, col.is_primary_key
                ));
            }
            if col.is_foreign_key != old_fk {
                changed = true;
                detail.push_str(&format!(
                    ", foreign_key: {} -> {}",
                    old_fk, col.is_foreign_key
                ));
            }
            if changed {
                lines.push(format!("~ {} {} ({})", col.name, col.column_type, detail));
            }
        }
    }

    if lines.is_empty() {
        "no changes".to_string()
    } else {
        lines.join("\n")
    }
}