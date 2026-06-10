//! Doctor command - system diagnostics

use crate::errors::Result;
use olenro_core::config::{get_cli_config_dir, get_cli_database_path, get_home_dir};

pub async fn run() -> Result<()> {
    println!("Olenro System Diagnostics");
    println!("========================");
    println!();
    println!("Environment:");
    println!("  Home dir: {}", get_home_dir().display());
    println!("  CLI config dir: {}", get_cli_config_dir().display());
    println!();

    // Check if config dir exists
    let config_dir = get_cli_config_dir();
    if config_dir.exists() {
        println!("  CLI config dir: exists");
    } else {
        println!("  CLI config dir: not found (will be created)");
    }

    // Check database
    let db_path = get_cli_database_path();
    if db_path.exists() {
        println!("  CLI database: found");
    } else {
        println!("  CLI database: not found (will be created on first run)");
    }

    println!();
    println!("Status: OK");

    Ok(())
}
