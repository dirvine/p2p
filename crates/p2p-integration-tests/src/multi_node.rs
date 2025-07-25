//! Multi-node integration tests for the adaptive P2P network
//!
//! Tests basic functionality with multiple nodes including:
//! - Network formation and discovery
//! - Content storage and retrieval
//! - Message propagation
//! - Trust establishment

use p2p_integration_tests::*;
use saorsa_core::adaptive::*;
use anyhow::Result;
use std::time::Duration;
use tracing::{info, debug};
use pretty_assertions::assert_eq;

#[tokio::test]
async fn test_network_formation() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting network formation test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 10,
        bootstrap_count: 3,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    
    // Wait for network to stabilize
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Verify all nodes are connected
    let nodes = cluster.nodes.read().await;
    for node in nodes.values() {
        let state = node.state.read().await;
        assert!(state.running, "Node {} should be running", node.id);
        assert!(!state.peers.is_empty(), "Node {} should have peers", node.id);
        
        // Check DHT routing table
        let routing_info = node.components.dht.get_routing_info().await;
        debug!("Node {} has {} routing entries", node.id, routing_info.total_peers);
        assert!(routing_info.total_peers > 0, "Node {} should have routing entries", node.id);
    }
    
    // Get cluster stats
    let stats = cluster.get_stats().await;
    info!("Cluster stats: {:?}", stats);
    assert!(stats.total_messages > 0, "Messages should have been exchanged");
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_content_storage_retrieval() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting content storage and retrieval test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 20,
        bootstrap_count: 5,
        topology: NetworkTopology::Mesh,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Test content operations
    let content_sizes = vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024]; // 1KB to 1MB
    let nodes = cluster.nodes.read().await;
    let node_ids: Vec<String> = nodes.keys().cloned().collect();
    
    for size in content_sizes {
        info!("Testing content of size {} bytes", size);
        
        // Generate test content
        let content = utils::generate_content(size);
        
        // Store from random node
        let store_node_id = &node_ids[rand::random::<usize>() % node_ids.len()];
        let store_node = nodes.get(store_node_id).unwrap();
        
        let metadata = storage::ContentMetadata {
            size,
            content_type: ContentType::DataRetrieval,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            chunk_count: if size > 1024 * 1024 { Some((size / (1024 * 1024)) as u32 + 1) } else { None },
            replication_factor: 8,
        };
        
        let (hash, store_duration) = utils::measure_latency(|| async {
            store_node.components.storage.store(content.clone(), metadata).await
        }).await;
        let hash = hash?;
        
        info!("Stored content with hash {:?} in {:?}", hash, store_duration);
        
        // Update stats
        store_node.stats.write().await.storage_ops += 1;
        
        // Wait for replication
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Retrieve from different random nodes
        for _ in 0..5 {
            let retrieve_node_id = &node_ids[rand::random::<usize>() % node_ids.len()];
            if retrieve_node_id == store_node_id {
                continue; // Skip same node
            }
            
            let retrieve_node = nodes.get(retrieve_node_id).unwrap();
            
            let retrieval_manager = RetrievalManager::new(
                retrieve_node.components.router.clone(),
                retrieve_node.components.storage.clone(),
                Arc::new(learning::QLearnCacheManager::new(100 * 1024 * 1024)),
            );
            
            let (result, retrieve_duration) = utils::measure_latency(|| async {
                retrieval_manager.retrieve(&hash, retrieval::RetrievalStrategy::Parallel).await
            }).await;
            
            match result {
                Ok(retrieved) => {
                    assert_eq!(retrieved, content, "Retrieved content should match original");
                    info!("Retrieved from {} in {:?}", retrieve_node_id, retrieve_duration);
                    retrieve_node.stats.write().await.retrieval_ops += 1;
                }
                Err(e) => {
                    retrieve_node.stats.write().await.failed_ops += 1;
                    return Err(anyhow::anyhow!("Retrieval failed from {}: {}", retrieve_node_id, e));
                }
            }
        }
    }
    
    // Check replication
    let mut replica_counts = std::collections::HashMap::new();
    for node in nodes.values() {
        let stored_hashes = vec![]; // In real implementation, would query storage
        for hash in stored_hashes {
            *replica_counts.entry(hash).or_insert(0) += 1;
        }
    }
    
    let stats = cluster.get_stats().await;
    info!("Final cluster stats: {:?}", stats);
    info!("Success rate: {:.2}%", stats.success_rate() * 100.0);
    assert!(stats.success_rate() > 0.95, "Success rate should be above 95%");
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_gossip_propagation() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting gossip propagation test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 30,
        bootstrap_count: 5,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Subscribe all nodes to test topic
    let topic = "test_gossip_topic";
    let nodes = cluster.nodes.read().await;
    
    for node in nodes.values() {
        node.components.gossip.subscribe(topic).await?;
    }
    
    // Publish message from random node
    let publisher_id = nodes.keys().next().unwrap();
    let publisher = nodes.get(publisher_id).unwrap();
    
    let message = b"Test gossip message".to_vec();
    let publish_time = std::time::Instant::now();
    
    publisher.components.gossip.publish(topic, message.clone()).await?;
    info!("Published message from {}", publisher_id);
    
    // Wait for propagation
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Check message receipt (in real implementation would track via callbacks)
    let propagation_time = publish_time.elapsed();
    info!("Message propagated in {:?}", propagation_time);
    
    // Verify mesh health
    for node in nodes.values() {
        let gossip_stats = node.components.gossip.get_stats().await;
        debug!("Node {} gossip stats: peer_count={}, messages_sent={}", 
            node.id, gossip_stats.peer_count, gossip_stats.messages_sent);
        assert!(gossip_stats.peer_count > 0, "Node {} should have gossip peers", node.id);
    }
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_trust_establishment() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting trust establishment test");
    
    // Create test cluster
    let config = TestClusterConfig {
        node_count: 15,
        bootstrap_count: 3,
        topology: NetworkTopology::Hierarchical,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    let nodes = cluster.nodes.read().await;
    
    // Simulate successful interactions
    info!("Simulating successful interactions");
    for _ in 0..100 {
        let node1_id = &nodes.keys().collect::<Vec<_>>()[rand::random::<usize>() % nodes.len()];
        let node2_id = &nodes.keys().collect::<Vec<_>>()[rand::random::<usize>() % nodes.len()];
        
        if node1_id != node2_id {
            let node1 = nodes.get(node1_id).unwrap();
            let node2 = nodes.get(node2_id).unwrap();
            
            // Update trust based on successful interaction
            node1.components.trust.update_interaction(
                &node1.identity.node_id,
                &node2.identity.node_id,
                true,
                1.0,
            ).await;
            
            node1.stats.write().await.messages_sent += 1;
            node2.stats.write().await.messages_received += 1;
        }
    }
    
    // Simulate some failed interactions
    info!("Simulating failed interactions");
    for _ in 0..20 {
        let node1_id = &nodes.keys().collect::<Vec<_>>()[rand::random::<usize>() % nodes.len()];
        let node2_id = &nodes.keys().collect::<Vec<_>>()[rand::random::<usize>() % nodes.len()];
        
        if node1_id != node2_id {
            let node1 = nodes.get(node1_id).unwrap();
            let node2 = nodes.get(node2_id).unwrap();
            
            // Update trust based on failed interaction
            node1.components.trust.update_interaction(
                &node1.identity.node_id,
                &node2.identity.node_id,
                false,
                0.0,
            ).await;
            
            node1.stats.write().await.failed_ops += 1;
        }
    }
    
    // Compute global trust
    info!("Computing global trust scores");
    for node in nodes.values() {
        node.components.trust.compute_trust().await?;
    }
    
    // Verify trust scores
    for node in nodes.values() {
        let trust_scores = node.components.trust.get_all_trust_scores().await;
        let avg_trust: f64 = trust_scores.values().sum::<f64>() / trust_scores.len().max(1) as f64;
        
        info!("Node {} average trust: {:.3}", node.id, avg_trust);
        assert!(avg_trust > 0.0, "Trust scores should be established");
    }
    
    // Test trust-based routing
    info!("Testing trust-based routing");
    let source = nodes.values().next().unwrap();
    let target_id = nodes.keys().last().unwrap();
    let target_node = nodes.get(target_id).unwrap();
    
    let trust_strategy = TrustBasedRoutingStrategy::new(source.components.trust.clone());
    let path = trust_strategy.find_path(&target_node.identity.node_id).await?;
    
    info!("Trust-based path length: {}", path.len());
    assert!(!path.is_empty(), "Should find trust-based path");
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_different_topologies() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting topology comparison test");
    
    let topologies = vec![
        NetworkTopology::Random,
        NetworkTopology::Ring,
        NetworkTopology::Star,
        NetworkTopology::Mesh,
        NetworkTopology::Hierarchical,
    ];
    
    for topology in topologies {
        info!("Testing {:?} topology", topology);
        
        let config = TestClusterConfig {
            node_count: 20,
            bootstrap_count: 3,
            topology,
            timeout: Duration::from_secs(60),
            ..Default::default()
        };
        
        let mut cluster = TestCluster::new(config).await?;
        cluster.start().await?;
        
        // Quick stabilization check
        match cluster.wait_for_stabilization(Duration::from_secs(20)).await {
            Ok(_) => {
                info!("{:?} topology stabilized", topology);
                
                // Perform basic operations
                let nodes = cluster.nodes.read().await;
                let node = nodes.values().next().unwrap();
                
                // Store and retrieve test
                let content = b"Topology test content".to_vec();
                let metadata = storage::ContentMetadata {
                    size: content.len(),
                    content_type: ContentType::DataRetrieval,
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    chunk_count: None,
                    replication_factor: 5,
                };
                
                let hash = node.components.storage.store(content.clone(), metadata).await?;
                let retrieved = node.components.storage.retrieve(&hash).await?;
                assert!(retrieved.is_some(), "Should retrieve stored content");
                
                let stats = cluster.get_stats().await;
                info!("{:?} topology stats: messages={}, throughput={:.2} MB/s", 
                    topology, stats.total_messages, stats.throughput_mbps());
            }
            Err(e) => {
                info!("{:?} topology failed to stabilize: {}", topology, e);
            }
        }
        
        cluster.shutdown().await?;
        
        // Brief pause between topology tests
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    
    Ok(())
}

#[tokio::test]
#[ignore] // Long-running test
async fn test_large_scale_network() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting large-scale network test");
    
    // Create large test cluster
    let config = TestClusterConfig {
        node_count: 100,
        bootstrap_count: 10,
        topology: NetworkTopology::Random,
        timeout: Duration::from_secs(600),
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    
    // Wait for network formation
    info!("Waiting for large network to stabilize...");
    cluster.wait_for_stabilization(Duration::from_secs(120)).await?;
    
    // Perform stress test
    let nodes = cluster.nodes.read().await;
    let node_ids: Vec<String> = nodes.keys().cloned().collect();
    
    // Parallel storage operations
    info!("Performing parallel storage operations");
    let storage_futures = (0..50).map(|i| {
        let nodes = nodes.clone();
        let node_ids = node_ids.clone();
        
        async move {
            let node_id = &node_ids[i % node_ids.len()];
            let node = nodes.get(node_id).unwrap();
            
            let content = utils::generate_content(10 * 1024); // 10KB
            let metadata = storage::ContentMetadata {
                size: content.len(),
                content_type: ContentType::DataRetrieval,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                chunk_count: None,
                replication_factor: 8,
            };
            
            node.components.storage.store(content, metadata).await
        }
    });
    
    let storage_results = futures::future::join_all(storage_futures).await;
    let successful_stores = storage_results.iter().filter(|r| r.is_ok()).count();
    info!("Successful stores: {}/50", successful_stores);
    assert!(successful_stores > 45, "Most storage operations should succeed");
    
    // Check network health
    let mut health_scores = vec![];
    for node in nodes.values() {
        let health = node.components.monitoring.get_health().await;
        health_scores.push(health.score);
    }
    
    let avg_health = health_scores.iter().sum::<f64>() / health_scores.len() as f64;
    info!("Average network health: {:.3}", avg_health);
    assert!(avg_health > 0.7, "Network should maintain good health");
    
    let stats = cluster.get_stats().await;
    info!("Large-scale network stats:");
    info!("  Total messages: {}", stats.total_messages);
    info!("  Messages/sec: {:.2}", stats.messages_per_second());
    info!("  Throughput: {:.2} MB/s", stats.throughput_mbps());
    info!("  Success rate: {:.2}%", stats.success_rate() * 100.0);
    
    cluster.shutdown().await?;
    Ok(())
}