//! Codex adapter
//!
//! Provider adapter for Codex API

use crate::proxy::providers::ProviderAdapter;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderAdapter for CodexAdapter {
    fn name(&self) -> &str {
        "codex"
    }

    fn extract_base_url(&self, config: &serde_json::Value) -> Option<String> {
        config
            .get("base_url")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| Some("https://api.openai.com".to_string()))
    }

    fn extract_auth(&self, config: &serde_json::Value) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        if let Some(key) = config.get("api_key").and_then(|v| v.as_str()) {
            headers.insert("authorization".to_string(), format!("Bearer {}", key));
        }

        headers
    }
}
