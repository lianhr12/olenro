//! Olenro Core Library
//!
//! Core business logic for Olenro - Agent Workspace for AI Coding Tools.
//! This library provides the underlying functionality for both the desktop app
//! and the CLI, including provider management, MCP management, prompts, skills,
//! local HTTP proxy, and session management.

#![allow(unused)]
#![allow(rustdoc::private_intra_doc_links)]

pub mod error;
pub mod config;
pub mod provider;
pub mod app_config;
pub mod provider_defaults;
pub mod prompt;
pub mod prompt_files;
pub mod database;
pub mod services;
pub mod proxy;
pub mod deeplink;
pub mod session_manager;
pub mod app_config_writers;
pub mod core_state;

// Re-exports for convenience
pub use error::AppError;
pub use config::{get_app_config_dir, get_home_dir, DB_FILENAME};
pub use core_state::CoreState;
pub use services::provider::ProviderService;
