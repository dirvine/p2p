//! S/Kademlia DHT Integration Tests
//!
//! Tests the integration between S/Kademlia security extensions and the main DHT implementation.
//! Covers secure get/put operations, reputation management, and security-aware node selection.

use anyhow::Result;
use p2p_foundation::dht::{DHT, DHTConfig, Key};
use p2p_foundation::dht::skademlia::SKademliaConfig;
use std::time::Duration;

/// Helper function to create test DHT with S/Kademlia
async fn create_secure_dht() -> DHT {
    let local_id = Key::new(b"test_local_node");
    let dht_config = DHTConfig::default();
    let skademlia_config = SKademliaConfig::default();
    
    DHT::new_with_security(local_id, dht_config, skademlia_config)
}

/// Helper function to create regular DHT
async fn create_regular_dht() -> DHT {
    let local_id = Key::new(b"test_local_node");
    let dht_config = DHTConfig::default();
    
    DHT::new(local_id, dht_config)
}

/// Test creating DHT with S/Kademlia security extensions
#[tokio::test]
async fn test_secure_dht_creation() -> Result<()> {
    let secure_dht = create_secure_dht().await;
    
    // Verify S/Kademlia is enabled
    assert!(secure_dht.skademlia.is_some());
    
    // Verify configuration
    let skademlia = secure_dht.skademlia.as_ref().unwrap();
    assert_eq!(skademlia.config.disjoint_path_count, 3);
    assert_eq!(skademlia.config.max_shared_nodes, 1);
    assert!(skademlia.config.enable_distance_verification);
    
    Ok(())
}

/// Test creating regular DHT without S/Kademlia
#[tokio::test]
async fn test_regular_dht_creation() -> Result<()> {
    let regular_dht = create_regular_dht().await;
    
    // Verify S/Kademlia is disabled
    assert!(regular_dht.skademlia.is_none());
    
    Ok(())
}

/// Test secure get operation with disjoint paths
#[tokio::test]
async fn test_secure_get_operation() -> Result<()> {
    let mut secure_dht = create_secure_dht().await;
    
    let key = Key::new(b"test_secure_key");
    
    // Test secure get on empty DHT
    let result = secure_dht.secure_get(&key).await?;
    assert!(result.is_none());
    
    // Store a value first using regular put
    let test_value = b"test_secure_value".to_vec();
    secure_dht.put(key.clone(), test_value.clone()).await?;
    
    // Now secure get should find it locally
    let result = secure_dht.secure_get(&key).await?;
    assert!(result.is_some());
    
    let record = result.unwrap();
    assert_eq!(record.key, key);
    assert_eq!(record.value, test_value);
    
    Ok(())
}

/// Test secure put operation with reputation-based node selection
#[tokio::test]
async fn test_secure_put_operation() -> Result<()> {
    let mut secure_dht = create_secure_dht().await;
    
    let key = Key::new(b"test_secure_put_key");
    let test_value = b"test_secure_put_value".to_vec();
    
    // Should succeed even with no other nodes
    let result = secure_dht.secure_put(key.clone(), test_value.clone()).await;
    assert!(result.is_ok());
    
    // Verify the value was stored locally
    let retrieved = secure_dht.get(&key).await;
    assert!(retrieved.is_some());
    
    let record = retrieved.unwrap();
    assert_eq!(record.value, test_value);
    
    Ok(())
}

/// Test sibling list management
#[tokio::test]
async fn test_sibling_list_management() -> Result<()> {
    let mut secure_dht = create_secure_dht().await;
    
    let key = Key::new(b"sibling_test_key");
    
    // Update sibling list (should work even with no nodes)
    let result = secure_dht.update_sibling_list(key).await;
    assert!(result.is_ok());
    
    Ok(())
}

/// Test routing consistency validation
#[tokio::test]
async fn test_routing_consistency_validation() -> Result<()> {
    let secure_dht = create_secure_dht().await;
    
    // Validate routing consistency
    let report = secure_dht.validate_routing_consistency().await?;
    
    // With no nodes, should have zero nodes checked
    assert_eq!(report.nodes_checked, 0);
    assert_eq!(report.inconsistencies, 0);
    assert!(report.suspicious_nodes.is_empty());
    
    Ok(())
}

/// Test distance verification challenge creation
#[tokio::test]
async fn test_distance_verification() -> Result<()> {
    let mut secure_dht = create_secure_dht().await;
    
    let peer_id = "test_peer_123".to_string();
    let key = Key::new(b"distance_test_key");
    
    // Create distance challenge
    let challenge = secure_dht.create_distance_challenge(&peer_id, &key);
    assert!(challenge.is_some());
    
    let challenge = challenge.unwrap();
    assert_eq!(challenge.challenger, peer_id);
    assert_eq!(challenge.target_key, key);
    assert_eq!(challenge.nonce.len(), 32);
    
    Ok(())
}

