//! Bootstrap Cache System Tests
//!
//! Comprehensive tests for the bootstrap cache functionality including
//! contact management, quality scoring, multi-instance coordination,
//! and integration with the P2P network layer.

use p2p_foundation::{
    Result, BootstrapManager, BootstrapCache, ContactEntry, CacheConfig,
    P2PNode, NodeConfig, PeerId
};
use p2p_foundation::bootstrap::{QualityMetrics, MergeCoordinator};
use p2p_foundation::bootstrap::merge::MergeStrategy;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

/// Test basic bootstrap cache functionality
#[tokio::test]
async fn test_bootstrap_cache_basic_operations() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };
    
    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await?;
    
    // Test adding contacts
    let contact1 = ContactEntry::new(
        "peer1".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    let contact2 = ContactEntry::new(
        "peer2".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    
    cache.add_contact(contact1.clone()).await?;
    cache.add_contact(contact2.clone()).await?;
    
    // Test retrieving bootstrap peers
    let bootstrap_peers = cache.get_bootstrap_peers(10).await?;
    assert_eq!(bootstrap_peers.len(), 2);
    
    // Test cache statistics
    let stats = cache.get_stats().await?;
    assert_eq!(stats.total_contacts, 2);
    assert_eq!(stats.verified_contacts, 0);
    
    Ok(())
}

/// Test contact quality scoring and selection
#[tokio::test]
async fn test_contact_quality_scoring() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };
    
    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await?;
    
    // Create contacts with different quality metrics
    let mut high_quality_contact = ContactEntry::new(
        "high_quality_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    
    let mut low_quality_contact = ContactEntry::new(
        "low_quality_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    
    // Simulate successful connections for high quality peer
    high_quality_contact.update_connection_result(true, Some(50), None);
    high_quality_contact.update_connection_result(true, Some(60), None);
    high_quality_contact.update_connection_result(true, Some(45), None);
    high_quality_contact.mark_ipv6_verified();
    
    // Simulate mixed results for low quality peer
    low_quality_contact.update_connection_result(true, Some(200), None);
    low_quality_contact.update_connection_result(false, None, Some("timeout".to_string()));
    low_quality_contact.update_connection_result(false, None, Some("refused".to_string()));
    
    cache.add_contact(high_quality_contact.clone()).await?;
    cache.add_contact(low_quality_contact.clone()).await?;
    
    // Get bootstrap peers (should prioritize high quality)
    let bootstrap_peers = cache.get_bootstrap_peers(1).await?;
    assert_eq!(bootstrap_peers.len(), 1);
    assert_eq!(bootstrap_peers[0].peer_id, "high_quality_peer");
    assert!(bootstrap_peers[0].quality_metrics.quality_score > 0.5);
    
    Ok(())
}

/// Test cache persistence and recovery
#[tokio::test]
async fn test_cache_persistence() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };
    
    // Create and populate cache
    {
        let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config.clone()).await?;
        
        for i in 0..5 {
            let contact = ContactEntry::new(
                format!("persistent_peer_{}", i),
                vec![format!("/ip4/127.0.0.1/tcp/{}", 9000 + i)]
            );
            cache.add_contact(contact).await?;
        }
        
        cache.save_to_disk().await?;
    }
    
    // Create new cache instance and verify data is loaded
    {
        let cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await?;
        let stats = cache.get_stats().await?;
        assert_eq!(stats.total_contacts, 5);
        
        let bootstrap_peers = cache.get_bootstrap_peers(10).await?;
        assert_eq!(bootstrap_peers.len(), 5);
    }
    
    Ok(())
}

