//! Provider service
//!
//! Business logic for provider management

use crate::database::dao::ProvidersDao;
use crate::error::{AppError, AppResult};
use crate::provider::{AppType, Provider, ProviderCategory};
use std::path::PathBuf;
use std::sync::Arc;

/// Shared provider service
pub type SharedProviderService = Arc<ProviderService>;

/// Provider service for managing AI providers
pub struct ProviderService {
    db_path: PathBuf,
}

impl ProviderService {
    /// Create a new ProviderService
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Get database connection
    fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&ProvidersDao) -> AppResult<T>,
    {
        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = rusqlite::Connection::open(&self.db_path).map_err(|e| AppError::Database(e))?;

        // Initialize schema if needed - create tables if they don't exist
        let schema = r#"
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                sort_index INTEGER NOT NULL DEFAULT 0,
                notes TEXT,
                is_partner INTEGER NOT NULL DEFAULT 0,
                meta TEXT NOT NULL DEFAULT '{}',
                icon TEXT,
                icon_color TEXT,
                in_failover_queue INTEGER NOT NULL DEFAULT 0
            );
        "#;
        conn.execute_batch(schema)
            .map_err(|e| AppError::Database(e))?;

        let dao = ProvidersDao::new(&conn);
        f(&dao)
    }

    /// List all providers
    pub fn list_providers(&self) -> AppResult<Vec<Provider>> {
        self.with_conn(|dao| dao.list_all())
    }

    /// Get provider by ID
    pub fn get_provider(&self, id: &str) -> AppResult<Option<Provider>> {
        self.with_conn(|dao| dao.get_by_id(id))
    }

    /// Create a new provider
    pub fn create_provider(&self, provider: Provider) -> AppResult<()> {
        self.with_conn(|dao| dao.insert(&provider))
    }

    /// Update an existing provider
    pub fn update_provider(&self, provider: Provider) -> AppResult<()> {
        self.with_conn(|dao| dao.update(&provider))
    }

    /// Delete a provider
    pub fn delete_provider(&self, id: &str) -> AppResult<()> {
        self.with_conn(|dao| dao.delete(id))
    }

    /// Providers in the failover queue, ordered by sort_index (queue priority).
    pub fn list_failover_queue(&self) -> AppResult<Vec<Provider>> {
        Ok(self
            .list_providers()?
            .into_iter()
            .filter(|p| p.in_failover_queue)
            .collect())
    }

    /// Add/remove a provider from the failover queue.
    pub fn set_in_failover_queue(&self, id: &str, enabled: bool) -> AppResult<()> {
        match self.get_provider(id)? {
            Some(mut p) => {
                p.in_failover_queue = enabled;
                self.update_provider(p)
            }
            None => Err(AppError::NotFound(format!("provider not found: {id}"))),
        }
    }

    /// Create a new provider with given parameters
    pub fn add_provider(
        &self,
        name: &str,
        endpoint: &str,
        category: ProviderCategory,
        api_key: Option<&str>,
    ) -> AppResult<Provider> {
        let sort_index = self.with_conn(|dao| dao.get_max_sort_index())? + 1;

        // Build settings_config
        let mut settings = serde_json::Map::new();
        settings.insert(
            "base_url".to_string(),
            serde_json::Value::String(endpoint.to_string()),
        );

        if let Some(key) = api_key {
            settings.insert(
                "api_key".to_string(),
                serde_json::Value::String(key.to_string()),
            );
        }

        let provider = Provider {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            settings_config: serde_json::Value::Object(settings),
            website_url: None,
            category,
            created_at: chrono::Utc::now().timestamp(),
            sort_index,
            notes: None,
            is_partner: false,
            meta: crate::provider::ProviderMeta {
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
                provider_type: None,
                github_account_id: None,
            },
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        };

        self.create_provider(provider.clone())?;
        Ok(provider)
    }

    /// Create a provider from a built-in preset, substituting template
    /// variables and injecting the API key into the app-appropriate slot.
    /// `endpoint_override`, when set, replaces the preset's base URL (both the
    /// top-level `base_url` and any `*_BASE_URL` env var) so the user can edit
    /// the prefilled endpoint or pick a different candidate.
    pub fn create_from_preset(
        &self,
        preset: &crate::provider_presets::ProviderPreset,
        api_key: Option<&str>,
        templates: &std::collections::HashMap<String, String>,
        endpoint_override: Option<&str>,
    ) -> AppResult<Provider> {
        let sort_index = self.with_conn(|dao| dao.get_max_sort_index())? + 1;
        let mut provider = preset.to_provider(api_key, templates, sort_index);
        if let Some(ep) = endpoint_override.filter(|e| !e.is_empty()) {
            apply_endpoint_override(&mut provider.settings_config, ep);
        }
        self.create_provider(provider.clone())?;
        Ok(provider)
    }

    /// Switch to a different provider for an app by writing its endpoint and
    /// credentials into the target app's live configuration file.
    ///
    /// Currently only Claude Code (`~/.claude/settings.json`) is supported for
    /// CLI/server use; other apps return an explanatory error rather than
    /// silently doing nothing.
    pub fn switch_provider(&self, provider_id: &str, app_type: AppType) -> AppResult<()> {
        let provider = self
            .get_provider(provider_id)?
            .ok_or_else(|| AppError::Provider(format!("Provider {} not found", provider_id)))?;

        match app_type {
            AppType::Claude => {
                write_claude_live(&provider)?;
                log::info!(
                    "Switched Claude to provider: {} ({})",
                    provider.name,
                    provider_id
                );
                Ok(())
            }
            AppType::ClaudeDesktop => {
                write_claude_desktop_live(&provider)?;
                log::info!(
                    "Switched Claude Desktop to provider: {} ({})",
                    provider.name,
                    provider_id
                );
                Ok(())
            }
            AppType::Gemini => {
                write_gemini_live(&provider)?;
                log::info!(
                    "Switched Gemini to provider: {} ({})",
                    provider.name,
                    provider_id
                );
                Ok(())
            }
            AppType::Codex => {
                write_codex_live(&provider)?;
                log::info!(
                    "Switched Codex to provider: {} ({})",
                    provider.name,
                    provider_id
                );
                Ok(())
            }
            AppType::OpenCode => {
                write_opencode_live(&provider)?;
                log::info!(
                    "Switched OpenCode to provider: {} ({})",
                    provider.name,
                    provider_id
                );
                Ok(())
            }
            AppType::Hermes => {
                write_hermes_live(&provider)?;
                log::info!(
                    "Switched Hermes to provider: {} ({})",
                    provider.name,
                    provider_id
                );
                Ok(())
            }
            AppType::OpenClaw => {
                write_openclaw_live(&provider)?;
                log::info!(
                    "Switched OpenClaw to provider: {} ({})",
                    provider.name,
                    provider_id
                );
                Ok(())
            }
        }
    }
}

