use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 获取用户主目录，带回退和日志
///
/// ## Windows 注意事项
///
/// - `dirs::home_dir()` 在 Windows 上使用 `SHGetKnownFolderPath(FOLDERID_Profile)`，
///   返回的是真实用户目录（类似 `C:\\Users\\Alice`），与 v3.10.2 行为一致。
/// - 不要直接使用 `HOME` 环境变量：它可能由 Git/Cygwin/MSYS 等第三方工具注入，
///   且不一定等于用户目录，可能导致 `.olenro/olenro.db` 路径变化，从而“看起来像数据丢失”。
///
/// ## 测试隔离
///
/// 为了让 Windows CI/本地测试能稳定隔离真实用户数据，可通过 `CC_SWITCH_TEST_HOME`
/// 显式覆盖 home dir（仅用于测试/调试场景）。
pub fn get_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CC_SWITCH_TEST_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::home_dir().unwrap_or_else(|| {
        log::warn!("无法获取用户主目录，回退到当前目录");
        PathBuf::from(".")
    })
}

/// 获取 Claude Code 配置目录路径
pub fn get_claude_config_dir() -> PathBuf {
    if let Some(custom) = crate::settings::get_claude_override_dir() {
        return custom;
    }

    get_home_dir().join(".claude")
}

/// 默认 Claude MCP 配置文件路径 (~/.claude.json)
pub fn get_default_claude_mcp_path() -> PathBuf {
    get_home_dir().join(".claude.json")
}

fn derive_mcp_path_from_override(dir: &Path) -> Option<PathBuf> {
    let file_name = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())?
        .trim()
        .to_string();
    if file_name.is_empty() {
        return None;
    }
    let parent = dir.parent().unwrap_or_else(|| Path::new(""));
    Some(parent.join(format!("{file_name}.json")))
}

/// 获取 Claude MCP 配置文件路径，若设置了目录覆盖则与覆盖目录同级
pub fn get_claude_mcp_path() -> PathBuf {
    if let Some(custom_dir) = crate::settings::get_claude_override_dir() {
        if let Some(path) = derive_mcp_path_from_override(&custom_dir) {
            return path;
        }
    }
    get_default_claude_mcp_path()
}

/// 获取 Claude Code 主配置文件路径
pub fn get_claude_settings_path() -> PathBuf {
    let dir = get_claude_config_dir();
    let settings = dir.join("settings.json");
    if settings.exists() {
        return settings;
    }
    // 兼容旧版命名：若存在旧文件则继续使用
    let legacy = dir.join("claude.json");
    if legacy.exists() {
        return legacy;
    }
    // 默认新建：回落到标准文件名 settings.json（不再生成 claude.json）
    settings
}

/// 应用数据库文件名（Olenro）。
pub const DB_FILENAME: &str = "olenro.db";
/// 旧版 CC Switch 数据库文件名，仅用于首次启动时的一次性迁移识别。
const LEGACY_DB_FILENAME: &str = "cc-switch.db";

/// 获取应用配置目录路径 (~/.olenro)
pub fn get_app_config_dir() -> PathBuf {
    if let Some(custom) = crate::app_store::get_app_config_dir_override() {
        return custom;
    }

    get_home_dir().join(".olenro")
}

/// 获取应用配置文件路径
pub fn get_app_config_path() -> PathBuf {
    get_app_config_dir().join("config.json")
}

/// 旧版 CC Switch 配置目录（~/.cc-switch）。
///
/// 本项目基于 CC Switch 二次开发，配置目录已独立为 `~/.olenro`。
/// 该函数仅用于首次启动时的一次性导入，见 [`migrate_legacy_config_dir_if_needed`]。
fn legacy_cc_switch_dir() -> PathBuf {
    get_home_dir().join(".cc-switch")
}

