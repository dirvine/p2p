//! Comprehensive Storage Integration Tests
//!
//! Tests store/retrieve operations, replication verification, consistency checks,
//! and performance under load.

use anyhow::{Context, Result};
use blake3::Hash;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

use saorsa_core::{
    config::Config,
    adaptive::replication::ReplicationManager,
    storage::StorageManager,
};

/// Test framework for storage scenarios
struct StorageTestFramework {
    nodes: Vec<Arc<Node>>,
    test_records: Arc<RwLock<HashMap<StorageKey, StorageValue>>>,
    replication_factor: usize,
}

impl StorageTestFramework {
    async fn new(node_count: usize, replication_factor: usize) -> Result<Self> {
        let mut nodes = Vec::new();

        for i in 0..node_count {
            let mut config = Config::default();
            config.network.listen_port = 8000 + i as u16;
            config.storage.replication_factor = replication_factor;
            config.storage.max_storage_size = 100 * 1024 * 1024; // 100MB per node

            let node = Node::new(config)
                .await
                .context(format!("Failed to create storage node {}", i))?;

            nodes.push(Arc::new(node));
        }

        Ok(Self {
            nodes,
            test_records: Arc::new(RwLock::new(HashMap::new())),
            replication_factor,
        })
    }

    async fn start_all_nodes(&self) -> Result<()> {
        for (i, node) in self.nodes.iter().enumerate() {
            node.start()
                .await
                .context(format!("Failed to start storage node {}", i))?;
            sleep(Duration::from_millis(100)).await;
        }

        // Connect nodes in mesh for storage replication
        for i in 0..self.nodes.len() {
            for j in (i + 1)..self.nodes.len() {
                let peer_addr = format!("/ip4/127.0.0.1/tcp/{}", 8000 + j);
                self.nodes[i].connect_to_peer(&peer_addr).await?;
                sleep(Duration::from_millis(50)).await;
            }
        }

        // Wait for storage network to stabilize
        sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    async fn store_test_data(&self, key: StorageKey, value: StorageValue) -> Result<()> {
        // Store via first node
        self.nodes[0]
            .store(key.clone(), value.clone())
            .await
            .context("Failed to store test data")?;

        // Track in test records
        self.test_records.write().await.insert(key, value);

        // Wait for replication
        sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    async fn verify_replication(&self, key: &StorageKey) -> Result<usize> {
        let mut replica_count = 0;

        for node in &self.nodes {
            if let Ok(Some(_)) = node.retrieve(key).await {
                replica_count += 1;
            }
        }

        Ok(replica_count)
    }

    async fn verify_consistency(&self) -> Result<bool> {
        let test_records = self.test_records.read().await;

        for (key, expected_value) in test_records.iter() {
            let mut retrieved_values = HashSet::new();

            for node in &self.nodes {
                if let Ok(Some(value)) = node.retrieve(key).await {
                    retrieved_values.insert(value);
                }
            }

            // All retrieved values should be identical
            if retrieved_values.len() > 1 {
                return Ok(false);
            }

            // At least one value should match expected
            if !retrieved_values.is_empty() {
                let retrieved_value = retrieved_values.iter().next().unwrap();
                if retrieved_value != expected_value {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn simulate_node_failure(&self, node_index: usize) -> Result<()> {
        if node_index < self.nodes.len() {
            self.nodes[node_index].shutdown().await?;
            sleep(Duration::from_secs(2)).await;
        }
        Ok(())
    }

    async fn get_storage_stats(&self) -> Result<Vec<(usize, usize, u64)>> {
        let mut stats = Vec::new();

        for (i, node) in self.nodes.iter().enumerate() {
            if let Ok(storage_info) = node.get_storage_info().await {
                stats.push((i, storage_info.record_count, storage_info.total_size));
            }
        }

        Ok(stats)
    }

    async fn shutdown_all(&self) -> Result<()> {
        for node in &self.nodes {
            let _ = node.shutdown().await; // Ignore errors during shutdown
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_basic_store_and_retrieve() -> Result<()> {
    let framework = StorageTestFramework::new(3, 2).await?;
    framework.start_all_nodes().await?;

    // Test basic store and retrieve
    let key = StorageKey::from_str("test_key_1")?;
    let value = StorageValue::from_bytes(b"test_value_1".to_vec())?;

    framework
        .store_test_data(key.clone(), value.clone())
        .await?;

    // Verify storage on all nodes
    for (i, node) in framework.nodes.iter().enumerate() {
        if let Ok(Some(retrieved_value)) = node.retrieve(&key).await {
            assert_eq!(retrieved_value, value, "Value mismatch on node {}", i);
        }
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_replication_factor_compliance() -> Result<()> {
    let replication_factor = 2;
    let framework = StorageTestFramework::new(5, replication_factor).await?;
    framework.start_all_nodes().await?;

    // Store multiple test records
    for i in 0..10 {
        let key = StorageKey::from_str(&format!("repl_test_{}", i))?;
        let value = StorageValue::from_bytes(format!("value_{}", i).into_bytes())?;
        framework.store_test_data(key.clone(), value).await?;

        // Wait for replication
        sleep(Duration::from_millis(100)).await;

        // Verify replication factor
        let replica_count = framework.verify_replication(&key).await?;
        assert!(
            replica_count >= replication_factor,
            "Key {} has {} replicas, expected >= {}",
            i,
            replica_count,
            replication_factor
        );
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_storage_consistency_under_load() -> Result<()> {
    let framework = StorageTestFramework::new(4, 3).await?;
    framework.start_all_nodes().await?;

    // Store data concurrently from multiple nodes
    let store_tasks = (0..20).map(|i| {
        let framework = &framework;
        let node_index = i % framework.nodes.len();

        async move {
            let key = StorageKey::from_str(&format!("concurrent_{}", i))?;
            let value = StorageValue::from_bytes(format!("concurrent_value_{}", i).into_bytes())?;

            framework.nodes[node_index]
                .store(key.clone(), value.clone())
                .await?;
            framework.test_records.write().await.insert(key, value);

            Result::<()>::Ok(())
        }
    });

    // Execute all stores concurrently
    let results: Result<Vec<_>, _> = futures::future::try_join_all(store_tasks).await;
    results.context("Concurrent storage operations failed")?;

    // Wait for replication to complete
    sleep(Duration::from_secs(3)).await;

    // Verify consistency across all nodes
    let is_consistent = framework.verify_consistency().await?;
    assert!(
        is_consistent,
        "Storage inconsistency detected under concurrent load"
    );

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_storage_recovery_after_node_failure() -> Result<()> {
    let framework = StorageTestFramework::new(4, 3).await?;
    framework.start_all_nodes().await?;

    // Store test data
    for i in 0..5 {
        let key = StorageKey::from_str(&format!("recovery_test_{}", i))?;
        let value = StorageValue::from_bytes(format!("recovery_value_{}", i).into_bytes())?;
        framework.store_test_data(key, value).await?;
    }

    // Verify initial replication
    let test_records = framework.test_records.read().await;
    for key in test_records.keys() {
        let replica_count = framework.verify_replication(key).await?;
        assert!(replica_count >= 3, "Initial replication insufficient");
    }
    drop(test_records);

    // Simulate node failure
    framework.simulate_node_failure(1).await?;

    // Wait for failure detection and potential re-replication
    sleep(Duration::from_secs(5)).await;

    // Verify data is still available from remaining nodes
    let test_records = framework.test_records.read().await;
    for (key, expected_value) in test_records.iter() {
        let mut found = false;

        for (i, node) in framework.nodes.iter().enumerate() {
            if i == 1 {
                continue;
            } // Skip failed node

            if let Ok(Some(retrieved_value)) = node.retrieve(key).await {
                assert_eq!(
                    &retrieved_value, expected_value,
                    "Value corruption detected after node failure"
                );
                found = true;
                break;
            }
        }

        assert!(found, "Data loss detected after node failure");
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_large_data_storage_and_retrieval() -> Result<()> {
    let framework = StorageTestFramework::new(3, 2).await?;
    framework.start_all_nodes().await?;

    // Test with various data sizes
    let test_sizes = vec![1024, 10240, 102400, 1024000]; // 1KB to 1MB

    for size in test_sizes {
        let key = StorageKey::from_str(&format!("large_data_{}", size))?;
        let large_data = vec![0xAB; size];
        let value = StorageValue::from_bytes(large_data.clone())?;

        let start_time = std::time::Instant::now();
        framework
            .store_test_data(key.clone(), value.clone())
            .await?;
        let store_duration = start_time.elapsed();

        // Verify storage and measure retrieval time
        let retrieve_start = std::time::Instant::now();
        let retrieved_value = framework.nodes[0]
            .retrieve(&key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve large data"))?;
        let retrieve_duration = retrieve_start.elapsed();

        assert_eq!(retrieved_value, value, "Large data corruption detected");

        println!(
            "Size: {}B, Store: {:?}, Retrieve: {:?}",
            size, store_duration, retrieve_duration
        );

        // Performance assertions (adjust based on requirements)
        assert!(
            store_duration.as_millis() < 5000,
            "Store operation too slow"
        );
        assert!(
            retrieve_duration.as_millis() < 1000,
            "Retrieve operation too slow"
        );
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_storage_space_management() -> Result<()> {
    let framework = StorageTestFramework::new(2, 1).await?;
    framework.start_all_nodes().await?;

    // Fill storage near capacity
    let mut stored_keys = Vec::new();
    let chunk_size = 1024 * 1024; // 1MB chunks

    for i in 0..90 {
        // Try to store 90MB (near the 100MB limit)
        let key = StorageKey::from_str(&format!("space_test_{}", i))?;
        let data = vec![i as u8; chunk_size];
        let value = StorageValue::from_bytes(data)?;

        match framework.nodes[0].store(key.clone(), value).await {
            Ok(_) => {
                stored_keys.push(key);
            }
            Err(_) => {
                // Expected when approaching storage limit
                break;
            }
        }

        sleep(Duration::from_millis(10)).await;
    }

    // Verify we stored a reasonable amount
    assert!(
        stored_keys.len() > 50,
        "Should be able to store at least 50MB"
    );
    assert!(
        stored_keys.len() < 100,
        "Should hit storage limits before 100MB"
    );

    // Verify storage stats
    let stats = framework.get_storage_stats().await?;
    for (node_id, record_count, total_size) in stats {
        println!(
            "Node {}: {} records, {} bytes",
            node_id, record_count, total_size
        );
        assert!(
            total_size > 50 * 1024 * 1024,
            "Node should have significant storage"
        );
        assert!(
            total_size <= 100 * 1024 * 1024,
            "Node should respect storage limits"
        );
    }

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_read_write_performance() -> Result<()> {
    let framework = StorageTestFramework::new(3, 2).await?;
    framework.start_all_nodes().await?;

    // Pre-populate with some data
    for i in 0..10 {
        let key = StorageKey::from_str(&format!("perf_test_{}", i))?;
        let value = StorageValue::from_bytes(format!("perf_value_{}", i).into_bytes())?;
        framework.store_test_data(key, value).await?;
    }

    let start_time = std::time::Instant::now();

    // Concurrent read and write operations
    let operations = (0..100).map(|i| {
        let framework = &framework;
        let node_index = i % framework.nodes.len();

        async move {
            if i % 3 == 0 {
                // Write operation
                let key = StorageKey::from_str(&format!("concurrent_perf_{}", i))?;
                let value = StorageValue::from_bytes(format!("value_{}", i).into_bytes())?;
                framework.nodes[node_index].store(key, value).await?;
            } else {
                // Read operation
                let key = StorageKey::from_str(&format!("perf_test_{}", i % 10))?;
                let _ = framework.nodes[node_index].retrieve(&key).await?;
            }

            Result::<()>::Ok(())
        }
    });

    // Execute all operations concurrently
    let results: Result<Vec<_>, _> = futures::future::try_join_all(operations).await;
    results.context("Concurrent operations failed")?;

    let total_duration = start_time.elapsed();
    let ops_per_second = 100.0 / total_duration.as_secs_f64();

    println!("Concurrent performance: {:.2} ops/sec", ops_per_second);
    assert!(
        ops_per_second > 10.0,
        "Performance should be > 10 ops/second"
    );

    framework.shutdown_all().await?;
    Ok(())
}
