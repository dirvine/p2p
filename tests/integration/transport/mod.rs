//! Transport layer integration tests
//!
//! Comprehensive tests for transport protocols including:
//! - QUIC transport with Quinn
//! - TCP fallback transport
//! - Transport switching and adaptation
//! - Performance and reliability
//! - Security and encryption

use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

use p2p_foundation::{P2PNode, transport::*};
use crate::common::{TestNetwork, PerformanceTest};

// Integration test submodules - TBD
// mod quic;
// mod tcp;
// mod switching;
// mod security;
// mod performance;

/// Test QUIC transport basic functionality
#[tokio::test]
async fn test_quic_transport_basic() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9100)
        .build();
    let mut node_config1 = config1;
    node_config1.preferred_transport = TransportType::QUIC;
    
    let config2 = TestNodeConfig::builder()
        .port(9101)
        .build();
    let mut node_config2 = config2;
    node_config2.preferred_transport = TransportType::QUIC;
    
    let node1 = P2PNode::new(node_config1).await?;
    let node2 = P2PNode::new(node_config2).await?;
    
    // Get node1's QUIC address
    let node1_addrs = node1.listen_addrs().await?;
    let quic_addr = node1_addrs.iter()
        .find(|addr| addr.to_string().contains("quic"))
        .expect("Node should have QUIC address");
    
    // Connect using QUIC
    let start = std::time::Instant::now();
    node2.connect_peer(&quic_addr.to_string()).await?;
    let connection_time = start.elapsed();
    
    println!("QUIC connection established in {:?}", connection_time);
    
    // Verify connection uses QUIC
    let node1_id = node1.peer_id();
    let connection_info = node2.get_connection_info(&node1_id).await?;
    assert_eq!(connection_info.transport_type, TransportType::QUIC);
    
    // Test data transmission over QUIC
    let test_data = TestDataGen::random_bytes(1024);
    let start = std::time::Instant::now();
    node2.send_message(&node1_id, test_data.clone()).await?;
    
    // Verify message received
    let received = timeout(
        Duration::from_secs(5),
        node1.wait_for_message()
    ).await??;
    
    let transmission_time = start.elapsed();
    println!("QUIC message transmission took {:?}", transmission_time);
    
    assert_eq!(received.data, test_data);
    assert_eq!(received.sender, node2.peer_id());
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test TCP transport basic functionality
#[tokio::test]
async fn test_tcp_transport_basic() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9110)
        .build();
    let mut node_config1 = config1;
    node_config1.preferred_transport = TransportType::TCP;
    
    let config2 = TestNodeConfig::builder()
        .port(9111)
        .build();
    let mut node_config2 = config2;
    node_config2.preferred_transport = TransportType::TCP;
    
    let node1 = P2PNode::new(node_config1).await?;
    let node2 = P2PNode::new(node_config2).await?;
    
    // Get node1's TCP address
    let node1_addrs = node1.listen_addrs().await?;
    let tcp_addr = node1_addrs.iter()
        .find(|addr| addr.to_string().contains("tcp") && !addr.to_string().contains("quic"))
        .expect("Node should have TCP address");
    
    // Connect using TCP
    node2.connect(tcp_addr.clone()).await?;
    
    // Verify connection uses TCP
    let node1_id = node1.peer_id();
    let connection_info = node2.get_connection_info(&node1_id).await?;
    assert_eq!(connection_info.transport_type, TransportType::TCP);
    
    // Test data transmission over TCP
    let test_data = TestDataGen::random_bytes(2048);
    node2.send_message(&node1_id, test_data.clone()).await?;
    
    // Verify message received
    let received = timeout(
        Duration::from_secs(5),
        node1.wait_for_message()
    ).await??;
    
    assert_eq!(received.data, test_data);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test transport switching between QUIC and TCP
