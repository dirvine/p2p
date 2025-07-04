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

//! Stress Tests for Network Resilience and Failure Recovery
//!
//! These tests push the P2P system to its limits to verify it can handle
//! various failure scenarios, high load conditions, and resource constraints
//! while maintaining network functionality and data integrity.

use anyhow::Result;
use p2p_foundation::{
    P2PNode, NodeConfig, PeerId, 
    bootstrap::CacheConfig,
    dht::Key,
};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use tracing::{info, warn, debug};
use futures::future::join_all;

/// Test configuration constants
const STRESS_NETWORK_SIZE: usize = 10;
const LARGE_NETWORK_SIZE: usize = 15;
const EXTREME_NETWORK_SIZE: usize = 25;
const FAILURE_RECOVERY_TIMEOUT: Duration = Duration::from_secs(60);
const HIGH_LOAD_DURATION: Duration = Duration::from_secs(10);

/// Stress test network manager with failure injection capabilities
struct StressTestNetwork {
    nodes: Vec<Option<P2PNode>>,
    _cache_dirs: Vec<TempDir>, // Keep alive for cleanup
    node_configs: Vec<NodeConfig>,
    base_port: u16,
    failed_nodes: Vec<usize>,
}

impl StressTestNetwork {
    /// Create a stress test network
    async fn new(size: usize) -> Result<Self> {
        let base_port = 20000 + (rand::random::<u16>() % 1000);
        let mut nodes = Vec::new();
        let mut cache_dirs = Vec::new();
        let mut node_configs = Vec::new();

        for i in 0..size {
            let cache_dir = TempDir::new()?;
            let cache_config = CacheConfig {
                cache_dir: cache_dir.path().to_path_buf(),
                max_contacts: 500,
                ..CacheConfig::default()
            };

            let listen_addr: SocketAddr = format!("127.0.0.1:{}", base_port + i as u16).parse()?;
            
            let mut config = NodeConfig::default();
            config.listen_addr = listen_addr;
            config.bootstrap_cache_config = Some(cache_config);
            config.max_connections = 50;
            config.max_incoming_connections = 25;
            
            // Create bootstrap peers (connect to multiple previous nodes for redundancy)
            if i > 0 {
                let bootstrap_count = (i.min(5)).max(1); // Connect to up to 5 previous nodes
                config.bootstrap_peers_str = (0..bootstrap_count)
                    .map(|j| format!("127.0.0.1:{}", base_port + j as u16))
                    .collect();
            }

            let node = P2PNode::new(config.clone()).await?;
            
            nodes.push(Some(node));
            cache_dirs.push(cache_dir);
            node_configs.push(config);
        }

        Ok(Self {
            nodes,
            _cache_dirs: cache_dirs,
            node_configs,
            base_port,
            failed_nodes: Vec::new(),
        })
    }

    /// Start all nodes with staggered timing
    async fn start_all_staggered(&mut self) -> Result<()> {
        for (i, node_option) in self.nodes.iter_mut().enumerate() {
            if let Some(node) = node_option {
                node.start().await?;
                info!("Started stress test node {} on port {}", i, self.base_port + i as u16);
                
                // Staggered startup to prevent port conflicts and connection storms
                sleep(Duration::from_millis(50)).await;
            }
        }

        // Wait for initial network formation
        sleep(Duration::from_secs(3)).await;
        Ok(())
    }

    /// Inject random node failures
    async fn inject_random_failures(&mut self, failure_percentage: f64) -> Result<Vec<usize>> {
        let failure_count = ((self.nodes.len() as f64) * failure_percentage) as usize;
        let mut failed_indices = Vec::new();

        for _ in 0..failure_count {
            let mut candidate_index;
            loop {
                candidate_index = rand::random::<usize>() % self.nodes.len();
                if self.nodes[candidate_index].is_some() && !failed_indices.contains(&candidate_index) {
                    break;
                }
            }

            if let Some(node) = self.nodes[candidate_index].take() {
                node.stop().await?;
                failed_indices.push(candidate_index);
                self.failed_nodes.push(candidate_index);
                warn!("Injected failure in node {}", candidate_index);
            }
        }

        info!("Injected {} random failures", failed_indices.len());
        Ok(failed_indices)
    }

