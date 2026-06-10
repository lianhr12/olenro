//! Session manager
//!
//! Provides session management across different AI coding tools.

use crate::error::AppResult;
use crate::provider::AppType;
use serde::{Deserialize, Serialize};

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub app_type: AppType,
    /// A human-friendly title (first user message, or the file name).
    pub name: String,
    /// Working directory the session ran in, if known.
    pub cwd: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: usize,
    /// Absolute path to the session transcript file.
    pub path: String,
}

/// Session manager for browsing and restoring sessions
pub struct SessionManager;

impl SessionManager {
    pub fn new() -> Self {
        Self
    }

    /// List sessions for an app, most recently updated first.
    ///
    /// Currently implemented for Claude Code, which stores one JSONL transcript
    /// per session under `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`.
    pub fn list_sessions(&self, app_type: &AppType) -> AppResult<Vec<Session>> {
        match app_type {
            AppType::Claude => list_claude_sessions(),
            _ => Ok(vec![]),
        }
    }

    /// Get a session by ID.
    pub fn get_session(&self, app_type: &AppType, session_id: &str) -> AppResult<Option<Session>> {
        Ok(self
            .list_sessions(app_type)?
            .into_iter()
            .find(|s| s.id == session_id))
    }

    /// Delete a session (removes the transcript file).
    pub fn delete_session(&self, app_type: &AppType, session_id: &str) -> AppResult<()> {
        if let Some(session) = self.get_session(app_type, session_id)? {
            let _ = std::fs::remove_file(&session.path);
        }
        Ok(())
    }
}

/// Read all Claude Code sessions from `~/.claude/projects/**/*.jsonl`.
fn list_claude_sessions() -> AppResult<Vec<Session>> {
    let root = crate::config::get_claude_config_dir().join("projects");
    let mut sessions = Vec::new();

    let project_dirs = match std::fs::read_dir(&root) {
        Ok(d) => d,
        Err(_) => return Ok(sessions), // no projects dir yet
    };

    for project in project_dirs.flatten() {
        let pdir = project.path();
        if !pdir.is_dir() {
            continue;
        }
        let files = match std::fs::read_dir(&pdir) {
            Ok(f) => f,
            Err(_) => continue,
        };
        for entry in files.flatten() {
            let path = entry.path();
            // Skip nested dirs (e.g. subagent transcripts) and non-jsonl files.
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Some(session) = read_claude_session(&path) {
                sessions.push(session);
            }
        }
    }

    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

/// Parse a single Claude session transcript into a `Session` summary.
fn read_claude_session(path: &std::path::Path) -> Option<Session> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut message_count = 0usize;
    let mut cwd: Option<String> = None;
    let mut title: Option<String> = None;
    let mut session_id: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        message_count += 1;
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if session_id.is_none() {
            if let Some(id) = value.get("sessionId").and_then(|v| v.as_str()) {
                session_id = Some(id.to_string());
            }
        }
        if cwd.is_none() {
            if let Some(c) = value.get("cwd").and_then(|v| v.as_str()) {
                cwd = Some(c.to_string());
            }
        }
        if title.is_none() {
            if let Some(t) = first_user_text(&value) {
                title = Some(t);
            }
        }
    }

    // id: prefer the embedded sessionId, fall back to the file stem.
    let id = session_id.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    })?;

    let name = title.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("session")
            .to_string()
    });

    // Use file timestamps for created/updated (robust, no per-line parsing).
    let meta = std::fs::metadata(path).ok();
    let updated_at = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let created_at = meta
        .as_ref()
        .and_then(|m| m.created().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(updated_at);

    Some(Session {
        id,
        app_type: AppType::Claude,
        name: truncate_title(&name, 80),
        cwd,
        created_at,
        updated_at,
        message_count,
        path: path.to_string_lossy().to_string(),
    })
}

/// Extract the first meaningful user message text from a transcript line,
/// skipping slash-command / caveat noise.
fn first_user_text(value: &serde_json::Value) -> Option<String> {
    if value.get("type").and_then(|v| v.as_str()) != Some("user") {
        return None;
    }
    let content = value.get("message").and_then(|m| m.get("content"))?;
    let text = match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .find_map(|item| item.get("text").and_then(|t| t.as_str()))
            .map(|s| s.to_string())?,
        _ => return None,
    };
    let trimmed = text.trim();
    // Skip injected command/caveat blocks.
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return None;
    }
    Some(trimmed.to_string())
}

fn truncate_title(s: &str, max: usize) -> String {
    let one_line = s.replace('\n', " ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut t: String = one_line.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_claude_sessions_from_projects_dir() {
        let home = std::env::temp_dir().join(format!("olenro-sess-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let proj = home.join(".claude").join("projects").join("-tmp-project");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(
            proj.join("sess-abc.jsonl"),
            "{\"type\":\"user\",\"sessionId\":\"sess-abc\",\"cwd\":\"/tmp/project\",\"message\":{\"role\":\"user\",\"content\":\"refactor this function\"}}\n\
             {\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}\n",
        )
        .unwrap();
        std::env::set_var("OLENRO_TEST_HOME", &home);

        let sessions = SessionManager::new()
            .list_sessions(&AppType::Claude)
            .unwrap();
        std::env::remove_var("OLENRO_TEST_HOME");

        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.id, "sess-abc");
        assert_eq!(s.name, "refactor this function");
        assert_eq!(s.cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(s.message_count, 2);

        let _ = std::fs::remove_dir_all(&home);
    }
}
