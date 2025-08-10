// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Integration tests for Four-Word DNS system

use anyhow::Result;
use communitas_cli::dns::{
    FourWordProfile, ProfileValidator, DHTProfileStorage, ProfileResolver,
    ProfilePacket, ResolutionQuery, BatchResolutionRequest,
};
use communitas_cli::identity::FourWordAddress;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_end_to_end_dns_resolution() -> Result<()> {
    let _temp_dir = tempdir()?;
    let storage = Arc::new(DHTProfileStorage::new());
    let resolver = ProfileResolver::new(storage.clone());
    
    // Create a profile
    let four_words = FourWordAddress::generate()?;
    let mut profile = FourWordProfile::new(four_words.clone())
        .with_website("# Test Website".to_string())
        .with_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string());
    
    // Sign the profile
    let private_key = vec![0x42u8; 32];
    profile.sign(&private_key)?;
    
    // Store in DHT
    let packet = ProfilePacket::new(four_words.clone(), profile.clone())?;
    let _dht_hash = storage.store_packet(packet).await?;
    
    // Resolve the profile
    let result = resolver.resolve(&four_words).await?;
    assert!(result.profile.is_some());
    assert_eq!(result.four_words, four_words);
    
    // Verify resolved profile matches
    let resolved_profile = result.profile.unwrap();
    assert_eq!(resolved_profile.get_website(), Some("# Test Website"));
    assert_eq!(resolved_profile.get_bitcoin_address(), Some("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"));
    
    Ok(())
}

