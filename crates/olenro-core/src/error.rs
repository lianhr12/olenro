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

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::Skill(format!("zip error: {e}"))
    }
}

/// Result type alias for Olenro operations
pub type AppResult<T> = Result<T, AppError>;

/// 构造结构化的技能错误负载（JSON 字符串）。
///
/// 由桌面端 `src-tauri/src/error.rs` 移植。返回形如
/// `{"code":"...","context":{...},"suggestion":"..."}` 的 JSON 串，
/// 便于 TUI/CLI 层做 i18n 与修复建议引导。序列化失败时回退为
/// `ERROR:<code>`。
pub fn format_skill_error(
    code: &str,
    context: &[(&str, &str)],
    suggestion: Option<&str>,
) -> String {
    use serde_json::json;

    let mut ctx_map = serde_json::Map::new();
    for (key, value) in context {
        ctx_map.insert(key.to_string(), json!(value));
    }

    let error_obj = json!({
        "code": code,
        "context": ctx_map,
        "suggestion": suggestion,
    });

    serde_json::to_string(&error_obj).unwrap_or_else(|_| format!("ERROR:{code}"))
}
