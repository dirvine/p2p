//! Tunneling protocol integration tests
//!
//! Comprehensive tests for IPv6/IPv4 tunneling protocols including:
//! - 6to4 tunneling for IPv6 over IPv4
//! - Teredo tunneling for NAT traversal
//! - 6in4 manual tunnels
//! - Protocol auto-selection and fallback
//! - Performance and reliability

use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

use p2p_foundation::{P2PNode, tunneling::*};
use crate::common::{TestNetwork, PerformanceTest};

// Integration test submodules - TBD
// mod sixto4;
// mod teredo;  
// mod manual;
// mod auto_selection;
// mod nat_traversal;

/// Test 6to4 tunneling basic functionality
#[tokio::test]
async fn test_6to4_tunneling_basic() -> Result<()> {
    // Create IPv4-only node
    let config1 = TestNodeConfig::builder()
        .port(9200)
        .enable_ipv6(false)
        .build();
    let mut ipv4_config = config1;
    ipv4_config.enable_tunneling = true;
    ipv4_config.preferred_tunnel_protocol = TunnelProtocol::SixToFour;
    
    // Create IPv6-enabled node
    let config2 = TestNodeConfig::builder()
        .port(9201)
        .enable_ipv6(true)
        .build();
    let mut ipv6_config = config2;
    ipv6_config.enable_tunneling = true;
    ipv6_config.preferred_tunnel_protocol = TunnelProtocol::SixToFour;
    
    let ipv4_node = P2PNode::new(ipv4_config).await?;
    let ipv6_node = P2PNode::new(ipv6_config).await?;
    
    // Get tunnel endpoints
    let ipv4_tunnel_addr = ipv4_node.get_tunnel_endpoint(TunnelProtocol::SixToFour).await?;
    let ipv6_tunnel_addr = ipv6_node.get_tunnel_endpoint(TunnelProtocol::SixToFour).await?;
    
    println!("IPv4 node 6to4 endpoint: {}", ipv4_tunnel_addr);
    println!("IPv6 node 6to4 endpoint: {}", ipv6_tunnel_addr);
    
    // Establish tunnel connection
    let tunnel_info = ipv4_node.connect_via_tunnel(
        ipv6_tunnel_addr,
        TunnelProtocol::SixToFour
    ).await?;
    
    assert_eq!(tunnel_info.protocol, TunnelProtocol::SixToFour);
    assert!(tunnel_info.is_active);
    
    // Test data transmission through tunnel
    let ipv6_peer_id = ipv6_node.peer_id();
    let test_data = b"6to4_tunnel_test".to_vec();
    
    let start = std::time::Instant::now();
    ipv4_node.send_message(&ipv6_peer_id, test_data.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(10),
        ipv6_node.wait_for_message()
    ).await??;
    
    let tunnel_latency = start.elapsed();
    println!("6to4 tunnel latency: {:?}", tunnel_latency);
    
    assert_eq!(received.data, test_data);
    assert_eq!(received.sender, ipv4_node.peer_id());
    
    // Verify tunnel statistics
    let tunnel_stats = ipv4_node.get_tunnel_stats(TunnelProtocol::SixToFour).await?;
    assert!(tunnel_stats.bytes_sent > 0);
    assert!(tunnel_stats.packets_sent > 0);
    assert_eq!(tunnel_stats.tunnel_errors, 0);
    
    // Cleanup
    ipv4_node.stop().await?;
    ipv6_node.stop().await?;
    
    Ok(())
}

