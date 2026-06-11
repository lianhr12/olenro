//! Claude 子代理（subagent）管理
//!
//! Claude Code 的子代理以带 YAML frontmatter 的 markdown 文件存储：
//!
//! ```markdown
//! ---
//! name: code-reviewer
//! description: Use this agent to review diffs for bugs.
//! tools: Read, Grep, Bash
//! model: sonnet
//! ---
//!
//! You are a meticulous code reviewer. ...
//! ```
//!
//! 用户级位于 `~/.claude/agents/*.md`，项目级位于 `<cwd>/.claude/agents/*.md`。
//! 本模块为 olenro CLI/TUI 新建（桌面端尚未实现此功能）。

use crate::config::get_claude_config_dir;
use crate::error::{AppError, AppResult};
use serde::Deserialize;
use std::path::PathBuf;

/// 子代理的作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentScope {
    /// `~/.claude/agents/`
    User,
    /// `<cwd>/.claude/agents/`
    Project,
}

impl AgentScope {
    pub fn label(self) -> &'static str {
        match self {
            AgentScope::User => "user",
            AgentScope::Project => "project",
        }
    }

    /// 该作用域下的 agents 目录。
    pub fn dir(self) -> PathBuf {
        match self {
            AgentScope::User => get_claude_config_dir().join("agents"),
            AgentScope::Project => std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".claude")
                .join("agents"),
        }
    }
}

/// 一个 Claude 子代理。
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub description: Option<String>,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub scope: AgentScope,
    pub path: String,
    /// frontmatter 之后的正文（系统提示词）。
    pub body: String,
}

/// frontmatter 的可反序列化形状。
#[derive(Debug, Default, Deserialize)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    tools: Option<serde_yaml::Value>,
    model: Option<String>,
}

/// 把 `tools` 字段（字符串 "a, b" 或 YAML 列表）归一化成 `Vec<String>`。
fn normalize_tools(value: Option<serde_yaml::Value>) -> Vec<String> {
    match value {
        Some(serde_yaml::Value::String(s)) => s
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
        Some(serde_yaml::Value::Sequence(seq)) => seq
            .into_iter()
            .filter_map(|v| match v {
                serde_yaml::Value::String(s) => Some(s.trim().to_string()),
                other => other.as_str().map(|s| s.to_string()),
            })
            .filter(|t| !t.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// 把 markdown 内容拆成 (frontmatter_yaml, body)。无 frontmatter 时 yaml 为 None。
fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let rest = match content.strip_prefix("---\n") {
        Some(r) => r,
        None => match content.strip_prefix("---\r\n") {
            Some(r) => r,
            None => return (None, content),
        },
    };
    // 找到结束的 `---` 行。
    let mut idx = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let yaml = &rest[..idx];
            let body = &rest[idx + line.len()..];
            return (Some(yaml), body.trim_start_matches(['\r', '\n']));
        }
        idx += line.len();
    }
    (None, content)
}

/// 解析单个 agent 文件。解析失败返回 None（跳过坏文件，不报错）。
fn parse_agent_file(path: &std::path::Path, scope: AgentScope) -> Option<Agent> {
    let content = std::fs::read_to_string(path).ok()?;
    let (yaml, body) = split_frontmatter(&content);
    let fm: Frontmatter = match yaml {
        Some(y) => serde_yaml::from_str(y).unwrap_or_default(),
        None => Frontmatter::default(),
    };
    let stem = path.file_stem()?.to_string_lossy().to_string();
    Some(Agent {
        name: fm.name.unwrap_or(stem),
        description: fm.description,
        tools: normalize_tools(fm.tools),
        model: fm.model,
        scope,
        path: path.to_string_lossy().to_string(),
        body: body.to_string(),
    })
}

/// 列出某作用域下的全部子代理（按名称排序）。
pub fn list_agents_in(scope: AgentScope) -> Vec<Agent> {
    let dir = scope.dir();
    let mut agents = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(agent) = parse_agent_file(&path, scope) {
                    agents.push(agent);
                }
            }
        }
    }
    agents.sort_by(|a, b| a.name.cmp(&b.name));
    agents
}

/// 列出用户级 + 项目级的全部子代理（用户级在前）。
pub fn list_agents() -> Vec<Agent> {
    let mut all = list_agents_in(AgentScope::User);
    all.extend(list_agents_in(AgentScope::Project));
    all
}

