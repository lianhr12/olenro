//! Session manager
//!
//! Provides session management across different AI coding tools

use crate::error::AppResult;
use crate::provider::AppType;
use serde::{Deserialize, Serialize};

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub app_type: AppType,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: usize,
}

/// Session manager for browsing and restoring sessions
pub struct SessionManager;

impl SessionManager {
    pub fn new() -> Self {
        Self
    }

    /// List sessions for an app
    pub fn list_sessions(&self, app_type: &AppType) -> AppResult<Vec<Session>> {
        Ok(vec![])
    }

    /// Get a session by ID
    pub fn get_session(&self, app_type: &AppType, session_id: &str) -> AppResult<Option<Session>> {
        Ok(None)
    }

    /// Delete a session
    pub fn delete_session(&self, app_type: &AppType, session_id: &str) -> AppResult<()> {
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
