//! Skill commands

use crate::output::print_table;
use crate::state::CliState;
use crate::errors::Result;

pub async fn list(state: &CliState) -> Result<()> {
    println!("Listing installed skills...");

    match state.skill.list_skills() {
        Ok(skills) => {
            if skills.is_empty() {
                println!("(empty)");
            } else {
                let rows: Vec<Vec<String>> = skills.iter().map(|s| {
                    let source_str = match &s.source {
                        olenro_core::app_config::SkillSource::GitHub { repo } => format!("github:{}", repo),
                        olenro_core::app_config::SkillSource::Zip { url } => format!("zip:{}", url),
                        olenro_core::app_config::SkillSource::Local { path } => format!("local:{}", path),
                    };
                    vec![
                        s.id.clone(),
                        s.name.clone(),
                        s.version.clone(),
                        source_str,
                    ]
                }).collect();
                print_table(&["ID", "Name", "Version", "Source"], &rows);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}

pub async fn discover() -> Result<()> {
    println!("Discovering available skills...");
    println!("(Discovery not yet implemented - would query skill registry)");
    Ok(())
}

pub async fn install(repo: &str, _apps: Option<String>, state: &CliState) -> Result<()> {
    println!("Installing skill from '{}'...", repo);

    match state.skill.install_from_github(repo) {
        Ok(skill) => {
            println!("✓ Installed skill: {} ({})", skill.name, skill.id);
        }
        Err(e) => {
            println!("✗ Failed to install skill: {}", e);
        }
    }
    Ok(())
}

pub async fn uninstall(id: &str, state: &CliState) -> Result<()> {
    println!("Uninstalling skill {}...", id);

    match state.skill.uninstall(id) {
        Ok(_) => println!("✓ Uninstalled skill {}", id),
        Err(e) => println!("✗ Failed to uninstall: {}", e),
    }
    Ok(())
}

pub async fn update(id: &str, state: &CliState) -> Result<()> {
    println!("Updating skill {}...", id);

    match state.skill.update(id) {
        Ok(_) => println!("✓ Updated skill {}", id),
        Err(e) => println!("✗ Failed to update: {}", e),
    }
    Ok(())
}

pub async fn sync(_app: &str, _state: &CliState) -> Result<()> {
    println!("Syncing skills to {}...", _app);
    println!("(Skill sync not yet implemented)");
    Ok(())
}