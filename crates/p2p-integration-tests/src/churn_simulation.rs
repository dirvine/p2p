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

//! Churn simulation tests for the adaptive P2P network
//!
//! Tests network resilience under various churn scenarios:
//! - Random node failures
//! - Correlated failures
//! - Flash crowds
//! - Network partitions

use p2p_integration_tests::*;
use saorsa_core::adaptive::*;
use anyhow::Result;
use std::{
    time::{Duration, Instant},
    collections::HashSet,
};
use tracing::{info, warn, debug};
use tokio::time::interval;
use rand::Rng;

/// Churn pattern types
#[derive(Debug, Clone, Copy)]
pub enum ChurnPattern {
    /// Random failures with exponential distribution
    Random { rate: f64 },
    
    /// Correlated failures (e.g., datacenter outage)
    Correlated { probability: f64, cluster_size: usize },
    
    /// Flash crowd (mass join/leave)
    FlashCrowd { join_rate: f64, leave_rate: f64 },
    
    /// Network partition
    Partition { partition_ratio: f64 },
    
    /// Diurnal pattern (daily cycles)
    Diurnal { peak_hours: (u8, u8), min_nodes: f64 },
}

/// Churn simulator
pub struct ChurnSimulator {
    /// Churn pattern
    pattern: ChurnPattern,
    
    /// Test cluster
    cluster: TestCluster,
    
    /// Failed nodes
    failed_nodes: Arc<RwLock<HashSet<String>>>,
    
    /// Churn events log
    events: Arc<RwLock<Vec<ChurnEvent>>>,
}

/// Churn event record
#[derive(Debug, Clone)]
pub struct ChurnEvent {
    pub timestamp: Instant,
    pub event_type: ChurnEventType,
    pub node_id: String,
    pub details: String,
}

#[derive(Debug, Clone)]
pub enum ChurnEventType {
    NodeFailed,
    NodeRecovered,
    PartitionCreated,
    PartitionHealed,
}

