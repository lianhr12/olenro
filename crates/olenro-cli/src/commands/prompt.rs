//! Prompt commands

use crate::errors::Result;
use crate::output::print_table;
use crate::state::CliState;
use olenro_core::provider::AppType;
use std::collections::HashMap;

pub async fn list(_app: Option<String>, state: &CliState) -> Result<()> {
    println!("Listing prompts...");

    match state.prompt.list_prompts() {
        Ok(prompts) => {
            if prompts.is_empty() {
                println!("(empty)");
            } else {
                let rows: Vec<Vec<String>> = prompts
                    .iter()
                    .map(|p| {
                        vec![
                            p.id.clone(),
                            p.name.clone(),
                            if p.enabled { "✓" } else { "✗" }.to_string(),
                            format!("{} chars", p.content.len()),
                        ]
                    })
                    .collect();
                print_table(&["ID", "Name", "Enabled", "Size"], &rows);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}

pub async fn set(
    content: &str,
    _file: Option<String>,
    app: Option<String>,
    state: &CliState,
) -> Result<()> {
    let app_str = app.as_deref().unwrap_or("claude");

    let app_type = match app_str {
        "claude" => AppType::Claude,
        "codex" => AppType::Codex,
        "gemini" => AppType::Gemini,
        "opencode" => AppType::OpenCode,
        "openclaw" => AppType::OpenClaw,
        "hermes" => AppType::Hermes,
        _ => {
            println!("✗ Unknown app type: {}", app_str);
            return Ok(());
        }
    };

    match state.prompt.set_prompt_file(&app_type, content) {
        Ok(_) => println!("✓ Prompt set for {} ({} chars)", app_str, content.len()),
        Err(e) => println!("✗ Failed to set prompt: {}", e),
    }
    Ok(())
}

pub async fn get(_file: Option<String>, app: Option<String>, state: &CliState) -> Result<()> {
    let app_str = app.as_deref().unwrap_or("claude");

    let app_type = match app_str {
        "claude" => AppType::Claude,
        "codex" => AppType::Codex,
        "gemini" => AppType::Gemini,
        "opencode" => AppType::OpenCode,
        "openclaw" => AppType::OpenClaw,
        "hermes" => AppType::Hermes,
        _ => {
            println!("✗ Unknown app type: {}", app_str);
            return Ok(());
        }
    };

    match state.prompt.get_prompt_file(&app_type) {
        Ok(Some(content)) => {
            println!("Prompt for {}:\n{}", app_str, content);
        }
        Ok(None) => {
            println!("No prompt file found for {}", app_str);
        }
        Err(e) => {
            println!("✗ Failed to get prompt: {}", e);
        }
    }
    Ok(())
}

pub async fn delete(id: &str, state: &CliState) -> Result<()> {
    println!("Deleting prompt {}...", id);

    match state.prompt.delete_prompt(id) {
        Ok(_) => println!("✓ Deleted prompt {}", id),
        Err(e) => println!("✗ Failed to delete: {}", e),
    }
    Ok(())
}
