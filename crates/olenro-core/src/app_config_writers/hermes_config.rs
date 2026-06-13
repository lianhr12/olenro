//! Hermes Agent 配置文件读写模块（memory + provider 子集）
//!
//! 处理 `~/.hermes/config.yaml`（YAML）以及 `~/.hermes/memories/` 下的
//! `MEMORY.md` / `USER.md` 记忆文件，以及供应商切换时的配置写入。
//!
//! 本模块由桌面端 `src-tauri/src/hermes_config.rs` 移植而来，目前覆盖
//! Hermes Memory 视图所需部分（配置读取 + YAML 段落保格式写入 + 记忆文件读写
//! + 记忆开关/预算）以及基础的供应商切换写入逻辑。
//!
//! 适配：错误类型用 `olenro_core::error::AppError`（`io`/`JsonSerialize` 构造器
//! 走 `#[from]` 的 `?`）；去掉桌面端 settings 覆盖目录依赖（CLI 固定 `~/.hermes`）。

use crate::config::{atomic_write, get_app_config_dir, get_home_dir};
use crate::error::AppError;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// 保留的 hermes 配置备份数量（桌面端可配置，CLI 固定）。
const HERMES_BACKUP_RETAIN_COUNT: usize = 10;

// ============================================================================
// Path Functions
// ============================================================================

/// 获取 Hermes 配置目录（`~/.hermes/`）。
pub fn get_hermes_dir() -> PathBuf {
    get_home_dir().join(".hermes")
}

/// 获取 Hermes 配置文件路径（`~/.hermes/config.yaml`）。
pub fn get_hermes_config_path() -> PathBuf {
    get_hermes_dir().join("config.yaml")
}

fn hermes_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// ============================================================================
// Type Definitions
// ============================================================================

/// Hermes 写入结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HermesWriteOutcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
}

// ============================================================================
// Core YAML Read Functions
// ============================================================================

/// 读取 Hermes 配置文件为 `serde_yaml::Value`；不存在或为空时返回空 Mapping。
pub fn read_hermes_config() -> Result<serde_yaml::Value, AppError> {
    let path = get_hermes_config_path();
    if !path.exists() {
        return Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }

    let content = fs::read_to_string(&path)?;
    if content.trim().is_empty() {
        return Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }

    serde_yaml::from_str(&content)
        .map_err(|e| AppError::Config(format!("Failed to parse Hermes config as YAML: {e}")))
}

// ============================================================================
// YAML Section-Level Replacement
// ============================================================================

/// 判断一行是否为 YAML 顶层键（第 0 列、非注释、非序列项、含 `key:`）。
fn is_top_level_key_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let first_char = line.as_bytes()[0];
    if first_char == b' ' || first_char == b'\t' || first_char == b'#' || first_char == b'-' {
        return false;
    }
    if let Some(colon_pos) = line.find(':') {
        let after_colon = &line[colon_pos + 1..];
        after_colon.is_empty() || after_colon.starts_with(' ') || after_colon.starts_with('\t')
    } else {
        false
    }
}

/// 找出某个顶层 YAML 段落的字节区间 `(start_inclusive, end_exclusive)`。
fn find_yaml_section_range(raw: &str, section_key: &str) -> Option<(usize, usize)> {
    let target = format!("{}:", section_key);
    let mut section_start = None;
    let mut offset = 0;

    for line in raw.split('\n') {
        if section_start.is_none() && is_top_level_key_line(line) && line.starts_with(&target) {
            let after_target = &line[target.len()..];
            if after_target.is_empty()
                || after_target.starts_with(' ')
                || after_target.starts_with('\t')
                || after_target.starts_with('\r')
            {
                section_start = Some(offset);
            }
        } else if section_start.is_some() && is_top_level_key_line(line) {
            return Some((section_start.unwrap(), offset));
        }
        offset += line.len() + 1; // +1 for the \n
    }

    section_start.map(|start| (start, raw.len()))
}

