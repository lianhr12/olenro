//! Usage service
//!
//! Business logic for usage/cost statistics

use crate::database::dao::usage::{UsageDao, UsageRollup};
use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// Aggregated usage totals across a time window.
#[derive(Debug, Clone, Default)]
pub struct UsageSummary {
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

/// Per-provider usage totals.
#[derive(Debug, Clone)]
pub struct ProviderUsage {
    pub provider_id: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

/// Usage service for recording and querying API usage rollups.
pub struct UsageService {
    db_path: PathBuf,
}

impl UsageService {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&UsageDao) -> AppResult<T>,
    {
        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = rusqlite::Connection::open(&self.db_path).map_err(AppError::Database)?;

        let schema = r#"
            CREATE TABLE IF NOT EXISTS usage_rollup (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                date TEXT NOT NULL,
                requests INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(provider_id, date)
            );
        "#;
        conn.execute_batch(schema).map_err(AppError::Database)?;

        let dao = UsageDao::new(&conn);
        f(&dao)
    }

    /// All rollups from the last `days` days.
    pub fn list_recent(&self, days: i32) -> AppResult<Vec<UsageRollup>> {
        self.with_conn(|dao| dao.list_recent(days))
    }

    /// Aggregate totals across the last `days` days.
    pub fn summary(&self, days: i32) -> AppResult<UsageSummary> {
        let rollups = self.list_recent(days)?;
        let mut s = UsageSummary::default();
        for r in &rollups {
            s.requests += r.requests;
            s.input_tokens += r.input_tokens;
            s.output_tokens += r.output_tokens;
            s.cost += r.cost;
        }
        Ok(s)
    }

    /// Per-provider totals across the last `days` days, sorted by requests desc.
    pub fn by_provider(&self, days: i32) -> AppResult<Vec<ProviderUsage>> {
        let rollups = self.list_recent(days)?;
        let mut map: std::collections::HashMap<String, ProviderUsage> =
            std::collections::HashMap::new();
        for r in rollups {
            let entry = map.entry(r.provider_id.clone()).or_insert(ProviderUsage {
                provider_id: r.provider_id.clone(),
                requests: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost: 0.0,
            });
            entry.requests += r.requests;
            entry.input_tokens += r.input_tokens;
            entry.output_tokens += r.output_tokens;
            entry.cost += r.cost;
        }
        let mut out: Vec<ProviderUsage> = map.into_values().collect();
        out.sort_by(|a, b| b.requests.cmp(&a.requests));
        Ok(out)
    }

    /// Record usage for a provider against today's date (accumulates).
    pub fn record(
        &self,
        provider_id: &str,
        requests: i64,
        input_tokens: i64,
        output_tokens: i64,
        cost: f64,
    ) -> AppResult<()> {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let rollup = UsageRollup {
            provider_id: provider_id.to_string(),
            date,
            requests,
            input_tokens,
            output_tokens,
            cost,
        };
        self.with_conn(|dao| dao.upsert_rollup(&rollup))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_then_summary_and_by_provider_roundtrip() {
        let dir = std::env::temp_dir().join(format!("olenro-usage-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("usage.db");
        let _ = std::fs::remove_file(&db);
        let svc = UsageService::new(db.clone());

        // Two records for provider A (accumulate same day), one for B.
        svc.record("prov-a", 3, 100, 50, 1.5).unwrap();
        svc.record("prov-a", 2, 40, 20, 0.5).unwrap();
        svc.record("prov-b", 1, 10, 5, 0.25).unwrap();

        let s = svc.summary(30).unwrap();
        assert_eq!(s.requests, 6);
        assert_eq!(s.input_tokens, 150);
        assert_eq!(s.output_tokens, 75);
        assert!((s.cost - 2.25).abs() < 1e-9);

        let by = svc.by_provider(30).unwrap();
        assert_eq!(by.len(), 2);
        // Sorted by requests desc => prov-a (5) first.
        assert_eq!(by[0].provider_id, "prov-a");
        assert_eq!(by[0].requests, 5);
        assert_eq!(by[1].provider_id, "prov-b");
        assert_eq!(by[1].requests, 1);

        let _ = std::fs::remove_file(&db);
    }
}
