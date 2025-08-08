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

//! Identity encryption performance tests

use saorsa_core::encrypted_key_storage::SecurityLevel;
use saorsa_core::identity_manager::{IdentityCreationParams, IdentityManager};
use saorsa_core::secure_memory::SecureString;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[tokio::test]
async fn test_encryption_performance_overhead() -> Result<()> {
    // Create temporary directory for storage
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Create identity manager
    let manager = IdentityManager::new(storage_path, SecurityLevel::Fast)
        .await
        .unwrap();

    // Initialize with storage password
    let storage_password = SecureString::from_str("test_storage_password_123!").unwrap();
    manager.initialize(&storage_password).await.unwrap();

    // Create an identity
    let params = IdentityCreationParams {
        display_name: Some("Performance Test User".to_string()),
        bio: Some("Test bio for performance testing".to_string()),
        derivation_path: None,
        avatar_url: None,
        key_lifetime: None,
        metadata: std::collections::HashMap::new(),
    };

    let identity = manager
        .create_identity(&storage_password, params)
        .await
        .unwrap();

    // Test sync package creation performance
    let device_password = SecureString::from_str("device_password_456!").unwrap();

    // Warm up
    for _ in 0..5 {
        let _ = manager
            .create_sync_package(&identity.id, &device_password, &storage_password)
            .await
            .unwrap();
    }

    // Measure encryption performance
    let mut encryption_times = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let _ = manager
            .create_sync_package(&identity.id, &device_password, &storage_password)
            .await
            .unwrap();
        let elapsed = start.elapsed();
        encryption_times.push(elapsed);
    }

    // Calculate statistics
    let total_time: Duration = encryption_times.iter().sum();
    let avg_time = total_time / encryption_times.len() as u32;
    let max_time = encryption_times.iter().max().unwrap();
    let min_time = encryption_times.iter().min().unwrap();

    println!("Encryption Performance Statistics:");
    println!("  Average time: {:?}", avg_time);
    println!("  Min time: {:?}", min_time);
    println!("  Max time: {:?}", max_time);
    println!("  Total samples: {}", encryption_times.len());

    // Check that performance meets requirements (< 10ms average)
    assert!(
        avg_time < Duration::from_millis(10),
        "Average encryption time {:?} exceeds 10ms requirement",
        avg_time
    );

    // Test decryption performance
    let sync_package = manager
        .create_sync_package(&identity.id, &device_password, &storage_password)
        .await
        .unwrap();

    let mut decryption_times = Vec::new();
    for _ in 0..100 {
        let temp_dir2 = TempDir::new().unwrap();
        let manager2 = IdentityManager::new(temp_dir2.path(), SecurityLevel::High)
            .await
            .unwrap();
        manager2.initialize(&storage_password).await.unwrap();

        let start = Instant::now();
        let _ = manager2
            .import_sync_package(&sync_package, &device_password, &storage_password)
            .await
            .unwrap();
        let elapsed = start.elapsed();
        decryption_times.push(elapsed);
    }

    // Calculate decryption statistics
    let total_decrypt_time: Duration = decryption_times.iter().sum();
    let avg_decrypt_time = total_decrypt_time / decryption_times.len() as u32;

    println!("\nDecryption Performance Statistics:");
    println!("  Average time: {:?}", avg_decrypt_time);
    println!("  Min time: {:?}", decryption_times.iter().min().unwrap());
    println!("  Max time: {:?}", decryption_times.iter().max().unwrap());

    // Check that decryption also meets requirements
    assert!(
        avg_decrypt_time < Duration::from_millis(10),
        "Average decryption time {:?} exceeds 10ms requirement",
        avg_decrypt_time
    );
    Ok(())
}

#[tokio::test]
async fn test_encryption_with_different_data_sizes() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::High)
        .await
        .unwrap();

    let storage_password = SecureString::from_str("test_password").unwrap();
    manager.initialize(&storage_password).await.unwrap();

    // Test with different bio sizes
    let sizes = vec![("Small", 100), ("Medium", 1000), ("Large", 10000)];

    for (name, size) in sizes {
        let bio = "x".repeat(size);
        let params = IdentityCreationParams {
            display_name: Some(format!("{} User", name)),
            bio: Some(bio),
            derivation_path: None,
        };

        let identity = manager
            .create_identity(&storage_password, params)
            .await
            .unwrap();

        let device_password = SecureString::from_str("device_pass").unwrap();

        let start = Instant::now();
        let sync_package = manager
            .create_sync_package(&identity.id, &device_password, &storage_password)
            .await
            .unwrap();
        let encryption_time = start.elapsed();

        println!("{} data ({} bytes):", name, size);
        println!("  Encryption time: {:?}", encryption_time);
        println!(
            "  Encrypted size: {} bytes",
            sync_package.encrypted_identity.len()
        );
        println!(
            "  Overhead: {} bytes",
            sync_package.encrypted_identity.len() - size
        );

        // All sizes should still meet the 10ms requirement
        assert!(
            encryption_time < Duration::from_millis(10),
            "{} data encryption time {:?} exceeds 10ms",
            name,
            encryption_time
        );
    }
    Ok(())
}