/// 按作用域 + 名称获取子代理。
pub fn get_agent(scope: AgentScope, name: &str) -> Option<Agent> {
    list_agents_in(scope).into_iter().find(|a| a.name == name)
}

/// 删除一个子代理文件（幂等）。
pub fn delete_agent(scope: AgentScope, name: &str) -> AppResult<()> {
    let path = scope.dir().join(format!("{name}.md"));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// 创建/覆盖一个子代理。`name` 必须是安全的文件名片段。
pub fn create_agent(
    scope: AgentScope,
    name: &str,
    description: Option<&str>,
    tools: &[String],
    model: Option<&str>,
    body: &str,
) -> AppResult<Agent> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(AppError::InvalidOperation(format!(
            "Invalid agent name: {name}"
        )));
    }

    let dir = scope.dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{trimmed}.md"));

    let mut out = String::from("---\n");
    out.push_str(&format!("name: {trimmed}\n"));
    if let Some(d) = description.filter(|d| !d.trim().is_empty()) {
        out.push_str(&format!("description: {d}\n"));
    }
    if !tools.is_empty() {
        out.push_str(&format!("tools: {}\n", tools.join(", ")));
    }
    if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
        out.push_str(&format!("model: {m}\n"));
    }
    out.push_str("---\n\n");
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }

    crate::config::write_text_file(&path, &out)?;

    Ok(Agent {
        name: trimmed.to_string(),
        description: description.map(|s| s.to_string()),
        tools: tools.to_vec(),
        model: model.map(|s| s.to_string()),
        scope,
        path: path.to_string_lossy().to_string(),
        body: body.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_parses_yaml_and_body() {
        let content = "---\nname: foo\ndescription: bar\n---\n\nHello body\n";
        let (yaml, body) = split_frontmatter(content);
        assert_eq!(yaml, Some("name: foo\ndescription: bar\n"));
        assert_eq!(body, "Hello body\n");
    }

    #[test]
    fn split_frontmatter_handles_no_frontmatter() {
        let (yaml, body) = split_frontmatter("just text\n");
        assert!(yaml.is_none());
        assert_eq!(body, "just text\n");
    }

    #[test]
    fn normalize_tools_handles_string_and_list() {
        let s = serde_yaml::Value::String("Read, Edit , Bash".to_string());
        assert_eq!(normalize_tools(Some(s)), vec!["Read", "Edit", "Bash"]);

        let list: serde_yaml::Value = serde_yaml::from_str("[Read, Grep]").unwrap();
        assert_eq!(normalize_tools(Some(list)), vec!["Read", "Grep"]);

        assert!(normalize_tools(None).is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn create_list_get_delete_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let old = std::env::var_os("OLENRO_TEST_HOME");
        std::env::set_var("OLENRO_TEST_HOME", tmp.path());

        let tools = vec!["Read".to_string(), "Bash".to_string()];
        let created = create_agent(
            AgentScope::User,
            "code-reviewer",
            Some("Reviews diffs"),
            &tools,
            Some("sonnet"),
            "You are a reviewer.",
        )
        .unwrap();
        assert_eq!(created.name, "code-reviewer");

        let agents = list_agents_in(AgentScope::User);
        assert_eq!(agents.len(), 1);
        let a = &agents[0];
        assert_eq!(a.name, "code-reviewer");
        assert_eq!(a.description.as_deref(), Some("Reviews diffs"));
        assert_eq!(a.tools, tools);
        assert_eq!(a.model.as_deref(), Some("sonnet"));
        assert!(a.body.contains("You are a reviewer."));

        assert!(get_agent(AgentScope::User, "code-reviewer").is_some());

        delete_agent(AgentScope::User, "code-reviewer").unwrap();
        assert!(list_agents_in(AgentScope::User).is_empty());

        match old {
            Some(v) => std::env::set_var("OLENRO_TEST_HOME", v),
            None => std::env::remove_var("OLENRO_TEST_HOME"),
        }
    }

    #[test]
    fn create_agent_rejects_unsafe_names() {
        assert!(create_agent(AgentScope::User, "../evil", None, &[], None, "x").is_err());
        assert!(create_agent(AgentScope::User, "a/b", None, &[], None, "x").is_err());
        assert!(create_agent(AgentScope::User, "", None, &[], None, "x").is_err());
    }
}
