//! Usage DAO
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

pub struct UsageDao<'a> {
    conn: &'a Connection,
}

impl<'a> UsageDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Fetch a single provider/date rollup, if present.
    pub fn get_rollup(&self, provider_id: &str, date: &str) -> AppResult<Option<UsageRollup>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT provider_id, date, requests, input_tokens, output_tokens, cost
                 FROM usage_rollup WHERE provider_id = ?1 AND date = ?2",
            )
            .map_err(AppError::Database)?;

        let mut rows = stmt
            .query_map(params![provider_id, date], Self::map_row)
            .map_err(AppError::Database)?;

        match rows.next() {
            Some(row) => Ok(Some(row.map_err(AppError::Database)?)),
            None => Ok(None),
        }
    }

    /// Insert or accumulate a rollup for the given provider/date.
    pub fn upsert_rollup(&self, rollup: &UsageRollup) -> AppResult<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn
            .execute(
                "INSERT INTO usage_rollup
                    (provider_id, date, requests, input_tokens, output_tokens, cost, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                 ON CONFLICT(provider_id, date) DO UPDATE SET
                    requests = requests + excluded.requests,
                    input_tokens = input_tokens + excluded.input_tokens,
                    output_tokens = output_tokens + excluded.output_tokens,
                    cost = cost + excluded.cost,
                    updated_at = excluded.updated_at",
                params![
                    rollup.provider_id,
                    rollup.date,
                    rollup.requests,
                    rollup.input_tokens,
                    rollup.output_tokens,
                    rollup.cost,
                    now,
                ],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// List rollups from the last `days` days, most recent first.
    pub fn list_recent(&self, days: i32) -> AppResult<Vec<UsageRollup>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days.max(0) as i64))
            .format("%Y-%m-%d")
            .to_string();

        let mut stmt = self
            .conn
            .prepare(
                "SELECT provider_id, date, requests, input_tokens, output_tokens, cost
                 FROM usage_rollup WHERE date >= ?1 ORDER BY date DESC, provider_id ASC",
            )
            .map_err(AppError::Database)?;

        let rows = stmt
            .query_map(params![cutoff], Self::map_row)
            .map_err(AppError::Database)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(AppError::Database)?);
        }
        Ok(out)
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<UsageRollup> {
        Ok(UsageRollup {
            provider_id: row.get(0)?,
            date: row.get(1)?,
            requests: row.get(2)?,
            input_tokens: row.get(3)?,
            output_tokens: row.get(4)?,
            cost: row.get(5)?,
        })
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
