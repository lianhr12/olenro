//! 配置目录的 Git 同步 + 数据库备份/恢复
//!
//! 把 CLI 配置目录（`get_cli_config_dir()`，内含数据库与配置）作为一个 git 仓库
//! 进行版本化同步：status / init / push / pull，均通过 `git` CLI 实现（与桌面端
//! `services/git_sync.rs` 一致地 shell out 到 git）。另提供数据库文件的本地
//! 时间戳备份与恢复。
//!
//! 相比桌面端，这里去掉了 token 鉴权 / GitSyncSettings(DB) / asset catalog 等复杂
//! 逻辑，专注于本地 git 仓库 + 远程 push/pull 与备份恢复。

use crate::config::{get_cli_config_dir, get_cli_database_path};
use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};
use std::process::Command;

/// git-sync 操作的目标目录（CLI 配置目录）。
pub fn sync_dir() -> PathBuf {
    get_cli_config_dir()
}

/// git 仓库状态。
#[derive(Debug, Clone, Default)]
pub struct GitSyncStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub remote: Option<String>,
    pub dirty: bool,
}

/// 在 `dir` 下运行 `git <args>`，返回去除首尾空白的 stdout；非零退出或无法启动则报错。
fn run_git(dir: &Path, args: &[&str]) -> AppResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| AppError::GitSync(format!("failed to run git (is it installed?): {e}")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(AppError::GitSync(if stderr.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            stderr
        }))
    }
}

fn status_in(dir: &Path) -> GitSyncStatus {
    if !dir.join(".git").exists() {
        return GitSyncStatus::default();
    }
    let branch = run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|s| !s.is_empty());
    let remote = run_git(dir, &["remote", "get-url", "origin"])
        .ok()
        .filter(|s| !s.is_empty());
    let dirty = run_git(dir, &["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    GitSyncStatus {
        is_repo: true,
        branch,
        remote,
        dirty,
    }
}

/// 报告配置目录的 git 状态。
pub fn status() -> GitSyncStatus {
    status_in(&sync_dir())
}

fn init_in(dir: &Path, remote_url: Option<&str>, branch: &str) -> AppResult<()> {
    std::fs::create_dir_all(dir)?;
    if !dir.join(".git").exists() {
        run_git(dir, &["init"])?;
    }
    // 设置默认分支名。
    run_git(dir, &["checkout", "-B", branch])?;
    if let Some(url) = remote_url.filter(|u| !u.trim().is_empty()) {
        // 已有 origin 则更新，否则添加。
        if run_git(dir, &["remote", "get-url", "origin"]).is_ok() {
            run_git(dir, &["remote", "set-url", "origin", url])?;
        } else {
            run_git(dir, &["remote", "add", "origin", url])?;
        }
    }
    Ok(())
}

/// 初始化/配置 git 同步：在配置目录 git init、切到 `branch`、（可选）设置 origin。
pub fn init(remote_url: Option<&str>, branch: &str) -> AppResult<()> {
    init_in(&sync_dir(), remote_url, branch)
}

fn push_in(dir: &Path, message: &str) -> AppResult<String> {
    if !dir.join(".git").exists() {
        return Err(AppError::GitSync(
            "not a git repo — run init first".to_string(),
        ));
    }
    run_git(dir, &["add", "-A"])?;
    // 没有变更时 commit 会失败；先判断是否有暂存内容。
    let staged = run_git(dir, &["status", "--porcelain"])?;
    if !staged.is_empty() {
        run_git(dir, &["commit", "-m", message])?;
    }
    let branch = run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    // 仅当配置了 origin 时才 push。
    if run_git(dir, &["remote", "get-url", "origin"]).is_ok() {
        run_git(dir, &["push", "-u", "origin", &branch])?;
        Ok(format!("pushed to origin/{branch}"))
    } else {
        Ok(format!("committed on {branch} (no remote configured)"))
    }
}

