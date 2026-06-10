//! Codex config writer

use crate::error::{AppError, AppResult};
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
