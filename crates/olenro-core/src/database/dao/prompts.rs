//! Prompts DAO

use crate::error::{AppError, AppResult};
use crate::prompt::Prompt;
use rusqlite::{params, Connection, Row};

pub struct PromptsDao<'a> {
    conn: &'a Connection,
}

impl<'a> PromptsDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all prompts
    pub fn list_all(&self) -> AppResult<Vec<Prompt>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, content, description, enabled, created_at, updated_at FROM prompts ORDER BY created_at DESC"
        )?;

        let prompts = stmt
            .query_map([], |row| self.row_to_prompt(row))
            .map_err(|e| AppError::Database(e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(prompts)
    }

    /// Get prompt by ID
    pub fn get_by_id(&self, id: &str) -> AppResult<Option<Prompt>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, content, description, enabled, created_at, updated_at FROM prompts WHERE id = ?"
        )?;

        let mut rows = stmt.query(params![id]).map_err(|e| AppError::Database(e))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e))? {
            Ok(Some(self.row_to_prompt(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get enabled prompt
    pub fn get_enabled(&self, app_type: &str) -> AppResult<Option<Prompt>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, content, description, enabled, created_at, updated_at FROM prompts WHERE enabled = 1 AND app_type = ? ORDER BY updated_at DESC LIMIT 1"
        )?;

        let mut rows = stmt
            .query(params![app_type])
            .map_err(|e| AppError::Database(e))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e))? {
            Ok(Some(self.row_to_prompt(row)?))
        } else {
            Ok(None)
        }
    }

    /// Insert a new prompt
    pub fn insert(&self, prompt: &Prompt) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO prompts (id, name, content, description, enabled, created_at, updated_at, app_type)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                prompt.id,
                prompt.name,
                prompt.content,
                prompt.description,
                prompt.enabled as i32,
                prompt.created_at,
                prompt.updated_at,
                "", // app_type stored separately for now
            ],
        ).map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Update an existing prompt
    pub fn update(&self, prompt: &Prompt) -> AppResult<()> {
        self.conn.execute(
            "UPDATE prompts SET name = ?, content = ?, description = ?, enabled = ?, updated_at = ? WHERE id = ?",
            params![
                prompt.name,
                prompt.content,
                prompt.description,
                prompt.enabled as i32,
                prompt.updated_at,
                prompt.id,
            ],
        ).map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Delete a prompt
    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM prompts WHERE id = ?", params![id])
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Disable all prompts for an app (when enabling a new one)
    pub fn disable_all(&self) -> AppResult<()> {
        self.conn
            .execute("UPDATE prompts SET enabled = 0", [])
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    fn row_to_prompt(&self, row: &Row) -> std::result::Result<Prompt, rusqlite::Error> {
        Ok(Prompt {
            id: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            content: row.get(2).unwrap_or_default(),
            description: row.get(3).ok(),
            enabled: row.get::<_, i32>(4).unwrap_or(0) != 0,
            created_at: row.get(5).unwrap_or(0),
            updated_at: row.get(6).unwrap_or(0),
        })
    }
}