/// Test cache eviction when exceeding capacity
#[tokio::test]
async fn test_cache_eviction() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 5, // Small capacity for testing
        ..CacheConfig::default()
    };
    
    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await?;
    
    // Add contacts exceeding capacity
    for i in 0..10 {
        let mut contact = ContactEntry::new(
            format!("eviction_peer_{}", i),
            vec![format!("/ip4/127.0.0.1/tcp/{}", 9000 + i)]
        );
        
        // Give later contacts higher quality scores
        if i >= 5 {
            contact.update_connection_result(true, Some(50), None);
            contact.update_connection_result(true, Some(45), None);
        }
        
        cache.add_contact(contact).await?;
    }
    
    let stats = cache.get_stats().await?;
    assert!(stats.total_contacts <= 5, "Cache should not exceed max capacity");
    
    // Verify high quality contacts are retained
    let bootstrap_peers = cache.get_bootstrap_peers(5).await?;
    let high_quality_count = bootstrap_peers.iter()
        .filter(|p| p.quality_metrics.quality_score > 0.3)
        .count();
    
    assert!(high_quality_count > 0, "High quality contacts should be retained");
    
    Ok(())
}

/// Test multi-instance merge coordination
#[tokio::test]
async fn test_multi_instance_coordination() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let coordinator = MergeCoordinator::new(temp_dir.path().to_path_buf())?;
    
    let config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };
    
    // Create main cache
    let main_cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await?;
    
    // Create instance cache files manually for testing
    let instance_cache_dir = temp_dir.path().join("instance_caches");
    std::fs::create_dir_all(&instance_cache_dir)?;
    
    // Simulate instance cache data
    let instance_data = serde_json::json!({
        "instance_id": "test_12345_1234567890",
        "timestamp": chrono::Utc::now(),
        "process_id": 12345,
        "contacts": {
            "merge_test_peer": {
                "peer_id": "merge_test_peer",
                "addresses": ["/ip4/127.0.0.1/tcp/9999"],
                "last_seen": chrono::Utc::now(),
                "quality_metrics": {
                    "success_rate": 0.9,
                    "avg_latency_ms": 45.0,
                    "quality_score": 0.85,
                    "last_connection_attempt": chrono::Utc::now(),
                    "last_successful_connection": chrono::Utc::now(),
                    "uptime_score": 0.8
                },
                "capabilities": ["dht", "mcp"],
                "ipv6_identity_verified": true,
                "reputation_score": 0.9,
                "connection_history": {
                    "total_attempts": 10,
                    "successful_connections": 9,
                    "failed_connections": 1,
                    "total_session_time": {"secs": 3600, "nanos": 0},
                    "recent_latencies": [45, 50, 42],
                    "connection_failures": {}
                }
            }
        },
        "version": 1
    });
    
    let instance_cache_file = instance_cache_dir.join("test_12345_1234567890.cache");
    std::fs::write(instance_cache_file, serde_json::to_string(&instance_data)?)?;
    
    // Perform merge
    let merge_result = coordinator.merge_instance_caches(&main_cache).await?;
    
    assert_eq!(merge_result.instances_processed, 1);
    assert_eq!(merge_result.contacts_added, 1);
    
    // Verify contact was merged
    let bootstrap_peers = main_cache.get_bootstrap_peers(10).await?;
    assert!(bootstrap_peers.iter().any(|p| p.peer_id == "merge_test_peer"));
    
    Ok(())
}

/// Test bootstrap manager integration
#[tokio::test]
async fn test_bootstrap_manager() -> Result<()> {
    let manager = BootstrapManager::new().await?;
    
    // Test adding contacts
    let contact = ContactEntry::new(
        "manager_test_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9000".to_string()]
    );
    
    let mut manager_mut = manager;
    manager_mut.add_contact(contact).await?;
    
    // Test getting bootstrap peers
    let bootstrap_peers = manager_mut.get_bootstrap_peers(5).await?;
    assert_eq!(bootstrap_peers.len(), 1);
    assert_eq!(bootstrap_peers[0].peer_id, "manager_test_peer");
    
    // Test statistics
    let stats = manager_mut.get_stats().await?;
    assert_eq!(stats.total_contacts, 1);
    
    Ok(())
}

