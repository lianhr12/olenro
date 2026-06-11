//! Skill commands

use crate::errors::Result;
use crate::output::print_table;
use crate::state::CliState;

pub async fn list(state: &CliState) -> Result<()> {
    println!("Listing installed skills...");

    match state.skill.list_skills() {
        Ok(skills) => {
            if skills.is_empty() {
                println!("(empty)");
            } else {
                let rows: Vec<Vec<String>> = skills
                    .iter()
                    .map(|s| {
                        let source_str = match &s.source {
                            olenro_core::app_config::SkillSource::GitHub { repo } => {
                                format!("github:{}", repo)
                            }
                            olenro_core::app_config::SkillSource::Zip { url } => {
                                format!("zip:{}", url)
                            }
                            olenro_core::app_config::SkillSource::Local { path } => {
                                format!("local:{}", path)
                            }
                        };
                        vec![s.id.clone(), s.name.clone(), s.version.clone(), source_str]
                    })
                    .collect();
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
    println!("Popular skills on skills.sh:\n");
    match olenro_core::skills_sh::popular(30).await {
        Ok(result) => {
            if result.skills.is_empty() {
                println!("(no skills found)");
            }
            for s in result.skills {
                println!(
                    "  {:<28} {:>7} installs   {}/{}",
                    s.name, s.installs, s.repo_owner, s.repo_name
                );
            }
            println!("\nInstall with: olenro skill install <owner/repo>");
        }
        Err(e) => println!("Failed to reach skills.sh: {e}"),
    }
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