/// Test security bucket management
#[tokio::test]
async fn test_security_bucket_management() -> Result<()> {
    let mut secure_dht = create_secure_dht().await;
    
    let key = Key::new(b"security_bucket_key");
    let peer_id = "trusted_peer_456".to_string();
    let addresses = vec!["127.0.0.1:9001".to_string()];
    
    // Add trusted node to security bucket
    let result = secure_dht.add_trusted_node(&key, peer_id, addresses).await;
    assert!(result.is_ok());
    
    // Get security bucket
    let bucket = secure_dht.get_security_bucket(&key);
    assert!(bucket.is_some());
    
    let bucket = bucket.unwrap();
    assert_eq!(bucket.trusted_nodes.len(), 1);
    
    Ok(())
}

/// Test fallback behavior when S/Kademlia is disabled
#[tokio::test]
async fn test_fallback_to_regular_operations() -> Result<()> {
    let regular_dht = create_regular_dht().await;
    
    // These should fail gracefully for regular DHT
    let result = regular_dht.validate_routing_consistency().await;
    assert!(result.is_err());
    
    Ok(())
}

/// Test S/Kademlia maintenance operations
#[tokio::test]
async fn test_skademlia_maintenance() -> Result<()> {
    let secure_dht = create_secure_dht().await;
    
    // Run maintenance (should include S/Kademlia cleanup)
    let result = secure_dht.maintenance().await;
    assert!(result.is_ok());
    
    Ok(())
}

/// Test error handling in secure operations
#[tokio::test]
async fn test_secure_operations_error_handling() -> Result<()> {
    let mut secure_dht = create_secure_dht().await;
    
    // Test with invalid key scenarios
    let key = Key::new(b"error_test_key");
    
    // Secure get should handle missing keys gracefully
    let result = secure_dht.secure_get(&key).await?;
    assert!(result.is_none());
    
    // Secure put should handle errors gracefully
    let empty_value = Vec::new();
    let result = secure_dht.secure_put(key, empty_value).await;
    assert!(result.is_ok());
    
    Ok(())
}

/// Test S/Kademlia configuration validation
#[tokio::test]
async fn test_skademlia_configuration() -> Result<()> {
    let local_id = Key::new(b"config_test_node");
    let dht_config = DHTConfig::default();
    
    // Test custom S/Kademlia configuration
    let custom_config = SKademliaConfig {
        disjoint_path_count: 5,
        max_shared_nodes: 2,
        sibling_list_size: 20,
        security_bucket_size: 10,
        enable_distance_verification: false,
        enable_routing_validation: false,
        min_routing_reputation: 0.5,
        lookup_timeout: Duration::from_secs(60),
    };
    
    let secure_dht = DHT::new_with_security(local_id, dht_config, custom_config);
    
    // Verify custom configuration is applied
    let skademlia = secure_dht.skademlia.as_ref().unwrap();
    assert_eq!(skademlia.config.disjoint_path_count, 5);
    assert_eq!(skademlia.config.max_shared_nodes, 2);
    assert_eq!(skademlia.config.sibling_list_size, 20);
    assert!(!skademlia.config.enable_distance_verification);
    
    Ok(())
}

/// Performance test for secure DHT operations
#[tokio::test]
async fn test_secure_operations_performance() -> Result<()> {
    let mut secure_dht = create_secure_dht().await;
    
    use std::time::Instant;
    
    // Test secure put performance
    let start = Instant::now();
    
    for i in 0..10 {
        let key = Key::new(format!("perf_test_key_{}", i).as_bytes());
        let value = format!("perf_test_value_{}", i).into_bytes();
        secure_dht.secure_put(key, value).await?;
    }
    
    let put_duration = start.elapsed();
    
    // Should be reasonable performance (less than 1 second for 10 operations)
    assert!(put_duration < Duration::from_secs(1));
    
    // Test secure get performance
    let start = Instant::now();
    
    for i in 0..10 {
        let key = Key::new(format!("perf_test_key_{}", i).as_bytes());
        let _result = secure_dht.secure_get(&key).await?;
    }
    
    let get_duration = start.elapsed();
    
    // Should be reasonable performance
    assert!(get_duration < Duration::from_secs(1));
    
    Ok(())
}

/// Test concurrent secure operations
#[tokio::test]
async fn test_concurrent_secure_operations() -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let secure_dht = Arc::new(Mutex::new(create_secure_dht().await));
    let mut handles = Vec::new();
    
    // Spawn concurrent secure put operations
    for i in 0..5 {
        let dht_clone = secure_dht.clone();
        let handle = tokio::spawn(async move {
            let mut dht = dht_clone.lock().await;
            let key = Key::new(format!("concurrent_key_{}", i).as_bytes());
            let value = format!("concurrent_value_{}", i).into_bytes();
            dht.secure_put(key, value).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }
    
    Ok(())
}

/// Test S/Kademlia statistics and metrics
#[tokio::test]
async fn test_skademlia_statistics() -> Result<()> {
    let secure_dht = create_secure_dht().await;
    
    // Get DHT statistics
    let stats = secure_dht.stats().await;
    
    // Should have basic stats structure
    assert_eq!(stats.total_nodes, 0); // No nodes added yet
    assert_eq!(stats.stored_records, 0); // No records stored yet
    
    Ok(())
}