/// 将段落 key + value 序列化成 YAML 片段。
fn serialize_yaml_section(key: &str, value: &serde_yaml::Value) -> Result<String, AppError> {
    let mut section = serde_yaml::Mapping::new();
    section.insert(serde_yaml::Value::String(key.to_string()), value.clone());
    let yaml_str = serde_yaml::to_string(&serde_yaml::Value::Mapping(section))
        .map_err(|e| AppError::Config(format!("Failed to serialize YAML section '{key}': {e}")))?;
    Ok(yaml_str)
}

/// 在原始文本中替换某段落；不存在则追加到末尾（保留注释与其它段落）。
fn replace_yaml_section(
    raw: &str,
    section_key: &str,
    value: &serde_yaml::Value,
) -> Result<String, AppError> {
    let serialized = serialize_yaml_section(section_key, value)?;

    if let Some((start, end)) = find_yaml_section_range(raw, section_key) {
        let mut result = String::with_capacity(raw.len());
        result.push_str(&raw[..start]);
        result.push_str(&serialized);
        let remainder = &raw[end..];
        if !serialized.ends_with('\n') && !remainder.is_empty() && !remainder.starts_with('\n') {
            result.push('\n');
        }
        result.push_str(remainder);
        Ok(result)
    } else {
        let mut result = raw.to_string();
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        result.push_str(&serialized);
        if !result.ends_with('\n') {
            result.push('\n');
        }
        Ok(result)
    }
}

// ============================================================================
// Backup & Cleanup
// ============================================================================

fn create_hermes_backup(source: &str) -> Result<PathBuf, AppError> {
    let backup_dir = get_app_config_dir().join("backups").join("hermes");
    fs::create_dir_all(&backup_dir)?;

    let base_id = format!("hermes_{}", Local::now().format("%Y%m%d_%H%M%S"));
    let mut filename = format!("{base_id}.yaml");
    let mut backup_path = backup_dir.join(&filename);
    let mut counter = 1;

    while backup_path.exists() {
        filename = format!("{base_id}_{counter}.yaml");
        backup_path = backup_dir.join(&filename);
        counter += 1;
    }

    atomic_write(&backup_path, source.as_bytes())?;
    cleanup_hermes_backups(&backup_dir)?;
    Ok(backup_path)
}

fn cleanup_hermes_backups(dir: &Path) -> Result<(), AppError> {
    let retain = HERMES_BACKUP_RETAIN_COUNT;
    let mut entries = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "yaml" || ext == "yml")
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    if entries.len() <= retain {
        return Ok(());
    }

    entries.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    let remove_count = entries.len().saturating_sub(retain);
    for entry in entries.into_iter().take(remove_count) {
        if let Err(err) = fs::remove_file(entry.path()) {
            log::warn!(
                "Failed to remove old Hermes config backup {}: {err}",
                entry.path().display()
            );
        }
    }

    Ok(())
}

// ============================================================================
// High-level Write Helper
// ============================================================================

/// 写入单个顶层 YAML 段落（调用方必须已持有写锁）。保留注释与其它段落。
fn write_yaml_section_to_config_locked(
    section_key: &str,
    value: &serde_yaml::Value,
) -> Result<HermesWriteOutcome, AppError> {
    let config_path = get_hermes_config_path();
    let raw = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    let new_raw = replace_yaml_section(&raw, section_key, value)?;

    if new_raw == raw {
        return Ok(HermesWriteOutcome::default());
    }

    let backup_path = if !raw.is_empty() {
        Some(create_hermes_backup(&raw)?)
    } else {
        None
    };

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    atomic_write(&config_path, new_raw.as_bytes())?;

    log::debug!(
        "Hermes config section '{}' written to {:?}",
        section_key,
        config_path
    );
    Ok(HermesWriteOutcome {
        backup_path: backup_path.map(|p| p.display().to_string()),
    })
}

// ============================================================================
// Memory Files
// ============================================================================

/// Hermes 的两个记忆文件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    Memory,
    User,
}

impl MemoryKind {
    fn filename(self) -> &'static str {
        match self {
            Self::Memory => "MEMORY.md",
            Self::User => "USER.md",
        }
    }
}

fn memories_dir() -> PathBuf {
    get_hermes_dir().join("memories")
}

/// 读取记忆文件为 markdown 文本；不存在时返回空串（首次使用）。
pub fn read_memory(kind: MemoryKind) -> Result<String, AppError> {
    let path = memories_dir().join(kind.filename());
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(AppError::Io(e)),
    }
}