/// 提交配置目录的全部变更并（若有 origin）推送。
pub fn push(message: &str) -> AppResult<String> {
    push_in(&sync_dir(), message)
}

fn pull_in(dir: &Path) -> AppResult<String> {
    if !dir.join(".git").exists() {
        return Err(AppError::GitSync(
            "not a git repo — run init first".to_string(),
        ));
    }
    if run_git(dir, &["remote", "get-url", "origin"]).is_err() {
        return Err(AppError::GitSync("no remote configured".to_string()));
    }
    let branch = run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    run_git(dir, &["pull", "origin", &branch])
}

/// 从 origin 拉取当前分支。
pub fn pull() -> AppResult<String> {
    pull_in(&sync_dir())
}

// ============================================================================
// 数据库备份 / 恢复
// ============================================================================

/// 备份存放目录（`<cli_config>/backups`）。
fn backups_dir() -> PathBuf {
    get_cli_config_dir().join("backups")
}

/// 一个备份条目。
#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: u64,
}

/// 备份当前 CLI 数据库到时间戳文件，返回备份路径。
pub fn create_backup() -> AppResult<PathBuf> {
    let db = get_cli_database_path();
    if !db.exists() {
        return Err(AppError::GitSync("no database to back up yet".to_string()));
    }
    let dir = backups_dir();
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let mut path = dir.join(format!("olenro_cli_{stamp}.db"));
    let mut counter = 1;
    while path.exists() {
        path = dir.join(format!("olenro_cli_{stamp}_{counter}.db"));
        counter += 1;
    }
    std::fs::copy(&db, &path)?;
    Ok(path)
}

/// 列出全部数据库备份，按时间倒序（最新在前）。
pub fn list_backups() -> Vec<BackupEntry> {
    let dir = backups_dir();
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(&dir) {
        for e in read.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("db") {
                continue;
            }
            let meta = match e.metadata() {
                Ok(m) if m.is_file() => m,
                _ => continue,
            };
            let modified_at = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            entries.push(BackupEntry {
                filename: e.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                size_bytes: meta.len(),
                modified_at,
            });
        }
    }
    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    entries
}

/// 从备份恢复数据库（覆盖当前 db；恢复前先自动备份当前 db）。
pub fn restore_backup(filename: &str) -> AppResult<()> {
    // 防止路径穿越。
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(AppError::GitSync(format!(
            "invalid backup filename: {filename}"
        )));
    }
    let src = backups_dir().join(filename);
    if !src.exists() {
        return Err(AppError::GitSync(format!("backup not found: {filename}")));
    }
    let db = get_cli_database_path();
    // 恢复前先快照当前 db（若存在），避免误操作丢数据。
    if db.exists() {
        let _ = create_backup();
    }
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&src, &db)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_on_non_repo_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let s = status_in(tmp.path());
        assert!(!s.is_repo);
        assert!(s.branch.is_none());
        assert!(s.remote.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn backup_and_restore_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let old = std::env::var_os("OLENRO_CLI_HOME");
        std::env::set_var("OLENRO_CLI_HOME", tmp.path());

        // 写一个假的 db 文件。
        let db = get_cli_database_path();
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        std::fs::write(&db, b"original").unwrap();

        let backup = create_backup().unwrap();
        assert!(backup.exists());
        assert_eq!(list_backups().len(), 1);

        // 改动 db，再恢复。
        std::fs::write(&db, b"changed").unwrap();
        let name = backup.file_name().unwrap().to_string_lossy().to_string();
        restore_backup(&name).unwrap();
        assert_eq!(std::fs::read(&db).unwrap(), b"original");

        // restore 前会自动快照 changed 版本，故现在应有 2 个备份。
        assert_eq!(list_backups().len(), 2);

        assert!(restore_backup("../evil.db").is_err());

        match old {
            Some(v) => std::env::set_var("OLENRO_CLI_HOME", v),
            None => std::env::remove_var("OLENRO_CLI_HOME"),
        }
    }
}
