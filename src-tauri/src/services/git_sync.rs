//! Git 同步服务层（取代 WebDAV）。
//!
//! 把 AI agent 资产以**明文目录结构**提交到一个私有 Git 仓库，
//! 通过 HTTPS + PAT 鉴权、手动推/拉同步，支持跨机器/跨平台恢复。
//!
//! 仓库工作区布局（明文，可在 GitHub 上浏览/diff）：
//! ```text
//! repo/
//! ├── manifest.json        # 协议版本 / 设备名 / 更新时间
//! ├── db.sql               # 无损权威备份（providers / MCP / skill 状态）
//! ├── providers/<app>.json # 各应用供应商配置的可读镜像
//! └── skills/<skill>/...   # SSOT 技能内容（取代 skills.zip）
//! ```
//!
//! 设计要点：
//! - 传输层调用系统 `git` 二进制；PAT 通过 `GIT_ASKPASS` 注入，不进入 argv / reflog。
//! - 推送 = 本地权威：检测到远端领先（diverged）时返回特殊结果，由 UI 让用户选择
//!   「强制用本地覆盖」或「改为拉取」。
//! - 拉取 = 远端权威：`reset --hard origin/<branch>` 后从工作区导入并重新分发。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use chrono::Utc;
use serde_json::{json, Value};

use crate::app_config::AppType;
use crate::config::{atomic_write, get_app_config_dir, get_home_dir};
use crate::database::Database;
use crate::error::AppError;
use crate::services::skill::SkillService;
use crate::settings::{GitSyncSettings, GitSyncStatus};

// ─── 常量 ────────────────────────────────────────────────────

const PROTOCOL_FORMAT: &str = "cc-switch-git-sync";
const PROTOCOL_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "manifest.json";
const DB_FILE: &str = "db.sql";
const PROVIDERS_DIR: &str = "providers";
const SKILLS_DIR: &str = "skills";
const MCP_DIR: &str = "mcp";
const MCP_FILE: &str = "servers.json";
const AGENTS_DIR: &str = "agents";
const COMMIT_PREFIX: &str = "cc-switch sync";

// ─── Agent 资产清单 ──────────────────────────────────────────
//
// 各工具的配置资产，仅做**跨机镜像**（备份/恢复），不做跨工具格式转换。
// `home_rel` 相对用户主目录；`repo_rel` 相对仓库的 `agents/` 目录。

/// 单个可同步的 agent 资产。
struct AssetSpec {
    /// 稳定 ID（前端用于勾选 / i18n）
    id: &'static str,
    /// 相对用户主目录的源路径
    home_rel: &'static str,
    /// 相对仓库 `agents/` 的存放路径
    repo_rel: &'static str,
    /// 是否为目录
    is_dir: bool,
}

/// 默认追踪的 agent 资产清单（settings/hooks、subagents、slash commands、记忆文件）。
const AGENT_ASSETS: &[AssetSpec] = &[
    // Claude Code：settings.json 内含 hooks
    AssetSpec {
        id: "claude.settings",
        home_rel: ".claude/settings.json",
        repo_rel: "claude/settings.json",
        is_dir: false,
    },
    AssetSpec {
        id: "claude.agents",
        home_rel: ".claude/agents",
        repo_rel: "claude/agents",
        is_dir: true,
    },
    AssetSpec {
        id: "claude.commands",
        home_rel: ".claude/commands",
        repo_rel: "claude/commands",
        is_dir: true,
    },
    AssetSpec {
        id: "claude.memory",
        home_rel: ".claude/CLAUDE.md",
        repo_rel: "claude/CLAUDE.md",
        is_dir: false,
    },
    AssetSpec {
        id: "claude.outputStyles",
        home_rel: ".claude/output-styles",
        repo_rel: "claude/output-styles",
        is_dir: true,
    },
    // Codex
    AssetSpec {
        id: "codex.memory",
        home_rel: ".codex/AGENTS.md",
        repo_rel: "codex/AGENTS.md",
        is_dir: false,
    },
    AssetSpec {
        id: "codex.prompts",
        home_rel: ".codex/prompts",
        repo_rel: "codex/prompts",
        is_dir: true,
    },
    // Gemini CLI
    AssetSpec {
        id: "gemini.memory",
        home_rel: ".gemini/GEMINI.md",
        repo_rel: "gemini/GEMINI.md",
        is_dir: false,
    },
    AssetSpec {
        id: "gemini.commands",
        home_rel: ".gemini/commands",
        repo_rel: "gemini/commands",
        is_dir: true,
    },
];

