use std::collections::HashMap;
use std::sync::Arc;

use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::*;
use opencatalog_server::AppState;
use opencatalog_store::MemoryCatalogStore;

/// Helper to create a fresh server state for testing.
fn test_state() -> Arc<AppState> {
    Arc::new(AppState {
        store: MemoryCatalogStore::new(),
        crawler_registry: opencatalog_crawler::CrawlerRegistry::new(),
        llm: None,
        api_keys: vec![],
    })
}

// =========================================================================
// Original tests (preserved)
// =========================================================================

#[tokio::test]
async fn test_datasource_crud() {
    let state = test_state();

    let ds = DataSource {
        id: uuid::Uuid::nil(),
        name: "test_openlake".into(),
        source_type: SourceType::OpenLake,
        description: Some("Test".into()),
        connection_config: [("url".into(), "http://localhost:9090".into())].into(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = state.store.create_datasource(ds).await.unwrap();
    assert_ne!(created.id, uuid::Uuid::nil());

    let fetched = state.store.get_datasource(created.id).await.unwrap();
    assert_eq!(fetched.name, "test_openlake");

    let all = state.store.list_datasources().await.unwrap();
    assert_eq!(all.data.len(), 1);

    state.store.delete_datasource(created.id).await.unwrap();
    assert!(state.store.get_datasource(created.id).await.is_err());
}

#[tokio::test]
async fn test_dataset_search() {
    let state = test_state();

    let ds = Dataset {
        id: uuid::Uuid::nil(),
        data_source_id: uuid::Uuid::nil(),
        name: "analytics.customers".into(),
        physical_name: "analytics.customers".into(),
        dataset_type: DatasetType::Table,
        schema: vec![Column {
            id: uuid::Uuid::nil(),
            dataset_id: uuid::Uuid::nil(),
            name: "email".into(),
            column_type: "string".into(),
            description: Some("Customer email address".into()),
            is_nullable: true,
            is_primary_key: false,
            is_foreign_key: false,
            ordinal_position: 0,
            classification: None,
            tags: vec![],
            glossary_term_id: None,
            metadata: HashMap::new(),
        }],
        description: Some("Customer data table".into()),
        tags: vec!["pii".into()],
        classification: None,
        location: None,
        row_count: None,
        last_crawled_at: None,
        version: 1,
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    state.store.create_dataset(ds).await.unwrap();

    let pagination = PaginationParams { offset: 0, limit: 50 };
    let all = state.store.list_datasets(None, &pagination).await.unwrap();
    assert_eq!(all.data.len(), 1, "Dataset should be stored");

    let results = state.store.search("customer", 10).await.unwrap();
    assert!(!results.results.is_empty());
    assert!(results.results.iter().any(|r| r.name.contains("customers")));

    let results = state.store.search("email", 10).await.unwrap();
    assert!(!results.results.is_empty());
}

#[tokio::test]
async fn test_glossary_term_crud() {
    let state = test_state();

    let term = GlossaryTerm {
        id: uuid::Uuid::nil(),
        name: "Customer Email".into(),
        description: "Email address of a customer".into(),
        short_description: None,
        domain: Some("Customer Data".into()),
        synonyms: vec!["email".into(), "e-mail".into()],
        related_term_ids: vec![],
        custom_properties: HashMap::new(),
        status: TermStatus::Draft,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = state.store.create_glossary_term(term).await.unwrap();
    assert_ne!(created.id, uuid::Uuid::nil());

    let fetched = state.store.get_glossary_term(created.id).await.unwrap();
    assert_eq!(fetched.name, "Customer Email");

    let pagination = PaginationParams { offset: 0, limit: 50 };
    let all = state.store.list_glossary_terms(&pagination).await.unwrap();
    assert_eq!(all.data.len(), 1);
}

#[tokio::test]
async fn test_policy_engine() {
    let state = test_state();

    let policy = Policy {
        id: uuid::Uuid::nil(),
        name: "Mask PII".into(),
        description: None,
        policy_type: PolicyType::Masking,
        rules: vec![PolicyRule {
            dataset_pattern: "*.customers".into(),
            column_pattern: Some("email".into()),
            condition: None,
            action: PolicyAction::Mask(MaskMethod::Redact),
            roles: vec!["analyst".into()],
        }],
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    state.store.create_policy(policy).await.unwrap();

    let request = PolicyEvalRequest {
        query: "SELECT email FROM customers".into(),
        dataset: Some("analytics.customers".into()),
        columns: vec!["email".into(), "name".into()],
        role: "analyst".into(),
    };

    let pagination = PaginationParams { offset: 0, limit: 50 };
    let result = opencatalog_policy::PolicyEvaluator::evaluate(
        &state.store.list_policies(&pagination).await.unwrap().data,
        &request,
    )
    .unwrap();

    assert_eq!(result.transformations.len(), 1);
    assert_eq!(result.transformations[0].column, "email");
}

#[tokio::test]
async fn test_lineage() {
    let state = test_state();

    let ds1_id = uuid::Uuid::now_v7();
    let ds2_id = uuid::Uuid::now_v7();

    let node1 = LineageNode {
        id: uuid::Uuid::nil(),
        dataset_id: ds1_id,
        column_id: None,
        label: "source.customers".into(),
    };
    let node1 = state.store.add_lineage_node(node1).await.unwrap();

    let node2 = LineageNode {
        id: uuid::Uuid::nil(),
        dataset_id: ds2_id,
        column_id: None,
        label: "analytics.masked_customers".into(),
    };
    let node2 = state.store.add_lineage_node(node2).await.unwrap();

    let edge = LineageEdge {
        id: uuid::Uuid::nil(),
        source_node_id: node1.id,
        target_node_id: node2.id,
        transformation_type: "DIRECT".into(),
        transformation_subtype: "TRANSFORMATION".into(),
        transformation_sql: Some("SELECT * FROM source.customers".into()),
        openlineage_run_id: None,
        job_name: Some("mask_pii_job".into()),
        created_at: chrono::Utc::now(),
    };
    state.store.add_lineage_edge(edge).await.unwrap();

    let graph = state
        .store
        .get_lineage(node2.dataset_id, opencatalog_core::traits::LineageDirection::Upstream)
        .await
        .unwrap();

    assert!(!graph.nodes.is_empty());
    assert!(!graph.edges.is_empty());
}

#[tokio::test]
async fn test_policy_mask_application() {
    let email = "user@example.com";
    assert_eq!(
        opencatalog_policy::PolicyEvaluator::apply_mask(email, &MaskMethod::Redact),
        "***REDACTED***"
    );
    assert_eq!(
        opencatalog_policy::PolicyEvaluator::apply_mask(email, &MaskMethod::Partial(4)),
        "user************"
    );
    assert!(opencatalog_policy::PolicyEvaluator::apply_mask(email, &MaskMethod::Hash).len() == 64);
}

#[tokio::test]
async fn test_sqlite_store() {
    let store = opencatalog_store::SqliteCatalogStore::in_memory().unwrap();

    let ds = DataSource {
        id: uuid::Uuid::nil(),
        name: "sqlite_test".into(),
        source_type: SourceType::OpenLake,
        description: None,
        connection_config: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = store.create_datasource(ds).await.unwrap();
    assert_ne!(created.id, uuid::Uuid::nil());

    let fetched = store.get_datasource(created.id).await.unwrap();
    assert_eq!(fetched.name, "sqlite_test");

    let all = store.list_datasources().await.unwrap();
    assert_eq!(all.data.len(), 1);

    let term = GlossaryTerm {
        id: uuid::Uuid::nil(),
        name: "Test Term".into(),
        description: "A test glossary term".into(),
        short_description: None,
        domain: Some("Test".into()),
        synonyms: vec![],
        related_term_ids: vec![],
        custom_properties: HashMap::new(),
        status: TermStatus::Published,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created_term = store.create_glossary_term(term).await.unwrap();
    assert_ne!(created_term.id, uuid::Uuid::nil());

    let pagination = PaginationParams { offset: 0, limit: 50 };
    let terms = store.list_glossary_terms(&pagination).await.unwrap();
    assert_eq!(terms.data.len(), 1);
}

#[tokio::test]
async fn test_openlineage_parsing() {
    let event = OpenLineageEvent {
        event_type: "COMPLETE".into(),
        event_time: "2025-01-01T00:00:00Z".into(),
        producer: "test".into(),
        schema_url: "https://openlineage.io/spec/1-1-0/OpenLineage.json".into(),
        job: OpenLineageJob {
            namespace: "openpipe".into(),
            name: "model.orders".into(),
            facets: None,
        },
        run: OpenLineageRun {
            run_id: "run-1".into(),
            facets: None,
        },
        inputs: vec![OpenLineageDatasetRef {
            namespace: "source_db".into(),
            name: "public.orders".into(),
            facets: None,
        }],
        outputs: vec![OpenLineageDatasetRef {
            namespace: "lakehouse".into(),
            name: "analytics.orders".into(),
            facets: Some(
                [(
                    "columnLineage".into(),
                    serde_json::json!({
                        "fields": {
                            "order_id": {
                                "inputFields": [{
                                    "namespace": "source_db",
                                    "name": "public.orders",
                                    "field": "id",
                                    "transformations": [{"type": "DIRECT", "subtype": "IDENTITY"}]
                                }]
                            }
                        }
                    }),
                )]
                .into(),
            ),
        }],
    };

    let lineage = opencatalog_lineage::extract_column_lineage(&event);
    assert_eq!(lineage.len(), 1);
    assert_eq!(lineage[0].target_column, "order_id");
    assert_eq!(lineage[0].source_column, "id");
}

// =========================================================================
// New comprehensive E2E tests
// =========================================================================

#[tokio::test]
async fn test_metadata_crud() {
    let state = test_state();

    let created = state
        .store
        .create_dataset(Dataset {
            id: uuid::Uuid::nil(),
            data_source_id: uuid::Uuid::nil(),
            name: "meta_test".into(),
            physical_name: "meta_test".into(),
            dataset_type: DatasetType::Table,
            schema: vec![Column {
                id: uuid::Uuid::nil(),
                dataset_id: uuid::Uuid::nil(),
                name: "col1".into(),
                column_type: "string".into(),
                description: None,
                is_nullable: false,
                is_primary_key: false,
                is_foreign_key: false,
                ordinal_position: 1,
                classification: None,
                tags: vec![],
                glossary_term_id: None,
                metadata: HashMap::new(),
            }],
            description: None,
            tags: vec![],
            classification: None,
            location: None,
            row_count: None,
            last_crawled_at: None,
            version: 1,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // set metadata on the dataset
    state
        .store
        .set_metadata(MetadataEntry {
            dataset_id: created.id,
            column_id: None,
            key: "owner".into(),
            value: "data-team".into(),
        })
        .await
        .unwrap();
    state
        .store
        .set_metadata(MetadataEntry {
            dataset_id: created.id,
            column_id: None,
            key: "sla".into(),
            value: "99.9".into(),
        })
        .await
        .unwrap();

    let ds_meta = state.store.get_metadata(created.id, None).await.unwrap();
    assert_eq!(ds_meta.len(), 2);
    assert_eq!(ds_meta.get("owner").unwrap(), "data-team");
    assert_eq!(ds_meta.get("sla").unwrap(), "99.9");

    // set metadata on a column
    let col = &created.schema[0];
    state
        .store
        .set_metadata(MetadataEntry {
            dataset_id: created.id,
            column_id: Some(col.id),
            key: "pii".into(),
            value: "true".into(),
        })
        .await
        .unwrap();

    let col_meta = state
        .store
        .get_metadata(created.id, Some(col.id))
        .await
        .unwrap();
    assert_eq!(col_meta.len(), 1);
    assert_eq!(col_meta.get("pii").unwrap(), "true");

    // delete a dataset-level metadata key
    state
        .store
        .delete_metadata(created.id, "sla", None)
        .await
        .unwrap();
    let ds_meta = state.store.get_metadata(created.id, None).await.unwrap();
    assert_eq!(ds_meta.len(), 1);
    assert!(ds_meta.get("sla").is_none());

    // column metadata should be untouched
    let col_meta = state
        .store
        .get_metadata(created.id, Some(col.id))
        .await
        .unwrap();
    assert_eq!(col_meta.len(), 1);
}

#[tokio::test]
async fn test_audit_log() {
    let state = test_state();

    let entries: Vec<AuditEntry> = (0..5)
        .map(|i| AuditEntry {
            id: uuid::Uuid::now_v7(),
            timestamp: chrono::Utc::now(),
            method: "GET".into(),
            path: format!("/api/v1/datasets/{}", i),
            status_code: 200,
            duration_ms: i as u64 * 10,
            user: Some(format!("user{}", i)),
            api_key: None,
        })
        .collect();

    for entry in &entries {
        state.store.append_audit_entry(entry.clone()).await.unwrap();
    }

    // list with default pagination
    let pagination = PaginationParams { offset: 0, limit: 50 };
    let page = state.store.list_audit_entries(&pagination).await.unwrap();
    assert_eq!(page.data.len(), 5);
    assert_eq!(page.total, 5);

    // paginate with small limit
    let pagination = PaginationParams { offset: 0, limit: 2 };
    let page = state.store.list_audit_entries(&pagination).await.unwrap();
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.total, 5);
    assert_eq!(page.offset, 0);
    assert_eq!(page.limit, 2);

    // second page
    let pagination = PaginationParams { offset: 2, limit: 2 };
    let page = state.store.list_audit_entries(&pagination).await.unwrap();
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.total, 5);

    // last page (single item)
    let pagination = PaginationParams { offset: 4, limit: 2 };
    let page = state.store.list_audit_entries(&pagination).await.unwrap();
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.total, 5);

    // entries are in append order
    assert_eq!(page.data[0].path, "/api/v1/datasets/4");
}

#[tokio::test]
async fn test_schema_versions() {
    let state = test_state();

    let created = state
        .store
        .create_dataset(Dataset {
            id: uuid::Uuid::nil(),
            data_source_id: uuid::Uuid::nil(),
            name: "schema_version_test".into(),
            physical_name: "schema_version_test".into(),
            dataset_type: DatasetType::Table,
            schema: vec![
                Column {
                    id: uuid::Uuid::nil(),
                    dataset_id: uuid::Uuid::nil(),
                    name: "id".into(),
                    column_type: "int".into(),
                    description: None,
                    is_nullable: false,
                    is_primary_key: true,
                    is_foreign_key: false,
                    ordinal_position: 1,
                    classification: None,
                    tags: vec![],
                    glossary_term_id: None,
                    metadata: HashMap::new(),
                },
                Column {
                    id: uuid::Uuid::nil(),
                    dataset_id: uuid::Uuid::nil(),
                    name: "name".into(),
                    column_type: "string".into(),
                    description: None,
                    is_nullable: true,
                    is_primary_key: false,
                    is_foreign_key: false,
                    ordinal_position: 2,
                    classification: None,
                    tags: vec![],
                    glossary_term_id: None,
                    metadata: HashMap::new(),
                },
            ],
            description: None,
            tags: vec![],
            classification: None,
            location: None,
            row_count: None,
            last_crawled_at: None,
            version: 1,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    assert_eq!(created.version, 1);

    // update dataset — adds column "email", drops "name"
    let mut updated = created.clone();
    updated.schema = vec![
        Column {
            id: uuid::Uuid::nil(),
            dataset_id: created.id,
            name: "id".into(),
            column_type: "int".into(),
            description: None,
            is_nullable: false,
            is_primary_key: true,
            is_foreign_key: false,
            ordinal_position: 1,
            classification: None,
            tags: vec![],
            glossary_term_id: None,
            metadata: HashMap::new(),
        },
        Column {
            id: uuid::Uuid::nil(),
            dataset_id: created.id,
            name: "email".into(),
            column_type: "string".into(),
            description: Some("Email address".into()),
            is_nullable: true,
            is_primary_key: false,
            is_foreign_key: false,
            ordinal_position: 2,
            classification: None,
            tags: vec![],
            glossary_term_id: None,
            metadata: HashMap::new(),
        },
    ];
    let updated = state.store.update_dataset(updated).await.unwrap();
    assert_eq!(updated.version, 2);

    // get schema versions
    let versions = state
        .store
        .get_schema_versions(created.id)
        .await
        .unwrap();
    assert_eq!(versions.len(), 2);

    let v1 = versions.iter().find(|v| v.version == 1).unwrap();
    assert_eq!(v1.schema.len(), 2);

    let v2 = versions.iter().find(|v| v.version == 2).unwrap();
    assert_eq!(v2.schema.len(), 2);

    // diff from v1 to v2
    let diff = state
        .store
        .diff_schema(created.id, 1, 2)
        .await
        .unwrap();
    assert!(diff.contains("+ email string (added)"));
    assert!(diff.contains("- name string (removed)"));

    // diff from v2 to v1 (reverse)
    let diff_rev = state
        .store
        .diff_schema(created.id, 2, 1)
        .await
        .unwrap();
    assert!(diff_rev.contains("+ name string (added)"));
    assert!(diff_rev.contains("- email string (removed)"));
}

#[tokio::test]
async fn test_metrics() {
    let state = test_state();

    // no data — all zeros (except uptime)
    let metrics = state.store.get_metrics().await.unwrap();
    assert_eq!(metrics.datasource_count, 0);
    assert_eq!(metrics.dataset_count, 0);
    assert_eq!(metrics.column_count, 0);
    assert_eq!(metrics.glossary_term_count, 0);
    assert_eq!(metrics.policy_count, 0);
    assert_eq!(metrics.lineage_edge_count, 0);
    assert_eq!(metrics.crawl_run_count, 0);
    assert_eq!(metrics.total_api_calls, 0);
    let _ = metrics.uptime_seconds; // u64, always non-negative

    // add one datasource with one dataset
    let ds = state
        .store
        .create_datasource(DataSource {
            id: uuid::Uuid::nil(),
            name: "metrics_test_source".into(),
            source_type: SourceType::OpenLake,
            description: None,
            connection_config: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    state
        .store
        .create_dataset(Dataset {
            id: uuid::Uuid::nil(),
            data_source_id: ds.id,
            name: "metrics_test".into(),
            physical_name: "metrics_test".into(),
            dataset_type: DatasetType::Table,
            schema: vec![
                Column {
                    id: uuid::Uuid::nil(),
                    dataset_id: uuid::Uuid::nil(),
                    name: "a".into(),
                    column_type: "int".into(),
                    description: None,
                    is_nullable: false,
                    is_primary_key: false,
                    is_foreign_key: false,
                    ordinal_position: 1,
                    classification: None,
                    tags: vec![],
                    glossary_term_id: None,
                    metadata: HashMap::new(),
                },
                Column {
                    id: uuid::Uuid::nil(),
                    dataset_id: uuid::Uuid::nil(),
                    name: "b".into(),
                    column_type: "string".into(),
                    description: None,
                    is_nullable: true,
                    is_primary_key: false,
                    is_foreign_key: false,
                    ordinal_position: 2,
                    classification: None,
                    tags: vec![],
                    glossary_term_id: None,
                    metadata: HashMap::new(),
                },
            ],
            description: None,
            tags: vec![],
            classification: None,
            location: None,
            row_count: None,
            last_crawled_at: None,
            version: 1,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // add a glossary term
    state
        .store
        .create_glossary_term(GlossaryTerm {
            id: uuid::Uuid::nil(),
            name: "Metric Term".into(),
            description: "For metrics test".into(),
            short_description: None,
            domain: None,
            synonyms: vec![],
            related_term_ids: vec![],
            custom_properties: HashMap::new(),
            status: TermStatus::Published,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // add audit entries to bump total_api_calls
    state
        .store
        .append_audit_entry(AuditEntry {
            id: uuid::Uuid::now_v7(),
            timestamp: chrono::Utc::now(),
            method: "GET".into(),
            path: "/api/v1/datasources".into(),
            status_code: 200,
            duration_ms: 5,
            user: None,
            api_key: None,
        })
        .await
        .unwrap();

    let metrics = state.store.get_metrics().await.unwrap();
    assert_eq!(metrics.datasource_count, 1);
    assert_eq!(metrics.dataset_count, 1);
    assert_eq!(metrics.column_count, 2);
    assert_eq!(metrics.glossary_term_count, 1);
    assert_eq!(metrics.policy_count, 0);
    assert_eq!(metrics.lineage_edge_count, 0);
    assert_eq!(metrics.crawl_run_count, 0);
    assert_eq!(metrics.total_api_calls, 1);
}

#[tokio::test]
async fn test_pagination() {
    let state = test_state();

    let ds = state
        .store
        .create_datasource(DataSource {
            id: uuid::Uuid::nil(),
            name: "pagination_source".into(),
            source_type: SourceType::OpenLake,
            description: None,
            connection_config: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // create 25 datasets under the same datasource
    for i in 0..25 {
        state
            .store
            .create_dataset(Dataset {
                id: uuid::Uuid::nil(),
                data_source_id: ds.id,
                name: format!("pagination_dataset_{}", i),
                physical_name: format!("pagination_dataset_{}", i),
                dataset_type: DatasetType::Table,
                schema: vec![],
                description: None,
                tags: vec![],
                classification: None,
                location: None,
                row_count: None,
                last_crawled_at: None,
                version: 1,
                metadata: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
    }

    // also create 25 glossary terms
    for i in 0..25 {
        state
            .store
            .create_glossary_term(GlossaryTerm {
                id: uuid::Uuid::nil(),
                name: format!("pagination_term_{}", i),
                description: format!("Term number {}", i),
                short_description: None,
                domain: None,
                synonyms: vec![],
                related_term_ids: vec![],
                custom_properties: HashMap::new(),
                status: TermStatus::Draft,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
    }

    // -- test list_datasets pagination --
    let page1 = state
        .store
        .list_datasets(Some(ds.id), &PaginationParams { offset: 0, limit: 10 })
        .await
        .unwrap();
    assert_eq!(page1.data.len(), 10);
    assert_eq!(page1.total, 25);
    assert_eq!(page1.offset, 0);
    assert_eq!(page1.limit, 10);

    let page2 = state
        .store
        .list_datasets(Some(ds.id), &PaginationParams { offset: 10, limit: 10 })
        .await
        .unwrap();
    assert_eq!(page2.data.len(), 10);
    assert_eq!(page2.total, 25);

    let page3 = state
        .store
        .list_datasets(Some(ds.id), &PaginationParams { offset: 20, limit: 10 })
        .await
        .unwrap();
    assert_eq!(page3.data.len(), 5);
    assert_eq!(page3.total, 25);

    // ensure pages are distinct
    let all_names: std::collections::HashSet<&str> = page1
        .data
        .iter()
        .chain(&page2.data)
        .chain(&page3.data)
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(all_names.len(), 25);

    // -- test list_glossary_terms pagination --
    let terms_page = state
        .store
        .list_glossary_terms(&PaginationParams { offset: 0, limit: 5 })
        .await
        .unwrap();
    assert_eq!(terms_page.data.len(), 5);
    assert_eq!(terms_page.total, 25);

    // -- test search_datasets pagination --
    let search_page = state
        .store
        .search_datasets("pagination", &PaginationParams { offset: 0, limit: 5 })
        .await
        .unwrap();
    assert_eq!(search_page.data.len(), 5);
    assert_eq!(search_page.total, 25);
}

#[tokio::test]
async fn test_crawl_run_persistence() {
    let state = test_state();

    let ds = state
        .store
        .create_datasource(DataSource {
            id: uuid::Uuid::nil(),
            name: "crawl_source".into(),
            source_type: SourceType::OpenLake,
            description: None,
            connection_config: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // create several crawl runs
    for i in 0..3 {
        let run = CrawlRun {
            id: uuid::Uuid::nil(),
            data_source_id: ds.id,
            status: if i == 2 { CrawlStatus::Failed } else { CrawlStatus::Completed },
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            datasets_found: (i + 1) as i32 * 10,
            events_processed: (i + 1) as i32 * 100,
            error: if i == 2 { Some("timeout".into()) } else { None },
        };
        state.store.create_crawl_run(run).await.unwrap();
    }

    // list crawl runs for this datasource
    let pagination = PaginationParams { offset: 0, limit: 10 };
    let runs = state
        .store
        .list_crawl_runs(ds.id, &pagination)
        .await
        .unwrap();
    assert_eq!(runs.data.len(), 3);
    assert_eq!(runs.total, 3);

    // verify data
    assert_eq!(runs.data[0].datasets_found, 10);
    assert_eq!(runs.data[1].datasets_found, 20);
    assert_eq!(runs.data[2].datasets_found, 30);
    assert_eq!(runs.data[2].status, CrawlStatus::Failed);
    assert_eq!(runs.data[2].error.as_deref(), Some("timeout"));

    // update the failed run to completed
    let mut updated = runs.data[2].clone();
    updated.status = CrawlStatus::Completed;
    updated.error = None;
    state.store.update_crawl_run(updated).await.unwrap();

    let runs = state
        .store
        .list_crawl_runs(ds.id, &pagination)
        .await
        .unwrap();
    assert_eq!(runs.data[2].status, CrawlStatus::Completed);
    assert!(runs.data[2].error.is_none());

    // no crawl runs for a different datasource
    let other_id = uuid::Uuid::now_v7();
    let empty_runs = state
        .store
        .list_crawl_runs(other_id, &pagination)
        .await
        .unwrap();
    assert!(empty_runs.data.is_empty());
    assert_eq!(empty_runs.total, 0);
}

#[tokio::test]
async fn test_search_across_entities() {
    let state = test_state();

    // create a datasource
    let ds = state
        .store
        .create_datasource(DataSource {
            id: uuid::Uuid::nil(),
            name: "search_source".into(),
            source_type: SourceType::OpenLake,
            description: None,
            connection_config: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // create a dataset with columns that will match different queries
    state
        .store
        .create_dataset(Dataset {
            id: uuid::Uuid::nil(),
            data_source_id: ds.id,
            name: "sales_orders".into(),
            physical_name: "sales_orders".into(),
            dataset_type: DatasetType::Table,
            schema: vec![
                Column {
                    id: uuid::Uuid::nil(),
                    dataset_id: uuid::Uuid::nil(),
                    name: "order_id".into(),
                    column_type: "bigint".into(),
                    description: Some("Unique order identifier".into()),
                    is_nullable: false,
                    is_primary_key: true,
                    is_foreign_key: false,
                    ordinal_position: 1,
                    classification: None,
                    tags: vec!["revenue".into()],
                    glossary_term_id: None,
                    metadata: HashMap::new(),
                },
                Column {
                    id: uuid::Uuid::nil(),
                    dataset_id: uuid::Uuid::nil(),
                    name: "customer_email".into(),
                    column_type: "string".into(),
                    description: Some("Customer email for order".into()),
                    is_nullable: true,
                    is_primary_key: false,
                    is_foreign_key: false,
                    ordinal_position: 2,
                    classification: None,
                    tags: vec!["pii".into()],
                    glossary_term_id: None,
                    metadata: HashMap::new(),
                },
            ],
            description: Some("Contains all sales order records".into()),
            tags: vec!["finance".into(), "ecommerce".into()],
            classification: None,
            location: None,
            row_count: None,
            last_crawled_at: None,
            version: 1,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // create a second dataset with a different theme
    state
        .store
        .create_dataset(Dataset {
            id: uuid::Uuid::nil(),
            data_source_id: ds.id,
            name: "product_catalog".into(),
            physical_name: "product_catalog".into(),
            dataset_type: DatasetType::Table,
            schema: vec![Column {
                id: uuid::Uuid::nil(),
                dataset_id: uuid::Uuid::nil(),
                name: "product_name".into(),
                column_type: "string".into(),
                description: Some("Name of the product".into()),
                is_nullable: false,
                is_primary_key: false,
                is_foreign_key: false,
                ordinal_position: 1,
                classification: None,
                tags: vec![],
                glossary_term_id: None,
                metadata: HashMap::new(),
            }],
            description: Some("Product catalog information".into()),
            tags: vec!["inventory".into()],
            classification: None,
            location: None,
            row_count: None,
            last_crawled_at: None,
            version: 1,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // create glossary terms
    state
        .store
        .create_glossary_term(GlossaryTerm {
            id: uuid::Uuid::nil(),
            name: "Order".into(),
            description: "A customer purchase order".into(),
            short_description: None,
            domain: Some("Ecommerce".into()),
            synonyms: vec!["purchase".into(), "transaction".into()],
            related_term_ids: vec![],
            custom_properties: HashMap::new(),
            status: TermStatus::Published,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    state
        .store
        .create_glossary_term(GlossaryTerm {
            id: uuid::Uuid::nil(),
            name: "Product".into(),
            description: "An item available for sale".into(),
            short_description: None,
            domain: Some("Ecommerce".into()),
            synonyms: vec!["item".into(), "sku".into()],
            related_term_ids: vec![],
            custom_properties: HashMap::new(),
            status: TermStatus::Published,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // -- search for "sales" should match dataset name --
    let results = state.store.search("sales", 10).await.unwrap();
    assert!(!results.results.is_empty());
    assert!(results.results.iter().any(|r| r.kind == "dataset" && r.name == "sales_orders"));

    // -- search for "order" should match dataset name + glossary term --
    let results = state.store.search("order", 10).await.unwrap();
    assert!(!results.results.is_empty());
    assert!(results.results.iter().any(|r| r.kind == "dataset"), "should match dataset");
    assert!(results.results.iter().any(|r| r.kind == "glossary_term"), "should match glossary term");

    // -- search for "email" should match column name --
    let results = state.store.search("email", 10).await.unwrap();
    assert!(!results.results.is_empty());
    assert!(results.results.iter().any(|r| r.kind == "column"));

    // -- search for "customer_email" should match column name --
    let results = state.store.search("customer_email", 10).await.unwrap();
    assert!(!results.results.is_empty());
    assert!(results.results.iter().any(|r| r.kind == "column"));

    // -- search for "inventory" should match dataset tags --
    let results = state.store.search("inventory", 10).await.unwrap();
    assert!(!results.results.is_empty());
    assert!(results.results.iter().any(|r| r.kind == "dataset" && r.name == "product_catalog"));

    // -- search for "purchase" should match glossary synonym --
    let results = state.store.search("purchase", 10).await.unwrap();
    assert!(!results.results.is_empty());
    assert!(results.results.iter().any(|r| r.kind == "glossary_term"));

    // -- search for something that does not exist --
    let results = state.store.search("xyznonexistent", 10).await.unwrap();
    assert!(results.results.is_empty());
    assert_eq!(results.total, 0);

    // -- limit parameter --
    let results = state.store.search("order", 1).await.unwrap();
    assert!(results.results.len() <= 1);

    // -- search_datasets filtered by query --
    let pagination = PaginationParams { offset: 0, limit: 50 };
    let ds_search = state
        .store
        .search_datasets("sales", &pagination)
        .await
        .unwrap();
    assert_eq!(ds_search.data.len(), 1);
    assert_eq!(ds_search.data[0].name, "sales_orders");

    let ds_search = state
        .store
        .search_datasets("product", &pagination)
        .await
        .unwrap();
    assert_eq!(ds_search.data.len(), 1);
    assert_eq!(ds_search.data[0].name, "product_catalog");
}