impl ChurnSimulator {
    /// Create a new churn simulator
    pub async fn new(cluster: TestCluster, pattern: ChurnPattern) -> Self {
        Self {
            pattern,
            cluster,
            failed_nodes: Arc::new(RwLock::new(HashSet::new())),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Start churn simulation
    pub async fn start(&self, duration: Duration) -> Result<()> {
        info!("Starting churn simulation with pattern {:?} for {:?}", self.pattern, duration);
        
        let start_time = Instant::now();
        let mut ticker = interval(Duration::from_secs(1));
        
        while start_time.elapsed() < duration {
            ticker.tick().await;
            
            match self.pattern {
                ChurnPattern::Random { rate } => {
                    self.simulate_random_churn(rate).await?;
                }
                ChurnPattern::Correlated { probability, cluster_size } => {
                    self.simulate_correlated_failures(probability, cluster_size).await?;
                }
                ChurnPattern::FlashCrowd { join_rate, leave_rate } => {
                    self.simulate_flash_crowd(join_rate, leave_rate).await?;
                }
                ChurnPattern::Partition { partition_ratio } => {
                    if start_time.elapsed().as_secs() % 60 == 30 {
                        self.simulate_partition(partition_ratio).await?;
                    }
                }
                ChurnPattern::Diurnal { peak_hours, min_nodes } => {
                    self.simulate_diurnal_pattern(peak_hours, min_nodes).await?;
                }
            }
            
            // Periodic health check
            if start_time.elapsed().as_secs() % 10 == 0 {
                self.check_network_health().await?;
            }
        }
        
        Ok(())
    }
    
    /// Simulate random node failures
    async fn simulate_random_churn(&self, rate: f64) -> Result<()> {
        let nodes = self.cluster.nodes.read().await;
        let active_nodes: Vec<String> = nodes.keys()
            .filter(|id| !self.failed_nodes.read().await.contains(*id))
            .cloned()
            .collect();
        
        if active_nodes.is_empty() {
            return Ok(());
        }
        
        // Poisson process for failures
        let failure_prob = 1.0 - (-rate / 3600.0).exp(); // Convert hourly rate to per-second
        
        for node_id in &active_nodes {
            if rand::random::<f64>() < failure_prob {
                self.fail_node(node_id).await?;
            }
        }
        
        // Recovery process
        let failed = self.failed_nodes.read().await.clone();
        for node_id in failed {
            if rand::random::<f64>() < 0.1 { // 10% recovery chance per second
                self.recover_node(&node_id).await?;
            }
        }
        
        Ok(())
    }
    
    /// Simulate correlated failures
    async fn simulate_correlated_failures(&self, probability: f64, cluster_size: usize) -> Result<()> {
        if rand::random::<f64>() > probability {
            return Ok(());
        }
        
        let nodes = self.cluster.nodes.read().await;
        let active_nodes: Vec<String> = nodes.keys()
            .filter(|id| !self.failed_nodes.read().await.contains(*id))
            .cloned()
            .collect();
        
        if active_nodes.len() < cluster_size {
            return Ok(());
        }
        
        // Select random cluster of nodes to fail
        let mut rng = rand::thread_rng();
        let start_idx = rng.gen_range(0..active_nodes.len() - cluster_size);
        
        info!("Correlated failure affecting {} nodes", cluster_size);
        
        for i in 0..cluster_size {
            let node_id = &active_nodes[start_idx + i];
            self.fail_node(node_id).await?;
        }
        
        // Record partition event
        self.events.write().await.push(ChurnEvent {
            timestamp: Instant::now(),
            event_type: ChurnEventType::PartitionCreated,
            node_id: format!("cluster_{}", start_idx),
            details: format!("Correlated failure of {} nodes", cluster_size),
        });
        
        Ok(())
    }
    
    /// Simulate flash crowd behavior
    async fn simulate_flash_crowd(&self, join_rate: f64, leave_rate: f64) -> Result<()> {
        let current_hour = (Instant::now().elapsed().as_secs() / 3600) % 24;
        
        // Peak hours (evening)
        let is_peak = current_hour >= 18 && current_hour <= 22;
        
        if is_peak {
            // More joins during peak
            if rand::random::<f64>() < join_rate {
                // In real implementation, would add new nodes
                debug!("Flash crowd: would add new node");
            }
        } else {
            // More leaves during off-peak
            let nodes = self.cluster.nodes.read().await;
            let active_nodes: Vec<String> = nodes.keys()
                .filter(|id| !self.failed_nodes.read().await.contains(*id))
                .cloned()
                .collect();
            
            if !active_nodes.is_empty() && rand::random::<f64>() < leave_rate {
                let node_id = &active_nodes[rand::random::<usize>() % active_nodes.len()];
                self.fail_node(node_id).await?;
            }
        }
        
        Ok(())
    }
    
    /// Simulate network partition
    async fn simulate_partition(&self, partition_ratio: f64) -> Result<()> {
        let nodes = self.cluster.nodes.read().await;
        let all_nodes: Vec<String> = nodes.keys().cloned().collect();
        
        let partition_size = (all_nodes.len() as f64 * partition_ratio) as usize;
        
        info!("Creating network partition affecting {} nodes", partition_size);
        
        // Partition nodes into two groups
        let mut rng = rand::thread_rng();
        let mut partition_a = HashSet::new();
        let mut partition_b = HashSet::new();
        
        for (i, node_id) in all_nodes.iter().enumerate() {
            if i < partition_size {
                partition_a.insert(node_id.clone());
            } else {
                partition_b.insert(node_id.clone());
            }
        }
        
        // Update peer connections to simulate partition
        for node_id in &all_nodes {
            if let Some(node) = nodes.get(node_id) {
                let mut state = node.state.write().await;
                
                // Filter peers based on partition
                if partition_a.contains(node_id) {
                    state.peers.retain(|peer| partition_a.contains(peer));
                } else {
                    state.peers.retain(|peer| partition_b.contains(peer));
                }
            }
        }
        
        self.events.write().await.push(ChurnEvent {
            timestamp: Instant::now(),
            event_type: ChurnEventType::PartitionCreated,
            node_id: "network".to_string(),
            details: format!("Network partitioned: {} vs {} nodes", 
                partition_a.len(), partition_b.len()),
        });
        
        // Schedule partition healing after 30 seconds
        let cluster = self.cluster.clone();
        let events = self.events.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            
            // Heal partition
            let nodes = cluster.nodes.read().await;
            for node in nodes.values() {
                // Restore original topology
                // In real implementation, would restore based on topology type
                let mut state = node.state.write().await;
                if state.peers.len() < 3 {
                    // Add random peers
                    for other_id in nodes.keys().take(5) {
                        if other_id != &node.id && !state.peers.contains(other_id) {
                            state.peers.push(other_id.clone());
                        }
                    }
                }
            }
            
            events.write().await.push(ChurnEvent {
                timestamp: Instant::now(),
                event_type: ChurnEventType::PartitionHealed,
                node_id: "network".to_string(),
                details: "Network partition healed".to_string(),
            });
            
            info!("Network partition healed");
        });
        
        Ok(())
    }
    