/// 资产目录中应忽略的易变/隐私内容（不进仓库）。
const AGENT_ASSET_IGNORE: &[&str] = &[".git", ".DS_Store", "node_modules"];

/// 返回 agent 资产清单（供前端勾选）。
pub fn asset_catalog() -> Vec<Value> {
    AGENT_ASSETS
        .iter()
        .map(|a| {
            let (tool, _) = a.id.split_once('.').unwrap_or((a.id, ""));
            json!({
                "id": a.id,
                "tool": tool,
                "homePath": format!("~/{}", a.home_rel),
                "isDir": a.is_dir,
            })
        })
        .collect()
}

// ─── 同步锁 ──────────────────────────────────────────────────

pub fn sync_mutex() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

pub async fn run_with_sync_lock<T, Fut>(operation: Fut) -> Result<T, AppError>
where
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    let _guard = sync_mutex().lock().await;
    operation.await
}

// ─── 错误辅助 ────────────────────────────────────────────────

fn localized(key: &'static str, zh: impl Into<String>, en: impl Into<String>) -> AppError {
    AppError::localized(key, zh, en)
}

// ─── 路径辅助 ────────────────────────────────────────────────

/// 本地仓库克隆目录：`~/.olenro/git-sync`
fn local_repo_dir() -> PathBuf {
    get_app_config_dir().join("git-sync")
}

/// GIT_ASKPASS 脚本目录（放在仓库之外，避免被 `git clean` 删除）
fn askpass_dir() -> PathBuf {
    get_app_config_dir().join(".git-askpass")
}

// ─── 设备名 ──────────────────────────────────────────────────

fn detect_device_name(settings: &GitSyncSettings) -> String {
    if !settings.device_name.trim().is_empty() {
        return settings.device_name.trim().to_string();
    }
    ["HOSTNAME", "COMPUTERNAME", "HOST"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "Unknown Device".to_string())
}

// ─── git 调用 ────────────────────────────────────────────────

/// 准备 GIT_ASKPASS 脚本，返回脚本路径。脚本读取环境变量 `CC_SWITCH_GIT_TOKEN` 并输出。
fn prepare_askpass() -> Result<PathBuf, AppError> {
    let dir = askpass_dir();
    fs::create_dir_all(&dir).map_err(|e| AppError::io(&dir, e))?;

    #[cfg(windows)]
    {
        let path = dir.join("askpass.bat");
        let script = "@echo off\r\necho %CC_SWITCH_GIT_TOKEN%\r\n";
        atomic_write(&path, script.as_bytes())?;
        Ok(path)
    }

    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("askpass.sh");
        let script = "#!/bin/sh\nprintf '%s' \"$CC_SWITCH_GIT_TOKEN\"\n";
        atomic_write(&path, script.as_bytes())?;
        let mut perms = fs::metadata(&path)
            .map_err(|e| AppError::io(&path, e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).map_err(|e| AppError::io(&path, e))?;
        Ok(path)
    }
}

/// 在远程 HTTPS URL 中嵌入用户名（git 因此只会向 askpass 询问 password=token）。
fn url_with_username(settings: &GitSyncSettings) -> String {
    let url = settings.repo_url.trim();
    let user = if settings.username.trim().is_empty() {
        "x-access-token"
    } else {
        settings.username.trim()
    };
    for scheme in ["https://", "http://"] {
        if let Some(rest) = url.strip_prefix(scheme) {
            // 若已包含 '@'，不重复注入
            if rest.contains('@') {
                return url.to_string();
            }
            return format!("{scheme}{user}@{rest}");
        }
    }
    url.to_string()
}

struct GitRunner {
    askpass: PathBuf,
    token: String,
}

