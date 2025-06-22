//! Common test utilities and helpers for integration tests.
//!
//! This module provides shared testing infrastructure for all integration tests,
//! including test network setup, configuration builders, and assertion helpers.

use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use anyhow::Result;
use tokio::time::timeout;
use p2p_foundation::{P2PNode, NodeConfig, PeerId, Multiaddr};

/// Test network configuration
#[derive(Debug, Clone)]
pub struct TestNetworkConfig {
    pub node_count: usize,
    pub base_port: u16,
    pub bootstrap_wait: Duration,
    pub connection_timeout: Duration,
    pub enable_ipv6: bool,
    pub enable_metrics: bool,
}

impl Default for TestNetworkConfig {
    fn default() -> Self {
        Self {
            node_count: 3,
            base_port: 9000,
            bootstrap_wait: Duration::from_secs(5),
            connection_timeout: Duration::from_secs(10),
            enable_ipv6: true,
            enable_metrics: false,
        }
    }
}

/// A test network of P2P nodes
#[derive(Debug)]
pub struct TestNetwork {
    pub nodes: Vec<P2PNode>,
    pub configs: Vec<NodeConfig>,
    pub addrs: Vec<Multiaddr>,
    pub config: TestNetworkConfig,
}

impl TestNetwork {
    /// Create a new test network with the specified configuration
    pub async fn new(config: TestNetworkConfig) -> Result<Self> {
        let mut nodes = Vec::new();
        let mut configs = Vec::new();
        let mut addrs = Vec::new();

        // Create node configurations
        for i in 0..config.node_count {
            let port = config.base_port + i as u16;
            let node_config = TestNodeConfig::builder()
                .port(port)
                .enable_ipv6(config.enable_ipv6)
                .enable_metrics(config.enable_metrics)
                .build();
            
            let addr = if config.enable_ipv6 {
                format!("/ip6/::1/tcp/{}", port).parse()?
            } else {
                format!("/ip4/127.0.0.1/tcp/{}", port).parse()?
            };

            configs.push(node_config);
            addrs.push(addr);
        }

        // Create nodes
        for (i, node_config) in configs.iter().enumerate() {
            let node = P2PNode::new(node_config.clone()).await
                .map_err(|e| anyhow::anyhow!("Failed to create node {}: {}", i, e))?;
            nodes.push(node);
        }

        let mut network = TestNetwork {
            nodes,
            configs,
            addrs,
            config,
        };

        // Bootstrap the network
        network.bootstrap().await?;

        Ok(network)
    }

    /// Create a simple test network with default configuration
    pub async fn simple(node_count: usize) -> Result<Self> {
        let config = TestNetworkConfig {
            node_count,
            ..Default::default()
        };
        Self::new(config).await
    }

    /// Bootstrap the network by connecting nodes
    async fn bootstrap(&mut self) -> Result<()> {
        if self.nodes.len() < 2 {
            return Ok(());
        }

        // Connect each node to the first node (star topology for bootstrap)
        let bootstrap_addr = self.addrs[0].clone();
        for i in 1..self.nodes.len() {
            timeout(
                self.config.connection_timeout,
                self.nodes[i].connect(bootstrap_addr.clone())
            ).await
            .map_err(|_| anyhow::anyhow!("Bootstrap connection timeout for node {}", i))?
            .map_err(|e| anyhow::anyhow!("Bootstrap connection failed for node {}: {}", i, e))?;
        }

        // Wait for network stabilization
        tokio::time::sleep(self.config.bootstrap_wait).await;

        Ok(())
    }

    /// Get a node by index
    pub fn node(&self, index: usize) -> Result<&P2PNode> {
        self.nodes.get(index)
            .ok_or_else(|| anyhow::anyhow!("Node index {} out of bounds", index))
    }

    /// Get a mutable node by index
    pub fn node_mut(&mut self, index: usize) -> Result<&mut P2PNode> {
        self.nodes.get_mut(index)
            .ok_or_else(|| anyhow::anyhow!("Node index {} out of bounds", index))
    }

