//! Gemini config writer
//!
//! Gemini CLI uses a .env file for configuration (not JSON).

use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::PathBuf;

/// Get Gemini .env path
pub fn get_gemini_env_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".gemini")
        .join(".env")
}

/// Get Gemini config path (for backward compatibility)
pub fn get_gemini_config_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".gemini")
        .join("config.json")
}

/// Parse .env file content into key-value pairs.
/// Loosely parses the .env file, skipping invalid lines.
pub fn parse_env_file(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut parts = line.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => Some((key.trim().to_string(), value.trim().to_string())),
                _ => None,
            }
        })
        .collect()
}

/// Serialize key-value pairs to .env format.
pub fn serialize_env_file(map: &HashMap<String, String>) -> String {
    map.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Read Gemini .env file.
pub fn read_gemini_env() -> AppResult<HashMap<String, String>> {
    let path = get_gemini_env_path();
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(parse_env_file(&content))
}

/// Write Gemini .env file atomically.
pub fn write_gemini_env_atomic(map: &HashMap<String, String>) -> AppResult<()> {
    let path = get_gemini_env_path();

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }

    let content = serialize_env_file(map);
    crate::config::write_text_file(&path, &content)?;
    log::debug!("Wrote Gemini .env to {:?}", path);
    Ok(())
}

/// Read Gemini config (legacy JSON format, for backward compatibility)
pub fn read_gemini_config() -> AppResult<Option<serde_json::Value>> {
    let path = get_gemini_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| AppError::Json(e))?;
    Ok(Some(config))
}
