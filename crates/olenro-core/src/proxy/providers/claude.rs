//! Claude adapter
//!
//! Provider adapter for Claude API

use crate::error::{AppError, AppResult};
use crate::proxy::providers::ProviderAdapter;
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;

pub struct ClaudeAdapter;

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderAdapter for ClaudeAdapter {
    fn name(&self) -> &str {
        "claude"
    }

    fn extract_base_url(&self, config: &serde_json::Value) -> Option<String> {
        config
            .get("env")
            .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| Some("https://api.anthropic.com".to_string()))
    }

    fn extract_auth(&self, config: &serde_json::Value) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        if let Some(token) = config
            .get("env")
            .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
            .and_then(|v| v.as_str())
        {
            headers.insert("x-api-key".to_string(), token.to_string());
            headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        }

        headers
    }
}
