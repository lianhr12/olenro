//! Settings DAO
//!
//! 键值对设置存储（`settings(key, value)` 表）。

use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};

pub struct SettingsDao<'a> {
    conn: &'a Connection,
}

impl<'a> SettingsDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 读取设置值，不存在时返回 None。
    pub fn get(&self, key: &str) -> AppResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key]).map_err(AppError::Database)?;
        if let Some(row) = rows.next().map_err(AppError::Database)? {
            Ok(Some(row.get(0).map_err(AppError::Database)?))
        } else {
            Ok(None)
        }
    }

    /// 写入设置值（upsert）。
    pub fn set(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?, ?)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 删除设置项。
    pub fn delete(&self, key: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?", params![key])
            .map_err(AppError::Database)?;
        Ok(())
    }
}
