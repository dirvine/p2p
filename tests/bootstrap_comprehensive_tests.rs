
//! Comprehensive Bootstrap Cache Tests
//!
//! Focused tests for the bootstrap cache system that work with the current
//! P2P infrastructure, testing cache functionality, persistence, and integration.

use anyhow::Result;
use p2p_foundation::{
    P2PNode, NodeConfig, PeerId, 
    bootstrap::{BootstrapCache, CacheConfig, ContactEntry, BootstrapManager},
    dht::Key,
};
use std::net::SocketAddr;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;
use tracing::{info, debug};

/// Test bootstrap cache basic operations
#[tokio::test]
async fn test_bootstrap_cache_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };

    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config).await?;

    // Test adding contacts
    let contact1 = ContactEntry::new(
        PeerId::from("test_peer_1"),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    
    let mut contact2 = ContactEntry::new(
        PeerId::from("test_peer_2"),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );
    
    // Make contact2 higher quality
    contact2.update_connection_result(true, Some(50), None);
    contact2.update_connection_result(true, Some(60), None);
    contact2.mark_ipv6_verified();

    cache.add_contact(contact1).await?;
    cache.add_contact(contact2).await?;

    // Test retrieving bootstrap peers (should prioritize high quality)
    let bootstrap_peers = cache.get_bootstrap_peers(5).await?;
    assert_eq!(bootstrap_peers.len(), 2);
    
    // First peer should be the higher quality one
    assert_eq!(bootstrap_peers[0].peer_id, "test_peer_2");
    assert!(bootstrap_peers[0].quality_metrics.quality_score > bootstrap_peers[1].quality_metrics.quality_score);
    
    info!("✅ Bootstrap cache operations test passed");
    Ok(())
}

/// Test cache persistence across restarts
#[tokio::test]
async fn test_cache_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };

    // Create cache and add contacts
    {
        let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config.clone()).await?;
        
        let contact = ContactEntry::new(
            PeerId::from("persistent_peer"),
            vec!["/ip4/127.0.0.1/tcp/9000".to_string()]
        );
        
        cache.add_contact(contact).await?;
        cache.save_to_disk().await?;
    }

    // Create new cache instance and verify data persisted
    {
        let cache = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config).await?;
        let bootstrap_peers = cache.get_bootstrap_peers(10).await?;
        
        assert_eq!(bootstrap_peers.len(), 1);
        assert_eq!(bootstrap_peers[0].peer_id, "persistent_peer");
    }

    info!("✅ Cache persistence test passed");
    Ok(())
}

/// Test cache eviction policy
#[tokio::test]
async fn test_cache_eviction() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 5, // Small limit to trigger eviction
        ..CacheConfig::default()
    };

    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config).await?;

    // Add more contacts than the limit
    for i in 0..10 {
        let mut contact = ContactEntry::new(
            PeerId::from(format!("test_peer_{}", i)),
            vec![format!("/ip4/127.0.0.1/tcp/{}", 9000 + i)]
        );
        
        // Make later contacts higher quality
        if i >= 5 {
            contact.update_connection_result(true, Some(50), None);
            contact.mark_ipv6_verified();
        }
        
        cache.add_contact(contact).await?;
    }

    let stats = cache.get_stats().await?;
    assert!(stats.total_contacts <= 5, "Cache should respect max_contacts limit");
    
    // Verify high quality contacts are retained
    let bootstrap_peers = cache.get_bootstrap_peers(5).await?;
    let high_quality_count = bootstrap_peers.iter()
        .filter(|p| p.quality_metrics.quality_score > 0.5)
        .count();
    
    assert!(high_quality_count > 0, "Some high quality contacts should be retained");

    info!("✅ Cache eviction test passed");
    Ok(())
}