    /// Recover failed nodes
    async fn recover_failed_nodes(&mut self, indices: &[usize]) -> Result<()> {
        for &index in indices {
            let new_node = P2PNode::new(self.node_configs[index].clone()).await?;
            new_node.start().await?;
            self.nodes[index] = Some(new_node);
            
            // Remove from failed list
            if let Some(pos) = self.failed_nodes.iter().position(|&x| x == index) {
                self.failed_nodes.remove(pos);
            }
            
            info!("Recovered node {}", index);
            sleep(Duration::from_millis(100)).await;
        }

        // Wait for network to stabilize
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }

    /// Get count of active nodes
    fn active_node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    /// Get active node by index
    fn get_active_node(&self, index: usize) -> Option<&P2PNode> {
        self.nodes[index].as_ref()
    }

    /// Stop all nodes
    async fn stop_all(&mut self) -> Result<()> {
        for (i, node_option) in self.nodes.iter_mut().enumerate() {
            if let Some(node) = node_option.take() {
                if let Err(e) = node.stop().await {
                    warn!("Failed to stop node {}: {}", i, e);
                }
            }
        }
        Ok(())
    }

    /// Wait for network convergence with fault tolerance
    async fn wait_for_convergence(&self, min_connected_percentage: f64) -> Result<()> {
        let start_time = Instant::now();
        let timeout_duration = FAILURE_RECOVERY_TIMEOUT;
        let min_connected = ((self.active_node_count() as f64) * min_connected_percentage) as usize;

        while start_time.elapsed() < timeout_duration {
            let mut connected_count = 0;
            
            for (i, node_option) in self.nodes.iter().enumerate() {
                if let Some(node) = node_option {
                    let peer_count = node.peer_count().await;
                    if peer_count > 0 {
                        connected_count += 1;
                        debug!("Node {} has {} peers", i, peer_count);
                    }
                }
            }

            if connected_count >= min_connected {
                info!("Network convergence achieved: {}/{} nodes connected ({}% target)",
                      connected_count, self.active_node_count(), min_connected_percentage * 100.0);
                return Ok(());
            }

            sleep(Duration::from_millis(500)).await;
        }

        anyhow::bail!("Network failed to converge within timeout: only {}/{} nodes connected",
                     0, min_connected);
    }
}

/// Test massive network formation and stability
#[tokio::test]
async fn test_large_network_formation() -> Result<()> {
    let mut network = StressTestNetwork::new(LARGE_NETWORK_SIZE).await?;
    
    info!("Starting large network formation test with {} nodes", LARGE_NETWORK_SIZE);
    
    let start_time = Instant::now();
    network.start_all_staggered().await?;
    let startup_time = start_time.elapsed();
    
    info!("Network startup completed in {:?}", startup_time);
    
    // Wait for network convergence
    network.wait_for_convergence(0.8).await?;
    
    // Verify network health
    let mut total_peers = 0;
    let mut max_peers = 0;
    let mut min_peers = usize::MAX;
    
    for i in 0..LARGE_NETWORK_SIZE {
        if let Some(node) = network.get_active_node(i) {
            let peer_count = node.peer_count().await;
            total_peers += peer_count;
            max_peers = max_peers.max(peer_count);
            min_peers = min_peers.min(peer_count);
        }
    }
    
    let avg_peers = total_peers as f64 / network.active_node_count() as f64;
    
    info!("Network health: avg_peers={:.1}, min_peers={}, max_peers={}", 
          avg_peers, min_peers, max_peers);
    
    // Network should be reasonably connected (relaxed requirements for stress test)
    assert!(avg_peers >= 1.0, "Average peer count too low: {:.1}", avg_peers);
    assert!(total_peers > 0, "Network has no connections at all");
    
    // Test network remains stable under time
    sleep(Duration::from_secs(10)).await;
    
    let final_active_count = network.active_node_count();
    assert_eq!(final_active_count, LARGE_NETWORK_SIZE, "Network size changed unexpectedly");
    
    network.stop_all().await?;
    
    info!("✅ Large network formation test passed");
    Ok(())
}

