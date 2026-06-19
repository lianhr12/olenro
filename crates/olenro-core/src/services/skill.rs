//! Skills 服务层（由桌面端 `src-tauri/src/services/skill.rs` 移植）
//!
//! v3.10.0+ 统一管理架构：
//! - SSOT（单一事实源）：`~/.olenro/skills/`（或 `~/.agents/skills/`）
//! - 安装时下载到 SSOT，按需同步（symlink/copy）到各应用目录
//! - 数据库存储安装记录和启用状态
//!
//! 与桌面端的差异：
//! - 不持有 `Arc<Database>`，改为 `db_path` + `with_conn`（与 core 其它 service 一致）
//! - 错误类型用 [`AppError`] 而非 anyhow
//! - 下载直连 reqwest（暂不接入代理）
//! - 路径使用 [`crate::config::get_home_dir`]（尊重 `OLENRO_TEST_HOME`）

use crate::app_config::{AppType, InstalledSkill, SkillApps, UnmanagedSkill};
use crate::config::get_app_config_dir;
use crate::database::dao::SkillsDao;
use crate::error::{format_skill_error, AppError, AppResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::timeout;

pub use crate::settings::{SkillStorageLocation, SyncMethod};

// ========== 数据结构 ==========

/// 可发现的技能（来自仓库扫描）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverableSkill {
    /// 唯一标识: "owner/name:directory"
    pub key: String,
    pub name: String,
    pub description: String,
    /// 目录名称（安装路径的最后一段或相对路径）
    pub directory: String,
    pub readme_url: Option<String>,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
}

/// 仓库配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRepo {
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub enabled: bool,
}

/// Skill 卸载结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillUninstallResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
}

/// Skill 更新检测结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInfo {
    pub id: String,
    pub name: String,
    pub current_hash: Option<String>,
    pub remote_hash: String,
}

/// 技能元数据（从 SKILL.md front-matter 解析）。
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Skill 存储位置迁移结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResult {
    pub migrated_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
}

