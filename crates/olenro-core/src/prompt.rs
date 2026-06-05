//! Prompt data models
//!
//! Migrated from src-tauri/src/prompt.rs

use serde::{Deserialize, Serialize};

/// Prompt structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Create prompt input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptInput {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
}

/// Update prompt input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePromptInput {
    pub name: Option<String>,
    pub content: Option<String>,
    pub description: Option<String>,
}
