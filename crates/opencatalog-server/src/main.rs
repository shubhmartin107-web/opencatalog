use std::sync::Arc;

use axum::{
    Router,
    extract::Extension,
    middleware,
    routing::{delete, get, post},
};
use opencatalog_server::{
    AppState, config::ServerConfig, mcp, middleware::rate_limit::RateLimiter, rest,
};
use opencatalog_store::MemoryCatalogStore;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let cfg = ServerConfig::default();
    let store = MemoryCatalogStore::new();
    let crawler_registry = opencatalog_crawler::CrawlerRegistry::new();

    let llm = cfg.llm_base_url.as_ref().map(|base_url| {
        opencatalog_llm::OllamaClient::new(
            base_url.clone(),
            cfg.llm_model.clone().unwrap_or_else(|| "llama3.2".into()),
        )
    });

    let api_keys: Vec<String> = std::env::var("CATALOG_API_KEYS")
        .ok()
        .map(|s| s.split(',').map(|k| k.trim().to_string()).collect())
        .unwrap_or_default();

    if !api_keys.is_empty() {
        tracing::info!("API key authentication enabled ({} key(s))", api_keys.len());
    }

    let rate_limit_disabled = std::env::var("RATE_LIMIT_DISABLE")
        .ok()
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    let rate_limiter = if rate_limit_disabled {
        None
    } else {
        Some(Arc::new(RateLimiter::new(cfg.rate_limit_per_minute)))
    };

    let state = Arc::new(AppState {
        store,
        crawler_registry,
        llm,
        api_keys,
    });

    let mut app = Router::new()
        .route("/api/v1/health", get(rest::health::health_check))
        .route(
            "/api/v1/datasources",
            get(rest::datasources::list_datasources).post(rest::datasources::create_datasource),
        )
        .route(
            "/api/v1/datasources/{id}",
            get(rest::datasources::get_datasource).delete(rest::datasources::delete_datasource),
        )
        .route("/api/v1/datasets", get(rest::datasets::list_datasets))
        .route(
            "/api/v1/datasets/search",
            get(rest::datasets::search_datasets),
        )
        .route("/api/v1/datasets/{id}", get(rest::datasets::get_dataset))
        .route(
            "/api/v1/datasets/{id}/lineage",
            get(rest::datasets::get_dataset_lineage),
        )
        .route(
            "/api/v1/datasets/{id}/impact",
            get(rest::datasets::get_dataset_impact),
        )
        .route(
            "/api/v1/datasets/{id}/metadata",
            get(rest::metadata::get_dataset_metadata).put(rest::metadata::set_dataset_metadata),
        )
        .route(
            "/api/v1/datasets/{id}/metadata/{key}",
            delete(rest::metadata::delete_dataset_metadata),
        )
        .route(
            "/api/v1/policies",
            get(rest::policies::list_policies).post(rest::policies::create_policy),
        )
        .route(
            "/api/v1/glossary",
            get(rest::glossary::list_glossary).post(rest::glossary::create_glossary_term),
        )
        .route(
            "/api/v1/datasources/{id}/crawl",
            post(rest::crawls::trigger_crawl),
        )
        .route("/api/v1/search", get(rest::search::search))
        .route(
            "/api/v1/lineage",
            post(rest::crawls::ingest_openlineage_event),
        )
        .route("/api/v1/audit", get(rest::audit::list_audit_entries))
        .route("/api/v1/metrics", get(rest::metrics::get_metrics))
        .route("/mcp", post(mcp::handler::handle_mcp))
        .layer(middleware::from_fn(
            opencatalog_server::middleware::auth::auth_middleware,
        ))
        .layer(Extension(state))
        .layer(CorsLayer::permissive());

    if let Some(limiter) = rate_limiter {
        app = app
            .layer(middleware::from_fn(
                opencatalog_server::middleware::rate_limit::rate_limit_middleware,
            ))
            .layer(Extension(limiter));
    }

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("OpenCatalog server starting on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