/// 导入已有 Skill 时，前端显式提交的启用应用选择。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSkillSelection {
    pub directory: String,
    #[serde(default)]
    pub apps: SkillApps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBackupEntry {
    pub backup_id: String,
    pub backup_path: String,
    pub created_at: i64,
    pub skill: InstalledSkill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillBackupMetadata {
    skill: InstalledSkill,
    backup_created_at: i64,
    source_path: String,
}

const SKILL_BACKUP_RETAIN_COUNT: usize = 20;

/// 默认技能仓库列表。
pub fn default_repos() -> Vec<SkillRepo> {
    vec![
        SkillRepo {
            owner: "anthropics".to_string(),
            name: "skills".to_string(),
            branch: "main".to_string(),
            enabled: true,
        },
        SkillRepo {
            owner: "ComposioHQ".to_string(),
            name: "awesome-claude-skills".to_string(),
            branch: "master".to_string(),
            enabled: true,
        },
        SkillRepo {
            owner: "cexll".to_string(),
            name: "myclaude".to_string(),
            branch: "master".to_string(),
            enabled: true,
        },
        SkillRepo {
            owner: "JimLiu".to_string(),
            name: "baoyu-skills".to_string(),
            branch: "main".to_string(),
            enabled: true,
        },
    ]
}

// ========== SkillService ==========

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
        let conn = rusqlite::Connection::open(&self.db_path).map_err(AppError::Database)?;
        let schema = r#"
            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                directory TEXT NOT NULL,
                repo_owner TEXT,
                repo_name TEXT,
                repo_branch TEXT,
                readme_url TEXT,
                apps TEXT NOT NULL DEFAULT '{}',
                installed_at INTEGER NOT NULL DEFAULT 0,
                content_hash TEXT,
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS skill_repos (
                owner TEXT NOT NULL,
                name TEXT NOT NULL,
                branch TEXT NOT NULL DEFAULT 'main',
                enabled INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (owner, name)
            );
            CREATE TABLE IF NOT EXISTS skill_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        "#;
        conn.execute_batch(schema).map_err(AppError::Database)?;
        let dao = SkillsDao::new(&conn);
        f(&dao)
    }

    /// 全部已安装 Skill，按 id 索引。
    fn all_skills_map(&self) -> AppResult<HashMap<String, InstalledSkill>> {
        let skills = self.with_conn(|dao| dao.list_all())?;
        Ok(skills.into_iter().map(|s| (s.id.clone(), s)).collect())
    }

    fn http_client() -> AppResult<reqwest::Client> {
        reqwest::Client::builder()
            .build()
            .map_err(|e| AppError::Network(format!("failed to build HTTP client: {e}")))
    }

    // ========== URL / doc 路径 ==========

    fn build_skill_doc_url(owner: &str, repo: &str, branch: &str, doc_path: &str) -> String {
        format!("https://github.com/{owner}/{repo}/blob/{branch}/{doc_path}")
    }

    fn extract_doc_path_from_url(url: &str) -> Option<String> {
        let marker = if url.contains("/blob/") {
            "/blob/"
        } else if url.contains("/tree/") {
            "/tree/"
        } else {
            return None;
        };
        let (_, tail) = url.split_once(marker)?;
        let (_, path) = tail.split_once('/')?;
        if path.is_empty() {
            None
        } else {
            Some(path.to_string())
        }
    }

    // ========== 路径管理 ==========

    /// SSOT 目录（根据设置返回 `~/.olenro/skills/` 或 `~/.agents/skills/`）。
    pub fn get_ssot_dir() -> AppResult<PathBuf> {
        let location = crate::settings::get_skill_storage_location();
        let dir = match location {
            SkillStorageLocation::CcSwitch => get_app_config_dir().join("skills"),
            SkillStorageLocation::Unified => {
                crate::config::get_home_dir().join(".agents").join("skills")
            }
        };
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Skill 卸载备份目录（`~/.olenro/skill-backups/`）。
    fn get_backup_dir() -> AppResult<PathBuf> {
        let dir = get_app_config_dir().join("skill-backups");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// 应用的 skills 目录（优先 settings 中的 override）。
    pub fn get_app_skills_dir(app: &AppType) -> AppResult<PathBuf> {
        match app {
            AppType::Claude => {
                if let Some(custom) = crate::settings::get_claude_override_dir() {
                    return Ok(custom.join("skills"));
                }
            }
            AppType::ClaudeDesktop => {}
            AppType::Codex => {
                if let Some(custom) = crate::settings::get_codex_override_dir() {
                    return Ok(custom.join("skills"));
                }
            }
            AppType::Gemini => {
                if let Some(custom) = crate::settings::get_gemini_override_dir() {
                    return Ok(custom.join("skills"));
                }
            }
            AppType::OpenCode => {
                if let Some(custom) = crate::settings::get_opencode_override_dir() {
                    return Ok(custom.join("skills"));
                }
            }
            AppType::OpenClaw => {
                if let Some(custom) = crate::settings::get_openclaw_override_dir() {
                    return Ok(custom.join("skills"));
                }
            }
            AppType::Hermes => {
                if let Some(custom) = crate::settings::get_hermes_override_dir() {
                    return Ok(custom.join("skills"));
                }
            }
        }

        let home = crate::config::get_home_dir();
        Ok(match app {
            AppType::Claude => home.join(".claude").join("skills"),
            AppType::ClaudeDesktop => home.join(".claude-desktop").join("skills"),
            AppType::Codex => home.join(".codex").join("skills"),
            AppType::Gemini => home.join(".gemini").join("skills"),
            AppType::OpenCode => home.join(".config").join("opencode").join("skills"),
            AppType::OpenClaw => home.join(".openclaw").join("skills"),
            AppType::Hermes => {
                crate::app_config_writers::hermes_config::get_hermes_dir().join("skills")
            }
        })
    }

    fn get_agents_skills_dir() -> Option<PathBuf> {
        let p = crate::config::get_home_dir().join(".agents").join("skills");
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    // ========== 查询 ==========

    /// 全部已安装 Skill。
    pub fn get_all_installed(&self) -> AppResult<Vec<InstalledSkill>> {
        self.with_conn(|dao| dao.list_all())
    }

    /// 别名（CLI list）。
    pub fn list_skills(&self) -> AppResult<Vec<InstalledSkill>> {
        self.get_all_installed()
    }

    pub fn get_skill(&self, id: &str) -> AppResult<Option<InstalledSkill>> {
        self.with_conn(|dao| dao.get_by_id(id))
    }

    // ========== 安装 ==========

    /// 安装一个可发现的技能。
    pub async fn install(
        &self,
        skill: &DiscoverableSkill,
        current_app: &AppType,
    ) -> AppResult<InstalledSkill> {
        let ssot_dir = Self::get_ssot_dir()?;

        let source_rel = Self::sanitize_skill_source_path(&skill.directory).ok_or_else(|| {
            AppError::Skill(format_skill_error(
                "INVALID_SKILL_DIRECTORY",
                &[("directory", &skill.directory)],
                Some("checkZipContent"),
            ))
        })?;
        let install_name = source_rel
            .file_name()
            .and_then(|name| Self::sanitize_install_name(&name.to_string_lossy()))
            .ok_or_else(|| {
                AppError::Skill(format_skill_error(
                    "INVALID_SKILL_DIRECTORY",
                    &[("directory", &skill.directory)],
                    Some("checkZipContent"),
                ))
            })?;

        // 查重 / 冲突
        let existing_skills = self.all_skills_map()?;
        for existing in existing_skills.values() {
            if existing.directory.eq_ignore_ascii_case(&install_name) {
                let same_repo = existing.repo_owner.as_deref() == Some(&skill.repo_owner)
                    && existing.repo_name.as_deref() == Some(&skill.repo_name);
                if same_repo {
                    let mut updated = existing.clone();
                    updated.apps.set_enabled_for(current_app, true);
                    self.with_conn(|dao| dao.save(&updated))?;
                    Self::sync_to_app_dir(&updated.directory, current_app)?;
                    log::info!(
                        "Skill {} 已存在，更新 {:?} 启用状态",
                        updated.name,
                        current_app
                    );
                    return Ok(updated);
                } else {
                    return Err(AppError::Skill(format_skill_error(
                        "SKILL_DIRECTORY_CONFLICT",
                        &[
                            ("directory", &install_name),
                            (
                                "existing_repo",
                                &format!(
                                    "{}/{}",
                                    existing.repo_owner.as_deref().unwrap_or("unknown"),
                                    existing.repo_name.as_deref().unwrap_or("unknown")
                                ),
                            ),
                            (
                                "new_repo",
                                &format!("{}/{}", skill.repo_owner, skill.repo_name),
                            ),
                        ],
                        Some("uninstallFirst"),
                    )));
                }
            }
        }

        let dest = ssot_dir.join(&install_name);
        let mut repo_branch = skill.repo_branch.clone();

        if !dest.exists() {
            let repo = SkillRepo {
                owner: skill.repo_owner.clone(),
                name: skill.repo_name.clone(),
                branch: skill.repo_branch.clone(),
                enabled: true,
            };

            let (temp_dir, used_branch) = timeout(
                std::time::Duration::from_secs(60),
                self.download_repo(&repo),
            )
            .await
            .map_err(|_| {
                AppError::Skill(format_skill_error(
                    "DOWNLOAD_TIMEOUT",
                    &[
                        ("owner", &repo.owner),
                        ("name", &repo.name),
                        ("timeout", "60"),
                    ],
                    Some("checkNetwork"),
                ))
            })??;
            repo_branch = used_branch;

            let source =
                Self::resolve_skill_source_dir(&temp_dir, &skill.directory).ok_or_else(|| {
                    let missing = temp_dir.join(&source_rel).display().to_string();
                    let _ = fs::remove_dir_all(&temp_dir);
                    AppError::Skill(format_skill_error(
                        "SKILL_DIR_NOT_FOUND",
                        &[("path", &missing)],
                        Some("checkRepoUrl"),
                    ))
                })?;

            let canonical_temp = temp_dir.canonicalize().unwrap_or_else(|_| temp_dir.clone());
            let canonical_source = source.canonicalize().map_err(|_| {
                AppError::Skill(format_skill_error(
                    "SKILL_DIR_NOT_FOUND",
                    &[("path", &source.display().to_string())],
                    Some("checkRepoUrl"),
                ))
            })?;
            if !canonical_source.starts_with(&canonical_temp) || !canonical_source.is_dir() {
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(AppError::Skill(format_skill_error(
                    "INVALID_SKILL_DIRECTORY",
                    &[("directory", &skill.directory)],
                    Some("checkZipContent"),
                )));
            }

            Self::copy_dir_recursive(&canonical_source, &dest)?;
            let _ = fs::remove_dir_all(&temp_dir);

            if repo_branch != skill.repo_branch {
                log::info!(
                    "Skill {}/{} 分支自动回退: {} -> {}",
                    skill.repo_owner,
                    skill.repo_name,
                    skill.repo_branch,
                    repo_branch
                );
            }
        }

        let doc_path = skill
            .readme_url
            .as_deref()
            .and_then(Self::extract_doc_path_from_url)
            .map(|path| {
                if path.ends_with("/SKILL.md") || path == "SKILL.md" {
                    path
                } else {
                    format!("{}/SKILL.md", path.trim_end_matches('/'))
                }
            })
            .unwrap_or_else(|| format!("{}/SKILL.md", skill.directory.trim_end_matches('/')));

        let readme_url = Some(Self::build_skill_doc_url(
            &skill.repo_owner,
            &skill.repo_name,
            &repo_branch,
            &doc_path,
        ));

        let content_hash = Self::compute_dir_hash(&dest).map(Some).unwrap_or_else(|e| {
            log::warn!("Failed to compute content hash for {}: {e}", install_name);
            None
        });

        let installed_skill = InstalledSkill {
            id: skill.key.clone(),
            name: skill.name.clone(),
            description: if skill.description.is_empty() {
                None
            } else {
                Some(skill.description.clone())
            },
            directory: install_name.clone(),
            repo_owner: Some(skill.repo_owner.clone()),
            repo_name: Some(skill.repo_name.clone()),
            repo_branch: Some(repo_branch),
            readme_url,
            apps: SkillApps::only(current_app),
            installed_at: Utc::now().timestamp(),
            content_hash,
            updated_at: 0,
        };

        self.with_conn(|dao| dao.save(&installed_skill))?;
        Self::sync_to_app_dir(&install_name, current_app)?;

        log::info!(
            "Skill {} 安装成功，已启用 {:?}",
            installed_skill.name,
            current_app
        );
        Ok(installed_skill)
    }

    /// CLI 便捷封装：从 `owner/repo` 或 `owner/repo:directory` 安装。
    ///
    /// 扫描仓库内全部技能；若给定 `:directory` 则仅安装匹配项，否则安装全部。
    pub async fn install_from_github(
        &self,
        slug: &str,
        current_app: &AppType,
    ) -> AppResult<Vec<InstalledSkill>> {
        let (repo_part, dir_filter) = match slug.split_once(':') {
            Some((r, d)) => (r, Some(d.to_string())),
            None => (slug, None),
        };
        let (owner, name) = repo_part.split_once('/').ok_or_else(|| {
            AppError::Skill(format_skill_error(
                "INVALID_REPO_SLUG",
                &[("slug", slug)],
                Some("checkRepoUrl"),
            ))
        })?;

        let repo = SkillRepo {
            owner: owner.to_string(),
            name: name.to_string(),
            branch: "main".to_string(),
            enabled: true,
        };

        let discoverable = self.fetch_repo_skills(&repo).await?;
        let mut installed = Vec::new();
        for d in discoverable {
            if let Some(filter) = &dir_filter {
                let install_name = d.directory.rsplit('/').next().unwrap_or(&d.directory);
                if !install_name.eq_ignore_ascii_case(filter)
                    && !d.directory.eq_ignore_ascii_case(filter)
                {
                    continue;
                }
            }
            match self.install(&d, current_app).await {
                Ok(s) => installed.push(s),
                Err(e) => log::warn!("安装 {} 失败: {e}", d.key),
            }
        }

        if installed.is_empty() {
            return Err(AppError::Skill(format_skill_error(
                "SKILL_DIR_NOT_FOUND",
                &[("path", slug)],
                Some("checkRepoUrl"),
            )));
        }
        Ok(installed)
    }

    // ========== 卸载（含备份）==========

    pub fn uninstall(&self, id: &str) -> AppResult<SkillUninstallResult> {
        let skill = self
            .get_skill(id)?
            .ok_or_else(|| AppError::NotFound(format!("Skill not found: {id}")))?;

        let backup_path =
            Self::create_uninstall_backup(&skill)?.map(|p| p.to_string_lossy().to_string());

        for app in AppType::all() {
            let _ = Self::remove_from_app(&skill.directory, &app);
        }

        let ssot_dir = Self::get_ssot_dir()?;
        let skill_path = ssot_dir.join(&skill.directory);
        if skill_path.exists() {
            fs::remove_dir_all(&skill_path)?;
        }

        self.with_conn(|dao| dao.delete(id))?;

        log::info!(
            "Skill {} 卸载成功{}",
            skill.name,
            backup_path
                .as_deref()
                .map(|p| format!(", backup: {p}"))
                .unwrap_or_default()
        );
        Ok(SkillUninstallResult { backup_path })
    }

    // ========== 更新检测 ==========

    pub fn compute_dir_hash(dir: &Path) -> AppResult<String> {
        use sha2::{Digest, Sha256};

        let mut files: Vec<PathBuf> = Vec::new();
        Self::collect_files_for_hash(dir, dir, &mut files)?;
        files.sort();

        let mut hasher = Sha256::new();
        for file_path in &files {
            let relative = file_path.strip_prefix(dir).unwrap_or(file_path);
            let rel_str = relative.to_string_lossy().replace('\\', "/");
            hasher.update(rel_str.as_bytes());
            hasher.update(b"\0");
            let content = fs::read(file_path).map_err(|e| {
                AppError::Skill(format!("读取文件失败 {}: {e}", file_path.display()))
            })?;
            hasher.update(&content);
            hasher.update(b"\0");
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn collect_files_for_hash(
        base: &Path,
        current: &Path,
        files: &mut Vec<PathBuf>,
    ) -> AppResult<()> {
        let entries = fs::read_dir(current)
            .map_err(|e| AppError::Skill(format!("读取目录失败 {}: {e}", current.display())))?;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                Self::collect_files_for_hash(base, &path, files)?;
            } else {
                files.push(path);
            }
        }
        Ok(())
    }

    /// 检查所有已安装 Skill 的更新。
    pub async fn check_updates(&self) -> AppResult<Vec<SkillUpdateInfo>> {
        let skills = self.all_skills_map()?;
        let mut updates = Vec::new();

        let mut repo_groups: HashMap<(String, String, String), Vec<InstalledSkill>> =
            HashMap::new();
        for skill in skills.into_values() {
            let (owner, name, branch) =
                match (&skill.repo_owner, &skill.repo_name, &skill.repo_branch) {
                    (Some(o), Some(n), Some(b)) => (o.clone(), n.clone(), b.clone()),
                    (Some(o), Some(n), None) => (o.clone(), n.clone(), "main".to_string()),
                    _ => continue,
                };
            repo_groups
                .entry((owner, name, branch))
                .or_default()
                .push(skill);
        }

        let ssot_dir = Self::get_ssot_dir()?;

        for ((owner, name, branch), group_skills) in &repo_groups {
            let repo = SkillRepo {
                owner: owner.clone(),
                name: name.clone(),
                branch: branch.clone(),
                enabled: true,
            };

            let (temp_dir, _used_branch) = match timeout(
                std::time::Duration::from_secs(60),
                self.download_repo(&repo),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => {
                    log::warn!("检查更新时下载 {}/{} 失败: {e}", owner, name);
                    continue;
                }
                Err(_) => {
                    log::warn!("检查更新时下载 {}/{} 超时", owner, name);
                    continue;
                }
            };

            let mut remote_skills: Vec<DiscoverableSkill> = Vec::new();
            let _ = self.scan_dir_recursive(&temp_dir, &temp_dir, &repo, &mut remote_skills);

            for skill in group_skills {
                let remote_match = remote_skills.iter().find(|rs| {
                    let remote_install_name =
                        rs.directory.rsplit('/').next().unwrap_or(&rs.directory);
                    remote_install_name.eq_ignore_ascii_case(&skill.directory)
                });
                let remote_skill_dir = match remote_match {
                    Some(rs) => match Self::resolve_skill_source_dir(&temp_dir, &rs.directory) {
                        Some(path) => path,
                        None => continue,
                    },
                    None => continue,
                };

                let remote_hash = match Self::compute_dir_hash(&remote_skill_dir) {
                    Ok(h) => h,
                    Err(e) => {
                        log::warn!("计算远程哈希失败 {}: {e}", skill.id);
                        continue;
                    }
                };

                let local_hash = match &skill.content_hash {
                    Some(h) => Some(h.clone()),
                    None => {
                        let local_dir = ssot_dir.join(&skill.directory);
                        if local_dir.exists() {
                            match Self::compute_dir_hash(&local_dir) {
                                Ok(h) => {
                                    let _ = self.with_conn(|dao| dao.update_hash(&skill.id, &h, 0));
                                    Some(h)
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        }
                    }
                };

                if local_hash.as_deref() != Some(&remote_hash) {
                    updates.push(SkillUpdateInfo {
                        id: skill.id.clone(),
                        name: skill.name.clone(),
                        current_hash: local_hash,
                        remote_hash,
                    });
                }
            }

            let _ = fs::remove_dir_all(&temp_dir);
        }

        Ok(updates)
    }

    /// 更新单个 Skill（重新下载并替换本地文件）。
    pub async fn update_skill(&self, skill_id: &str) -> AppResult<InstalledSkill> {
        let skill = self
            .get_skill(skill_id)?
            .ok_or_else(|| AppError::NotFound(format!("Skill not found: {skill_id}")))?;

        let (owner, name, branch) = match (&skill.repo_owner, &skill.repo_name) {
            (Some(o), Some(n)) => (
                o.clone(),
                n.clone(),
                skill
                    .repo_branch
                    .clone()
                    .unwrap_or_else(|| "main".to_string()),
            ),
            _ => {
                return Err(AppError::InvalidOperation(format!(
                    "Cannot update local skill: {skill_id}"
                )))
            }
        };

        let repo = SkillRepo {
            owner: owner.clone(),
            name: name.clone(),
            branch: branch.clone(),
            enabled: true,
        };

        let ssot_dir = Self::get_ssot_dir()?;

        let (temp_dir, used_branch) = timeout(
            std::time::Duration::from_secs(60),
            self.download_repo(&repo),
        )
        .await
        .map_err(|_| {
            AppError::Skill(format_skill_error(
                "DOWNLOAD_TIMEOUT",
                &[("owner", &owner), ("name", &name), ("timeout", "60")],
                Some("checkNetwork"),
            ))
        })??;

        let mut remote_skills: Vec<DiscoverableSkill> = Vec::new();
        let _ = self.scan_dir_recursive(&temp_dir, &temp_dir, &repo, &mut remote_skills);

        let remote_match = remote_skills
            .iter()
            .find(|rs| {
                let remote_install_name = rs.directory.rsplit('/').next().unwrap_or(&rs.directory);
                remote_install_name.eq_ignore_ascii_case(&skill.directory)
            })
            .ok_or_else(|| {
                let _ = fs::remove_dir_all(&temp_dir);
                AppError::Skill(format_skill_error(
                    "SKILL_DIR_NOT_FOUND",
                    &[("path", &skill.directory)],
                    Some("checkRepoUrl"),
                ))
            })?;

        let source = Self::resolve_skill_source_dir(&temp_dir, &remote_match.directory)
            .ok_or_else(|| {
                let missing = temp_dir.join(&remote_match.directory).display().to_string();
                let _ = fs::remove_dir_all(&temp_dir);
                AppError::Skill(format_skill_error(
                    "SKILL_DIR_NOT_FOUND",
                    &[("path", &missing)],
                    Some("checkRepoUrl"),
                ))
            })?;

        let _ = Self::create_uninstall_backup(&skill);

        let dest = ssot_dir.join(&skill.directory);
        if dest.exists() {
            fs::remove_dir_all(&dest)?;
        }
        Self::copy_dir_recursive(&source, &dest)?;
        let _ = fs::remove_dir_all(&temp_dir);

        let new_hash = Self::compute_dir_hash(&dest).ok();
        let skill_md = dest.join("SKILL.md");
        let (new_name, new_description) = Self::read_skill_name_desc(&skill_md, &skill.directory);

        let doc_path = skill
            .readme_url
            .as_deref()
            .and_then(Self::extract_doc_path_from_url)
            .unwrap_or_else(|| format!("{}/SKILL.md", skill.directory.trim_end_matches('/')));
        let readme_url = Some(Self::build_skill_doc_url(
            &owner,
            &name,
            &used_branch,
            &doc_path,
        ));

        let updated_skill = InstalledSkill {
            id: skill.id.clone(),
            name: new_name,
            description: new_description,
            directory: skill.directory.clone(),
            repo_owner: skill.repo_owner.clone(),
            repo_name: skill.repo_name.clone(),
            repo_branch: Some(used_branch),
            readme_url,
            apps: skill.apps.clone(),
            installed_at: skill.installed_at,
            content_hash: new_hash,
            updated_at: Utc::now().timestamp(),
        };

        self.with_conn(|dao| dao.save(&updated_skill))?;

        for app in updated_skill.apps.enabled_apps() {
            if let Err(e) = Self::sync_to_app_dir(&updated_skill.directory, &app) {
                log::warn!("同步更新后的 skill 到 {:?} 失败: {e}", app);
            }
        }

        log::info!("Skill {} 更新成功", updated_skill.name);
        Ok(updated_skill)
    }

    /// 为缺少 content_hash 的已安装 Skill 补算哈希。
    pub fn backfill_content_hashes(&self) -> AppResult<usize> {
        let skills = self.all_skills_map()?;
        let ssot_dir = Self::get_ssot_dir()?;
        let mut count = 0;
        for skill in skills.values() {
            if skill.content_hash.is_some() {
                continue;
            }
            let skill_dir = ssot_dir.join(&skill.directory);
            if !skill_dir.exists() {
                continue;
            }
            if let Ok(hash) = Self::compute_dir_hash(&skill_dir) {
                let _ = self.with_conn(|dao| dao.update_hash(&skill.id, &hash, 0));
                count += 1;
            }
        }
        if count > 0 {
            log::info!("已为 {count} 个 Skill 补算内容哈希");
        }
        Ok(count)
    }

    // ========== 备份 ==========

    pub fn list_backups() -> AppResult<Vec<SkillBackupEntry>> {
        let backup_dir = Self::get_backup_dir()?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&backup_dir)? {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    log::warn!("读取 Skill 备份目录项失败: {err}");
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            match Self::read_backup_metadata(&path) {
                Ok(metadata) => entries.push(SkillBackupEntry {
                    backup_id: entry.file_name().to_string_lossy().to_string(),
                    backup_path: path.to_string_lossy().to_string(),
                    created_at: metadata.backup_created_at,
                    skill: metadata.skill,
                }),
                Err(err) => log::warn!("解析 Skill 备份失败 {}: {err}", path.display()),
            }
        }
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.created_at));
        Ok(entries)
    }

    pub fn delete_backup(backup_id: &str) -> AppResult<()> {
        let backup_path = Self::backup_path_for_id(backup_id)?;
        let metadata = fs::symlink_metadata(&backup_path)?;
        if !metadata.is_dir() {
            return Err(AppError::InvalidOperation(format!(
                "Skill backup is not a directory: {}",
                backup_path.display()
            )));
        }
        fs::remove_dir_all(&backup_path)?;
        log::info!("Skill 备份已删除: {}", backup_path.display());
        Ok(())
    }

    pub fn restore_from_backup(
        &self,
        backup_id: &str,
        current_app: &AppType,
    ) -> AppResult<InstalledSkill> {
        let backup_path = Self::backup_path_for_id(backup_id)?;
        let metadata = Self::read_backup_metadata(&backup_path)?;
        let backup_skill_dir = backup_path.join("skill");
        if !backup_skill_dir.join("SKILL.md").exists() {
            return Err(AppError::InvalidOperation(format!(
                "Skill backup is invalid or missing SKILL.md: {}",
                backup_path.display()
            )));
        }

        let existing_skills = self.all_skills_map()?;
        if existing_skills.contains_key(&metadata.skill.id)
            || existing_skills.values().any(|skill| {
                skill
                    .directory
                    .eq_ignore_ascii_case(&metadata.skill.directory)
            })
        {
            return Err(AppError::InvalidOperation(format!(
                "Skill already exists, please uninstall the current one first: {}",
                metadata.skill.directory
            )));
        }

        let ssot_dir = Self::get_ssot_dir()?;
        let restore_path = ssot_dir.join(&metadata.skill.directory);
        if restore_path.exists() || Self::is_symlink(&restore_path) {
            return Err(AppError::InvalidOperation(format!(
                "Restore target already exists: {}",
                restore_path.display()
            )));
        }

        let mut restored_skill = metadata.skill;
        restored_skill.installed_at = Utc::now().timestamp();
        restored_skill.apps = SkillApps::only(current_app);
        restored_skill.updated_at = 0;

        Self::copy_dir_recursive(&backup_skill_dir, &restore_path)?;
        restored_skill.content_hash = Self::compute_dir_hash(&restore_path).ok();

        if let Err(err) = self.with_conn(|dao| dao.save(&restored_skill)) {
            let _ = fs::remove_dir_all(&restore_path);
            return Err(err);
        }

        if !restored_skill.apps.is_empty() {
            if let Err(err) = Self::sync_to_app_dir(&restored_skill.directory, current_app) {
                let _ = self.with_conn(|dao| dao.delete(&restored_skill.id));
                let _ = fs::remove_dir_all(&restore_path);
                return Err(err);
            }
        }

        log::info!(
            "Skill {} 已从备份恢复到 {}",
            restored_skill.name,
            restore_path.display()
        );
        Ok(restored_skill)
    }

    // ========== 启用切换 ==========

    pub fn toggle_app(&self, id: &str, app: &AppType, enabled: bool) -> AppResult<()> {
        let mut skill = self
            .get_skill(id)?
            .ok_or_else(|| AppError::NotFound(format!("Skill not found: {id}")))?;

        skill.apps.set_enabled_for(app, enabled);

        if enabled {
            Self::sync_to_app_dir(&skill.directory, app)?;
        } else {
            Self::remove_from_app(&skill.directory, app)?;
        }

        self.with_conn(|dao| dao.update_apps(id, &skill.apps))?;
        log::info!("Skill {} 的 {:?} 状态已更新为 {}", skill.name, app, enabled);
        Ok(())
    }

    // ========== 同步 ==========

    #[cfg(unix)]
    fn create_symlink(src: &Path, dest: &Path) -> AppResult<()> {
        std::os::unix::fs::symlink(src, dest).map_err(|e| {
            AppError::Skill(format!(
                "创建符号链接失败 {} -> {}: {e}",
                src.display(),
                dest.display()
            ))
        })
    }

    #[cfg(windows)]
    fn create_symlink(src: &Path, dest: &Path) -> AppResult<()> {
        std::os::windows::fs::symlink_dir(src, dest).map_err(|e| {
            AppError::Skill(format!(
                "创建符号链接失败 {} -> {}: {e}",
                src.display(),
                dest.display()
            ))
        })
    }

    fn is_symlink(path: &Path) -> bool {
        path.symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
    }

    fn get_sync_method() -> SyncMethod {
        crate::settings::get_skill_sync_method()
    }

    /// 同步 Skill 到应用目录（symlink/copy/auto）。
    pub fn sync_to_app_dir(directory: &str, app: &AppType) -> AppResult<()> {
        if matches!(app, AppType::ClaudeDesktop) {
            return Ok(());
        }

        let ssot_dir = Self::get_ssot_dir()?;
        let source = ssot_dir.join(directory);
        Self::validate_sync_source_dir(&source, directory)?;

        let app_dir = Self::get_app_skills_dir(app)?;
        fs::create_dir_all(&app_dir)?;
        let dest = app_dir.join(directory);

        match Self::get_sync_method() {
            SyncMethod::Auto => {
                if dest.exists() && !Self::is_symlink(&dest) {
                    Self::replace_dest_with_copy(&source, &dest, directory)?;
                    return Ok(());
                }
                if Self::is_symlink(&dest) {
                    Self::remove_path(&dest)?;
                }
                match Self::create_symlink(&source, &dest) {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        log::warn!(
                            "Symlink 创建失败，将回退到文件复制: {} -> {}. 错误: {err}",
                            source.display(),
                            dest.display()
                        );
                    }
                }
                Self::replace_dest_with_copy(&source, &dest, directory)?;
            }
            SyncMethod::Symlink => {
                if dest.exists() || Self::is_symlink(&dest) {
                    Self::remove_path(&dest)?;
                }
                Self::create_symlink(&source, &dest)?;
            }
            SyncMethod::Copy => {
                Self::replace_dest_with_copy(&source, &dest, directory)?;
            }
        }
        Ok(())
    }

    fn remove_path(path: &Path) -> AppResult<()> {
        if Self::is_symlink(path) {
            #[cfg(unix)]
            fs::remove_file(path)?;
            #[cfg(windows)]
            fs::remove_dir(path)?;
        } else if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn validate_sync_source_dir(source: &Path, directory: &str) -> AppResult<()> {
        if !source.is_dir() {
            return Err(AppError::Skill(format!("Skill 不存在于 SSOT: {directory}")));
        }
        if !source.join("SKILL.md").is_file() {
            return Err(AppError::Skill(format!(
                "Skill 源目录缺少 SKILL.md，拒绝同步: {}",
                source.display()
            )));
        }
        Ok(())
    }

    fn replace_dest_with_copy(source: &Path, dest: &Path, directory: &str) -> AppResult<()> {
        Self::validate_sync_source_dir(source, directory)?;
        let parent = dest.parent().ok_or_else(|| {
            AppError::Skill(format!("Invalid skill destination: {}", dest.display()))
        })?;
        fs::create_dir_all(parent)?;

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_name = Self::sanitize_backup_segment(directory);
        let tmp = parent.join(format!(".{tmp_name}.tmp-{}-{nonce}", std::process::id()));
        if tmp.exists() || Self::is_symlink(&tmp) {
            Self::remove_path(&tmp)?;
        }

        if let Err(err) = Self::copy_dir_recursive(source, &tmp) {
            let _ = Self::remove_path(&tmp);
            return Err(err);
        }

        if dest.exists() || Self::is_symlink(dest) {
            Self::remove_path(dest)?;
        }

        fs::rename(&tmp, dest).map_err(|e| {
            let _ = Self::remove_path(&tmp);
            AppError::Skill(format!(
                "替换 Skill 目录失败 {} -> {}: {e}",
                tmp.display(),
                dest.display()
            ))
        })?;
        Ok(())
    }

    fn is_symlink_to_ssot(path: &Path, ssot_dir: &Path) -> bool {
        if !Self::is_symlink(path) {
            return false;
        }
        let Ok(target) = fs::read_link(path) else {
            return false;
        };
        if target.is_absolute() && target.starts_with(ssot_dir) {
            return true;
        }
        let resolved = path
            .parent()
            .map(|parent| parent.join(&target))
            .unwrap_or(target.clone());
        let canonical_ssot = ssot_dir
            .canonicalize()
            .unwrap_or_else(|_| ssot_dir.to_path_buf());
        let canonical_target = resolved.canonicalize().unwrap_or(resolved);
        canonical_target.starts_with(&canonical_ssot)
    }

    pub fn remove_from_app(directory: &str, app: &AppType) -> AppResult<()> {
        if matches!(app, AppType::ClaudeDesktop) {
            return Ok(());
        }
        let app_dir = Self::get_app_skills_dir(app)?;
        let skill_path = app_dir.join(directory);
        if skill_path.exists() || Self::is_symlink(&skill_path) {
            Self::remove_path(&skill_path)?;
            log::debug!("Skill {directory} 已从 {app:?} 删除");
        }
        Ok(())
    }

    /// 同步所有已启用的 Skills 到指定应用，返回同步数量。
    pub fn sync_to_app(&self, app: &AppType) -> AppResult<usize> {
        if matches!(app, AppType::ClaudeDesktop) {
            return Ok(0);
        }

        let skills = self.all_skills_map()?;
        let ssot_dir = Self::get_ssot_dir()?;
        let app_dir = Self::get_app_skills_dir(app)?;

        let indexed_skills: HashMap<String, &InstalledSkill> = skills
            .values()
            .map(|skill| (skill.directory.to_lowercase(), skill))
            .collect();

        if app_dir.exists() {
            for entry in fs::read_dir(&app_dir)? {
                let entry = entry?;
                let path = entry.path();
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if dir_name.starts_with('.') {
                    continue;
                }
                if let Some(skill) = indexed_skills.get(&dir_name.to_lowercase()) {
                    if !skill.apps.is_enabled_for(app) {
                        Self::remove_path(&path)?;
                    }
                    continue;
                }
                if Self::is_symlink_to_ssot(&path, &ssot_dir) {
                    Self::remove_path(&path)?;
                }
            }
        }

        let mut synced = 0;
        for skill in skills.values() {
            if skill.apps.is_enabled_for(app) {
                Self::sync_to_app_dir(&skill.directory, app)?;
                synced += 1;
            }
        }
        Ok(synced)
    }

    // ========== 发现 / 市场 ==========

    /// 列出默认仓库中的可发现技能。
    pub async fn discover_available(&self) -> AppResult<Vec<DiscoverableSkill>> {
        self.discover_from_repos(default_repos()).await
    }

    /// 从给定仓库列表发现技能。
    pub async fn discover_from_repos(
        &self,
        repos: Vec<SkillRepo>,
    ) -> AppResult<Vec<DiscoverableSkill>> {
        let enabled_repos: Vec<SkillRepo> = repos.into_iter().filter(|r| r.enabled).collect();
        let fetch_tasks = enabled_repos
            .iter()
            .map(|repo| self.fetch_repo_skills(repo));
        let results = futures::future::join_all(fetch_tasks).await;

        let mut skills = Vec::new();
        for (repo, result) in enabled_repos.into_iter().zip(results) {
            match result {
                Ok(repo_skills) => skills.extend(repo_skills),
                Err(e) => log::warn!("获取仓库 {}/{} 技能失败: {e}", repo.owner, repo.name),
            }
        }

        Self::deduplicate_discoverable_skills(&mut skills);
        skills.sort_by_key(|skill| skill.name.to_lowercase());
        Ok(skills)
    }

    /// 搜索 skills.sh 公共目录（委托给 [`crate::skills_sh`]）。
    pub async fn search_skills_sh(
        query: &str,
        limit: usize,
        offset: usize,
    ) -> AppResult<crate::skills_sh::SkillsShSearchResult> {
        crate::skills_sh::search(query, limit, offset).await
    }

    /// skills.sh 热门技能（委托给 [`crate::skills_sh`]）。
    pub async fn get_popular_skills_sh(
        limit: usize,
    ) -> AppResult<crate::skills_sh::SkillsShSearchResult> {
        crate::skills_sh::popular(limit).await
    }

    async fn fetch_repo_skills(&self, repo: &SkillRepo) -> AppResult<Vec<DiscoverableSkill>> {
        let (temp_dir, resolved_branch) =
            timeout(std::time::Duration::from_secs(60), self.download_repo(repo))
                .await
                .map_err(|_| {
                    AppError::Skill(format_skill_error(
                        "DOWNLOAD_TIMEOUT",
                        &[
                            ("owner", &repo.owner),
                            ("name", &repo.name),
                            ("timeout", "60"),
                        ],
                        Some("checkNetwork"),
                    ))
                })??;

        let mut skills = Vec::new();
        let mut resolved_repo = repo.clone();
        resolved_repo.branch = resolved_branch;
        let _ = self.scan_dir_recursive(&temp_dir, &temp_dir, &resolved_repo, &mut skills);
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(skills)
    }

    fn scan_dir_recursive(
        &self,
        current_dir: &Path,
        base_dir: &Path,
        repo: &SkillRepo,
        skills: &mut Vec<DiscoverableSkill>,
    ) -> AppResult<()> {
        let skill_md = current_dir.join("SKILL.md");
        if skill_md.exists() {
            let directory = if current_dir == base_dir {
                repo.name.clone()
            } else {
                current_dir
                    .strip_prefix(base_dir)
                    .unwrap_or(current_dir)
                    .to_string_lossy()
                    .to_string()
            };
            let doc_path = skill_md
                .strip_prefix(base_dir)
                .unwrap_or(skill_md.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            if let Ok(skill) =
                self.build_skill_from_metadata(&skill_md, &directory, &doc_path, repo)
            {
                skills.push(skill);
            }
            return Ok(());
        }
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.scan_dir_recursive(&path, base_dir, repo, skills)?;
            }
        }
        Ok(())
    }

    fn build_skill_from_metadata(
        &self,
        skill_md: &Path,
        directory: &str,
        doc_path: &str,
        repo: &SkillRepo,
    ) -> AppResult<DiscoverableSkill> {
        let meta = Self::parse_skill_metadata_static(skill_md)?;
        Ok(DiscoverableSkill {
            key: format!("{}/{}:{}", repo.owner, repo.name, directory),
            name: meta.name.unwrap_or_else(|| directory.to_string()),
            description: meta.description.unwrap_or_default(),
            directory: directory.to_string(),
            readme_url: Some(Self::build_skill_doc_url(
                &repo.owner,
                &repo.name,
                &repo.branch,
                doc_path,
            )),
            repo_owner: repo.owner.clone(),
            repo_name: repo.name.clone(),
            repo_branch: repo.branch.clone(),
        })
    }

    fn parse_skill_metadata_static(path: &Path) -> AppResult<SkillMetadata> {
        let content = fs::read_to_string(path)?;
        let content = content.trim_start_matches('\u{feff}');
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Ok(SkillMetadata {
                name: None,
                description: None,
            });
        }
        let front_matter = parts[1].trim();
        let meta: SkillMetadata = serde_yaml::from_str(front_matter).unwrap_or(SkillMetadata {
            name: None,
            description: None,
        });
        Ok(meta)
    }

    fn read_skill_name_desc(skill_md: &Path, fallback_name: &str) -> (String, Option<String>) {
        if skill_md.exists() {
            match Self::parse_skill_metadata_static(skill_md) {
                Ok(meta) => (
                    meta.name.unwrap_or_else(|| fallback_name.to_string()),
                    meta.description,
                ),
                Err(_) => (fallback_name.to_string(), None),
            }
        } else {
            (fallback_name.to_string(), None)
        }
    }

    fn deduplicate_discoverable_skills(skills: &mut Vec<DiscoverableSkill>) {
        let mut seen = std::collections::HashSet::new();
        skills.retain(|skill| seen.insert(skill.key.to_lowercase()));
    }

    // ========== 安全路径校验 ==========

    fn sanitize_skill_source_path(raw: &str) -> Option<PathBuf> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut normalized = PathBuf::new();
        let mut has_component = false;
        for component in Path::new(trimmed).components() {
            match component {
                Component::Normal(name) => {
                    let segment = name.to_string_lossy().trim().to_string();
                    if segment.is_empty() || segment == "." || segment == ".." {
                        return None;
                    }
                    normalized.push(segment);
                    has_component = true;
                }
                Component::CurDir
                | Component::ParentDir
                | Component::RootDir
                | Component::Prefix(_) => return None,
            }
        }
        has_component.then_some(normalized)
    }

    fn sanitize_install_name(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let path = Path::new(trimmed);
        let mut components = path.components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(name)), None) => {
                let normalized = name.to_string_lossy().trim().to_string();
                if normalized.is_empty()
                    || normalized == "."
                    || normalized == ".."
                    || normalized.starts_with('.')
                {
                    None
                } else {
                    Some(normalized)
                }
            }
            _ => None,
        }
    }

    fn find_skill_dir_by_name(root: &Path, target_name: &str) -> Option<PathBuf> {
        fn walk(dir: &Path, target: &str, depth: usize) -> Option<PathBuf> {
            if depth > 3 {
                return None;
            }
            let entries = fs::read_dir(dir).ok()?;
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') {
                    continue;
                }
                if name_str.eq_ignore_ascii_case(target) && path.join("SKILL.md").exists() {
                    return Some(path);
                }
                if let Some(found) = walk(&path, target, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
        walk(root, target_name, 0)
    }

    fn resolve_skill_source_dir(root: &Path, raw_directory: &str) -> Option<PathBuf> {
        let source_rel = Self::sanitize_skill_source_path(raw_directory)?;
        let direct = root.join(&source_rel);
        if direct.is_dir() {
            return Some(direct);
        }
        let target_name = source_rel.file_name()?.to_string_lossy().to_string();
        if let Some(found) = Self::find_skill_dir_by_name(root, &target_name) {
            return Some(found);
        }
        if root.is_dir() && root.join("SKILL.md").exists() {
            return Some(root.to_path_buf());
        }
        None
    }

    // ========== 下载 ==========

    async fn download_repo(&self, repo: &SkillRepo) -> AppResult<(PathBuf, String)> {
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().to_path_buf();
        let _ = temp_dir.keep();

        let mut branches = Vec::new();
        if !repo.branch.is_empty() && !repo.branch.eq_ignore_ascii_case("HEAD") {
            branches.push(repo.branch.as_str());
        }
        if !branches.contains(&"main") {
            branches.push("main");
        }
        if !branches.contains(&"master") {
            branches.push("master");
        }

        let mut last_error = None;
        for branch in branches {
            let url = format!(
                "https://github.com/{}/{}/archive/refs/heads/{}.zip",
                repo.owner, repo.name, branch
            );
            match self.download_and_extract(&url, &temp_path).await {
                Ok(_) => return Ok((temp_path, branch.to_string())),
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }
        Err(last_error.unwrap_or_else(|| AppError::Skill("所有分支下载失败".to_string())))
    }

    async fn download_and_extract(&self, url: &str, dest: &Path) -> AppResult<()> {
        let client = Self::http_client()?;
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16().to_string();
            return Err(AppError::Skill(format_skill_error(
                "DOWNLOAD_FAILED",
                &[("status", &status)],
                match status.as_str() {
                    "403" => Some("http403"),
                    "404" => Some("http404"),
                    "429" => Some("http429"),
                    _ => Some("checkNetwork"),
                },
            )));
        }

        let bytes = response.bytes().await?;
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;

        let root_name = if !archive.is_empty() {
            let first_file = archive.by_index(0)?;
            first_file
                .name()
                .split('/')
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            return Err(AppError::Skill(format_skill_error(
                "EMPTY_ARCHIVE",
                &[],
                Some("checkRepoUrl"),
            )));
        };

        let mut symlinks: Vec<(PathBuf, String)> = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = file.name().to_string();
            let relative_path = match file_path.strip_prefix(&format!("{root_name}/")) {
                Some(stripped) => stripped,
                None => continue,
            };
            if relative_path.is_empty() {
                continue;
            }
            let outpath = dest.join(relative_path);
            if file.is_symlink() {
                let mut target = String::new();
                std::io::Read::read_to_string(&mut file, &mut target)?;
                symlinks.push((outpath, target.trim().to_string()));
            } else if file.is_dir() {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Self::resolve_symlinks_in_dir(dest, &symlinks)?;
        Ok(())
    }

    fn resolve_symlinks_in_dir(base_dir: &Path, symlinks: &[(PathBuf, String)]) -> AppResult<()> {
        let canonical_base = base_dir
            .canonicalize()
            .unwrap_or_else(|_| base_dir.to_path_buf());
        for (link_path, target) in symlinks {
            let parent = link_path.parent().unwrap_or(base_dir);
            let resolved = parent.join(target);
            let resolved = match resolved.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    log::warn!(
                        "Symlink 目标不存在，跳过: {} -> {target}",
                        link_path.display()
                    );
                    continue;
                }
            };
            if !resolved.starts_with(&canonical_base) {
                log::warn!(
                    "Symlink 目标超出仓库范围，跳过: {} -> {}",
                    link_path.display(),
                    resolved.display()
                );
                continue;
            }
            if resolved.is_dir() {
                Self::copy_dir_recursive(&resolved, link_path)?;
            } else if resolved.is_file() {
                if let Some(parent) = link_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&resolved, link_path)?;
            }
        }
        Ok(())
    }

    fn copy_dir_recursive(src: &Path, dest: &Path) -> AppResult<()> {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if path.is_dir() {
                Self::copy_dir_recursive(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    // ========== 备份辅助 ==========

    fn resolve_uninstall_backup_source(skill: &InstalledSkill) -> AppResult<Option<PathBuf>> {
        let ssot_path = Self::get_ssot_dir()?.join(&skill.directory);
        if ssot_path.is_dir() {
            return Ok(Some(ssot_path));
        }
        for app in AppType::all() {
            let app_dir = match Self::get_app_skills_dir(&app) {
                Ok(dir) => dir,
                Err(_) => continue,
            };
            let candidate = app_dir.join(&skill.directory);
            if candidate.is_dir() {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    fn sanitize_backup_segment(segment: &str) -> String {
        let sanitized = segment
            .chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
                _ => '-',
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        if sanitized.is_empty() {
            "skill".to_string()
        } else {
            sanitized
        }
    }

    fn cleanup_old_skill_backups(dir: &Path) -> AppResult<()> {
        let mut entries = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let metadata = entry.metadata().ok()?;
                if !metadata.is_dir() {
                    return None;
                }
                Some((entry.path(), metadata.modified().ok()))
            })
            .collect::<Vec<_>>();
        if entries.len() <= SKILL_BACKUP_RETAIN_COUNT {
            return Ok(());
        }
        entries.sort_by_key(|(_, modified)| *modified);
        let remove_count = entries.len().saturating_sub(SKILL_BACKUP_RETAIN_COUNT);
        for (path, _) in entries.into_iter().take(remove_count) {
            fs::remove_dir_all(&path)?;
        }
        Ok(())
    }

    fn backup_path_for_id(backup_id: &str) -> AppResult<PathBuf> {
        if backup_id.contains("..")
            || backup_id.contains('/')
            || backup_id.contains('\\')
            || backup_id.trim().is_empty()
        {
            return Err(AppError::InvalidOperation(format!(
                "Invalid backup id: {backup_id}"
            )));
        }
        Ok(Self::get_backup_dir()?.join(backup_id))
    }

    fn read_backup_metadata(backup_path: &Path) -> AppResult<SkillBackupMetadata> {
        let metadata_path = backup_path.join("meta.json");
        let content = fs::read_to_string(&metadata_path)?;
        serde_json::from_str(&content).map_err(AppError::Json)
    }

    fn create_uninstall_backup(skill: &InstalledSkill) -> AppResult<Option<PathBuf>> {
        let Some(source_path) = Self::resolve_uninstall_backup_source(skill)? else {
            log::warn!(
                "Skill {} 卸载前未找到可备份的目录，将跳过备份",
                skill.directory
            );
            return Ok(None);
        };

        let backup_root = Self::get_backup_dir()?;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let slug = Self::sanitize_backup_segment(&skill.directory);
        let mut backup_path = backup_root.join(format!("{timestamp}_{slug}"));
        let mut counter = 1;
        while backup_path.exists() {
            backup_path = backup_root.join(format!("{timestamp}_{slug}_{counter}"));
            counter += 1;
        }

        let write_backup = || -> AppResult<()> {
            let skill_backup_dir = backup_path.join("skill");
            Self::copy_dir_recursive(&source_path, &skill_backup_dir)?;
            let metadata = SkillBackupMetadata {
                skill: skill.clone(),
                backup_created_at: Utc::now().timestamp(),
                source_path: source_path.to_string_lossy().to_string(),
            };
            let metadata_json = serde_json::to_string_pretty(&metadata).map_err(AppError::Json)?;
            fs::write(backup_path.join("meta.json"), metadata_json)?;
            Ok(())
        };

        if let Err(err) = write_backup() {
            let _ = fs::remove_dir_all(&backup_path);
            return Err(err);
        }
        if let Err(err) = Self::cleanup_old_skill_backups(&backup_root) {
            log::warn!("清理旧 Skill 备份失败: {err}");
        }

        log::info!(
            "Skill {} 已在卸载前备份到 {}",
            skill.name,
            backup_path.display()
        );
        Ok(Some(backup_path))
    }

    // ========== 本地 ZIP 安装 ==========

    pub fn install_from_zip(
        &self,
        zip_path: &Path,
        current_app: &AppType,
    ) -> AppResult<Vec<InstalledSkill>> {
        let temp_dir = Self::extract_local_zip(zip_path)?;
        let skill_dirs = Self::scan_skills_in_dir(&temp_dir)?;
        if skill_dirs.is_empty() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(AppError::Skill(format_skill_error(
                "NO_SKILLS_IN_ZIP",
                &[],
                Some("checkZipContent"),
            )));
        }

        let ssot_dir = Self::get_ssot_dir()?;
        let mut installed = Vec::new();
        let existing_skills = self.all_skills_map()?;
        let zip_stem = zip_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        for skill_dir in skill_dirs {
            let skill_md = skill_dir.join("SKILL.md");
            let meta = if skill_md.exists() {
                Self::parse_skill_metadata_static(&skill_md).ok()
            } else {
                None
            };

            let install_name = {
                let dir_name = skill_dir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if skill_dir == temp_dir || dir_name.is_empty() || dir_name.starts_with('.') {
                    meta.as_ref()
                        .and_then(|m| m.name.as_deref())
                        .and_then(Self::sanitize_install_name)
                        .or_else(|| zip_stem.as_deref().and_then(Self::sanitize_install_name))
                } else {
                    Self::sanitize_install_name(&dir_name)
                        .or_else(|| {
                            meta.as_ref()
                                .and_then(|m| m.name.as_deref())
                                .and_then(Self::sanitize_install_name)
                        })
                        .or_else(|| zip_stem.as_deref().and_then(Self::sanitize_install_name))
                }
            };
            let install_name = match install_name {
                Some(name) => name,
                None => {
                    let _ = fs::remove_dir_all(&temp_dir);
                    return Err(AppError::Skill(format_skill_error(
                        "INVALID_SKILL_DIRECTORY",
                        &[("zip", &zip_path.display().to_string())],
                        Some("checkZipContent"),
                    )));
                }
            };

            if existing_skills
                .values()
                .any(|s| s.directory.eq_ignore_ascii_case(&install_name))
            {
                log::warn!("Skill directory '{install_name}' already exists, skipping");
                continue;
            }

            let (name, description) = match meta {
                Some(m) => (
                    m.name.unwrap_or_else(|| install_name.clone()),
                    m.description,
                ),
                None => (install_name.clone(), None),
            };

            let dest = ssot_dir.join(&install_name);
            if dest.exists() {
                let _ = fs::remove_dir_all(&dest);
            }
            Self::copy_dir_recursive(&skill_dir, &dest)?;
            let content_hash = Self::compute_dir_hash(&dest).ok();

            let skill = InstalledSkill {
                id: format!("local:{install_name}"),
                name,
                description,
                directory: install_name.clone(),
                repo_owner: None,
                repo_name: None,
                repo_branch: None,
                readme_url: None,
                apps: SkillApps::only(current_app),
                installed_at: Utc::now().timestamp(),
                content_hash,
                updated_at: 0,
            };

            self.with_conn(|dao| dao.save(&skill))?;
            Self::sync_to_app_dir(&install_name, current_app)?;
            log::info!("Skill {} installed from ZIP", skill.name);
            installed.push(skill);
        }

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(installed)
    }

    fn extract_local_zip(zip_path: &Path) -> AppResult<PathBuf> {
        let file = fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        if archive.is_empty() {
            return Err(AppError::Skill(format_skill_error(
                "EMPTY_ARCHIVE",
                &[],
                Some("checkZipContent"),
            )));
        }

        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().to_path_buf();
        let _ = temp_dir.keep();

        let mut symlinks: Vec<(PathBuf, String)> = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };
            let outpath = temp_path.join(&file_path);
            if file.is_symlink() {
                let mut target = String::new();
                std::io::Read::read_to_string(&mut file, &mut target)?;
                symlinks.push((outpath, target.trim().to_string()));
            } else if file.is_dir() {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        Self::resolve_symlinks_in_dir(&temp_path, &symlinks)?;
        Ok(temp_path)
    }

    fn scan_skills_in_dir(dir: &Path) -> AppResult<Vec<PathBuf>> {
        let mut skill_dirs = Vec::new();
        Self::scan_skills_recursive(dir, &mut skill_dirs)?;
        Ok(skill_dirs)
    }

    fn scan_skills_recursive(current: &Path, results: &mut Vec<PathBuf>) -> AppResult<()> {
        if current.join("SKILL.md").exists() {
            results.push(current.to_path_buf());
            return Ok(());
        }
        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    if dir_name.starts_with('.') {
                        continue;
                    }
                    Self::scan_skills_recursive(&path, results)?;
                }
            }
        }
        Ok(())
    }
}

