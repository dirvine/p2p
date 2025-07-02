//! Comprehensive End-to-End Test Suite for Saorsa Core
//! 
//! This test suite provides full IPv6-only testing with multi-node support,
//! real-time reporting, and comprehensive feature coverage.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Instant;
use tokio::runtime::Runtime;

pub mod infrastructure;
pub mod scenarios;
pub mod stress;

use infrastructure::{
    distributed_runner::DistributedTestCoordinator,
    test_network::{DistributedTestConfig, DistributedTestNetwork},
    test_reporter::{TestReporter, TestReport},
};

use scenarios::{
    chat_tests, identity_tests, mcp_tests, project_tests, 
    threshold_tests, tunneling_tests,
};

#[derive(Parser)]
#[command(name = "saorsa-e2e-tests")]
#[command(about = "Saorsa Core End-to-End Test Suite")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Number of local nodes to run
    #[arg(short, long, default_value = "8")]
    local_nodes: usize,
    
    /// Remote node endpoints (comma-separated IPv6 addresses)
    #[arg(short, long)]
    remote_nodes: Option<String>,
    
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Output directory for test reports
    #[arg(short, long, default_value = "./test-reports")]
    output_dir: String,
    
    /// Test timeout in seconds
    #[arg(short = 't', long, default_value = "3600")]
    timeout: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all tests
    All,
    
    /// Run identity and organization tests
    Identity,
    
    /// Run chat system tests
    Chat,
    
    /// Run project management tests
    Projects,
    
    /// Run threshold signature tests
    Threshold,
    
    /// Run tunneling tests
    Tunneling,
    
    /// Run MCP integration tests
    Mcp,
    
    /// Run stress tests
    Stress {
        #[arg(long, default_value = "100")]
        max_nodes: usize,
        
        #[arg(long, default_value = "1000")]
        operations_per_node: usize,
    },
    
    /// Run distributed test coordinator
    Coordinator {
        #[arg(long)]
        bind_addr: String,
        
        #[arg(long, default_value = "9999")]
        port: u16,
    },
    
    /// Join distributed test as remote node
    Remote {
        #[arg(long)]
        coordinator: String,
        
        #[arg(long)]
        node_count: usize,
        
        #[arg(long)]
        name: Option<String>,
    },
}

/// Main entry point for the test suite
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_test_logging(cli.verbose);
    
    // Create test configuration
    let config = create_test_config(&cli)?;
    
    // Execute based on command
    match cli.command {
        Commands::All => run_all_tests(config).await?,
        Commands::Identity => run_identity_tests(config).await?,
        Commands::Chat => run_chat_tests(config).await?,
        Commands::Projects => run_project_tests(config).await?,
        Commands::Threshold => run_threshold_tests(config).await?,
        Commands::Tunneling => run_tunneling_tests(config).await?,
        Commands::Mcp => run_mcp_tests(config).await?,
        Commands::Stress { max_nodes, operations_per_node } => {
            run_stress_tests(config, max_nodes, operations_per_node).await?
        }
        Commands::Coordinator { bind_addr, port } => {
            run_test_coordinator(bind_addr, port).await?
        }
        Commands::Remote { coordinator, node_count, name } => {
            join_distributed_test(coordinator, node_count, name).await?
        }
    }
    
    Ok(())
}

/// Initialize test logging
fn init_test_logging(verbose: bool) {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    
    let filter = if verbose {
        EnvFilter::new("debug,hyper=info,h2=info")
    } else {
        EnvFilter::new("info,saorsa=debug")
    };
    
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();
}

