//! Skill commands

use crate::errors::Result;
use crate::output::print_table;
use crate::state::CliState;
use olenro_core::provider::AppType;

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
                        let source = match (&s.repo_owner, &s.repo_name) {
                            (Some(o), Some(n)) => format!("{o}/{n}"),
                            _ => "local".to_string(),
                        };
                        let apps = s
                            .apps
                            .enabled_apps()
                            .iter()
                            .map(|a| a.as_str().to_string())
                            .collect::<Vec<_>>()
                            .join(",");
                        vec![
                            s.id.clone(),
                            s.name.clone(),
                            s.directory.clone(),
                            source,
                            apps,
                        ]
                    })
                    .collect();
                print_table(&["ID", "Name", "Directory", "Source", "Apps"], &rows);
            }
        }
        Err(e) => println!("Error: {}", e),
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

pub async fn install(repo: &str, apps: Option<String>, state: &CliState) -> Result<()> {
    let current_app = apps
        .as_deref()
        .and_then(AppType::from_str)
        .unwrap_or(AppType::Claude);
    println!(
        "Installing skill from '{}' for {}...",
        repo,
        current_app.as_str()
    );

    match state.skill.install_from_github(repo, &current_app).await {
        Ok(skills) => {
            for skill in &skills {
                println!("✓ Installed skill: {} ({})", skill.name, skill.id);
            }
            println!("Installed {} skill(s).", skills.len());
        }
        Err(e) => println!("✗ Failed to install skill: {}", e),
    }
    Ok(())
}

pub async fn uninstall(id: &str, state: &CliState) -> Result<()> {
    println!("Uninstalling skill {}...", id);

    match state.skill.uninstall(id) {
        Ok(res) => {
            print!("✓ Uninstalled skill {}", id);
            match res.backup_path {
                Some(path) => println!(" (backup: {path})"),
                None => println!(),
            }
        }
        Err(e) => println!("✗ Failed to uninstall: {}", e),
    }
    Ok(())
}

pub async fn update(id: &str, state: &CliState) -> Result<()> {
    println!("Updating skill {}...", id);

    match state.skill.update_skill(id).await {
        Ok(skill) => println!("✓ Updated skill {} ({})", skill.name, skill.id),
        Err(e) => println!("✗ Failed to update: {}", e),
    }
    Ok(())
}

pub async fn sync(app: &str, state: &CliState) -> Result<()> {
    let Some(app_type) = AppType::from_str(app) else {
        println!("✗ Unknown app: {app}");
        return Ok(());
    };
    println!("Syncing skills to {app}...");
    match state.skill.sync_to_app(&app_type) {
        Ok(n) => println!("✓ Synced {n} skill(s) to {app}"),
        Err(e) => println!("✗ Sync failed: {e}"),
    }
    Ok(())
}

pub async fn toggle(id: &str, app: &str, enabled: bool, state: &CliState) -> Result<()> {
    let Some(app_type) = AppType::from_str(app) else {
        println!("✗ Unknown app: {app}");
        return Ok(());
    };
    match state.skill.toggle_app(id, &app_type, enabled) {
        Ok(_) => println!(
            "✓ {} skill {id} for {app}",
            if enabled { "Enabled" } else { "Disabled" }
        ),
        Err(e) => println!("✗ Toggle failed: {e}"),
    }
    Ok(())
}

pub async fn scan(state: &CliState) -> Result<()> {
    println!("Scanning for unmanaged skills...");
    match state.skill.scan_unmanaged() {
        Ok(skills) if skills.is_empty() => println!("(none)"),
        Ok(skills) => {
            let rows: Vec<Vec<String>> = skills
                .iter()
                .map(|s| vec![s.directory.clone(), s.name.clone(), s.found_in.join(",")])
                .collect();
            print_table(&["Directory", "Name", "Found In"], &rows);
            println!("\nImport with: olenro skill import <directory> --apps <app>");
        }
        Err(e) => println!("✗ Scan failed: {e}"),
    }
    Ok(())
}