// ========== 阶段5：仓库 / 扫描 / 导入 / 迁移 ==========

impl SkillService {
    /// 首次访问仓库时，把默认仓库种入 DB（一次性，标记存于 skill_meta）。
    ///
    /// 用标记而非"空则回退"，这样用户删除全部仓库后不会被默认仓库重新填充，
    /// 且无论先 add 还是先 list，默认仓库都会持久存在。
    fn ensure_repos_seeded(&self) -> AppResult<()> {
        if self
            .with_conn(|dao| dao.get_meta("skill_repos_seeded"))?
            .is_some()
        {
            return Ok(());
        }
        for repo in default_repos() {
            let _ = self.with_conn(|dao| {
                dao.save_repo(&repo.owner, &repo.name, &repo.branch, repo.enabled)
            });
        }
        self.with_conn(|dao| dao.set_meta("skill_repos_seeded", "1"))?;
        Ok(())
    }

    /// 列出技能仓库（含持久化的默认仓库）。
    pub fn list_repos(&self) -> AppResult<Vec<SkillRepo>> {
        self.ensure_repos_seeded()?;
        let rows = self.with_conn(|dao| dao.list_repos())?;
        Ok(rows
            .into_iter()
            .map(|(owner, name, branch, enabled)| SkillRepo {
                owner,
                name,
                branch,
                enabled,
            })
            .collect())
    }

