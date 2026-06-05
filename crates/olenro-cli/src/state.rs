//! CLI state
//!
//! Global state for the CLI

use olenro_core::{CoreState, proxy::ProxyConfig, services::proxy::ProxyService, services::mcp::McpService, services::prompt::PromptService, services::skill::SkillService, config::get_app_config_dir};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global CLI state
pub struct CliState {
    pub core: Arc<CoreState>,
    pub proxy: Arc<RwLock<ProxyService>>,
    pub mcp: Arc<McpService>,
    pub prompt: Arc<PromptService>,
    pub skill: Arc<SkillService>,
}

impl CliState {
    /// Create new CLI state
    pub fn new() -> Self {
        let config = ProxyConfig {
            enabled: true,
            port: 15721,
            address: "127.0.0.1".to_string(),
        };
        let db_path = get_app_config_dir().join("olenro.db");
        Self {
            core: Arc::new(CoreState::new()),
            proxy: Arc::new(RwLock::new(ProxyService::new(config))),
            mcp: Arc::new(McpService::new(db_path.clone())),
            prompt: Arc::new(PromptService::new(db_path.clone())),
            skill: Arc::new(SkillService::new(db_path)),
        }
    }
}

impl Default for CliState {
    fn default() -> Self {
        Self::new()
    }
}
