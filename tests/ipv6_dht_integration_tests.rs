//! IPv6-DHT Integration Tests
//!
//! Comprehensive tests for the integration of IPv6-based node identity system with
//! DHT operations, S/Kademlia security extensions, and IP diversity enforcement.

use anyhow::Result;
use ed25519_dalek::Keypair;
use p2p_foundation::dht::{DHT, DHTConfig, Key};
use p2p_foundation::dht::skademlia::SKademliaConfig;
use p2p_foundation::dht::ipv6_identity::{IPv6DHTConfig, IPv6DHTIdentityManager};
use p2p_foundation::security::{IPv6NodeID, IPDiversityConfig};
use std::net::Ipv6Addr;
use std::str::FromStr;
use std::time::Duration;

/// Helper function to create test IPv6 identity
fn create_test_ipv6_identity(ipv6_addr: Ipv6Addr) -> Result<IPv6NodeID> {
    let mut csprng = rand::rngs::OsRng {};
    let keypair = Keypair::generate(&mut csprng);
    IPv6NodeID::generate(ipv6_addr, &keypair)
}

/// Helper function to create DHT with full IPv6 security
async fn create_ipv6_secure_dht() -> Result<DHT> {
    let local_id = Key::new(b"ipv6_test_node");
    let dht_config = DHTConfig::default();
    let skademlia_config = SKademliaConfig::default();
    let ipv6_config = IPv6DHTConfig::default();
    
    Ok(DHT::new_with_ipv6_security(local_id, dht_config, skademlia_config, ipv6_config))
}

/// Test IPv6 DHT creation and initialization
#[tokio::test]
async fn test_ipv6_dht_creation() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Verify IPv6 identity manager is enabled
    assert!(dht.ipv6_identity_manager.is_some());
    assert!(dht.skademlia.is_some());
    
    // Create and set local IPv6 identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let identity = create_test_ipv6_identity(local_ipv6)?;
    
    let result = dht.set_local_ipv6_identity(identity.clone());
    assert!(result.is_ok());
    
    // Verify local identity is set
    let local_identity = dht.get_local_ipv6_identity();
    assert!(local_identity.is_some());
    assert_eq!(local_identity.unwrap().ipv6_addr, local_ipv6);
    
    Ok(())
}

/// Test IPv6 node addition with verification
#[tokio::test]
async fn test_ipv6_node_addition() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity first
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    // Add IPv6-verified node
    let peer_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7335")?;
    let peer_identity = create_test_ipv6_identity(peer_ipv6)?;
    let peer_id = "ipv6_test_peer_001".to_string();
    let addresses = vec!["127.0.0.1:9001".to_string()];
    
    let result = dht.add_ipv6_node(peer_id.clone(), addresses, peer_identity).await;
    assert!(result.is_ok());
    
    // Verify node is not banned
    assert!(!dht.is_node_banned(&peer_id));
    
    Ok(())
}

/// Test IPv6 node rejection due to invalid identity
#[tokio::test]
async fn test_ipv6_node_rejection() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    // Create invalid identity (corrupted signature)
    let peer_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7335")?;
    let mut invalid_identity = create_test_ipv6_identity(peer_ipv6)?;
    invalid_identity.signature[0] ^= 0xFF; // Corrupt signature
    
    let peer_id = "invalid_ipv6_peer".to_string();
    let addresses = vec!["127.0.0.1:9001".to_string()];
    
    let result = dht.add_ipv6_node(peer_id.clone(), addresses, invalid_identity).await;
    assert!(result.is_err());
    
    Ok(())
}