/// Test P2P node integration with bootstrap cache
#[tokio::test]
async fn test_p2p_node_bootstrap_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };

    let listen_addr: SocketAddr = "127.0.0.1:19001".parse()?;
    
    let mut config = NodeConfig::default();
    config.listen_addr = listen_addr;
    config.bootstrap_cache_config = Some(cache_config);

    let node = P2PNode::new(config).await?;
    
    // Test adding discovered peers
    node.add_discovered_peer(
        PeerId::from("discovered_peer_1"),
        vec!["127.0.0.1:19002".to_string()]
    ).await?;
    
    node.add_discovered_peer(
        PeerId::from("discovered_peer_2"),
        vec!["127.0.0.1:19003".to_string()]
    ).await?;

    // Wait a bit for cache operations
    sleep(Duration::from_millis(100)).await;

    // Check that contacts were added to cache
    let cached_count = node.cached_peer_count().await;
    assert!(cached_count >= 2, "At least 2 peers should be cached");

    // Test bootstrap cache stats
    if let Ok(Some(stats)) = node.get_bootstrap_cache_stats().await {
        assert!(stats.total_contacts >= 2);
        info!("Cache stats: {} total contacts", stats.total_contacts);
    }

    info!("✅ P2P node bootstrap integration test passed");
    Ok(())
}

/// Test DHT operations with bootstrap cache
#[tokio::test]
async fn test_dht_with_bootstrap_cache() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };

    let listen_addr: SocketAddr = "127.0.0.1:19004".parse()?;
    
    let mut config = NodeConfig::default();
    config.listen_addr = listen_addr;
    config.bootstrap_cache_config = Some(cache_config);
    config.enable_mcp_server = true; // Enable DHT

    let node = P2PNode::new(config).await?;
    node.start().await?;

    // Test DHT operations
    let test_key = Key::new(b"test_key");
    let test_value = b"test_value".to_vec();

    // Store data in DHT
    node.dht_put(test_key.clone(), test_value.clone()).await?;

    // Retrieve data from DHT
    let retrieved = node.dht_get(test_key).await?;
    assert_eq!(retrieved, Some(test_value));

    node.stop().await?;

    info!("✅ DHT with bootstrap cache test passed");
    Ok(())
}

/// Test bootstrap manager functionality
#[tokio::test]
async fn test_bootstrap_manager() -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Create unique temp dir to avoid test interference
    let temp_dir = TempDir::new()?;
    
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 100,
        ..CacheConfig::default()
    };

    let mut manager = BootstrapManager::with_config(cache_config).await?;

    // Test adding contacts
    let contact = ContactEntry::new(
        PeerId::from("manager_test_peer"),
        vec!["/ip4/127.0.0.1/tcp/9000".to_string()]
    );

    manager.add_contact(contact).await?;

    // Test getting bootstrap peers
    let peers = manager.get_bootstrap_peers(5).await?;
    assert!(peers.len() >= 1, "Should have at least 1 peer, got {}", peers.len());
    
    // Verify our specific peer is included
    let has_our_peer = peers.iter().any(|p| p.peer_id == "manager_test_peer");
    assert!(has_our_peer, "Should contain our test peer");

    // Test stats (may include contacts from background initialization)
    let stats = manager.get_stats().await?;
    assert!(stats.total_contacts >= 1, "Should have at least 1 contact, got {}", stats.total_contacts);
    
    // Verify our specific contact is included in the stats
    let bootstrap_peers = manager.get_bootstrap_peers(100).await?;
    let has_our_contact = bootstrap_peers.iter().any(|p| p.peer_id == "manager_test_peer");
    assert!(has_our_contact, "Our contact should be in the bootstrap peers");

    info!("✅ Bootstrap manager test passed");
    Ok(())
}

