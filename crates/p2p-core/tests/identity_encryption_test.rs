// Integration tests for identity encryption

use saorsa_core::{
    IdentityManager, 
    IdentityCreationParams,
    secure_memory::SecureString,
    SecurityLevel,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_identity_sync_package_encryption() {
    let temp_dir = TempDir::new().unwrap();
    let identity_dir = temp_dir.path().join("identities");
    std::fs::create_dir_all(&identity_dir).unwrap();
    
    // Create identity manager
    let manager = IdentityManager::new(
        identity_dir,
        SecurityLevel::Fast,
    ).unwrap();
    
    // Initialize manager
    let storage_password = SecureString::from_str("StoragePassword123!").unwrap();
    manager.initialize(&storage_password).await.unwrap();
    
    // Create an identity
    let params = IdentityCreationParams {
        display_name: "Test User".to_string(),
        email: Some("test@example.com".to_string()),
        bio: Some("Test bio".to_string()),
        avatar_url: None,
        metadata: Default::default(),
    };
    
    let identity = manager.create_identity(params, &storage_password).await.unwrap();
    let identity_id = identity.id.clone();
    
    // Create sync package with device password
    let device_password = SecureString::from_str("DevicePassword456!").unwrap();
    let sync_package = manager.create_sync_package(
        &identity_id,
        &storage_password,
        &device_password,
    ).await.unwrap();
    
    // Verify encrypted data is present
    assert!(!sync_package.encrypted_identity.is_empty());
    assert!(!sync_package.encrypted_keys.is_empty());
    assert_ne!(sync_package.device_fingerprint, [0u8; 32]);
    assert!(!sync_package.signature.is_empty());
    
    // Create a new manager to simulate different device
    let temp_dir2 = TempDir::new().unwrap();
    let identity_dir2 = temp_dir2.path().join("identities");
    std::fs::create_dir_all(&identity_dir2).unwrap();
    
    let manager2 = IdentityManager::new(
        identity_dir2,
        SecurityLevel::Fast,
    ).unwrap();
    
    let storage_password2 = SecureString::from_str("NewStoragePassword789!").unwrap();
    manager2.initialize(&storage_password2).await.unwrap();
    
    // Import the sync package
    let imported_identity = manager2.import_sync_package(
        &sync_package,
        &device_password,
        &storage_password2,
    ).await.unwrap();
    
    // Verify imported identity matches original
    assert_eq!(imported_identity.id, identity.id);
    assert_eq!(imported_identity.display_name, identity.display_name);
    assert_eq!(imported_identity.email, identity.email);
    assert_eq!(imported_identity.bio, identity.bio);
}

#[tokio::test]
async fn test_sync_package_wrong_password_fails() {
    let temp_dir = TempDir::new().unwrap();
    let identity_dir = temp_dir.path().join("identities");
    std::fs::create_dir_all(&identity_dir).unwrap();
    
    let manager = IdentityManager::new(
        identity_dir,
        SecurityLevel::Fast,
    ).unwrap();
    
    let storage_password = SecureString::from_str("StoragePassword123!").unwrap();
    manager.initialize(&storage_password).await.unwrap();
    
    // Create identity
    let params = IdentityCreationParams {
        display_name: "Test User".to_string(),
        email: None,
        bio: None,
        avatar_url: None,
        metadata: Default::default(),
    };
    
    let identity = manager.create_identity(params, &storage_password).await.unwrap();
    
    // Create sync package
    let device_password = SecureString::from_str("CorrectPassword123!").unwrap();
    let sync_package = manager.create_sync_package(
        &identity.id,
        &storage_password,
        &device_password,
    ).await.unwrap();
    
    // Try to import with wrong password
    let wrong_password = SecureString::from_str("WrongPassword456!").unwrap();
    let result = manager.import_sync_package(
        &sync_package,
        &wrong_password,
        &storage_password,
    ).await;
    
    // Should fail
    assert!(result.is_err());
}