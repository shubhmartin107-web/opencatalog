#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub llm_base_url: Option<String>,
    pub llm_model: Option<String>,
    pub rate_limit_per_minute: u64,
    pub prometheus_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let rate_limit_per_minute = std::env::var("RATE_LIMIT_PER_MINUTE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        let prometheus_enabled = std::env::var("PROMETHEUS_ENABLED")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            llm_base_url: Some("http://localhost:11434".into()),
            llm_model: Some("llama3.2".into()),
            rate_limit_per_minute,
            prometheus_enabled,
        }
    }
}
