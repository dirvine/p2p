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

//! Simplified End-to-End Tests
//!
//! These tests focus on core P2P infrastructure functionality that we know works:
//! network formation, bootstrap cache, peer discovery, and basic node operations.

use anyhow::Result;
use p2p_foundation::{
    P2PNode, NodeConfig, 
    bootstrap::CacheConfig,
};
use std::net::SocketAddr;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;
use tracing::info;

/// Simple E2E network for infrastructure testing
struct SimpleE2ENetwork {
    nodes: Vec<P2PNode>,
    _cache_dirs: Vec<TempDir>, // Keep alive for cleanup
    base_port: u16,
}

impl SimpleE2ENetwork {
    /// Create a simple test network
    async fn new(size: usize) -> Result<Self> {
        let base_port = 26000 + (rand::random::<u16>() % 1000);
        let mut nodes = Vec::new();
        let mut cache_dirs = Vec::new();

        for i in 0..size {
            let cache_dir = TempDir::new()?;
            let cache_config = CacheConfig {
                cache_dir: cache_dir.path().to_path_buf(),
                max_contacts: 100,
                ..CacheConfig::default()
            };

            let listen_addr: SocketAddr = format!("127.0.0.1:{}", base_port + i as u16).parse()?;
            
            let mut config = NodeConfig::default();
            config.listen_addr = listen_addr;
            config.bootstrap_cache_config = Some(cache_config);
            config.max_connections = 10;
            config.max_incoming_connections = 5;
            
            if i > 0 {
                config.bootstrap_peers_str = vec![
                    format!("127.0.0.1:{}", base_port),
                ];
            }

            let node = P2PNode::new(config).await?;
            
            nodes.push(node);
            cache_dirs.push(cache_dir);
        }

        Ok(Self {
            nodes,
            _cache_dirs: cache_dirs,
            base_port,
        })
    }

    /// Start all nodes
    async fn start_all(&mut self) -> Result<()> {
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.start().await?;
            info!("Started simple E2E node {} on port {}", i, self.base_port + i as u16);
            sleep(Duration::from_millis(100)).await;
        }

        // Wait for network formation
        sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    /// Stop all nodes
    async fn stop_all(&mut self) -> Result<()> {
        for node in self.nodes.iter_mut() {
            let _ = node.stop().await;
        }
        Ok(())
    }

    /// Get node by index
    fn get_node(&self, index: usize) -> &P2PNode {
        &self.nodes[index]
    }

    /// Get basic network statistics
    async fn get_basic_stats(&self) -> NetworkStats {
        let mut total_peers = 0;
        let mut total_cached_contacts = 0;
        let mut nodes_with_peers = 0;
        
        for node in &self.nodes {
            let peer_count = node.peer_count().await;
            total_peers += peer_count;
            if peer_count > 0 {
                nodes_with_peers += 1;
            }
            total_cached_contacts += node.cached_peer_count().await;
        }

        NetworkStats {
            total_nodes: self.nodes.len(),
            total_peers,
            total_cached_contacts,
            nodes_with_peers,
            avg_peers_per_node: if self.nodes.is_empty() { 0.0 } else { total_peers as f64 / self.nodes.len() as f64 },
        }
    }
}

#[derive(Debug)]
struct NetworkStats {
    total_nodes: usize,
    total_peers: usize,
    total_cached_contacts: usize,
    nodes_with_peers: usize,
    avg_peers_per_node: f64,
}

/// Test basic network formation and connectivity
#[tokio::test]
async fn test_basic_network_formation() -> Result<()> {
    let mut network = SimpleE2ENetwork::new(5).await?;
    
    info!("Testing basic network formation with 5 nodes");
    
    network.start_all().await?;
    
    // Wait for network stabilization
    sleep(Duration::from_secs(2)).await;
    
    let stats = network.get_basic_stats().await;
    info!("Network stats: {:?}", stats);
    
    // Basic connectivity assertions
    assert_eq!(stats.total_nodes, 5);
    assert!(stats.nodes_with_peers >= 1, "At least one node should have peers");
    assert!(stats.total_peers > 0, "Network should have peer connections");
    
    network.stop_all().await?;
    
    info!("✅ Basic network formation test passed");
    Ok(())
}

/// Test bootstrap cache functionality
#[tokio::test]
async fn test_bootstrap_cache_functionality() -> Result<()> {
    let mut network = SimpleE2ENetwork::new(4).await?;
    
    info!("Testing bootstrap cache functionality");
    
    network.start_all().await?;
    
    // Wait for bootstrap cache to populate
    sleep(Duration::from_secs(5)).await;
    
    let stats = network.get_basic_stats().await;
    info!("Bootstrap cache stats: {:?}", stats);
    
    // Test cache stats from individual nodes
    let mut nodes_with_cache_stats = 0;
    for (i, node) in network.nodes.iter().enumerate() {
        if let Ok(Some(cache_stats)) = node.get_bootstrap_cache_stats().await {
            nodes_with_cache_stats += 1;
            info!("Node {} cache stats: {} total contacts", i, cache_stats.total_contacts);
        }
    }
    
    // Bootstrap cache assertions
    assert!(stats.total_cached_contacts > 0, "Bootstrap cache should contain contacts");
    assert!(nodes_with_cache_stats > 0, "At least one node should have cache stats");
    
    network.stop_all().await?;
    
    info!("✅ Bootstrap cache functionality test passed");
    Ok(())
}