/// 原子写入记忆文件（自动创建 `~/.hermes/memories/`）。
pub fn write_memory(kind: MemoryKind, content: &str) -> Result<(), AppError> {
    let path = memories_dir().join(kind.filename());
    atomic_write(&path, content.as_bytes())
}

/// 两个记忆 blob 的字符预算 + 开关，取自 `config.yaml`。默认值与 Hermes 自身默认一致。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesMemoryLimits {
    pub memory: usize,
    pub user: usize,
    pub memory_enabled: bool,
    pub user_enabled: bool,
}

impl Default for HermesMemoryLimits {
    fn default() -> Self {
        Self {
            memory: 2200,
            user: 1375,
            memory_enabled: true,
            user_enabled: true,
        }
    }
}

/// 切换某个记忆 blob 的开关，保留 `memory:` 段其它字段。Hermes 把用户档案开关存在
/// `user_profile_enabled`（非 `user_enabled`），该映射在此处理。
pub fn set_memory_enabled(kind: MemoryKind, enabled: bool) -> Result<HermesWriteOutcome, AppError> {
    let _guard = hermes_write_lock()
        .lock()
        .map_err(|_| AppError::Config("Hermes config write lock poisoned".to_string()))?;
    let config = read_hermes_config()?;

    let mut memory = match config.get("memory") {
        Some(serde_yaml::Value::Mapping(m)) => m.clone(),
        _ => serde_yaml::Mapping::new(),
    };

    let key = match kind {
        MemoryKind::Memory => "memory_enabled",
        MemoryKind::User => "user_profile_enabled",
    };
    memory.insert(
        serde_yaml::Value::String(key.to_string()),
        serde_yaml::Value::Bool(enabled),
    );

    write_yaml_section_to_config_locked("memory", &serde_yaml::Value::Mapping(memory))
}

/// 读取记忆预算 + 开关；缺失/不可解析字段回退到默认值，不报错。
pub fn read_memory_limits() -> Result<HermesMemoryLimits, AppError> {
    let mut out = HermesMemoryLimits::default();
    let config = read_hermes_config()?;
    let Some(memory) = config.get("memory") else {
        return Ok(out);
    };

    if let Some(v) = memory.get("memory_char_limit").and_then(|v| v.as_u64()) {
        out.memory = v as usize;
    }
    if let Some(v) = memory.get("user_char_limit").and_then(|v| v.as_u64()) {
        out.user = v as usize;
    }
    if let Some(v) = memory.get("memory_enabled").and_then(|v| v.as_bool()) {
        out.memory_enabled = v;
    }
    if let Some(v) = memory.get("user_profile_enabled").and_then(|v| v.as_bool()) {
        out.user_enabled = v;
    }

    Ok(out)
}

// ============================================================================
// Provider Switch Support
// ============================================================================

/// Convenience to build a YAML string key.
fn yaml_key(key: &str) -> serde_yaml::Value {
    serde_yaml::Value::String(key.to_string())
}

