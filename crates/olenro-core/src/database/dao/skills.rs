//! Skills DAO

use crate::app_config::{InstalledSkill, SkillSource};
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, Row};

pub struct SkillsDao<'a> {
    conn: &'a Connection,
}

impl<'a> SkillsDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all installed skills
    pub fn list_all(&self) -> AppResult<Vec<InstalledSkill>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, version, path, source_type, source_data FROM skills")?;

        let skills = stmt
            .query_map([], |row| self.row_to_skill(row))
            .map_err(|e| AppError::Database(e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(skills)
    }

    /// Get skill by ID
    pub fn get_by_id(&self, id: &str) -> AppResult<Option<InstalledSkill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, version, path, source_type, source_data FROM skills WHERE id = ?",
        )?;

        let mut rows = stmt.query(params![id]).map_err(|e| AppError::Database(e))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e))? {
            Ok(Some(self.row_to_skill(row)?))
        } else {
            Ok(None)
        }
    }

    /// Insert a new skill
    pub fn insert(&self, skill: &InstalledSkill) -> AppResult<()> {
        let source_json = serde_json::to_string(&skill.source).unwrap_or_default();

        self.conn
            .execute(
                "INSERT INTO skills (id, name, version, path, source_type, source_data)
             VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    skill.id,
                    skill.name,
                    skill.version,
                    skill.path,
                    source_json.split(':').next().unwrap_or("unknown"),
                    source_json,
                ],
            )
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Delete a skill
    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM skills WHERE id = ?", params![id])
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    fn row_to_skill(&self, row: &Row) -> std::result::Result<InstalledSkill, rusqlite::Error> {
        let source_str: String = row.get(5).unwrap_or_default();
        let source: SkillSource = serde_json::from_str(&source_str).unwrap_or(SkillSource::Local {
            path: row.get(3).unwrap_or_default(),
        });

        Ok(InstalledSkill {
            id: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            version: row.get(2).unwrap_or_default(),
            path: row.get(3).unwrap_or_default(),
            source,
            enabled_apps: vec![],
        })
    }
}
