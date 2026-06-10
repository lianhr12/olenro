//! Usage commands

use crate::errors::{CliError, Result};
use olenro_core::config::get_cli_database_path;
use olenro_core::services::usage::UsageService;

fn service() -> UsageService {
    UsageService::new(get_cli_database_path())
}

/// Format a USD cost with adaptive precision so small amounts stay visible.
fn fmt_cost(cost: f64) -> String {
    if cost >= 1.0 {
        format!("${:.2}", cost)
    } else if cost >= 0.01 {
        format!("${:.4}", cost)
    } else {
        format!("${:.6}", cost)
    }
}

pub async fn summary(days: i32) -> Result<()> {
    let s = service()
        .summary(days)
        .map_err(|e| CliError::Failed(e.to_string()))?;
    println!("Usage summary (last {} days):", days);
    println!("  Requests:      {}", s.requests);
    println!("  Input tokens:  {}", s.input_tokens);
    println!("  Output tokens: {}", s.output_tokens);
    println!("  Cost:          {}", fmt_cost(s.cost));
    if s.requests == 0 {
        println!(
            "\n(No usage recorded yet — usage accrues once the local proxy records requests.)"
        );
    }
    Ok(())
}

pub async fn trends(model: Option<String>) -> Result<()> {
    let rollups = service()
        .list_recent(30)
        .map_err(|e| CliError::Failed(e.to_string()))?;
    let scope = model.map(|m| format!(" for {}", m)).unwrap_or_default();
    println!("Usage trends{} (last 30 days):", scope);
    if rollups.is_empty() {
        println!("  No data.");
        return Ok(());
    }
    println!(
        "  {:<12} {:<24} {:>10} {:>12}",
        "Date", "Provider", "Requests", "Cost"
    );
    for r in rollups {
        println!(
            "  {:<12} {:<24} {:>10} {:>12}",
            r.date,
            r.provider_id,
            r.requests,
            fmt_cost(r.cost)
        );
    }
    Ok(())
}

pub async fn costs(provider: Option<String>) -> Result<()> {
    let by_provider = service()
        .by_provider(30)
        .map_err(|e| CliError::Failed(e.to_string()))?;
    println!("Cost breakdown (last 30 days):");
    let filtered: Vec<_> = by_provider
        .into_iter()
        .filter(|p| {
            provider
                .as_ref()
                .map(|f| &p.provider_id == f)
                .unwrap_or(true)
        })
        .collect();
    if filtered.is_empty() {
        println!("  No data.");
        return Ok(());
    }
    for p in filtered {
        println!(
            "  {:<24} requests={:<8} cost={}",
            p.provider_id,
            p.requests,
            fmt_cost(p.cost)
        );
    }
    Ok(())
}