/// Test Teredo tunneling for NAT traversal
#[tokio::test]
async fn test_teredo_tunneling() -> Result<()> {
    // Create nodes behind simulated NAT
    let config1 = TestNodeConfig::builder()
        .port(9210)
        .enable_ipv6(false)
        .build();
    let mut nat_config1 = config1;
    nat_config1.enable_tunneling = true;
    nat_config1.preferred_tunnel_protocol = TunnelProtocol::Teredo;
    nat_config1.behind_nat = true;
    nat_config1.teredo_server = Some("teredo.test.com".to_string());
    
    let config2 = TestNodeConfig::builder()
        .port(9211)
        .enable_ipv6(false)
        .build();
    let mut nat_config2 = config2;
    nat_config2.enable_tunneling = true;
    nat_config2.preferred_tunnel_protocol = TunnelProtocol::Teredo;
    nat_config2.behind_nat = true;
    nat_config2.teredo_server = Some("teredo.test.com".to_string());
    
    let node1 = P2PNode::new(nat_config1).await?;
    let node2 = P2PNode::new(nat_config2).await?;
    
    // Wait for Teredo initialization
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Get Teredo IPv6 addresses
    let node1_teredo_addr = node1.get_teredo_address().await?;
    let node2_teredo_addr = node2.get_teredo_address().await?;
    
    println!("Node1 Teredo address: {}", node1_teredo_addr);
    println!("Node2 Teredo address: {}", node2_teredo_addr);
    
    // Verify Teredo addresses are in correct prefix (2001::/32)
    assert!(node1_teredo_addr.to_string().starts_with("2001:"));
    assert!(node2_teredo_addr.to_string().starts_with("2001:"));
    
    // Establish Teredo tunnel connection
    let tunnel_info = node1.connect_via_tunnel(
        node2_teredo_addr.into(),
        TunnelProtocol::Teredo
    ).await?;
    
    assert_eq!(tunnel_info.protocol, TunnelProtocol::Teredo);
    assert!(tunnel_info.nat_traversed);
    
    // Test NAT traversal
    let node2_id = node2.peer_id();
    let test_data = b"teredo_nat_traversal_test".to_vec();
    
    node1.send_message(&node2_id, test_data.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(15),
        node2.wait_for_message()
    ).await??;
    
    assert_eq!(received.data, test_data);
    
    // Verify bidirectional communication through NAT
    let response_data = b"teredo_response".to_vec();
    node2.send_message(&node1.peer_id(), response_data.clone()).await?;
    
    let response = timeout(
        Duration::from_secs(10),
        node1.wait_for_message()
    ).await??;
    
    assert_eq!(response.data, response_data);
    
    // Check Teredo tunnel health
    let tunnel_health = node1.check_tunnel_health(TunnelProtocol::Teredo).await?;
    assert!(tunnel_health.is_operational);
    assert!(tunnel_health.nat_type != NATType::Unknown);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test 6in4 manual tunneling
#[tokio::test]
async fn test_6in4_manual_tunnel() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9220)
        .enable_ipv6(false)
        .build();
    let mut tunnel_config1 = config1;
    tunnel_config1.enable_tunneling = true;
    tunnel_config1.manual_tunnel_endpoint = Some("192.168.1.100".parse()?);
    
    let config2 = TestNodeConfig::builder()
        .port(9221)
        .enable_ipv6(true)
        .build();
    let mut tunnel_config2 = config2;
    tunnel_config2.enable_tunneling = true;
    tunnel_config2.manual_tunnel_endpoint = Some("192.168.1.101".parse()?);
    
    let node1 = P2PNode::new(tunnel_config1).await?;
    let node2 = P2PNode::new(tunnel_config2).await?;
    
    // Set up manual tunnel
    let tunnel_config = ManualTunnelConfig {
        local_endpoint: "192.168.1.100".parse()?,
        remote_endpoint: "192.168.1.101".parse()?,
        tunnel_interface: "sit0".to_string(),
        mtu: 1480,
    };
    
    let tunnel = node1.create_manual_tunnel(tunnel_config).await?;
    assert!(tunnel.is_active());
    
    // Test communication through manual tunnel
    let node2_addrs = node2.listen_addrs().await?;
    let ipv6_addr = node2_addrs.iter()
        .find(|addr| addr.to_string().contains("ip6"))
        .expect("Should have IPv6 address");
    
    node1.connect_via_tunnel(ipv6_addr.clone(), TunnelProtocol::SixInFour).await?;
    
    // Verify tunnel connection
    let tunnel_stats = node1.get_tunnel_stats(TunnelProtocol::SixInFour).await?;
    assert!(tunnel_stats.is_connected);
    
    // Test data transmission
    let node2_id = node2.peer_id();
    let test_data = TestDataGen::random_bytes(1024);
    
    node1.send_message(&node2_id, test_data.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(10),
        node2.wait_for_message()
    ).await??;
    
    assert_eq!(received.data, test_data);
    
    // Cleanup
    node1.destroy_manual_tunnel(tunnel.id()).await?;
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test tunnel protocol auto-selection
#[tokio::test]
async fn test_tunnel_auto_selection() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9230)
        .enable_ipv6(false)
        .build();
    let mut auto_config1 = config1;
    auto_config1.enable_tunneling = true;
    auto_config1.tunnel_selection = TunnelSelection::Auto;
    auto_config1.behind_nat = true;
    
    let config2 = TestNodeConfig::builder()
        .port(9231)
        .enable_ipv6(true)
        .build();
    let mut auto_config2 = config2;
    auto_config2.enable_tunneling = true;
    auto_config2.tunnel_selection = TunnelSelection::Auto;
    
    let node1 = P2PNode::new(auto_config1).await?;
    let node2 = P2PNode::new(auto_config2).await?;
    
    // Wait for auto-selection to complete
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Check what protocol was auto-selected
    let selected_protocol = node1.get_selected_tunnel_protocol().await?;
    println!("Auto-selected tunnel protocol: {:?}", selected_protocol);
    
    // Should select appropriate protocol based on network conditions
    match selected_protocol {
        TunnelProtocol::Teredo => {
            println!("Teredo selected for NAT traversal");
            assert!(node1.is_behind_nat().await?);
        },
        TunnelProtocol::SixToFour => {
            println!("6to4 selected for direct tunneling");
        },
        _ => {
            println!("Other protocol selected: {:?}", selected_protocol);
        }
    }
    
    // Test that auto-selected tunnel works
    let node2_addrs = node2.listen_addrs().await?;
    let connection_result = node1.connect(node2_addrs[0].clone()).await;
    
    // Connection should succeed using the auto-selected tunnel
    assert!(connection_result.is_ok());
    
    let node2_id = node2.peer_id();
    let test_data = b"auto_tunnel_test".to_vec();
    
    node1.send_message(&node2_id, test_data.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(10),
        node2.wait_for_message()
    ).await??;
    
    assert_eq!(received.data, test_data);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test tunnel failover and redundancy