/// Write provider settings to Hermes config.yaml for provider switching.
/// Updates the `provider` field and related env vars in the `model` section.
pub fn write_provider_for_switch(
    provider_name: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
    provider_config: &serde_json::Value,
) -> Result<HermesWriteOutcome, AppError> {
    let _guard = hermes_write_lock().lock().unwrap();

    // Read existing config (defaults to an empty mapping) and grab its map.
    let mut config = read_hermes_config()?;
    let root = config.as_mapping_mut().ok_or_else(|| {
        AppError::Config("Hermes config is not a valid YAML mapping".to_string())
    })?;

    // Take (or create) the `model` mapping.
    let mut model = root
        .get(yaml_key("model"))
        .and_then(|v| v.as_mapping())
        .cloned()
        .unwrap_or_default();

    model.insert(yaml_key("provider"), yaml_key(provider_name));

    if let Some(url) = base_url.filter(|s| !s.is_empty()) {
        model.insert(yaml_key("base_url"), yaml_key(url));
    }

    // Build the env block from the provider config plus the api key.
    let mut env = serde_yaml::Mapping::new();
    if let Some(obj) = provider_config.get("env").and_then(|e| e.as_object()) {
        for (k, v) in obj {
            if let Some(v_str) = v.as_str() {
                env.insert(yaml_key(k), yaml_key(v_str));
            }
        }
    }
    if let Some(key) = api_key.filter(|s| !s.is_empty()) {
        env.insert(yaml_key("api_key"), yaml_key(key));
    }
    if !env.is_empty() {
        model.insert(yaml_key("env"), serde_yaml::Value::Mapping(env));
    }

    root.insert(yaml_key("model"), serde_yaml::Value::Mapping(model));

    // Serialize and write atomically.
    let yaml_content = serde_yaml::to_string(&config)
        .map_err(|e| AppError::Config(format!("Failed to serialize Hermes config: {}", e)))?;

    let config_path = get_hermes_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    atomic_write(&config_path, yaml_content.as_bytes())?;

    log::info!("Wrote Hermes config.yaml for provider: {}", provider_name);
    Ok(HermesWriteOutcome { backup_path: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::{Mutex, OnceLock};

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|err| err.into_inner())
    }

    fn with_test_home<T>(test_fn: impl FnOnce() -> T) -> T {
        let _guard = test_guard();
        let tmp = tempfile::tempdir().unwrap();
        let old = std::env::var_os("OLENRO_TEST_HOME");
        std::env::set_var("OLENRO_TEST_HOME", tmp.path());
        let result = test_fn();
        match old {
            Some(value) => std::env::set_var("OLENRO_TEST_HOME", value),
            None => std::env::remove_var("OLENRO_TEST_HOME"),
        }
        result
    }

    #[test]
    fn find_section_in_multi_section_yaml() {
        let yaml = "\
model:
  default: gpt-4
  provider: openai
agent:
  max_turns: 10
custom_providers:
  - name: foo
";
        let (start, end) = find_yaml_section_range(yaml, "agent").unwrap();
        let section = &yaml[start..end];
        assert!(section.starts_with("agent:"));
        assert!(section.contains("max_turns"));
        assert!(!section.contains("custom_providers"));
    }

    #[test]
    fn find_section_at_end_of_file() {
        let yaml = "\
model:
  default: gpt-4
agent:
  max_turns: 10
";
        let (start, end) = find_yaml_section_range(yaml, "agent").unwrap();
        let section = &yaml[start..end];
        assert!(section.starts_with("agent:"));
        assert!(section.contains("max_turns"));
    }

    #[test]
    #[serial]
    fn memory_file_roundtrip() {
        with_test_home(|| {
            assert_eq!(read_memory(MemoryKind::Memory).unwrap(), "");
            write_memory(MemoryKind::Memory, "# hello\n").unwrap();
            assert_eq!(read_memory(MemoryKind::Memory).unwrap(), "# hello\n");

            write_memory(MemoryKind::User, "profile").unwrap();
            assert_eq!(read_memory(MemoryKind::User).unwrap(), "profile");
        });
    }

    #[test]
    #[serial]
    fn set_memory_enabled_preserves_comments_and_limits() {
        with_test_home(|| {
            let path = get_hermes_config_path();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(
                &path,
                "# top comment\nmodel:\n  default: gpt-4\nmemory:\n  memory_char_limit: 999\n  memory_enabled: true\n",
            )
            .unwrap();

            let outcome = set_memory_enabled(MemoryKind::Memory, false).unwrap();
            assert!(outcome.backup_path.is_some());

            let written = fs::read_to_string(&path).unwrap();
            assert!(written.contains("# top comment"));
            assert!(written.contains("default: gpt-4"));

            let limits = read_memory_limits().unwrap();
            assert_eq!(limits.memory, 999);
            assert!(!limits.memory_enabled);
        });
    }

    #[test]
    #[serial]
    fn read_memory_limits_defaults_when_absent() {
        with_test_home(|| {
            let limits = read_memory_limits().unwrap();
            assert_eq!(limits.memory, 2200);
            assert_eq!(limits.user, 1375);
            assert!(limits.memory_enabled);
            assert!(limits.user_enabled);
        });
    }
}