/// Replace the base URL inside a provider's `settings_config`: set the
/// top-level `base_url` and overwrite any `*_BASE_URL` env var (Claude/Gemini
/// keep the endpoint in the env block).
fn apply_endpoint_override(settings: &mut serde_json::Value, endpoint: &str) {
    if let Some(obj) = settings.as_object_mut() {
        obj.insert(
            "base_url".to_string(),
            serde_json::Value::String(endpoint.to_string()),
        );
        if let Some(env) = obj.get_mut("env").and_then(|e| e.as_object_mut()) {
            for key in env.keys().cloned().collect::<Vec<_>>() {
                if key.ends_with("_BASE_URL") {
                    env.insert(key, serde_json::Value::String(endpoint.to_string()));
                }
            }
        }
    }
}

/// Write a provider's endpoint + credentials into Claude Desktop's
/// `~/.claude-desktop/config.json`, merging with any existing settings.
fn write_claude_desktop_live(provider: &Provider) -> AppResult<()> {
    use crate::app_config_writers::claude_config::{read_claude_desktop_config, write_claude_desktop_config, ClaudeDesktopConfig, ClaudeDesktopSettings};

    let path = crate::app_config_writers::claude_config::get_claude_desktop_config_path();

    // Load existing config or start with a default structure.
    let mut config = if path.exists() {
        read_claude_desktop_config()?.unwrap_or_else(|| ClaudeDesktopConfig {
            version: 1,
            settings: ClaudeDesktopSettings {
                base_url: None,
                api_key: None,
                model: None,
            },
        })
    } else {
        ClaudeDesktopConfig {
            version: 1,
            settings: ClaudeDesktopSettings {
                base_url: None,
                api_key: None,
                model: None,
            },
        }
    };

    // Extract endpoint and API key from provider config.
    let base_url = provider
        .settings_config
        .get("baseUrl")
        .and_then(|v| v.as_str())
        .or_else(|| provider.settings_config.get("base_url").and_then(|v| v.as_str()))
        .filter(|s| !s.is_empty());

    let api_key = if let Some(env) = provider.settings_config.get("env").and_then(|e| e.as_object()) {
        env.get("ANTHROPIC_AUTH_TOKEN")
            .or_else(|| env.get("ANTHROPIC_API_KEY"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    } else {
        provider.settings_config.get("api_key").and_then(|v| v.as_str())
    };

    let model = provider.settings_config.get("model").and_then(|v| v.as_str());

    // Update settings.
    config.settings.base_url = base_url.map(str::to_string);
    config.settings.api_key = api_key.map(str::to_string);
    config.settings.model = model.map(str::to_string);

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }

    write_claude_desktop_config(&config)?;
    log::info!("Wrote Claude Desktop config to {:?}", path);
    Ok(())
}

/// Write a provider's endpoint + credentials into Gemini's
/// `~/.gemini/.env`, preserving existing env vars.
fn write_gemini_live(provider: &Provider) -> AppResult<()> {
    use crate::app_config_writers::gemini_config;

    // Load existing env vars.
    let mut env = gemini_config::read_gemini_env()?;

    // Extract endpoint and API key from provider config.
    let base_url = provider
        .settings_config
        .get("baseURL")
        .and_then(|v| v.as_str())
        .or_else(|| provider.settings_config.get("base_url").and_then(|v| v.as_str()))
        .filter(|s| !s.is_empty());

    if let Some(url) = base_url {
        env.insert("GOOGLE_GEMINI_BASE_URL".to_string(), url.to_string());
    }

    let api_key = if let Some(e) = provider.settings_config.get("env").and_then(|e| e.as_object()) {
        e.get("GOOGLE_GEMINI_API_KEY")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    } else {
        provider.settings_config.get("api_key").and_then(|v| v.as_str())
    };

    if let Some(key) = api_key {
        env.insert("GOOGLE_GEMINI_API_KEY".to_string(), key.to_string());
    }

    gemini_config::write_gemini_env_atomic(&env)?;
    log::info!("Wrote Gemini .env for provider: {}", provider.name);
    Ok(())
}

/// Write a provider's endpoint + credentials into Codex's
/// `~/.codex/config.toml` (as `experimental_bearer_token`).
fn write_codex_live(provider: &Provider) -> AppResult<()> {
    use crate::app_config_writers::codex_config;

    // Get API key from provider config.
    let api_key = if let Some(auth) = provider.settings_config.get("auth").and_then(|a| a.as_object()) {
        auth.get("OPENAI_API_KEY")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    } else {
        provider.settings_config.get("api_key").and_then(|v| v.as_str())
    };

    let key = api_key.ok_or_else(|| {
        AppError::Provider("Codex provider missing API key".to_string())
    })?;

    // Read existing config.toml or start with minimal content.
    let path = codex_config::get_codex_config_path();
    let existing_config = if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_else(|_| String::new())
    } else {
        String::new()
    };

    // Update or create experimental_bearer_token.
    let updated = codex_config::set_codex_bearer_token(&existing_config, key)?;

    codex_config::write_codex_config_text(&updated)?;
    log::info!("Wrote Codex config.toml for provider: {}", provider.name);
    Ok(())
}