/// Create test configuration from CLI arguments
fn create_test_config(cli: &Cli) -> Result<DistributedTestConfig> {
    let remote_endpoints = cli.remote_nodes
        .as_ref()
        .map(|nodes| {
            nodes.split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    
    Ok(DistributedTestConfig {
        local_node_count: cli.local_nodes,
        remote_endpoints,
        output_dir: PathBuf::from(&cli.output_dir),
        timeout_secs: cli.timeout,
        verbose: cli.verbose,
        ipv6_only: true, // Always IPv6-only
    })
}

/// Run all test suites
async fn run_all_tests(config: DistributedTestConfig) -> Result<()> {
    println!("🚀 Starting Saorsa Core End-to-End Test Suite");
    println!("==================================================");
    println!("Configuration:");
    println!("  Local nodes: {}", config.local_node_count);
    println!("  Remote nodes: {}", config.remote_endpoints.len());
    println!("  IPv6-only: {}", config.ipv6_only);
    println!("  Timeout: {}s", config.timeout_secs);
    println!();
    
    let start_time = Instant::now();
    let mut network = DistributedTestNetwork::new(config.clone()).await
        .context("Failed to create test network")?;
    
    let mut failed_tests = Vec::new();
    
    // Start network
    println!("\n📡 Starting distributed test network...");
    network.start_all_nodes().await
        .context("Failed to start nodes")?;
    
    // Verify connectivity
    println!("\n🔗 Verifying network connectivity...");
    let connectivity_report = network.verify_connectivity().await
        .context("Failed to verify connectivity")?;
    network.reporter.display_connectivity_report(&connectivity_report).await;
    
    // Display initial topology
    let topology = network.get_topology().await;
    network.reporter.display_topology(&topology).await;
    
    // Run each test category
    let test_suites = vec![
        ("Identity & Organizations", identity_tests::test_full_identity_system),
        ("Chat & Communication", chat_tests::test_full_chat_system),
        ("Project Management", project_tests::test_full_project_system),
        ("Threshold Signatures", threshold_tests::test_threshold_signatures),
        ("IPv6 Tunneling", tunneling_tests::test_ipv6_tunneling),
        ("MCP Integration", mcp_tests::test_mcp_integration),
    ];
    
    for (name, test_fn) in test_suites {
        println!("\n🧪 Running {} tests...", name);
        let suite_start = Instant::now();
        
        match test_fn(&mut network).await {
            Ok(_) => {
                println!("✅ {} tests passed in {:?}", name, suite_start.elapsed());
            }
            Err(e) => {
                println!("❌ {} tests failed: {}", name, e);
                failed_tests.push(name);
            }
        }
        
        // Show current network status
        network.reporter.show_network_summary(&network).await;
    }
    
    // Stop network
    println!("\n🛑 Stopping test network...");
    network.stop_all_nodes().await
        .context("Failed to stop nodes")?;
    
    // Generate final report
    let report = network.reporter.generate_final_report().await
        .context("Failed to generate report")?;
    save_test_report(&report, &config.output_dir).await
        .context("Failed to save report")?;
    
    // Display summary
    println!("\n📊 Test Summary");
    println!("==================================================");
    println!("Total test time: {:?}", start_time.elapsed());
    println!("Total nodes: {}", network.total_nodes());
    println!("Total tests run: {}", test_suites.len());
    println!("Failed tests: {}", failed_tests.len());
    
    if !failed_tests.is_empty() {
        println!("\nFailed test suites:");
        for test in &failed_tests {
            println!("  - {}", test);
        }
        return Err(anyhow::anyhow!("{} test suites failed", failed_tests.len()));
    }
    
    println!("\n🎉 All tests passed!");
    println!("\nTest report saved to: {}", config.output_dir.display());
    
    Ok(())
}

/// Run identity tests only
async fn run_identity_tests(config: DistributedTestConfig) -> Result<()> {
    let mut network = DistributedTestNetwork::new(config).await?;
    network.start_all_nodes().await?;
    
    identity_tests::test_full_identity_system(&mut network).await?;
    
    network.stop_all_nodes().await?;
    Ok(())
}

/// Run chat tests only
async fn run_chat_tests(config: DistributedTestConfig) -> Result<()> {
    let mut network = DistributedTestNetwork::new(config).await?;
    network.start_all_nodes().await?;
    
    chat_tests::test_full_chat_system(&mut network).await?;
    
    network.stop_all_nodes().await?;
    Ok(())
}

/// Run project tests only
async fn run_project_tests(config: DistributedTestConfig) -> Result<()> {
    let mut network = DistributedTestNetwork::new(config).await?;
    network.start_all_nodes().await?;
    
    project_tests::test_full_project_system(&mut network).await?;
    
    network.stop_all_nodes().await?;
    Ok(())
}

/// Run threshold tests only
async fn run_threshold_tests(config: DistributedTestConfig) -> Result<()> {
    let mut network = DistributedTestNetwork::new(config).await?;
    network.start_all_nodes().await?;
    
    threshold_tests::test_threshold_signatures(&mut network).await?;
    
    network.stop_all_nodes().await?;
    Ok(())
}

/// Run tunneling tests only
async fn run_tunneling_tests(config: DistributedTestConfig) -> Result<()> {
    let mut network = DistributedTestNetwork::new(config).await?;
    network.start_all_nodes().await?;
    
    tunneling_tests::test_ipv6_tunneling(&mut network).await?;
    
    network.stop_all_nodes().await?;
    Ok(())
}

/// Run MCP tests only
async fn run_mcp_tests(config: DistributedTestConfig) -> Result<()> {
    let mut network = DistributedTestNetwork::new(config).await?;
    network.start_all_nodes().await?;
    
    mcp_tests::test_mcp_integration(&mut network).await?;
    
    network.stop_all_nodes().await?;
    Ok(())
}

/// Run stress tests
async fn run_stress_tests(
    config: DistributedTestConfig, 
    max_nodes: usize,
    operations_per_node: usize
) -> Result<()> {
    stress::scale_tests::test_network_scale(config, max_nodes, operations_per_node).await
}

/// Run test coordinator for distributed testing
async fn run_test_coordinator(bind_addr: String, port: u16) -> Result<()> {
    let addr: SocketAddr = format!("[{}]:{}", bind_addr, port).parse()
        .context("Invalid bind address")?;
    
    let coordinator = DistributedTestCoordinator::new(addr).await
        .context("Failed to create coordinator")?;
    
    println!("🎮 Test Coordinator running on {}", addr);
    println!("Remote nodes can join with:");
    println!("  cargo test --test e2e_full_network -- remote --coordinator {} --node-count N", addr);
    
    coordinator.run().await
}

/// Join distributed test as remote node
async fn join_distributed_test(
    coordinator: String, 
    node_count: usize,
    name: Option<String>
) -> Result<()> {
    let coordinator_addr: SocketAddr = coordinator.parse()
        .context("Invalid coordinator address")?;
    
    println!("🔗 Joining distributed test at {}", coordinator_addr);
    println!("Starting {} local nodes", node_count);
    
    let mut config = DistributedTestConfig::default();
    config.local_node_count = node_count;
    
    let mut network = DistributedTestNetwork::new(config).await?;
    network.join_coordinator(coordinator_addr, name).await?;
    
    // Keep running until coordinator signals completion
    network.run_as_remote().await
}

/// Save test report to disk
async fn save_test_report(report: &TestReport, output_dir: &PathBuf) -> Result<()> {
    use tokio::fs;
    
    // Create output directory
    fs::create_dir_all(output_dir).await
        .context("Failed to create output directory")?;
    
    // Generate timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    
    // Save JSON report
    let json_path = output_dir.join(format!("test_report_{}.json", timestamp));
    let json_content = serde_json::to_string_pretty(report)
        .context("Failed to serialize report")?;
    fs::write(&json_path, json_content).await
        .context("Failed to write JSON report")?;
    
    // Save Markdown report
    let md_path = output_dir.join(format!("test_report_{}.md", timestamp));
    let md_content = report.to_markdown();
    fs::write(&md_path, md_content).await
        .context("Failed to write Markdown report")?;
    
    // Save HTML report
    let html_path = output_dir.join(format!("test_report_{}.html", timestamp));
    let html_content = report.to_html();
    fs::write(&html_path, html_content).await
        .context("Failed to write HTML report")?;
    
    println!("📄 Test reports saved:");
    println!("  JSON: {}", json_path.display());
    println!("  Markdown: {}", md_path.display());
    println!("  HTML: {}", html_path.display());
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(&[
            "saorsa-e2e-tests",
            "--local-nodes", "10",
            "--verbose",
            "all"
        ]).unwrap();
        
        assert_eq!(cli.local_nodes, 10);
        assert!(cli.verbose);
    }
    
    #[test]
    fn test_config_creation() {
        let cli = Cli {
            command: Commands::All,
            local_nodes: 5,
            remote_nodes: Some("[2001:db8::1]:9000,[2001:db8::2]:9000".to_string()),
            verbose: false,
            output_dir: "./reports".to_string(),
            timeout: 1800,
        };
        
        let config = create_test_config(&cli).unwrap();
        assert_eq!(config.local_node_count, 5);
        assert_eq!(config.remote_endpoints.len(), 2);
        assert!(config.ipv6_only);
    }
}