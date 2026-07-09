use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CatalogResult;
use crate::types::*;

#[async_trait]
pub trait CatalogStore: Send + Sync {
    // DataSources
    async fn create_datasource(&self, ds: DataSource) -> CatalogResult<DataSource>;
    async fn get_datasource(&self, id: Uuid) -> CatalogResult<DataSource>;
    async fn list_datasources(&self) -> CatalogResult<PaginatedResponse<DataSource>>;
    async fn update_datasource(&self, ds: DataSource) -> CatalogResult<DataSource>;
    async fn delete_datasource(&self, id: Uuid) -> CatalogResult<()>;

    // Datasets
    async fn create_dataset(&self, ds: Dataset) -> CatalogResult<Dataset>;
    async fn get_dataset(&self, id: Uuid) -> CatalogResult<Dataset>;
    async fn get_dataset_by_name(&self, name: &str) -> CatalogResult<Dataset>;
    async fn list_datasets(&self, datasource_id: Option<Uuid>, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<Dataset>>;
    async fn search_datasets(&self, query: &str, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<Dataset>>;
    async fn update_dataset(&self, ds: Dataset) -> CatalogResult<Dataset>;
    async fn delete_dataset(&self, id: Uuid) -> CatalogResult<()>;

    // Columns
    async fn get_columns(&self, dataset_id: Uuid) -> CatalogResult<Vec<Column>>;
    async fn update_column(&self, col: Column) -> CatalogResult<Column>;

    // Metadata
    async fn set_metadata(&self, entry: MetadataEntry) -> CatalogResult<()>;
    async fn get_metadata(&self, dataset_id: Uuid, column_id: Option<Uuid>) -> CatalogResult<HashMap<String, String>>;
    async fn delete_metadata(&self, dataset_id: Uuid, key: &str, column_id: Option<Uuid>) -> CatalogResult<()>;

    // Audit
    async fn append_audit_entry(&self, entry: AuditEntry) -> CatalogResult<()>;
    async fn list_audit_entries(&self, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<AuditEntry>>;

    // Schema Versions
    async fn get_schema_versions(&self, dataset_id: Uuid) -> CatalogResult<Vec<SchemaVersion>>;
    async fn diff_schema(&self, dataset_id: Uuid, from_version: i32, to_version: i32) -> CatalogResult<String>;

    // Lineage
    async fn add_lineage_node(&self, node: LineageNode) -> CatalogResult<LineageNode>;
    async fn add_lineage_edge(&self, edge: LineageEdge) -> CatalogResult<LineageEdge>;
    async fn get_lineage(
        &self,
        dataset_id: Uuid,
        direction: LineageDirection,
    ) -> CatalogResult<LineageGraph>;
    async fn get_column_lineage(
        &self,
        dataset_id: Uuid,
        column_name: &str,
    ) -> CatalogResult<Vec<ColumnLineageInfo>>;

    // OpenLineage events
    async fn ingest_openlineage_event(&self, event: OpenLineageEvent) -> CatalogResult<()>;

    // Glossary
    async fn create_glossary_term(&self, term: GlossaryTerm) -> CatalogResult<GlossaryTerm>;
    async fn get_glossary_term(&self, id: Uuid) -> CatalogResult<GlossaryTerm>;
    async fn list_glossary_terms(&self, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<GlossaryTerm>>;
    async fn update_glossary_term(&self, term: GlossaryTerm) -> CatalogResult<GlossaryTerm>;
    async fn delete_glossary_term(&self, id: Uuid) -> CatalogResult<()>;
    async fn create_term_mapping(&self, mapping: TermMapping) -> CatalogResult<TermMapping>;
    async fn get_term_mappings(&self, term_id: Uuid) -> CatalogResult<Vec<TermMapping>>;

    // Policies
    async fn create_policy(&self, policy: Policy) -> CatalogResult<Policy>;
    async fn get_policy(&self, id: Uuid) -> CatalogResult<Policy>;
    async fn list_policies(&self, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<Policy>>;
    async fn update_policy(&self, policy: Policy) -> CatalogResult<Policy>;
    async fn delete_policy(&self, id: Uuid) -> CatalogResult<()>;
    async fn get_active_policies(&self) -> CatalogResult<Vec<Policy>>;

    // Crawls
    async fn create_crawl_run(&self, run: CrawlRun) -> CatalogResult<CrawlRun>;
    async fn update_crawl_run(&self, run: CrawlRun) -> CatalogResult<CrawlRun>;
    async fn list_crawl_runs(&self, datasource_id: Uuid, pagination: &PaginationParams) -> CatalogResult<PaginatedResponse<CrawlRun>>;

    // Search
    async fn search(&self, query: &str, limit: usize) -> CatalogResult<SearchResults>;

    // Metrics
    async fn get_metrics(&self) -> CatalogResult<MetricsSnapshot>;
}

pub enum LineageDirection {
    Upstream,
    Downstream,
    Both,
}

#[async_trait]
pub trait Crawler: Send + Sync {
    fn source_type(&self) -> SourceType;
    async fn crawl(&self, config: &HashMap<String, String>) -> CatalogResult<CrawlResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub datasets: Vec<Dataset>,
    pub lineage_edges: Vec<LineageEdge>,
    pub openlineage_events: Vec<OpenLineageEvent>,
}

#[async_trait]
pub trait PolicyEngine: Send + Sync {
    async fn evaluate(
        &self,
        request: &PolicyEvalRequest,
    ) -> CatalogResult<PolicyEvalResult>;
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate_documentation(
        &self,
        dataset: &Dataset,
    ) -> CatalogResult<HashMap<String, String>>;
    async fn semantic_search(
        &self,
        query: &str,
        datasets: &[Dataset],
    ) -> CatalogResult<Vec<(f64, Dataset)>>;
    async fn generate_embeddings(&self, texts: &[String]) -> CatalogResult<Vec<Vec<f64>>>;
}
