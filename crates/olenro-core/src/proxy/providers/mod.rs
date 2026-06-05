//! Providers
//!
//! Provider-specific adapter implementations

pub mod adapter;
pub mod claude;
pub mod codex;
pub mod gemini;

pub use adapter::ProviderAdapter;