#[tokio::test]
async fn test_transport_switching() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9120)
        .build();
    let mut node_config1 = config1;
    node_config1.enable_transport_switching = true;
    node_config1.preferred_transport = TransportType::QUIC;
    
    let config2 = TestNodeConfig::builder()
        .port(9121)
        .build();
    let mut node_config2 = config2;
    node_config2.enable_transport_switching = true;
    node_config2.preferred_transport = TransportType::QUIC;
    
    let node1 = P2PNode::new(node_config1).await?;
    let node2 = P2PNode::new(node_config2).await?;
    
    let node1_addrs = node1.listen_addrs().await?;
    let node1_id = node1.peer_id();
    
    // Initial connection should use QUIC (preferred)
    node2.connect_with_transport(
        node1_addrs[0].clone(),
        TransportType::QUIC
    ).await?;
    
    let initial_info = node2.get_connection_info(&node1_id).await?;
    assert_eq!(initial_info.transport_type, TransportType::QUIC);
    
    // Simulate QUIC failure and switch to TCP
    node2.switch_transport(&node1_id, TransportType::TCP).await?;
    
    // Wait for transport switch
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let switched_info = node2.get_connection_info(&node1_id).await?;
    assert_eq!(switched_info.transport_type, TransportType::TCP);
    
    // Verify connection still works after transport switch
    let test_data = b"post_switch_test".to_vec();
    node2.send_message(&node1_id, test_data.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(5),
        node1.wait_for_message()
    ).await??;
    
    assert_eq!(received.data, test_data);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test transport auto-selection based on network conditions
#[tokio::test]
async fn test_transport_auto_selection() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9130)
        .build();
    let mut node_config1 = config1;
    node_config1.transport_selection = TransportSelection::Auto;
    
    let config2 = TestNodeConfig::builder()
        .port(9131)
        .build();
    let mut node_config2 = config2;
    node_config2.transport_selection = TransportSelection::Auto;
    
    let node1 = P2PNode::new(node_config1).await?;
    let node2 = P2PNode::new(node_config2).await?;
    
    let node1_addrs = node1.listen_addrs().await?;
    let node1_id = node1.peer_id();
    
    // Connect with auto-selection
    node2.connect(node1_addrs[0].clone()).await?;
    
    // Verify a transport was selected
    let connection_info = node2.get_connection_info(&node1_id).await?;
    assert!(
        connection_info.transport_type == TransportType::QUIC ||
        connection_info.transport_type == TransportType::TCP,
        "Auto-selection should choose QUIC or TCP"
    );
    
    println!("Auto-selected transport: {:?}", connection_info.transport_type);
    
    // Test connection quality measurement
    let quality = node2.measure_connection_quality(&node1_id).await?;
    assert!(quality.latency.as_millis() < 1000);
    assert!(quality.throughput_mbps > 0.0);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test QUIC 0-RTT connections
#[tokio::test]
async fn test_quic_0rtt() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9140)
        .build();
    let mut node_config1 = config1;
    node_config1.enable_quic_0rtt = true;
    
    let config2 = TestNodeConfig::builder()
        .port(9141)
        .build();
    let mut node_config2 = config2;
    node_config2.enable_quic_0rtt = true;
    
    let node1 = P2PNode::new(node_config1).await?;
    let node2 = P2PNode::new(node_config2).await?;
    
    let node1_addrs = node1.listen_addrs().await?;
    let quic_addr = node1_addrs.iter()
        .find(|addr| addr.to_string().contains("quic"))
        .unwrap();
    
    // First connection to establish session
    node2.connect_peer(&quic_addr.to_string()).await?;
    let node1_id = node1.peer_id();
    
    // Send some data to establish session state
    node2.send_message(&node1_id, b"session_establishment".to_vec()).await?;
    let _ = node1.wait_for_message().await?;
    
    // Disconnect
    node2.disconnect(&node1_id).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Reconnect - should use 0-RTT
    let start = std::time::Instant::now();
    node2.connect_with_0rtt(quic_addr.clone()).await?;
    let reconnect_time = start.elapsed();
    
    println!("0-RTT reconnection took {:?}", reconnect_time);
    
    // 0-RTT should be faster than initial connection
    assert!(
        reconnect_time < Duration::from_millis(100),
        "0-RTT connection should be very fast"
    );
    
    // Verify 0-RTT was actually used
    let connection_info = node2.get_connection_info(&node1_id).await?;
    assert!(connection_info.used_0rtt);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test transport with IPv6