/// Write a provider's endpoint + credentials into OpenCode's
/// `~/.opencode/config.json`, storing it in the `provider` field.
fn write_opencode_live(provider: &Provider) -> AppResult<()> {
    use crate::app_config_writers::opencode_config;

    // Build the provider config object from the stored provider.
    let mut provider_config = serde_json::Map::new();

    // Add base_url.
    if let Some(base) = provider.settings_config.get("base_url").and_then(|v| v.as_str()) {
        if !base.is_empty() {
            provider_config.insert("base_url".to_string(), serde_json::json!(base));
        }
    }

    // Add API key.
    if let Some(key) = provider.settings_config.get("api_key").and_then(|v| v.as_str()) {
        if !key.is_empty() {
            provider_config.insert("api_key".to_string(), serde_json::json!(key));
        }
    }

    // Add any env vars.
    if let Some(env) = provider.settings_config.get("env").and_then(|e| e.as_object()) {
        if !env.is_empty() {
            provider_config.insert("env".to_string(), serde_json::json!(env));
        }
    }

    // Load or create the full config.
    let path = opencode_config::get_opencode_config_path();
    let mut full_config = if path.exists() {
        opencode_config::read_opencode_config()?.unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Set the provider field.
    if let Some(obj) = full_config.as_object_mut() {
        obj.insert("provider".to_string(), serde_json::Value::Object(provider_config));
    }

    opencode_config::write_opencode_config(&full_config)?;
    log::info!("Wrote OpenCode config.json for provider: {}", provider.name);
    Ok(())
}

/// Write a provider's endpoint + credentials into Hermes's
/// `~/.hermes/config.yaml`, updating the `model.provider` and related fields.
fn write_hermes_live(provider: &Provider) -> AppResult<()> {
    use crate::app_config_writers::hermes_config;

    let base_url = provider
        .settings_config
        .get("base_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());

    let api_key = provider.settings_config.get("api_key").and_then(|v| v.as_str());

    hermes_config::write_provider_for_switch(
        &provider.name,
        base_url,
        api_key,
        &provider.settings_config,
    )?;

    log::info!("Wrote Hermes config.yaml for provider: {}", provider.name);
    Ok(())
}

/// Write a provider's config into OpenClaw's `~/.openclaw/openclaw.json`,
/// storing it under `models.providers.<id>`.
fn write_openclaw_live(provider: &Provider) -> AppResult<()> {
    use crate::app_config_writers::openclaw_config;

    // Use the provider's name (slugified) as the OpenClaw provider id.
    let provider_id: String = provider
        .name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let provider_id = provider_id.trim_matches('-').to_string();
    let provider_id = if provider_id.is_empty() {
        "custom".to_string()
    } else {
        provider_id
    };

    // The OpenClaw provider config lives in settings_config directly (the
    // preset snapshot stores the OpenClawProviderConfig shape there).
    openclaw_config::set_provider(&provider_id, provider.settings_config.clone())?;
    log::info!(
        "Wrote OpenClaw config.json for provider: {} (id={})",
        provider.name,
        provider_id
    );
    Ok(())
}

/// Write a provider's endpoint + credentials into Claude Code's
/// `~/.claude/settings.json`, merging with any existing settings so unrelated
/// keys (permissions, mcpServers, other env vars) are preserved.
fn write_claude_live(provider: &Provider) -> AppResult<()> {
    use crate::config::{get_claude_settings_path, read_json_file, write_json_file};

    let path = get_claude_settings_path();

    // Load existing settings (preserve other keys) or start fresh.
    let mut settings: serde_json::Value = if path.exists() {
        read_json_file(&path).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if !settings.is_object() {
        settings = serde_json::json!({});
    }

    // Determine the env vars to apply: an explicit `env` block in the provider
    // config wins; otherwise map base_url/api_key to Claude's env vars.
    let new_env: Vec<(String, serde_json::Value)> = if let Some(env) = provider
        .settings_config
        .get("env")
        .and_then(|e| e.as_object())
    {
        env.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        let mut out = Vec::new();
        if let Some(base) = provider
            .settings_config
            .get("base_url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            out.push((
                "ANTHROPIC_BASE_URL".to_string(),
                serde_json::Value::String(base.to_string()),
            ));
        }
        if let Some(key) = provider
            .settings_config
            .get("api_key")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            out.push((
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                serde_json::Value::String(key.to_string()),
            ));
        }
        out
    };

    // Merge env into settings.env.
    {
        let obj = settings.as_object_mut().unwrap();
        let env = obj.entry("env").or_insert_with(|| serde_json::json!({}));
        if !env.is_object() {
            *env = serde_json::json!({});
        }
        let env_obj = env.as_object_mut().unwrap();
        for (k, v) in new_env {
            env_obj.insert(k, v);
        }
    }

    // Merge any additional non-internal top-level keys from the provider config
    // (e.g. model, permissions) while skipping CLI-internal / already-handled ones.
    if let Some(sc) = provider.settings_config.as_object() {
        let obj = settings.as_object_mut().unwrap();
        for (k, v) in sc {
            if matches!(
                k.as_str(),
                "base_url"
                    | "api_key"
                    | "env"
                    | "api_format"
                    | "apiFormat"
                    | "openrouter_compat_mode"
                    | "openrouterCompatMode"
            ) {
                continue;
            }
            obj.insert(k.clone(), v.clone());
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    write_json_file(&path, &settings)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderCategory;
    use serial_test::serial;

    #[test]
    fn failover_queue_add_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        let svc = ProviderService::new(dir.path().join("test.db"));
        let p = svc
            .add_provider("Acme", "https://acme.test", ProviderCategory::Custom, None)
            .unwrap();
        assert!(svc.list_failover_queue().unwrap().is_empty());

        svc.set_in_failover_queue(&p.id, true).unwrap();
        let queue = svc.list_failover_queue().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].id, p.id);

        svc.set_in_failover_queue(&p.id, false).unwrap();
        assert!(svc.list_failover_queue().unwrap().is_empty());

        assert!(svc.set_in_failover_queue("missing", true).is_err());
    }

    // Both phases mutate the process-global OLENRO_TEST_HOME, so they live in a
    // single (serial) test to avoid racing other tests.
    #[test]
    #[serial]
    fn switch_writes_and_merges_claude_live_config() {
        let home = std::env::temp_dir().join(format!("olenro-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        // Pre-existing settings.json with unrelated keys to verify merge.
        std::fs::write(
            home.join(".claude").join("settings.json"),
            r#"{"permissions":{"allow":["Bash"]},"env":{"FOO":"bar"}}"#,
        )
        .unwrap();
        std::env::set_var("OLENRO_TEST_HOME", &home);

        let db = home.join("olenro.db");
        let svc = ProviderService::new(db.clone());
        let provider = svc
            .add_provider(
                "My Claude",
                "https://api.example.com",
                ProviderCategory::Custom,
                Some("sk-test-123"),
            )
            .unwrap();

        svc.switch_provider(&provider.id, AppType::Claude).unwrap();

        let settings_path = home.join(".claude").join("settings.json");
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();

        // New credentials written.
        assert_eq!(
            v["env"]["ANTHROPIC_BASE_URL"].as_str(),
            Some("https://api.example.com")
        );
        assert_eq!(
            v["env"]["ANTHROPIC_AUTH_TOKEN"].as_str(),
            Some("sk-test-123")
        );
        // Unrelated pre-existing keys preserved.
        assert_eq!(v["permissions"]["allow"][0].as_str(), Some("Bash"));
        assert_eq!(v["env"]["FOO"].as_str(), Some("bar"));

        std::env::remove_var("OLENRO_TEST_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Each non-Claude app now writes its own live config file on switch.
    #[test]
    #[serial]
    fn switch_writes_each_app_live_config() {
        let home = std::env::temp_dir().join(format!("olenro-switch-apps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("OLENRO_TEST_HOME", &home);

        let svc = ProviderService::new(home.join("olenro.db"));

        // Gemini: writes ~/.gemini/.env with base url + key.
        let gemini = svc
            .add_provider(
                "My Gemini",
                "https://gemini.example.com",
                ProviderCategory::Custom,
                Some("g-key"),
            )
            .unwrap();
        svc.switch_provider(&gemini.id, AppType::Gemini).unwrap();
        let env = std::fs::read_to_string(home.join(".gemini").join(".env")).unwrap();
        assert!(env.contains("GOOGLE_GEMINI_BASE_URL=https://gemini.example.com"));
        assert!(env.contains("GOOGLE_GEMINI_API_KEY=g-key"));

        // Codex: writes ~/.codex/config.toml with the bearer token.
        let codex = svc
            .add_provider(
                "My Codex",
                "https://codex.example.com",
                ProviderCategory::Custom,
                Some("c-key"),
            )
            .unwrap();
        svc.switch_provider(&codex.id, AppType::Codex).unwrap();
        let toml = std::fs::read_to_string(home.join(".codex").join("config.toml")).unwrap();
        assert!(toml.contains("experimental_bearer_token"));
        assert!(toml.contains("c-key"));

        // OpenCode: writes ~/.opencode/config.json under `provider`.
        let oc = svc
            .add_provider(
                "My OpenCode",
                "https://oc.example.com",
                ProviderCategory::Custom,
                Some("o-key"),
            )
            .unwrap();
        svc.switch_provider(&oc.id, AppType::OpenCode).unwrap();
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.join(".opencode").join("config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(json["provider"]["api_key"].as_str(), Some("o-key"));
        assert_eq!(
            json["provider"]["base_url"].as_str(),
            Some("https://oc.example.com")
        );

        // Hermes: writes ~/.hermes/config.yaml under model.provider.
        let hermes = svc
            .add_provider(
                "My Hermes",
                "https://hermes.example.com",
                ProviderCategory::Custom,
                Some("h-key"),
            )
            .unwrap();
        svc.switch_provider(&hermes.id, AppType::Hermes).unwrap();
        let yaml = std::fs::read_to_string(home.join(".hermes").join("config.yaml")).unwrap();
        assert!(yaml.contains("provider: My Hermes"));
        assert!(yaml.contains("h-key"));

        // Claude Desktop: writes ~/.claude-desktop/config.json.
        let cd = svc
            .add_provider(
                "My CD",
                "https://cd.example.com",
                ProviderCategory::Custom,
                Some("cd-key"),
            )
            .unwrap();
        svc.switch_provider(&cd.id, AppType::ClaudeDesktop).unwrap();
        let cd_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.join(".claude-desktop").join("config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            cd_json["settings"]["base_url"].as_str(),
            Some("https://cd.example.com")
        );

        // OpenClaw: writes ~/.openclaw/config.json under models.providers.
        let openclaw = svc
            .add_provider(
                "My OpenClaw",
                "https://openclaw.example.com",
                ProviderCategory::Custom,
                Some("oc-key"),
            )
            .unwrap();
        svc.switch_provider(&openclaw.id, AppType::OpenClaw).unwrap();
        // OpenClaw config is JSON5 (unquoted keys) — parse leniently.
        let oc_text =
            std::fs::read_to_string(home.join(".openclaw").join("openclaw.json")).unwrap();
        let oc_json: serde_json::Value = json5::from_str(&oc_text).unwrap();
        assert!(oc_json["models"]["providers"]["my-openclaw"].is_object());

        std::env::remove_var("OLENRO_TEST_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }
}
