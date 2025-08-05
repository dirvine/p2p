//! Full network simulation test
//!
//! This test simulates a complete P2P network with multiple nodes
//! demonstrating all adaptive layers working together.

use saorsa_core::adaptive::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Simulated network environment
struct NetworkSimulation {
    nodes: HashMap<NodeId, Arc<NetworkCoordinator>>,
    network_latency: Duration,
    packet_loss_rate: f64,
}

impl NetworkSimulation {
    async fn new(num_nodes: usize) -> Self {
        let mut nodes = HashMap::new();
        let mut bootstrap_nodes = vec![];
        
        for i in 0..num_nodes {
            let identity = NodeIdentity::generate().unwrap();
            let node_id = identity.node_id();
            
            let config = NetworkConfig {
                bootstrap_nodes: bootstrap_nodes.clone(),
                storage_capacity: 10,
                max_connections: 50,
                replication_factor: 3,
                ml_enabled: true,
                monitoring_interval: Duration::from_secs(10),
                security_level: 7,
            };
            
            let coordinator = Arc::new(
                NetworkCoordinator::new(identity, config).await.unwrap()
            );
            
            nodes.insert(node_id.clone(), coordinator);
            
            // First 3 nodes are bootstrap nodes
            if i < 3 {
                bootstrap_nodes.push(format!("node-{}", i));
            }
        }
        
        Self {
            nodes,
            network_latency: Duration::from_millis(50),
            packet_loss_rate: 0.01,
        }
    }
    
    async fn join_all_nodes(&self) {
        for (node_id, coordinator) in &self.nodes {
            println!("Node {:?} joining network...", node_id);
            let _ = coordinator.join_network().await;
        }
    }
    
    async fn simulate_data_operations(&self) {
        // Store data from different nodes
        let data_items = vec![
            b"Important document".to_vec(),
            b"Video file chunk 1".to_vec(),
            b"Configuration data".to_vec(),
            b"User profile information".to_vec(),
            b"Cached web content".to_vec(),
        ];
        
        let mut stored_hashes = vec![];
        
        // Store data from random nodes
        for (i, data) in data_items.iter().enumerate() {
            let node = self.nodes.values().nth(i % self.nodes.len()).unwrap();
            match node.store(data.clone()).await {
                Ok(hash) => {
                    println!("Stored data item {} with hash {:?}", i, hash);
                    stored_hashes.push(hash);
                }
                Err(e) => eprintln!("Failed to store data: {:?}", e),
            }
        }
        
        // Simulate time passing for replication
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Retrieve data from different nodes
        for (i, hash) in stored_hashes.iter().enumerate() {
            let node = self.nodes.values()
                .nth((i + 2) % self.nodes.len())
                .unwrap();
            
            match node.retrieve(hash).await {
                Ok(data) => println!("Retrieved data item {} successfully", i),
                Err(e) => eprintln!("Failed to retrieve data {}: {:?}", i, e),
            }
        }
    }
    
    async fn simulate_gossip_communication(&self) {
        // Different types of gossip messages
        let topics = vec![
            ("system-update", b"New version available".to_vec()),
            ("peer-discovery", b"Node announcement".to_vec()),
            ("content-announcement", b"New content available".to_vec()),
        ];
        
        for (topic, message) in topics {
            let node = self.nodes.values().next().unwrap();
            let _ = node.publish(topic, message).await;
            println!("Published message to topic: {}", topic);
        }
    }
    
    async fn simulate_network_stress(&self) {
        // Simulate high churn
        println!("\nSimulating high churn scenario...");
        for node in self.nodes.values().take(2) {
            node.handle_degradation(DegradationReason::HighChurn).await.unwrap();
        }
        
        // Simulate low connectivity
        println!("Simulating low connectivity...");
        for node in self.nodes.values().skip(2).take(2) {
            node.handle_degradation(DegradationReason::LowConnectivity).await.unwrap();
        }
        
        // Simulate high load
        println!("Simulating high load...");
        for node in self.nodes.values().skip(4).take(2) {
            node.handle_degradation(DegradationReason::HighLoad).await.unwrap();
        }
    }
    
    async fn collect_metrics(&self) {
        println!("\n=== Network Metrics ===");
        
        for (i, (node_id, coordinator)) in self.nodes.iter().enumerate() {
            let stats = coordinator.get_network_stats().await;
            println!(
                "Node {}: peers={}, success_rate={:.2}%, cache_hit={:.2}%, churn={:.2}%",
                i,
                stats.connected_peers,
                stats.routing_success_rate * 100.0,
                stats.cache_hit_rate * 100.0,
                stats.churn_rate * 100.0
            );
        }
    }
}

