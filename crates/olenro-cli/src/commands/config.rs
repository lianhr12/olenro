//! Config commands

use crate::errors::Result;
use olenro_core::config::get_cli_config_dir;

pub async fn handle(app: Option<String>) -> Result<()> {
    println!(
        "Configuration{}...",
        app.map(|a| format!(" for {}", a))
            .unwrap_or_default()
            .as_str()
    );
    println!("  CLI config dir: {}", get_cli_config_dir().display());
    Ok(())
}

pub async fn set(key: &str, value: &str, app: Option<String>) -> Result<()> {
    println!(
        "Setting {}={}{}...",
        key,
        value,
        app.map(|a| format!(" for {}", a))
            .unwrap_or_default()
            .as_str()
    );
    Ok(())
}

pub async fn dir() -> Result<()> {
    println!("{}", get_cli_config_dir().display());
    Ok(())
}
