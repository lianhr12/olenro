//! MCP commands

use crate::errors::Result;
use crate::output::print_table;
use crate::state::CliState;
use std::collections::HashMap;

pub async fn list(app: Option<String>, state: &CliState) -> Result<()> {
    println!(
        "Listing MCP servers{}...",
        app.map(|a| format!(" for {}", a))
            .unwrap_or_default()
            .as_str()
    );

    match state.mcp.list_servers() {
        Ok(servers) => {
            if servers.is_empty() {
                println!("(empty)");
            } else {
                let rows: Vec<Vec<String>> = servers
                    .iter()
                    .map(|s| {
                        vec![
                            s.id.clone(),
                            s.name.clone(),
                            s.command.clone(),
                            if s.enabled { "✓" } else { "✗" }.to_string(),
                        ]
                    })
                    .collect();
                print_table(&["ID", "Name", "Command", "Enabled"], &rows);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}

pub async fn add(
    name: &str,
    command: &str,
    args: Option<String>,
    _app: Option<String>,
    state: &CliState,
) -> Result<()> {
    println!("Adding MCP server '{}' with {}...", name, command);

    let args_vec: Vec<String> = args
        .map(|a| a.split_whitespace().map(String::from).collect())
        .unwrap_or_default();

    match state
        .mcp
        .add_server(name, command, args_vec, HashMap::new(), None)
    {
        Ok(server) => {
            println!("✓ Added MCP server: {} ({})", server.name, server.id);
        }
        Err(e) => {
            println!("✗ Failed to add MCP server: {}", e);
        }
    }
    Ok(())
}

pub async fn delete(id: &str, state: &CliState) -> Result<()> {
    println!("Deleting MCP server {}...", id);

    match state.mcp.remove_server(id) {
        Ok(_) => println!("✓ Deleted MCP server {}", id),
        Err(e) => println!("✗ Failed to delete: {}", e),
    }
    Ok(())
}

pub async fn sync(app: &str, state: &CliState) -> Result<()> {
    println!("Syncing MCP servers to {}...", app);

    let app_type = match app {
        "claude" => Some(olenro_core::provider::AppType::Claude),
        "codex" => Some(olenro_core::provider::AppType::Codex),
        "gemini" => Some(olenro_core::provider::AppType::Gemini),
        "opencode" => Some(olenro_core::provider::AppType::OpenCode),
        "hermes" => Some(olenro_core::provider::AppType::Hermes),
        _ => None,
    };

    let app_type = match app_type {
        Some(t) => t,
        None => {
            println!("✗ Unknown app type: {}", app);
            return Ok(());
        }
    };

    match state.mcp.sync_to_app(&app_type) {
        Ok(_) => println!("✓ Synced MCP servers to {}", app),
        Err(e) => println!("✗ Failed to sync: {}", e),
    }
    Ok(())
}

pub async fn sync_all(state: &CliState) -> Result<()> {
    println!("Syncing MCP servers to all apps...");

    let apps = [
        ("claude", olenro_core::provider::AppType::Claude),
        ("codex", olenro_core::provider::AppType::Codex),
        ("gemini", olenro_core::provider::AppType::Gemini),
        ("opencode", olenro_core::provider::AppType::OpenCode),
        ("hermes", olenro_core::provider::AppType::Hermes),
    ];

    for (name, app_type) in apps.iter() {
        match state.mcp.sync_to_app(app_type) {
            Ok(_) => println!("✓ Synced to {}", name),
            Err(e) => println!("✗ Failed to sync to {}: {}", name, e),
        }
    }
    Ok(())
}
