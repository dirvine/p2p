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

//! Network module integration tests
//!
//! Comprehensive tests for the core networking functionality including:
//! - Node creation and configuration
//! - Peer discovery and connection management
//! - Network topology and routing
//! - Connection stability and recovery
//! - IPv4/IPv6 dual stack support

use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

use p2p_foundation::{P2PNode, NodeConfig, Multiaddr};
use crate::common::{TestNetwork, TestAssertions, PerformanceTest};

// Integration test submodules - TBD
// mod node_lifecycle;
// mod peer_discovery;
// mod connection_management;
// mod topology;
// mod ipv6_support;

/// Test node creation with default configuration
#[tokio::test]
async fn test_node_creation_default() -> Result<()> {
    let config = NodeConfig::default();
    let node = P2PNode::new(config).await?;
    
    // Verify node is created with valid peer ID
    assert!(!node.peer_id().to_string().is_empty());
    
    // Verify node is listening
    let addrs = node.listen_addrs().await?;
    assert!(!addrs.is_empty(), "Node should be listening on at least one address");
    
    node.stop().await?;
    Ok(())
}

/// Test node creation with custom configuration
#[tokio::test]
async fn test_node_creation_custom() -> Result<()> {
    let config = NodeConfig::builder()
        .port(9001)
        .enable_ipv6(true)
        .enable_metrics(true)
        .enable_mcp(true)
        .build();
    
    let node = P2PNode::new(config).await?;
    
    // Verify node configuration
    let addrs = node.listen_addrs().await?;
    assert!(addrs.iter().any(|addr| addr.to_string().contains("9001")));
    
    // Verify IPv6 is enabled
    assert!(addrs.iter().any(|addr| addr.to_string().contains("ip6")));
    
    // Verify metrics are enabled
    let metrics = node.get_metrics().await?;
    assert!(metrics.is_some());
    
    // Verify MCP server is running
    let mcp_services = node.mcp_list_services().await?;
    assert!(!mcp_services.is_empty());
    
    node.stop().await?;
    Ok(())
}

/// Test basic peer connection
#[tokio::test]
async fn test_peer_connection() -> Result<()> {
    let network = TestNetwork::simple(2).await?;
    
    // Verify nodes are connected
    let node1_peers = network.node(0)?.peer_count().await;
    let node2_peers = network.node(1)?.peer_count().await;
    
    assert!(node1_peers >= 1, "Node 1 should have at least 1 peer");
    assert!(node2_peers >= 1, "Node 2 should have at least 1 peer");
    
    // Verify bidirectional connectivity
    let node1_id = network.node(1)?.peer_id();
    let node2_id = network.node(0)?.peer_id();
    
    assert!(network.node(0)?.is_connected(&node1_id).await?);
    assert!(network.node(1)?.is_connected(&node2_id).await?);
    
    network.stop().await?;
    Ok(())
}

/// Test multiple peer connections
#[tokio::test]
async fn test_multiple_peer_connections() -> Result<()> {
    let network = TestNetwork::simple(5).await?;
    
    // Wait for full discovery
    network.wait_for_discovery().await?;
    
    // Verify all nodes have discovered each other
    for i in 0..5 {
        let peer_count = network.node(i)?.peer_count().await;
        assert!(
            peer_count >= 4,
            "Node {} should have at least 4 peers, got {}",
            i, peer_count
        );
    }
    
    // Verify full connectivity
    TestAssertions::assert_full_connectivity(&network).await?;
    
    network.stop().await?;
    Ok(())
}

/// Test connection timeout and retry
#[tokio::test]
async fn test_connection_timeout() -> Result<()> {
    let node = P2PNode::new(NodeConfig::default()).await?;
    
    // Try to connect to non-existent peer
    let invalid_addr: Multiaddr = "/ip4/192.0.2.1/tcp/1234".parse()?;
    let start = std::time::Instant::now();
    
    let result = timeout(
        Duration::from_secs(5),
        node.connect_peer(&invalid_addr.to_string())
    ).await;
    
    assert!(result.is_err() || result.unwrap().is_err());
    assert!(start.elapsed() >= Duration::from_secs(1)); // Should respect timeout
    
    node.stop().await?;
    Ok(())
}

