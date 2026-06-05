//! Claude Desktop config writer
//!
//! Reads and writes Claude Desktop configuration

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Claude Desktop configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeDesktopConfig {
    pub version: i32,
    pub settings: ClaudeDesktopSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeDesktopSettings {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Get Claude Desktop config path
pub fn get_claude_desktop_config_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".claude-desktop")
        .join("config.json")
}

/// Read Claude Desktop config
pub fn read_claude_desktop_config() -> AppResult<Option<ClaudeDesktopConfig>> {
    let path = get_claude_desktop_config_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)?;
    let config: ClaudeDesktopConfig = serde_json::from_str(&content)
        .map_err(|e| AppError::Config(format!("Failed to parse Claude Desktop config: {}", e)))?;

    Ok(Some(config))
}

/// Write Claude Desktop config
pub fn write_claude_desktop_config(config: &ClaudeDesktopConfig) -> AppResult<()> {
    let path = get_claude_desktop_config_path();
    crate::config::write_json_file(&path, config)
}
