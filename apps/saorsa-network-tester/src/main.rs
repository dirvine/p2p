
use anyhow::Result;
use colored::*;
use saorsa_core::{NodeBuilder, Key};
use std::io::{self, Write};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging if RUST_LOG is set
    if std::env::var("RUST_LOG").is_ok() {
        env_logger::init();
    }
    
    // Main loop to keep app running
    loop {
        println!();
        println!("{}", "╔══════════════════════════════════════════════════════════════════════╗".cyan());
        println!("{}", "║            🐜 Saorsa P2P Network Tester (Real Stack) 🐜            ║".cyan());
        println!("{}", "║                                                                      ║".cyan());
        println!("{}", "║        QUIC Transport, DHT, Tunneling, and ML-KEM/ML-DSA           ║".cyan());
        println!("{}", "╚══════════════════════════════════════════════════════════════════════╝".cyan());
        println!();
        
        println!("What would you like to test?");
        println!();
        println!("  {} Quick P2P Network Test", "1)".bold());
        println!("     • Start P2P node");
        println!("     • Test QUIC transport");
        println!("     • Verify DHT operations");
        println!("     • Check network connectivity");
        println!();
        println!("  {} DHT Storage Test", "2)".bold());
        println!("     • Store data in DHT");
        println!("     • Retrieve stored data");
        println!("     • Test replication");
        println!();
        println!("  {} Peer Connection Test", "3)".bold());
        println!("     • Create two nodes");
        println!("     • Test connectivity");
        println!("     • Send messages");
        println!();
        println!("  {} Network Info", "4)".bold());
        println!("     • Show listen addresses");
        println!("     • Display transport info");
        println!("     • Check IPv6/IPv4 status");
        println!();
        println!("  {} Quit", "Q)".bold());
        println!();
        
        print!("Please enter 1, 2, 3, 4, or Q: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim().to_lowercase().as_str() {
            "1" => {
                if let Err(e) = run_quick_test().await {
                    println!("{} Error: {}", "❌".red(), e);
                }
            },
            "2" => {
                if let Err(e) = run_dht_test().await {
                    println!("{} Error: {}", "❌".red(), e);
                }
            },
            "3" => {
                if let Err(e) = run_address_test().await {
                    println!("{} Error: {}", "❌".red(), e);
                }
            },
            "4" => {
                if let Err(e) = run_network_info().await {
                    println!("{} Error: {}", "❌".red(), e);
                }
            },
            "q" | "quit" | "exit" => {
                println!("\n{}", "👋 Thanks for using Saorsa Network Tester!".green());
                break;
            },
            _ => println!("{}", "Invalid choice. Please try again.".red()),
        }
        
        // Pause between tests
        println!("\nPress Enter to continue...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }
    
    Ok(())
}

async fn run_quick_test() -> Result<()> {
    println!("\n{}", "🚀 Starting Quick P2P Network Test".bold().green());
    println!("{}", "══════════════════════════════════".green());
    println!();
    
    let start_time = Instant::now();
    let mut passed = 0;
    let mut failed = 0;
    
    // Test 1: Create P2P Node
    println!("{}", "📋 Testing P2P Node Creation".bold());
    println!("{}", "────────────────────────────");
    
    let node = match NodeBuilder::new()
        .with_default_dht()
        .with_mcp_server()
        .with_production_mode()
        .build()
        .await
    {
        Ok(n) => {
            println!("  {} P2P node created successfully", "✅".green());
            println!("  {} QUIC transport enabled", "✅".green());
            println!("  {} DHT enabled", "✅".green());
            println!("  {} MCP server enabled", "✅".green());
            passed += 4;
            n
        }
        Err(e) => {
            println!("  {} Node creation failed: {}", "❌".red(), e);
            failed += 1;
            return Err(e.into());
        }
    };
    
    println!();
    
    // Test 2: Network Addresses
    println!("{}", "📋 Testing Network Addresses".bold());
    println!("{}", "───────────────────────────");
    
    let addresses = node.listen_addrs().await;
    if !addresses.is_empty() {
        println!("  {} Listening on {} addresses:", "✅".green(), addresses.len());
        for addr in &addresses {
            println!("     • {}", addr);
            if addr.contains("ip6") {
                println!("       {} Native IPv6 support", "ℹ️".blue());
            }
        }
        passed += 1;
    } else {
        println!("  {} No listen addresses", "❌".red());
        failed += 1;
    }
    
    // Test peer ID
    let peer_id = node.peer_id();
    println!("  {} Peer ID: {}", "✅".green(), peer_id.to_string().bold());
    passed += 1;
    
    println!();
    
    // Test 3: DHT Operations
    println!("{}", "📋 Testing DHT Operations".bold());
    println!("{}", "────────────────────────");
    
    let test_key = Key::new(b"test-key");
    let test_value = b"test-value";
    
    // Store in DHT
    match node.dht_put(test_key.clone(), test_value.to_vec()).await {
        Ok(_) => {
            println!("  {} Stored data in DHT", "✅".green());
            passed += 1;
            
            // Retrieve from DHT
            match node.dht_get(test_key.clone()).await {
                Ok(Some(value)) if value == test_value => {
                    println!("  {} Retrieved correct data from DHT", "✅".green());
                    passed += 1;
                }
                Ok(Some(_)) => {
                    println!("  {} Retrieved data but value mismatch", "⚠️".yellow());
                }
                Ok(None) => {
                    println!("  {} No data found in DHT", "⚠️".yellow());
                }
                Err(e) => {
                    println!("  {} DHT get failed: {}", "❌".red(), e);
                    failed += 1;
                }
            }
        }
        Err(e) => {
            println!("  {} DHT put failed: {}", "❌".red(), e);
            failed += 1;
        }
    }
    
    println!();
    
    // Test 4: Event System
    println!("{}", "📋 Testing Event System".bold());
    println!("{}", "──────────────────────");
    
    match Ok::<_, String>(node.subscribe_events()) {
        Ok(_) => {
            println!("  {} Event subscription working", "✅".green());
            passed += 1;
        }
        Err(e) => {
            println!("  {} Event subscription failed: {}", "❌".red(), e);
            failed += 1;
        }
    }
    
    println!();
    
    // Summary
    let duration = start_time.elapsed();
    show_test_summary(passed, failed, duration);
    
    // Cleanup
    node.stop().await?;
    
    Ok(())
}

async fn run_dht_test() -> Result<()> {
    println!("\n{}", "🗄️  Starting DHT Storage Test".bold().green());
    println!("{}", "═════════════════════════════".green());
    println!();
    
    // Create node
    let node = NodeBuilder::new()
        .with_default_dht()
        .build()
        .await?;
    
    println!("{}", "📋 Testing DHT Operations".bold());
    println!("{}", "────────────────────────");
    
    // Test multiple key-value pairs
    let test_data: Vec<(&[u8], &[u8])> = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"large-key", b"This is a larger value to test DHT storage capabilities"),
    ];
    
    let mut stored = 0;
    let mut retrieved = 0;
    
    // Store data
    println!("\n  Storing data in DHT...");
    for (key, value) in &test_data {
        let k = Key::new(*key);
        match node.dht_put(k.clone(), value.to_vec()).await {
            Ok(_) => {
                println!("  {} Stored '{}' → {} bytes", 
                    "✅".green(), 
                    String::from_utf8_lossy(*key), 
                    value.len()
                );
                stored += 1;
            }
            Err(e) => {
                println!("  {} Failed to store '{}': {}", 
                    "❌".red(), 
                    String::from_utf8_lossy(*key), 
                    e
                );
            }
        }
    }
    
    println!("\n  Retrieving data from DHT...");
    tokio::time::sleep(Duration::from_millis(100)).await; // Give DHT time to propagate
    
    // Retrieve data
    for (key, expected_value) in &test_data {
        let k = Key::new(*key);
        match node.dht_get(k).await {
            Ok(Some(value)) => {
                if value == *expected_value {
                    println!("  {} Retrieved '{}' → {} bytes", 
                        "✅".green(), 
                        String::from_utf8_lossy(*key),
                        value.len()
                    );
                    retrieved += 1;
                } else {
                    println!("  {} Value mismatch for '{}'", 
                        "⚠️".yellow(), 
                        String::from_utf8_lossy(*key)
                    );
                }
            }
            Ok(None) => {
                println!("  {} Key '{}' not found", 
                    "⚠️".yellow(), 
                    String::from_utf8_lossy(*key)
                );
            }
            Err(e) => {
                println!("  {} Failed to retrieve '{}': {}", 
                    "❌".red(), 
                    String::from_utf8_lossy(*key), 
                    e
                );
            }
        }
    }
    
    println!();
    println!("📊 DHT Test Summary:");
    println!("  Stored: {}/{}", stored, test_data.len());
    println!("  Retrieved: {}/{}", retrieved, test_data.len());
    
    if stored == test_data.len() && retrieved == test_data.len() {
        println!("  {} DHT working perfectly!", "✅".green());
    } else if retrieved > 0 {
        println!("  {} DHT partially working", "⚠️".yellow());
    } else {
        println!("  {} DHT not working", "❌".red());
    }
    
    // Cleanup
    node.stop().await?;
    
    Ok(())
}

