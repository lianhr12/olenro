//! Settings DAO
use crate::error::AppResult;
use rusqlite::Connection;

pub struct SettingsDao<'a> {
    conn: &'a Connection,
}

impl<'a> SettingsDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn get(&self, key: &str) -> AppResult<Option<String>> {
        Ok(None)
    }

    pub fn set(&self, key: &str, value: &str) -> AppResult<()> {
        Ok(())
    }

    pub fn delete(&self, key: &str) -> AppResult<()> {
        Ok(())
    }
}