    /// Simulate diurnal pattern
    async fn simulate_diurnal_pattern(&self, peak_hours: (u8, u8), min_nodes: f64) -> Result<()> {
        let current_hour = (Instant::now().elapsed().as_secs() / 60) % 24; // Minutes as hours for testing
        
        let nodes = self.cluster.nodes.read().await;
        let total_nodes = nodes.len();
        let active_nodes = total_nodes - self.failed_nodes.read().await.len();
        
        let is_peak = current_hour >= peak_hours.0 as u64 && current_hour <= peak_hours.1 as u64;
        let target_active = if is_peak {
            total_nodes
        } else {
            (total_nodes as f64 * min_nodes) as usize
        };
        
        if active_nodes > target_active {
            // Fail some nodes
            let to_fail = active_nodes - target_active;
            let active: Vec<String> = nodes.keys()
                .filter(|id| !self.failed_nodes.read().await.contains(*id))
                .cloned()
                .collect();
            
            for i in 0..to_fail.min(active.len()) {
                self.fail_node(&active[i]).await?;
            }
        } else if active_nodes < target_active {
            // Recover some nodes
            let to_recover = target_active - active_nodes;
            let failed = self.failed_nodes.read().await.clone();
            
            for (i, node_id) in failed.iter().enumerate() {
                if i >= to_recover {
                    break;
                }
                self.recover_node(node_id).await?;
            }
        }
        
        Ok(())
    }
    
    /// Fail a specific node
    async fn fail_node(&self, node_id: &str) -> Result<()> {
        let nodes = self.cluster.nodes.read().await;
        
        if let Some(node) = nodes.get(node_id) {
            // Mark node as failed
            node.state.write().await.running = false;
            self.failed_nodes.write().await.insert(node_id.to_string());
            
            // Stop node services
            node.components.monitoring.stop().await;
            node.components.churn.stop_monitoring().await;
            
            // Remove from other nodes' peer lists
            for other_node in nodes.values() {
                if other_node.id != node_id {
                    other_node.state.write().await.peers.retain(|p| p != node_id);
                    
                    // Notify trust system
                    other_node.components.trust.remove_node(&node.identity.node_id).await;
                }
            }
            
            // Record event
            self.events.write().await.push(ChurnEvent {
                timestamp: Instant::now(),
                event_type: ChurnEventType::NodeFailed,
                node_id: node_id.to_string(),
                details: "Node failed".to_string(),
            });
            
            debug!("Node {} failed", node_id);
        }
        
        Ok(())
    }
    
