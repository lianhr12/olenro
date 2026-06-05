//! Error types for Olenro Core

use thiserror::Error;

/// Main error type for Olenro operations
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(String),

    #[error("TOML edit error: {0}")]
    TomlEdit(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Database migration error: {0}")]
    Migration(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("Skill error: {0}")]
    Skill(String),

    #[error("Prompt error: {0}")]
    Prompt(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Git sync error: {0}")]
    GitSync(String),

    #[error("Usage error: {0}")]
    Usage(String),

    #[error("Deep link error: {0}")]
    DeepLink(String),

    #[error("Auth error: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Atomic write failed: {0}")]
    AtomicWrite(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unknown app type: {0}")]
    UnknownAppType(String),
}

/// Result type alias for Olenro operations
pub type AppResult<T> = Result<T, AppError>;
