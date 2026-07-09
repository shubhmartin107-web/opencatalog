use thiserror::Error;

#[derive(Error, Debug)]
pub enum CatalogError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("crawl failed: {0}")]
    CrawlFailed(String),

    #[error("lineage error: {0}")]
    LineageError(String),

    #[error("policy error: {0}")]
    PolicyError(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("rate limit exceeded")]
    RateLimitExceeded,

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type CatalogResult<T> = Result<T, CatalogError>;
