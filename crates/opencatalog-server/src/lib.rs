pub mod config;
pub mod mcp;
pub mod middleware;
pub mod rest;

use std::sync::Arc;

use opencatalog_crawler::CrawlerRegistry;
use opencatalog_store::MemoryCatalogStore;

pub struct AppState {
    pub store: MemoryCatalogStore,
    pub crawler_registry: CrawlerRegistry,
    pub llm: Option<opencatalog_llm::OllamaClient>,
    pub api_keys: Vec<String>,
}
