//! Bootstrap Cache Integration Tests
//!
//! Comprehensive tests for the bootstrap cache system including multi-node
//! bootstrapping, data storage/retrieval, cache persistence, and network resilience.

use anyhow::Result;
use p2p_foundation::{
    P2PNode, NodeConfig, PeerId, 
    bootstrap::{BootstrapCache, CacheConfig, ContactEntry, MergeCoordinator},
    dht::Key,
};
use std::net::SocketAddr;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use tracing::{info, debug, warn};

/// Test network configuration
const TEST_NETWORK_SIZE: usize = 5;
const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(30);
const DHT_REPLICATION_TIMEOUT: Duration = Duration::from_secs(10);
const TEST_DATA_COUNT: usize = 10;

/// Helper struct for managing test networks with bootstrap caches
struct BootstrapTestNetwork {
    nodes: Vec<P2PNode>,
    cache_dirs: Vec<TempDir>,
    node_configs: Vec<NodeConfig>,
    base_port: u16,
}

impl BootstrapTestNetwork {
    /// Create a new test network with bootstrap cache support
    async fn new(size: usize) -> Result<Self> {
        let base_port = 19000 + (rand::random::<u16>() % 1000);
        let mut nodes = Vec::new();
        let mut cache_dirs = Vec::new();
        let mut node_configs = Vec::new();

        // Create nodes with isolated cache directories
        for i in 0..size {
            let cache_dir = TempDir::new()?;
            let cache_config = CacheConfig {
                cache_dir: cache_dir.path().to_path_buf(),
                max_contacts: 1000,
                ..CacheConfig::default()
            };

            let listen_addr: SocketAddr = format!("127.0.0.1:{}", base_port + i as u16).parse()?;
            
            let mut config = NodeConfig::default();
            config.listen_addr = listen_addr;
            config.bootstrap_cache_config = Some(cache_config);
            
            // Only the first node starts without bootstrap peers
            // Others will bootstrap from previous nodes
            if i > 0 {
                config.bootstrap_peers_str = vec![
                    format!("127.0.0.1:{}", base_port + (i - 1) as u16)
                ];
            }

            let node = P2PNode::new(config.clone()).await?;
            
            nodes.push(node);
            cache_dirs.push(cache_dir);
            node_configs.push(config);
        }

        Ok(Self {
            nodes,
            cache_dirs,
            node_configs,
            base_port,
        })
    }

    /// Start all nodes in the network
    async fn start_all(&mut self) -> Result<()> {
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.start().await?;
            info!("Started node {} on port {}", i, self.base_port + i as u16);
            
            // Small delay to prevent port conflicts
            sleep(Duration::from_millis(100)).await;
        }

        // Wait for initial connections
        sleep(Duration::from_secs(2)).await;
        