#[tokio::test]
async fn test_transport_ipv6() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9150)
        .enable_ipv6(true)
        .build();
    
    let config2 = TestNodeConfig::builder()
        .port(9151)
        .enable_ipv6(true)
        .build();
    
    let node1 = P2PNode::new(config1).await?;
    let node2 = P2PNode::new(config2).await?;
    
    // Get IPv6 addresses
    let node1_addrs = node1.listen_addrs().await?;
    let ipv6_addr = node1_addrs.iter()
        .find(|addr| addr.to_string().contains("ip6"))
        .expect("Node should have IPv6 address");
    
    println!("Connecting to IPv6 address: {}", ipv6_addr);
    
    // Connect over IPv6
    node2.connect(ipv6_addr.clone()).await?;
    
    let node1_id = node1.peer_id();
    let connection_info = node2.get_connection_info(&node1_id).await?;
    
    // Verify IPv6 is being used
    assert!(connection_info.local_addr.to_string().contains("ip6"));
    assert!(connection_info.remote_addr.to_string().contains("ip6"));
    
    // Test data transmission over IPv6
    let test_data = TestDataGen::random_bytes(1024);
    node2.send_message(&node1_id, test_data.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(5),
        node1.wait_for_message()
    ).await??;
    
    assert_eq!(received.data, test_data);
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}

/// Test large message transmission
#[tokio::test]
async fn test_large_message_transmission() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    let large_message = TestDataGen::random_bytes(10 * 1024 * 1024); // 10MB
    let node1_id = network.node(1)?.peer_id();
    
    let start = std::time::Instant::now();
    network.node(0)?.send_message(&node1_id, large_message.clone()).await?;
    
    let received = timeout(
        Duration::from_secs(30),
        network.node(1)?.wait_for_message()
    ).await??;
    
    let transmission_time = start.elapsed();
    println!("Large message (10MB) transmission took {:?}", transmission_time);
    
    assert_eq!(received.data, large_message);
    
    // Calculate throughput
    let throughput_mbps = (large_message.len() as f64 * 8.0) / 
                         (transmission_time.as_secs_f64() * 1_000_000.0);
    println!("Throughput: {:.2} Mbps", throughput_mbps);
    
    assert!(throughput_mbps > 1.0, "Throughput should be at least 1 Mbps");
    
    network.stop().await?;
    Ok(())
}

/// Test transport encryption and security
#[tokio::test]
async fn test_transport_security() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9160)
        .build();
    let mut node_config1 = config1;
    node_config1.enforce_encryption = true;
    
    let config2 = TestNodeConfig::builder()
        .port(9161)
        .build();
    let mut node_config2 = config2;
    node_config2.enforce_encryption = true;
    
    let node1 = P2PNode::new(node_config1).await?;
    let node2 = P2PNode::new(node_config2).await?;
    
    // Connect with enforced encryption
    let node1_addrs = node1.listen_addrs().await?;
    node2.connect(node1_addrs[0].clone()).await?;
    
    let node1_id = node1.peer_id();
    let connection_info = node2.get_connection_info(&node1_id).await?;
    
    // Verify encryption is active
    assert!(connection_info.is_encrypted);
    assert!(!connection_info.cipher_suite.is_empty());
    
    println!("Connection encrypted with: {}", connection_info.cipher_suite);
    
    // Test that unencrypted connections are rejected
    let insecure_config = TestNodeConfig::builder()
        .port(9162)
        .build();
    let mut insecure_node_config = insecure_config;
    insecure_node_config.enforce_encryption = false;
    insecure_node_config.allow_plaintext = true;
    
    let insecure_node = P2PNode::new(insecure_node_config).await?;
    
    // Attempt connection from insecure node should fail
    let insecure_addrs = insecure_node.listen_addrs().await?;
    let result = timeout(
        Duration::from_secs(5),
        node2.connect(insecure_addrs[0].clone())
    ).await;
    
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Connection to insecure node should be rejected"
    );
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    insecure_node.stop().await?;
    
    Ok(())
}