    /// 添加 / 更新技能仓库。
    pub fn add_repo(&self, repo: &SkillRepo) -> AppResult<()> {
        self.ensure_repos_seeded()?;
        self.with_conn(|dao| dao.save_repo(&repo.owner, &repo.name, &repo.branch, repo.enabled))
    }

    /// 删除技能仓库。
    pub fn remove_repo(&self, owner: &str, name: &str) -> AppResult<()> {
        self.ensure_repos_seeded()?;
        self.with_conn(|dao| dao.delete_repo(owner, name))
    }

    /// 将 lock 文件中发现的仓库保存到 skill_repos（去重）。
    fn save_repos_from_lock<'a>(
        &self,
        lock: &HashMap<String, LockRepoInfo>,
        directories: impl Iterator<Item = &'a str>,
    ) {
        let existing: std::collections::HashSet<(String, String)> = self
            .with_conn(|dao| dao.list_repos())
            .unwrap_or_default()
            .into_iter()
            .map(|(o, n, _, _)| (o, n))
            .collect();
        let mut added = std::collections::HashSet::new();

        for dir_name in directories {
            if let Some(info) = lock.get(dir_name) {
                let key = (info.owner.clone(), info.repo.clone());
                if !existing.contains(&key) && added.insert(key) {
                    let branch = info.branch.clone().unwrap_or_else(|| "HEAD".to_string());
                    if let Err(e) =
                        self.with_conn(|dao| dao.save_repo(&info.owner, &info.repo, &branch, true))
                    {
                        log::warn!("保存 skill 仓库 {}/{} 失败: {e}", info.owner, info.repo);
                    }
                }
            }
        }
    }

    /// 扫描各应用目录，找出未被 Olenro 管理的 Skills。
    pub fn scan_unmanaged(&self) -> AppResult<Vec<UnmanagedSkill>> {
        let managed_skills = self.all_skills_map()?;
        let managed_dirs: std::collections::HashSet<String> = managed_skills
            .values()
            .map(|s| s.directory.clone())
            .collect();

        let mut scan_sources: Vec<(PathBuf, String)> = Vec::new();
        for app in AppType::all() {
            if let Ok(d) = Self::get_app_skills_dir(&app) {
                scan_sources.push((d, app.as_str().to_string()));
            }
        }
        if let Some(agents_dir) = Self::get_agents_skills_dir() {
            scan_sources.push((agents_dir, "agents".to_string()));
        }
        if let Ok(ssot_dir) = Self::get_ssot_dir() {
            scan_sources.push((ssot_dir, "cc-switch".to_string()));
        }

        let mut unmanaged: HashMap<String, UnmanagedSkill> = HashMap::new();
        for (scan_dir, label) in &scan_sources {
            let entries = match fs::read_dir(scan_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if dir_name.starts_with('.') || managed_dirs.contains(&dir_name) {
                    continue;
                }
                let skill_md = path.join("SKILL.md");
                if !skill_md.exists() {
                    continue;
                }
                let (name, description) = Self::read_skill_name_desc(&skill_md, &dir_name);
                unmanaged
                    .entry(dir_name.clone())
                    .and_modify(|s| s.found_in.push(label.clone()))
                    .or_insert(UnmanagedSkill {
                        directory: dir_name,
                        name,
                        description,
                        found_in: vec![label.clone()],
                        path: path.display().to_string(),
                    });
            }
        }
        Ok(unmanaged.into_values().collect())
    }

    /// 将未管理的 Skills 导入到 Olenro 统一管理。
    pub fn import_from_apps(
        &self,
        imports: Vec<ImportSkillSelection>,
    ) -> AppResult<Vec<InstalledSkill>> {
        let ssot_dir = Self::get_ssot_dir()?;
        let agents_lock = parse_agents_lock();
        let mut imported = Vec::new();

        self.save_repos_from_lock(&agents_lock, imports.iter().map(|s| s.directory.as_str()));

        let mut search_sources: Vec<(PathBuf, String)> = Vec::new();
        for app in AppType::all() {
            if let Ok(d) = Self::get_app_skills_dir(&app) {
                search_sources.push((d, app.as_str().to_string()));
            }
        }
        if let Some(agents_dir) = Self::get_agents_skills_dir() {
            search_sources.push((agents_dir, "agents".to_string()));
        }
        search_sources.push((ssot_dir.clone(), "cc-switch".to_string()));

        for selection in imports {
            let dir_name = selection.directory;
            let mut source_path: Option<PathBuf> = None;
            for (base, _label) in &search_sources {
                let skill_path = base.join(&dir_name);
                if skill_path.exists() && source_path.is_none() {
                    source_path = Some(skill_path);
                }
            }

            let source = match source_path {
                Some(p) => p,
                None => continue,
            };
            if !source.join("SKILL.md").exists() {
                log::warn!("跳过导入 '{dir_name}'：源缺少 SKILL.md");
                continue;
            }

            let dest = ssot_dir.join(&dir_name);
            if !dest.exists() {
                Self::copy_dir_recursive(&source, &dest)?;
            }

            let skill_md = dest.join("SKILL.md");
            let (name, description) = Self::read_skill_name_desc(&skill_md, &dir_name);
            let apps = selection.apps;
            let (id, repo_owner, repo_name, repo_branch, readme_url) =
                build_repo_info_from_lock(&agents_lock, &dir_name);
            let content_hash = Self::compute_dir_hash(&dest).ok();

            let skill = InstalledSkill {
                id,
                name,
                description,
                directory: dir_name,
                repo_owner,
                repo_name,
                repo_branch,
                readme_url,
                apps,
                installed_at: Utc::now().timestamp(),
                content_hash,
                updated_at: 0,
            };
            self.with_conn(|dao| dao.save(&skill))?;
            imported.push(skill);
        }

        log::info!("成功导入 {} 个 Skills", imported.len());
        Ok(imported)
    }

    /// 迁移 Skill 存储位置（在两个 SSOT 目录间移动文件）。
    ///
    /// 安全策略：先移文件，后改设置。中途崩溃时设置仍指向旧目录。
    pub fn migrate_storage(&self, target: SkillStorageLocation) -> AppResult<MigrationResult> {
        let current = crate::settings::get_skill_storage_location();
        if current == target {
            return Ok(MigrationResult::default());
        }

        let old_dir = Self::get_ssot_dir()?;
        let new_dir = match target {
            SkillStorageLocation::CcSwitch => get_app_config_dir().join("skills"),
            SkillStorageLocation::Unified => {
                crate::config::get_home_dir().join(".agents").join("skills")
            }
        };
        fs::create_dir_all(&new_dir)?;

        let skills = self.all_skills_map()?;
        let mut result = MigrationResult::default();

        for skill in skills.values() {
            let src = old_dir.join(&skill.directory);
            let dst = new_dir.join(&skill.directory);
            if !src.exists() || dst.exists() {
                result.skipped_count += 1;
                continue;
            }
            match fs::rename(&src, &dst) {
                Ok(()) => result.migrated_count += 1,
                Err(_) => match Self::copy_dir_recursive(&src, &dst) {
                    Ok(()) => {
                        let _ = fs::remove_dir_all(&src);
                        result.migrated_count += 1;
                    }
                    Err(e) => result.errors.push(format!("{}: {e}", skill.directory)),
                },
            }
        }

        crate::settings::set_skill_storage_location(target)?;

        for app in AppType::all() {
            let _ = self.sync_to_app(&app);
        }

        log::info!(
            "Skill 存储迁移完成: {} 迁移, {} 跳过, {} 错误",
            result.migrated_count,
            result.skipped_count,
            result.errors.len()
        );
        Ok(result)
    }
}

