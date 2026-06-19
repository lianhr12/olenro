//! Skills DAO（v3.10.0+ 统一管理结构）

use crate::app_config::{InstalledSkill, SkillApps};
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, Row};

pub struct SkillsDao<'a> {
    conn: &'a Connection,
}

impl<'a> SkillsDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    const COLUMNS: &'static str = "id, name, description, directory, repo_owner, repo_name, \
         repo_branch, readme_url, apps, installed_at, content_hash, updated_at";

    /// 列出所有已安装 Skill。
    pub fn list_all(&self) -> AppResult<Vec<InstalledSkill>> {
        let sql = format!("SELECT {} FROM skills", Self::COLUMNS);
        let mut stmt = self.conn.prepare(&sql)?;
        let skills = stmt
            .query_map([], Self::row_to_skill)
            .map_err(AppError::Database)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(skills)
    }

    /// 按 ID 获取 Skill。
    pub fn get_by_id(&self, id: &str) -> AppResult<Option<InstalledSkill>> {
        let sql = format!("SELECT {} FROM skills WHERE id = ?", Self::COLUMNS);
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![id]).map_err(AppError::Database)?;
        if let Some(row) = rows.next().map_err(AppError::Database)? {
            Ok(Some(Self::row_to_skill(row)?))
        } else {
            Ok(None)
        }
    }

    /// 插入或更新 Skill（按 id upsert）。
    pub fn save(&self, skill: &InstalledSkill) -> AppResult<()> {
        let apps_json = serde_json::to_string(&skill.apps).unwrap_or_else(|_| "{}".to_string());
        self.conn
            .execute(
                "INSERT INTO skills
                 (id, name, description, directory, repo_owner, repo_name, repo_branch,
                  readme_url, apps, installed_at, content_hash, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     description = excluded.description,
                     directory = excluded.directory,
                     repo_owner = excluded.repo_owner,
                     repo_name = excluded.repo_name,
                     repo_branch = excluded.repo_branch,
                     readme_url = excluded.readme_url,
                     apps = excluded.apps,
                     installed_at = excluded.installed_at,
                     content_hash = excluded.content_hash,
                     updated_at = excluded.updated_at",
                params![
                    skill.id,
                    skill.name,
                    skill.description,
                    skill.directory,
                    skill.repo_owner,
                    skill.repo_name,
                    skill.repo_branch,
                    skill.readme_url,
                    apps_json,
                    skill.installed_at,
                    skill.content_hash,
                    skill.updated_at,
                ],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 删除 Skill。
    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM skills WHERE id = ?", params![id])
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 仅更新启用状态（apps 列）。
    pub fn update_apps(&self, id: &str, apps: &SkillApps) -> AppResult<()> {
        let apps_json = serde_json::to_string(apps).unwrap_or_else(|_| "{}".to_string());
        self.conn
            .execute(
                "UPDATE skills SET apps = ?1 WHERE id = ?2",
                params![apps_json, id],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 仅更新内容哈希与更新时间。
    pub fn update_hash(&self, id: &str, hash: &str, updated_at: i64) -> AppResult<()> {
        self.conn
            .execute(
                "UPDATE skills SET content_hash = ?1, updated_at = ?2 WHERE id = ?3",
                params![hash, updated_at, id],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    // ========== 仓库（skill_repos 表） ==========

    /// 列出所有技能仓库 (owner, name, branch, enabled)。
    pub fn list_repos(&self) -> AppResult<Vec<(String, String, String, bool)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT owner, name, branch, enabled FROM skill_repos")?;
        let repos = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)? != 0,
                ))
            })
            .map_err(AppError::Database)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(repos)
    }

    /// 插入或更新仓库（按 owner+name upsert）。
    pub fn save_repo(&self, owner: &str, name: &str, branch: &str, enabled: bool) -> AppResult<()> {
        self.conn
            .execute(
                "INSERT INTO skill_repos (owner, name, branch, enabled) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(owner, name) DO UPDATE SET branch = excluded.branch, enabled = excluded.enabled",
                params![owner, name, branch, enabled as i64],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 读取内部元数据（skill_meta 表）。
    pub fn get_meta(&self, key: &str) -> AppResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM skill_meta WHERE key = ?")?;
        let mut rows = stmt.query(params![key]).map_err(AppError::Database)?;
        if let Some(row) = rows.next().map_err(AppError::Database)? {
            Ok(Some(row.get(0).map_err(AppError::Database)?))
        } else {
            Ok(None)
        }
    }

    /// 写入内部元数据（skill_meta 表）。
    pub fn set_meta(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn
            .execute(
                "INSERT INTO skill_meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 删除仓库。
    pub fn delete_repo(&self, owner: &str, name: &str) -> AppResult<()> {
        self.conn
            .execute(
                "DELETE FROM skill_repos WHERE owner = ? AND name = ?",
                params![owner, name],
            )
            .map_err(AppError::Database)?;
        Ok(())
    }

    fn row_to_skill(row: &Row) -> std::result::Result<InstalledSkill, rusqlite::Error> {
        let apps_str: String = row.get::<_, Option<String>>(8)?.unwrap_or_default();
        let apps: SkillApps = serde_json::from_str(&apps_str).unwrap_or_default();

        Ok(InstalledSkill {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            directory: row.get(3)?,
            repo_owner: row.get(4)?,
            repo_name: row.get(5)?,
            repo_branch: row.get(6)?,
            readme_url: row.get(7)?,
            apps,
            installed_at: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
            content_hash: row.get(10)?,
            updated_at: row.get::<_, Option<i64>>(11)?.unwrap_or(0),
        })
    }
}
