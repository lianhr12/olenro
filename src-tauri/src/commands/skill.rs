//! Skills 命令层
//!
//! v3.10.0+ 统一管理架构：
//! - 支持三应用开关（Claude/Codex/Gemini）
//! - SSOT 存储在 ~/.olenro/skills/

use crate::app_config::{AppType, InstalledSkill, UnmanagedSkill};
use crate::error::format_skill_error;
use crate::services::skill::{
    DiscoverableSkill, ImportSkillSelection, MigrationResult, Skill, SkillBackupEntry, SkillRepo,
    SkillService, SkillStorageLocation, SkillUninstallResult, SkillUpdateInfo,
    SkillsShSearchResult,
};
use crate::store::AppState;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

/// SkillService 状态包装
pub struct SkillServiceState(pub Arc<SkillService>);

/// 解析 app 参数为 AppType
fn parse_app_type(app: &str) -> Result<AppType, String> {
    AppType::from_str(app).map_err(|e| e.to_string())
}

// ========== 统一管理命令 ==========

/// 获取所有已安装的 Skills
#[tauri::command]
pub fn get_installed_skills(app_state: State<'_, AppState>) -> Result<Vec<InstalledSkill>, String> {
    SkillService::get_all_installed(&app_state.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_skill_backups() -> Result<Vec<SkillBackupEntry>, String> {
    SkillService::list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_skill_backup(backup_id: String) -> Result<bool, String> {
    SkillService::delete_backup(&backup_id).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 安装 Skill（新版统一安装）
///
/// 参数：
/// - skill: 从发现列表获取的技能信息
/// - current_app: 当前选中的应用，安装后默认启用该应用
#[tauri::command]
pub async fn install_skill_unified(
    skill: DiscoverableSkill,
    current_app: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<InstalledSkill, String> {
    let app_type = parse_app_type(&current_app)?;

    service
        .0
        .install(&app_state.db, &skill, &app_type)
        .await
        .map_err(|e| e.to_string())
}

/// 卸载 Skill（新版统一卸载）
#[tauri::command]
pub fn uninstall_skill_unified(
    id: String,
    app_state: State<'_, AppState>,
) -> Result<SkillUninstallResult, String> {
    SkillService::uninstall(&app_state.db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_skill_backup(
    backup_id: String,
    current_app: String,
    app_state: State<'_, AppState>,
) -> Result<InstalledSkill, String> {
    let app_type = parse_app_type(&current_app)?;
    SkillService::restore_from_backup(&app_state.db, &backup_id, &app_type)
        .map_err(|e| e.to_string())
}

/// 切换 Skill 的应用启用状态
#[tauri::command]
pub fn toggle_skill_app(
    id: String,
    app: String,
    enabled: bool,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let app_type = parse_app_type(&app)?;
    SkillService::toggle_app(&app_state.db, &id, &app_type, enabled).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 扫描未管理的 Skills
#[tauri::command]
pub fn scan_unmanaged_skills(
    app_state: State<'_, AppState>,
) -> Result<Vec<UnmanagedSkill>, String> {
    SkillService::scan_unmanaged(&app_state.db).map_err(|e| e.to_string())
}

/// 从应用目录导入 Skills
#[tauri::command]
pub fn import_skills_from_apps(
    imports: Vec<ImportSkillSelection>,
    app_state: State<'_, AppState>,
) -> Result<Vec<InstalledSkill>, String> {
    SkillService::import_from_apps(&app_state.db, imports).map_err(|e| e.to_string())
}

// ========== 发现功能命令 ==========

/// 发现可安装的 Skills（从仓库获取）
#[tauri::command]
pub async fn discover_available_skills(
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<DiscoverableSkill>, String> {
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    service
        .0
        .discover_available(repos)
        .await
        .map_err(|e| e.to_string())
}

/// 检查 Skills 更新
#[tauri::command]
pub async fn check_skill_updates(
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<SkillUpdateInfo>, String> {
    service
        .0
        .check_updates(&app_state.db)
        .await
        .map_err(|e| e.to_string())
}

/// 更新单个 Skill
#[tauri::command]
pub async fn update_skill(
    id: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<InstalledSkill, String> {
    service
        .0
        .update_skill(&app_state.db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 迁移 Skill 存储位置
#[tauri::command]
pub async fn migrate_skill_storage(
    target: SkillStorageLocation,
    app_state: State<'_, AppState>,
) -> Result<MigrationResult, String> {
    SkillService::migrate_storage(&app_state.db, target).map_err(|e| e.to_string())
}

/// 搜索 skills.sh 公共目录
#[tauri::command]
pub async fn search_skills_sh(
    query: String,
    limit: usize,
    offset: usize,
) -> Result<SkillsShSearchResult, String> {
    SkillService::search_skills_sh(&query, limit, offset)
        .await
        .map_err(|e| e.to_string())
}

/// 获取 skills.sh 热门技能（按安装量倒序）
#[tauri::command]
pub async fn get_popular_skills_sh(limit: usize) -> Result<SkillsShSearchResult, String> {
    SkillService::get_popular_skills_sh(limit)
        .await
        .map_err(|e| e.to_string())
}

// ========== 兼容旧 API 的命令 ==========

/// 获取技能列表（兼容旧 API）
#[tauri::command]
pub async fn get_skills(
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<Skill>, String> {
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    service
        .0
        .list_skills(repos, &app_state.db)
        .await
        .map_err(|e| e.to_string())
}

/// 获取指定应用的技能列表（兼容旧 API）
#[tauri::command]
pub async fn get_skills_for_app(
    app: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<Skill>, String> {
    // 新版本不再区分应用，统一返回所有技能
    let _ = parse_app_type(&app)?; // 验证 app 参数有效
    get_skills(service, app_state).await
}

/// 安装技能（兼容旧 API）
#[tauri::command]
pub async fn install_skill(
    directory: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    install_skill_for_app("claude".to_string(), directory, service, app_state).await
}

/// 安装指定应用的技能（兼容旧 API）
#[tauri::command]
pub async fn install_skill_for_app(
    app: String,
    directory: String,
    service: State<'_, SkillServiceState>,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let app_type = parse_app_type(&app)?;

    // 先获取技能信息
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    let skills = service
        .0
        .discover_available(repos)
        .await
        .map_err(|e| e.to_string())?;

    let skill = skills
        .into_iter()
        .find(|s| {
            let install_name = std::path::Path::new(&s.directory)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| s.directory.clone());
            install_name.eq_ignore_ascii_case(&directory)
                || s.directory.eq_ignore_ascii_case(&directory)
        })
        .ok_or_else(|| {
            format_skill_error(
                "SKILL_NOT_FOUND",
                &[("directory", &directory)],
                Some("checkRepoUrl"),
            )
        })?;

    service
        .0
        .install(&app_state.db, &skill, &app_type)
        .await
        .map_err(|e| e.to_string())?;

    Ok(true)
}

/// 卸载技能（兼容旧 API）
#[tauri::command]
pub fn uninstall_skill(
    directory: String,
    app_state: State<'_, AppState>,
) -> Result<SkillUninstallResult, String> {
    uninstall_skill_for_app("claude".to_string(), directory, app_state)
}

/// 卸载指定应用的技能（兼容旧 API）
#[tauri::command]
pub fn uninstall_skill_for_app(
    app: String,
    directory: String,
    app_state: State<'_, AppState>,
) -> Result<SkillUninstallResult, String> {
    let _ = parse_app_type(&app)?; // 验证参数

    // 通过 directory 找到对应的 skill id
    let skills = SkillService::get_all_installed(&app_state.db).map_err(|e| e.to_string())?;

    let skill = skills
        .into_iter()
        .find(|s| s.directory.eq_ignore_ascii_case(&directory))
        .ok_or_else(|| format!("未找到已安装的 Skill: {directory}"))?;

    SkillService::uninstall(&app_state.db, &skill.id).map_err(|e| e.to_string())
}

// ========== 仓库管理命令 ==========

/// 获取技能仓库列表
#[tauri::command]
pub fn get_skill_repos(app_state: State<'_, AppState>) -> Result<Vec<SkillRepo>, String> {
    app_state.db.get_skill_repos().map_err(|e| e.to_string())
}

/// 添加技能仓库
#[tauri::command]
pub fn add_skill_repo(repo: SkillRepo, app_state: State<'_, AppState>) -> Result<bool, String> {
    app_state
        .db
        .save_skill_repo(&repo)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 删除技能仓库
#[tauri::command]
pub fn remove_skill_repo(
    owner: String,
    name: String,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    app_state
        .db
        .delete_skill_repo(&owner, &name)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 从 ZIP 文件安装 Skills
#[tauri::command]
pub fn install_skills_from_zip(
    file_path: String,
    current_app: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<InstalledSkill>, String> {
    let app_type = parse_app_type(&current_app)?;
    let path = std::path::Path::new(&file_path);

    SkillService::install_from_zip(&app_state.db, path, &app_type).map_err(|e| e.to_string())
}

// ========== Skill 详情查看命令 ==========

/// Skill 文件信息
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillFileInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub modified_at: u64,
}

/// Skill 完整详情
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub directory: String,
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
    pub repo_branch: Option<String>,
    pub readme_url: Option<String>,
    pub apps: crate::app_config::SkillApps,
    pub installed_at: i64,
    pub updated_at: i64,
    pub content_hash: Option<String>,
    pub manifest_content: Option<String>,
    pub files: Vec<SkillFileInfo>,
}

/// 解析 Skill 的真实文件目录
///
/// Skill 文件可能位于多个位置（SSOT、各应用目录、软链接目标）。
/// 按优先级依次检查候选位置，返回第一个存在且包含文件的目录。
/// 找不到则返回 None（例如数据库中的孤立记录，物理文件已被删除）。
fn resolve_skill_dir(skill: &InstalledSkill) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. SSOT 目录（首选）
    if let Ok(ssot) = SkillService::get_ssot_dir() {
        candidates.push(ssot.join(&skill.directory));
    }

    // 2. 各应用的 skills 目录（按启用状态优先）
    for app in AppType::all() {
        if let Ok(dir) = SkillService::get_app_skills_dir(&app) {
            candidates.push(dir.join(&skill.directory));
        }
    }

    // 3. ~/.agents/skills 目录
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".agents").join("skills").join(&skill.directory));
    }

    for candidate in candidates {
        // 跟随软链接解析真实路径
        if let Ok(resolved) = candidate.canonicalize() {
            if resolved.is_dir() {
                return Some(resolved);
            }
        }
    }

    None
}

/// 递归收集目录下的所有文件
fn collect_files_recursive(
    base: &Path,
    current: &Path,
    files: &mut Vec<SkillFileInfo>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(current).map_err(|e| format!("Failed to read directory: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // 跳过隐藏文件
        if name.starts_with('.') {
            continue;
        }

        let relative_path = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size_bytes = if meta.is_file() { meta.len() } else { 0 };
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        files.push(SkillFileInfo {
            path: relative_path,
            name,
            size_bytes,
            is_dir: meta.is_dir(),
            modified_at,
        });

        // 递归进入子目录
        if meta.is_dir() {
            collect_files_recursive(base, &path, files)?;
        }
    }

    Ok(())
}

/// 获取 Skill 完整详情（包含 manifest 内容和文件列表）
#[tauri::command]
pub fn get_skill_detail(
    skill_id: String,
    app_state: State<'_, AppState>,
) -> Result<SkillDetail, String> {
    let skill = app_state
        .db
        .get_installed_skill(&skill_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    // 解析真实文件目录（可能不存在，例如孤立的数据库记录）
    let skill_path = resolve_skill_dir(&skill);

    // 读取 manifest 内容
    let manifest_content = skill_path.as_ref().and_then(|dir| {
        let manifest_path = dir.join("SKILL.md");
        if manifest_path.exists() {
            std::fs::read_to_string(&manifest_path).ok()
        } else {
            None
        }
    });

    // 收集所有文件
    let mut files = Vec::new();
    if let Some(dir) = &skill_path {
        // 忽略收集错误，保证详情仍能展示元数据
        let _ = collect_files_recursive(dir, dir, &mut files);
    }

    // 按路径排序
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(SkillDetail {
        id: skill.id,
        name: skill.name,
        description: skill.description,
        directory: skill.directory,
        repo_owner: skill.repo_owner,
        repo_name: skill.repo_name,
        repo_branch: skill.repo_branch,
        readme_url: skill.readme_url,
        apps: skill.apps,
        installed_at: skill.installed_at,
        updated_at: skill.updated_at,
        content_hash: skill.content_hash,
        manifest_content,
        files,
    })
}

/// 列出 Skill 目录下的所有文件
#[tauri::command]
pub fn list_skill_files(
    skill_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<SkillFileInfo>, String> {
    let skill = app_state
        .db
        .get_installed_skill(&skill_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    let skill_path = resolve_skill_dir(&skill)
        .ok_or_else(|| format!("Skill files not found for: {}", skill.directory))?;

    let mut files = Vec::new();
    collect_files_recursive(&skill_path, &skill_path, &mut files)?;

    // 按路径排序
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(files)
}

/// 验证路径在 Skill 目录内（防止路径遍历攻击）
fn validate_skill_file_path(skill_dir: &Path, requested_path: &str) -> Result<PathBuf, String> {
    // 规范化路径
    let requested = Path::new(requested_path);
    let full_path = skill_dir.join(requested);

    // 使用 canonicalize 来解析真实路径
    let canonical_skill = skill_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve skill directory: {e}"))?;

    // 检查路径是否存在
    if !full_path.exists() {
        return Err(format!("Path does not exist: {}", requested_path));
    }

    let canonical_full = full_path
        .canonicalize()
        .map_err(|_| format!("Path does not exist or is invalid: {}", requested_path))?;

    // 确保完整路径在技能目录内
    if !canonical_full.starts_with(&canonical_skill) {
        return Err("Access denied: path outside skill directory".to_string());
    }

    Ok(canonical_full)
}

/// 读取 Skill 文件内容
#[tauri::command]
pub fn read_skill_file(
    skill_id: String,
    file_path: String,
    app_state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let skill = app_state
        .db
        .get_installed_skill(&skill_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    let skill_dir = resolve_skill_dir(&skill)
        .ok_or_else(|| format!("Skill files not found for: {}", skill.directory))?;

    // 安全验证路径
    let full_path = validate_skill_file_path(&skill_dir, &file_path)?;

    // 文件大小限制：1MB
    let meta = std::fs::metadata(&full_path)
        .map_err(|e| format!("Failed to read file metadata: {e}"))?;
    if meta.len() > 1024 * 1024 {
        return Err("File too large (>1MB)".to_string());
    }

    if meta.is_dir() {
        return Err("Cannot read directory".to_string());
    }

    std::fs::read_to_string(&full_path)
        .map(Some)
        .map_err(|e| format!("Failed to read file: {e}"))
}

/// 写入 Skill 文件内容
#[tauri::command]
pub fn write_skill_file(
    skill_id: String,
    file_path: String,
    content: String,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let skill = app_state
        .db
        .get_installed_skill(&skill_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    let skill_dir = resolve_skill_dir(&skill)
        .ok_or_else(|| format!("Skill files not found for: {}", skill.directory))?;

    // 安全验证路径
    let full_path = validate_skill_file_path(&skill_dir, &file_path)?;

    // 内容大小限制：1MB
    if content.len() > 1024 * 1024 {
        return Err("Content too large (>1MB)".to_string());
    }

    // 检查是否为文件
    if !full_path.is_file() {
        return Err("Cannot write to directory".to_string());
    }

    std::fs::write(&full_path, content).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(true)
}

/// 在系统文件管理器中打开 Skill 目录
#[tauri::command]
pub async fn open_skill_directory(
    handle: AppHandle,
    skill_id: String,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let skill = app_state
        .db
        .get_installed_skill(&skill_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())?;

    let skill_path = resolve_skill_dir(&skill)
        .ok_or_else(|| format!("Skill files not found for: {}", skill.directory))?;

    handle
        .opener()
        .open_path(skill_path.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| format!("Failed to open directory: {e}"))?;

    Ok(true)
}
