//! Usage commands

use crate::errors::Result;

pub async fn summary(days: i32) -> Result<()> {
    println!("Usage summary (last {} days):", days);
    println!("  Requests: 0");
    println!("  Input tokens: 0");
    println!("  Output tokens: 0");
    println!("  Cost: $0.00");
    Ok(())
}

pub async fn trends(model: Option<String>) -> Result<()> {
    println!("Usage trends{}...", model.map(|m| format!(" for {}", m)).unwrap_or_default().as_str());
    Ok(())
}

pub async fn costs(provider: Option<String>) -> Result<()> {
    println!("Cost breakdown{}...", provider.map(|p| format!(" for provider {}", p)).unwrap_or_default().as_str());
    Ok(())
}