impl GitRunner {
    fn new(settings: &GitSyncSettings) -> Result<Self, AppError> {
        Ok(Self {
            askpass: prepare_askpass()?,
            token: settings.token.clone(),
        })
    }

    fn command(&self, cwd: Option<&Path>) -> Command {
        let mut cmd = Command::new("git");
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.env("GIT_ASKPASS", &self.askpass)
            .env("SSH_ASKPASS", &self.askpass)
            .env("CC_SWITCH_GIT_TOKEN", &self.token)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never");
        cmd
    }

    /// 执行 git，成功返回 stdout，失败返回带 stderr 的错误。
    fn run(&self, cwd: Option<&Path>, args: &[&str]) -> Result<String, AppError> {
        let output = self.command(cwd).args(args).output().map_err(|e| {
            localized(
                "git.spawn_failed",
                format!("无法执行 git（请确认系统已安装 git）：{e}"),
                format!("Failed to run git (is git installed?): {e}"),
            )
        })?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(localized(
                "git.command_failed",
                format!("git {} 失败：{stderr}", args.join(" ")),
                format!("git {} failed: {stderr}", args.join(" ")),
            ))
        }
    }

    /// 执行 git，仅返回是否成功（用于 ancestor / rev-parse 等判定）。
    fn check(&self, cwd: Option<&Path>, args: &[&str]) -> bool {
        self.command(cwd)
            .args(args)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

// ─── 仓库初始化 ──────────────────────────────────────────────

/// 打开本地仓库；不存在则初始化，并把 origin 指向远端。
fn ensure_repo(runner: &GitRunner, settings: &GitSyncSettings) -> Result<PathBuf, AppError> {
    let dir = local_repo_dir();
    let git_dir = dir.join(".git");
    if !git_dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| AppError::io(&dir, e))?;
        // 尽量用 -b 指定初始分支；旧版 git 不支持时回退到 symbolic-ref。
        if runner
            .run(Some(&dir), &["init", "-b", settings.branch.as_str()])
            .is_err()
        {
            runner.run(Some(&dir), &["init"])?;
            let head_ref = format!("refs/heads/{}", settings.branch);
            runner.run(Some(&dir), &["symbolic-ref", "HEAD", head_ref.as_str()])?;
        }
    }

    // 重置 origin，确保 URL（含用户名）为最新。
    let url = url_with_username(settings);
    let _ = runner.run(Some(&dir), &["remote", "remove", "origin"]);
    runner.run(Some(&dir), &["remote", "add", "origin", url.as_str()])?;

    Ok(dir)
}

// ─── 资产导出（DB / providers / skills）→ 工作区 ─────────────

fn write_manifest(repo: &Path, settings: &GitSyncSettings) -> Result<(), AppError> {
    let manifest = json!({
        "format": PROTOCOL_FORMAT,
        "version": PROTOCOL_VERSION,
        "deviceName": detect_device_name(settings),
        "updatedAt": Utc::now().to_rfc3339(),
        "app": "cc-switch",
    });
    let path = repo.join(MANIFEST_FILE);
    let body = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| AppError::Config(format!("序列化 manifest 失败: {e}")))?;
    atomic_write(&path, &body)
}

/// 把每个应用的供应商配置导出为可读 JSON 镜像（仅供浏览/diff，恢复以 db.sql 为准）。
fn write_provider_mirror(db: &Database, repo: &Path) -> Result<(), AppError> {
    let providers_dir = repo.join(PROVIDERS_DIR);
    if providers_dir.exists() {
        fs::remove_dir_all(&providers_dir).map_err(|e| AppError::io(&providers_dir, e))?;
    }
    for app in AppType::all() {
        let map = match db.get_all_providers(app.as_str()) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if map.is_empty() {
            continue;
        }
        fs::create_dir_all(&providers_dir).map_err(|e| AppError::io(&providers_dir, e))?;
        let path = providers_dir.join(format!("{}.json", app.as_str()));
        let body = serde_json::to_vec_pretty(&map)
            .map_err(|e| AppError::Config(format!("序列化供应商配置失败: {e}")))?;
        atomic_write(&path, &body)?;
    }
    Ok(())
}

