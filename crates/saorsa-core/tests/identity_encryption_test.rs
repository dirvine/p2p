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

use saorsa_core::encrypted_key_storage::SecurityLevel;
use saorsa_core::identity_manager::{IdentityCreationParams, IdentityManager};
use saorsa_core::secure_memory::SecureString;
use std::time::Duration;
use tempfile::TempDir;

#[tokio::test]
async fn test_identity_encryption_sync_package() {
    // Create temporary directory for storage
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Create identity manager
    let manager = IdentityManager::new(storage_path, SecurityLevel::High)
        .await
        .unwrap();

    // Initialize with storage password
    let storage_password = SecureString::from_str("test_storage_password_123!").unwrap();
    manager.initialize(&storage_password).await.unwrap();

    // Create an identity
    let params = IdentityCreationParams {
        display_name: Some("Test User".to_string()),
        bio: Some("Test bio for encryption".to_string()),
        derivation_path: None,
        avatar_url: None,
        key_lifetime: None,
        metadata: std::collections::HashMap::new(),
    };

    let identity = manager
        .create_identity(&storage_password, params)
        .await
        .unwrap();

    // Create sync package with device password
    let device_password = SecureString::from_str("device_password_456!").unwrap();
    let sync_package = manager
        .create_sync_package(&identity.id, &device_password, &storage_password)
        .await
        .unwrap();

    // Verify encrypted data is not plaintext
    assert!(sync_package.encrypted_identity.len() > 44); // Salt + nonce + encrypted data
    assert!(sync_package.encrypted_keys.len() > 44);

    // The encrypted data should not contain plaintext identity information
    let encrypted_str = String::from_utf8_lossy(&sync_package.encrypted_identity);
    assert!(!encrypted_str.contains("Test User"));
    assert!(!encrypted_str.contains("Test bio"));

    // Create a new identity manager to simulate different device
    let temp_dir2 = TempDir::new().unwrap();
    let manager2 = IdentityManager::new(temp_dir2.path(), SecurityLevel::High)
        .await
        .unwrap();

    let storage_password2 = SecureString::from_str("new_storage_password_789!").unwrap();
    manager2.initialize(&storage_password2).await.unwrap();

    // Import the sync package
    let imported_identity = manager2
        .import_sync_package(&sync_package, &device_password, &storage_password2)
        .await
        .unwrap();

    // Verify imported identity matches original
    assert_eq!(imported_identity.id, identity.id);
    assert_eq!(imported_identity.display_name, identity.display_name);
    assert_eq!(imported_identity.bio, identity.bio);
    assert_eq!(
        imported_identity.four_word_address,
        identity.four_word_address
    );
}

#[tokio::test]
async fn test_identity_encryption_wrong_password() {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::High)
        .await
        .unwrap();

    let storage_password = SecureString::from_str("storage_pass").unwrap();
    manager.initialize(&storage_password).await.unwrap();

    // Create identity
    let params = IdentityCreationParams {
        display_name: Some("Test User".to_string()),
        bio: None,
        derivation_path: None,
        recovery_threshold: None,
    };

    let identity = manager
        .create_identity(&storage_password, params)
        .await
        .unwrap();

    // Create sync package
    let device_password = SecureString::from_str("correct_password").unwrap();
    let sync_package = manager
        .create_sync_package(&identity.id, &device_password, &storage_password)
        .await
        .unwrap();

    // Try to import with wrong password
    let temp_dir2 = TempDir::new().unwrap();
    let manager2 = IdentityManager::new(temp_dir2.path(), SecurityLevel::High)
        .await
        .unwrap();
    manager2.initialize(&storage_password).await.unwrap();

    let wrong_password = SecureString::from_str("wrong_password").unwrap();
    let result = manager2
        .import_sync_package(&sync_package, &wrong_password, &storage_password)
        .await;

    // Should fail with decryption error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Decryption") || err.to_string().contains("Invalid"));
}

#[tokio::test]
async fn test_identity_encryption_key_rotation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::High)
        .await
        .unwrap();

    let storage_password = SecureString::from_str("storage_pass").unwrap();
    manager.initialize(&storage_password).await.unwrap();

    // Create identity
    let params = IdentityCreationParams {
        display_name: Some("Rotation Test".to_string()),
        bio: None,
        derivation_path: None,
        recovery_threshold: None,
    };

    let identity = manager
        .create_identity(&storage_password, params)
        .await
        .unwrap();

    // Create first sync package
    let password1 = SecureString::from_str("password_v1").unwrap();
    let package1 = manager
        .create_sync_package(&identity.id, &password1)
        .await
        .unwrap();

    // Wait a bit to ensure different timestamp
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Create second sync package with different password (simulating key rotation)
    let password2 = SecureString::from_str("password_v2").unwrap();
    let package2 = manager
        .create_sync_package(&identity.id, &password2)
        .await
        .unwrap();

    // Packages should be different
    assert_ne!(package1.encrypted_identity, package2.encrypted_identity);
    assert_ne!(package1.encrypted_keys, package2.encrypted_keys);
    assert_ne!(package1.timestamp, package2.timestamp);

    // Both should be importable with their respective passwords
    let temp_dir2 = TempDir::new().unwrap();
    let manager2 = IdentityManager::new(temp_dir2.path(), SecurityLevel::High)
        .await
        .unwrap();
    manager2.initialize(&storage_password).await.unwrap();

    let imported1 = manager2
        .import_sync_package(&package1, &password1, &storage_password)
        .await
        .unwrap();
    assert_eq!(imported1.id, identity.id);

    let temp_dir3 = TempDir::new().unwrap();
    let manager3 = IdentityManager::new(temp_dir3.path(), SecurityLevel::High)
        .await
        .unwrap();
    manager3.initialize(&storage_password).await.unwrap();

    let imported2 = manager3
        .import_sync_package(&package2, &password2, &storage_password)
        .await
        .unwrap();
    assert_eq!(imported2.id, identity.id);
}