#[tokio::test]
async fn test_profile_validation_and_security() -> Result<()> {
    let validator = ProfileValidator::new();
    let four_words = FourWordAddress::generate()?;
    
    // Test valid profile
    let valid_profile = FourWordProfile::new(four_words.clone())
        .with_website("# Valid Website".to_string());
    
    let result = validator.validate_profile(&valid_profile)?;
    assert!(result.is_valid || result.warnings.len() > 0); // May have warnings but should be valid
    
    // Test malicious content detection
    let malicious_profile = FourWordProfile::new(four_words.clone())
        .with_website("<script>alert('xss')</script>".to_string());
    
    let result = validator.check_malicious_content(&malicious_profile)?;
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("malicious")));
    
    // Test oversized content
    let oversized_profile = FourWordProfile::new(four_words)
        .with_website("x".repeat(11 * 1024 * 1024)); // 11MB
    
    let result = validator.check_size_limits(&oversized_profile)?;
    assert!(!result.is_valid);
    assert!(result.errors.iter().any(|e| e.contains("exceeds")));
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_storage_operations() -> Result<()> {
    let storage = Arc::new(DHTProfileStorage::new());
    let mut handles = vec![];
    
    // Spawn 50 concurrent storage operations
    for i in 0..50 {
        let storage_clone = storage.clone();
        let handle = tokio::spawn(async move {
            let four_words = FourWordAddress::generate().unwrap();
            let profile = FourWordProfile::new(four_words.clone())
                .with_website(format!("Website {}", i));
            
            let packet = ProfilePacket::new(four_words, profile).unwrap();
            storage_clone.store_packet(packet).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }
    
    // Most should succeed (some may fail due to rate limiting)
    assert!(success_count >= 40, "Too many operations failed: {}/50", success_count);
    
    // Verify storage stats
    let stats = storage.get_stats().await?;
    assert!(stats.total_packets > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_cache_functionality() -> Result<()> {
    let storage = Arc::new(DHTProfileStorage::new());
    let resolver = ProfileResolver::new(storage.clone());
    
    let four_words = FourWordAddress::generate()?;
    let profile = FourWordProfile::new(four_words.clone())
        .with_website("Cached content".to_string());
    
    let packet = ProfilePacket::new(four_words.clone(), profile)?;
    storage.store_packet(packet).await?;
    
    // First resolution - cache miss
    let start = std::time::Instant::now();
    let result1 = resolver.resolve(&four_words).await?;
    let first_time = start.elapsed();
    assert!(!result1.cache_hit);
    
    // Second resolution - should be cached
    let start = std::time::Instant::now();
    let _result2 = resolver.resolve(&four_words).await?;
    let second_time = start.elapsed();
    
    // Cached resolution should be faster
    assert!(second_time < first_time / 2, "Cache doesn't seem to be working");
    
    // Clear cache and verify
    let cleared = resolver.clear_cache().await?;
    assert!(cleared > 0);
    
    // Third resolution - cache miss again
    let result3 = resolver.resolve(&four_words).await?;
    assert!(!result3.cache_hit);
    
    Ok(())
}

#[tokio::test]
async fn test_batch_resolution() -> Result<()> {
    let storage = Arc::new(DHTProfileStorage::new());
    let resolver = ProfileResolver::new(storage.clone());
    
    // Create multiple profiles
    let mut queries = vec![];
    for i in 0..10 {
        let four_words = FourWordAddress::generate()?;
        let profile = FourWordProfile::new(four_words.clone())
            .with_website(format!("Website {}", i));
        
        let packet = ProfilePacket::new(four_words.clone(), profile)?;
        storage.store_packet(packet).await?;
        
        queries.push(ResolutionQuery::new(four_words));
    }
    
    // Batch resolve
    let batch_request = BatchResolutionRequest::new(queries)
        .with_concurrency(5);
    
    let start = std::time::Instant::now();
    let batch_response = resolver.resolve_batch(batch_request).await?;
    let batch_time = start.elapsed();
    
    assert_eq!(batch_response.results.len(), 10);
    assert!(batch_response.successful > 0);
    
    // Batch should be faster than sequential
    assert!(batch_time < Duration::from_secs(2), "Batch resolution too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_profile_update_versioning() -> Result<()> {
    let storage = Arc::new(DHTProfileStorage::new());
    let four_words = FourWordAddress::generate()?;
    
    // Store initial version
    let mut profile_v1 = FourWordProfile::new(four_words.clone())
        .with_website("Version 1".to_string());
    profile_v1.version = 1;
    
    let packet_v1 = ProfilePacket::new(four_words.clone(), profile_v1)?;
    storage.store_packet(packet_v1).await?;
    
    // Try to store older version - should fail
    let mut profile_v0 = FourWordProfile::new(four_words.clone())
        .with_website("Version 0".to_string());
    profile_v0.version = 0;
    
    let packet_v0 = ProfilePacket::new(four_words.clone(), profile_v0)?;
    let result = storage.store_packet(packet_v0).await;
    assert!(result.is_err());
    
    // Store newer version - should succeed
    let mut profile_v2 = FourWordProfile::new(four_words.clone())
        .with_website("Version 2".to_string());
    profile_v2.version = 2;
    
    let packet_v2 = ProfilePacket::new(four_words.clone(), profile_v2.clone())?;
    storage.store_packet(packet_v2).await?;
    
    // Verify latest version is stored
    let retrieved = storage.get_profile(&four_words).await?;
    assert!(retrieved.is_some());
    let retrieved_profile = retrieved.unwrap();
    assert_eq!(retrieved_profile.version, 2);
    assert_eq!(retrieved_profile.get_website(), Some("Version 2"));
    
    Ok(())
}

#[tokio::test]
async fn test_ttl_and_expiration() -> Result<()> {
    let storage = Arc::new(DHTProfileStorage::with_limits(100 * 1024, 1)); // 1 second TTL
    let four_words = FourWordAddress::generate()?;
    
    let profile = FourWordProfile::new(four_words.clone());
    let packet = ProfilePacket::new(four_words.clone(), profile)?.with_ttl(1);
    let packet_hash = packet.dht_hash.clone();
    
    storage.store_packet(packet).await?;
    
    // Should be retrievable immediately
    let result1 = storage.get_packet(&packet_hash).await?;
    assert!(result1.is_some());
    
    // Wait for expiration
    sleep(Duration::from_secs(2)).await;
    
    // Should be expired
    let result2 = storage.get_packet(&packet_hash).await?;
    assert!(result2.is_none());
    
    // Cleanup should remove expired packets
    let removed = storage.cleanup_expired().await?;
    assert!(removed > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_storage_backup_restore() -> Result<()> {
    let _temp_dir = tempdir()?;
    let storage1 = Arc::new(DHTProfileStorage::new());
    
    // Add test data
    for i in 0..5 {
        let four_words = FourWordAddress::generate()?;
        let profile = FourWordProfile::new(four_words.clone())
            .with_website(format!("Site {}", i));
        let packet = ProfilePacket::new(four_words, profile)?;
        storage1.store_packet(packet).await?;
    }
    
    // Backup
    let backup_path = _temp_dir.path().join("backup.json");
    let backed_up = storage1.backup_to_file(backup_path.to_str().unwrap()).await?;
    assert_eq!(backed_up, 5);
    
    // Create new storage and restore
    let storage2 = Arc::new(DHTProfileStorage::new());
    let restored = storage2.restore_from_file(backup_path.to_str().unwrap()).await?;
    assert_eq!(restored, 5);
    
    // Verify data integrity
    let stats1 = storage1.get_stats().await?;
    let stats2 = storage2.get_stats().await?;
    assert_eq!(stats1.total_packets, stats2.total_packets);
    
    Ok(())
}

#[tokio::test]
async fn test_four_word_address_validation() -> Result<()> {
    // Valid addresses
    let valid_cases = vec![
        "alpha-beta-gamma-delta",
        "one-two-three-four",
        "word1-word2-word3-word4",
    ];
    
    for address in valid_cases {
        let four_words = FourWordAddress::from_string(address)?;
        assert!(four_words.is_valid());
        assert_eq!(four_words.words().len(), 4);
    }
    
    // Invalid addresses
    let invalid_cases = vec![
        "only-three-words",
        "too-many-words-here-five",
        "special@char-word-word-word",
        "word_underscore-word-word-word",
        "",
        "----",
    ];
    
    for address in invalid_cases {
        let result = FourWordAddress::from_string(address);
        assert!(result.is_err() || !result.unwrap().is_valid());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_resolver_timeout() -> Result<()> {
    let storage = Arc::new(DHTProfileStorage::new());
    let resolver = ProfileResolver::new(storage);
    
    let four_words = FourWordAddress::generate()?;
    
    // Query with timeout for non-existent profile
    let query = ResolutionQuery::new(four_words)
        .with_timeout(100); // 100ms timeout
    
    let start = std::time::Instant::now();
    let result = resolver.resolve_with_query(query).await;
    let elapsed = start.elapsed();
    
    // Should complete within timeout
    assert!(elapsed < Duration::from_millis(200));
    
    // Result should indicate not found
    if let Ok(resolution) = result {
        assert!(resolution.profile.is_none());
    }
    
    Ok(())
}