/// 首次启动时，从旧版 CC Switch 目录（`~/.cc-switch`）一次性导入配置到 `~/.olenro`。
///
/// 仅当新目录尚无实际数据（`config.json` / `cc-switch.db` 均不存在）、且旧目录确有数据时才迁移，
/// 因此可安全地重复调用，也不受日志/panic 目录被提前创建的影响。
/// 采用复制而非移动：保留旧目录不动，避免影响仍在使用官方 CC Switch 的用户，
/// 同时让两个应用各自拥有独立的配置副本，互不干扰。
pub fn migrate_legacy_config_dir_if_needed() {
    // 用户自定义了配置目录时，跳过自动迁移
    if crate::app_store::get_app_config_dir_override().is_some() {
        return;
    }

    let new_dir = get_home_dir().join(".olenro");

    // 若新目录内仍是旧库名 cc-switch.db（早期版本只迁移了目录、未改库名），
    // 先就地重命名为 olenro.db，避免代码新建空库、看起来像“数据丢失”。
    rename_legacy_db_files(&new_dir);

    let new_has_data =
        new_dir.join("config.json").exists() || new_dir.join(DB_FILENAME).exists();
    if new_has_data {
        return;
    }

    let legacy = legacy_cc_switch_dir();
    let legacy_has_data =
        legacy.join("config.json").exists() || legacy.join(LEGACY_DB_FILENAME).exists();
    if !legacy_has_data {
        return;
    }

    log::info!(
        "检测到旧版 CC Switch 配置目录 {}，正在导入到 {}",
        legacy.display(),
        new_dir.display()
    );
    match copy_dir_recursive(&legacy, &new_dir) {
        Ok(count) => {
            // 旧库文件名为 cc-switch.db；复制后重命名为 olenro.db（含 -wal/-shm 边车文件）
            rename_legacy_db_files(&new_dir);
            log::info!("✓ 已从旧目录一次性导入 {count} 个文件");
        }
        Err(e) => log::warn!("导入旧配置目录失败（将以全新配置启动）：{e}"),
    }
}

/// 迁移后把旧库文件名 `cc-switch.db`（含 `-wal`/`-shm` 边车文件）重命名为 `olenro.db`。
///
/// 在应用启动早期、数据库尚未打开时调用，因此重命名是安全的；
/// 仅当目标不存在时才重命名，保证幂等。
fn rename_legacy_db_files(dir: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let from = dir.join(format!("{LEGACY_DB_FILENAME}{suffix}"));
        let to = dir.join(format!("{DB_FILENAME}{suffix}"));
        if from.exists() && !to.exists() {
            if let Err(e) = fs::rename(&from, &to) {
                log::warn!(
                    "重命名数据库文件失败 {} -> {}: {e}",
                    from.display(),
                    to.display()
                );
            }
        }
    }
}

/// 递归复制目录内容（合并到目标；跳过目标已存在的同名文件，保证幂等安全）。
///
/// 返回成功复制的文件数。在应用启动早期、数据库尚未打开时调用，
/// 因此对 SQLite 数据库及其 `-wal`/`-shm` 边车文件是安全的冷拷贝。
fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<usize> {
    let mut copied = 0usize;
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copied += copy_dir_recursive(&src, &dst)?;
        } else if !dst.exists() {
            fs::copy(&src, &dst)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// 清理供应商名称，确保文件名安全
#[allow(dead_code)]
pub fn sanitize_provider_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// 获取供应商配置文件路径
#[allow(dead_code)]
pub fn get_provider_config_path(provider_id: &str, provider_name: Option<&str>) -> PathBuf {
    let base_name = provider_name
        .map(sanitize_provider_name)
        .unwrap_or_else(|| sanitize_provider_name(provider_id));

    get_claude_config_dir().join(format!("settings-{base_name}.json"))
}

/// 读取 JSON 配置文件
pub fn read_json_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, AppError> {
    if !path.exists() {
        return Err(AppError::Config(format!("文件不存在: {}", path.display())));
    }

    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;

    serde_json::from_str(&content).map_err(|e| AppError::json(path, e))
}

/// 递归排序 JSON 对象的键（按字母顺序），确保序列化输出是确定性的
fn sort_json_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted_map = Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted_map.insert(key.clone(), sort_json_keys(&map[key]));
            }
            Value::Object(sorted_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_json_keys).collect()),
        other => other.clone(),
    }
}

/// 写入 JSON 配置文件（键按字母排序，确保确定性输出）
pub fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let value = serde_json::to_value(data).map_err(|e| AppError::JsonSerialize { source: e })?;
    let sorted_value = sort_json_keys(&value);
    let json = serde_json::to_string_pretty(&sorted_value)
        .map_err(|e| AppError::JsonSerialize { source: e })?;

    atomic_write(path, json.as_bytes())
}

