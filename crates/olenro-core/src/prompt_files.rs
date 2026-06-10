//! Prompt files management
//!
//! Manages prompt files on disk (CLAUDE.md, AGENTS.md, GEMINI.md)

use crate::app_config::AppType;
use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// Get the prompt file path for an app type
pub fn get_prompt_file_path(app_type: &AppType) -> Option<PathBuf> {
    match app_type {
        AppType::Claude => Some(get_claude_config_dir().join("CLAUDE.md")),
        AppType::Codex => Some(get_codex_config_dir().join("AGENTS.md")),
        AppType::Gemini => Some(get_gemini_config_dir().join("GEMINI.md")),
        AppType::OpenCode => Some(get_opencode_config_dir().join("AGENTS.md")),
        AppType::OpenClaw => Some(get_openclaw_config_dir().join("AGENTS.md")),
        AppType::Hermes => Some(get_hermes_config_dir().join("AGENTS.md")),
        AppType::ClaudeDesktop => None, // Not supported
    }
}

fn get_claude_config_dir() -> PathBuf {
    crate::config::get_home_dir().join(".claude")
}

fn get_codex_config_dir() -> PathBuf {
    crate::config::get_home_dir().join(".codex")
}

fn get_gemini_config_dir() -> PathBuf {
    crate::config::get_home_dir().join(".gemini")
}

fn get_opencode_config_dir() -> PathBuf {
    crate::config::get_home_dir().join(".opencode")
}

fn get_openclaw_config_dir() -> PathBuf {
    crate::config::get_home_dir().join(".openclaw")
}

fn get_hermes_config_dir() -> PathBuf {
    crate::config::get_home_dir().join(".hermes")
}

/// Read current prompt file content
pub fn read_prompt_file(app_type: &AppType) -> AppResult<Option<String>> {
    let path = get_prompt_file_path(app_type)
        .ok_or_else(|| AppError::Prompt(format!("Prompts not supported for {:?}", app_type)))?;

    if !path.exists() {
        return Ok(None);
    }

    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| AppError::Io(e))
}

/// Write prompt file content
pub fn write_prompt_file(app_type: &AppType, content: &str) -> AppResult<()> {
    let path = get_prompt_file_path(app_type)
        .ok_or_else(|| AppError::Prompt(format!("Prompts not supported for {:?}", app_type)))?;

    crate::config::atomic_write(&path, content.as_bytes())
}
