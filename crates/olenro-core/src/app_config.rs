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

/// 已安装的 Skill（v3.10.0+ 统一结构，由桌面端移植）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkill {
    /// 唯一标识符（格式："owner/repo:directory" 或 "local:directory"）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 安装目录名（在 SSOT 目录中的子目录名）
    pub directory: String,
    /// 仓库所有者（GitHub 用户/组织）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_owner: Option<String>,
    /// 仓库名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_name: Option<String>,
    /// 仓库分支
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_branch: Option<String>,
    /// README URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme_url: Option<String>,
    /// 应用启用状态
    pub apps: SkillApps,
    /// 安装时间（Unix 时间戳）
    pub installed_at: i64,
    /// 内容哈希（SHA-256，用于更新检测）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// 最近更新时间（Unix 时间戳，0 = 从未更新）
    #[serde(default)]
    pub updated_at: i64,
}

/// 未管理的 Skill（在应用目录中发现但未被 Olenro 管理）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmanagedSkill {
    /// 目录名
    pub directory: String,
    /// 显示名称（从 SKILL.md 解析）
    pub name: String,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 在哪些应用目录中发现（如 ["claude", "codex"]）
    pub found_in: Vec<String>,
    /// 发现路径（首个匹配的完整路径）
    pub path: String,
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

/// Skill 应用启用状态（标记 Skill 应用到哪些客户端，由桌面端移植）。
///
/// OpenClaw / ClaudeDesktop 不支持 Olenro skill 同步，对应启用状态恒为 false。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SkillApps {
    #[serde(default)]
    pub claude: bool,
    #[serde(default)]
    pub codex: bool,
    #[serde(default)]
    pub gemini: bool,
    #[serde(default)]
    pub opencode: bool,
    #[serde(default)]
    pub hermes: bool,
}

impl SkillApps {
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查指定应用是否启用。
    pub fn is_enabled_for(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.claude,
            AppType::Codex => self.codex,
            AppType::Gemini => self.gemini,
            AppType::OpenCode => self.opencode,
            AppType::Hermes => self.hermes,
            AppType::OpenClaw => false,
            AppType::ClaudeDesktop => false,
        }
    }

    /// 设置指定应用的启用状态。
    pub fn set_enabled_for(&mut self, app: &AppType, enabled: bool) {
        match app {
            AppType::Claude => self.claude = enabled,
            AppType::Codex => self.codex = enabled,
            AppType::Gemini => self.gemini = enabled,
            AppType::OpenCode => self.opencode = enabled,
            AppType::Hermes => self.hermes = enabled,
            AppType::OpenClaw => {}
            AppType::ClaudeDesktop => {}
        }
    }

    /// 获取所有启用的应用列表。
    pub fn enabled_apps(&self) -> Vec<AppType> {
        let mut apps = Vec::new();
        if self.claude {
            apps.push(AppType::Claude);
        }
        if self.codex {
            apps.push(AppType::Codex);
        }
        if self.gemini {
            apps.push(AppType::Gemini);
        }
        if self.opencode {
            apps.push(AppType::OpenCode);
        }
        if self.hermes {
            apps.push(AppType::Hermes);
        }
        apps
    }

    /// 检查是否所有应用都未启用。
    pub fn is_empty(&self) -> bool {
        !self.claude && !self.codex && !self.gemini && !self.opencode && !self.hermes
    }

    /// 仅启用指定应用（其他应用设为禁用）。
    pub fn only(app: &AppType) -> Self {
        let mut apps = Self::default();
        apps.set_enabled_for(app, true);
        apps
    }

    /// 从来源标签列表构建启用状态。
    ///
    /// 标签与 `AppType::as_str()` 一致时启用对应应用，其他标签
    /// （如 "agents"、"cc-switch"）忽略。
    pub fn from_labels(labels: &[String]) -> Self {
        let mut apps = Self::default();
        for label in labels {
            if let Some(app) = AppType::from_str(label) {
                apps.set_enabled_for(&app, true);
            }
        }
        apps
    }
}
