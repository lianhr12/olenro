//! skills.sh 技能市场发现
//!
//! 由桌面端 `src-tauri/src/services/skill.rs` 移植：搜索 <https://skills.sh> 公共
//! 目录，以及抓取首页解析出热门技能。返回的可安装技能映射到 GitHub 仓库，
//! 复用现有的 `SkillService::install_from_github` 安装流程。

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ========== skills.sh API 类型 ==========

/// skills.sh `/api/search` 原始响应。
#[derive(Debug, Clone, Deserialize)]
struct SkillsShApiResponse {
    query: String,
    skills: Vec<SkillsShApiSkill>,
    count: usize,
}

/// skills.sh 原始技能条目。
#[derive(Debug, Clone, Deserialize)]
struct SkillsShApiSkill {
    id: String,
    #[serde(rename = "skillId")]
    skill_id: String,
    name: String,
    installs: u64,
    source: String,
}

/// 搜索结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillsShSearchResult {
    pub skills: Vec<SkillsShDiscoverableSkill>,
    pub total_count: usize,
    pub query: String,
}

/// 可安装的市场技能。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsShDiscoverableSkill {
    pub key: String,
    pub name: String,
    pub directory: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub installs: u64,
    pub readme_url: Option<String>,
}

impl SkillsShDiscoverableSkill {
    /// `owner/repo`，用于 `SkillService::install_from_github`。
    pub fn repo_slug(&self) -> String {
        format!("{}/{}", self.repo_owner, self.repo_name)
    }
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Network(format!("failed to build HTTP client: {e}")))
}

/// 搜索 skills.sh 公共目录（`q` 至少 2 个字符）。
pub async fn search(query: &str, limit: usize, offset: usize) -> AppResult<SkillsShSearchResult> {
    let url = url::Url::parse_with_params(
        "https://skills.sh/api/search",
        &[
            ("q", query),
            ("limit", &limit.to_string()),
            ("offset", &offset.to_string()),
        ],
    )
    .map_err(|e| AppError::Network(format!("bad skills.sh URL: {e}")))?;

    let resp = client()?
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("skills.sh request failed: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Network(format!("skills.sh returned error: {e}")))?
        .json::<SkillsShApiResponse>()
        .await
        .map_err(|e| AppError::Network(format!("skills.sh response parse failed: {e}")))?;

    let skills = resp
        .skills
        .into_iter()
        .filter_map(|s| map_skill(&s.source, s.id, s.skill_id, s.name, s.installs))
        .collect();

    Ok(SkillsShSearchResult {
        skills,
        total_count: resp.count,
        query: resp.query,
    })
}

/// 获取热门技能（按安装量倒序）。
///
/// skills.sh 没有公开的“热门”接口，但首页将按安装量排序的技能内联在
/// Next.js flight 数据中。这里抓取首页 HTML 并正则解析出这些条目。
pub async fn popular(limit: usize) -> AppResult<SkillsShSearchResult> {
    let html = client()?
        .get("https://www.skills.sh/")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("skills.sh request failed: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Network(format!("skills.sh returned error: {e}")))?
        .text()
        .await
        .map_err(|e| AppError::Network(format!("skills.sh response read failed: {e}")))?;

    let skills = parse_popular_html(&html, limit);
    let total_count = skills.len();
    Ok(SkillsShSearchResult {
        skills,
        total_count,
        query: String::new(),
    })
}

/// 从首页 HTML 的 flight 数据中解析热门技能条目（去重、限量）。
fn parse_popular_html(html: &str, limit: usize) -> Vec<SkillsShDiscoverableSkill> {
    static POPULAR_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = POPULAR_RE.get_or_init(|| {
        regex::Regex::new(
            r#"\\"source\\":\\"([^"\\]+)\\",\\"skillId\\":\\"([^"\\]+)\\",\\"name\\":\\"([^"\\]+)\\",\\"installs\\":(\d+)"#,
        )
        .expect("valid popular skills regex")
    });

    let mut seen = std::collections::HashSet::new();
    let mut skills = Vec::new();
    for caps in re.captures_iter(html) {
        let source = &caps[1];
        let skill_id = caps[2].to_string();
        let name = caps[3].to_string();
        let installs: u64 = caps[4].parse().unwrap_or(0);
        if !seen.insert(format!("{source}/{skill_id}")) {
            continue;
        }
        let key = format!("{source}/{skill_id}");
        if let Some(skill) = map_skill(source, key, skill_id, name, installs) {
            skills.push(skill);
        }
        if skills.len() >= limit {
            break;
        }
    }
    skills
}

/// 把 skills.sh 原始字段映射为可安装技能。`source` 形如 `owner/repo`；
/// 非 GitHub 来源（含 `.` 的 owner/repo，如 `skills.volces.com`）被过滤掉。
fn map_skill(
    source: &str,
    key: String,
    skill_id: String,
    name: String,
    installs: u64,
) -> Option<SkillsShDiscoverableSkill> {
    let parts: Vec<&str> = source.splitn(2, '/').collect();
    if parts.len() != 2 {
        return None;
    }
    let (owner, repo) = (parts[0], parts[1]);
    if owner.contains('.') || repo.contains('.') {
        return None;
    }
    Some(SkillsShDiscoverableSkill {
        key,
        name,
        directory: skill_id,
        repo_owner: owner.to_string(),
        repo_name: repo.to_string(),
        repo_branch: "main".to_string(),
        installs,
        readme_url: Some(format!("https://github.com/{owner}/{repo}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_skill_filters_non_github_sources() {
        assert!(map_skill(
            "owner/repo",
            "k".into(),
            "dir".into(),
            "Name".into(),
            5
        )
        .is_some());
        // non-github (host-like) sources are dropped
        assert!(map_skill(
            "skills.volces.com/x",
            "k".into(),
            "dir".into(),
            "N".into(),
            1
        )
        .is_none());
        // missing slash
        assert!(map_skill("noslash", "k".into(), "d".into(), "N".into(), 1).is_none());
    }

    #[test]
    fn parse_popular_html_extracts_and_dedupes() {
        let html = r#"junk \"source\":\"foo/bar\",\"skillId\":\"baz\",\"name\":\"Baz Skill\",\"installs\":42 more
            \"source\":\"foo/bar\",\"skillId\":\"baz\",\"name\":\"Baz Skill\",\"installs\":42 dup
            \"source\":\"a/b\",\"skillId\":\"c\",\"name\":\"C\",\"installs\":7"#;
        let skills = parse_popular_html(html, 10);
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].repo_owner, "foo");
        assert_eq!(skills[0].repo_name, "bar");
        assert_eq!(skills[0].directory, "baz");
        assert_eq!(skills[0].installs, 42);
        assert_eq!(skills[0].repo_slug(), "foo/bar");
    }

    #[test]
    fn parse_popular_respects_limit() {
        let html = r#"\"source\":\"a/b\",\"skillId\":\"c1\",\"name\":\"N\",\"installs\":1
            \"source\":\"d/e\",\"skillId\":\"c2\",\"name\":\"N\",\"installs\":2"#;
        assert_eq!(parse_popular_html(html, 1).len(), 1);
    }
}