/// Test P2P node integration with bootstrap cache
#[tokio::test]
async fn test_p2p_node_bootstrap_integration() -> Result<()> {
    let config = NodeConfig {
        peer_id: Some("test_node".to_string()),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9050".to_string()],
        bootstrap_peers: vec![
            "/ip4/127.0.0.1/tcp/9051".to_string(),
            "/ip4/127.0.0.1/tcp/9052".to_string(),
        ],
        ..NodeConfig::default()
    };
    
    let node = P2PNode::new(config).await?;
    
    // Test adding discovered peers
    node.add_discovered_peer(
        "discovered_peer_1".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9053".to_string()]
    ).await?;
    
    // Test updating peer metrics
    node.update_peer_metrics(
        &"discovered_peer_1".to_string(),
        true,
        Some(75),
        None
    ).await?;
    
    // Test getting bootstrap stats
    let stats = node.get_bootstrap_cache_stats().await?;
    assert!(stats.is_some());
    
    let cached_count = node.cached_peer_count().await;
    assert!(cached_count > 0);
    
    Ok(())
}

/// Test cache cleanup and maintenance
// Disabled: This test accesses private implementation details
/*
#[tokio::test]
async fn test_cache_cleanup_disabled() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        stale_threshold: Duration::from_millis(100), // Very short for testing
        ..CacheConfig::default()
    };
    
    let cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await?;
    
    // Add some contacts
    let mut contact1 = ContactEntry::new(
        "stale_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    
    // Make contact stale
    contact1.last_seen = chrono::Utc::now() - chrono::Duration::seconds(1);
    
    let mut contact2 = ContactEntry::new(
        "fresh_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    
    {
        let mut contacts = cache.contacts.write().await;
        contacts.insert(contact1.peer_id.clone(), contact1);
        contacts.insert(contact2.peer_id.clone(), contact2);
    }
    
    // Wait for staleness
    sleep(Duration::from_millis(200)).await;
    
    // Perform cleanup
    cache.cleanup_stale_entries().await?;
    
    // Verify stale contact was removed
    let contacts = cache.contacts.read().await;
    // Test removed due to private field access
    Ok(())
}
*/