        Ok(())
    }

    /// Stop all nodes in the network
    async fn stop_all(&mut self) -> Result<()> {
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.stop().await?;
            debug!("Stopped node {}", i);
        }
        Ok(())
    }

    /// Get node by index
    fn get_node(&self, index: usize) -> &P2PNode {
        &self.nodes[index]
    }

    /// Get mutable node by index
    fn get_node_mut(&mut self, index: usize) -> &mut P2PNode {
        &mut self.nodes[index]
    }

    /// Wait for network convergence
    async fn wait_for_convergence(&self) -> Result<()> {
        let start_time = std::time::Instant::now();
        let timeout_duration = BOOTSTRAP_TIMEOUT;

        while start_time.elapsed() < timeout_duration {
            let mut all_connected = true;
            
            for (i, node) in self.nodes.iter().enumerate() {
                let peer_count = node.peer_count().await;
                if peer_count == 0 {
                    all_connected = false;
                    debug!("Node {} has no peers yet", i);
                    break;
                }
            }

            if all_connected {
                info!("Network convergence achieved in {:?}", start_time.elapsed());
                return Ok(());
            }

            sleep(Duration::from_millis(500)).await;
        }

        anyhow::bail!("Network failed to converge within timeout");
    }

    /// Verify DHT data replication across the network
    async fn verify_dht_replication(&self, key: &Key, expected_value: &Vec<u8>) -> Result<bool> {
        let mut successful_retrievals = 0;

        for (i, node) in self.nodes.iter().enumerate() {
            match timeout(Duration::from_secs(5), node.dht_get(key.clone())).await {
                Ok(Ok(Some(value))) if value == *expected_value => {
                    successful_retrievals += 1;
                    debug!("Node {} successfully retrieved expected value", i);
                }
                Ok(Ok(Some(value))) => {
                    warn!("Node {} retrieved incorrect value: expected {:?}, got {:?}", i, expected_value, value);
                }
                Ok(Ok(None)) => {
                    debug!("Node {} could not find the key", i);
                }
                Ok(Err(e)) => {
                    warn!("Node {} DHT get failed: {}", i, e);
                }
                Err(_) => {
                    warn!("Node {} DHT get timed out", i);
                }
            }
        }

        // Consider replication successful if majority of nodes have the data
        let replication_threshold = (self.nodes.len() + 1) / 2;
        Ok(successful_retrievals >= replication_threshold)
    }

    /// Get cache statistics from all nodes
    async fn get_cache_stats(&self) -> Result<Vec<p2p_foundation::bootstrap::CacheStats>> {
        let mut stats = Vec::new();
        
        for node in &self.nodes {
            if let Some(cache_stats) = node.get_bootstrap_cache_stats().await? {
                stats.push(cache_stats);
            }
        }
        
        Ok(stats)
    }

    /// Restart a specific node and verify it can rejoin via cache
    async fn restart_node(&mut self, index: usize) -> Result<()> {
        info!("Restarting node {}", index);
        
        // Stop the node
        self.nodes[index].stop().await?;
        sleep(Duration::from_millis(500)).await;

        // Create new node with same config (cache directory preserved)
        let new_node = P2PNode::new(self.node_configs[index].clone()).await?;
        self.nodes[index] = new_node;

        // Start the node (should use cache for bootstrap)
        self.nodes[index].start().await?;
        sleep(Duration::from_secs(2)).await;

        Ok(())
    }
}

/// Test basic bootstrap cache functionality with multiple nodes
#[tokio::test]
async fn test_bootstrap_cache_basic_functionality() -> Result<()> {
    let mut network = BootstrapTestNetwork::new(TEST_NETWORK_SIZE).await?;
    
    // Start all nodes
    network.start_all().await?;
    
    // Wait for network convergence
    network.wait_for_convergence().await?;
    
    // Verify all nodes have peers
    for (i, node) in network.nodes.iter().enumerate() {
        let peer_count = node.peer_count().await;
        assert!(peer_count > 0, "Node {} should have at least one peer", i);
        info!("Node {} has {} peers", i, peer_count);
    }

    // Check that bootstrap caches are being populated
    let cache_stats = network.get_cache_stats().await?;
    assert!(!cache_stats.is_empty(), "At least one node should have cache stats");
    
    for (i, stats) in cache_stats.iter().enumerate() {
        info!("Node {} cache: {} total contacts, {} high quality", 
              i, stats.total_contacts, stats.high_quality_contacts);
    }

    network.stop_all().await?;
    Ok(())
}

/// Test DHT operations through bootstrap cache network
#[tokio::test]
async fn test_dht_operations_with_bootstrap_cache() -> Result<()> {
    let mut network = BootstrapTestNetwork::new(TEST_NETWORK_SIZE).await?;
    
    // Start network
    network.start_all().await?;
    network.wait_for_convergence().await?;

    // Store multiple key-value pairs from different nodes
    let test_data: Vec<(Key, Vec<u8>)> = (0..TEST_DATA_COUNT)
        .map(|i| {
            let key = Key::new(format!("test_key_{}", i).as_bytes());
            let value = format!("test_value_{}", i).into_bytes();
            (key, value)
        })
        .collect();

    // Store data using different nodes
    for (i, (key, value)) in test_data.iter().enumerate() {
        let node_index = i % network.nodes.len();
        let node = network.get_node(node_index);
        
        timeout(Duration::from_secs(10), node.dht_put(key.clone(), value.clone())).await??;
        info!("Stored key {} via node {}", i, node_index);
    }

    // Wait for DHT replication
    sleep(DHT_REPLICATION_TIMEOUT).await;

    // Verify data can be retrieved from any node
    let mut successful_operations = 0;
    
    for (i, (key, expected_value)) in test_data.iter().enumerate() {
        if network.verify_dht_replication(key, expected_value).await? {
            successful_operations += 1;
            info!("Data {} successfully replicated across network", i);
        } else {
            warn!("Data {} replication incomplete", i);
        }
    }

    // Expect at least 80% success rate for DHT operations
    let success_rate = successful_operations as f64 / test_data.len() as f64;
    assert!(success_rate >= 0.8, 
           "DHT success rate too low: {:.1}% (expected >= 80%)", 
           success_rate * 100.0);

    info!("DHT operations success rate: {:.1}%", success_rate * 100.0);

    network.stop_all().await?;
    Ok(())
}