/// Test peer disconnection and reconnection
#[tokio::test]
async fn test_peer_disconnection_reconnection() -> Result<()> {
    let mut network = TestNetwork::simple(2).await?;
    
    // Verify initial connection
    let node1_id = network.node(1)?.peer_id();
    assert!(network.node(0)?.is_connected(&node1_id).await?);
    
    // Disconnect peer
    network.node_mut(0)?.disconnect(&node1_id).await?;
    
    // Wait for disconnection to propagate
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!network.node(0)?.is_connected(&node1_id).await?);
    
    // Reconnect
    let node1_addr = network.addrs[1].clone();
    network.node_mut(0)?.connect_peer(&node1_addr.to_string()).await?;
    
    // Wait for reconnection
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(network.node(0)?.is_connected(&node1_id).await?);
    
    network.stop().await?;
    Ok(())
}

/// Test network resilience to node failure
#[tokio::test]
async fn test_network_resilience() -> Result<()> {
    let mut network = TestNetwork::simple(4).await?;
    
    // Wait for full discovery
    network.wait_for_discovery().await?;
    
    // Remove one node (simulate failure)
    let failed_node = network.nodes.remove(2);
    failed_node.stop().await?;
    
    // Wait for failure detection
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Verify remaining nodes still have connectivity
    for i in 0..network.nodes.len() {
        for j in (i + 1)..network.nodes.len() {
            let peer_id = network.nodes[j].peer_id();
            assert!(
                network.nodes[i].can_reach_peer(peer_id).await?,
                "Node {} cannot reach node {} after failure",
                i, j
            );
        }
    }
    
    network.stop().await?;
    Ok(())
}

/// Test bandwidth and connection limits
#[tokio::test]
async fn test_connection_limits() -> Result<()> {
    let config = NodeConfig::builder()
        .port(9010)
        .build();
    
    let mut node_config = config;
    node_config.max_peers = 2; // Limit to 2 peers
    
    let node = P2PNode::new(node_config).await?;
    
    // Create 3 other nodes and try to connect all to the limited node
    let mut other_nodes = Vec::new();
    for i in 0..3 {
        let other_config = NodeConfig::builder()
            .port(9011 + i as u16)
            .build();
        other_nodes.push(P2PNode::new(other_config).await?);
    }
    
    let node_addr = node.listen_addrs().await?[0].clone();
    
    // Connect all nodes to the limited node
    for other_node in &other_nodes {
        let _ = other_node.connect_peer(&node_addr.to_string()).await;
    }
    
    // Wait for connections to stabilize
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Verify connection limit is respected
    let peer_count = node.peer_count().await;
    assert!(
        peer_count <= 2,
        "Node should respect max_peers limit, got {} peers",
        peer_count
    );
    
    // Cleanup
    node.stop().await?;
    for other_node in other_nodes {
        other_node.stop().await?;
    }
    
    Ok(())
}

/// Test network topology discovery
#[tokio::test]
async fn test_topology_discovery() -> Result<()> {
    let network = TestNetwork::simple(6).await?;
    
    // Wait for full discovery
    network.wait_for_discovery().await?;
    
    // Verify each node knows about network topology
    for (i, node) in network.nodes.iter().enumerate() {
        let topology = node.get_network_topology().await?;
        
        // Should know about all other nodes
        assert!(
            topology.nodes.len() >= 5,
            "Node {} should know about at least 5 other nodes, got {}",
            i, topology.nodes.len()
        );
        
        // Should have routing information
        assert!(
            !topology.routes.is_empty(),
            "Node {} should have routing information",
            i
        );
    }
    
    network.stop().await?;
    Ok(())
}

/// Performance test for connection establishment
#[tokio::test]
async fn test_connection_performance() -> Result<()> {
    let mut perf = PerformanceTest::new();
    
    // Test single connection performance
    let connection_time = perf.measure_async("single_connection", async {
        let network = TestNetwork::simple(2).await?;
        network.stop().await?;
        Ok::<(), anyhow::Error>(())
    }).await?;
    
    // Test multiple connection performance
    let multi_connection_time = perf.measure_async("multi_connection", async {
        let network = TestNetwork::simple(10).await?;
        network.wait_for_discovery().await?;
        network.stop().await?;
        Ok::<(), anyhow::Error>(())
    }).await?;
    
    // Print performance results
    perf.print_results();
    
    // Basic performance assertions
    assert!(
        perf.get_measurement("single_connection").unwrap() < Duration::from_secs(5),
        "Single connection should take less than 5 seconds"
    );
    
    assert!(
        perf.get_measurement("multi_connection").unwrap() < Duration::from_secs(30),
        "Multi-node network setup should take less than 30 seconds"
    );
    
    Ok(())
}