/// 把 SSOT 技能目录复制到仓库 `skills/`（明文，取代 skills.zip）。
fn write_skills_mirror(repo: &Path) -> Result<(), AppError> {
    let dest = repo.join(SKILLS_DIR);
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| AppError::io(&dest, e))?;
    }
    let ssot = SkillService::get_ssot_dir().map_err(|e| {
        localized(
            "git.skills_ssot_failed",
            format!("获取 Skills SSOT 目录失败: {e}"),
            format!("Failed to resolve Skills SSOT directory: {e}"),
        )
    })?;
    if ssot.exists() {
        copy_dir_follow_symlinks(&ssot, &dest)?;
    }
    Ok(())
}

/// 把 MCP 服务器定义导出为可读 JSON 镜像（仅供浏览/diff，恢复以 db.sql 为准）。
fn write_mcp_mirror(db: &Database, repo: &Path) -> Result<(), AppError> {
    let mcp_dir = repo.join(MCP_DIR);
    let map = match db.get_all_mcp_servers() {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };
    if map.is_empty() {
        if mcp_dir.exists() {
            fs::remove_dir_all(&mcp_dir).map_err(|e| AppError::io(&mcp_dir, e))?;
        }
        return Ok(());
    }
    fs::create_dir_all(&mcp_dir).map_err(|e| AppError::io(&mcp_dir, e))?;
    let body = serde_json::to_vec_pretty(&map)
        .map_err(|e| AppError::Config(format!("序列化 MCP 配置失败: {e}")))?;
    atomic_write(&mcp_dir.join(MCP_FILE), &body)
}

/// 是否启用某资产（不在 disabled 列表即启用）。
fn asset_enabled(settings: &GitSyncSettings, id: &str) -> bool {
    !settings.disabled_assets.iter().any(|d| d == id)
}

/// 导出各工具的 agent 资产到仓库 `agents/`（仅镜像，不转换）。
fn export_agent_assets(repo: &Path, settings: &GitSyncSettings) -> Result<(), AppError> {
    let agents_root = repo.join(AGENTS_DIR);
    // 整体重建以反映被删除/关闭的资产
    if agents_root.exists() {
        fs::remove_dir_all(&agents_root).map_err(|e| AppError::io(&agents_root, e))?;
    }
    let home = get_home_dir();
    for spec in AGENT_ASSETS {
        if !asset_enabled(settings, spec.id) {
            continue;
        }
        let src = home.join(spec.home_rel);
        if !src.exists() {
            continue;
        }
        let dest = agents_root.join(spec.repo_rel);
        if spec.is_dir {
            copy_dir_follow_symlinks(&src, &dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
            }
            fs::copy(&src, &dest).map_err(|e| AppError::io(&dest, e))?;
        }
    }
    Ok(())
}

/// 从仓库 `agents/` 恢复各工具的 agent 资产（加性恢复：仓库没有的不删除本地）。
fn import_agent_assets(repo: &Path, settings: &GitSyncSettings) -> Result<(), AppError> {
    let agents_root = repo.join(AGENTS_DIR);
    if !agents_root.exists() {
        return Ok(());
    }
    let home = get_home_dir();
    for spec in AGENT_ASSETS {
        if !asset_enabled(settings, spec.id) {
            continue;
        }
        let src = agents_root.join(spec.repo_rel);
        if !src.exists() {
            continue;
        }
        let dest = home.join(spec.home_rel);
        if spec.is_dir {
            if dest.exists() {
                fs::remove_dir_all(&dest).map_err(|e| AppError::io(&dest, e))?;
            }
            copy_dir_follow_symlinks(&src, &dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
            }
            fs::copy(&src, &dest).map_err(|e| AppError::io(&dest, e))?;
        }
    }
    Ok(())
}

fn export_assets(db: &Database, repo: &Path, settings: &GitSyncSettings) -> Result<(), AppError> {
    // db.sql：无损权威备份
    let sql = db.export_sql_string_for_sync()?;
    atomic_write(&repo.join(DB_FILE), sql.as_bytes())?;
    // 可读镜像
    write_provider_mirror(db, repo)?;
    write_mcp_mirror(db, repo)?;
    write_skills_mirror(repo)?;
    export_agent_assets(repo, settings)?;
    write_manifest(repo, settings)?;
    Ok(())
}

