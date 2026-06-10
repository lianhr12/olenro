//! Hermes config writer

use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// Get Hermes config path
pub fn get_hermes_config_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".hermes")
        .join("config.json")
}

/// Read Hermes config
pub fn read_hermes_config() -> AppResult<Option<serde_json::Value>> {
    let path = get_hermes_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| AppError::Json(e))?;
    Ok(Some(config))
}
