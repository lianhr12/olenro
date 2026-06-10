//! Provider commands

use crate::errors::Result;
use crate::output::print_table;
use crate::state::CliState;
use std::sync::Arc;

pub async fn list(app: Option<String>, state: &CliState) -> Result<()> {
    println!(
        "Listing providers{}...",
        app.map(|a| format!(" for {}", a))
            .unwrap_or_default()
            .as_str()
    );

    match state.core.provider_service.list_providers() {
        Ok(providers) => {
            if providers.is_empty() {
                println!("(empty)");
            } else {
                let rows: Vec<Vec<String>> = providers
                    .iter()
                    .map(|p| {
                        vec![
                            p.id.clone(),
                            p.name.clone(),
                            format!("{:?}", p.category),
                            p.website_url.clone().unwrap_or_default(),
                        ]
                    })
                    .collect();
                print_table(&["ID", "Name", "Category", "URL"], &rows);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}

pub async fn current(app: &str, state: &CliState) -> Result<()> {
    println!("Current provider for {}: (not implemented)", app);
    Ok(())
}

pub async fn add(
    name: &str,
    app: &str,
    endpoint: &str,
    api_key: Option<String>,
    state: &CliState,
) -> Result<()> {
    println!("Adding provider '{}' for {} at {}...", name, app, endpoint);

    let category = match app {
        "claude" | "claude-desktop" => olenro_core::provider::ProviderCategory::Official,
        "codex" => olenro_core::provider::ProviderCategory::Custom,
        "gemini" => olenro_core::provider::ProviderCategory::Official,
        _ => olenro_core::provider::ProviderCategory::Custom,
    };

    match state
        .core
        .provider_service
        .add_provider(name, endpoint, category, api_key.as_deref())
    {
        Ok(provider) => {
            println!("✓ Added provider: {} ({})", provider.name, provider.id);
        }
        Err(e) => {
            println!("✗ Failed to add provider: {}", e);
        }
    }
    Ok(())
}

pub async fn update(
    id: &str,
    name: Option<String>,
    endpoint: Option<String>,
    state: &CliState,
) -> Result<()> {
    println!("Updating provider {}...", id);

    // Fetch existing provider
    let existing = match state.core.provider_service.get_provider(id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            println!("✗ Provider {} not found", id);
            return Ok(());
        }
        Err(e) => {
            println!("✗ Error: {}", e);
            return Ok(());
        }
    };

    let mut updated = existing;
    if let Some(n) = name {
        updated.name = n;
    }
    if let Some(ep) = endpoint {
        if let serde_json::Value::Object(ref mut map) = updated.settings_config {
            map.insert("base_url".to_string(), serde_json::Value::String(ep));
        }
    }

    match state.core.provider_service.update_provider(updated) {
        Ok(_) => println!("✓ Updated provider {}", id),
        Err(e) => println!("✗ Failed to update: {}", e),
    }
    Ok(())
}

pub async fn delete(id: &str, state: &CliState) -> Result<()> {
    println!("Deleting provider {}...", id);

    match state.core.provider_service.delete_provider(id) {
        Ok(_) => println!("✓ Deleted provider {}", id),
        Err(e) => println!("✗ Failed to delete: {}", e),
    }
    Ok(())
}

pub async fn switch(id: &str, app: &str, state: &CliState) -> Result<()> {
    println!("Switching {} to provider {}...", app, id);

    // Parse app type
    let app_type = match app {
        "claude" => Some(olenro_core::provider::AppType::Claude),
        "claude-desktop" => Some(olenro_core::provider::AppType::ClaudeDesktop),
        "codex" => Some(olenro_core::provider::AppType::Codex),
        "gemini" => Some(olenro_core::provider::AppType::Gemini),
        "opencode" => Some(olenro_core::provider::AppType::OpenCode),
        "openclaw" => Some(olenro_core::provider::AppType::OpenClaw),
        "hermes" => Some(olenro_core::provider::AppType::Hermes),
        _ => None,
    };

    if app_type.is_none() {
        println!("✗ Unknown app type: {}", app);
        return Ok(());
    }

    match state
        .core
        .provider_service
        .switch_provider(id, app_type.unwrap())
    {
        Ok(_) => println!("✓ Switched {} to provider {}", app, id),
        Err(e) => println!("✗ Failed to switch: {}", e),
    }
    Ok(())
}
