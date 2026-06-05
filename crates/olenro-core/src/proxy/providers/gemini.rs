//! Gemini adapter
//!
//! Provider adapter for Gemini API

use async_trait::async_trait;
use std::collections::HashMap;
use crate::proxy::providers::ProviderAdapter;

pub struct GeminiAdapter;

impl GeminiAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderAdapter for GeminiAdapter {
    fn name(&self) -> &str {
        "gemini"
    }

    fn extract_base_url(&self, config: &serde_json::Value) -> Option<String> {
        config
            .get("base_url")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| Some("https://generativelanguage.googleapis.com".to_string()))
    }

    fn extract_auth(&self, config: &serde_json::Value) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        if let Some(key) = config.get("api_key").and_then(|v| v.as_str()) {
            headers.insert("x-goog-api-key".to_string(), key.to_string());
        }

        headers
    }
}
