//! Usage DAO
use crate::error::AppResult;
use rusqlite::Connection;

pub struct UsageDao<'a> {
    conn: &'a Connection,
}

impl<'a> UsageDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn get_rollup(&self, provider_id: &str, date: &str) -> AppResult<Option<UsageRollup>> {
        Ok(None)
    }

    pub fn upsert_rollup(&self, rollup: &UsageRollup) -> AppResult<()> {
        Ok(())
    }

    pub fn list_recent(&self, days: i32) -> AppResult<Vec<UsageRollup>> {
        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub struct UsageRollup {
    pub provider_id: String,
    pub date: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}
