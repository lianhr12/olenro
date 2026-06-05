//! Deep link URL parser
//!
//! Parses olenro:// and legacy ccswitch:// URLs

use crate::error::{AppError, AppResult};
use crate::provider::AppType;
use serde::{Deserialize, Serialize};
use url::Url;

/// Deep link import request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepLinkImportRequest {
    /// Resource type (provider, mcp, prompt, skill)
    pub resource: String,
    /// Target app
    pub app: Option<AppType>,
    /// Resource name
    pub name: Option<String>,
    /// Additional data (JSON encoded)
    pub data: Option<serde_json::Value>,
}

impl DeepLinkImportRequest {
    pub fn new(resource: &str) -> Self {
        Self {
            resource: resource.to_string(),
            app: None,
            name: None,
            data: None,
        }
    }

    pub fn with_app(mut self, app: AppType) -> Self {
        self.app = Some(app);
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }
}

/// Parse a deep link URL
pub fn parse_deeplink_url(url_str: &str) -> AppResult<DeepLinkImportRequest> {
    // Support both olenro:// and legacy ccswitch://
    let url_str = if url_str.starts_with("ccswitch://") {
        url_str.replace("ccswitch://", "olenro://")
    } else if !url_str.starts_with("olenro://") {
        return Err(AppError::DeepLink(format!("Invalid URL scheme: {}", url_str)));
    } else {
        url_str.to_string()
    };

    let url = Url::parse(&url_str).map_err(|e| AppError::DeepLink(format!("URL parse error: {}", e)))?;

    // Extract path (e.g., "/v1/import/provider" -> "provider")
    let path_segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();
    let resource = path_segments.last().unwrap_or(&"").to_string();

    let mut request = DeepLinkImportRequest::new(&resource);

    // Parse query parameters
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "app" => {
                request.app = AppType::from_str(&value);
            }
            "name" => {
                request.name = Some(value.to_string());
            }
            _ => {}
        }
    }

    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_deeplink() {
        let result = parse_deeplink_url("olenro://import/provider").unwrap();
        assert_eq!(result.resource, "provider");
    }

    #[test]
    fn test_parse_deeplink_with_params() {
        let result = parse_deeplink_url("olenro://import/mcp?app=claude&name=test").unwrap();
        assert_eq!(result.resource, "mcp");
        assert_eq!(result.app, Some(AppType::Claude));
        assert_eq!(result.name, Some("test".to_string()));
    }

    #[test]
    fn test_parse_legacy_ccswitch_url() {
        let result = parse_deeplink_url("ccswitch://import/provider").unwrap();
        assert_eq!(result.resource, "provider");
    }
}