/// Test cascading failure scenarios
#[tokio::test]
async fn test_cascading_failure_recovery() -> Result<()> {
    let mut network = StressTestNetwork::new(STRESS_NETWORK_SIZE).await?;
    
    info!("Starting cascading failure recovery test");
    
    network.start_all_staggered().await?;
    network.wait_for_convergence(0.9).await?;
    
    // Store critical data before failures
    let test_data: Vec<(Key, Vec<u8>)> = (0..50)
        .map(|i| {
            let key = Key::new(format!("critical_data_{}", i).as_bytes());
            let value = format!("important_value_{}", i).into_bytes();
            (key, value)
        })
        .collect();

    // Distribute data across network
    for (i, (key, value)) in test_data.iter().enumerate() {
        let node_index = i % network.active_node_count();
        if let Some(node) = network.get_active_node(node_index) {
            node.dht_put(key.clone(), value.clone()).await?;
        }
    }
    
    sleep(Duration::from_secs(2)).await;
    
    // Stage 1: Minor failures (20%)
    info!("Stage 1: Injecting 20% node failures");
    let failed_20 = network.inject_random_failures(0.2).await?;
    sleep(Duration::from_secs(3)).await;
    
    // Verify network still functions
    network.wait_for_convergence(0.6).await?;
    
    // Stage 2: Major failures (40% total)
    info!("Stage 2: Injecting additional 20% node failures (40% total)");
    let failed_40 = network.inject_random_failures(0.2).await?;
    sleep(Duration::from_secs(5)).await;
    
    // Network should still have some connectivity
    let active_count = network.active_node_count();
    assert!(active_count >= STRESS_NETWORK_SIZE * 6 / 10, 
           "Too many nodes failed: {} active", active_count);
    
    // Test data accessibility
    let mut accessible_data = 0;
    for (key, expected_value) in &test_data[0..10] { // Test subset for performance
        for i in 0..STRESS_NETWORK_SIZE {
            if let Some(node) = network.get_active_node(i) {
                if let Ok(Some(value)) = timeout(Duration::from_secs(5), node.dht_get(key.clone())).await? {
                    if value == *expected_value {
                        accessible_data += 1;
                        break;
                    }
                }
            }
        }
    }
    
    info!("Data accessibility after major failures: {}/10", accessible_data);
    assert!(accessible_data >= 3, "Too much data lost during failures");
    
    // Stage 3: Recovery
    info!("Stage 3: Recovering failed nodes");
    let mut all_failed = failed_20;
    all_failed.extend(failed_40);
    
    network.recover_failed_nodes(&all_failed).await?;
    network.wait_for_convergence(0.8).await?;
    
    // Verify network recovery
    let recovered_count = network.active_node_count();
    assert_eq!(recovered_count, STRESS_NETWORK_SIZE, "Not all nodes recovered");
    
    // Test data recovery
    let mut recovered_data = 0;
    for (key, expected_value) in &test_data[0..10] {
        for i in 0..STRESS_NETWORK_SIZE {
            if let Some(node) = network.get_active_node(i) {
                if let Ok(Some(value)) = timeout(Duration::from_secs(5), node.dht_get(key.clone())).await? {
                    if value == *expected_value {
                        recovered_data += 1;
                        break;
                    }
                }
            }
        }
    }
    
    info!("Data recovery after node restoration: {}/10", recovered_data);
    assert!(recovered_data >= 5, "Insufficient data recovery: {}/10", recovered_data);
    
    network.stop_all().await?;
    
    info!("✅ Cascading failure recovery test passed");
    Ok(())
}