// ========== ~/.agents/ lock 文件解析 ==========

#[derive(Deserialize)]
struct AgentsLockFile {
    skills: HashMap<String, AgentsLockSkill>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentsLockSkill {
    source: Option<String>,
    source_type: Option<String>,
    source_url: Option<String>,
    skill_path: Option<String>,
    branch: Option<String>,
    source_branch: Option<String>,
}

#[derive(Debug, Clone)]
struct LockRepoInfo {
    owner: String,
    repo: String,
    skill_path: Option<String>,
    branch: Option<String>,
}

fn normalize_optional_branch(branch: Option<String>) -> Option<String> {
    branch.and_then(|b| {
        let trimmed = b.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_branch_from_source_url(source_url: Option<&str>) -> Option<String> {
    let source_url = source_url?.trim();
    if source_url.is_empty() {
        return None;
    }
    if let Some((_, after_tree)) = source_url.split_once("/tree/") {
        let branch = after_tree
            .split('/')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())?;
        return Some(branch.to_string());
    }
    if let Some((_, fragment)) = source_url.split_once('#') {
        let branch = fragment
            .split('&')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())?;
        return Some(branch.to_string());
    }
    if let Some((_, query)) = source_url.split_once('?') {
        for pair in query.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if matches!(key, "branch" | "ref") {
                let branch = value.trim();
                if !branch.is_empty() {
                    return Some(branch.to_string());
                }
            }
        }
    }
    None
}

/// 解析 `~/.agents/.skill-lock.json`，返回 skill_name -> 仓库信息。
fn parse_agents_lock() -> HashMap<String, LockRepoInfo> {
    let path = crate::config::get_home_dir()
        .join(".agents")
        .join(".skill-lock.json");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let lock: AgentsLockFile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            log::warn!("解析 agents lock 文件失败 ({}): {e}", path.display());
            return HashMap::new();
        }
    };
    lock.skills
        .into_iter()
        .filter_map(|(name, skill)| {
            let source = skill.source?;
            if skill.source_type.as_deref() != Some("github") {
                return None;
            }
            let (owner, repo) = source.split_once('/')?;
            let branch = normalize_optional_branch(skill.branch)
                .or_else(|| normalize_optional_branch(skill.source_branch))
                .or_else(|| parse_branch_from_source_url(skill.source_url.as_deref()));
            Some((
                name,
                LockRepoInfo {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    skill_path: skill.skill_path,
                    branch,
                },
            ))
        })
        .collect()
}