/// 原子写入文本文件（用于 TOML/纯文本）
pub fn write_text_file(path: &Path, data: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    atomic_write(path, data.as_bytes())
}

/// 原子写入：写入临时文件后 rename 替换，避免半写状态
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let parent = path
        .parent()
        .ok_or_else(|| AppError::Config("无效的路径".to_string()))?;
    let mut tmp = parent.to_path_buf();
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Config("无效的文件名".to_string()))?
        .to_string_lossy()
        .to_string();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    tmp.push(format!("{file_name}.tmp.{ts}"));

    {
        let mut f = fs::File::create(&tmp).map_err(|e| AppError::io(&tmp, e))?;
        f.write_all(data).map_err(|e| AppError::io(&tmp, e))?;
        f.flush().map_err(|e| AppError::io(&tmp, e))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let perm = meta.permissions().mode();
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(perm));
        }
    }

    #[cfg(windows)]
    {
        // Windows 上 rename 目标存在会失败，先移除再重命名（尽量接近原子性）
        if path.exists() {
            let _ = fs::remove_file(path);
        }
        fs::rename(&tmp, path).map_err(|e| AppError::IoContext {
            context: format!("原子替换失败: {} -> {}", tmp.display(), path.display()),
            source: e,
        })?;
    }

    #[cfg(not(windows))]
    {
        fs::rename(&tmp, path).map_err(|e| AppError::IoContext {
            context: format!("原子替换失败: {} -> {}", tmp.display(), path.display()),
            source: e,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_mcp_path_from_override_preserves_folder_name() {
        let override_dir = PathBuf::from("/tmp/profile/.claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for nested dir");
        assert_eq!(derived, PathBuf::from("/tmp/profile/.claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_handles_non_hidden_folder() {
        let override_dir = PathBuf::from("/data/claude-config");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for standard dir");
        assert_eq!(derived, PathBuf::from("/data/claude-config.json"));
    }

    #[test]
    fn derive_mcp_path_from_override_supports_relative_rootless_dir() {
        let override_dir = PathBuf::from("claude");
        let derived = derive_mcp_path_from_override(&override_dir)
            .expect("should derive path for single segment");
        assert_eq!(derived, PathBuf::from("claude.json"));
    }

    #[test]
    fn derive_mcp_path_from_root_like_dir_returns_none() {
        let override_dir = PathBuf::from("/");
        assert!(derive_mcp_path_from_override(&override_dir).is_none());
    }

    #[test]
    fn sort_json_keys_sorts_top_level_object() {
        let input = serde_json::json!({
            "z": 1,
            "a": 2,
            "m": 3,
        });
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn sort_json_keys_recurses_into_nested_objects() {
        let input = serde_json::json!({
            "outer_b": {"z": 1, "a": 2},
            "outer_a": {"y": 3, "b": 4},
        });
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(
            serialized,
            r#"{"outer_a":{"b":4,"y":3},"outer_b":{"a":2,"z":1}}"#
        );
    }

    #[test]
    fn sort_json_keys_preserves_array_order() {
        let input = serde_json::json!([3, 1, 2]);
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, "[3,1,2]");
    }

    #[test]
    fn sort_json_keys_sorts_objects_inside_arrays_but_keeps_array_order() {
        let input = serde_json::json!([
            {"z": 1, "a": 2},
            {"y": 3, "b": 4},
        ]);
        let sorted = sort_json_keys(&input);
        let serialized = serde_json::to_string(&sorted).unwrap();
        assert_eq!(serialized, r#"[{"a":2,"z":1},{"b":4,"y":3}]"#);
    }

    #[test]
    fn sort_json_keys_passes_through_primitives() {
        let cases = vec![
            serde_json::json!("hello"),
            serde_json::json!(42),
            serde_json::json!(3.5),
            serde_json::json!(true),
            serde_json::json!(null),
        ];
        for value in cases {
            let sorted = sort_json_keys(&value);
            assert_eq!(sorted, value);
        }
    }

    #[test]
    fn sort_json_keys_handles_empty_collections() {
        let empty_obj = serde_json::json!({});
        assert_eq!(
            serde_json::to_string(&sort_json_keys(&empty_obj)).unwrap(),
            "{}"
        );

        let empty_arr = serde_json::json!([]);
        assert_eq!(
            serde_json::to_string(&sort_json_keys(&empty_arr)).unwrap(),
            "[]"
        );
    }

    #[test]
    fn sort_json_keys_produces_identical_output_for_different_insertion_orders() {
        // 核心保证：同一逻辑配置无论键的插入顺序如何，写出的字节序列必须一致。
        let mut a = Map::new();
        a.insert("env".to_string(), serde_json::json!({"PATH": "/usr/bin"}));
        a.insert("model".to_string(), serde_json::json!("claude-sonnet-4-5"));
        a.insert("permissions".to_string(), serde_json::json!({"allow": []}));

        let mut b = Map::new();
        b.insert("permissions".to_string(), serde_json::json!({"allow": []}));
        b.insert("model".to_string(), serde_json::json!("claude-sonnet-4-5"));
        b.insert("env".to_string(), serde_json::json!({"PATH": "/usr/bin"}));

        let sorted_a = sort_json_keys(&Value::Object(a));
        let sorted_b = sort_json_keys(&Value::Object(b));

        assert_eq!(
            serde_json::to_string(&sorted_a).unwrap(),
            serde_json::to_string(&sorted_b).unwrap(),
        );
    }

    #[test]
    fn copy_dir_recursive_merges_subdirs_and_skips_existing_files() {
        let base = std::env::temp_dir().join(format!(
            "olenro_copy_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let from = base.join("from");
        let to = base.join("to");
        fs::create_dir_all(from.join("logs")).unwrap();
        fs::write(from.join("config.json"), b"new").unwrap();
        fs::write(from.join("cc-switch.db"), b"db").unwrap();
        fs::write(from.join("logs/a.log"), b"log").unwrap();

        // 目标已存在同名文件：应被保留，不被覆盖
        fs::create_dir_all(&to).unwrap();
        fs::write(to.join("config.json"), b"existing").unwrap();

        let copied = copy_dir_recursive(&from, &to).unwrap();

        // config.json 被跳过；cc-switch.db 与 logs/a.log 被复制 = 2
        assert_eq!(copied, 2);
        assert_eq!(fs::read_to_string(to.join("config.json")).unwrap(), "existing");
        assert_eq!(fs::read_to_string(to.join("cc-switch.db")).unwrap(), "db");
        assert_eq!(fs::read_to_string(to.join("logs/a.log")).unwrap(), "log");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rename_legacy_db_files_renames_db_and_sidecars() {
        let base = std::env::temp_dir().join(format!(
            "olenro_rename_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("cc-switch.db"), b"main").unwrap();
        fs::write(base.join("cc-switch.db-wal"), b"wal").unwrap();
        fs::write(base.join("cc-switch.db-shm"), b"shm").unwrap();

        rename_legacy_db_files(&base);

        // 主库与 -wal/-shm 边车文件都应改名为 olenro.db*，旧名消失，内容不变
        assert!(base.join("olenro.db").exists());
        assert!(base.join("olenro.db-wal").exists());
        assert!(base.join("olenro.db-shm").exists());
        assert!(!base.join("cc-switch.db").exists());
        assert_eq!(fs::read_to_string(base.join("olenro.db")).unwrap(), "main");
        assert_eq!(fs::read_to_string(base.join("olenro.db-wal")).unwrap(), "wal");

        let _ = fs::remove_dir_all(&base);
    }
}

/// 复制文件
pub fn copy_file(from: &Path, to: &Path) -> Result<(), AppError> {
    fs::copy(from, to).map_err(|e| AppError::IoContext {
        context: format!("复制文件失败 ({} -> {})", from.display(), to.display()),
        source: e,
    })?;
    Ok(())
}

/// 删除文件
pub fn delete_file(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| AppError::io(path, e))?;
    }
    Ok(())
}

/// 检查 Claude Code 配置状态
#[derive(Serialize, Deserialize)]
pub struct ConfigStatus {
    pub exists: bool,
    pub path: String,
}

/// 获取 Claude Code 配置状态
pub fn get_claude_config_status() -> ConfigStatus {
    let path = get_claude_settings_path();
    ConfigStatus {
        exists: path.exists(),
        path: path.to_string_lossy().to_string(),
    }
}
