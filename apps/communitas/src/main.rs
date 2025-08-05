//! Communitas - P2P Diagnostic Chat Application
//!
//! This will be replaced with Tauri setup in Task 2

use anyhow::Result;
use clap::Parser;
use communitas::CommuniasApp;

/// CLI Arguments
#[derive(Debug, Parser)]
#[command(name = "communitas")]
#[command(about = "P2P Diagnostic Chat Application", long_about = None)]
struct Args {
    /// Bootstrap node address
    #[arg(short, long, default_value = "/ip6/::1/tcp/9000")]
    bootstrap: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("communitas=debug,saorsa_core=info")
        .init();
    
    // Parse arguments
    let args = Args::parse();
    
    tracing::info!("Starting Communitas...");
    tracing::info!("Bootstrap node: {}", args.bootstrap);
    
    // Create app instance
    let app = CommuniasApp::new(args.bootstrap).await?;
    
    // Connect to network
    app.connect().await?;
    
    // Get network health
    let health = app.get_network_health().await;
    tracing::info!("Network health: {:?}", health);
    
    // Note: This is a placeholder - Tauri v2 setup coming in Task 2
    tracing::info!("Note: GUI will be implemented in Task 2 with Tauri v2");
    
    // Run for a bit to demonstrate functionality
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    tracing::info!("Shutting down...");
    
    Ok(())
}