/// Test IP diversity enforcement
#[tokio::test]
async fn test_ip_diversity_enforcement() -> Result<()> {
    // Create DHT with strict diversity settings
    let local_id = Key::new(b"diversity_test_node");
    let dht_config = DHTConfig::default();
    let skademlia_config = SKademliaConfig::default();
    let ipv6_config = IPv6DHTConfig {
        diversity_config: IPDiversityConfig {
            max_nodes_per_64: 1,
            max_nodes_per_48: 2,
            ..IPDiversityConfig::default()
        },
        ..IPv6DHTConfig::default()
    };
    
    let mut dht = DHT::new_with_ipv6_security(local_id, dht_config, skademlia_config, ipv6_config);
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    // Add first node in /64 subnet
    let peer1_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:1234:0000:8a2e:0370:7335")?;
    let peer1_identity = create_test_ipv6_identity(peer1_ipv6)?;
    let peer1_id = "diversity_peer_001".to_string();
    let addresses1 = vec!["127.0.0.1:9001".to_string()];
    
    let result1 = dht.add_ipv6_node(peer1_id.clone(), addresses1, peer1_identity).await;
    assert!(result1.is_ok());
    
    // Try to add second node in same /64 subnet (should fail)
    let peer2_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:1234:0000:8a2e:0370:7336")?;
    let peer2_identity = create_test_ipv6_identity(peer2_ipv6)?;
    let peer2_id = "diversity_peer_002".to_string();
    let addresses2 = vec!["127.0.0.1:9002".to_string()];
    
    let result2 = dht.add_ipv6_node(peer2_id.clone(), addresses2, peer2_identity).await;
    assert!(result2.is_err());
    
    // Add node in different /64 subnet (should succeed)
    let peer3_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:5678:0000:8a2e:0370:7337")?;
    let peer3_identity = create_test_ipv6_identity(peer3_ipv6)?;
    let peer3_id = "diversity_peer_003".to_string();
    let addresses3 = vec!["127.0.0.1:9003".to_string()];
    
    let result3 = dht.add_ipv6_node(peer3_id.clone(), addresses3, peer3_identity).await;
    assert!(result3.is_ok());
    
    Ok(())
}

/// Test IPv6-enhanced secure get operation
#[tokio::test]
async fn test_ipv6_secure_get() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    let key = Key::new(b"ipv6_secure_test_key");
    
    // Test get on empty DHT
    let result = dht.ipv6_secure_get(&key).await?;
    assert!(result.is_none());
    
    // Store a value first
    let test_value = b"ipv6_secure_test_value".to_vec();
    dht.ipv6_secure_put(key.clone(), test_value.clone()).await?;
    
    // Now secure get should find it
    let result = dht.ipv6_secure_get(&key).await?;
    assert!(result.is_some());
    
    let record = result.unwrap();
    assert_eq!(record.key, key);
    assert_eq!(record.value, test_value);
    
    Ok(())
}

/// Test IPv6-enhanced secure put operation
#[tokio::test]
async fn test_ipv6_secure_put() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    let key = Key::new(b"ipv6_secure_put_key");
    let test_value = b"ipv6_secure_put_value".to_vec();
    
    // Should succeed even with no other nodes
    let result = dht.ipv6_secure_put(key.clone(), test_value.clone()).await;
    assert!(result.is_ok());
    
    // Verify the value was stored locally
    let retrieved = dht.get(&key).await;
    assert!(retrieved.is_some());
    
    let record = retrieved.unwrap();
    assert_eq!(record.value, test_value);
    
    Ok(())
}

/// Test node banning and reputation system
#[tokio::test]
async fn test_ipv6_node_banning() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    let peer_id = "test_ban_peer".to_string();
    
    // Initially node should not be banned
    assert!(!dht.is_node_banned(&peer_id));
    
    // Ban the node
    dht.ban_ipv6_node(&peer_id, "Test ban reason");
    
    // Now node should be banned
    assert!(dht.is_node_banned(&peer_id));
    
    Ok(())
}

/// Test DHT key generation from IPv6 identity
#[tokio::test]
async fn test_dht_key_generation() -> Result<()> {
    let ipv6_addr = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let identity = create_test_ipv6_identity(ipv6_addr)?;
    
    // Generate DHT key from IPv6 identity
    let dht_key = IPv6DHTIdentityManager::generate_dht_key(&identity);
    
    // Verify key is derived from identity
    let expected_key = Key::from_hash(
        identity.node_id.as_slice()
            .try_into()
            .unwrap_or([0u8; 32])
    );
    
    assert_eq!(dht_key.as_bytes(), expected_key.as_bytes());
    
    Ok(())
}

