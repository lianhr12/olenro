//! Session commands

use crate::output::print_table;
use crate::errors::Result;

pub async fn list(app: Option<String>, limit: usize) -> Result<()> {
    println!("Listing sessions{} (limit {})...", app.map(|a| format!(" for {}", a)).unwrap_or_default(), limit);
    print_table(&["ID", "Name", "Updated", "Messages"], &[]);
    Ok(())
}

pub async fn show(id: &str, app: &str) -> Result<()> {
    println!("Showing session {} for {}...", id, app);
    Ok(())
}

pub async fn resume(id: &str, app: &str) -> Result<()> {
    println!("Resuming session {} for {}...", id, app);
    Ok(())
}