/// Test cache persistence and recovery across node restarts
#[tokio::test]
async fn test_cache_persistence_and_recovery() -> Result<()> {
    let mut network = BootstrapTestNetwork::new(3).await?;
    
    // Start network and let it stabilize
    network.start_all().await?;
    network.wait_for_convergence().await?;
    
    // Let nodes discover each other and populate caches
    sleep(Duration::from_secs(5)).await;
    
    // Get initial cache stats
    let initial_stats = network.get_cache_stats().await?;
    let initial_contact_count: usize = initial_stats.iter()
        .map(|s| s.total_contacts)
        .sum();
    
    info!("Initial total contacts across all caches: {}", initial_contact_count);
    
    // Stop all nodes
    network.stop_all().await?;
    sleep(Duration::from_secs(1)).await;
    
    // Restart nodes (should use cached contacts for bootstrap)
    network.start_all().await?;
    
    // Verify network can reconnect using cache
    network.wait_for_convergence().await?;
    
    // Check that caches were loaded and used
    let recovered_stats = network.get_cache_stats().await?;
    let recovered_contact_count: usize = recovered_stats.iter()
        .map(|s| s.total_contacts)
        .sum();
    
    info!("Recovered total contacts across all caches: {}", recovered_contact_count);
    
    // Cache should have preserved contacts
    assert!(recovered_contact_count > 0, "Cache should contain contacts after recovery");
    
    // Verify network is functional after recovery
    let test_key = Key::new(b"recovery_test");
    let test_value = b"recovery_value".to_vec();
    
    network.get_node(0).dht_put(test_key.clone(), test_value.clone()).await?;
    sleep(Duration::from_secs(2)).await;
    
    let retrieved = network.get_node(1).dht_get(test_key).await?;
    assert_eq!(retrieved, Some(test_value), "DHT should work after cache recovery");
    
    network.stop_all().await?;
    Ok(())
}

/// Test individual node restart and cache-based rejoin
#[tokio::test]
async fn test_individual_node_restart_with_cache() -> Result<()> {
    let mut network = BootstrapTestNetwork::new(4).await?;
    
    // Start network
    network.start_all().await?;
    network.wait_for_convergence().await?;
    
    // Store some test data
    let test_key = Key::new(b"restart_test");
    let test_value = b"restart_value".to_vec();
    
    network.get_node(0).dht_put(test_key.clone(), test_value.clone()).await?;
    sleep(Duration::from_secs(2)).await;
    
    // Restart node 2 (middle node)
    let restart_node_index = 2;
    info!("Restarting node {}", restart_node_index);
    
    network.restart_node(restart_node_index).await?;
    
    // Verify restarted node can rejoin network
    sleep(Duration::from_secs(3)).await;
    
    let peer_count = network.get_node(restart_node_index).peer_count().await;
    assert!(peer_count > 0, "Restarted node should reconnect to network via cache");
    
    // Verify restarted node can participate in DHT
    let retrieved = network.get_node(restart_node_index).dht_get(test_key.clone()).await?;
    assert_eq!(retrieved, Some(test_value), "Restarted node should access DHT data");
    
    network.stop_all().await?;
    Ok(())
}