pub async fn import(directory: &str, apps: Option<String>, state: &CliState) -> Result<()> {
    let app = apps
        .as_deref()
        .and_then(AppType::from_str)
        .unwrap_or(AppType::Claude);
    let mut selection_apps = olenro_core::app_config::SkillApps::default();
    selection_apps.set_enabled_for(&app, true);
    let selection = olenro_core::services::skill::ImportSkillSelection {
        directory: directory.to_string(),
        apps: selection_apps,
    };
    match state.skill.import_from_apps(vec![selection]) {
        Ok(imported) if imported.is_empty() => println!("✗ Nothing imported (not found?)"),
        Ok(imported) => {
            for s in &imported {
                println!("✓ Imported {} ({})", s.name, s.id);
            }
        }
        Err(e) => println!("✗ Import failed: {e}"),
    }
    Ok(())
}

pub async fn check_updates(state: &CliState) -> Result<()> {
    println!("Checking for skill updates...");
    match state.skill.check_updates().await {
        Ok(updates) if updates.is_empty() => println!("✓ All skills up to date"),
        Ok(updates) => {
            for u in &updates {
                println!("  {} ({}) — update available", u.name, u.id);
            }
            println!("\nUpdate with: olenro skill update <id>");
        }
        Err(e) => println!("✗ Check failed: {e}"),
    }
    Ok(())
}

pub async fn backups(state: &CliState) -> Result<()> {
    let _ = state;
    match olenro_core::services::skill::SkillService::list_backups() {
        Ok(entries) if entries.is_empty() => println!("(no backups)"),
        Ok(entries) => {
            let rows: Vec<Vec<String>> = entries
                .iter()
                .map(|e| {
                    vec![
                        e.backup_id.clone(),
                        e.skill.name.clone(),
                        e.created_at.to_string(),
                    ]
                })
                .collect();
            print_table(&["Backup ID", "Skill", "Created At"], &rows);
            println!("\nRestore with: olenro skill restore <backup-id>");
        }
        Err(e) => println!("✗ Failed to list backups: {e}"),
    }
    Ok(())
}

pub async fn restore(backup_id: &str, apps: Option<String>, state: &CliState) -> Result<()> {
    let app = apps
        .as_deref()
        .and_then(AppType::from_str)
        .unwrap_or(AppType::Claude);
    match state.skill.restore_from_backup(backup_id, &app) {
        Ok(s) => println!("✓ Restored {} ({})", s.name, s.id),
        Err(e) => println!("✗ Restore failed: {e}"),
    }
    Ok(())
}

pub async fn repo_list(state: &CliState) -> Result<()> {
    match state.skill.list_repos() {
        Ok(repos) if repos.is_empty() => println!("(no repos)"),
        Ok(repos) => {
            let rows: Vec<Vec<String>> = repos
                .iter()
                .map(|r| {
                    vec![
                        r.owner.clone(),
                        r.name.clone(),
                        r.branch.clone(),
                        if r.enabled { "yes" } else { "no" }.to_string(),
                    ]
                })
                .collect();
            print_table(&["Owner", "Name", "Branch", "Enabled"], &rows);
        }
        Err(e) => println!("✗ Failed to list repos: {e}"),
    }
    Ok(())
}

pub async fn repo_add(
    owner: &str,
    name: &str,
    branch: Option<String>,
    state: &CliState,
) -> Result<()> {
    let repo = olenro_core::services::skill::SkillRepo {
        owner: owner.to_string(),
        name: name.to_string(),
        branch: branch.unwrap_or_else(|| "main".to_string()),
        enabled: true,
    };
    match state.skill.add_repo(&repo) {
        Ok(_) => println!("✓ Added repo {owner}/{name}"),
        Err(e) => println!("✗ Failed to add repo: {e}"),
    }
    Ok(())
}

pub async fn repo_remove(owner: &str, name: &str, state: &CliState) -> Result<()> {
    match state.skill.remove_repo(owner, name) {
        Ok(_) => println!("✓ Removed repo {owner}/{name}"),
        Err(e) => println!("✗ Failed to remove repo: {e}"),
    }
    Ok(())
}
