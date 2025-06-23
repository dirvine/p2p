//! DHT (Distributed Hash Table) integration tests
//!
//! Comprehensive tests for the Kademlia DHT implementation including:
//! - Key-value storage and retrieval
//! - DHT routing and replication
//! - Network partition tolerance
//! - Performance and scalability
//! - Data consistency and conflict resolution

use anyhow::Result;
use std::time::Duration;
use std::collections::HashMap;
use tokio::time::timeout;

use p2p_foundation::{Key, Record, P2PNode};
use crate::common::{TestNetwork, TestNetworkConfig, TestDataGen, TestAssertions, PerformanceTest};

// Integration test submodules - TBD  
// mod storage;
// mod routing;
// mod replication;
// mod consistency;
// mod performance;

/// Test basic DHT put/get operations
#[tokio::test]
async fn test_dht_basic_put_get() -> Result<()> {
    let network = TestNetwork::simple(3).await?;
    network.wait_for_discovery().await?;
    
    let key = TestDataGen::dht_key("test_key");
    let value = b"test_value".to_vec();
    
    // Store value in first node
    network.node(0)?.dht_put(key.clone(), value.clone()).await?;
    
    // Wait for replication
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Retrieve from different node
    let retrieved = network.node(1)?.dht_get(&key).await?;
    assert_eq!(retrieved, Some(value));
    
    network.stop().await?;
    Ok(())
}

/// Test DHT operations with large values
#[tokio::test]
async fn test_dht_large_values() -> Result<()> {
    let network = TestNetwork::simple(3).await?;
    network.wait_for_discovery().await?;
    
    let key = TestDataGen::dht_key("large_key");
    let large_value = TestDataGen::random_bytes(1024 * 1024); // 1MB
    
    // Store large value
    let start = std::time::Instant::now();
    network.node(0)?.dht_put(key.clone(), large_value.clone()).await?;
    let store_time = start.elapsed();
    
    println!("Large value store time: {:?}", store_time);
    
    // Wait for replication
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Retrieve large value
    let start = std::time::Instant::now();
    let retrieved = network.node(1)?.dht_get(&key).await?;
    let retrieve_time = start.elapsed();
    
    println!("Large value retrieve time: {:?}", retrieve_time);
    
    assert_eq!(retrieved, Some(large_value));
    
    // Performance assertions
    assert!(store_time < Duration::from_secs(10));
    assert!(retrieve_time < Duration::from_secs(10));
    
    network.stop().await?;
    Ok(())
}

/// Test DHT key routing and closest peer selection
#[tokio::test]
async fn test_dht_routing() -> Result<()> {
    let network = TestNetwork::simple(8).await?;
    network.wait_for_discovery().await?;
    
    let key = TestDataGen::dht_key("routing_test");
    
    // Find which node should be responsible for this key
    let mut closest_distances = Vec::new();
    for (i, node) in network.nodes.iter().enumerate() {
        let distance = node.dht_key_distance(&key).await?;
        closest_distances.push((i, distance));
    }
    
    // Sort by distance to find closest nodes
    closest_distances.sort_by_key(|(_, distance)| *distance);
    let closest_node_idx = closest_distances[0].0;
    
    println!("Key should be stored closest to node {}", closest_node_idx);
    
    // Store value from a different node
    let storing_node_idx = (closest_node_idx + 3) % network.nodes.len();
    let value = b"routed_value".to_vec();
    
    network.node(storing_node_idx)?.dht_put(key.clone(), value.clone()).await?;
    
    // Wait for routing
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Verify the value ended up on the closest node(s)
    let stored_on_closest = network.node(closest_node_idx)?.dht_get(&key).await?;
    assert_eq!(stored_on_closest, Some(value.clone()));
    
    // Verify we can retrieve from any node due to routing
    for i in 0..network.nodes.len() {
        let retrieved = timeout(
            Duration::from_secs(5),
            network.node(i)?.dht_get(&key)
        ).await??;
        assert_eq!(
            retrieved, Some(value.clone()),
            "Failed to retrieve from node {} via routing", i
        );
    }
    
    network.stop().await?;
    Ok(())
}

