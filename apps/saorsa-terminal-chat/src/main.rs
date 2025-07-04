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
use colored::*;
use saorsa_core::{P2PNode, NodeBuilder};
use std::io::{self, Write};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║               🐜 Saorsa P2P Chat (Real Network) 🐜                  ║".cyan());
    println!("{}", "║                                                                      ║".cyan());
    println!("{}", "║        Using QUIC, DHT, and Quantum-Resistant Crypto                ║".cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════╝".cyan());
    println!();
    
    // Main menu
    println!("Choose an option:");
    println!();
    println!("  {} Create a new chat room", "1)".bold());
    println!("  {} Join an existing chat room", "2)".bold());
    println!();
    
    print!("Enter 1 or 2: ");
    io::stdout().flush()?;
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    match choice.trim() {
        "1" => create_chat_room().await?,
        "2" => join_chat_room().await?,
        _ => {
            println!("{}", "Invalid choice".red());
            return Ok(());
        }
    }
    
    Ok(())
}

async fn create_chat_room() -> Result<()> {
    println!("\n{}", "🏠 Creating P2P Chat Room".green());
    println!("{}", "========================".green());
    
    // Build P2P node
    println!("\n⏳ Starting P2P network with QUIC transport...");
    let node = Arc::new(NodeBuilder::new()
        .with_default_dht()
        .with_mcp_server()
        .with_production_mode()
        .build()
        .await?);
    
    println!("✅ P2P node started!");
    
    // Get our addresses
    let addresses = node.listen_addrs().await;
    let peer_id = node.peer_id();
    
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                    💬 Chat Room Created! 💬                         ║".cyan());
    println!("{}", "║                                                                      ║".cyan());
    println!("{}  Your peer ID: {}       {}", 
        "║".cyan(), peer_id.to_string().green(), "║".cyan());
    println!("{}", "║                                                                      ║".cyan());
    println!("{}", "║  Share this peer ID with friends to let them join!                  ║".cyan());
    println!("{}", "║                                                                      ║".cyan());
    
    // Show network info
    if addresses.iter().any(|a| a.contains("ip6")) {
        println!("{}", "║  Transport: QUIC over IPv6 (native)                                 ║".cyan());
    } else {
        println!("{}", "║  Transport: QUIC with automatic tunneling                           ║".cyan());
    }
    
    println!("{}", "╚══════════════════════════════════════════════════════════════════════╝".cyan());
    
    // Handle incoming messages
    let mut events = node.subscribe_events();
    
    println!("\n{}", "Waiting for friends to connect...".dimmed());
    println!("{}", "Type messages to send, or /quit to exit\n".dimmed());
    
    // Spawn event handler
    let node_clone = node.clone();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            match event {
                saorsa_core::P2PEvent::Message { topic, source, data } => {
                    if topic == "/chat/1.0.0" {
                        let message = String::from_utf8_lossy(&data);
                        println!("\r{} {}: {}", "💬".green(), source, message);
                        print!("> ");
                        io::stdout().flush().unwrap();
                    }
                }
                saorsa_core::P2PEvent::PeerConnected(peer_id) => {
                    println!("\n{} {} connected!", "🔔".green(), peer_id.to_string().bold());
                    print!("> ");
                    io::stdout().flush().unwrap();
                }
                saorsa_core::P2PEvent::PeerDisconnected(peer_id) => {
                    println!("\n{} {} disconnected", "🔔".yellow(), peer_id);
                    print!("> ");
                    io::stdout().flush().unwrap();
                }
                _ => {}
            }
        }
    });
    
    // Run chat loop
    run_chat_loop(node).await?;
    
    Ok(())
}

async fn join_chat_room() -> Result<()> {
    println!("\n{}", "🔗 Joining P2P Chat Room".green());
    println!("{}", "=======================".green());
    
    print!("\nEnter your friend's multiaddress (e.g., /ip4/127.0.0.1/tcp/9000): ");
    io::stdout().flush()?;
    
    let mut address = String::new();
    io::stdin().read_line(&mut address)?;
    let address = address.trim();
    
    // Build P2P node
    println!("\n⏳ Starting P2P network...");
    let node = Arc::new(NodeBuilder::new()
        .with_default_dht()
        .with_mcp_server()
        .with_production_mode()
        .build()
        .await?);
    
    println!("✅ P2P node started");
    
    // Connect to peer
    println!("\n⏳ Connecting to {}...", address);
    match node.connect_peer(address).await {
        Ok(peer_id) => {
            println!("✅ Connected to peer: {}", peer_id);
            
            // Send a hello message
            node.send_message(&peer_id, "/chat/1.0.0", b"Hello from the chat!".to_vec()).await?;
        }
        Err(e) => {
            println!("{}", format!("❌ Failed to connect: {}", e).red());
            println!("   Make sure your friend's chat is running!");
            return Err(e.into());
        }
    }
    
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                   🎊 Connected to chat room! 🎊                     ║".cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════╝".cyan());
    
    // Handle incoming messages
    let mut events = node.subscribe_events();
    
    // Spawn event handler
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            match event {
                saorsa_core::P2PEvent::Message { topic, source, data } => {
                    if topic == "/chat/1.0.0" {
                        let message = String::from_utf8_lossy(&data);
                        println!("\r{} {}: {}", "💬".green(), source, message);
                        print!("> ");
                        io::stdout().flush().unwrap();
                    }
                }
                saorsa_core::P2PEvent::PeerDisconnected(peer_id) => {
                    println!("\n{} {} disconnected", "🔔".yellow(), peer_id);
                    print!("> ");
                    io::stdout().flush().unwrap();
                }
                _ => {}
            }
        }
    });
    
    println!("\n{}", "Type messages to send, or /quit to exit\n".dimmed());
    
    // Run chat loop
    run_chat_loop(node).await?;
    
    Ok(())
}

async fn run_chat_loop(node: Arc<P2PNode>) -> Result<()> {
    let stdin = io::stdin();
    let mut line = String::new();
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        
        line.clear();
        stdin.read_line(&mut line)?;
        
        let trimmed = line.trim();
        match trimmed {
            "/quit" => {
                println!("{}", "👋 Goodbye!".yellow());
                node.stop().await?;
                break;
            }
            "/help" => {
                println!("{}", "Commands:".bold());
                println!("  /help     - Show this help");
                println!("  /peers    - List connected peers");
                println!("  /info     - Show network information");
                println!("  /quit     - Exit the chat");
            }
            "/peers" => {
                let peers = node.connected_peers().await;
                println!("{}: {}", "Connected peers".bold(), peers.len());
                for peer in peers {
                    println!("  • {}", peer);
                }
            }
            "/info" => {
                let addresses = node.listen_addrs().await;
                let peer_id = node.peer_id();
                println!("{}", "Network Information:".bold());
                println!("  Peer ID: {}", peer_id);
                println!("  Listen addresses:");
                for addr in addresses {
                    println!("    • {}", addr);
                }
            }
            _ if !trimmed.is_empty() => {
                // Send message to all connected peers
                let peers = node.connected_peers().await;
                if peers.is_empty() {
                    println!("{}", "No peers connected".yellow());
                } else {
                    for peer in peers {
                        node.send_message(&peer, "/chat/1.0.0", trimmed.as_bytes().to_vec()).await?;
                    }
                    println!("{} {}", "[You]".cyan(), trimmed);
                }
            }
            _ => {} // Empty line, ignore
        }
    }
    
    Ok(())
}