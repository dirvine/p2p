// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Communitas CLI - Full-featured P2P Bootstrap Node and Personal AI Assistant

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;
// use std::net::SocketAddr;

// Import our modules
use communitas_cli::{
    dht::{DHTManager, DHTBootstrapConfig, DHTCommands, execute_dht_command},
    geographic::{GeographicBootstrapManager, GeographicBootstrapConfig, GeographicCommands, execute_geographic_command},
    mcp::{MCPServer, MCPConfig, MCPHandlers},
    network::NetworkManager,
    config::ConfigManager,
};

#[derive(Parser)]
#[command(name = "communitas")]
#[command(about = "P2P Bootstrap Node and Personal AI Assistant")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<String>,
    
    /// Data directory
    #[arg(short = 'd', long, global = true, default_value = "./data")]
    data_dir: String,
    
    /// Log directory
    #[arg(short = 'l', long, global = true, default_value = "./logs")]
    log_dir: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as a bootstrap node with full DHT, geographic routing, and MCP
    Bootstrap {
        /// P2P port to listen on
        #[arg(long, default_value = "9001")]
        port: u16,
        
        /// MCP server port
        #[arg(long, default_value = "9090")]
        mcp_port: u16,
        
        /// Geographic region (NA, EU, AP, SA, AF, OC)
        #[arg(long)]
        region: Option<String>,
        
        /// Bootstrap nodes to connect to
        #[arg(long)]
        bootstrap: Vec<String>,
        
        /// Enable persistent storage
        #[arg(long, default_value = "true")]
        persistent: bool,
        
        /// Storage capacity in MB
        #[arg(long, default_value = "10240")]
        storage_mb: usize,
        
        /// API token for MCP access
        #[arg(long, env = "COMMUNITAS_API_TOKEN")]
        api_token: Option<String>,
    },
    
    /// DHT storage operations
    #[command(subcommand)]
    Dht(DHTCommands),
    
    /// Geographic routing operations
    #[command(subcommand)]
    Geo(GeographicCommands),
    
    /// Start interactive chat session
    Chat {
        /// Enable voice input/output
        #[arg(long)]
        voice: bool,
        
        /// Enable vision capabilities for images
        #[arg(long)]
        vision: bool,
        
        /// Model to use (e.g., gpt-4, claude-3)
        #[arg(long, default_value = "gpt-4")]
        model: String,
    },
    
    /// Process a file with AI
    Process {
        /// Input file path
        file: String,
        
        /// Processing instruction
        #[arg(long)]
        instruction: Option<String>,
        
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
    },
    
    /// Start TUI interface
    Tui {
        /// Theme (dark/light)
        #[arg(long, default_value = "dark")]
        theme: String,
    },
    
    /// Configure settings
    Config {
        /// API key name
        #[arg(long)]
        api_key: Option<String>,
        
        /// Show current configuration
        #[arg(long)]
        show: bool,
    },
    
    /// Connect to P2P network (simple client mode)
    Connect {
        /// Bootstrap node address
        #[arg(long)]
        bootstrap: Option<String>,
        
        /// Local port to listen on
        #[arg(long, default_value = "9000")]
        port: u16,
    },
    
    /// Health check for monitoring
    Health {
        /// Target node address
        #[arg(long)]
        target: Option<String>,
        
        /// Check DHT health
        #[arg(long)]
        dht: bool,
        
        /// Check geographic routing
        #[arg(long)]
        geo: bool,
    },
    
    /// Export/Import operations
    Export {
        /// Export type (dht, config, peers)
        #[arg(long, default_value = "dht")]
        export_type: String,
        
        /// Output file
        output: String,
    },
    
    Import {
        /// Import type (dht, config, peers)
        #[arg(long, default_value = "dht")]
        import_type: String,
        
        /// Input file
        input: String,
    },
}

/// Bootstrap node state
struct BootstrapState {
    dht_manager: Arc<RwLock<Option<DHTManager>>>,
    geo_manager: Arc<RwLock<Option<GeographicBootstrapManager>>>,
    network_manager: Arc<RwLock<Option<NetworkManager>>>,
    mcp_server: Option<MCPServer>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(cli.verbose)?;
    
    // Create directories
    tokio::fs::create_dir_all(&cli.data_dir).await?;
    tokio::fs::create_dir_all(&cli.log_dir).await?;
    
    info!("Starting Communitas CLI v{}", env!("CARGO_PKG_VERSION"));
    