/// Test DHT replication across multiple nodes
#[tokio::test]
async fn test_dht_replication() -> Result<()> {
    let network = TestNetwork::simple(10).await?;
    network.wait_for_discovery().await?;
    
    let key = TestDataGen::dht_key("replication_test");
    let value = b"replicated_value".to_vec();
    
    // Store value
    network.node(0)?.dht_put(key.clone(), value.clone()).await?;
    
    // Wait for replication to complete
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Count how many nodes have the value
    let mut replication_count = 0;
    for (i, node) in network.nodes.iter().enumerate() {
        if let Some(_) = node.dht_get_local(&key).await? {
            replication_count += 1;
            println!("Value replicated to node {}", i);
        }
    }
    
    // Should be replicated to multiple nodes (typically k=20 in Kademlia)
    assert!(
        replication_count >= 3,
        "Value should be replicated to at least 3 nodes, found on {} nodes",
        replication_count
    );
    assert!(
        replication_count <= 8,
        "Value shouldn't be on all nodes, found on {} nodes",
        replication_count
    );
    
    network.stop().await?;
    Ok(())
}

/// Test DHT behavior under network partitions
#[tokio::test]
async fn test_dht_network_partition() -> Result<()> {
    let mut network = TestNetwork::simple(6).await?;
    network.wait_for_discovery().await?;
    
    let key1 = TestDataGen::dht_key("partition_test_1");
    let key2 = TestDataGen::dht_key("partition_test_2");
    let value1 = b"value_before_partition".to_vec();
    let value2 = b"value_during_partition".to_vec();
    
    // Store value before partition
    network.node(0)?.dht_put(key1.clone(), value1.clone()).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Create partition by isolating nodes 0,1,2 from 3,4,5
    // (In real implementation, this would involve network simulation)
    // For now, we simulate by shutting down some nodes
    let partition_nodes = vec![
        network.nodes.remove(5),
        network.nodes.remove(4),
        network.nodes.remove(3),
    ];
    
    for node in &partition_nodes {
        node.stop().await?;
    }
    
    // Wait for partition detection
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Store value during partition
    network.node(0)?.dht_put(key2.clone(), value2.clone()).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Values should still be available in the remaining partition
    assert_eq!(network.node(1)?.dht_get(&key1).await?, Some(value1));
    assert_eq!(network.node(2)?.dht_get(&key2).await?, Some(value2));
    
    network.stop().await?;
    Ok(())
}

/// Test DHT record expiration and refresh
#[tokio::test]
async fn test_dht_record_expiration() -> Result<()> {
    let network = TestNetwork::simple(3).await?;
    network.wait_for_discovery().await?;
    
    let key = TestDataGen::dht_key("expiration_test");
    let value = b"expiring_value".to_vec();
    
    // Store value with short TTL
    let record = Record::new(key.clone(), value.clone())
        .with_ttl(Duration::from_secs(5))
        .with_publisher(network.node(0)?.peer_id());
    
    network.node(0)?.dht_put_record(record).await?;
    
    // Verify value is initially available
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(network.node(1)?.dht_get(&key).await?, Some(value.clone()));
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(6)).await;
    
    // Value should be expired
    let expired_result = network.node(1)?.dht_get(&key).await?;
    assert_eq!(expired_result, None, "Value should have expired");
    
    network.stop().await?;
    Ok(())
}