    /// Wait for all nodes to discover each other
    pub async fn wait_for_discovery(&self) -> Result<()> {
        let expected_peers = self.nodes.len() - 1; // Each node should know about all others
        let timeout_duration = Duration::from_secs(30);

        for (i, node) in self.nodes.iter().enumerate() {
            timeout(timeout_duration, async {
                loop {
                    let peer_count = node.peer_count().await;
                    if peer_count >= expected_peers {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await
            .map_err(|_| anyhow::anyhow!("Discovery timeout for node {}", i))?;
        }

        Ok(())
    }

    /// Shutdown all nodes gracefully
    pub async fn shutdown(self) -> Result<()> {
        for (i, node) in self.nodes.into_iter().enumerate() {
            node.shutdown().await
                .map_err(|e| anyhow::anyhow!("Failed to shutdown node {}: {}", i, e))?;
        }
        Ok(())
    }
}

/// Builder for test node configurations
#[derive(Debug, Default)]
pub struct TestNodeConfig {
    port: Option<u16>,
    enable_ipv6: bool,
    enable_metrics: bool,
    enable_mcp: bool,
    bootstrap_peers: Vec<Multiaddr>,
    custom_keypair: Option<Vec<u8>>,
}

impl TestNodeConfig {
    pub fn builder() -> Self {
        Self::default()
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn enable_ipv6(mut self, enable: bool) -> Self {
        self.enable_ipv6 = enable;
        self
    }

    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    pub fn enable_mcp(mut self, enable: bool) -> Self {
        self.enable_mcp = enable;
        self
    }

    pub fn bootstrap_peers(mut self, peers: Vec<Multiaddr>) -> Self {
        self.bootstrap_peers = peers;
        self
    }

    pub fn custom_keypair(mut self, keypair: Vec<u8>) -> Self {
        self.custom_keypair = Some(keypair);
        self
    }

    pub fn build(self) -> NodeConfig {
        let port = self.port.unwrap_or(0); // 0 means random port
        let listen_addr = if self.enable_ipv6 {
            format!("/ip6/::1/tcp/{}", port)
        } else {
            format!("/ip4/127.0.0.1/tcp/{}", port)
        };

        NodeConfig {
            listen_addrs: vec![listen_addr.parse().expect("Valid multiaddr")],
            bootstrap_peers: self.bootstrap_peers,
            enable_ipv6: self.enable_ipv6,
            enable_metrics: self.enable_metrics,
            enable_mcp_server: self.enable_mcp,
            keypair: self.custom_keypair,
            // Test defaults
            connection_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
            max_peers: 50,
            ..Default::default()
        }
    }
}

/// Test data generators
pub struct TestDataGen;

impl TestDataGen {
    /// Generate random test data of specified size
    pub fn random_bytes(size: usize) -> Vec<u8> {
        use rand::RngCore;
        let mut data = vec![0u8; size];
        rand::thread_rng().fill_bytes(&mut data);
        data
    }

    /// Generate a test DHT key
    pub fn dht_key(prefix: &str) -> p2p_foundation::Key {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(prefix.as_bytes());
        let hash = hasher.finalize();
        p2p_foundation::Key::from_bytes(&hash[..])
    }

    /// Generate test MCP tool configuration
    pub fn mcp_tool(name: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "description": format!("Test tool: {}", name),
            "input_schema": {
                "type": "object",
                "properties": {
                    "data": { "type": "string" }
                }
            }
        })
    }
}

/// Test assertion helpers
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that all nodes in the network can reach each other
    pub async fn assert_full_connectivity(network: &TestNetwork) -> Result<()> {
        for i in 0..network.nodes.len() {
            for j in 0..network.nodes.len() {
                if i != j {
                    let reachable = network.nodes[i].can_reach_peer(
                        network.nodes[j].peer_id()
                    ).await?;
                    assert!(
                        reachable,
                        "Node {} cannot reach node {}",
                        i, j
                    );
                }
            }
        }
        Ok(())
    }

    /// Assert DHT convergence across the network
    pub async fn assert_dht_convergence(network: &TestNetwork, key: &p2p_foundation::Key, expected_value: &[u8]) -> Result<()> {
        for (i, node) in network.nodes.iter().enumerate() {
            let stored_value = timeout(
                Duration::from_secs(10),
                node.dht_get(key)
            ).await
            .map_err(|_| anyhow::anyhow!("DHT get timeout on node {}", i))?
            .map_err(|e| anyhow::anyhow!("DHT get failed on node {}: {}", i, e))?;

            match stored_value {
                Some(value) => assert_eq!(
                    value, expected_value,
                    "DHT value mismatch on node {}", i
                ),
                None => return Err(anyhow::anyhow!(
                    "DHT key not found on node {}", i
                )),
            }
        }
        Ok(())
    }

    /// Assert that MCP services are available across the network
    pub async fn assert_mcp_availability(network: &TestNetwork) -> Result<()> {
        for (i, node) in network.nodes.iter().enumerate() {
            let services = node.mcp_list_services().await
                .map_err(|e| anyhow::anyhow!("Failed to list MCP services on node {}: {}", i, e))?;
            
            assert!(
                !services.is_empty(),
                "No MCP services available on node {}",
                i
            );
        }
        Ok(())
    }
}

/// Performance test utilities
pub struct PerformanceTest {
    start_time: std::time::Instant,
    measurements: HashMap<String, Duration>,
}

impl PerformanceTest {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            measurements: HashMap::new(),
        }
    }

    pub fn measure<F, R>(&mut self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed();
        self.measurements.insert(name.to_string(), duration);
        result
    }

    pub async fn measure_async<F, Fut, R>(&mut self, name: &str, f: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = std::time::Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        self.measurements.insert(name.to_string(), duration);
        result
    }

    pub fn get_measurement(&self, name: &str) -> Option<Duration> {
        self.measurements.get(name).copied()
    }

    pub fn total_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn print_results(&self) {
        println!("Performance Test Results:");
        println!("========================");
        for (name, duration) in &self.measurements {
            println!("{}: {:?}", name, duration);
        }
        println!("Total Time: {:?}", self.total_time());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_creation() {
        let config = TestNetworkConfig {
            node_count: 2,
            ..Default::default()
        };
        
        let result = TestNetwork::new(config).await;
        assert!(result.is_ok(), "Failed to create test network: {:?}", result);
        
        let network = result.unwrap();
        assert_eq!(network.nodes.len(), 2);
        
        // Clean shutdown
        network.shutdown().await.expect("Failed to shutdown network");
    }

    #[test]
    fn test_data_generation() {
        let data = TestDataGen::random_bytes(100);
        assert_eq!(data.len(), 100);
        
        let key = TestDataGen::dht_key("test");
        assert!(!key.as_bytes().is_empty());
        
        let tool = TestDataGen::mcp_tool("test_tool");
        assert_eq!(tool["name"], "test_tool");
    }

    #[test]
    fn test_performance_measurement() {
        let mut perf = PerformanceTest::new();
        
        perf.measure("test_op", || {
            std::thread::sleep(Duration::from_millis(10));
        });
        
        let measurement = perf.get_measurement("test_op").unwrap();
        assert!(measurement >= Duration::from_millis(10));
    }
}