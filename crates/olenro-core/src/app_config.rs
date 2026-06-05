//! App configuration types
//!
//! Migrated from src-tauri/src/app_config.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::provider::AppType;

/// MCP server definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// Installed skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: String,
    pub source: SkillSource,
    pub enabled_apps: Vec<String>,
}

/// Skill installation source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillSource {
    GitHub { repo: String },
    Zip { url: String },
    Local { path: String },
}

/// MCP apps configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpApps {
    pub claude: Vec<McpServer>,
    pub codex: Vec<McpServer>,
    pub gemini: Vec<McpServer>,
    pub opencode: Vec<McpServer>,
    pub hermes: Vec<McpServer>,
}

impl McpApps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, app: &AppType) -> &[McpServer] {
        match app {
            AppType::Claude => &self.claude,
            AppType::Codex => &self.codex,
            AppType::Gemini => &self.gemini,
            AppType::OpenCode => &self.opencode,
            AppType::Hermes => &self.hermes,
            AppType::ClaudeDesktop | AppType::OpenClaw => &[],
        }
    }

    pub fn get_mut(&mut self, app: &AppType) -> &mut Vec<McpServer> {
        match app {
            AppType::Claude => &mut self.claude,
            AppType::Codex => &mut self.codex,
            AppType::Gemini => &mut self.gemini,
            AppType::OpenCode => &mut self.opencode,
            AppType::Hermes => &mut self.hermes,
            AppType::ClaudeDesktop | AppType::OpenClaw => {
                // These apps don't support MCP - use UnsafeCell for interior mutability
                thread_local! {
                    static EMPTY_VEC: std::cell::UnsafeCell<Vec<McpServer>> = std::cell::UnsafeCell::new(Vec::new());
                }
                EMPTY_VEC.with(|v| unsafe { &mut *v.get() })
            }
        }
    }

    pub fn is_enabled_for(&self, app: &AppType, server_id: &str) -> bool {
        self.get(app).iter().any(|s| s.id == server_id && s.enabled)
    }

    pub fn set_enabled_for(&mut self, app: &AppType, server_id: &str, enabled: bool) {
        if let Some(server) = self.get_mut(app).iter_mut().find(|s| s.id == server_id) {
            server.enabled = enabled;
        }
    }
}

/// Skill apps configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillApps {
    pub claude: Vec<InstalledSkill>,
    pub codex: Vec<InstalledSkill>,
    pub gemini: Vec<InstalledSkill>,
    pub opencode: Vec<InstalledSkill>,
    pub openclaw: Vec<InstalledSkill>,
    pub hermes: Vec<InstalledSkill>,
}

impl SkillApps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, app: &AppType) -> &[InstalledSkill] {
        match app {
            AppType::Claude => &self.claude,
            AppType::Codex => &self.codex,
            AppType::Gemini => &self.gemini,
            AppType::OpenCode => &self.opencode,
            AppType::OpenClaw => &self.openclaw,
            AppType::Hermes => &self.hermes,
            AppType::ClaudeDesktop => &[],
        }
    }

    pub fn enabled_apps(&self) -> Vec<AppType> {
        let mut apps = Vec::new();
        if !self.claude.is_empty() {
            apps.push(AppType::Claude);
        }
        if !self.codex.is_empty() {
            apps.push(AppType::Codex);
        }
        if !self.gemini.is_empty() {
            apps.push(AppType::Gemini);
        }
        if !self.opencode.is_empty() {
            apps.push(AppType::OpenCode);
        }
        if !self.openclaw.is_empty() {
            apps.push(AppType::OpenClaw);
        }
        if !self.hermes.is_empty() {
            apps.push(AppType::Hermes);
        }
        apps
    }
}