/// Test DHT record versioning and conflict resolution
#[tokio::test]
async fn test_dht_versioning() -> Result<()> {
    let network = TestNetwork::simple(3).await?;
    network.wait_for_discovery().await?;
    
    let key = TestDataGen::dht_key("versioning_test");
    let value1 = b"version_1".to_vec();
    let value2 = b"version_2".to_vec();
    
    // Store initial version
    let record1 = Record::new(key.clone(), value1.clone())
        .with_version(1)
        .with_publisher(network.node(0)?.peer_id());
    
    network.node(0)?.dht_put_record(record1).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Store newer version from different node
    let record2 = Record::new(key.clone(), value2.clone())
        .with_version(2)
        .with_publisher(network.node(1)?.peer_id());
    
    network.node(1)?.dht_put_record(record2).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // All nodes should have the newer version
    for i in 0..3 {
        let retrieved = network.node(i)?.dht_get(&key).await?;
        assert_eq!(
            retrieved, Some(value2.clone()),
            "Node {} should have the newer version", i
        );
    }
    
    // Try to store older version - should be rejected
    let old_record = Record::new(key.clone(), b"old_version".to_vec())
        .with_version(1)
        .with_publisher(network.node(2)?.peer_id());
    
    let result = network.node(2)?.dht_put_record(old_record).await;
    assert!(result.is_err(), "Older version should be rejected");
    
    network.stop().await?;
    Ok(())
}

/// Test DHT with many concurrent operations
#[tokio::test]
async fn test_dht_concurrent_operations() -> Result<()> {
    let network = TestNetwork::simple(5).await?;
    network.wait_for_discovery().await?;
    
    let num_operations = 50;
    let mut handles = Vec::new();
    
    // Launch concurrent put operations
    for i in 0..num_operations {
        let node_idx = i % network.nodes.len();
        let node = &network.nodes[node_idx];
        let key = TestDataGen::dht_key(&format!("concurrent_key_{}", i));
        let value = format!("concurrent_value_{}", i).into_bytes();
        
        let handle = {
            let node = node.clone(); // Assuming P2PNode implements Clone
            let key = key.clone();
            let value = value.clone();
            tokio::spawn(async move {
                node.dht_put(key, value).await
            })
        };
        handles.push((i, handle));
    }
    
    // Wait for all operations to complete
    let mut successful_ops = 0;
    for (i, handle) in handles {
        match handle.await {
            Ok(Ok(_)) => successful_ops += 1,
            Ok(Err(e)) => println!("Operation {} failed: {}", i, e),
            Err(e) => println!("Operation {} panicked: {}", i, e),
        }
    }
    
    println!("Successful concurrent operations: {}/{}", successful_ops, num_operations);
    assert!(
        successful_ops >= num_operations * 8 / 10, // At least 80% success rate
        "Too many concurrent operations failed"
    );
    
    // Wait for propagation
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Verify stored values
    let mut verified_count = 0;
    for i in 0..num_operations {
        let key = TestDataGen::dht_key(&format!("concurrent_key_{}", i));
        let expected_value = format!("concurrent_value_{}", i).into_bytes();
        
        if let Some(retrieved) = network.node(0)?.dht_get(&key).await? {
            if retrieved == expected_value {
                verified_count += 1;
            }
        }
    }
    
    println!("Verified values: {}/{}", verified_count, successful_ops);
    assert!(
        verified_count >= successful_ops * 9 / 10,
        "Too many stored values were not retrievable"
    );
    
    network.stop().await?;
    Ok(())
}