/// Test contact quality scoring
#[tokio::test]
async fn test_contact_quality_scoring() -> Result<()> {
    let mut low_quality = ContactEntry::new(
        PeerId::from("low_quality"),
        vec!["/ip4/127.0.0.1/tcp/9001".to_string()]
    );
    
    let mut high_quality = ContactEntry::new(
        PeerId::from("high_quality"),
        vec!["/ip4/127.0.0.1/tcp/9002".to_string()]
    );

    // Low quality: failed connections
    low_quality.update_connection_result(false, None, Some("timeout".to_string()));
    low_quality.update_connection_result(false, None, Some("connection_refused".to_string()));

    // High quality: successful connections, low latency, verified
    high_quality.update_connection_result(true, Some(30), None);
    high_quality.update_connection_result(true, Some(40), None);
    high_quality.update_connection_result(true, Some(35), None);
    high_quality.mark_ipv6_verified();
    high_quality.update_capabilities(vec!["dht".to_string(), "mcp".to_string()]);

    assert!(high_quality.quality_metrics.quality_score > low_quality.quality_metrics.quality_score);
    assert!(high_quality.quality_metrics.success_rate > low_quality.quality_metrics.success_rate);
    assert!(high_quality.ipv6_identity_verified);

    info!("Quality scores: low={:.3}, high={:.3}",
          low_quality.quality_metrics.quality_score,
          high_quality.quality_metrics.quality_score);

    info!("✅ Contact quality scoring test passed");
    Ok(())
}

/// Test cache performance with many contacts
#[tokio::test]
async fn test_cache_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 1000,
        ..CacheConfig::default()
    };

    let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), cache_config).await?;

    let start_time = std::time::Instant::now();

    // Add many contacts
    for i in 0..500 {
        let contact = ContactEntry::new(
            PeerId::from(format!("perf_peer_{}", i)),
            vec![format!("/ip4/192.168.1.{}/tcp/9000", i % 255)]
        );
        cache.add_contact(contact).await?;
    }

    let addition_time = start_time.elapsed();
    info!("Added 500 contacts in {:?}", addition_time);

    // Test retrieval performance
    let start_time = std::time::Instant::now();
    
    for _ in 0..100 {
        let _peers = cache.get_bootstrap_peers(20).await?;
    }
    
    let retrieval_time = start_time.elapsed();
    info!("Performed 100 retrieval operations in {:?}", retrieval_time);

    // Performance assertions (generous limits for CI)
    assert!(addition_time.as_secs() < 30, "Contact addition should complete in reasonable time");
    assert!(retrieval_time.as_secs() < 5, "Retrieval should be fast");

    info!("✅ Cache performance test passed");
    Ok(())
}

/// Integration test simulating real-world usage
#[tokio::test]
async fn test_real_world_simulation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        max_contacts: 1000,
        ..CacheConfig::default()
    };

    let listen_addr: SocketAddr = "127.0.0.1:19005".parse()?;
    
    let mut config = NodeConfig::default();
    config.listen_addr = listen_addr;
    config.bootstrap_cache_config = Some(cache_config);

    let node = P2PNode::new(config).await?;
    node.start().await?;

    // Simulate discovering peers over time
    let peers_to_discover = vec![
        ("peer_1", "127.0.0.1:19006"),
        ("peer_2", "127.0.0.1:19007"),
        ("peer_3", "127.0.0.1:19008"),
        ("peer_4", "127.0.0.1:19009"),
    ];

    for (peer_id, addr) in peers_to_discover {
        node.add_discovered_peer(
            PeerId::from(peer_id),
            vec![addr.to_string()]
        ).await?;
        
        // Simulate some connection attempts with varying success
        if peer_id.ends_with('1') || peer_id.ends_with('3') {
            // Simulate successful connections
            node.update_peer_metrics(&PeerId::from(peer_id), true, Some(50), None).await?;
        } else {
            // Simulate failed connections
            node.update_peer_metrics(&PeerId::from(peer_id), false, None, Some("timeout".to_string())).await?;
        }
        
        sleep(Duration::from_millis(10)).await;
    }

    // Wait for cache operations to complete
    sleep(Duration::from_millis(100)).await;

    // Verify cache state
    let cached_count = node.cached_peer_count().await;
    assert!(cached_count >= 4, "All discovered peers should be cached");

    if let Ok(Some(stats)) = node.get_bootstrap_cache_stats().await {
        info!("Final cache stats: {} total, {} high quality", 
              stats.total_contacts, stats.high_quality_contacts);
        assert!(stats.total_contacts >= 4);
    }

    node.stop().await?;

    info!("✅ Real-world simulation test passed");
    Ok(())
}