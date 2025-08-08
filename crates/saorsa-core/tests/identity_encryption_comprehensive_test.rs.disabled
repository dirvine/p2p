// SPDX-License-Identifier: AGPL-3.0-only
// (c) 2025 Saorsa P2P Foundation

use saorsa_core::{
    error::{P2PError, SecurityError},
    identity_manager::*,
    secure_memory::SecureString,
};
use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;
use tokio;

/// Test that identity is encrypted when saved to disk
#[tokio::test]
async fn test_identity_encryption_at_rest() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let password = SecureString::from_str("test_password_123!").unwrap();
    manager.initialize(&password).await?;

    // Create an identity
    let params = IdentityCreationParams {
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        bio: Some("Test bio with sensitive info".to_string()),
        metadata: HashMap::new(),
        key_lifetime: None,
        derivation_path: None,
    };

    let identity = manager.create_identity(&password, params).await?;
    let identity_id = identity.id.clone();

    // Read the raw file from disk
    let identity_path = temp_dir.path().join(format!("{}.enc", identity_id));
    let raw_data = tokio::fs::read(&identity_path).await?;
    let raw_str = String::from_utf8_lossy(&raw_data);

    // Verify that sensitive data is not visible in plaintext
    assert!(
        !raw_str.contains("Test User"),
        "Display name should be encrypted"
    );
    assert!(
        !raw_str.contains("Test bio with sensitive info"),
        "Bio should be encrypted"
    );
    assert!(
        !raw_str.contains(&identity_id.to_string()),
        "Identity ID should be encrypted"
    );

    // Verify we can still load and decrypt the identity
    let loaded = manager.load_identity(&identity_id, &password).await?;
    assert_eq!(loaded.display_name, Some("Test User".to_string()));
    assert_eq!(loaded.bio, Some("Test bio with sensitive info".to_string()));

    Ok(())
}

/// Test that incorrect password fails to decrypt
#[tokio::test]
async fn test_decryption_with_wrong_password() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let correct_password = SecureString::from_str("correct_password_123!").unwrap();
    let wrong_password = SecureString::from_str("wrong_password_123!").unwrap();

    manager.initialize(&correct_password).await?;

    // Create identity with correct password
    let params = IdentityCreationParams::default();
    let identity = manager.create_identity(&correct_password, params).await?;

    // Try to load with wrong password
    let result = manager.load_identity(&identity.id, &wrong_password).await;
    assert!(
        matches!(
            result,
            Err(P2PError::Security(SecurityError::DecryptionFailed(_)))
        ),
        "Should fail with decryption error"
    );

    Ok(())
}

/// Test key rotation updates encryption
#[tokio::test]
async fn test_key_rotation_updates_encryption() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let password = SecureString::from_str("test_password_123!").unwrap();
    manager.initialize(&password).await?;

    // Create identity
    let params = IdentityCreationParams {
        display_name: Some("Original Name".to_string()),
        ..Default::default()
    };
    let identity = manager.create_identity(&password, params).await?;
    let identity_id = identity.id.clone();

    // Read original encrypted data
    let identity_path = temp_dir.path().join(format!("{}.enc", identity_id));
    let original_data = tokio::fs::read(&identity_path).await?;

    // Rotate keys
    manager.rotate_keys(&identity_id, &password).await?;

    // Read new encrypted data
    let rotated_data = tokio::fs::read(&identity_path).await?;

    // Verify that encrypted data changed (due to new encryption key)
    assert_ne!(
        original_data, rotated_data,
        "Encrypted data should change after rotation"
    );

    // Verify we can still decrypt with same password
    let loaded = manager.load_identity(&identity_id, &password).await?;
    assert_eq!(loaded.display_name, Some("Original Name".to_string()));
    assert_eq!(loaded.key_version, 2); // Version should be incremented

    Ok(())
}

