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
            other => Err(AppError::Provider(format!(
                "Live switch for {:?} is not yet supported in the CLI (only Claude). \
                 Configure {:?} via the desktop app.",
                other, other
            ))),
        }
    }
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

        // Non-Claude apps return an explanatory error (not a silent no-op).
        let err = svc
            .switch_provider(&provider.id, AppType::Codex)
            .unwrap_err();
        assert!(format!("{}", err).contains("not yet supported"));

        std::env::remove_var("OLENRO_TEST_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }
}