/// Performance benchmark for DHT operations
#[tokio::test]
async fn test_dht_performance() -> Result<()> {
    let network = TestNetwork::simple(10).await?;
    network.wait_for_discovery().await?;
    
    let mut perf = PerformanceTest::new();
    
    // Benchmark single put operation
    let key = TestDataGen::dht_key("perf_test_single");
    let value = TestDataGen::random_bytes(1024);
    
    perf.measure_async("single_put", async {
        network.node(0)?.dht_put(key.clone(), value.clone()).await
    }).await?;
    
    // Benchmark single get operation
    perf.measure_async("single_get", async {
        network.node(1)?.dht_get(&key).await
    }).await?;
    
    // Benchmark batch operations
    let batch_size = 20;
    let mut batch_keys = Vec::new();
    
    let batch_put_time = perf.measure_async("batch_put", async {
        for i in 0..batch_size {
            let key = TestDataGen::dht_key(&format!("batch_key_{}", i));
            let value = TestDataGen::random_bytes(512);
            batch_keys.push(key.clone());
            network.node(i % network.nodes.len())?.dht_put(key, value).await?;
        }
        Ok::<(), anyhow::Error>(())
    }).await?;
    
    // Wait for propagation
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    let batch_get_time = perf.measure_async("batch_get", async {
        for key in &batch_keys {
            network.node(0)?.dht_get(key).await?;
        }
        Ok::<(), anyhow::Error>(())
    }).await?;
    
    // Print results
    perf.print_results();
    
    // Performance assertions
    let single_put_time = perf.get_measurement("single_put").unwrap();
    let single_get_time = perf.get_measurement("single_get").unwrap();
    
    assert!(
        single_put_time < Duration::from_secs(2),
        "Single put should take less than 2 seconds, took {:?}",
        single_put_time
    );
    
    assert!(
        single_get_time < Duration::from_millis(500),
        "Single get should take less than 500ms, took {:?}",
        single_get_time
    );
    
    // Batch operations should be more efficient per operation
    let avg_batch_put_time = batch_put_time / batch_size as u32;
    let avg_batch_get_time = batch_get_time / batch_size as u32;
    
    println!("Average batch put time: {:?}", avg_batch_put_time);
    println!("Average batch get time: {:?}", avg_batch_get_time);
    
    network.stop().await?;
    Ok(())
}

/// Test DHT data persistence across node restarts
#[tokio::test]
async fn test_dht_persistence() -> Result<()> {
    let mut network = TestNetwork::simple(3).await?;
    network.wait_for_discovery().await?;
    
    let key = TestDataGen::dht_key("persistence_test");
    let value = b"persistent_value".to_vec();
    
    // Store value
    network.node(0)?.dht_put(key.clone(), value.clone()).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Shutdown one node
    let restarting_node = network.nodes.remove(1);
    let restarting_config = network.configs[1].clone();
    restarting_node.stop().await?;
    
    // Wait a bit
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Restart the node
    let restarted_node = P2PNode::new(restarting_config).await?;
    
    // Reconnect to network
    let bootstrap_addr = network.addrs[0].clone();
    restarted_node.connect(bootstrap_addr).await?;
    
    // Wait for DHT sync
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Verify data is still available
    let retrieved = restarted_node.dht_get(&key).await?;
    assert_eq!(retrieved, Some(value));
    
    // Cleanup
    network.nodes.push(restarted_node);
    network.stop().await?;
    Ok(())
}

/// Test DHT key space distribution
#[tokio::test]
async fn test_dht_key_space_distribution() -> Result<()> {
    let network = TestNetwork::simple(8).await?;
    network.wait_for_discovery().await?;
    
    let num_keys = 100;
    let mut key_distribution: HashMap<usize, usize> = HashMap::new();
    
    // Store many keys and track which nodes they end up on
    for i in 0..num_keys {
        let key = TestDataGen::dht_key(&format!("distribution_key_{}", i));
        let value = format!("value_{}", i).into_bytes();
        
        network.node(0)?.dht_put(key.clone(), value).await?;
        
        // Find which node(s) store this key
        for (node_idx, node) in network.nodes.iter().enumerate() {
            if let Some(_) = node.dht_get_local(&key).await? {
                *key_distribution.entry(node_idx).or_insert(0) += 1;
            }
        }
    }
    
    // Wait for all operations to complete
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    println!("Key distribution across nodes: {:?}", key_distribution);
    
    // Verify reasonably even distribution
    let total_stored: usize = key_distribution.values().sum();
    let avg_per_node = total_stored as f64 / network.nodes.len() as f64;
    
    for (node_idx, count) in key_distribution {
        let deviation = (count as f64 - avg_per_node).abs() / avg_per_node;
        assert!(
            deviation < 0.5, // Allow 50% deviation from average
            "Node {} has too uneven key distribution: {} keys (avg: {:.1})",
            node_idx, count, avg_per_node
        );
    }
    
    network.stop().await?;
    Ok(())
}