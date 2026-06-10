//! Proxy module
//!
//! Local HTTP proxy server for AI API requests

pub mod circuit_breaker;
pub mod failover_switch;
pub mod forwarder;
pub mod handlers;
pub mod providers;
pub mod server;
pub mod types;

pub use server::ProxyServer;
pub use types::*;
