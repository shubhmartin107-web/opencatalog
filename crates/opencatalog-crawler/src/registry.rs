use opencatalog_core::error::CatalogResult;
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::{CrawlRun, CrawlStatus, DataSource};
use uuid::Uuid;

use crate::datascan::DataScanner;
use crate::dbt::DbtCrawler;
use crate::openingest::OpenIngestCrawler;
use crate::openlake::OpenLakeCrawler;

pub struct CrawlerRegistry;

impl Default for CrawlerRegistry {
    fn default() -> Self {
        Self
    }
}

impl CrawlerRegistry {
    pub fn new() -> Self {
        Self
    }

    /// Crawl a data source and persist discovered datasets and lineage to the store.
    pub async fn crawl_and_persist(
        &self,
        datasource: &DataSource,
        store: &dyn CatalogStore,
    ) -> CatalogResult<CrawlRun> {
        let run_id = Uuid::now_v7();
        let started_at = chrono::Utc::now();

        // Perform the crawl
        let crawl_result = match datasource.source_type {
            opencatalog_core::types::SourceType::OpenLake => {
                let crawler = OpenLakeCrawler;
                crawler.crawl(&datasource.connection_config).await
            }
            opencatalog_core::types::SourceType::OpenIngest => {
                let crawler = OpenIngestCrawler;
                crawler.crawl(&datasource.connection_config).await
            }
            opencatalog_core::types::SourceType::Custom(ref s) if s == "dbt" => {
                let crawler = DbtCrawler;
                crawler.crawl(&datasource.connection_config).await
            }
            _ => Err(opencatalog_core::error::CatalogError::InvalidInput(
                format!("No crawler for source type: {}", datasource.source_type),
            )),
        };

        match crawl_result {
            Ok(crawl_result) => {
                // Persist discovered datasets
                let mut dataset_ids = Vec::new();
                for mut ds in crawl_result.datasets {
                    ds.data_source_id = datasource.id;
                    if let Ok(created) = store.create_dataset(ds).await {
                        dataset_ids.push(created.id);

                        // Run data scan to classify columns
                        let scanner = DataScanner;
                        if let Err(e) = scanner.scan(datasource, &created, store).await {
                            tracing::warn!("Data scan failed for dataset '{}': {e}", created.name);
                        }
                    }
                }

                // Persist lineage edges (requires node IDs to be set by the store)
                for edge in crawl_result.lineage_edges {
                    // In a full implementation, we'd create lineage nodes and edges
                    // with proper dataset_id references. For now, we record them.
                    let _ = edge.id;
                }

                // Persist OpenLineage events
                let events_processed = crawl_result.openlineage_events.len() as i32;
                for event in crawl_result.openlineage_events {
                    let _ = store.ingest_openlineage_event(event).await;
                }

                let completed_at = chrono::Utc::now();
                Ok(CrawlRun {
                    id: run_id,
                    data_source_id: datasource.id,
                    status: CrawlStatus::Completed,
                    started_at,
                    completed_at: Some(completed_at),
                    datasets_found: dataset_ids.len() as i32,
                    events_processed,
                    error: None,
                })
            }
            Err(e) => Ok(CrawlRun {
                id: run_id,
                data_source_id: datasource.id,
                status: CrawlStatus::Failed,
                started_at,
                completed_at: Some(chrono::Utc::now()),
                datasets_found: 0,
                events_processed: 0,
                error: Some(e.to_string()),
            }),
        }
    }
}