/// Test multi-instance coordination (simulated)
#[tokio::test]
async fn test_multi_instance_cache_coordination() -> Result<()> {
    use p2p_foundation::bootstrap::{BootstrapCache, MergeCoordinator};
    
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };

    // Create two cache instances (simulating multiple P2P instances)
    let mut cache1 = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config.clone()).await?;
    let mut cache2 = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config.clone()).await?;

    // Add different contacts to each cache
    let contact1 = ContactEntry::new(
        PeerId::from("peer1"), 
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    
    let mut contact2 = ContactEntry::new(
        PeerId::from("peer2"), 
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    
    // Make contact2 higher quality
    contact2.update_connection_result(true, Some(50), None);
    contact2.update_connection_result(true, Some(60), None);
    
    cache1.add_contact(contact1.clone()).await?;
    cache2.add_contact(contact2.clone()).await?;

    // Create merge coordinator and test merging
    let merge_coordinator = MergeCoordinator::new(temp_dir.path().to_path_buf())?;
    
    // Force a merge operation (in real scenario this would be automatic)
    let merge_result = merge_coordinator.merge_instance_caches(&cache1).await?;
    
    info!("Merge result: contacts_merged={}, conflicts_resolved={}", 
          merge_result.contacts_merged, merge_result.conflicts_resolved);

    // Verify merged cache contains contacts from both instances
    let bootstrap_peers = cache1.get_bootstrap_peers(10).await?;
    
    // Should have at least the contacts from both caches
    assert!(!bootstrap_peers.is_empty(), "Merged cache should contain contacts");
    
    // Higher quality contact should be prioritized
    if bootstrap_peers.len() >= 2 {
        assert!(bootstrap_peers[0].quality_metrics.quality_score >= 
                bootstrap_peers[1].quality_metrics.quality_score,
                "Contacts should be sorted by quality score");
    }

    Ok(())
}

/// Test network resilience with node failures and recovery
#[tokio::test]
async fn test_network_resilience_with_cache() -> Result<()> {
    let mut network = BootstrapTestNetwork::new(5).await?;
    
    // Start network
    network.start_all().await?;
    network.wait_for_convergence().await?;
    
    // Store critical data
    let test_data: Vec<(Key, Vec<u8>)> = (0..5)
        .map(|i| {
            let key = Key::new(format!("critical_data_{}", i).as_bytes());
            let value = format!("important_value_{}", i).into_bytes();
            (key, value)
        })
        .collect();

    // Distribute data across network
    for (i, (key, value)) in test_data.iter().enumerate() {
        let node_index = i % network.nodes.len();
        network.get_node(node_index).dht_put(key.clone(), value.clone()).await?;
    }
    
    sleep(Duration::from_secs(3)).await;

    // Simulate cascading failures (stop 2 nodes)
    info!("Simulating node failures");
    network.get_node_mut(1).stop().await?;
    network.get_node_mut(3).stop().await?;
    
    sleep(Duration::from_secs(2)).await;

    // Verify remaining nodes can still access some data
    let mut accessible_data = 0;
    for (key, expected_value) in &test_data {
        // Try to retrieve from remaining nodes (0, 2, 4)
        for &node_idx in &[0, 2, 4] {
            if let Ok(Some(value)) = network.get_node(node_idx).dht_get(key.clone()).await {
                if value == *expected_value {
                    accessible_data += 1;
                    break;
                }
            }
        }
    }

    info!("Accessible data after failures: {}/{}", accessible_data, test_data.len());
    
    // Should maintain some data availability even with node failures
    assert!(accessible_data > 0, "Some data should remain accessible despite node failures");

    // Restart failed nodes - they should rejoin via cache
    info!("Recovering failed nodes");
    network.restart_node(1).await?;
    network.restart_node(3).await?;
    
    sleep(Duration::from_secs(3)).await;

    // Verify network recovery
    let mut recovered_data = 0;
    for (key, expected_value) in &test_data {
        if network.verify_dht_replication(key, expected_value).await? {
            recovered_data += 1;
        }
    }

    info!("Recovered data after node restart: {}/{}", recovered_data, test_data.len());
    
    // Network should recover most or all data after node restart
    let recovery_rate = recovered_data as f64 / test_data.len() as f64;
    assert!(recovery_rate >= 0.6, 
           "Network recovery rate too low: {:.1}% (expected >= 60%)", 
           recovery_rate * 100.0);

    network.stop_all().await?;
    Ok(())
}

/// Test cache quality scoring and peer selection
#[tokio::test]
async fn test_cache_quality_scoring_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 50,
        ..CacheConfig::default()
    };

    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config).await?;

    // Create contacts with varying quality
    let mut contacts = Vec::new();
    
    for i in 0..10 {
        let mut contact = ContactEntry::new(
            PeerId::from(format!("peer_{}", i)),
            vec![format!("/ip4/127.0.0.1/tcp/{}", 9000 + i)]
        );

        // Simulate different connection patterns
        match i % 3 {
            0 => {
                // High quality peer
                for _ in 0..10 {
                    contact.update_connection_result(true, Some(50 + i * 5), None);
                }
                contact.mark_ipv6_verified();
                contact.update_capabilities(vec!["dht".to_string(), "mcp".to_string()]);
            }
            1 => {
                // Medium quality peer
                for _ in 0..5 {
                    contact.update_connection_result(true, Some(100 + i * 10), None);
                }
                contact.update_connection_result(false, None, Some("timeout".to_string()));
            }
            2 => {
                // Low quality peer
                contact.update_connection_result(true, Some(200 + i * 20), None);
                contact.update_connection_result(false, None, Some("connection_refused".to_string()));
                contact.update_connection_result(false, None, Some("timeout".to_string()));
            }
            _ => unreachable!()
        }

        contacts.push(contact);
    }

    // Add contacts to cache
    for contact in contacts {
        cache.add_contact(contact).await?;
    }

    // Get bootstrap peers (should be sorted by quality)
    let bootstrap_peers = cache.get_bootstrap_peers(5).await?;
    
    assert!(!bootstrap_peers.is_empty(), "Should have bootstrap peers");
    info!("Selected {} bootstrap peers", bootstrap_peers.len());

    // Verify quality ordering
    for (i, peer) in bootstrap_peers.iter().enumerate() {
        info!("Peer {}: quality={:.3}, success_rate={:.3}, latency={:.1}ms, verified={}",
              i, peer.quality_metrics.quality_score, peer.quality_metrics.success_rate,
              peer.quality_metrics.avg_latency_ms, peer.ipv6_identity_verified);
        
        if i > 0 {
            assert!(peer.quality_metrics.quality_score <= bootstrap_peers[i-1].quality_metrics.quality_score,
                   "Peers should be sorted by quality score in descending order");
        }
    }

    // High quality peers should be selected first
    assert!(bootstrap_peers[0].quality_metrics.quality_score > 0.5,
           "Best peer should have high quality score");

    Ok(())
}

