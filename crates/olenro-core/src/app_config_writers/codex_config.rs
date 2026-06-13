//! Codex config writer
//!
//! Codex uses two config files:
//! - `~/.codex/auth.json` - stores authentication credentials
//! - `~/.codex/config.toml` - stores model configuration

use crate::error::{AppError, AppResult};
use serde_json::Value;
use std::path::PathBuf;

/// Get Codex config path
pub fn get_codex_config_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".codex")
        .join("config.toml")
}

/// Get Codex auth path
pub fn get_codex_auth_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".codex")
        .join("auth.json")
}

/// Read Codex config
pub fn read_codex_config() -> AppResult<Option<toml::Value>> {
    let path = get_codex_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let config: toml::Value =
        toml::from_str(&content).map_err(|e| AppError::Toml(e.to_string()))?;
    Ok(Some(config))
}

/// Read Codex auth.json
pub fn read_codex_auth() -> AppResult<Value> {
    let path = get_codex_auth_path();
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = std::fs::read_to_string(&path)?;
    let auth: Value = serde_json::from_str(&content).map_err(|e| AppError::Json(e))?;
    Ok(auth)
}

/// Write Codex auth.json
pub fn write_codex_auth(auth: &Value) -> AppResult<()> {
    let path = get_codex_auth_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    crate::config::write_json_file(&path, auth)?;
    log::debug!("Wrote Codex auth.json");
    Ok(())
}

/// Set the `experimental_bearer_token` in Codex config.toml.
/// This updates or creates the top-level `experimental_bearer_token` field.
pub fn set_codex_bearer_token(config_text: &str, token: &str) -> AppResult<String> {
    use toml_edit::DocumentMut;

    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Toml(format!("Failed to parse config.toml: {}", e)))?;

    doc["experimental_bearer_token"] = toml_edit::value(token);

    Ok(doc.to_string())
}

/// Write Codex config.toml atomically.
pub fn write_codex_config_text(content: &str) -> AppResult<()> {
    let path = get_codex_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    crate::config::write_text_file(&path, content)?;
    log::debug!("Wrote Codex config.toml");
    Ok(())
}
