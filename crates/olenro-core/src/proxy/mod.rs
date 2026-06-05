//! Proxy module
//!
//! Local HTTP proxy server for AI API requests

pub mod types;
pub mod server;
pub mod forwarder;
pub mod handlers;
pub mod circuit_breaker;
pub mod failover_switch;
pub mod providers;

pub use types::*;
pub use server::ProxyServer;
