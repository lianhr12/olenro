//! Proxy commands

use crate::errors::Result;
use crate::state::CliState;

pub async fn start(port: u16, address: &str, state: &CliState) -> Result<()> {
    println!("Starting proxy on {}:{}...", address, port);

    // Start the proxy
    match state.proxy.write().await.start().await {
        Ok(_) => println!("✓ Proxy started on http://{}:{}", address, port),
        Err(e) => println!("✗ Failed to start proxy: {}", e),
    }
    Ok(())
}

pub async fn stop(state: &CliState) -> Result<()> {
    println!("Stopping proxy...");

    match state.proxy.write().await.stop().await {
        Ok(_) => println!("✓ Proxy stopped"),
        Err(e) => println!("✗ Failed to stop proxy: {}", e),
    }
    Ok(())
}

pub async fn status(state: &CliState) -> Result<()> {
    let proxy = state.proxy.read().await;
    let status = proxy.status();
    let config = proxy.config();

    println!("Proxy status: {:?}", status);
    println!("  Address: {}", config.address);
    println!("  Port: {}", config.port);
    println!("  Enabled: {}", config.enabled);

    Ok(())
}

pub async fn takeover(app: &str, action: &str) -> Result<()> {
    println!("Setting {} takeover to {}...", app, action);
    println!("(Not yet implemented - would write to app's live config)");
    Ok(())
}

pub async fn failover(state: &CliState) -> Result<()> {
    let _ = state;
    let svc = olenro_core::services::provider::ProviderService::new(
        olenro_core::config::get_cli_database_path(),
    );
    println!("Failover queue:");
    match svc.list_failover_queue() {
        Ok(queue) if queue.is_empty() => println!("  (empty)"),
        Ok(queue) => {
            for (i, p) in queue.iter().enumerate() {
                println!("  {}. {}", i + 1, p.name);
            }
        }
        Err(e) => println!("  error: {e}"),
    }
    Ok(())
}
