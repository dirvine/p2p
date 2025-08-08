// Copyright 2024 Saorsa Labs Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

use anyhow::Result;
use clap::Parser;
use saorsa_core::Multiaddr;
use saorsa_core::bootstrap::BootstrapDiscovery;
use saorsa_core::network::{NodeConfig, P2PNode};
use std::str::FromStr;
use tokio::io::{self, AsyncBufReadExt};
use tracing::{error, info, warn};

/// A simple P2P chat application using three-word addresses
#[derive(Parser, Debug)]
#[command(name = "p2p-chat")]
#[command(about = "P2P chat with three-word address support")]
struct Args {
    /// The port to listen on
    #[arg(short, long, default_value = "0")]
    port: u16,

    /// A peer to bootstrap from (multiaddr format)
    #[arg(long)]
    bootstrap: Vec<String>,

    /// Bootstrap using three-word addresses (e.g., "global.fast.eagle")
    #[arg(short = 'w', long)]
    bootstrap_words: Vec<String>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.debug {
        "debug,quinn=info"
    } else {
        "info"
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("🐜 P2P Foundation Chat");
    info!("======================");

    // Create node configuration
    let mut config = NodeConfig::default();
    config.listen_addr = format!("127.0.0.1:{}", args.port).parse().unwrap();

    // Create and start the node
    let node = P2PNode::new(config).await?;

    // Handle bootstrap peers
    let discovery = BootstrapDiscovery::new();
    let mut bootstrap_addrs = Vec::new();

    // Resolve three-word addresses
    for word_addr in &args.bootstrap_words {
        match discovery.resolve_three_words(word_addr) {
            Ok(multiaddr) => {
                info!("✅ Resolved '{}' to {}", word_addr, multiaddr);
                bootstrap_addrs.push(multiaddr);
            }
            Err(e) => {
                warn!("❌ Failed to resolve '{}': {}", word_addr, e);
            }
        }
    }

    // Add traditional multiaddrs
    for addr_str in &args.bootstrap {
        match Multiaddr::from_str(addr_str) {
            Ok(addr) => bootstrap_addrs.push(addr),
            Err(e) => warn!("Invalid multiaddr '{}': {}", addr_str, e),
        }
    }

    // If no bootstrap peers specified, try auto-discovery
    if bootstrap_addrs.is_empty() && args.bootstrap.is_empty() && args.bootstrap_words.is_empty() {
        info!("🔍 Auto-discovering bootstrap nodes...");
        match discovery.discover_bootstraps().await {
            Ok(bootstraps) => {
                info!("✅ Found {} bootstrap nodes", bootstraps.len());
                bootstrap_addrs = bootstraps;
            }
            Err(e) => {
                warn!("⚠️  Bootstrap discovery failed: {}", e);
                info!("💡 You can specify bootstrap nodes with:");
                info!("    --bootstrap /ip6/::1/udp/9000/quic");
                info!("    --bootstrap-words global.fast.eagle");
            }
        }
    }

    // Connect to bootstrap peers
    for addr in bootstrap_addrs {
        match node.connect_peer(&addr.to_string()).await {
            Ok(peer_id) => info!("🔗 Connected to bootstrap: {}", peer_id),
            Err(e) => warn!("Failed to connect to {}: {}", addr, e),
        }
    }

    // Get our listening address
    let listen_addrs = node.listen_addrs().await;
    info!("📍 Listening on: {:?}", listen_addrs);

    // Try to determine our three-word address
    // In a real implementation, this would be derived from our peer ID
    info!("🎯 Your three-word address: ocean.swift.mountain");
    info!("");
    info!("💬 Type messages to send to all connected peers");
    info!("📝 Commands: /peers, /status, /quit");
    info!("");

    // Start reading from stdin
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin).lines();

    loop {
        match reader.next_line().await? {
            Some(line) => {
                match line.trim() {
                    "/quit" => {
                        info!("👋 Goodbye!");
                        break;
                    }
                    "/peers" => {
                        let peers = node.connected_peers().await;
                        info!("🔗 Connected peers: {}", peers.len());
                        for peer in peers {
                            info!("   • {}", peer);
                        }
                    }
                    "/status" => match node.mcp_stats().await {
                        Ok(stats) => {
                            info!("📊 Network Status:");
                            info!("   • Active sessions: {}", stats.active_sessions);
                            info!("   • Total requests: {}", stats.total_requests);
                            info!("   • Total responses: {}", stats.total_responses);
                        }
                        Err(e) => error!("Failed to get stats: {}", e),
                    },
                    msg if !msg.is_empty() => {
                        // In a real chat app, we'd publish to a topic or send to specific peers
                        // For now, just echo locally
                        info!("[You]: {}", msg);

                        // Here you would normally do something like:
                        // node.publish("chat", msg.as_bytes()).await?;
                    }
                    _ => {}
                }
            }
            None => break,
        }
    }

    Ok(())
}