/// Test IPv6 diversity statistics
#[tokio::test]
async fn test_ipv6_diversity_statistics() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    // Initially no diversity stats
    let initial_stats = dht.get_ipv6_diversity_stats();
    assert!(initial_stats.is_some());
    let stats = initial_stats.unwrap();
    assert_eq!(stats.total_64_subnets, 0);
    
    // Add some nodes from different subnets
    let addresses = vec!["127.0.0.1:9001".to_string()];
    
    let peer1_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:1234:0000:8a2e:0370:7335")?;
    let peer1_identity = create_test_ipv6_identity(peer1_ipv6)?;
    dht.add_ipv6_node("stats_peer_001".to_string(), addresses.clone(), peer1_identity).await?;
    
    let peer2_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:5678:0000:8a2e:0370:7336")?;
    let peer2_identity = create_test_ipv6_identity(peer2_ipv6)?;
    dht.add_ipv6_node("stats_peer_002".to_string(), addresses.clone(), peer2_identity).await?;
    
    // Check updated stats
    let updated_stats = dht.get_ipv6_diversity_stats();
    assert!(updated_stats.is_some());
    let stats = updated_stats.unwrap();
    assert!(stats.total_64_subnets > 0);
    
    Ok(())
}

/// Test IPv6 identity refresh and cleanup
#[tokio::test]
async fn test_ipv6_identity_refresh() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    // Cleanup should not fail
    dht.cleanup_ipv6_data();
    
    // IPv6 identity manager should still be available
    assert!(dht.ipv6_identity_manager.is_some());
    
    Ok(())
}

/// Test node removal with IPv6 cleanup
#[tokio::test]
async fn test_ipv6_node_removal() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    // Add IPv6 node
    let peer_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7335")?;
    let peer_identity = create_test_ipv6_identity(peer_ipv6)?;
    let peer_id = "removal_test_peer".to_string();
    let addresses = vec!["127.0.0.1:9001".to_string()];
    
    dht.add_ipv6_node(peer_id.clone(), addresses, peer_identity).await?;
    
    // Remove the node
    let result = dht.remove_ipv6_node(&peer_id).await;
    assert!(result.is_ok());
    
    Ok(())
}

/// Test error handling with IPv6 security
#[tokio::test]
async fn test_ipv6_security_error_handling() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Try to set invalid local identity
    let invalid_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let mut invalid_identity = create_test_ipv6_identity(invalid_ipv6)?;
    invalid_identity.signature[0] ^= 0xFF; // Corrupt signature
    
    let result = dht.set_local_ipv6_identity(invalid_identity);
    assert!(result.is_err());
    
    Ok(())
}

/// Performance test for IPv6-enhanced operations
#[tokio::test]
async fn test_ipv6_operations_performance() -> Result<()> {
    let mut dht = create_ipv6_secure_dht().await?;
    
    // Set local identity
    let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    let local_identity = create_test_ipv6_identity(local_ipv6)?;
    dht.set_local_ipv6_identity(local_identity)?;
    
    use std::time::Instant;
    
    let start = Instant::now();
    
    // Perform multiple IPv6 secure operations
    for i in 0..10 {
        let key = Key::new(format!("ipv6_perf_key_{}", i).as_bytes());
        let value = format!("ipv6_perf_value_{}", i).into_bytes();
        dht.ipv6_secure_put(key.clone(), value).await?;
        let _result = dht.ipv6_secure_get(&key).await?;
    }
    
    let duration = start.elapsed();
    
    // Should complete within reasonable time
    assert!(duration < Duration::from_secs(2));
    
    println!("IPv6 operations for 10 records took: {:?}", duration);
    
    Ok(())
}

/// Test concurrent IPv6 operations
#[tokio::test]
async fn test_concurrent_ipv6_operations() -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let dht = Arc::new(Mutex::new(create_ipv6_secure_dht().await?));
    
    // Set local identity
    {
        let mut dht_lock = dht.lock().await;
        let local_ipv6 = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
        let local_identity = create_test_ipv6_identity(local_ipv6)?;
        dht_lock.set_local_ipv6_identity(local_identity)?;
    }
    
    let mut handles = Vec::new();
    
    // Spawn concurrent IPv6 secure put operations
    for i in 0..5 {
        let dht_clone = dht.clone();
        let handle = tokio::spawn(async move {
            let mut dht = dht_clone.lock().await;
            let key = Key::new(format!("concurrent_ipv6_key_{}", i).as_bytes());
            let value = format!("concurrent_ipv6_value_{}", i).into_bytes();
            dht.ipv6_secure_put(key, value).await
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