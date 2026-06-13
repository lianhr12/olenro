//! OpenCode config writer
//!
//! OpenCode uses `~/.opencode/config.json` with a `provider` field
//! containing the active provider configuration.

use crate::error::{AppError, AppResult};
use serde_json::Value;
use std::path::PathBuf;

/// Get OpenCode config path
pub fn get_opencode_config_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".opencode")
        .join("config.json")
}

/// Read OpenCode config
pub fn read_opencode_config() -> AppResult<Option<serde_json::Value>> {
    let path = get_opencode_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| AppError::Json(e))?;
    Ok(Some(config))
}

/// Write OpenCode config
pub fn write_opencode_config(config: &Value) -> AppResult<()> {
    let path = get_opencode_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    crate::config::write_json_file(&path, config)?;
    log::debug!("Wrote OpenCode config.json");
    Ok(())
}
