//! Core state
//!
//! Shared state for olenro-core

use crate::config::get_cli_database_path;
use crate::services::provider::ProviderService;
use std::path::PathBuf;
use std::sync::Arc;

/// Core state holding all services
pub struct CoreState {
    pub db_path: PathBuf,
    pub provider_service: Arc<ProviderService>,
}

impl CoreState {
    /// Create new core state
    pub fn new() -> Self {
        let db_path = get_cli_database_path();
        Self {
            db_path: db_path.clone(),
            provider_service: Arc::new(ProviderService::new(db_path)),
        }
    }

    /// Create with custom db path (for testing)
    pub fn with_db_path(db_path: PathBuf) -> Self {
        Self {
            db_path: db_path.clone(),
            provider_service: Arc::new(ProviderService::new(db_path)),
        }
    }
}

impl Default for CoreState {
    fn default() -> Self {
        Self::new()
    }
}
