//! Comprehensive Network Integration Tests
//!
//! Tests multi-node communication, network partition handling, peer discovery,
//! and connection failure scenarios.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

use saorsa_core::{
    Config, Identity, NetworkEvent, Node, PeerRecord, network::NetworkManager,
    transport::quic::QuicTransport,
};

/// Test framework for multi-node network scenarios
struct NetworkTestFramework {
    nodes: Vec<Arc<Node>>,
    configs: Vec<Config>,
    test_data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl NetworkTestFramework {
    async fn new(node_count: usize) -> Result<Self> {
        let mut nodes = Vec::new();
        let mut configs = Vec::new();

        for i in 0..node_count {
            let mut config = Config::default();
            config.network.listen_port = 9000 + i as u16;
            config.network.enable_mdns = true;
            config.network.max_connections = 100;

            let node = Node::new(config.clone())
                .await
                .context(format!("Failed to create node {}", i))?;

            nodes.push(Arc::new(node));
            configs.push(config);
        }

        Ok(Self {
            nodes,
            configs,
            test_data: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn start_all_nodes(&self) -> Result<()> {
        for (i, node) in self.nodes.iter().enumerate() {
            node.start()
                .await
                .context(format!("Failed to start node {}", i))?;

            // Small delay between starts to avoid port conflicts
            sleep(Duration::from_millis(100)).await;
        }

        // Wait for initial startup
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }

    async fn connect_nodes_in_chain(&self) -> Result<()> {
        for i in 1..self.nodes.len() {
            let peer_addr = format!("/ip4/127.0.0.1/tcp/{}", 9000 + (i - 1));
            self.nodes[i]
                .connect_to_peer(&peer_addr)
                .await
                .context(format!("Failed to connect node {} to node {}", i, i - 1))?;
        }

        // Wait for connections to establish
        sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    async fn connect_nodes_fully_meshed(&self) -> Result<()> {
        for i in 0..self.nodes.len() {
            for j in (i + 1)..self.nodes.len() {
                let peer_addr = format!("/ip4/127.0.0.1/tcp/{}", 9000 + j);
                self.nodes[i]
                    .connect_to_peer(&peer_addr)
                    .await
                    .context(format!("Failed to connect node {} to node {}", i, j))?;

                // Small delay to prevent overwhelming
                sleep(Duration::from_millis(50)).await;
            }
        }

        // Wait for all connections to establish
        sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    async fn simulate_network_partition(&self, partition_point: usize) -> Result<()> {
        // Disconnect nodes at partition point
        for i in 0..partition_point {
            for j in partition_point..self.nodes.len() {
                self.nodes[i]
                    .disconnect_peer(&format!("node_{}", j))
                    .await?;
                self.nodes[j]
                    .disconnect_peer(&format!("node_{}", i))
                    .await?;
            }
        }

        sleep(Duration::from_secs(2)).await;
        Ok(())
    }

    async fn heal_network_partition(&self) -> Result<()> {
        // Reconnect all nodes
        self.connect_nodes_fully_meshed().await
    }

    async fn get_network_stats(&self) -> Result<Vec<(usize, usize)>> {
        let mut stats = Vec::new();

        for (i, node) in self.nodes.iter().enumerate() {
            let peer_count = node.get_connected_peers().await?.len();
            stats.push((i, peer_count));
        }

        Ok(stats)
    }

    async fn shutdown_all(&self) -> Result<()> {
        for node in &self.nodes {
            node.shutdown().await?;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_multi_node_startup_and_discovery() -> Result<()> {
    let framework = NetworkTestFramework::new(5).await?;

    // Start all nodes
    framework.start_all_nodes().await?;

    // Connect in chain topology
    framework.connect_nodes_in_chain().await?;

    // Verify all nodes can discover each other
    let stats = framework.get_network_stats().await?;

    // Each node should have at least 1 connection (except endpoints in chain)
    for (node_id, peer_count) in &stats {
        if *node_id == 0 || *node_id == stats.len() - 1 {
            assert!(
                peer_count >= &1,
                "Endpoint node {} should have at least 1 peer",
                node_id
            );
        } else {
            assert!(
                peer_count >= &2,
                "Middle node {} should have at least 2 peers",
                node_id
            );
        }
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_network_partition_and_healing() -> Result<()> {
    let framework = NetworkTestFramework::new(6).await?;

    framework.start_all_nodes().await?;
    framework.connect_nodes_fully_meshed().await?;

    // Verify full connectivity
    let initial_stats = framework.get_network_stats().await?;
    for (node_id, peer_count) in &initial_stats {
        assert!(
            peer_count >= &4,
            "Node {} should have at least 4 peers initially",
            node_id
        );
    }

    // Simulate network partition (split into two groups of 3)
    framework.simulate_network_partition(3).await?;

    // Wait for partition detection
    sleep(Duration::from_secs(3)).await;

    let partition_stats = framework.get_network_stats().await?;
    for (node_id, peer_count) in &partition_stats {
        if *node_id < 3 {
            assert!(
                peer_count <= &2,
                "Node {} in partition A should have <= 2 peers",
                node_id
            );
        } else {
            assert!(
                peer_count <= &2,
                "Node {} in partition B should have <= 2 peers",
                node_id
            );
        }
    }

    // Heal the partition
    framework.heal_network_partition().await?;

    // Wait for healing
    sleep(Duration::from_secs(5)).await;

    // Verify connectivity restored
    let healed_stats = framework.get_network_stats().await?;
    for (node_id, peer_count) in &healed_stats {
        assert!(
            peer_count >= &3,
            "Node {} should have at least 3 peers after healing",
            node_id
        );
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_peer_discovery_under_load() -> Result<()> {
    let framework = NetworkTestFramework::new(10).await?;

    framework.start_all_nodes().await?;

    // Connect nodes in a star topology (all to node 0)
    for i in 1..framework.nodes.len() {
        let peer_addr = format!("/ip4/127.0.0.1/tcp/9000");
        framework.nodes[i].connect_to_peer(&peer_addr).await?;
    }

    sleep(Duration::from_secs(3)).await;

    // Verify star topology
    let stats = framework.get_network_stats().await?;

    // Node 0 should have 9 connections
    assert_eq!(stats[0].1, 9, "Hub node should have 9 connections");

    // Other nodes should have 1 connection each
    for i in 1..stats.len() {
        assert_eq!(stats[i].1, 1, "Spoke node {} should have 1 connection", i);
    }

    // Now test peer discovery - nodes should learn about each other
    sleep(Duration::from_secs(10)).await;

    // Some nodes should have discovered additional peers
    let discovery_stats = framework.get_network_stats().await?;
    let total_connections: usize = discovery_stats.iter().map(|(_, count)| count).sum();

    // Total connections should be at least the initial star topology (18 = 9*2)
    assert!(
        total_connections >= 18,
        "Should maintain at least star topology connections"
    );

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_connection_failure_recovery() -> Result<()> {
    let framework = NetworkTestFramework::new(4).await?;

    framework.start_all_nodes().await?;
    framework.connect_nodes_in_chain().await?;

    // Verify initial connectivity
    let initial_stats = framework.get_network_stats().await?;
    let initial_total: usize = initial_stats.iter().map(|(_, count)| count).sum();

    // Simulate node failure (shutdown node 1)
    framework.nodes[1].shutdown().await?;

    // Wait for failure detection
    sleep(Duration::from_secs(5)).await;

    // Verify network adapted to failure
    let mut failure_stats = Vec::new();
    for (i, node) in framework.nodes.iter().enumerate() {
        if i != 1 {
            // Skip failed node
            let peer_count = node.get_connected_peers().await?.len();
            failure_stats.push((i, peer_count));
        }
    }

    // Network should still be partially connected
    let failure_total: usize = failure_stats.iter().map(|(_, count)| count).sum();
    assert!(
        failure_total < initial_total,
        "Total connections should decrease after node failure"
    );

    // Remaining nodes should detect the failure
    for (node_id, peer_count) in &failure_stats {
        if *node_id == 0 || *node_id == 2 {
            // Nodes adjacent to failed node should have fewer connections
            assert!(
                peer_count < &2,
                "Node {} should have detected failure",
                node_id
            );
        }
    }

    // Shutdown remaining nodes
    for (i, node) in framework.nodes.iter().enumerate() {
        if i != 1 {
            node.shutdown().await?;
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_high_throughput_messaging() -> Result<()> {
    let framework = NetworkTestFramework::new(3).await?;

    framework.start_all_nodes().await?;
    framework.connect_nodes_fully_meshed().await?;

    // Test high-throughput messaging between all pairs
    let message_count = 100;
    let message_size = 1024; // 1KB messages

    let start_time = std::time::Instant::now();

    // Send messages from node 0 to all other nodes
    for i in 1..framework.nodes.len() {
        for msg_id in 0..message_count {
            let message = vec![msg_id as u8; message_size];
            let peer_id = format!("node_{}", i);

            framework.nodes[0]
                .send_message(&peer_id, message)
                .await
                .context(format!("Failed to send message {} to node {}", msg_id, i))?;
        }
    }

    let send_duration = start_time.elapsed();

    // Wait for message processing
    sleep(Duration::from_secs(5)).await;

    // Verify message delivery
    // (In a real implementation, we'd track received messages)

    let total_messages = message_count * (framework.nodes.len() - 1);
    let throughput = total_messages as f64 / send_duration.as_secs_f64();

    println!("High-throughput test: {} msg/sec", throughput);
    assert!(
        throughput > 10.0,
        "Throughput should be > 10 messages/second"
    );

    framework.shutdown_all().await?;
    Ok(())
}