    /// Recover a failed node
    async fn recover_node(&self, node_id: &str) -> Result<()> {
        let nodes = self.cluster.nodes.read().await;
        
        if let Some(node) = nodes.get(node_id) {
            // Mark node as running
            node.state.write().await.running = true;
            self.failed_nodes.write().await.remove(node_id);
            
            // Restart node services
            node.components.monitoring.start().await;
            node.components.churn.start_monitoring().await;
            
            // Reconnect to bootstrap nodes
            let bootstrap_nodes = &self.cluster.bootstrap_nodes;
            node.state.write().await.peers = bootstrap_nodes.clone();
            
            // Record event
            self.events.write().await.push(ChurnEvent {
                timestamp: Instant::now(),
                event_type: ChurnEventType::NodeRecovered,
                node_id: node_id.to_string(),
                details: "Node recovered".to_string(),
            });
            
            debug!("Node {} recovered", node_id);
        }
        
        Ok(())
    }
    
    /// Check network health during churn
    async fn check_network_health(&self) -> Result<()> {
        let nodes = self.cluster.nodes.read().await;
        let failed_count = self.failed_nodes.read().await.len();
        let active_count = nodes.len() - failed_count;
        
        if active_count == 0 {
            warn!("All nodes have failed!");
            return Ok(());
        }
        
        let mut health_scores = vec![];
        let mut connected_components = 0;
        
        for node in nodes.values() {
            if node.state.read().await.running {
                let health = node.components.monitoring.get_health().await;
                health_scores.push(health.score);
                
                if !node.state.read().await.peers.is_empty() {
                    connected_components += 1;
                }
            }
        }
        
        let avg_health = if health_scores.is_empty() {
            0.0
        } else {
            health_scores.iter().sum::<f64>() / health_scores.len() as f64
        };
        
        let connectivity = connected_components as f64 / active_count as f64;
        
        info!("Network health: active={}/{}, avg_health={:.3}, connectivity={:.3}",
            active_count, nodes.len(), avg_health, connectivity);
        
        if connectivity < 0.5 {
            warn!("Network connectivity below 50%!");
        }
        
        Ok(())
    }
    
    /// Get churn statistics
    pub async fn get_stats(&self) -> ChurnStats {
        let events = self.events.read().await;
        let failed_nodes = self.failed_nodes.read().await.len();
        let total_nodes = self.cluster.nodes.read().await.len();
        
        let mut failures = 0;
        let mut recoveries = 0;
        let mut partitions = 0;
        
        for event in events.iter() {
            match event.event_type {
                ChurnEventType::NodeFailed => failures += 1,
                ChurnEventType::NodeRecovered => recoveries += 1,
                ChurnEventType::PartitionCreated => partitions += 1,
                _ => {}
            }
        }
        
        ChurnStats {
            total_failures: failures,
            total_recoveries: recoveries,
            current_failed: failed_nodes,
            total_nodes,
            partition_events: partitions,
            availability: (total_nodes - failed_nodes) as f64 / total_nodes as f64,
        }
    }
}

/// Churn statistics
#[derive(Debug, Clone)]
pub struct ChurnStats {
    pub total_failures: usize,
    pub total_recoveries: usize,
    pub current_failed: usize,
    pub total_nodes: usize,
    pub partition_events: usize,
    pub availability: f64,
}

