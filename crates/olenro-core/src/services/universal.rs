//! Universal provider service
//!
//! 跨 App（Claude/Codex/Gemini）共享的统一供应商管理。数据存储为 `settings`
//! 表中 key=`universal_providers` 的 JSON blob（`HashMap<id, UniversalProvider>`），
//! 与桌面端 `src-tauri/src/database/dao/universal_providers.rs` 保持一致。

use crate::error::{AppError, AppResult};
use crate::provider::UniversalProvider;
use std::collections::HashMap;
use std::path::PathBuf;

/// settings 表中统一供应商数据的 key。
const UNIVERSAL_PROVIDERS_KEY: &str = "universal_providers";

/// 统一供应商服务
pub struct UniversalService {
    db_path: PathBuf,
}

impl UniversalService {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// 打开连接并确保 settings 表存在。
    fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&rusqlite::Connection) -> AppResult<T>,
    {
        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = rusqlite::Connection::open(&self.db_path).map_err(AppError::Database)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .map_err(AppError::Database)?;
        f(&conn)
    }

    /// 读取全部统一供应商（id -> provider）。
    fn read_all(conn: &rusqlite::Connection) -> AppResult<HashMap<String, UniversalProvider>> {
        let mut stmt = conn
            .prepare("SELECT value FROM settings WHERE key = ?")
            .map_err(AppError::Database)?;
        let json: Option<String> = stmt
            .query_row([UNIVERSAL_PROVIDERS_KEY], |row| row.get(0))
            .ok();
        match json {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Config(format!("解析统一供应商数据失败: {e}"))),
            None => Ok(HashMap::new()),
        }
    }

    /// 写回全部统一供应商。
    fn write_all(
        conn: &rusqlite::Connection,
        providers: &HashMap<String, UniversalProvider>,
    ) -> AppResult<()> {
        let json = serde_json::to_string(providers)?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            rusqlite::params![UNIVERSAL_PROVIDERS_KEY, json],
        )
        .map_err(AppError::Database)?;
        Ok(())
    }

    /// 列出全部统一供应商，按 sort_index 然后 name 排序。
    pub fn list(&self) -> AppResult<Vec<UniversalProvider>> {
        self.with_conn(|conn| {
            let mut providers: Vec<UniversalProvider> =
                Self::read_all(conn)?.into_values().collect();
            providers.sort_by(|a, b| {
                a.sort_index
                    .unwrap_or(usize::MAX)
                    .cmp(&b.sort_index.unwrap_or(usize::MAX))
                    .then_with(|| a.name.cmp(&b.name))
            });
            Ok(providers)
        })
    }

    /// 获取单个统一供应商。
    pub fn get(&self, id: &str) -> AppResult<Option<UniversalProvider>> {
        self.with_conn(|conn| Ok(Self::read_all(conn)?.get(id).cloned()))
    }

    /// 保存（新增或更新）统一供应商。
    pub fn save(&self, provider: &UniversalProvider) -> AppResult<()> {
        self.with_conn(|conn| {
            let mut providers = Self::read_all(conn)?;
            providers.insert(provider.id.clone(), provider.clone());
            Self::write_all(conn, &providers)
        })
    }

    /// 删除统一供应商，返回是否存在过。
    pub fn delete(&self, id: &str) -> AppResult<bool> {
        self.with_conn(|conn| {
            let mut providers = Self::read_all(conn)?;
            let existed = providers.remove(id).is_some();
            if existed {
                Self::write_all(conn, &providers)?;
            }
            Ok(existed)
        })
    }

    /// 便捷方法：创建并保存一个新的统一供应商，返回它。
    pub fn add(
        &self,
        name: &str,
        provider_type: &str,
        base_url: &str,
        api_key: &str,
    ) -> AppResult<UniversalProvider> {
        let id = format!("up_{}", chrono::Utc::now().timestamp_millis());
        let provider = UniversalProvider::new(
            id,
            name.to_string(),
            provider_type.to_string(),
            base_url.to_string(),
            api_key.to_string(),
        );
        self.save(&provider)?;
        Ok(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_service() -> (tempfile::TempDir, UniversalService) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("test.db");
        (dir, UniversalService::new(db))
    }

    #[test]
    fn crud_roundtrip() {
        let (_dir, svc) = temp_service();
        assert!(svc.list().unwrap().is_empty());

        let mut p = svc
            .add(
                "OpenRouter",
                "custom",
                "https://openrouter.ai/api/v1",
                "sk-1",
            )
            .unwrap();
        assert_eq!(svc.list().unwrap().len(), 1);

        // toggle apps + persist
        p.apps.claude = true;
        p.apps.codex = true;
        svc.save(&p).unwrap();
        let got = svc.get(&p.id).unwrap().unwrap();
        assert!(got.apps.claude && got.apps.codex && !got.apps.gemini);
        assert_eq!(got.base_url, "https://openrouter.ai/api/v1");

        assert!(svc.delete(&p.id).unwrap());
        assert!(!svc.delete(&p.id).unwrap());
        assert!(svc.list().unwrap().is_empty());
    }
}
