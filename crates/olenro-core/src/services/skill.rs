//! Skill service
//!
//! Business logic for skill management

use crate::app_config::{InstalledSkill, SkillSource};
use crate::database::dao::SkillsDao;
use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill service for managing skills
pub struct SkillService {
    db_path: PathBuf,
}

impl SkillService {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&SkillsDao) -> AppResult<T>,
    {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| AppError::Database(e))?;

        // Initialize schema if needed
        let schema = r#"
            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                path TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_data TEXT NOT NULL,
                created_at INTEGER DEFAULT 0,
                updated_at INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS skill_app_mapping (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_type TEXT NOT NULL,
                skill_id TEXT NOT NULL,
                UNIQUE(app_type, skill_id)
            );
        "#;
        conn.execute_batch(schema)
            .map_err(|e| AppError::Database(e))?;

        let dao = SkillsDao::new(&conn);
        f(&dao)
    }

    /// List all installed skills
    pub fn list_skills(&self) -> AppResult<Vec<InstalledSkill>> {
        self.with_conn(|dao| dao.list_all())
    }

    /// Get skill by ID
    pub fn get_skill(&self, id: &str) -> AppResult<Option<InstalledSkill>> {
        self.with_conn(|dao| dao.get_by_id(id))
    }

    /// Install a skill from GitHub
    pub fn install_from_github(&self, repo: &str) -> AppResult<InstalledSkill> {
        let now = chrono::Utc::now().timestamp();
        let skill_id = uuid::Uuid::new_v4().to_string();

        // Create skill directory
        let skill_path = std::env::temp_dir()
            .join("olenro-skills")
            .join(&skill_id);

        std::fs::create_dir_all(&skill_path).map_err(|e| AppError::Io(e))?;

        let skill = InstalledSkill {
            id: skill_id,
            name: repo.to_string(),
            version: "0.1.0".to_string(),
            path: skill_path.to_string_lossy().to_string(),
            source: SkillSource::GitHub {
                repo: repo.to_string(),
            },
            enabled_apps: vec![],
        };

        self.with_conn(|dao| dao.insert(&skill))?;
        Ok(skill)
    }

    /// Uninstall a skill
    pub fn uninstall(&self, id: &str) -> AppResult<()> {
        // Get skill first to clean up files
        if let Some(skill) = self.get_skill(id)? {
            // Remove skill directory if it exists
            let path = PathBuf::from(&skill.path);
            if path.exists() && path.starts_with(std::env::temp_dir()) {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
        self.with_conn(|dao| dao.delete(id))
    }

    /// Update a skill (re-fetch from source)
    pub fn update(&self, id: &str) -> AppResult<()> {
        // For now, just re-download from GitHub
        if let Some(skill) = self.get_skill(id)? {
            match &skill.source {
                SkillSource::GitHub { repo } => {
                    // Re-install from GitHub
                    self.uninstall(id)?;
                    let _ = self.install_from_github(repo);
                }
                SkillSource::Zip { url } => {
                    eprintln!("Updating from zip not yet implemented");
                }
                SkillSource::Local { path } => {
                    eprintln!("Local skill {} doesn't need updating", path);
                }
            }
        }
        Ok(())
    }
}