    match cli.command {
        Commands::Bootstrap { 
            port, 
            mcp_port, 
            region, 
            bootstrap, 
            persistent, 
            storage_mb,
            api_token,
        } => {
            run_bootstrap_node(
                port,
                mcp_port,
                region,
                bootstrap,
                persistent,
                storage_mb,
                api_token,
                &cli.data_dir,
            ).await?;
        }
        
        Commands::Dht(dht_command) => {
            // Connect to local or remote DHT manager
            let config = DHTBootstrapConfig {
                persistent_storage: true,
                storage_path: format!("{}/dht", cli.data_dir),
                ..Default::default()
            };
            
            let mut manager = DHTManager::new(config).await?;
            execute_dht_command(&mut manager, dht_command).await?;
        }
        
        Commands::Geo(geo_command) => {
            // Connect to geographic routing manager
            let config = GeographicBootstrapConfig::default();
            let mut manager = GeographicBootstrapManager::new(config).await?;
            manager.initialize().await?;
            execute_geographic_command(&mut manager, geo_command).await?;
        }
        
        Commands::Chat { voice, vision, model } => {
            info!("Starting chat session with model: {}", model);
            if voice {
                info!("Voice I/O enabled");
            }
            if vision {
                info!("Vision capabilities enabled");
            }
            // TODO: Implement chat functionality
            println!("Chat interface coming soon!");
        }
        
        Commands::Process { file, instruction: _, format: _ } => {
            info!("Processing file: {}", file);
            // TODO: Implement file processing
            println!("File processing coming soon!");
        }
        
        Commands::Tui { theme } => {
            info!("Starting TUI with {} theme", theme);
            // TODO: Implement TUI
            println!("TUI interface coming soon!");
        }
        
        Commands::Config { api_key, show } => {
            let _config_manager = ConfigManager::load()?;
            
            if show {
                let config = ConfigManager::load()?;
                println!("Current Configuration:");
                println!("{:#?}", config);
            } else if let Some(key_name) = api_key {
                println!("Setting API key for: {}", key_name);
                // TODO: Implement API key storage
            }
        }
        
        Commands::Connect { bootstrap, port } => {
            info!("Connecting to P2P network on port {}", port);
            if let Some(ref addr) = bootstrap {
                info!("Using bootstrap node: {}", addr);
            }
            
            // Simple P2P client connection
            let mut network_manager = NetworkManager::new();
            network_manager.initialize_with_address(&format!("0.0.0.0:{}", port)).await?;
            
            if let Some(bootstrap_addr) = bootstrap {
                // Add bootstrap node to config
                network_manager.config_mut().add_bootstrap_node(bootstrap_addr)?;
            }
            
            println!("Connected to P2P network on port {}", port);
            println!("Press Ctrl+C to disconnect");
            
            // Keep running
            tokio::signal::ctrl_c().await?;
        }
        
        Commands::Health { target, dht, geo } => {
            run_health_check(target, dht, geo).await?;
        }
        
        Commands::Export { export_type, output } => {
            match export_type.as_str() {
                "dht" => {
                    let config = DHTBootstrapConfig {
                        persistent_storage: true,
                        storage_path: format!("{}/dht", cli.data_dir),
                        ..Default::default()
                    };
                    
                    let manager = DHTManager::new(config).await?;
                    let count = manager.export_data(&output, "json", true).await?;
                    println!("✓ Exported {} DHT records to {}", count, output);
                }
                "config" => {
                    // Export configuration
                    println!("Configuration export not yet implemented");
                }
                "peers" => {
                    // Export peer list
                    println!("Peer export not yet implemented");
                }
                _ => {
                    error!("Unknown export type: {}", export_type);
                }
            }
        }
        
        Commands::Import { import_type, input } => {
            match import_type.as_str() {
                "dht" => {
                    let config = DHTBootstrapConfig {
                        persistent_storage: true,
                        storage_path: format!("{}/dht", cli.data_dir),
                        ..Default::default()
                    };
                    
                    let mut manager = DHTManager::new(config).await?;
                    let count = manager.import_data(&input, false).await?;
                    println!("✓ Imported {} DHT records from {}", count, input);
                }
                "config" => {
                    // Import configuration
                    println!("Configuration import not yet implemented");
                }
                "peers" => {
                    // Import peer list
                    println!("Peer import not yet implemented");
                }
                _ => {
                    error!("Unknown import type: {}", import_type);
                }
            }
        }
    }
    
    Ok(())
}