/// 从 lock 文件信息构建 skill 的 (id, repo_owner, repo_name, repo_branch, readme_url)。
fn build_repo_info_from_lock(
    lock: &HashMap<String, LockRepoInfo>,
    dir_name: &str,
) -> (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    match lock.get(dir_name) {
        Some(info) => {
            let branch = info.branch.clone();
            let url_branch = branch.clone().unwrap_or_else(|| "HEAD".to_string());
            let fallback = format!("{dir_name}/SKILL.md");
            let doc_path = info.skill_path.as_deref().unwrap_or(&fallback);
            let url = Some(SkillService::build_skill_doc_url(
                &info.owner,
                &info.repo,
                &url_branch,
                doc_path,
            ));
            (
                format!("{}/{}:{dir_name}", info.owner, info.repo),
                Some(info.owner.clone()),
                Some(info.repo.clone()),
                branch,
                url,
            )
        }
        None => (format!("local:{dir_name}"), None, None, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(dir: &Path, name: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: test\n---\nbody"),
        )
        .unwrap();
    }

    #[test]
    fn sanitize_rejects_traversal() {
        assert!(SkillService::sanitize_skill_source_path("../evil").is_none());
        assert!(SkillService::sanitize_skill_source_path("/abs").is_none());
        assert_eq!(
            SkillService::sanitize_skill_source_path("skills/foo"),
            Some(PathBuf::from("skills/foo"))
        );
        assert_eq!(
            SkillService::sanitize_install_name("foo").as_deref(),
            Some("foo")
        );
        assert!(SkillService::sanitize_install_name("a/b").is_none());
        assert!(SkillService::sanitize_install_name(".hidden").is_none());
    }

    #[test]
    fn compute_dir_hash_is_stable_and_ignores_hidden() {
        let dir = tempfile::tempdir().unwrap();
        write_skill(dir.path(), "S");
        let h1 = SkillService::compute_dir_hash(dir.path()).unwrap();
        // 隐藏文件不影响哈希
        fs::write(dir.path().join(".DS_Store"), b"junk").unwrap();
        let h2 = SkillService::compute_dir_hash(dir.path()).unwrap();
        assert_eq!(h1, h2);
        // 内容变化影响哈希
        fs::write(dir.path().join("SKILL.md"), b"changed").unwrap();
        let h3 = SkillService::compute_dir_hash(dir.path()).unwrap();
        assert_ne!(h1, h3);
    }

    #[test]
    fn resolve_skill_source_dir_handles_root_and_nested() {
        let root = tempfile::tempdir().unwrap();
        // 根目录即 skill
        write_skill(root.path(), "Root");
        assert_eq!(
            SkillService::resolve_skill_source_dir(root.path(), "anything"),
            Some(root.path().to_path_buf())
        );

        let root2 = tempfile::tempdir().unwrap();
        let nested = root2.path().join("skills").join("foo");
        write_skill(&nested, "Foo");
        assert_eq!(
            SkillService::resolve_skill_source_dir(root2.path(), "skills/foo"),
            Some(nested)
        );
    }

    #[test]
    #[serial_test::serial]
    fn install_from_zip_and_sync_and_uninstall() {
        let home = tempfile::tempdir().unwrap();
        let old = std::env::var_os("OLENRO_TEST_HOME");
        std::env::set_var("OLENRO_TEST_HOME", home.path());

        let db = home.path().join(".olenro-cli").join("test.db");
        let svc = SkillService::new(db);

        // 构造一个本地 zip
        let src = tempfile::tempdir().unwrap();
        let skill_dir = src.path().join("my-skill");
        write_skill(&skill_dir, "My Skill");
        let zip_path = src.path().join("pack.zip");
        {
            let f = fs::File::create(&zip_path).unwrap();
            let mut zw = zip::ZipWriter::new(f);
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            zw.start_file("my-skill/SKILL.md", opts).unwrap();
            use std::io::Write;
            zw.write_all(b"---\nname: My Skill\ndescription: d\n---\nbody")
                .unwrap();
            zw.finish().unwrap();
        }

        let installed = svc.install_from_zip(&zip_path, &AppType::Claude).unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].directory, "my-skill");

        // 已同步到 ~/.claude/skills/my-skill/SKILL.md
        let synced = SkillService::get_app_skills_dir(&AppType::Claude)
            .unwrap()
            .join("my-skill")
            .join("SKILL.md");
        assert!(synced.exists(), "synced skill should exist at {synced:?}");

        // 列表可见
        assert_eq!(svc.list_skills().unwrap().len(), 1);

        // 卸载（含备份）
        let res = svc.uninstall("local:my-skill").unwrap();
        assert!(res.backup_path.is_some());
        assert!(svc.list_skills().unwrap().is_empty());

        match old {
            Some(v) => std::env::set_var("OLENRO_TEST_HOME", v),
            None => std::env::remove_var("OLENRO_TEST_HOME"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn scan_unmanaged_then_import_roundtrip() {
        let home = tempfile::tempdir().unwrap();
        let old = std::env::var_os("OLENRO_TEST_HOME");
        std::env::set_var("OLENRO_TEST_HOME", home.path());

        let db = home.path().join(".olenro-cli").join("test.db");
        let svc = SkillService::new(db);

        // 在 ~/.claude/skills/ 放一个未纳管的 skill
        let app_skill = home.path().join(".claude").join("skills").join("orphan");
        write_skill(&app_skill, "Orphan");

        let unmanaged = svc.scan_unmanaged().unwrap();
        assert!(
            unmanaged
                .iter()
                .any(|u| u.directory == "orphan" && u.found_in.iter().any(|f| f == "claude")),
            "should detect unmanaged skill, got: {unmanaged:?}"
        );

        let imported = svc
            .import_from_apps(vec![ImportSkillSelection {
                directory: "orphan".to_string(),
                apps: SkillApps::only(&AppType::Claude),
            }])
            .unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].directory, "orphan");
        assert!(imported[0].apps.is_enabled_for(&AppType::Claude));

        // 已纳管：DB 可见、SSOT 存在、scan 不再列出
        assert_eq!(svc.list_skills().unwrap().len(), 1);
        assert!(SkillService::get_ssot_dir()
            .unwrap()
            .join("orphan")
            .join("SKILL.md")
            .exists());
        let after = svc.scan_unmanaged().unwrap();
        assert!(!after.iter().any(|u| u.directory == "orphan"));

        match old {
            Some(v) => std::env::set_var("OLENRO_TEST_HOME", v),
            None => std::env::remove_var("OLENRO_TEST_HOME"),
        }
    }

    #[test]
    fn repo_default_and_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let svc = SkillService::new(dir.path().join("repos.db"));
        // 空 DB 返回默认仓库
        assert!(!svc.list_repos().unwrap().is_empty());
        svc.add_repo(&SkillRepo {
            owner: "me".into(),
            name: "mine".into(),
            branch: "dev".into(),
            enabled: true,
        })
        .unwrap();
        let repos = svc.list_repos().unwrap();
        assert!(repos
            .iter()
            .any(|r| r.owner == "me" && r.name == "mine" && r.branch == "dev"));
        svc.remove_repo("me", "mine").unwrap();
        assert!(!svc.list_repos().unwrap().iter().any(|r| r.owner == "me"));
    }
}