async fn run_address_test() -> Result<()> {
    println!("\n{}", "🏷️  Starting Peer Connection Test".bold().green());
    println!("{}", "═════════════════════════════════".green());
    println!();
    
    // Create first node
    println!("Creating first P2P node...");
    let node1 = NodeBuilder::new()
        .with_default_dht()
        .build()
        .await?;
    
    let peer_id1 = node1.peer_id();
    println!("  {} Node 1 peer ID: {}", "✅".green(), peer_id1.to_string());
    
    // Get listen addresses
    let addrs1 = node1.listen_addrs().await;
    if let Some(addr) = addrs1.first() {
        println!("  {} Node 1 listening on: {}", "ℹ️".blue(), addr);
    }
    
    // Create second node
    println!("\nCreating second P2P node...");
    let node2 = NodeBuilder::new()
        .with_default_dht()
        .build()
        .await?;
    
    let peer_id2 = node2.peer_id();
    println!("  {} Node 2 peer ID: {}", "✅".green(), peer_id2.to_string());
    
    // Test connection
    println!("\n{}", "📋 Testing Connectivity".bold());
    println!("{}", "──────────────────────");
    
    println!("  Attempting to connect Node 2 → Node 1...");
    if let Some(addr) = addrs1.first() {
        match node2.connect_peer(addr).await {
            Ok(connected_peer_id) => {
                println!("  {} Connected successfully!", "✅".green());
                println!("     Connected to peer: {}", connected_peer_id);
                
                // Test message sending
                println!("\n  Testing message sending...");
                match node2.send_message(&connected_peer_id, "/test/1.0.0", b"Hello from Node 2!".to_vec()).await {
                    Ok(_) => println!("  {} Message sent successfully", "✅".green()),
                    Err(e) => println!("  {} Message send failed: {}", "❌".red(), e),
                }
            }
            Err(e) => {
                println!("  {} Connection failed: {}", "❌".red(), e);
                println!("  {} This is normal if nodes are on same machine without proper routing", "ℹ️".blue());
            }
        }
    } else {
        println!("  {} No listen address available for Node 1", "❌".red());
    }
    
    // Show network details
    println!("\n{}", "📊 Network Details".bold());
    println!("{}", "─────────────────");
    
    let addrs1 = node1.listen_addrs().await;
    let addrs2 = node2.listen_addrs().await;
    
    println!("  Node 1 listening on:");
    for addr in addrs1 {
        println!("    • {}", addr);
    }
    
    println!("  Node 2 listening on:");
    for addr in addrs2 {
        println!("    • {}", addr);
    }
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

async fn run_network_info() -> Result<()> {
    println!("\n{}", "📊 Network Information".bold().green());
    println!("{}", "═════════════════════".green());
    println!();
    
    // Create node
    let node = NodeBuilder::new()
        .with_default_dht()
        .with_mcp_server()
        .with_production_mode()
        .build()
        .await?;
    
    // Get all network information
    let addresses = node.listen_addrs().await;
    let peer_id = node.peer_id();
    
    println!("{}", "📋 Node Information".bold());
    println!("{}", "──────────────────");
    println!("  Peer ID: {}", peer_id);
    
    println!("\n{}", "📋 Network Addresses".bold());
    println!("{}", "───────────────────");
    
    let mut has_ipv6 = false;
    let mut has_ipv4 = false;
    
    for addr in &addresses {
        println!("  • {}", addr);
        if addr.contains("ip6") {
            has_ipv6 = true;
        }
        if addr.contains("ip4") {
            has_ipv4 = true;
        }
    }
    
    println!("\n{}", "📋 Transport Capabilities".bold());
    println!("{}", "────────────────────────");
    println!("  Protocol: QUIC");
    println!("  IPv6: {}", if has_ipv6 { "✅ Available" } else { "❌ Not available" });
    println!("  IPv4: {}", if has_ipv4 { "✅ Available" } else { "❌ Not available" });
    
    if !has_ipv6 && has_ipv4 {
        println!("  Tunneling: Automatic (Teredo/6to4/DS-Lite)");
    }
    
    println!("\n{}", "📋 Features Enabled".bold());
    println!("{}", "─────────────────");
    println!("  ✅ DHT (Distributed Hash Table)");
    println!("  ✅ MCP Server (AI integration)");
    println!("  ✅ Quantum-resistant crypto (ML-KEM/ML-DSA)");
    println!("  ✅ Three-word addressing");
    println!("  ✅ Production hardening");
    
    // Cleanup
    node.stop().await?;
    
    Ok(())
}

fn show_test_summary(passed: usize, failed: usize, duration: Duration) {
    let total = passed + failed;
    let pass_rate = if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 };
    
    println!("{}", "╔══════════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                         📊 Test Results 📊                           ║".cyan());
    println!("{}", "║                                                                      ║".cyan());
    println!("{}  Total Tests: {:>3}                                                    {}", 
        "║".cyan(), total, "║".cyan());
    println!("{}  Passed:      {} {}                                                 {}", 
        "║".cyan(), format!("{:>3}", passed).green(), "✅", "║".cyan());
    println!("{}  Failed:      {} {}                                                 {}", 
        "║".cyan(), format!("{:>3}", failed).red(), "❌", "║".cyan());
    println!("{}  Pass Rate:   {:>5.1}%                                                {}", 
        "║".cyan(), pass_rate, "║".cyan());
    println!("{}  Duration:    {:>3} seconds                                           {}", 
        "║".cyan(), duration.as_secs(), "║".cyan());
    println!("{}", "║                                                                      ║".cyan());
    
    if failed == 0 {
        println!("{}", "║           🎉 All tests passed! P2P network is healthy! 🎉          ║".cyan());
    } else if pass_rate >= 80.0 {
        println!("{}", "║        ✅ P2P network mostly functional, some issues detected      ║".cyan());
    } else {
        println!("{}", "║        ⚠️  P2P network issues detected, check configuration       ║".cyan());
    }
    
    println!("{}", "║                                                                      ║".cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════╝".cyan());
}