// ─── 资产导入：工作区 → DB / skills ─────────────────────────

fn import_assets(db: &Database, repo: &Path, settings: &GitSyncSettings) -> Result<(), AppError> {
    let sql_path = repo.join(DB_FILE);
    if !sql_path.exists() {
        return Err(localized(
            "git.no_db_in_repo",
            "远端仓库缺少 db.sql，无法恢复",
            "Remote repository has no db.sql to restore from.",
        ));
    }
    let sql = fs::read_to_string(&sql_path).map_err(|e| AppError::io(&sql_path, e))?;

    // 先备份当前 SSOT，再替换 skills，最后导入 DB；任一步失败回滚 skills。
    let ssot = SkillService::get_ssot_dir().map_err(|e| {
        localized(
            "git.skills_ssot_failed",
            format!("获取 Skills SSOT 目录失败: {e}"),
            format!("Failed to resolve Skills SSOT directory: {e}"),
        )
    })?;
    let backup = backup_dir_to_temp(&ssot)?;

    if let Err(e) = restore_skills_from_repo(repo, &ssot) {
        let _ = restore_backup(&backup, &ssot);
        return Err(e);
    }

    if let Err(db_err) = db.import_sql_string_for_sync(sql.trim_start_matches('\u{feff}')) {
        let _ = restore_backup(&backup, &ssot);
        return Err(db_err);
    }

    // agent 资产（settings/hooks、subagents、commands、记忆文件）：加性恢复，
    // best-effort——失败不回滚已成功的 DB / skills 恢复，仅返回错误。
    import_agent_assets(repo, settings)?;
    Ok(())
}

fn restore_skills_from_repo(repo: &Path, ssot: &Path) -> Result<(), AppError> {
    if ssot.exists() {
        fs::remove_dir_all(ssot).map_err(|e| AppError::io(ssot, e))?;
    }
    let src = repo.join(SKILLS_DIR);
    if src.exists() {
        copy_dir_follow_symlinks(&src, ssot)?;
    } else {
        fs::create_dir_all(ssot).map_err(|e| AppError::io(ssot, e))?;
    }
    Ok(())
}

struct DirBackup {
    _tmp: tempfile::TempDir,
    backup: PathBuf,
    existed: bool,
}

fn backup_dir_to_temp(dir: &Path) -> Result<DirBackup, AppError> {
    let tmp = tempfile::tempdir().map_err(|e| AppError::IoContext {
        context: "创建临时备份目录失败 (failed to create temp backup dir)".into(),
        source: e,
    })?;
    let backup = tmp.path().join("backup");
    let existed = dir.exists();
    if existed {
        copy_dir_follow_symlinks(dir, &backup)?;
    }
    Ok(DirBackup {
        _tmp: tmp,
        backup,
        existed,
    })
}

fn restore_backup(backup: &DirBackup, dir: &Path) -> Result<(), AppError> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| AppError::io(dir, e))?;
    }
    if backup.existed {
        copy_dir_follow_symlinks(&backup.backup, dir)?;
    }
    Ok(())
}

/// 递归复制目录，跟随符号链接复制真实内容（导出明文时需要展开 skill 软链接）。
fn copy_dir_follow_symlinks(src: &Path, dest: &Path) -> Result<(), AppError> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dest).map_err(|e| AppError::io(dest, e))?;
    for entry in fs::read_dir(src).map_err(|e| AppError::io(src, e))? {
        let entry = entry.map_err(|e| AppError::io(src, e))?;
        let name = entry.file_name();
        // 跳过 VCS / 易变 / 隐私内容，避免把 .git、缓存等卷进来
        if AGENT_ASSET_IGNORE
            .iter()
            .any(|ig| name.to_string_lossy() == *ig)
        {
            continue;
        }
        let from = entry.path();
        let to = dest.join(&name);
        // 用 metadata 跟随符号链接判断真实类型
        let meta = fs::metadata(&from).map_err(|e| AppError::io(&from, e))?;
        if meta.is_dir() {
            copy_dir_follow_symlinks(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| AppError::io(&to, e))?;
        }
    }
    Ok(())
}