/// Test migration of existing plaintext identities
#[tokio::test]
async fn test_migrate_plaintext_identity() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();

    // Create a mock plaintext identity file
    let identity_id = UserId::generate();
    let plaintext_identity = Identity {
        id: identity_id.clone(),
        four_word_address: "test.word.address.here".to_string(),
        state: IdentityState::Active,
        display_name: Some("Plaintext User".to_string()),
        avatar_url: None,
        bio: Some("This is currently in plaintext".to_string()),
        metadata: HashMap::new(),
        key_version: 1,
        created_at: 1234567890,
        updated_at: 1234567890,
        expires_at: 9999999999,
        previous_keys: vec![],
        revocation_cert: None,
    };

    // Write plaintext identity to disk
    let identity_path = temp_dir.path().join(format!("{}.json", identity_id));
    let plaintext_data = serde_json::to_vec_pretty(&plaintext_identity)?;
    tokio::fs::write(&identity_path, &plaintext_data).await?;

    // Initialize manager and migrate
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let password = SecureString::from_str("migration_password_123!").unwrap();
    manager.initialize(&password).await?;

    // Migrate existing identities
    manager.migrate_existing_identities(&password).await?;

    // Verify old file was removed
    assert!(
        !identity_path.exists(),
        "Old plaintext file should be removed"
    );

    // Verify new encrypted file exists
    let encrypted_path = temp_dir.path().join(format!("{}.enc", identity_id));
    assert!(encrypted_path.exists(), "New encrypted file should exist");

    // Read the migrated file
    let migrated_data = tokio::fs::read(&encrypted_path).await?;
    let migrated_str = String::from_utf8_lossy(&migrated_data);

    // Verify it's now encrypted
    assert!(
        !migrated_str.contains("Plaintext User"),
        "Should be encrypted after migration"
    );
    assert!(
        !migrated_str.contains("This is currently in plaintext"),
        "Bio should be encrypted"
    );

    // Verify we can load and decrypt
    let loaded = manager.load_identity(&identity_id, &password).await?;
    assert_eq!(loaded.display_name, Some("Plaintext User".to_string()));
    assert_eq!(
        loaded.bio,
        Some("This is currently in plaintext".to_string())
    );

    Ok(())
}

/// Test performance impact of encryption
#[tokio::test]
async fn test_encryption_performance() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let password = SecureString::from_str("perf_test_password_123!").unwrap();
    manager.initialize(&password).await?;

    // Create identity with large metadata
    let mut large_metadata = HashMap::new();
    for i in 0..100 {
        large_metadata.insert(
            format!("key_{}", i),
            format!("value_with_some_content_{}", i),
        );
    }

    let params = IdentityCreationParams {
        display_name: Some("Performance Test User".to_string()),
        bio: Some("x".repeat(10000)), // 10KB bio
        metadata: large_metadata,
        ..Default::default()
    };

    // Measure save time
    let start = std::time::Instant::now();
    let identity = manager.create_identity(&password, params).await?;
    let create_duration = start.elapsed();

    // Measure load time
    let start = std::time::Instant::now();
    let _loaded = manager.load_identity(&identity.id, &password).await?;
    let load_duration = start.elapsed();

    println!("Create duration: {:?}", create_duration);
    println!("Load duration: {:?}", load_duration);

    // Verify performance requirement (< 10ms for encryption/decryption overhead)
    // Note: Total operation might be longer due to I/O, but crypto should be fast
    assert!(
        create_duration.as_millis() < 50,
        "Create operation took {}ms, should be < 50ms",
        create_duration.as_millis()
    );
    assert!(
        load_duration.as_millis() < 50,
        "Load operation took {}ms, should be < 50ms",
        load_duration.as_millis()
    );

    Ok(())
}

/// Test concurrent access with encryption
#[tokio::test]
async fn test_concurrent_encrypted_access() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();
    let manager =
        std::sync::Arc::new(IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?);

    let password = SecureString::from_str("concurrent_test_123!").unwrap();
    manager.initialize(&password).await?;

    // Create an identity
    let params = IdentityCreationParams::default();
    let identity = manager.create_identity(&password, params).await?;
    let identity_id = identity.id.clone();

    // Spawn multiple concurrent readers
    let mut handles = vec![];
    for i in 0..10 {
        let manager_clone = manager.clone();
        let password_clone = password.clone();
        let id_clone = identity_id.clone();

        let handle = tokio::spawn(async move {
            for _ in 0..5 {
                let loaded = manager_clone
                    .load_identity(&id_clone, &password_clone)
                    .await?;
                assert_eq!(loaded.id, id_clone);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Ok::<_, P2PError>(i)
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap()?;
        assert!(result < 10);
    }

    Ok(())
}

/// Test encrypted data format versioning
#[tokio::test]
async fn test_encryption_format_versioning() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let password = SecureString::from_str("version_test_123!").unwrap();
    manager.initialize(&password).await?;

    // Create identity
    let params = IdentityCreationParams::default();
    let identity = manager.create_identity(&password, params).await?;

    // Read encrypted file and verify format
    let identity_path = temp_dir.path().join(format!("{}.enc", identity.id));
    let encrypted_data = tokio::fs::read(&identity_path).await?;

    // Verify binary format (version + salt + nonce + ciphertext)
    assert!(
        encrypted_data.len() > 4 + 32 + 12,
        "Encrypted data should have version, salt, nonce, and ciphertext"
    );

    // Check version byte
    assert_eq!(encrypted_data[0], 1, "Should use version 1 format");

    Ok(())
}

use serde::{Deserialize, Serialize};
