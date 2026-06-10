//! App config writers module
//!
//! Handles writing configuration to each AI tool's config directory

pub mod claude_config;
pub mod codex_config;
pub mod gemini_config;
pub mod hermes_config;
pub mod openclaw_config;
pub mod opencode_config;

pub use claude_config::ClaudeDesktopConfig;