#[tokio::test]
async fn test_tunnel_failover() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9240)
        .enable_ipv6(false)
        .build();
    let mut failover_config1 = config1;
    failover_config1.enable_tunneling = true;
    failover_config1.tunnel_redundancy = true;
    failover_config1.tunnel_protocols = vec![
        TunnelProtocol::SixToFour,
        TunnelProtocol::Teredo,
        TunnelProtocol::SixInFour,
    ];
    
    let config2 = TestNodeConfig::builder()
        .port(9241)
        .enable_ipv6(true)
        .build();
    let mut failover_config2 = config2;
    failover_config2.enable_tunneling = true;
    failover_config2.tunnel_redundancy = true;
    
    let node1 = P2PNode::new(failover_config1).await?;
    let node2 = P2PNode::new(failover_config2).await?;
    
    // Establish connection with multiple tunnel protocols
    let node2_addrs = node2.listen_addrs().await?;
    let connection_info = node1.connect_with_failover(node2_addrs[0].clone()).await?;
    
    println!("Primary tunnel: {:?}", connection_info.primary_tunnel);
    println!("Backup tunnels: {:?}", connection_info.backup_tunnels);
    
    assert!(connection_info.backup_tunnels.len() > 0);
    
    // Test primary tunnel
    let node2_id = node2.peer_id();
    let test_data = b"primary_tunnel_test".to_vec();
    
    node1.send_message(&node2_id, test_data.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(10),
        node2.wait_for_message()
    ).await??;
    
    assert_eq!(received.data, test_data);
    
    // Simulate primary tunnel failure
    node1.simulate_tunnel_failure(connection_info.primary_tunnel).await?;
    
    // Wait for failover
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Test that backup tunnel is now active
    let current_info = node1.get_connection_info(&node2_id).await?;
    assert_ne!(current_info.active_tunnel, connection_info.primary_tunnel);
    
    // Test communication through backup tunnel
    let failover_data = b"failover_tunnel_test".to_vec();
    node1.send_message(&node2_id, failover_data.clone()).await?;
    
    let failover_received = timeout(
        Duration::from_secs(10),
        node2.wait_for_message()
    ).await??;
    
    assert_eq!(failover_received.data, failover_data);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test tunnel performance and overhead
