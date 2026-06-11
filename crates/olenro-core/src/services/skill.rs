//! Skill service
//!
//! Business logic for skill management

use crate::app_config::{InstalledSkill, SkillSource};
use crate::database::dao::SkillsDao;
use crate::error::{AppError, AppResult};
use crate::provider::AppType;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = rusqlite::Connection::open(&self.db_path).map_err(|e| AppError::Database(e))?;

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
        let skill_path = std::env::temp_dir().join("olenro-skills").join(&skill_id);

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
                    log::debug!("Updating from zip not yet implemented");
                }
                SkillSource::Local { path } => {
                    log::debug!("Local skill {} doesn't need updating", path);
                }
            }
        }
        Ok(())
    }

    /// 某个 app 的技能目录（`~/.<app>/skills/`）。
    pub fn app_skills_dir(app: &AppType) -> PathBuf {
        crate::config::get_home_dir()
            .join(app.config_dir_name())
            .join("skills")
    }

    /// 把全部已安装技能同步（复制）到指定 app 的技能目录，返回同步成功的数量。
    ///
    /// 每个技能复制到 `~/.<app>/skills/<dir>/`，其中 `<dir>` 取技能源目录的
    /// basename（回退到技能 id）。已存在的目标目录会被覆盖。
    pub fn sync_to_app(&self, app: &AppType) -> AppResult<usize> {
        let skills = self.list_skills()?;
        let dest_root = Self::app_skills_dir(app);
        std::fs::create_dir_all(&dest_root)?;

        let mut synced = 0;
        for skill in &skills {
            let source = PathBuf::from(&skill.path);
            if !source.is_dir() {
                log::warn!("Skill source missing, skipping: {}", skill.path);
                continue;
            }
            let dir_name = source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| skill.id.clone());
            let dest = dest_root.join(&dir_name);
            if dest.exists() {
                let _ = std::fs::remove_dir_all(&dest);
            }
            copy_dir_recursive(&source, &dest)?;
            synced += 1;
        }
        Ok(synced)
    }
}

/// 递归复制目录 `src` 到 `dst`。
fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn sync_to_app_copies_skill_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let svc = SkillService::new(dir.path().join("test.db"));
        let skill = svc.install_from_github("owner/repo").unwrap();
        // Put a file into the skill's source directory.
        let src = PathBuf::from(&skill.path);
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("SKILL.md"), b"hello").unwrap();
        let dir_name = src.file_name().unwrap().to_string_lossy().to_string();

        let home = tempfile::tempdir().unwrap();
        let old = std::env::var_os("OLENRO_TEST_HOME");
        std::env::set_var("OLENRO_TEST_HOME", home.path());

        let n = svc.sync_to_app(&AppType::Claude).unwrap();
        assert_eq!(n, 1);
        let dest = SkillService::app_skills_dir(&AppType::Claude)
            .join(&dir_name)
            .join("SKILL.md");
        assert!(dest.exists(), "synced skill file should exist at {dest:?}");
        assert_eq!(std::fs::read(&dest).unwrap(), b"hello");

        match old {
            Some(v) => std::env::set_var("OLENRO_TEST_HOME", v),
            None => std::env::remove_var("OLENRO_TEST_HOME"),
        }
    }
}