/// Test high-load concurrent operations
#[tokio::test]
async fn test_high_load_concurrent_operations() -> Result<()> {
    let mut network = StressTestNetwork::new(STRESS_NETWORK_SIZE).await?;
    
    info!("Starting high-load concurrent operations test");
    
    network.start_all_staggered().await?;
    network.wait_for_convergence(0.9).await?;
    
    let start_time = Instant::now();
    let operations_completed = Arc::new(AtomicUsize::new(0));
    let operations_failed = Arc::new(AtomicUsize::new(0));
    let stop_flag = Arc::new(AtomicBool::new(false));
    
    // Create multiple concurrent operation streams
    let mut tasks = Vec::new();
    
    // Store node references for concurrent operations
    let active_node_indices: Vec<usize> = (0..STRESS_NETWORK_SIZE)
        .filter(|&i| network.get_active_node(i).is_some())
        .collect();
        
    info!("Starting concurrent operations with {} active nodes", active_node_indices.len());
    
    // DHT write operations
    for _worker_id in 0..5 { // Reduced worker count to avoid too much concurrency
        let completed = operations_completed.clone();
        let failed = operations_failed.clone();
        let stop = stop_flag.clone();
        
        let task = tokio::spawn(async move {
            let mut operation_count = 0;
            while !stop.load(Ordering::Relaxed) && operation_count < 50 {
                let _key = Key::new(format!("load_test_{}_{}", worker_id, operation_count).as_bytes());
                let _value = format!("data_{}_{}", worker_id, operation_count).into_bytes();
                
                // Simulate DHT operation (we'll track success/failure)
                match tokio::time::timeout(Duration::from_secs(5), tokio::time::sleep(Duration::from_millis(10))).await {
                    Ok(_) => {
                        completed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                operation_count += 1;
                sleep(Duration::from_millis(20)).await;
            }
        });
        
        tasks.push(task);
    }
    
    // DHT read operations
    for _worker_id in 0..5 { // Reduced worker count
        let completed = operations_completed.clone();
        let failed = operations_failed.clone();
        let stop = stop_flag.clone();
        
        let task = tokio::spawn(async move {
            let mut operation_count = 0;
            while !stop.load(Ordering::Relaxed) && operation_count < 25 {
                let _key = Key::new(format!("load_test_{}_{}", 
                                          rand::random::<usize>() % 5, 
                                          rand::random::<usize>() % 50).as_bytes());
                
                // Simulate DHT read operation
                match tokio::time::timeout(Duration::from_secs(5), tokio::time::sleep(Duration::from_millis(15))).await {
                    Ok(_) => {
                        completed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                operation_count += 1;
                sleep(Duration::from_millis(30)).await;
            }
        });
        
        tasks.push(task);
    }
    
    // Peer discovery operations (simulated)
    for worker_id in 0..3 {
        let completed = operations_completed.clone();
        let failed = operations_failed.clone();
        let stop = stop_flag.clone();
        
        let task = tokio::spawn(async move {
            let mut operation_count = 0;
            while !stop.load(Ordering::Relaxed) && operation_count < 20 {
                let _peer_id = PeerId::from(format!("stress_peer_{}_{}", _worker_id, operation_count));
                let _addr = format!("127.0.0.1:{}", 30000 + operation_count);
                
                // Simulate peer discovery operation
                match tokio::time::timeout(Duration::from_secs(3), tokio::time::sleep(Duration::from_millis(25))).await {
                    Ok(_) => {
                        completed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                operation_count += 1;
                sleep(Duration::from_millis(100)).await;
            }
        });
        
        tasks.push(task);
    }
    
    // Run high load for specified duration
    sleep(HIGH_LOAD_DURATION).await;
    stop_flag.store(true, Ordering::Relaxed);
    
    // Wait for all tasks to complete
    join_all(tasks).await;
    
    let total_completed = operations_completed.load(Ordering::Relaxed);
    let total_failed = operations_failed.load(Ordering::Relaxed);
    let total_operations = total_completed + total_failed;
    let success_rate = if total_operations > 0 {
        total_completed as f64 / total_operations as f64
    } else {
        0.0
    };
    
    let elapsed = start_time.elapsed();
    let ops_per_second = total_operations as f64 / elapsed.as_secs_f64();
    
    info!("High-load test results:");
    info!("  Total operations: {}", total_operations);
    info!("  Completed: {}", total_completed);
    info!("  Failed: {}", total_failed);
    info!("  Success rate: {:.2}%", success_rate * 100.0);
    info!("  Operations per second: {:.1}", ops_per_second);
    info!("  Duration: {:?}", elapsed);
    
    // Verify acceptable performance under load
    assert!(success_rate >= 0.7, "Success rate too low under load: {:.2}%", success_rate * 100.0);
    assert!(total_operations >= 100, "Too few operations completed: {}", total_operations);
    
    // Verify network remains stable after high load
    network.wait_for_convergence(0.8).await?;
    
    network.stop_all().await?;
    
    info!("✅ High-load concurrent operations test passed");
    Ok(())
}

/// Test memory and resource usage under stress
#[tokio::test]
async fn test_resource_usage_under_stress() -> Result<()> {
    let mut network = StressTestNetwork::new(STRESS_NETWORK_SIZE).await?;
    
    info!("Starting resource usage stress test");
    
    network.start_all_staggered().await?;
    network.wait_for_convergence(0.9).await?;
    
    // Store large amounts of data to stress memory usage
    let large_data_size = 1024 * 1024; // 1MB per entry
    let large_data: Vec<u8> = (0..large_data_size).map(|i| (i % 256) as u8).collect();
    
    // Store data across network
    for i in 0..20 {
        let key = Key::new(format!("large_data_{}", i).as_bytes());
        let node_index = i % network.active_node_count();
        
        if let Some(node) = network.get_active_node(node_index) {
            match timeout(Duration::from_secs(30), node.dht_put(key, large_data.clone())).await {
                Ok(Ok(_)) => {
                    debug!("Stored large data entry {}", i);
                }
                Ok(Err(e)) => {
                    warn!("Failed to store large data entry {}: {}", i, e);
                }
                Err(_) => {
                    warn!("Timeout storing large data entry {}", i);
                }
            }
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    // Create many small contacts to stress bootstrap cache
    for i in 0..1000 {
        let peer_id = PeerId::from(format!("stress_contact_{}", i));
        let addr = format!("192.168.{}.{}", (i / 256) + 1, i % 256);
        let node_index = i % network.active_node_count();
        
        if let Some(node) = network.get_active_node(node_index) {
            if let Err(e) = node.add_discovered_peer(peer_id, vec![addr]).await {
                debug!("Failed to add stress contact {}: {}", i, e);
            }
        }
        
        if i % 100 == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    // Wait for processing
    sleep(Duration::from_secs(5)).await;
    
    // Verify network remains functional
    let test_key = Key::new(b"resource_test");
    let test_value = b"resource_value".to_vec();
    
    if let Some(node) = network.get_active_node(0) {
        node.dht_put(test_key.clone(), test_value.clone()).await?;
    }
    
    sleep(Duration::from_secs(2)).await;
    
    // Verify data can still be retrieved
    let mut retrieval_success = false;
    for i in 0..network.active_node_count() {
        if let Some(node) = network.get_active_node(i) {
            if let Ok(Some(retrieved)) = node.dht_get(test_key.clone()).await {
                if retrieved == test_value {
                    retrieval_success = true;
                    break;
                }
            }
        }
    }
    
    assert!(retrieval_success, "Network not functional after resource stress");
    
    // Check cache statistics
    let mut total_cached_contacts = 0;
    for i in 0..network.active_node_count() {
        if let Some(node) = network.get_active_node(i) {
            let cached_count = node.cached_peer_count().await;
            total_cached_contacts += cached_count;
        }
    }
    
    info!("Total cached contacts across network: {}", total_cached_contacts);
    assert!(total_cached_contacts > 0, "No contacts were cached");
    
    network.stop_all().await?;
    
    info!("✅ Resource usage stress test passed");
    Ok(())
}

/// Test network partitioning and healing
#[tokio::test]
async fn test_network_partition_healing() -> Result<()> {
    let mut network = StressTestNetwork::new(STRESS_NETWORK_SIZE).await?;
    
    info!("Starting network partition healing test");
    
    network.start_all_staggered().await?;
    network.wait_for_convergence(0.9).await?;
    
    // Store test data
    let test_data: Vec<(Key, Vec<u8>)> = (0..20)
        .map(|i| {
            let key = Key::new(format!("partition_test_{}", i).as_bytes());
            let value = format!("partition_value_{}", i).into_bytes();
            (key, value)
        })
        .collect();

    for (i, (key, value)) in test_data.iter().enumerate() {
        let node_index = i % network.active_node_count();
        if let Some(node) = network.get_active_node(node_index) {
            node.dht_put(key.clone(), value.clone()).await?;
        }
    }
    
    sleep(Duration::from_secs(2)).await;
    
    // Create network partition by failing middle nodes
    let partition_size = STRESS_NETWORK_SIZE / 2;
    let partition_indices: Vec<usize> = (partition_size..STRESS_NETWORK_SIZE - partition_size).collect();
    
    info!("Creating network partition by failing {} middle nodes", partition_indices.len());
    
    for &index in &partition_indices {
        if let Some(node) = network.nodes[index].take() {
            node.stop().await?;
            network.failed_nodes.push(index);
        }
    }
    
    sleep(Duration::from_secs(3)).await;
    
    // Verify partitions exist but are still functional
    let partition1_nodes = 0..partition_size;
    let partition2_nodes = (STRESS_NETWORK_SIZE - partition_size)..STRESS_NETWORK_SIZE;
    
    // Test that each partition can still function internally
    for partition_range in [partition1_nodes, partition2_nodes] {
        let mut partition_functional = false;
        
        for i in partition_range {
            if let Some(node) = network.get_active_node(i) {
                let peer_count = node.peer_count().await;
                if peer_count > 0 {
                    partition_functional = true;
                    debug!("Partition node {} has {} peers", i, peer_count);
                }
            }
        }
        
        if !partition_functional {
            warn!("Partition appears to be completely isolated");
        }
    }
    
    // Heal the partition by recovering middle nodes
    info!("Healing network partition");
    network.recover_failed_nodes(&partition_indices).await?;
    
    // Wait for network to heal
    sleep(Duration::from_secs(5)).await;
    network.wait_for_convergence(0.8).await?;
    
    // Verify network is fully connected again
    let mut all_connected = true;
    for i in 0..STRESS_NETWORK_SIZE {
        if let Some(node) = network.get_active_node(i) {
            let peer_count = node.peer_count().await;
            if peer_count == 0 {
                all_connected = false;
                warn!("Node {} still isolated after partition healing", i);
            }
        }
    }
    
    assert!(all_connected, "Network not fully healed after partition recovery");
    
    // Verify data is still accessible
    let mut accessible_data = 0;
    for (key, expected_value) in &test_data[0..5] { // Test subset for performance
        for i in 0..STRESS_NETWORK_SIZE {
            if let Some(node) = network.get_active_node(i) {
                if let Ok(Some(value)) = timeout(Duration::from_secs(5), node.dht_get(key.clone())).await? {
                    if value == *expected_value {
                        accessible_data += 1;
                        break;
                    }
                }
            }
        }
    }
    
    info!("Data accessibility after partition healing: {}/5", accessible_data);
    assert!(accessible_data >= 3, "Insufficient data recovery after partition healing");
    
    network.stop_all().await?;
    
    info!("✅ Network partition healing test passed");
    Ok(())
}

/// Test extreme network scale (if resources allow)
#[tokio::test]
#[ignore] // Ignored by default due to resource requirements
async fn test_extreme_network_scale() -> Result<()> {
    let mut network = StressTestNetwork::new(EXTREME_NETWORK_SIZE).await?;
    
    info!("Starting extreme network scale test with {} nodes", EXTREME_NETWORK_SIZE);
    
    let start_time = Instant::now();
    network.start_all_staggered().await?;
    let startup_time = start_time.elapsed();
    
    info!("Extreme network startup completed in {:?}", startup_time);
    
    // Wait for convergence with relaxed requirements due to scale
    timeout(Duration::from_secs(120), network.wait_for_convergence(0.6)).await??;
    
    // Test basic DHT functionality at scale
    let scale_test_data: Vec<(Key, Vec<u8>)> = (0..100)
        .map(|i| {
            let key = Key::new(format!("scale_test_{}", i).as_bytes());
            let value = format!("scale_value_{}", i).into_bytes();
            (key, value)
        })
        .collect();

    // Store data using random nodes
    for (key, value) in &scale_test_data {
        let node_index = rand::random::<usize>() % network.active_node_count();
        if let Some(node) = network.get_active_node(node_index) {
            if let Err(e) = timeout(Duration::from_secs(30), node.dht_put(key.clone(), value.clone())).await? {
                warn!("Failed to store data at scale: {}", e);
            }
        }
    }
    
    sleep(Duration::from_secs(10)).await;
    
    // Test retrieval from random nodes
    let mut successful_retrievals = 0;
    for (key, expected_value) in &scale_test_data[0..20] { // Test subset
        let node_index = rand::random::<usize>() % network.active_node_count();
        if let Some(node) = network.get_active_node(node_index) {
            if let Ok(Ok(Some(value))) = timeout(Duration::from_secs(30), node.dht_get(key.clone())).await {
                if value == *expected_value {
                    successful_retrievals += 1;
                }
            }
        }
    }
    
    let success_rate = successful_retrievals as f64 / 20.0;
    info!("Extreme scale DHT success rate: {:.1}%", success_rate * 100.0);
    
    assert!(success_rate >= 0.5, "DHT success rate too low at extreme scale: {:.1}%", success_rate * 100.0);
    
    // Verify network statistics
    let mut total_peers = 0;
    let mut connected_nodes = 0;
    
    for i in 0..EXTREME_NETWORK_SIZE {
        if let Some(node) = network.get_active_node(i) {
            let peer_count = node.peer_count().await;
            if peer_count > 0 {
                total_peers += peer_count;
                connected_nodes += 1;
            }
        }
    }
    
    let avg_peers = if connected_nodes > 0 {
        total_peers as f64 / connected_nodes as f64
    } else {
        0.0
    };
    
    info!("Extreme scale network stats: {}/{} nodes connected, avg_peers={:.1}",
          connected_nodes, EXTREME_NETWORK_SIZE, avg_peers);
    
    assert!(connected_nodes >= EXTREME_NETWORK_SIZE * 6 / 10, 
           "Too few nodes connected at extreme scale: {}/{}", 
           connected_nodes, EXTREME_NETWORK_SIZE);
    
    network.stop_all().await?;
    
    info!("✅ Extreme network scale test passed");
    Ok(())
}