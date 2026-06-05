//! MCP DAO
//!
//! Data access for MCP servers table

use crate::app_config::McpServer;
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, Row};

pub struct McpDao<'a> {
    conn: &'a Connection,
}

impl<'a> McpDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all MCP servers
    pub fn list_all(&self) -> AppResult<Vec<McpServer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, args, env, url, enabled, created_at, updated_at FROM mcp_servers"
        )?;

        let servers = stmt
            .query_map([], |row| self.row_to_server(row))
            .map_err(|e| AppError::Database(e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(servers)
    }

    /// Get server by ID
    pub fn get_by_id(&self, id: &str) -> AppResult<Option<McpServer>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, args, env, url, enabled, created_at, updated_at FROM mcp_servers WHERE id = ?"
        )?;

        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::Database(e))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e))? {
            Ok(Some(self.row_to_server(row)?))
        } else {
            Ok(None)
        }
    }

    /// Insert a new server
    pub fn insert(&self, server: &McpServer) -> AppResult<()> {
        let args_json = serde_json::to_string(&server.args).unwrap_or_else(|_| "[]".to_string());
        let env_json = serde_json::to_string(&server.env).unwrap_or_else(|_| "{}".to_string());

        self.conn.execute(
            "INSERT INTO mcp_servers (id, name, command, args, env, url, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                server.id,
                server.name,
                server.command,
                args_json,
                env_json,
                server.url,
                server.enabled as i32,
                server.created_at,
                server.updated_at,
            ],
        ).map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Update an existing server
    pub fn update(&self, server: &McpServer) -> AppResult<()> {
        let args_json = serde_json::to_string(&server.args).unwrap_or_else(|_| "[]".to_string());
        let env_json = serde_json::to_string(&server.env).unwrap_or_else(|_| "{}".to_string());

        self.conn.execute(
            "UPDATE mcp_servers SET name = ?, command = ?, args = ?, env = ?, url = ?, enabled = ?, updated_at = ? WHERE id = ?",
            params![
                server.name,
                server.command,
                args_json,
                env_json,
                server.url,
                server.enabled as i32,
                server.updated_at,
                server.id,
            ],
        ).map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    /// Delete a server
    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.conn.execute("DELETE FROM mcp_servers WHERE id = ?", params![id])
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    fn row_to_server(&self, row: &Row) -> std::result::Result<McpServer, rusqlite::Error> {
        let args_str: String = row.get(3).unwrap_or_default();
        let env_str: String = row.get(4).unwrap_or_default();

        let args: Vec<String> = serde_json::from_str(&args_str).unwrap_or_default();
        let env: std::collections::HashMap<String, String> =
            serde_json::from_str(&env_str).unwrap_or_default();

        Ok(McpServer {
            id: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            command: row.get(2).unwrap_or_default(),
            args,
            env,
            url: row.get(5).ok(),
            enabled: row.get::<_, i32>(6).unwrap_or(1) != 0,
            created_at: row.get(7).unwrap_or(0),
            updated_at: row.get(8).unwrap_or(0),
        })
    }
}
