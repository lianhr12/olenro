//! Provider service
//!
//! Business logic for provider management

use crate::database::dao::ProvidersDao;
use crate::error::{AppError, AppResult};
use crate::provider::{Provider, ProviderCategory, AppType};
use std::sync::Arc;
use std::path::PathBuf;

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
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| AppError::Database(e))?;

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
        settings.insert("base_url".to_string(), serde_json::Value::String(endpoint.to_string()));

        if let Some(key) = api_key {
            settings.insert("api_key".to_string(), serde_json::Value::String(key.to_string()));
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

    /// Switch to a different provider for an app
    pub fn switch_provider(&self, provider_id: &str, _app_type: AppType) -> AppResult<()> {
        // For now, just verify the provider exists
        let provider = self.get_provider(provider_id)?
            .ok_or_else(|| AppError::Provider(format!("Provider {} not found", provider_id)))?;

        // TODO: Write to live config file for the target app
        // This would involve:
        // 1. Reading the app's current config
        // 2. Updating the API endpoint and key
        // 3. Writing back atomically

        eprintln!("Switched to provider: {} ({})", provider.name, provider_id);
        Ok(())
    }
}
