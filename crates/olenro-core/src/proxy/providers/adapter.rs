//! Provider adapter trait
//!
//! Defines the interface for provider-specific request/response handling

use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;

/// Provider adapter for handling provider-specific logic
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Extract base URL from config
    fn extract_base_url(&self, config: &serde_json::Value) -> Option<String>;

    /// Extract auth headers from config
    fn extract_auth(&self, config: &serde_json::Value) -> HashMap<String, String>;

    /// Build request URL for a given path
    fn build_url(&self, base_url: &str, path: &str) -> String {
        format!("{}{}", base_url.trim_end_matches('/'), path)
    }

    /// Transform request before forwarding
    async fn transform_request(&self, request: Bytes, api_format: &str) -> AppResult<Bytes> {
        // Default: no transformation
        Ok(request)
    }

    /// Transform response before returning
    async fn transform_response(&self, response: Bytes, api_format: &str) -> AppResult<Bytes> {
        // Default: no transformation
        Ok(response)
    }
}

use crate::error::{AppError, AppResult};
