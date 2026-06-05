//! Proxy service
//!
//! Local HTTP proxy server for AI API requests

use crate::error::{AppError, AppResult};
use crate::proxy::{ProxyConfig, ProxyEvent, ProxyStatus};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};

/// Shared proxy service
pub type SharedProxyService = Arc<RwLock<ProxyService>>;

/// Proxy service for managing the local HTTP proxy
pub struct ProxyService {
    config: ProxyConfig,
    status: ProxyStatus,
    shutdown_tx: Option<mpsc::Sender<()>>,
    event_tx: Option<mpsc::Sender<ProxyEvent>>,
}

impl ProxyService {
    /// Create a new proxy service
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            status: ProxyStatus::Stopped,
            shutdown_tx: None,
            event_tx: None,
        }
    }

    /// Set event channel for proxy events
    #[allow(dead_code)]
    pub fn set_event_channel(&mut self, tx: mpsc::Sender<ProxyEvent>) {
        self.event_tx = Some(tx);
    }

    /// Start the proxy server
    pub async fn start(&mut self) -> AppResult<()> {
        if self.status == ProxyStatus::Running {
            return Ok(());
        }

        self.status = ProxyStatus::Starting;

        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.port));
        let listener = TcpListener::bind(addr).await
            .map_err(|e| AppError::Proxy(format!("Failed to bind to {}: {}", addr, e)))?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        self.status = ProxyStatus::Running;

        // Spawn the server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                eprintln!("Proxy connection from {}", addr);
                                // Handle connection asynchronously
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream).await {
                                        eprintln!("Proxy connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("Accept error: {}", e);
                            }
                        }
                    }
                }
            }
        });

        eprintln!("Proxy server started on http://{}", addr);
        Ok(())
    }

    /// Stop the proxy server
    pub async fn stop(&mut self) -> AppResult<()> {
        if self.status != ProxyStatus::Running {
            return Ok(());
        }

        self.status = ProxyStatus::Stopping;

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        self.status = ProxyStatus::Stopped;
        eprintln!("Proxy server stopped");
        Ok(())
    }

    /// Get current status
    pub fn status(&self) -> ProxyStatus {
        self.status
    }

    /// Get current config
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

/// Handle a proxy connection
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    // Parse the request
    let _request = String::from_utf8_lossy(&buf[..n]).to_string();

    // For now, send a simple OK response indicating proxy is running
    // In full implementation, this would forward to the actual AI provider
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 18\r\n\r\nProxy Running!";
    stream.write_all(response).await?;

    Ok(())
}