/// Performance test for bootstrap cache operations
#[tokio::test]
async fn test_bootstrap_cache_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 1000,
        ..CacheConfig::default()
    };

    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config).await?;

    // Measure contact addition performance
    let start_time = std::time::Instant::now();
    
    for i in 0..500 {
        let contact = ContactEntry::new(
            PeerId::from(format!("perf_peer_{}", i)),
            vec![format!("/ip4/192.168.1.{}/tcp/9000", i % 255)]
        );
        cache.add_contact(contact).await?;
    }
    
    let addition_time = start_time.elapsed();
    info!("Added 500 contacts in {:?} ({:.2} contacts/sec)", 
          addition_time, 500.0 / addition_time.as_secs_f64());

    // Measure bootstrap peer selection performance
    let start_time = std::time::Instant::now();
    
    for _ in 0..100 {
        let _peers = cache.get_bootstrap_peers(20).await?;
    }
    
    let selection_time = start_time.elapsed();
    info!("Performed 100 bootstrap selections in {:?} ({:.2} ops/sec)",
          selection_time, 100.0 / selection_time.as_secs_f64());

    // Performance assertions
    assert!(addition_time.as_secs_f64() < 10.0, "Contact addition should be fast");
    assert!(selection_time.as_secs_f64() < 1.0, "Bootstrap selection should be very fast");

    Ok(())
}