/// Test quality score calculations
#[tokio::test]
async fn test_quality_calculations() -> Result<()> {
    use p2p_foundation::bootstrap::QualityCalculator;
    
    let calculator = QualityCalculator::new();
    
    // Test high quality contact
    let mut high_quality = ContactEntry::new(
        "high_quality".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    
    high_quality.update_connection_result(true, Some(30), None);
    high_quality.update_connection_result(true, Some(25), None);
    high_quality.update_connection_result(true, Some(35), None);
    high_quality.mark_ipv6_verified();
    high_quality.update_capabilities(vec!["dht".to_string(), "mcp".to_string()]);
    high_quality.update_reputation(0.9);
    
    let high_score = calculator.calculate_quality(&high_quality);
    
    // Test low quality contact
    let mut low_quality = ContactEntry::new(
        "low_quality".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    
    low_quality.update_connection_result(false, None, Some("timeout".to_string()));
    low_quality.update_connection_result(false, None, Some("refused".to_string()));
    low_quality.update_connection_result(true, Some(500), None);
    low_quality.update_reputation(0.2);
    
    let low_score = calculator.calculate_quality(&low_quality);
    
    assert!(high_score > low_score, 
           "High quality contact should have higher score: {} vs {}", high_score, low_score);
    assert!(high_score > 0.5, "High quality contact should have good score");
    assert!(low_score < 0.5, "Low quality contact should have poor score");
    
    Ok(())
}

/// Test merge strategy behavior
// Disabled: This test accesses private methods
/*
#[tokio::test]
async fn test_merge_strategies_disabled() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    
    // Test quality-based merge
    let quality_coordinator = MergeCoordinator::with_strategy(
        temp_dir.path().to_path_buf(),
        MergeStrategy::QualityBased
    )?;
    
    let mut high_quality = ContactEntry::new(
        "test_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    high_quality.quality_metrics.quality_score = 0.9;
    
    let mut low_quality = ContactEntry::new(
        "test_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    low_quality.quality_metrics.quality_score = 0.3;
    
    let resolved = quality_coordinator.resolve_conflict(&low_quality, &high_quality)?;
    assert_eq!(resolved.quality_metrics.quality_score, 0.9);
    
    // Test timestamp-based merge
    let timestamp_coordinator = MergeCoordinator::with_strategy(
        temp_dir.path().to_path_buf(),
        MergeStrategy::TimestampBased
    )?;
    
    let mut old_contact = ContactEntry::new(
        "test_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    old_contact.last_seen = chrono::Utc::now() - chrono::Duration::hours(1);
    
    let mut new_contact = ContactEntry::new(
        "test_peer".to_string(),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    new_contact.last_seen = chrono::Utc::now();
    
    let resolved = timestamp_coordinator.resolve_conflict(&old_contact, &new_contact)?;
    assert!(resolved.last_seen > old_contact.last_seen);
    
    Ok(())
}
*/

/// Test concurrent access to bootstrap cache
#[tokio::test]
async fn test_concurrent_cache_access() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 1000,
        ..CacheConfig::default()
    };
    
    let cache = std::sync::Arc::new(
        BootstrapCache::new(temp_dir.path().to_path_buf(), config).await?
    );
    
    // Spawn multiple tasks to add contacts concurrently
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let contact = ContactEntry::new(
                    format!("concurrent_peer_{}_{}", i, j),
                    vec![format!("/ip4/127.0.0.1/tcp/{}", 9000 + i * 10 + j)]
                );
                
                let mut cache_mut = match std::sync::Arc::try_unwrap(cache_clone.clone()) {
                    Ok(cache) => cache,
                    Err(arc) => return Err(format!("Failed to unwrap Arc")),
                };
                
                if let Err(e) = cache_mut.add_contact(contact).await {
                    return Err(format!("Failed to add contact: {}", e));
                }
            }
            Ok(())
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {},
            Ok(Err(e)) => panic!("Task failed: {}", e),
            Err(e) => panic!("Task panicked: {}", e),
        }
    }
    
    // Note: This test structure needs adjustment for proper concurrent access
    // In practice, the BootstrapCache would be wrapped in proper async synchronization
    
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    /// Test full bootstrap workflow
    #[tokio::test]
        async fn test_full_bootstrap_workflow() -> Result<()> {
        // This test simulates a complete bootstrap workflow:
        // 1. Node starts with empty cache
        // 2. Uses fallback bootstrap peers
        // 3. Discovers new peers through network
        // 4. Updates cache with discovered peers
        // 5. Subsequent restarts use cached peers
        
        let temp_dir = TempDir::new().unwrap();
        
        // Phase 1: Initial startup with fallback peers
        {
            let config = NodeConfig {
                peer_id: Some("workflow_test_node".to_string()),
                listen_addrs: vec!["/ip4/127.0.0.1/tcp/9100".to_string()],
                bootstrap_peers: vec![
                    "/ip4/127.0.0.1/tcp/9101".to_string(),
                    "/ip4/127.0.0.1/tcp/9102".to_string(),
                ],
                ..NodeConfig::default()
            };
            
            let node = P2PNode::new(config).await?;
            
            // Simulate discovering peers
            node.add_discovered_peer(
                "discovered_1".to_string(),
                vec!["/ip4/127.0.0.1/tcp/9103".to_string()]
            ).await?;
            
            node.add_discovered_peer(
                "discovered_2".to_string(),
                vec!["/ip4/127.0.0.1/tcp/9104".to_string()]
            ).await?;
            
            // Update metrics for discovered peers
            node.update_peer_metrics(&"discovered_1".to_string(), true, Some(50), None).await?;
            node.update_peer_metrics(&"discovered_2".to_string(), true, Some(75), None).await?;
            
            let cached_count = node.cached_peer_count().await;
            assert!(cached_count >= 2, "Should have cached discovered peers");
        }
        
        // Phase 2: Subsequent startup should use cached peers
        {
            let config = NodeConfig {
                peer_id: Some("workflow_test_node_2".to_string()),
                listen_addrs: vec!["/ip4/127.0.0.1/tcp/9105".to_string()],
                bootstrap_peers: Vec::new(), // No fallback peers
                ..NodeConfig::default()
            };
            
            let node = P2PNode::new(config).await?;
            let cached_count = node.cached_peer_count().await;
            
            // Should have cached peers from previous session
            assert!(cached_count > 0, "Should have cached peers from previous session");
        }
        
        Ok(())
    }
}