// ─── 公开 API ────────────────────────────────────────────────

/// 测试连接：对远端执行 `ls-remote`。
pub fn check_connection(settings: &GitSyncSettings) -> Result<(), AppError> {
    let runner = GitRunner::new(settings)?;
    let url = url_with_username(settings);
    runner.run(None, &["ls-remote", "--heads", url.as_str()])?;
    Ok(())
}

/// 查询远端分支当前 commit（用于 UI 展示）。
pub fn fetch_remote_info(settings: &GitSyncSettings) -> Result<Value, AppError> {
    let runner = GitRunner::new(settings)?;
    let url = url_with_username(settings);
    let out = runner.run(
        None,
        &[
            "ls-remote",
            "--heads",
            url.as_str(),
            settings.branch.as_str(),
        ],
    )?;
    let commit = out.split_whitespace().next().map(|s| s.to_string());
    Ok(json!({
        "branch": settings.branch,
        "remoteCommit": commit,
        "exists": commit.is_some(),
    }))
}

/// 推送（本地权威）。检测到远端领先时返回 `{ diverged: true }`，不修改远端。
pub fn push(db: &Database, settings: &mut GitSyncSettings) -> Result<Value, AppError> {
    push_inner(db, settings, false)
}

/// 强制推送（本地覆盖远端，`--force-with-lease`）。
pub fn force_push(db: &Database, settings: &mut GitSyncSettings) -> Result<Value, AppError> {
    push_inner(db, settings, true)
}

fn push_inner(
    db: &Database,
    settings: &mut GitSyncSettings,
    force: bool,
) -> Result<Value, AppError> {
    let runner = GitRunner::new(settings)?;
    let dir = ensure_repo(&runner, settings)?;
    let branch = settings.branch.clone();
    let remote_ref = format!("origin/{branch}");
    let remote_tracking = format!("refs/remotes/{remote_ref}");

    // 拉取远端引用（远端分支可能尚不存在，失败可忽略）。
    let _ = runner.run(Some(&dir), &["fetch", "origin", branch.as_str()]);
    let remote_exists = runner.check(
        Some(&dir),
        &["rev-parse", "--verify", "--quiet", remote_tracking.as_str()],
    );

    // 导出资产 → 暂存
    export_assets(db, &dir, settings)?;
    runner.run(Some(&dir), &["add", "-A"])?;

    let dirty = !runner
        .run(Some(&dir), &["status", "--porcelain"])?
        .trim()
        .is_empty();
    let has_commit = runner.check(Some(&dir), &["rev-parse", "--verify", "--quiet", "HEAD"]);

    // 无改动且本地与远端一致 → 无需同步
    if !dirty && has_commit && remote_exists {
        let up_to_date = runner.check(
            Some(&dir),
            &["diff", "--quiet", "HEAD", remote_ref.as_str()],
        );
        if up_to_date {
            return Ok(json!({ "success": true, "noChanges": true }));
        }
    }

    if dirty {
        let device = detect_device_name(settings);
        let msg = format!("{COMMIT_PREFIX} ({device}) {}", Utc::now().to_rfc3339());
        let user_cfg = format!("user.name={}", settings.author_name);
        let email_cfg = format!("user.email={}", settings.author_email);
        runner.run(
            Some(&dir),
            &[
                "-c",
                user_cfg.as_str(),
                "-c",
                email_cfg.as_str(),
                "commit",
                "-m",
                msg.as_str(),
            ],
        )?;
    }

    // 分歧检测：远端存在且其不是本地 HEAD 的祖先 → diverged
    if remote_exists && !force {
        let remote_is_ancestor = runner.check(
            Some(&dir),
            &["merge-base", "--is-ancestor", remote_ref.as_str(), "HEAD"],
        );
        if !remote_is_ancestor {
            return Ok(json!({
                "success": false,
                "diverged": true,
                "message": "远端有本地没有的提交，请选择强制覆盖或先拉取。",
            }));
        }
    }

    // 推送
    let push_args: Vec<&str> = if force {
        vec![
            "push",
            "--force-with-lease",
            "-u",
            "origin",
            branch.as_str(),
        ]
    } else {
        vec!["push", "-u", "origin", branch.as_str()]
    };
    runner.run(Some(&dir), &push_args)?;

    let commit = runner
        .run(Some(&dir), &["rev-parse", "HEAD"])
        .map(|s| s.trim().to_string())
        .ok();
    record_success(settings, if force { "force_push" } else { "push" }, commit);

    Ok(json!({ "success": true }))
}