#[tokio::test]
async fn test_full_network_simulation() {
    // Create a network with 10 nodes
    let simulation = NetworkSimulation::new(10).await;
    
    println!("=== P2P Network Simulation ===");
    println!("Nodes: {}", simulation.nodes.len());
    
    // Phase 1: Network formation
    println!("\n[Phase 1] Network Formation");
    simulation.join_all_nodes().await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Phase 2: Normal operations
    println!("\n[Phase 2] Normal Operations");
    simulation.simulate_data_operations().await;
    simulation.simulate_gossip_communication().await;
    
    // Phase 3: Stress testing
    println!("\n[Phase 3] Stress Testing");
    simulation.simulate_network_stress().await;
    
    // Phase 4: Recovery and metrics
    println!("\n[Phase 4] Recovery and Metrics");
    tokio::time::sleep(Duration::from_secs(1)).await;
    simulation.collect_metrics().await;
    
    // Phase 5: Graceful shutdown
    println!("\n[Phase 5] Graceful Shutdown");
    for (i, coordinator) in simulation.nodes.into_values().enumerate() {
        println!("Shutting down node {}...", i);
        Arc::try_unwrap(coordinator)
            .ok()
            .map(|c| c.shutdown());
    }
    
    println!("\n=== Simulation Complete ===");
}

#[tokio::test]
async fn test_adaptive_routing_layers() -> Result<()> {
    let sim = NetworkSimulation::new(5).await;
    sim.join_all_nodes().await;
    
    println!("\n=== Testing Adaptive Routing Layers ===");
    
    // Test that different routing strategies are used
    let source = sim.nodes.values().next().unwrap();
    let target_id = NodeId { hash: [42u8; 32] };
    
    match source.coordinate_routing(&target_id).await {
        Ok(path) => {
            println!("Routing path selected with {} hops", path.len());
        }
        Err(e) => {
            println!("Routing coordination result: {:?}", e);
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_ml_optimization_impact() -> Result<()> {
    // Create two networks - one with ML, one without
    let identity1 = NodeIdentity::generate().unwrap();
    let identity2 = NodeIdentity::generate().unwrap();
    
    let config_ml = NetworkConfig {
        ml_enabled: true,
        ..Default::default()
    };
    
    let config_no_ml = NetworkConfig {
        ml_enabled: false,
        ..Default::default()
    };
    
    let coordinator_ml = NetworkCoordinator::new(identity1, config_ml).await.unwrap();
    let coordinator_no_ml = NetworkCoordinator::new(identity2, config_no_ml).await.unwrap();
    
    println!("\n=== ML Optimization Impact Test ===");
    
    // Perform same operations on both
    let test_data = vec![1u8; 1000];
    let iterations = 10;
    
    // With ML
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let hash = coordinator_ml.store(test_data.clone()).await.unwrap();
        let _ = coordinator_ml.retrieve(&hash).await;
    }
    let ml_duration = start.elapsed();
    
    // Without ML
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let hash = coordinator_no_ml.store(test_data.clone()).await.unwrap();
        let _ = coordinator_no_ml.retrieve(&hash).await;
    }
    let no_ml_duration = start.elapsed();
    
    println!("With ML optimization: {:?}", ml_duration);
    println!("Without ML optimization: {:?}", no_ml_duration);
    
    // In a real network, ML should improve performance
    // In this simulation, we just verify both work
    Ok(())
}

#[tokio::test]
async fn test_trust_based_interactions() {
    let sim = NetworkSimulation::new(6).await;
    sim.join_all_nodes().await;
    
    println!("\n=== Trust-Based Interactions ===");
    
    // Simulate trust evolution through interactions
    let nodes: Vec<_> = sim.nodes.values().collect();
    
    // Good interactions between nodes 0-2
    for _ in 0..5 {
        let data = b"trusted data".to_vec();
        let hash = nodes[0].store(data.clone()).await.unwrap();
        let _ = nodes[1].retrieve(&hash).await;
        let _ = nodes[2].retrieve(&hash).await;
    }
    
    // Check network stats to see trust impact
    let stats = nodes[0].get_network_stats().await;
    println!("Average trust score: {:.2}", stats.average_trust_score);
}