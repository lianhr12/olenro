//! OpenClaw 工作区文件管理
//!
//! 由桌面端 `src-tauri/src/commands/workspace.rs` 移植而来。管理
//! `~/.openclaw/workspace/` 下的白名单 markdown 文件，以及
//! `~/.openclaw/workspace/memory/YYYY-MM-DD.md` 每日记忆文件。
//!
//! 相比桌面端去掉了 Tauri / opener 相关命令；文件名校验改为手写实现，
//! 不引入 `regex` 依赖。

use crate::app_config_writers::openclaw_config::get_openclaw_dir;
use crate::config::write_text_file;
use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// 允许的工作区文件名（安全白名单）。
pub const ALLOWED_FILES: &[&str] = &[
    "AGENTS.md",
    "SOUL.md",
    "USER.md",
    "IDENTITY.md",
    "TOOLS.md",
    "MEMORY.md",
    "HEARTBEAT.md",
    "BOOTSTRAP.md",
    "BOOT.md",
];

fn workspace_dir() -> PathBuf {
    get_openclaw_dir().join("workspace")
}

fn memory_dir() -> PathBuf {
    workspace_dir().join("memory")
}

fn validate_filename(filename: &str) -> AppResult<()> {
    if !ALLOWED_FILES.contains(&filename) {
        return Err(AppError::InvalidOperation(format!(
            "Invalid workspace filename: {filename}. Allowed: {}",
            ALLOWED_FILES.join(", ")
        )));
    }
    Ok(())
}

/// 校验每日记忆文件名格式 `YYYY-MM-DD.md`（手写，等价于正则 `^\d{4}-\d{2}-\d{2}\.md$`）。
fn validate_daily_memory_filename(filename: &str) -> AppResult<()> {
    let ok = is_daily_memory_filename(filename);
    if !ok {
        return Err(AppError::InvalidOperation(format!(
            "Invalid daily memory filename: {filename}. Expected: YYYY-MM-DD.md"
        )));
    }
    Ok(())
}

/// `YYYY-MM-DD.md` 形如 `2026-06-10.md`，共 13 个字符。
fn is_daily_memory_filename(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() != 13 {
        return false;
    }
    let digit = |b: u8| b.is_ascii_digit();
    digit(bytes[0])
        && digit(bytes[1])
        && digit(bytes[2])
        && digit(bytes[3])
        && bytes[4] == b'-'
        && digit(bytes[5])
        && digit(bytes[6])
        && bytes[7] == b'-'
        && digit(bytes[8])
        && digit(bytes[9])
        && &name[10..] == ".md"
}

/// 工作区文件信息（白名单文件，可能不存在）。
#[derive(Debug, Clone)]
pub struct WorkspaceFileInfo {
    pub filename: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub preview: String,
}

/// 每日记忆文件信息。
#[derive(Debug, Clone)]
pub struct DailyMemoryFileInfo {
    pub filename: String,
    pub date: String,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub preview: String,
}

/// 列出所有白名单工作区文件及其存在状态/预览。
pub fn list_workspace_files() -> Vec<WorkspaceFileInfo> {
    let dir = workspace_dir();
    ALLOWED_FILES
        .iter()
        .map(|name| {
            let path = dir.join(name);
            let (exists, size_bytes, preview) = match std::fs::metadata(&path) {
                Ok(meta) if meta.is_file() => {
                    let preview = std::fs::read_to_string(&path)
                        .unwrap_or_default()
                        .chars()
                        .take(200)
                        .collect::<String>();
                    (true, meta.len(), preview)
                }
                _ => (false, 0, String::new()),
            };
            WorkspaceFileInfo {
                filename: name.to_string(),
                exists,
                size_bytes,
                preview,
            }
        })
        .collect()
}

/// 读取工作区文件内容（不存在返回 None）。
pub fn read_workspace_file(filename: &str) -> AppResult<Option<String>> {
    validate_filename(filename)?;
    let path = workspace_dir().join(filename);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(std::fs::read_to_string(&path)?))
}

/// 原子写入工作区文件（自动创建目录）。
pub fn write_workspace_file(filename: &str, content: &str) -> AppResult<()> {
    validate_filename(filename)?;
    let dir = workspace_dir();
    std::fs::create_dir_all(&dir)?;
    write_text_file(&dir.join(filename), content)
}

/// 列出所有每日记忆文件，按日期倒序（最新在前）。
pub fn list_daily_memory_files() -> AppResult<Vec<DailyMemoryFileInfo>> {
    let dir = memory_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<DailyMemoryFileInfo> = Vec::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };

        let date = name.trim_end_matches(".md").to_string();
        let size_bytes = meta.len();
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let preview = std::fs::read_to_string(entry.path())
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect::<String>();

        files.push(DailyMemoryFileInfo {
            filename: name,
            date,
            size_bytes,
            modified_at,
            preview,
        });
    }

    files.sort_by(|a, b| b.filename.cmp(&a.filename));
    Ok(files)
}

/// 读取每日记忆文件（不存在返回 None）。
pub fn read_daily_memory_file(filename: &str) -> AppResult<Option<String>> {
    validate_daily_memory_filename(filename)?;
    let path = memory_dir().join(filename);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(std::fs::read_to_string(&path)?))
}

/// 原子写入每日记忆文件（自动创建目录）。
pub fn write_daily_memory_file(filename: &str, content: &str) -> AppResult<()> {
    validate_daily_memory_filename(filename)?;
    let dir = memory_dir();
    std::fs::create_dir_all(&dir)?;
    write_text_file(&dir.join(filename), content)
}

/// 删除每日记忆文件（幂等）。
pub fn delete_daily_memory_file(filename: &str) -> AppResult<()> {
    validate_daily_memory_filename(filename)?;
    let path = memory_dir().join(filename);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_memory_filename_validation() {
        assert!(is_daily_memory_filename("2026-06-10.md"));
        assert!(is_daily_memory_filename("0000-00-00.md"));
        assert!(!is_daily_memory_filename("2026-6-10.md"));
        assert!(!is_daily_memory_filename("2026-06-10.txt"));
        assert!(!is_daily_memory_filename("AGENTS.md"));
        assert!(!is_daily_memory_filename("2026-06-10.md.bak"));
    }

    #[test]
    fn workspace_filename_whitelist() {
        assert!(validate_filename("AGENTS.md").is_ok());
        assert!(validate_filename("SOUL.md").is_ok());
        assert!(validate_filename("../escape.md").is_err());
        assert!(validate_filename("random.md").is_err());
    }
}