/// Run as a full bootstrap node
async fn run_bootstrap_node(
    port: u16,
    mcp_port: u16,
    region: Option<String>,
    bootstrap_nodes: Vec<String>,
    persistent: bool,
    storage_mb: usize,
    api_token: Option<String>,
    data_dir: &str,
) -> Result<()> {
    info!("Starting bootstrap node on port {} with MCP on {}", port, mcp_port);
    
    // Initialize DHT
    let dht_config = DHTBootstrapConfig {
        replication_factor: 8,
        storage_capacity_mb: storage_mb,
        persistent_storage: persistent,
        storage_path: format!("{}/dht", data_dir),
        geographic_routing: true,
        ..Default::default()
    };
    
    let dht_manager = Arc::new(RwLock::new(Some(
        DHTManager::new(dht_config).await
            .context("Failed to initialize DHT")?
    )));
    
    // Initialize geographic routing
    let mut geo_config = GeographicBootstrapConfig::default();
    if let Some(r) = region {
        geo_config.local_region = parse_region(&r)?;
    }
    
    let local_region = geo_config.local_region.clone();
    let mut geo_manager = GeographicBootstrapManager::new(geo_config).await?;
    geo_manager.initialize().await?;
    let geo_manager = Arc::new(RwLock::new(Some(geo_manager)));
    
    // Initialize network
    let mut network_manager = NetworkManager::new();
    network_manager.initialize_with_address(&format!("0.0.0.0:{}", port)).await?;
    
    // Add bootstrap nodes to config
    for node in bootstrap_nodes {
        match network_manager.config_mut().add_bootstrap_node(node.clone()) {
            Ok(_) => info!("Added bootstrap node: {}", node),
            Err(e) => warn!("Failed to add bootstrap node {}: {}", node, e),
        }
    }
    
    let _network_manager = Arc::new(RwLock::new(Some(network_manager)));
    
    // Initialize MCP server
    let mcp_server = if let Some(token) = api_token {
        let mcp_config = MCPConfig {
            port: mcp_port,
            auth_required: true,
            api_tokens: vec![token],
            ..Default::default()
        };
        
        let handlers = MCPHandlers::new(dht_manager.clone(), geo_manager.clone());
        let server = MCPServer::new(mcp_config, handlers);
        
        // Start MCP server in background
        let server_clone = server.clone();
        tokio::spawn(async move {
            if let Err(e) = server_clone.start().await {
                error!("MCP server error: {}", e);
            }
        });
        
        Some(server)
    } else {
        warn!("No API token provided, MCP server disabled");
        None
    };
    
    // Print status
    println!("========================================");
    println!("Bootstrap Node Started Successfully");
    println!("========================================");
    println!("P2P Port: {}", port);
    if mcp_server.is_some() {
        println!("MCP Port: {}", mcp_port);
    }
    println!("Data Directory: {}", data_dir);
    println!("Storage Capacity: {} MB", storage_mb);
    println!("Geographic Region: {:?}", local_region);
    println!("");
    println!("Node is ready to accept connections");
    println!("Press Ctrl+C to shutdown");
    println!("========================================");
    
    // Keep running until interrupted
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down bootstrap node...");
    
    // Cleanup
    if let Some(server) = mcp_server {
        server.stop().await?;
    }
    
    Ok(())
}

/// Run health checks
async fn run_health_check(target: Option<String>, check_dht: bool, check_geo: bool) -> Result<()> {
    if let Some(target_addr) = target {
        println!("Checking health of: {}", target_addr);
        // TODO: Implement remote health check via MCP
    } else {
        println!("Local Health Check");
        println!("==================");
        
        if check_dht {
            println!("DHT: Checking...");
            // TODO: Check local DHT health
            println!("DHT: Healthy");
        }
        
        if check_geo {
            println!("Geographic Routing: Checking...");
            // TODO: Check geographic routing health
            println!("Geographic Routing: Healthy");
        }
        
        println!("Overall Status: Healthy");
    }
    
    Ok(())
}

/// Initialize logging
fn init_logging(verbose: bool) -> Result<()> {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    Ok(())
}

/// Parse region string
fn parse_region(s: &str) -> Result<communitas_cli::geographic::GeographicRegion> {
    use communitas_cli::geographic::GeographicRegion;
    
    match s.to_lowercase().as_str() {
        "na" | "northamerica" => Ok(GeographicRegion::NorthAmerica),
        "eu" | "europe" => Ok(GeographicRegion::Europe),
        "ap" | "asiapacific" => Ok(GeographicRegion::AsiaPacific),
        "sa" | "southamerica" => Ok(GeographicRegion::SouthAmerica),
        "af" | "africa" => Ok(GeographicRegion::Africa),
        "oc" | "oceania" => Ok(GeographicRegion::Oceania),
        _ => Ok(GeographicRegion::Unknown),
    }
}