/// Test IPv4 and IPv6 mixed network
#[tokio::test]
async fn test_mixed_ipv4_ipv6_network() -> Result<()> {
    // Create IPv4 node
    let ipv4_config = NodeConfig::builder()
        .port(9020)
        .enable_ipv6(false)
        .build();
    let ipv4_node = P2PNode::new(ipv4_config).await?;
    
    // Create IPv6 node
    let ipv6_config = NodeConfig::builder()
        .port(9021)
        .enable_ipv6(true)
        .build();
    let ipv6_node = P2PNode::new(ipv6_config).await?;
    
    // Test that nodes can discover and connect despite different IP versions
    // This should work through dual-stack support or tunneling
    let ipv4_addrs = ipv4_node.listen_addrs().await?;
    let ipv6_addrs = ipv6_node.listen_addrs().await?;
    
    // Try to connect IPv6 node to IPv4 node
    if let Some(ipv4_addr) = ipv4_addrs.iter().find(|addr| addr.to_string().contains("ip4")) {
        let result = timeout(
            Duration::from_secs(10),
            ipv6_node.connect_peer(&ipv4_addr.to_string())
        ).await;
        
        // Connection should succeed or fail gracefully
        match result {
            Ok(Ok(_)) => {
                // Connection successful - verify it works
                let ipv4_peer_id = ipv4_node.peer_id();
                assert!(ipv6_node.is_connected(&ipv4_peer_id).await?);
            },
            Ok(Err(_)) | Err(_) => {
                // Connection failed - this is acceptable for mixed IP versions
                // without tunneling implemented
                println!("Mixed IPv4/IPv6 connection failed - expected without tunneling");
            }
        }
    }
    
    // Cleanup
    ipv4_node.stop().await?;
    ipv6_node.stop().await?;
    
    Ok(())
}

/// Test keep-alive and connection maintenance
#[tokio::test]
async fn test_keep_alive() -> Result<()> {
    let config = TestNetworkConfig {
        node_count: 2,
        bootstrap_wait: Duration::from_secs(2),
        ..Default::default()
    };
    
    let network = TestNetwork::new(config).await?;
    
    // Verify initial connection
    let node1_id = network.node(1)?.peer_id();
    assert!(network.node(0)?.is_connected(&node1_id).await?);
    
    // Wait longer than typical keep-alive interval
    tokio::time::sleep(Duration::from_secs(35)).await;
    
    // Connection should still be alive
    assert!(
        network.node(0)?.is_connected(&node1_id).await?,
        "Connection should be maintained by keep-alive"
    );
    
    // Verify connection is still functional
    let test_data = b"keep-alive test";
    network.node(0)?.send_message(&node1_id, test_data.to_vec()).await?;
    
    network.stop().await?;
    Ok(())
}

/// Stress test with rapid connection/disconnection
#[tokio::test]
async fn test_connection_stress() -> Result<()> {
    let node1 = P2PNode::new(NodeConfig::builder().port(9030).build()).await?;
    let node2 = P2PNode::new(NodeConfig::builder().port(9031).build()).await?;
    
    let node1_addr = node1.listen_addrs().await?[0].clone();
    let node2_id = node2.peer_id();
    
    // Rapidly connect and disconnect
    for i in 0..10 {
        println!("Stress test iteration {}", i);
        
        // Connect
        node2.connect_peer(&node1_addr.to_string()).await?;
        assert!(node1.is_connected(&node2_id).await?);
        
        // Disconnect
        node1.disconnect(&node2_id).await?;
        
        // Brief pause
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert!(!node1.is_connected(&node2_id).await?);
    }
    
    // Final cleanup
    node1.stop().await?;
    node2.stop().await?;
    
    Ok(())
}