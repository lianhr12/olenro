//! MCP service
//!
//! Business logic for MCP server management

use crate::app_config::{McpServer, AppType};
use crate::database::dao::McpDao;
use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::path::PathBuf;

/// MCP service for managing MCP servers
pub struct McpService {
    db_path: PathBuf,
}

impl McpService {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&McpDao) -> AppResult<T>,
    {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| AppError::Database(e))?;

        // Initialize schema if needed
        let schema = r#"
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL DEFAULT '[]',
                env TEXT NOT NULL DEFAULT '{}',
                url TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS mcp_app_mapping (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_type TEXT NOT NULL,
                server_id TEXT NOT NULL,
                UNIQUE(app_type, server_id)
            );
        "#;
        conn.execute_batch(schema)
            .map_err(|e| AppError::Database(e))?;

        let dao = McpDao::new(&conn);
        f(&dao)
    }

    /// List all MCP servers
    pub fn list_servers(&self) -> AppResult<Vec<McpServer>> {
        self.with_conn(|dao| dao.list_all())
    }

    /// Get server by ID
    pub fn get_server(&self, id: &str) -> AppResult<Option<McpServer>> {
        self.with_conn(|dao| dao.get_by_id(id))
    }

    /// Add a new MCP server
    pub fn add_server(&self, name: &str, command: &str, args: Vec<String>, env: HashMap<String, String>, url: Option<String>) -> AppResult<McpServer> {
        let now = chrono::Utc::now().timestamp();

        let server = McpServer {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            command: command.to_string(),
            args,
            env,
            url,
            enabled: true,
            created_at: now,
            updated_at: now,
        };

        self.with_conn(|dao| dao.insert(&server))?;
        Ok(server)
    }

    /// Remove an MCP server
    pub fn remove_server(&self, id: &str) -> AppResult<()> {
        self.with_conn(|dao| dao.delete(id))
    }

    /// Update an MCP server
    pub fn update_server(&self, server: &McpServer) -> AppResult<()> {
        self.with_conn(|dao| dao.update(server))
    }

    /// Enable/disable a server
    pub fn set_enabled(&self, id: &str, enabled: bool) -> AppResult<()> {
        if let Some(mut server) = self.get_server(id)? {
            server.enabled = enabled;
            server.updated_at = chrono::Utc::now().timestamp();
            self.with_conn(|dao| dao.update(&server))?;
        }
        Ok(())
    }

    /// Sync MCP servers to an app's config file
    pub fn sync_to_app(&self, app_type: &AppType) -> AppResult<()> {
        let servers = self.list_servers()?;
        let enabled_servers: Vec<&McpServer> = servers.iter().filter(|s| s.enabled).collect();

        // Build MCP config for the target app
        let mcp_config: Vec<serde_json::Value> = enabled_servers.iter().map(|s| {
            let mut obj = serde_json::Map::new();
            obj.insert("command".to_string(), serde_json::Value::String(s.command.clone()));
            if !s.args.is_empty() {
                obj.insert("args".to_string(), serde_json::json!(s.args));
            }
            if !s.env.is_empty() {
                obj.insert("env".to_string(), serde_json::json!(s.env));
            }
            if let Some(ref url) = s.url {
                obj.insert("url".to_string(), serde_json::Value::String(url.clone()));
            }
            serde_json::Value::Object(obj)
        }).collect();

        // Write to the app's MCP config file
        let config_path = self.get_app_mcp_path(app_type)?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::Io(e))?;
        }

        let content = serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": mcp_config
        })).map_err(|e| AppError::Json(e))?;

        std::fs::write(&config_path, content).map_err(|e| AppError::Io(e))?;

        eprintln!("Synced {} MCP servers to {:?}", enabled_servers.len(), app_type);
        Ok(())
    }

    fn get_app_mcp_path(&self, app_type: &AppType) -> AppResult<PathBuf> {
        let base = crate::config::get_home_dir();
        let path = match app_type {
            AppType::Claude => base.join(".claude.json"),
            AppType::Codex => base.join(".codex/mcp.json"),
            AppType::Gemini => base.join(".gemini/mcp.json"),
            AppType::OpenCode => base.join(".opencode/mcp.json"),
            AppType::Hermes => base.join(".hermes/mcp.json"),
            AppType::ClaudeDesktop | AppType::OpenClaw => {
                return Err(AppError::Mcp(format!("{:?} does not support MCP config", app_type)));
            }
        };
        Ok(path)
    }
}