/// 拉取（远端权威）：`reset --hard origin/<branch>` 后导入资产。
pub fn pull(db: &Database, settings: &mut GitSyncSettings) -> Result<Value, AppError> {
    let runner = GitRunner::new(settings)?;
    let dir = ensure_repo(&runner, settings)?;
    let branch = settings.branch.clone();
    let remote_ref = format!("origin/{branch}");
    let remote_tracking = format!("refs/remotes/{remote_ref}");

    runner.run(Some(&dir), &["fetch", "origin", branch.as_str()])?;
    let remote_exists = runner.check(
        Some(&dir),
        &["rev-parse", "--verify", "--quiet", remote_tracking.as_str()],
    );
    if !remote_exists {
        return Ok(json!({ "success": true, "empty": true }));
    }

    // 远端权威：丢弃本地工作区改动，硬重置到远端分支。
    runner.run(Some(&dir), &["reset", "--hard", remote_ref.as_str()])?;
    runner.run(Some(&dir), &["clean", "-ffd"])?;

    import_assets(db, &dir, settings)?;

    let commit = runner
        .run(Some(&dir), &["rev-parse", "HEAD"])
        .map(|s| s.trim().to_string())
        .ok();
    record_success(settings, "pull", commit);

    Ok(json!({ "success": true }))
}

