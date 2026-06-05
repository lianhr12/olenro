//! Proxy server
//!
//! HTTP proxy server implementation using axum

use crate::error::{AppError, AppResult};
use crate::proxy::{ProxyConfig, ProxyEvent, ProxyStatus};
use axum::{
    Router,
    body::Body,
    extract::Request,
    response::Response,
    middleware,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

/// Proxy server state
pub struct ProxyState {
    pub config: ProxyConfig,
    pub status: ProxyStatus,
    notification_tx: Option<mpsc::Sender<ProxyEvent>>,
}

impl ProxyState {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            status: ProxyStatus::Stopped,
            notification_tx: None,
        }
    }

    /// Set notification channel for events
    pub fn set_notification_channel(&mut self, tx: mpsc::Sender<ProxyEvent>) {
        self.notification_tx = Some(tx);
    }
}

/// The main proxy server
pub struct ProxyServer {
    state: Arc<RwLock<ProxyState>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl ProxyServer {
    /// Create a new proxy server
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(ProxyState::new(config))),
            shutdown_tx: None,
        }
    }

    /// Start the proxy server
    pub async fn start(&mut self) -> AppResult<()> {
        let mut state = self.state.write().await;

        if state.status == ProxyStatus::Running {
            return Ok(());
        }

        state.status = ProxyStatus::Starting;

        // TODO: Build router and start axum server
        // For now, just mark as running
        state.status = ProxyStatus::Running;

        Ok(())
    }

    /// Stop the proxy server
    pub async fn stop(&mut self) -> AppResult<()> {
        let mut state = self.state.write().await;
        state.status = ProxyStatus::Stopping;
        // TODO: Send shutdown signal to server
        state.status = ProxyStatus::Stopped;
        Ok(())
    }

    /// Get current status
    pub async fn status(&self) -> ProxyStatus {
        self.state.read().await.status
    }

    /// Get current config
    pub async fn config(&self) -> ProxyConfig {
        self.state.read().await.config.clone()
    }
}

impl Default for ProxyServer {
    fn default() -> Self {
        Self::new(ProxyConfig::default())
    }
}
