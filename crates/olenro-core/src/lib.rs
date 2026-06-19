//! Olenro Core Library
//!
//! Core business logic for Olenro - Agent Workspace for AI Coding Tools.
//! This library provides the underlying functionality for both the desktop app
//! and the CLI, including provider management, MCP management, prompts, skills,
//! local HTTP proxy, and session management.

#![allow(unused)]
#![allow(rustdoc::private_intra_doc_links)]

pub mod app_config;
pub mod app_config_writers;
pub mod claude_agents;
pub mod config;
pub mod core_state;
pub mod database;
pub mod deeplink;
pub mod error;
pub mod git_sync;
pub mod openclaw_workspace;
pub mod pricing;
pub mod prompt;
pub mod prompt_files;
pub mod provider;
pub mod provider_defaults;
pub mod provider_presets;
pub mod proxy;
pub mod services;
pub mod session_manager;
pub mod settings;
pub mod skills_sh;

// Re-exports for convenience
pub use config::{
    get_app_config_dir, get_app_database_path, get_cli_config_dir, get_cli_database_path,
    get_home_dir, CLI_DB_FILENAME, DB_FILENAME,
};
pub use core_state::CoreState;
pub use error::AppError;
pub use services::provider::ProviderService;
