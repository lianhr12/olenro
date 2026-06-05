//! Providers DAO
//!
//! Data access for providers table

use crate::error::{AppError, AppResult};
use crate::provider::{Provider, ProviderCategory, ProviderMeta};
use rusqlite::{params, Connection, Row};
use std::sync::Arc;

pub struct ProvidersDao<'a> {
    conn: &'a Connection,
}

impl<'a> ProvidersDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all providers ordered by sort_index
    pub fn list_all(&self) -> AppResult<Vec<Provider>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at,
                    sort_index, notes, is_partner, meta, icon, icon_color, in_failover_queue
             FROM providers ORDER BY sort_index ASC"
        )?;

        let providers = stmt
            .query_map([], |row| self.row_to_provider(row))
            .map_err(|e| AppError::Database(e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(providers)
    }

    /// Get provider by ID
    pub fn get_by_id(&self, id: &str) -> AppResult<Option<Provider>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at,
                    sort_index, notes, is_partner, meta, icon, icon_color, in_failover_queue
             FROM providers WHERE id = ?"
        )?;

        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::Database(e))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e))? {
            Ok(Some(self.row_to_provider(row)?))
        } else {
            Ok(None)
        }
    }

    /// Insert a new provider
    pub fn insert(&self, provider: &Provider) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO providers (id, name, settings_config, website_url, category,
                                    created_at, sort_index, notes, is_partner, meta, icon, icon_color, in_failover_queue)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                provider.id,
                provider.name,
                serde_json::to_string(&provider.settings_config).unwrap_or_default(),
                provider.website_url,
                category_to_string(&provider.category),
                provider.created_at,
                provider.sort_index,
                provider.notes,
                provider.is_partner as i32,
                serde_json::to_string(&provider.meta).unwrap_or_default(),
                provider.icon,
                provider.icon_color,
                provider.in_failover_queue as i32,
            ],
        ).map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Update an existing provider
    pub fn update(&self, provider: &Provider) -> AppResult<()> {
        self.conn.execute(
            "UPDATE providers SET name = ?, settings_config = ?, website_url = ?,
                                 category = ?, sort_index = ?, notes = ?, is_partner = ?,
                                 meta = ?, icon = ?, icon_color = ?, in_failover_queue = ?
             WHERE id = ?",
            params![
                provider.name,
                serde_json::to_string(&provider.settings_config).unwrap_or_default(),
                provider.website_url,
                category_to_string(&provider.category),
                provider.sort_index,
                provider.notes,
                provider.is_partner as i32,
                serde_json::to_string(&provider.meta).unwrap_or_default(),
                provider.icon,
                provider.icon_color,
                provider.in_failover_queue as i32,
                provider.id,
            ],
        ).map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Delete a provider by ID
    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.conn.execute("DELETE FROM providers WHERE id = ?", params![id])
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Get max sort_index
    pub fn get_max_sort_index(&self) -> AppResult<i32> {
        let max: Option<i32> = self.conn
            .query_row("SELECT MAX(sort_index) FROM providers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e))?;
        Ok(max.unwrap_or(0))
    }

    fn row_to_provider(&self, row: &Row) -> std::result::Result<Provider, rusqlite::Error> {
        let settings_config_str: String = row.get(2).unwrap_or_default();
        let meta_str: String = row.get(9).unwrap_or_default();

        let settings_config = serde_json::from_str(&settings_config_str)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let meta: ProviderMeta = serde_json::from_str(&meta_str)
            .unwrap_or(ProviderMeta {
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
            });

        let category_str: String = row.get(4).unwrap_or_default();

        Ok(Provider {
            id: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            settings_config,
            website_url: row.get(3).ok(),
            category: string_to_category(&category_str),
            created_at: row.get(5).unwrap_or(0),
            sort_index: row.get(6).unwrap_or(0),
            notes: row.get(7).ok(),
            is_partner: row.get::<_, i32>(8).unwrap_or(0) != 0,
            meta,
            icon: row.get(10).ok(),
            icon_color: row.get(11).ok(),
            in_failover_queue: row.get::<_, i32>(12).unwrap_or(0) != 0,
        })
    }
}

fn category_to_string(category: &ProviderCategory) -> String {
    match category {
        ProviderCategory::Official => "official".to_string(),
        ProviderCategory::CnOfficial => "cn_official".to_string(),
        ProviderCategory::CloudProvider => "cloud_provider".to_string(),
        ProviderCategory::Aggregator => "aggregator".to_string(),
        ProviderCategory::ThirdParty => "third_party".to_string(),
        ProviderCategory::Custom => "custom".to_string(),
        ProviderCategory::Omo => "omo".to_string(),
        ProviderCategory::OmoSlim => "omo-slim".to_string(),
    }
}

fn string_to_category(s: &str) -> ProviderCategory {
    match s {
        "official" => ProviderCategory::Official,
        "cn_official" => ProviderCategory::CnOfficial,
        "cloud_provider" => ProviderCategory::CloudProvider,
        "aggregator" => ProviderCategory::Aggregator,
        "third_party" => ProviderCategory::ThirdParty,
        "custom" => ProviderCategory::Custom,
        "omo" => ProviderCategory::Omo,
        "omo-slim" => ProviderCategory::OmoSlim,
        _ => ProviderCategory::Custom,
    }
}
