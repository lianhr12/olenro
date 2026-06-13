//! Provider data models
//!
//! Migrated from src-tauri/src/provider.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported AI coding tool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppType {
    Claude,
    ClaudeDesktop,
    Codex,
    Gemini,
    OpenCode,
    OpenClaw,
    Hermes,
}

impl AppType {
    /// Returns the config directory name for this app type
    pub fn config_dir_name(&self) -> &'static str {
        match self {
            AppType::Claude => ".claude",
            AppType::ClaudeDesktop => ".claude-desktop",
            AppType::Codex => ".codex",
            AppType::Gemini => ".gemini",
            AppType::OpenCode => ".opencode",
            AppType::OpenClaw => ".openclaw",
            AppType::Hermes => ".hermes",
        }
    }

    /// Parse from string (for deserialization)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "claude" => Some(AppType::Claude),
            "claude-desktop" => Some(AppType::ClaudeDesktop),
            "codex" => Some(AppType::Codex),
            "gemini" => Some(AppType::Gemini),
            "opencode" => Some(AppType::OpenCode),
            "openclaw" => Some(AppType::OpenClaw),
            "hermes" => Some(AppType::Hermes),
            _ => None,
        }
    }

    /// Convert to string for serialization
    pub fn as_str(&self) -> &'static str {
        self.config_dir_name().trim_start_matches('.')
    }
}

impl Default for AppType {
    fn default() -> Self {
        AppType::Claude
    }
}

/// Application configuration state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_type: AppType,
    pub enabled: bool,
    pub current_provider_id: Option<String>,
    pub config_dir: Option<String>,
}

impl AppConfig {
    pub fn new(app_type: AppType) -> Self {
        Self {
            app_type,
            enabled: true,
            current_provider_id: None,
            config_dir: None,
        }
    }
}

/// Multi-app configuration container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiAppConfig {
    pub apps: HashMap<String, AppConfig>,
}

impl MultiAppConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, app_type: &AppType) -> Option<&AppConfig> {
        self.apps.get(app_type.as_str())
    }

    pub fn get_mut(&mut self, app_type: &AppType) -> Option<&mut AppConfig> {
        self.apps.get_mut(app_type.as_str())
    }
}

/// Provider metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMeta {
    /// Custom API endpoints
    #[serde(default)]
    pub custom_endpoints: Option<CustomEndpoints>,

    /// Enable common config snippet
    #[serde(default = "default_true")]
    pub common_config_enabled: bool,

    /// Claude Desktop mode
    #[serde(default)]
    pub claude_desktop_mode: Option<ClaudeDesktopMode>,

    /// Model routes for Claude Desktop
    #[serde(default)]
    pub claude_desktop_model_routes: Option<HashMap<String, String>>,

    /// Usage script configuration
    #[serde(default)]
    pub usage_script: Option<UsageScript>,

    /// Enable endpoint auto-select
    #[serde(default)]
    pub endpoint_auto_select: Option<EndpointAutoSelect>,

    /// Partner promotion key
    #[serde(default)]
    pub is_partner: Option<bool>,

    /// Partner promotion key
    #[serde(default)]
    pub partner_promotion_key: Option<String>,

    /// Cost multiplier
    #[serde(default)]
    pub cost_multiplier: Option<f64>,

    /// Pricing model source
    #[serde(default)]
    pub pricing_model_source: Option<PricingModelSource>,

    /// API format
    #[serde(default)]
    pub api_format: Option<ClaudeApiFormat>,

    /// Auth binding
    #[serde(default)]
    pub auth_binding: Option<AuthBinding>,

    /// API key field name
    #[serde(default)]
    pub api_key_field: Option<String>,

    /// Is full URL
    #[serde(default = "default_true")]
    pub is_full_url: bool,

    /// Prompt cache key
    #[serde(default)]
    pub prompt_cache_key: Option<String>,

    /// Codex fast mode
    #[serde(default)]
    pub codex_fast_mode: Option<bool>,

    /// Codex chat reasoning
    #[serde(default)]
    pub codex_chat_reasoning: Option<CodexChatReasoning>,

    /// Provider type
    #[serde(default)]
    pub provider_type: Option<ProviderType>,

    /// GitHub account ID
    #[serde(default)]
    pub github_account_id: Option<String>,

    /// Explicit provider key used as the stable identifier inside an app's
    /// config file (`models.providers.<key>` for OpenClaw, etc.). When set,
    /// switching writes under this key instead of deriving one from the name.
    /// Must match `^[a-z0-9]+(-[a-z0-9]+)*$`.
    #[serde(default)]
    pub provider_key: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEndpoints {
    pub claude: Option<String>,
    pub codex: Option<String>,
    pub gemini: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeDesktopMode {
    Auto,
    CrossTasks,
    Legacy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScript {
    pub enabled: bool,
    pub script: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointAutoSelect {
    pub enabled: bool,
    pub providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingModelSource {
    Official,
    Custom,
    #[serde(rename = "coding_plan")]
    CodingPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeApiFormat {
    Anthropic,
    OpenAiChat,
    OpenAiResponses,
    GeminiNative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthBinding {
    pub provider: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexChatReasoning {
    Enabled,
    Disabled,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Claude,
    ClaudeAuth,
    Codex,
    Gemini,
    GeminiCli,
    OpenRouter,
    GitHubCopilot,
    CodexOauth,
}

impl ProviderType {
    pub fn default_endpoint(&self) -> &'static str {
        match self {
            ProviderType::Claude | ProviderType::ClaudeAuth => "https://api.anthropic.com",
            ProviderType::Codex => "https://api.openai.com",
            ProviderType::Gemini | ProviderType::GeminiCli => {
                "https://generativelanguage.googleapis.com"
            }
            ProviderType::OpenRouter => "https://openrouter.ai/api",
            ProviderType::GitHubCopilot => "https://api.githubcopilot.com",
            ProviderType::CodexOauth => "https://chatgpt.com/backend-api/codex",
        }
    }
}

/// Main Provider structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub settings_config: serde_json::Value,
    pub website_url: Option<String>,
    pub category: ProviderCategory,
    pub created_at: i64,
    pub sort_index: i32,
    pub notes: Option<String>,
    pub is_partner: bool,
    pub meta: ProviderMeta,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
    pub in_failover_queue: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCategory {
    Official,
    CnOfficial,
    CloudProvider,
    Aggregator,
    ThirdParty,
    Custom,
    Omo,
    #[serde(rename = "omo-slim")]
    OmoSlim,
}

impl Provider {
    /// Check if this provider uses managed account auth (like GitHub Copilot)
    pub fn uses_managed_account_auth(&self) -> bool {
        self.meta.provider_type == Some(ProviderType::GitHubCopilot)
            || self.meta.provider_type == Some(ProviderType::CodexOauth)
    }

    /// Check if this is a Codex OAuth provider
    pub fn is_codex_oauth(&self) -> bool {
        self.meta.provider_type == Some(ProviderType::CodexOauth)
    }

    /// Check if this is a GitHub Copilot provider
    pub fn is_github_copilot(&self) -> bool {
        self.meta.provider_type == Some(ProviderType::GitHubCopilot)
    }

    /// Check if provider has usage script enabled
    pub fn has_usage_script_enabled(&self) -> bool {
        self.meta
            .usage_script
            .as_ref()
            .map(|s| s.enabled)
            .unwrap_or(false)
    }

    /// Check if Codex fast mode is enabled
    pub fn codex_fast_mode_enabled(&self) -> bool {
        self.meta.codex_fast_mode.unwrap_or(false)
    }
}

/// Provider creation input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderInput {
    pub name: String,
    pub settings_config: serde_json::Value,
    pub website_url: Option<String>,
    pub category: ProviderCategory,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub icon_color: Option<String>,
}

// ============================================================================
// 统一供应商（Universal Provider）- 跨应用共享配置
//
// 由 src-tauri/src/provider.rs 移植：跨 App（Claude/Codex/Gemini）共享同一套
// base_url/api_key，并为各 App 单独配置模型。存储为 settings 表中的 JSON blob。
// ============================================================================

/// 统一供应商的应用启用状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UniversalProviderApps {
    #[serde(default)]
    pub claude: bool,
    #[serde(default)]
    pub codex: bool,
    #[serde(default)]
    pub gemini: bool,
}

/// Claude 模型配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClaudeModelConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "haikuModel")]
    pub haiku_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sonnetModel")]
    pub sonnet_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "opusModel")]
    pub opus_model: Option<String>,
}

/// Codex 模型配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexModelConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "reasoningEffort")]
    pub reasoning_effort: Option<String>,
}