/// Performance comparison between transports
#[tokio::test]
async fn test_transport_performance_comparison() -> Result<()> {
    let mut perf = PerformanceTest::new();
    
    // Test QUIC performance
    let quic_throughput = perf.measure_async("quic_performance", async {
        let config1 = TestNodeConfig::builder().port(9170).build();
        let mut quic_config1 = config1;
        quic_config1.preferred_transport = TransportType::QUIC;
        
        let config2 = TestNodeConfig::builder().port(9171).build();
        let mut quic_config2 = config2;
        quic_config2.preferred_transport = TransportType::QUIC;
        
        let node1 = P2PNode::new(quic_config1).await?;
        let node2 = P2PNode::new(quic_config2).await?;
        
        let node1_addrs = node1.listen_addrs().await?;
        node2.connect(node1_addrs[0].clone()).await?;
        
        let node1_id = node1.peer_id();
        let test_data = TestDataGen::random_bytes(1024 * 1024); // 1MB
        
        let start = std::time::Instant::now();
        for _ in 0..10 {
            node2.send_message(&node1_id, test_data.clone()).await?;
            let _ = node1.wait_for_message().await?;
        }
        let duration = start.elapsed();
        
        let throughput = (test_data.len() * 10 * 8) as f64 / 
                        (duration.as_secs_f64() * 1_000_000.0);
        
        node1.stop().await?;
        node2.stop().await?;
        
        Ok::<f64, anyhow::Error>(throughput)
    }).await?;
    
    // Test TCP performance
    let tcp_throughput = perf.measure_async("tcp_performance", async {
        let config1 = TestNodeConfig::builder().port(9172).build();
        let mut tcp_config1 = config1;
        tcp_config1.preferred_transport = TransportType::TCP;
        
        let config2 = TestNodeConfig::builder().port(9173).build();
        let mut tcp_config2 = config2;
        tcp_config2.preferred_transport = TransportType::TCP;
        
        let node1 = P2PNode::new(tcp_config1).await?;
        let node2 = P2PNode::new(tcp_config2).await?;
        
        let node1_addrs = node1.listen_addrs().await?;
        node2.connect(node1_addrs[0].clone()).await?;
        
        let node1_id = node1.peer_id();
        let test_data = TestDataGen::random_bytes(1024 * 1024); // 1MB
        
        let start = std::time::Instant::now();
        for _ in 0..10 {
            node2.send_message(&node1_id, test_data.clone()).await?;
            let _ = node1.wait_for_message().await?;
        }
        let duration = start.elapsed();
        
        let throughput = (test_data.len() * 10 * 8) as f64 / 
                        (duration.as_secs_f64() * 1_000_000.0);
        
        node1.stop().await?;
        node2.stop().await?;
        
        Ok::<f64, anyhow::Error>(throughput)
    }).await?;
    
    perf.print_results();
    
    println!("QUIC throughput: {:.2} Mbps", quic_throughput);
    println!("TCP throughput: {:.2} Mbps", tcp_throughput);
    
    // Both should have reasonable performance
    assert!(quic_throughput > 10.0, "QUIC throughput too low");
    assert!(tcp_throughput > 10.0, "TCP throughput too low");
    
    Ok(())
}

/// Test connection pooling and reuse
#[tokio::test]
async fn test_connection_pooling() -> Result<()> {
    let config1 = TestNodeConfig::builder()
        .port(9180)
        .build();
    let mut node_config1 = config1;
    node_config1.enable_connection_pooling = true;
    node_config1.max_connections_per_peer = 3;
    
    let config2 = TestNodeConfig::builder()
        .port(9181)
        .build();
    let mut node_config2 = config2;
    node_config2.enable_connection_pooling = true;
    node_config2.max_connections_per_peer = 3;
    
    let node1 = P2PNode::new(node_config1).await?;
    let node2 = P2PNode::new(node_config2).await?;
    
    let node1_addrs = node1.listen_addrs().await?;
    let node1_id = node1.peer_id();
    
    // Establish multiple connections to the same peer
    for i in 0..3 {
        node2.connect(node1_addrs[0].clone()).await?;
        println!("Established connection {}", i + 1);
    }
    
    // Verify connection pool
    let pool_info = node2.get_connection_pool_info(&node1_id).await?;
    assert_eq!(pool_info.active_connections, 3);
    assert!(pool_info.total_connections >= 3);
    
    // Test load balancing across connections
    let test_data = b"pooled_message".to_vec();
    for i in 0..9 {
        node2.send_message(&node1_id, format!("message_{}", i).into_bytes()).await?;
    }
    
    // Verify messages were distributed across connections
    let pool_stats = node2.get_connection_pool_stats(&node1_id).await?;
    assert!(pool_stats.messages_per_connection.len() == 3);
    
    // All connections should have been used
    for (conn_id, message_count) in pool_stats.messages_per_connection {
        assert!(
            message_count > 0,
            "Connection {} should have been used for load balancing",
            conn_id
        );
    }
    
    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}