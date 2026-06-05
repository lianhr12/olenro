//! Doctor command - system diagnostics

use crate::errors::Result;
use olenro_core::config::{get_app_config_dir, get_home_dir};

pub async fn run() -> Result<()> {
    println!("Olenro System Diagnostics");
    println!("========================");
    println!();
    println!("Environment:");
    println!("  Home dir: {}", get_home_dir().display());
    println!("  Config dir: {}", get_app_config_dir().display());
    println!();

    // Check if config dir exists
    let config_dir = get_app_config_dir();
    if config_dir.exists() {
        println!("  Config dir: exists");
    } else {
        println!("  Config dir: not found (will be created)");
    }

    // Check database
    let db_path = config_dir.join("olenro.db");
    if db_path.exists() {
        println!("  Database: found");
    } else {
        println!("  Database: not found (will be created on first run)");
    }

    println!();
    println!("Status: OK");

    Ok(())
}
