use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Data Sources
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    OpenLake,
    OpenIngest,
    OpenPipe,
    OpenStream,
    Custom(String),
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::OpenLake => write!(f, "openlake"),
            SourceType::OpenIngest => write!(f, "openingest"),
            SourceType::OpenPipe => write!(f, "openpipe"),
            SourceType::OpenStream => write!(f, "openstream"),
            SourceType::Custom(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub id: Uuid,
    pub name: String,
    pub source_type: SourceType,
    pub description: Option<String>,
    pub connection_config: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Datasets & Columns
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetType {
    Table,
    View,
    Topic,
    File,
    Stream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub id: Uuid,
    pub data_source_id: Uuid,
    pub name: String,
    pub physical_name: String,
    pub dataset_type: DatasetType,
    pub schema: Vec<Column>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub classification: Option<String>,
    pub location: Option<String>,
    pub row_count: Option<i64>,
    pub last_crawled_at: Option<DateTime<Utc>>,
    pub version: i32,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub name: String,
    pub column_type: String,
    pub description: Option<String>,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub ordinal_position: i32,
    pub classification: Option<String>,
    pub tags: Vec<String>,
    pub glossary_term_id: Option<Uuid>,
    pub metadata: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Enrichment tags for classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Classification {
    Public,
    Internal,
    Confidential,
    Restricted,
    Custom(String),
}

impl std::fmt::Display for Classification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Classification::Public => write!(f, "public"),
            Classification::Internal => write!(f, "internal"),
            Classification::Confidential => write!(f, "confidential"),
            Classification::Restricted => write!(f, "restricted"),
            Classification::Custom(s) => write!(f, "{s}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Lineage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub column_id: Option<Uuid>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub transformation_type: String,
    pub transformation_subtype: String,
    pub transformation_sql: Option<String>,
    pub openlineage_run_id: Option<String>,
    pub job_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnLineageInfo {
    pub source_dataset: String,
    pub source_column: String,
    pub target_dataset: String,
    pub target_column: String,
    pub transformation_type: String,
    pub transformation_subtype: String,
    pub transformation_sql: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
}

// ---------------------------------------------------------------------------
// OpenLineage integration types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLineageEvent {
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "eventTime")]
    pub event_time: String,
    pub producer: String,
    #[serde(rename = "schemaURL")]
    pub schema_url: String,
    pub job: OpenLineageJob,
    pub run: OpenLineageRun,
    pub inputs: Vec<OpenLineageDatasetRef>,
    pub outputs: Vec<OpenLineageDatasetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLineageJob {
    pub namespace: String,
    pub name: String,
    pub facets: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLineageRun {
    #[serde(rename = "runId")]
    pub run_id: String,
    pub facets: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLineageDatasetRef {
    pub namespace: String,
    pub name: String,
    pub facets: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnLineageFacet {
    pub schema: Option<String>,
    pub fields: HashMap<String, ColumnLineageField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnLineageField {
    pub input_fields: Vec<ColumnLineageInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnLineageInput {
    pub namespace: String,
    pub name: String,
    pub field: String,
    pub transformations: Vec<ColumnLineageTransformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnLineageTransformation {
    #[serde(rename = "type")]
    pub trans_type: String,
    pub subtype: String,
    pub description: Option<String>,
    pub masking: Option<bool>,
}

// ---------------------------------------------------------------------------
// Glossary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TermStatus {
    Draft,
    Published,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
    pub domain: Option<String>,
    pub synonyms: Vec<String>,
    pub related_term_ids: Vec<Uuid>,
    pub custom_properties: HashMap<String, String>,
    pub status: TermStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermMapping {
    pub id: Uuid,
    pub term_id: Uuid,
    pub dataset_id: Uuid,
    pub column_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyType {
    Masking,
    RowFilter,
    Access,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaskMethod {
    Redact,
    Hash,
    Nullify,
    Partial(usize),
    Tokenize,
    Sha256 { salt: Option<String> },
    Mask { character: char, count: usize },
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    Mask(MaskMethod),
    Filter(String),
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub dataset_pattern: String,
    pub column_pattern: Option<String>,
    pub condition: Option<String>,
    pub action: PolicyAction,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub policy_type: PolicyType,
    pub rules: Vec<PolicyRule>,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvalRequest {
    pub query: String,
    pub dataset: Option<String>,
    pub columns: Vec<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvalResult {
    pub matched_policies: Vec<Policy>,
    pub transformations: Vec<ColumnTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnTransform {
    pub dataset: String,
    pub column: String,
    pub action: PolicyAction,
}

// ---------------------------------------------------------------------------
// Crawl
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlRun {
    pub id: Uuid,
    pub data_source_id: Uuid,
    pub status: CrawlStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub datasets_found: i32,
    pub events_processed: i32,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub dataset_id: Option<Uuid>,
    pub column_id: Option<Uuid>,
    pub glossary_term_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub score: f64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub total: usize,
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

// ---------------------------------------------------------------------------
// Custom Metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataEntry {
    pub dataset_id: Uuid,
    pub column_id: Option<Uuid>,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMetadataRequest {
    pub key: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// Audit Log
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub user: Option<String>,
    pub api_key: Option<String>,
}

// ---------------------------------------------------------------------------
// Schema Version
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub dataset_id: Uuid,
    pub version: i32,
    pub schema: Vec<Column>,
    pub diff_from_previous: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub datasource_count: usize,
    pub dataset_count: usize,
    pub column_count: usize,
    pub glossary_term_count: usize,
    pub policy_count: usize,
    pub lineage_edge_count: usize,
    pub crawl_run_count: usize,
    pub total_api_calls: u64,
    pub uptime_seconds: u64,
}
