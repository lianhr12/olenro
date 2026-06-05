//! Forwarder
//!
//! Request forwarding logic

use crate::error::AppResult;
use bytes::Bytes;

/// Forwarder for proxying requests
pub struct Forwarder;

impl Forwarder {
    pub fn new() -> Self {
        Self
    }

    /// Forward a request to the target
    pub async fn forward(&self, _request: Bytes, _target: &str) -> AppResult<Bytes> {
        // TODO: Implement actual forwarding logic
        Ok(Bytes::new())
    }
}

impl Default for Forwarder {
    fn default() -> Self {
        Self::new()
    }
}