#[tokio::test]
async fn test_tunnel_performance() -> Result<()> {
    let mut perf = PerformanceTest::new();
    
    // Measure direct connection performance
    let direct_throughput = perf.measure_async("direct_connection", async {
        let network = TestNetwork::simple(2).await?;
        let test_data = TestDataGen::random_bytes(1024 * 1024); // 1MB
        
        let start = std::time::Instant::now();
        for _ in 0..10 {
            network.node(0)?.send_message(&network.node(1)?.peer_id(), test_data.clone()).await?;
            let _ = network.node(1)?.wait_for_message().await?;
        }
        let duration = start.elapsed();
        
        let throughput = (test_data.len() * 10 * 8) as f64 / 
                        (duration.as_secs_f64() * 1_000_000.0);
        
        network.stop().await?;
        Ok::<f64, anyhow::Error>(throughput)
    }).await?;
    
    // Measure 6to4 tunnel performance
    let tunnel_throughput = perf.measure_async("6to4_tunnel", async {
        let config1 = TestNodeConfig::builder().port(9250).enable_ipv6(false).build();
        let mut tunnel_config1 = config1;
        tunnel_config1.enable_tunneling = true;
        tunnel_config1.preferred_tunnel_protocol = TunnelProtocol::SixToFour;
        
        let config2 = TestNodeConfig::builder().port(9251).enable_ipv6(true).build();
        let mut tunnel_config2 = config2;
        tunnel_config2.enable_tunneling = true;
        
        let node1 = P2PNode::new(tunnel_config1).await?;
        let node2 = P2PNode::new(tunnel_config2).await?;
        
        // Establish tunnel
        let node2_addrs = node2.listen_addrs().await?;
        node1.connect_via_tunnel(node2_addrs[0].clone(), TunnelProtocol::SixToFour).await?;
        
        let test_data = TestDataGen::random_bytes(1024 * 1024); // 1MB
        let node2_id = node2.peer_id();
        
        let start = std::time::Instant::now();
        for _ in 0..10 {
            node1.send_message(&node2_id, test_data.clone()).await?;
            let _ = node2.wait_for_message().await?;
        }
        let duration = start.elapsed();
        
        let throughput = (test_data.len() * 10 * 8) as f64 / 
                        (duration.as_secs_f64() * 1_000_000.0);
        
        node1.stop().await?;
        node2.stop().await?;
        
        Ok::<f64, anyhow::Error>(throughput)
    }).await?;
    
    perf.print_results();
    
    println!("Direct connection throughput: {:.2} Mbps", direct_throughput);
    println!("6to4 tunnel throughput: {:.2} Mbps", tunnel_throughput);
    
    // Calculate tunnel overhead
    let overhead_percentage = ((direct_throughput - tunnel_throughput) / direct_throughput) * 100.0;
    println!("Tunnel overhead: {:.1}%", overhead_percentage);
    
    // Tunnel should have reasonable performance (< 50% overhead)
    assert!(
        overhead_percentage < 50.0,
        "Tunnel overhead too high: {:.1}%",
        overhead_percentage
    );
    
    // Both should have minimum acceptable throughput
    assert!(direct_throughput > 10.0, "Direct connection too slow");
    assert!(tunnel_throughput > 5.0, "Tunnel too slow");
    
    Ok(())
}

