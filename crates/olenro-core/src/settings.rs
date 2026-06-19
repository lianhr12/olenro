//! 设置访问层
//!
//! 由桌面端 `crate::settings` 移植。桌面端基于 settings.json 文件；CLI 端
//! 改为基于 SQLite 的键值表（`settings(key, value)`），但对外仍是**免参**的
//! free function，以匹配 `SkillService::get_ssot_dir()` 等静态方法的调用方式。
//!
//! 所有 getter 在数据库缺失/读取失败时静默回退到默认值，绝不 panic。
//! 持久化使用 CLI 数据库（[`crate::config::get_cli_database_path`]），与
//! `SkillService` 构造时传入的 db_path 在生产环境一致。

use crate::config::get_cli_database_path;
use crate::database::dao::SettingsDao;
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use rusqlite::Connection;

/// Skill 同步方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SyncMethod {
    /// 自动选择：优先 symlink，失败时回退到 copy。
    #[default]
    Auto,
    /// 符号链接（推荐，节省磁盘空间）。
    Symlink,
    /// 文件复制（兼容模式）。
    Copy,
}

impl SyncMethod {
    fn from_value(s: &str) -> Self {
        match s {
            "symlink" => SyncMethod::Symlink,
            "copy" => SyncMethod::Copy,
            _ => SyncMethod::Auto,
        }
    }

    fn as_value(&self) -> &'static str {
        match self {
            SyncMethod::Auto => "auto",
            SyncMethod::Symlink => "symlink",
            SyncMethod::Copy => "copy",
        }
    }
}

/// Skill 存储位置（SSOT 目录选择）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillStorageLocation {
    /// Olenro 管理目录 (~/.olenro/skills/)。
    #[default]
    CcSwitch,
    /// Agent Skills 统一标准目录 (~/.agents/skills/)。
    Unified,
}

impl SkillStorageLocation {
    fn from_value(s: &str) -> Self {
        match s {
            "unified" => SkillStorageLocation::Unified,
            _ => SkillStorageLocation::CcSwitch,
        }
    }

    fn as_value(&self) -> &'static str {
        match self {
            SkillStorageLocation::CcSwitch => "cc_switch",
            SkillStorageLocation::Unified => "unified",
        }
    }
}

// ========== 内部读写 ==========

fn open_settings_conn() -> AppResult<Connection> {
    let path = get_cli_database_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    Ok(conn)
}

/// 读取设置值，任何错误都回退为 None（不 panic）。
fn read_setting(key: &str) -> Option<String> {
    let conn = open_settings_conn().ok()?;
    SettingsDao::new(&conn).get(key).ok().flatten()
}

/// 写入设置值。
fn write_setting(key: &str, value: &str) -> AppResult<()> {
    let conn = open_settings_conn()?;
    SettingsDao::new(&conn).set(key, value)
}

// ========== 公共 API ==========

const KEY_STORAGE_LOCATION: &str = "skill_storage_location";
const KEY_SYNC_METHOD: &str = "skill_sync_method";

/// 读取 Skill 存储位置（默认 `CcSwitch`）。
pub fn get_skill_storage_location() -> SkillStorageLocation {
    read_setting(KEY_STORAGE_LOCATION)
        .map(|v| SkillStorageLocation::from_value(&v))
        .unwrap_or_default()
}

/// 持久化 Skill 存储位置。
pub fn set_skill_storage_location(location: SkillStorageLocation) -> AppResult<()> {
    write_setting(KEY_STORAGE_LOCATION, location.as_value())
}

/// 读取 Skill 同步方式（默认 `Auto`）。
pub fn get_skill_sync_method() -> SyncMethod {
    read_setting(KEY_SYNC_METHOD)
        .map(|v| SyncMethod::from_value(&v))
        .unwrap_or_default()
}

/// 持久化 Skill 同步方式。
pub fn set_skill_sync_method(method: SyncMethod) -> AppResult<()> {
    write_setting(KEY_SYNC_METHOD, method.as_value())
}

/// 读取某个应用的配置目录覆盖（设置项为空/缺失时返回 None）。
fn get_override_dir(key: &str) -> Option<PathBuf> {
    read_setting(key).and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(PathBuf::from(trimmed))
        }
    })
}

pub fn get_claude_override_dir() -> Option<PathBuf> {
    get_override_dir("claude_config_dir")
}

pub fn get_codex_override_dir() -> Option<PathBuf> {
    get_override_dir("codex_config_dir")
}

pub fn get_gemini_override_dir() -> Option<PathBuf> {
    get_override_dir("gemini_config_dir")
}

pub fn get_opencode_override_dir() -> Option<PathBuf> {
    get_override_dir("opencode_config_dir")
}

pub fn get_openclaw_override_dir() -> Option<PathBuf> {
    get_override_dir("openclaw_config_dir")
}

pub fn get_hermes_override_dir() -> Option<PathBuf> {
    get_override_dir("hermes_config_dir")
}
