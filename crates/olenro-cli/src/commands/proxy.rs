//! Proxy commands

use crate::state::CliState;
use crate::errors::Result;

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
    println!("Failover queue:");
    println!("(empty - no failover configured)");
    Ok(())
}