#[tokio::test]
async fn test_random_churn_resilience() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting random churn resilience test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 50,
        bootstrap_count: 5,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Store test content before churn
    let nodes = cluster.nodes.read().await;
    let test_node = nodes.values().next().unwrap();
    
    let test_content = utils::generate_content(100 * 1024); // 100KB
    let metadata = storage::ContentMetadata {
        size: test_content.len(),
        content_type: ContentType::DataRetrieval,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        chunk_count: None,
        replication_factor: 12, // Higher replication for churn resilience
    };
    
    let content_hash = test_node.components.storage.store(test_content.clone(), metadata).await?;
    info!("Stored test content with hash {:?}", content_hash);
    
    // Wait for replication
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Start churn simulation
    let simulator = ChurnSimulator::new(
        cluster, 
        ChurnPattern::Random { rate: 10.0 } // 10 failures per hour
    ).await;
    
    // Run churn for 2 minutes (simulated hours)
    let churn_handle = tokio::spawn({
        let simulator = simulator.clone();
        async move {
            simulator.start(Duration::from_secs(120)).await
        }
    });
    
    // Periodically test content availability during churn
    let mut availability_checks = 0;
    let mut successful_retrievals = 0;
    
    for i in 0..12 {
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        let nodes = simulator.cluster.nodes.read().await;
        let active_nodes: Vec<_> = nodes.values()
            .filter(|n| n.state.read().await.running)
            .collect();
        
        if active_nodes.is_empty() {
            warn!("No active nodes for retrieval test");
            continue;
        }
        
        // Try to retrieve from random active node
        let test_node = active_nodes[rand::random::<usize>() % active_nodes.len()];
        
        let retrieval_manager = RetrievalManager::new(
            test_node.components.router.clone(),
            test_node.components.storage.clone(),
            Arc::new(learning::QLearnCacheManager::new(100 * 1024 * 1024)),
        );
        
        availability_checks += 1;
        match retrieval_manager.retrieve(&content_hash, retrieval::RetrievalStrategy::Parallel).await {
            Ok(retrieved) => {
                if retrieved == test_content {
                    successful_retrievals += 1;
                    info!("Content successfully retrieved during churn (check {}/{})", i+1, 12);
                } else {
                    warn!("Retrieved content does not match original");
                }
            }
            Err(e) => {
                warn!("Failed to retrieve content during churn: {}", e);
            }
        }
    }
    
    // Wait for churn to complete
    churn_handle.await??;
    
    // Get final stats
    let churn_stats = simulator.get_stats().await;
    let cluster_stats = simulator.cluster.get_stats().await;
    
    info!("Churn simulation complete:");
    info!("  Total failures: {}", churn_stats.total_failures);
    info!("  Total recoveries: {}", churn_stats.total_recoveries);
    info!("  Final availability: {:.2}%", churn_stats.availability * 100.0);
    info!("  Content availability: {}/{} ({:.2}%)", 
        successful_retrievals, availability_checks,
        (successful_retrievals as f64 / availability_checks as f64) * 100.0);
    
    // Assert content remained available despite churn
    assert!(successful_retrievals as f64 / availability_checks as f64 > 0.8,
        "Content should remain available in at least 80% of checks during churn");
    
    simulator.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_correlated_failures() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting correlated failures test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 60,
        bootstrap_count: 6,
        topology: NetworkTopology::Hierarchical,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Create simulator for correlated failures
    let simulator = ChurnSimulator::new(
        cluster,
        ChurnPattern::Correlated { 
            probability: 0.05, // 5% chance per second
            cluster_size: 10   // Fail 10 nodes at once
        }
    ).await;
    
    // Run simulation
    let churn_handle = tokio::spawn({
        let simulator = simulator.clone();
        async move {
            simulator.start(Duration::from_secs(60)).await
        }
    });
    
    // Monitor network partitions
    let mut partition_detected = false;
    for _ in 0..6 {
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        let stats = simulator.get_stats().await;
        if stats.partition_events > 0 {
            partition_detected = true;
            info!("Detected {} partition events", stats.partition_events);
        }
        
        // Check if network can still function
        let nodes = simulator.cluster.nodes.read().await;
        let active_count = nodes.values()
            .filter(|n| n.state.read().await.running)
            .count();
        
        info!("Active nodes: {}/{}", active_count, nodes.len());
        assert!(active_count >= nodes.len() / 3, 
            "At least 1/3 of nodes should remain active");
    }
    
    churn_handle.await??;
    
    assert!(partition_detected, "Should have detected correlated failures");
    
    simulator.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_network_partition_recovery() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting network partition recovery test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 40,
        bootstrap_count: 4,
        topology: NetworkTopology::Mesh,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Store content before partition
    let nodes = cluster.nodes.read().await;
    let node_ids: Vec<_> = nodes.keys().cloned().collect();
    
    // Store content in both future partitions
    let content1 = b"Partition A content".to_vec();
    let content2 = b"Partition B content".to_vec();
    
    let node_a = nodes.get(&node_ids[0]).unwrap();
    let node_b = nodes.get(&node_ids[nodes.len() - 1]).unwrap();
    
    let metadata = storage::ContentMetadata {
        size: 20,
        content_type: ContentType::DataRetrieval,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        chunk_count: None,
        replication_factor: 8,
    };
    
    let hash1 = node_a.components.storage.store(content1.clone(), metadata.clone()).await?;
    let hash2 = node_b.components.storage.store(content2.clone(), metadata).await?;
    
    drop(nodes);
    
    // Create partition simulator
    let simulator = ChurnSimulator::new(
        cluster,
        ChurnPattern::Partition { partition_ratio: 0.5 }
    ).await;
    
    // Trigger partition
    simulator.simulate_partition(0.5).await?;
    
    // Wait during partition
    info!("Network partitioned, waiting 20 seconds...");
    tokio::time::sleep(Duration::from_secs(20)).await;
    
    // Test that each partition can still function independently
    let nodes = simulator.cluster.nodes.read().await;
    let partition_a_nodes: Vec<_> = nodes.values()
        .take(nodes.len() / 2)
        .collect();
    let partition_b_nodes: Vec<_> = nodes.values()
        .skip(nodes.len() / 2)
        .collect();
    
    // Each partition should maintain connectivity
    for node in &partition_a_nodes {
        let peers = &node.state.read().await.peers;
        assert!(!peers.is_empty(), "Nodes in partition A should have peers");
    }
    
    for node in &partition_b_nodes {
        let peers = &node.state.read().await.peers;
        assert!(!peers.is_empty(), "Nodes in partition B should have peers");
    }
    
    // Wait for partition to heal (automatic after 30s)
    info!("Waiting for partition to heal...");
    tokio::time::sleep(Duration::from_secs(15)).await;
    
    // Verify partition healed
    let events = simulator.events.read().await;
    let heal_events: Vec<_> = events.iter()
        .filter(|e| matches!(e.event_type, ChurnEventType::PartitionHealed))
        .collect();
    
    assert!(!heal_events.is_empty(), "Partition should have healed");
    
    // Test cross-partition content retrieval after healing
    info!("Testing content retrieval after partition healing");
    
    // Node from partition A retrieving content from partition B
    let retrieval_manager_a = RetrievalManager::new(
        partition_a_nodes[0].components.router.clone(),
        partition_a_nodes[0].components.storage.clone(),
        Arc::new(learning::QLearnCacheManager::new(100 * 1024 * 1024)),
    );
    
    let result = retrieval_manager_a.retrieve(&hash2, retrieval::RetrievalStrategy::Parallel).await;
    assert!(result.is_ok(), "Should retrieve content from healed partition");
    
    simulator.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_diurnal_pattern() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting diurnal pattern test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 100,
        bootstrap_count: 10,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Create simulator with diurnal pattern
    let simulator = ChurnSimulator::new(
        cluster,
        ChurnPattern::Diurnal {
            peak_hours: (8, 20), // 8am to 8pm
            min_nodes: 0.3,      // 30% active during off-peak
        }
    ).await;
    
    // Track node availability over time
    let mut availability_samples = vec![];
    
    // Simulate 24 hours (24 minutes in test time)
    let simulation = tokio::spawn({
        let simulator = simulator.clone();
        async move {
            for hour in 0..24 {
                // Set the "hour" for testing
                for _ in 0..60 {
                    simulator.simulate_diurnal_pattern((8, 20), 0.3).await.ok();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                
                let stats = simulator.get_stats().await;
                availability_samples.push((hour, stats.availability));
                
                info!("Hour {}: {:.1}% nodes active", hour, stats.availability * 100.0);
            }
        }
    });
    
    simulation.await?;
    
    // Verify diurnal pattern
    let peak_availability = availability_samples.iter()
        .filter(|(h, _)| *h >= 8 && *h <= 20)
        .map(|(_, a)| a)
        .sum::<f64>() / 13.0;
    
    let offpeak_availability = availability_samples.iter()
        .filter(|(h, _)| *h < 8 || *h > 20)
        .map(|(_, a)| a)
        .sum::<f64>() / 11.0;
    
    info!("Peak availability: {:.2}%", peak_availability * 100.0);
    info!("Off-peak availability: {:.2}%", offpeak_availability * 100.0);
    
    assert!(peak_availability > offpeak_availability,
        "Peak hours should have higher availability");
    assert!(offpeak_availability >= 0.25 && offpeak_availability <= 0.35,
        "Off-peak availability should be around 30%");
    
    simulator.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Long-running test
async fn test_extreme_churn() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting extreme churn test");
    
    // Create resilient cluster configuration
    let config = TestClusterConfig {
        node_count: 200,
        bootstrap_count: 20,
        topology: NetworkTopology::Random,
        conditions: NetworkConditions {
            packet_loss: 0.05,    // 5% packet loss
            latency_ms: 50,       // 50ms latency
            jitter_ms: 10,        // 10ms jitter
            bandwidth_mbps: 10,   // 10 Mbps
            failure_rate: 50.0,   // 50 failures/hour
        },
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(60)).await?;
    
    // Apply network conditions
    cluster.apply_network_conditions().await?;
    
    // Store critical content with high replication
    let nodes = cluster.nodes.read().await;
    let bootstrap_node = nodes.get(&cluster.bootstrap_nodes[0]).unwrap();
    
    let critical_content = b"Critical data that must survive extreme churn".to_vec();
    let metadata = storage::ContentMetadata {
        size: critical_content.len(),
        content_type: ContentType::DataRetrieval,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        chunk_count: None,
        replication_factor: 20, // Maximum replication
    };
    
    let critical_hash = bootstrap_node.components.storage
        .store(critical_content.clone(), metadata).await?;
    
    drop(nodes);
    
    // Run extreme churn simulation
    let simulator = ChurnSimulator::new(
        cluster,
        ChurnPattern::Random { rate: 100.0 } // 100 failures per hour
    ).await;
    
    let churn_handle = tokio::spawn({
        let simulator = simulator.clone();
        async move {
            simulator.start(Duration::from_secs(300)).await // 5 minutes
        }
    });
    
    // Monitor critical content availability
    let mut check_times = vec![];
    
    for i in 0..30 {
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        let nodes = simulator.cluster.nodes.read().await;
        let active_nodes: Vec<_> = nodes.values()
            .filter(|n| n.state.read().await.running)
            .take(5) // Try from up to 5 nodes
            .collect();
        
        if active_nodes.is_empty() {
            warn!("No active nodes available for check {}", i);
            continue;
        }
        
        let start = Instant::now();
        let mut retrieved = false;
        
        for node in active_nodes {
            let retrieval_manager = RetrievalManager::new(
                node.components.router.clone(),
                node.components.storage.clone(),
                Arc::new(learning::QLearnCacheManager::new(100 * 1024 * 1024)),
            );
            
            if let Ok(content) = retrieval_manager.retrieve(
                &critical_hash, 
                retrieval::RetrievalStrategy::Parallel
            ).await {
                if content == critical_content {
                    retrieved = true;
                    break;
                }
            }
        }
        
        let retrieval_time = start.elapsed();
        check_times.push(retrieval_time);
        
        assert!(retrieved, "Critical content must remain available even under extreme churn");
        info!("Check {}: Retrieved in {:?}", i+1, retrieval_time);
    }
    
    churn_handle.await??;
    
    // Calculate retrieval statistics
    let avg_time = check_times.iter().sum::<Duration>() / check_times.len() as u32;
    let max_time = check_times.iter().max().unwrap();
    
    let final_stats = simulator.get_stats().await;
    
    info!("Extreme churn test complete:");
    info!("  Total failures: {}", final_stats.total_failures);
    info!("  Total recoveries: {}", final_stats.total_recoveries);
    info!("  Average retrieval time: {:?}", avg_time);
    info!("  Maximum retrieval time: {:?}", max_time);
    info!("  Final availability: {:.2}%", final_stats.availability * 100.0);
    
    assert!(avg_time < Duration::from_secs(5), 
        "Average retrieval time should be under 5 seconds even with extreme churn");
    
    simulator.cluster.shutdown().await?;
    Ok(())
}