/// Test tunnel with packet loss and recovery
#[tokio::test]
async fn test_tunnel_packet_loss_recovery() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9260)
        .enable_ipv6(false)
        .build();
    let mut lossy_config1 = config1;
    lossy_config1.enable_tunneling = true;
    lossy_config1.preferred_tunnel_protocol = TunnelProtocol::SixToFour;
    lossy_config1.simulate_packet_loss = true;
    lossy_config1.packet_loss_rate = 0.05; // 5% packet loss
    
    let config2 = TestNodeConfig::builder()
        .port(9261)
        .enable_ipv6(true)
        .build();
    let mut lossy_config2 = config2;
    lossy_config2.enable_tunneling = true;
    
    let node1 = P2PNode::new(lossy_config1).await?;
    let node2 = P2PNode::new(lossy_config2).await?;
    
    // Establish tunnel with packet loss simulation
    let node2_addrs = node2.listen_addrs().await?;
    let tunnel_info = node1.connect_via_tunnel(
        node2_addrs[0].clone(),
        TunnelProtocol::SixToFour
    ).await?;
    
    let node2_id = node2.peer_id();
    let total_messages = 100;
    let mut successful_messages = 0;
    
    // Send many messages to test recovery
    for i in 0..total_messages {
        let test_data = format!("recovery_test_{}", i).into_bytes();
        
        match timeout(
            Duration::from_secs(5),
            node1.send_message(&node2_id, test_data.clone())
        ).await {
            Ok(Ok(_)) => {
                // Wait for acknowledgment or delivery confirmation
                if let Ok(Ok(_)) = timeout(
                    Duration::from_secs(2),
                    node2.wait_for_message()
                ).await {
                    successful_messages += 1;
                }
            },
            _ => {
                // Message failed - this is expected with packet loss
            }
        }
    }
    
    let success_rate = successful_messages as f64 / total_messages as f64;
    println!("Message success rate: {:.1}%", success_rate * 100.0);
    
    // Should have reasonable success rate despite packet loss
    assert!(
        success_rate > 0.90, // 90% success rate despite 5% packet loss (due to retransmission)
        "Success rate too low: {:.1}%",
        success_rate * 100.0
    );
    
    // Check tunnel recovery statistics
    let recovery_stats = node1.get_tunnel_recovery_stats(TunnelProtocol::SixToFour).await?;
    assert!(recovery_stats.retransmissions > 0);
    assert!(recovery_stats.recovered_packets > 0);
    
    println!("Tunnel recovery stats: {:?}", recovery_stats);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test tunnel with multiple concurrent connections
#[tokio::test]
async fn test_tunnel_concurrent_connections() -> Result<()> {
    let config_server = TestNodeConfig::builder()
        .port(9270)
        .enable_ipv6(true)
        .build();
    let mut server_config = config_server;
    server_config.enable_tunneling = true;
    server_config.max_tunnel_connections = 10;
    
    let server_node = P2PNode::new(server_config).await?;
    let server_addrs = server_node.listen_addrs().await?;
    let server_id = server_node.peer_id();
    
    // Create multiple client nodes
    let mut client_nodes = Vec::new();
    for i in 0..5 {
        let config = TestNodeConfig::builder()
            .port(9271 + i as u16)
            .enable_ipv6(false)
            .build();
        let mut client_config = config;
        client_config.enable_tunneling = true;
        client_config.preferred_tunnel_protocol = TunnelProtocol::SixToFour;
        
        let client = P2PNode::new(client_config).await?;
        
        // Connect to server via tunnel
        client.connect_via_tunnel(
            server_addrs[0].clone(),
            TunnelProtocol::SixToFour
        ).await?;
        
        client_nodes.push(client);
    }
    
    // Test concurrent communication
    let mut handles = Vec::new();
    for (i, client) in client_nodes.iter().enumerate() {
        let client = client.clone();
        let server_id = server_id.clone();
        
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let message = format!("client_{}_message_{}", i, j).into_bytes();
                if let Err(e) = client.send_message(&server_id, message).await {
                    println!("Client {} message {} failed: {}", i, j, e);
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            true
        });
        handles.push(handle);
    }
    
    // Wait for all clients to finish
    let mut successful_clients = 0;
    for handle in handles {
        if handle.await.unwrap_or(false) {
            successful_clients += 1;
        }
    }
    
    println!("Successful concurrent clients: {}/{}", successful_clients, client_nodes.len());
    assert!(
        successful_clients >= 4, // At least 80% success rate
        "Too many concurrent clients failed"
    );
    
    // Check server tunnel statistics
    let tunnel_stats = server_node.get_tunnel_stats(TunnelProtocol::SixToFour).await?;
    assert!(tunnel_stats.concurrent_connections >= 4);
    assert!(tunnel_stats.total_bytes_received > 0);
    
    // Cleanup
    server_node.stop().await?;
    for client in client_nodes {
        client.stop().await?;
    }
    
    Ok(())
}