/// Gemini 模型配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeminiModelConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// 各应用的模型配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UniversalProviderModels {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude: Option<ClaudeModelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex: Option<CodexModelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini: Option<GeminiModelConfig>,
}

/// 统一供应商（跨应用共享配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalProvider {
    pub id: String,
    pub name: String,
    #[serde(rename = "providerType")]
    pub provider_type: String,
    pub apps: UniversalProviderApps,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(default)]
    pub models: UniversalProviderModels,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iconColor")]
    pub icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProviderMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sortIndex")]
    pub sort_index: Option<usize>,
}

impl UniversalProvider {
    /// 创建新的统一供应商
    pub fn new(
        id: String,
        name: String,
        provider_type: String,
        base_url: String,
        api_key: String,
    ) -> Self {
        Self {
            id,
            name,
            provider_type,
            apps: UniversalProviderApps::default(),
            base_url,
            api_key,
            models: UniversalProviderModels::default(),
            website_url: None,
            notes: None,
            icon: None,
            icon_color: None,
            meta: None,
            created_at: Some(chrono::Utc::now().timestamp_millis()),
            sort_index: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_type_config_dir() {
        assert_eq!(AppType::Claude.config_dir_name(), ".claude");
        assert_eq!(AppType::ClaudeDesktop.config_dir_name(), ".claude-desktop");
        assert_eq!(AppType::Codex.config_dir_name(), ".codex");
    }

    #[test]
    fn test_provider_uses_managed_account_auth() {
        let mut provider = Provider {
            id: "test".to_string(),
            name: "Test".to_string(),
            settings_config: serde_json::json!({}),
            website_url: None,
            category: ProviderCategory::Official,
            created_at: 0,
            sort_index: 0,
            notes: None,
            is_partner: false,
            meta: ProviderMeta {
                custom_endpoints: None,
                common_config_enabled: true,
                claude_desktop_mode: None,
                claude_desktop_model_routes: None,
                usage_script: None,
                endpoint_auto_select: None,
                is_partner: None,
                partner_promotion_key: None,
                cost_multiplier: None,
                pricing_model_source: None,
                api_format: None,
                auth_binding: None,
                api_key_field: None,
                is_full_url: true,
                prompt_cache_key: None,
                codex_fast_mode: None,
                codex_chat_reasoning: None,
                provider_type: Some(ProviderType::GitHubCopilot),
                github_account_id: None,
                provider_key: None,
            },
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        };

        assert!(provider.uses_managed_account_auth());
        assert!(!provider.is_codex_oauth());

        provider.meta.provider_type = Some(ProviderType::CodexOauth);
        assert!(provider.uses_managed_account_auth());
        assert!(provider.is_codex_oauth());
    }
}
