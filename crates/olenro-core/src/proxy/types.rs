//! Proxy types
//!
//! Type definitions for the proxy server

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether the proxy is enabled
    pub enabled: bool,
    /// Port to listen on
    pub port: u16,
    /// Address to bind to
    pub address: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 15721,
            address: "127.0.0.1".to_string(),
        }
    }
}

/// Proxy status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

/// Active target for proxy forwarding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTarget {
    pub provider_id: String,
    pub base_url: String,
    pub api_format: String,
}

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStats {
    pub provider_id: String,
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub last_failure: Option<i64>,
}

/// Failover queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverQueueItem {
    pub provider_id: String,
    pub priority: i32,
    pub added_at: i64,
}

/// Proxy server events (for notification)
#[derive(Debug, Clone)]
pub enum ProxyEvent {
    /// A provider was switched
    ProviderSwitched { app: String, provider_id: String },
    /// Circuit breaker opened for a provider
    CircuitBreakerOpened { provider_id: String },
    /// Proxy error occurred
    Error { message: String },
}