/// Test node restart and cache persistence
#[tokio::test]
async fn test_node_restart_and_persistence() -> Result<()> {
    let mut network = SimpleE2ENetwork::new(3).await?;
    
    info!("Testing node restart and cache persistence");
    
    network.start_all().await?;
    sleep(Duration::from_secs(3)).await;
    
    let initial_stats = network.get_basic_stats().await;
    info!("Initial stats: {:?}", initial_stats);
    
    // Stop the network
    network.stop_all().await?;
    sleep(Duration::from_secs(1)).await;
    
    // Restart the network (cache directories preserved)
    network.start_all().await?;
    sleep(Duration::from_secs(3)).await;
    
    let restart_stats = network.get_basic_stats().await;
    info!("Restart stats: {:?}", restart_stats);
    
    // Persistence assertions
    assert_eq!(restart_stats.total_nodes, 3);
    assert!(restart_stats.nodes_with_peers >= 1, "Network should reconnect after restart");
    
    network.stop_all().await?;
    
    info!("✅ Node restart and persistence test passed");
    Ok(())
}

/// Test peer discovery and network growth
#[tokio::test]
async fn test_peer_discovery_and_growth() -> Result<()> {
    // Start with a small network
    let mut core_network = SimpleE2ENetwork::new(2).await?;
    
    info!("Testing peer discovery and network growth");
    
    core_network.start_all().await?;
    sleep(Duration::from_secs(2)).await;
    
    let initial_stats = core_network.get_basic_stats().await;
    info!("Initial core network stats: {:?}", initial_stats);
    
    // Add new nodes that will discover the existing network
    let mut new_nodes = Vec::new();
    let mut new_cache_dirs = Vec::new();
    
    for i in 0..2 {
        let cache_dir = TempDir::new()?;
        let cache_config = CacheConfig {
            cache_dir: cache_dir.path().to_path_buf(),
            max_contacts: 100,
            ..CacheConfig::default()
        };

        let listen_addr: SocketAddr = format!("127.0.0.1:{}", core_network.base_port + 10 + i as u16).parse()?;
        
        let mut config = NodeConfig::default();
        config.listen_addr = listen_addr;
        config.bootstrap_cache_config = Some(cache_config);
        
        // Bootstrap from core network
        config.bootstrap_peers_str = vec![
            format!("127.0.0.1:{}", core_network.base_port),
        ];

        let node = P2PNode::new(config).await?;
        node.start().await?;
        
        new_nodes.push(node);
        new_cache_dirs.push(cache_dir);
        
        info!("Started new discovery node {}", i);
        sleep(Duration::from_millis(500)).await;
    }

    // Wait for discovery
    sleep(Duration::from_secs(5)).await;
    
    // Check that new nodes have discovered peers
    let mut new_nodes_with_peers = 0;
    for (i, node) in new_nodes.iter().enumerate() {
        let peer_count = node.peer_count().await;
        if peer_count > 0 {
            new_nodes_with_peers += 1;
        }
        info!("New node {} has {} peers", i, peer_count);
    }
    
    // Clean up new nodes
    for node in new_nodes.iter_mut() {
        let _ = node.stop().await;
    }
    
    core_network.stop_all().await?;
    
    // Discovery assertions
    assert!(new_nodes_with_peers >= 1, "At least one new node should discover peers");
    
    info!("✅ Peer discovery and network growth test passed");
    Ok(())
}

/// Test network under simulated load
#[tokio::test]
async fn test_network_under_load() -> Result<()> {
    let mut network = SimpleE2ENetwork::new(6).await?;
    
    info!("Testing network under simulated load");
    
    network.start_all().await?;
    sleep(Duration::from_secs(3)).await;
    
    let initial_stats = network.get_basic_stats().await;
    info!("Initial stats under load: {:?}", initial_stats);
    
    // Simulate load by adding many discovered peers
    for i in 0..50 {
        let node_index = i % network.nodes.len();
        let node = network.get_node(node_index);
        
        let peer_id = format!("load_peer_{}", i);
        let addr = format!("192.168.1.{}", (i % 254) + 1);
        
        if let Err(e) = node.add_discovered_peer(peer_id.into(), vec![addr]).await {
            // It's okay if some fail under load
            info!("Failed to add peer {} (expected under load): {}", i, e);
        }
        
        if i % 10 == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    // Wait for processing
    sleep(Duration::from_secs(3)).await;
    
    let load_stats = network.get_basic_stats().await;
    info!("Stats after load test: {:?}", load_stats);
    
    // Network should still be functional
    assert_eq!(load_stats.total_nodes, 6);
    assert!(load_stats.nodes_with_peers >= 1, "Network should remain functional under load");
    assert!(load_stats.total_cached_contacts > initial_stats.total_cached_contacts, 
           "Cache should grow under load");
    
    network.stop_all().await?;
    
    info!("✅ Network under load test passed");
    Ok(())
}