fn record_success(settings: &mut GitSyncSettings, action: &str, commit: Option<String>) {
    let status = GitSyncStatus {
        last_sync_at: Some(Utc::now().timestamp_millis()),
        last_error: None,
        last_error_source: None,
        last_commit: commit,
        last_action: Some(action.to_string()),
    };
    settings.status = status.clone();
    let _ = crate::settings::update_git_sync_status(status);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::GitSyncSettings;

    fn sample_settings() -> GitSyncSettings {
        let mut s = GitSyncSettings {
            repo_url: "https://github.com/owner/repo.git".to_string(),
            ..GitSyncSettings::default()
        };
        s.normalize();
        s
    }

    #[test]
    fn url_with_username_injects_default_user() {
        let s = sample_settings();
        assert_eq!(
            url_with_username(&s),
            "https://x-access-token@github.com/owner/repo.git"
        );
    }

    #[test]
    fn url_with_username_respects_existing_at() {
        let mut s = sample_settings();
        s.repo_url = "https://user@github.com/owner/repo.git".to_string();
        assert_eq!(
            url_with_username(&s),
            "https://user@github.com/owner/repo.git"
        );
    }

    #[test]
    fn url_with_username_uses_custom_username() {
        let mut s = sample_settings();
        s.username = "alice".to_string();
        assert_eq!(
            url_with_username(&s),
            "https://alice@github.com/owner/repo.git"
        );
    }

    #[test]
    fn detect_device_name_prefers_explicit() {
        let mut s = sample_settings();
        s.device_name = "My Mac".to_string();
        assert_eq!(detect_device_name(&s), "My Mac");
    }

    /// 端到端：在设备 A 上 push 供应商配置到本地裸仓库，再在全新设备 B 上 pull 回来。
    #[test]
    #[serial_test::serial]
    fn push_then_pull_round_trips_providers_via_local_repo() {
        use crate::database::Database;
        use crate::provider::Provider;

        let base = tempfile::tempdir().expect("tempdir");
        // 本地裸仓库充当远端（local path remote，无需鉴权）。
        let remote = base.path().join("remote.git");
        let status = std::process::Command::new("git")
            .args(["init", "--bare", remote.to_str().unwrap()])
            .output()
            .expect("git init --bare");
        assert!(status.status.success(), "failed to init bare repo");
        let repo_url = remote.to_string_lossy().to_string();

        // ── 设备 A：写入一个供应商并 push ──
        let home_a = base.path().join("home_a");
        std::fs::create_dir_all(&home_a).unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", &home_a);

        let db_a = Database::init().expect("init db A");
        let provider = Provider::with_id(
            "p-test-1".to_string(),
            "My Provider".to_string(),
            serde_json::json!({ "env": { "ANTHROPIC_API_KEY": "sk-secret" } }),
            Some("https://example.com".to_string()),
        );
        db_a.save_provider("claude", &provider)
            .expect("save provider");

        // P2：agent 资产（记忆文件 + 子代理目录）
        let claude_a = home_a.join(".claude");
        std::fs::create_dir_all(claude_a.join("agents")).unwrap();
        std::fs::write(claude_a.join("CLAUDE.md"), b"# my rules\n").unwrap();
        std::fs::write(
            claude_a.join("agents").join("reviewer.md"),
            b"---\nname: reviewer\n---\nreview things",
        )
        .unwrap();

        let mut settings_a = GitSyncSettings {
            enabled: true,
            repo_url: repo_url.clone(),
            ..GitSyncSettings::default()
        };
        settings_a.normalize();
        let res = push(&db_a, &mut settings_a).expect("push ok");
        assert_eq!(res.get("success").and_then(|v| v.as_bool()), Some(true));

        // ── 设备 B：全新 home（空数据库），pull 后应恢复供应商 ──
        let home_b = base.path().join("home_b");
        std::fs::create_dir_all(&home_b).unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", &home_b);

        let db_b = Database::init().expect("init db B");
        assert!(
            db_b.get_all_providers("claude").unwrap().is_empty(),
            "fresh device should start empty"
        );

        let mut settings_b = GitSyncSettings {
            enabled: true,
            repo_url: repo_url.clone(),
            ..GitSyncSettings::default()
        };
        settings_b.normalize();
        let res = pull(&db_b, &mut settings_b).expect("pull ok");
        assert_eq!(res.get("success").and_then(|v| v.as_bool()), Some(true));

        let restored = db_b.get_all_providers("claude").expect("read providers B");
        assert!(
            restored.values().any(|p| p.name == "My Provider"),
            "pulled providers should include the one pushed from device A: {restored:?}"
        );

        // P2：agent 资产应被恢复到设备 B 的 ~/.claude
        let claude_b = home_b.join(".claude");
        assert_eq!(
            std::fs::read_to_string(claude_b.join("CLAUDE.md")).unwrap(),
            "# my rules\n",
            "CLAUDE.md should be restored on device B"
        );
        assert!(
            claude_b.join("agents").join("reviewer.md").exists(),
            "subagent should be restored on device B"
        );

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    /// 关闭某资产后，导出不应包含它。
    #[test]
    #[serial_test::serial]
    fn disabled_asset_is_skipped_on_export() {
        let base = tempfile::tempdir().expect("tempdir");
        let home = base.path().join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::write(home.join(".claude").join("CLAUDE.md"), b"hi").unwrap();
        std::env::set_var("CC_SWITCH_TEST_HOME", &home);

        let repo = base.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        let mut settings = GitSyncSettings::default();
        settings.disabled_assets = vec!["claude.memory".to_string()];

        export_agent_assets(&repo, &settings).expect("export agent assets");
        assert!(
            !repo.join("agents/claude/CLAUDE.md").exists(),
            "disabled asset must not be exported"
        );

        // 启用时应导出
        export_agent_assets(&repo, &GitSyncSettings::default()).expect("export agent assets");
        assert!(
            repo.join("agents/claude/CLAUDE.md").exists(),
            "enabled asset must be exported"
        );

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    #[test]
    fn asset_catalog_lists_known_assets() {
        let catalog = asset_catalog();
        assert!(!catalog.is_empty());
        assert!(catalog
            .iter()
            .any(|a| a.get("id").and_then(|v| v.as_str()) == Some("claude.settings")));
    }
}
