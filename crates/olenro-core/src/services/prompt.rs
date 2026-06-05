//! Prompt service
//!
//! Business logic for prompt management

use crate::app_config::AppType;
use crate::database::dao::PromptsDao;
use crate::error::{AppError, AppResult};
use crate::prompt::{Prompt, CreatePromptInput};
use crate::prompt_files;
use std::path::PathBuf;

/// Prompt service for managing prompts
pub struct PromptService {
    db_path: PathBuf,
}

impl PromptService {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&PromptsDao) -> AppResult<T>,
    {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| AppError::Database(e))?;

        // Initialize schema if needed
        let schema = r#"
            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                description TEXT,
                enabled INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                app_type TEXT NOT NULL DEFAULT ''
            );
        "#;
        conn.execute_batch(schema)
            .map_err(|e| AppError::Database(e))?;

        let dao = PromptsDao::new(&conn);
        f(&dao)
    }

    /// List all prompts
    pub fn list_prompts(&self) -> AppResult<Vec<Prompt>> {
        self.with_conn(|dao| dao.list_all())
    }

    /// Get prompt by ID
    pub fn get_prompt(&self, id: &str) -> AppResult<Option<Prompt>> {
        self.with_conn(|dao| dao.get_by_id(id))
    }

    /// Create a new prompt
    pub fn create_prompt(&self, input: CreatePromptInput) -> AppResult<Prompt> {
        let now = chrono::Utc::now().timestamp();

        let prompt = Prompt {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            content: input.content,
            description: input.description,
            enabled: false, // New prompts are disabled by default
            created_at: now,
            updated_at: now,
        };

        self.with_conn(|dao| dao.insert(&prompt))?;
        Ok(prompt)
    }

    /// Update prompt content
    pub fn update_prompt(&self, id: &str, content: &str) -> AppResult<()> {
        if let Some(mut prompt) = self.get_prompt(id)? {
            prompt.content = content.to_string();
            prompt.updated_at = chrono::Utc::now().timestamp();
            self.with_conn(|dao| dao.update(&prompt))?;
        }
        Ok(())
    }

    /// Enable a prompt (disables others)
    pub fn enable_prompt(&self, id: &str, app_type: &AppType) -> AppResult<()> {
        // First disable all prompts
        self.with_conn(|dao| dao.disable_all())?;

        // Then enable this one
        if let Some(mut prompt) = self.get_prompt(id)? {
            prompt.enabled = true;
            prompt.updated_at = chrono::Utc::now().timestamp();
            self.with_conn(|dao| dao.update(&prompt))?;

            // Write to the app's prompt file
            prompt_files::write_prompt_file(app_type, &prompt.content)?;
        }
        Ok(())
    }

    /// Delete a prompt
    pub fn delete_prompt(&self, id: &str) -> AppResult<()> {
        self.with_conn(|dao| dao.delete(id))
    }

    /// Get prompt file content for an app
    pub fn get_prompt_file(&self, app_type: &AppType) -> AppResult<Option<String>> {
        prompt_files::read_prompt_file(app_type)
    }

    /// Set prompt content for an app (creates/updates prompt and syncs to file)
    pub fn set_prompt_file(&self, app_type: &AppType, content: &str) -> AppResult<()> {
        // Write directly to the file
        prompt_files::write_prompt_file(app_type, content)
    }
}