// Unit tests for identity storage module

use saorsa_lib::identity_storage::{IdentityStorage, IdentityStorageConfig};
use saorsa_lib::passkey_auth::StoredPasskeyCredential;
use saorsa_core::identity::{UserIdentity, EncryptedUserProfile, VerificationLevel};
use ed25519_dalek::Keypair;
use tempfile::TempDir;
use std::time::SystemTime;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create test identity storage
    fn create_test_storage() -> (IdentityStorage, TempDir, tauri::AppHandle) {
        let temp_dir = TempDir::new().unwrap();
        let app = tauri::test::mock_app();
        
        let config = IdentityStorageConfig {
            file_name: "test_identity.enc".to_string(),
            auto_save: true,
            password: None,
        };
        
        let storage = IdentityStorage::new(app.app_handle(), config).unwrap();
        (storage, temp_dir, app.app_handle())
    }

    // Helper to create test identity
    fn create_test_identity() -> (UserIdentity, Keypair) {
        let keypair = Keypair::generate(&mut rand::rngs::OsRng);
        let identity = UserIdentity {
            user_id: "test_user_123".to_string(),
            public_key: keypair.public.to_bytes().to_vec(),
            display_name_hint: "Test User".to_string(),
            three_word_address: "test.user.address".to_string(),
            created_at: SystemTime::now(),
            version: 1,
            verification_level: VerificationLevel::SelfSigned,
        };
        (identity, keypair)
    }

    #[tokio::test]
    async fn test_storage_creation() {
        let (_storage, _temp_dir, _app) = create_test_storage();
        // Should create without errors
    }

    #[tokio::test]
    async fn test_init_encryption() {
        let (storage, _temp_dir, _app) = create_test_storage();
        
        let result = storage.init_encryption("test_password").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_save_and_load_identity() {
        let (storage, _temp_dir, _app) = create_test_storage();
        let (identity, keypair) = create_test_identity();
        
        // Save identity
        let save_result = storage.save_identity(
            &identity,
            &keypair,
            None,
            "test_password"
        ).await;
        assert!(save_result.is_ok());
        
        // Load identity
        let load_result = storage.load_identity("test_password").await;
        assert!(load_result.is_ok());
        
        let loaded = load_result.unwrap();
        assert!(loaded.is_some());
        
        let (loaded_identity, loaded_keypair, loaded_profile) = loaded.unwrap();
        assert_eq!(loaded_identity.user_id, identity.user_id);
        assert_eq!(loaded_identity.display_name_hint, identity.display_name_hint);
        assert_eq!(loaded_identity.three_word_address, identity.three_word_address);
        assert_eq!(loaded_keypair.public, keypair.public);
        assert!(loaded_profile.is_none());
    }

    #[tokio::test]
    async fn test_save_and_load_with_profile() {
        let (storage, _temp_dir, _app) = create_test_storage();
        let (identity, keypair) = create_test_identity();
        
        // Create encrypted profile
        let profile = EncryptedUserProfile {
            user_id: identity.user_id.clone(),
            public_key: identity.public_key.clone(),
            encrypted_data: vec![1, 2, 3, 4, 5], // Mock encrypted data
            signature: vec![6, 7, 8, 9, 10], // Mock signature
            ipv6_binding_proof: None,
            created_at: SystemTime::now(),
        };
        
        // Save with profile
        let save_result = storage.save_identity(
            &identity,
            &keypair,
            Some(&profile),
            "test_password"
        ).await;
        assert!(save_result.is_ok());
        
        // Load and verify profile
        let load_result = storage.load_identity("test_password").await;
        assert!(load_result.is_ok());
        
        let loaded = load_result.unwrap().unwrap();
        assert!(loaded.2.is_some());
        
        let loaded_profile = loaded.2.unwrap();
        assert_eq!(loaded_profile.user_id, profile.user_id);
        assert_eq!(loaded_profile.encrypted_data, profile.encrypted_data);
        assert_eq!(loaded_profile.signature, profile.signature);
    }

    #[tokio::test]
    async fn test_wrong_password_fails() {
        let (storage, _temp_dir, _app) = create_test_storage();
        let (identity, keypair) = create_test_identity();
        
        // Save with one password
        storage.save_identity(&identity, &keypair, None, "correct_password").await.unwrap();
        
        // Try to load with wrong password
        let result = storage.load_identity("wrong_password").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_identity() {
        let (storage, _temp_dir, _app) = create_test_storage();
        let (identity, keypair) = create_test_identity();
        
        // Save identity
        storage.save_identity(&identity, &keypair, None, "test_password").await.unwrap();
        assert!(storage.identity_exists());
        
        // Delete identity
        let result = storage.delete_identity().await;
        assert!(result.is_ok());
        assert!(!storage.identity_exists());
        
        // Try to load - should return None
        let load_result = storage.load_identity("test_password").await;
        assert!(load_result.is_ok());
        assert!(load_result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_passkey_credential_storage() {
        let (storage, _temp_dir, _app) = create_test_storage();
        
        // Add passkey credential
        let credential = StoredPasskeyCredential {
            credential_id: "test_cred_123".to_string(),
            public_key: vec![1, 2, 3, 4, 5],
            counter: 0,
            created_at: 1234567890,
            three_word_address: "test.passkey.address".to_string(),
            user_id: "test_user".to_string(),
        };
        
        let result = storage.add_passkey_credential(&credential, "test_password").await;
        assert!(result.is_ok());
        
        // Get credentials
        let creds = storage.get_passkey_credentials().await.unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].credential_id, credential.credential_id);
    }

    #[tokio::test]
    async fn test_remove_passkey_credential() {
        let (storage, _temp_dir, _app) = create_test_storage();
        
        // Add two credentials
        let cred1 = StoredPasskeyCredential {
            credential_id: "cred1".to_string(),
            public_key: vec![1, 2, 3],
            counter: 0,
            created_at: 1234567890,
            three_word_address: "test1.passkey.address".to_string(),
            user_id: "user1".to_string(),
        };
        
        let cred2 = StoredPasskeyCredential {
            credential_id: "cred2".to_string(),
            public_key: vec![4, 5, 6],
            counter: 0,
            created_at: 1234567891,
            three_word_address: "test2.passkey.address".to_string(),
            user_id: "user2".to_string(),
        };
        
        storage.add_passkey_credential(&cred1, "test_password").await.unwrap();
        storage.add_passkey_credential(&cred2, "test_password").await.unwrap();
        
        // Remove one
        let removed = storage.remove_passkey_credential("cred1", "test_password").await.unwrap();
        assert!(removed);
        
        // Verify only one remains
        let creds = storage.get_passkey_credentials().await.unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].credential_id, "cred2");
        
        // Try to remove non-existent
        let removed = storage.remove_passkey_credential("non_existent", "test_password").await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_unlock_with_derived_key() {
        let (storage, _temp_dir, _app) = create_test_storage();
        
        // Create a derived key
        let key = [42u8; 32];
        
        let result = storage.unlock_with_derived_key(&key).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_storage_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let app = tauri::test::mock_app();
        let (identity, keypair) = create_test_identity();
        
        // Create and save with first storage instance
        {
            let config = IdentityStorageConfig {
                file_name: "persist_test.enc".to_string(),
                auto_save: true,
                password: None,
            };
            
            let storage = IdentityStorage::new(app.app_handle(), config).unwrap();
            storage.save_identity(&identity, &keypair, None, "test_password").await.unwrap();
        }
        
        // Load with new storage instance
        {
            let config = IdentityStorageConfig {
                file_name: "persist_test.enc".to_string(),
                auto_save: true,
                password: None,
            };
            
            let storage = IdentityStorage::new(app.app_handle(), config).unwrap();
            let loaded = storage.load_identity("test_password").await.unwrap();
            
            assert!(loaded.is_some());
            let (loaded_identity, _, _) = loaded.unwrap();
            assert_eq!(loaded_identity.user_id, identity.user_id);
        }
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let (storage, _temp_dir, _app) = create_test_storage();
        let storage = std::sync::Arc::new(storage);
        let (identity, keypair) = create_test_identity();
        
        // Save identity first
        storage.save_identity(&identity, &keypair, None, "test_password").await.unwrap();
        
        // Concurrent reads
        let storage1 = storage.clone();
        let task1 = tokio::spawn(async move {
            storage1.load_identity("test_password").await
        });
        
        let storage2 = storage.clone();
        let task2 = tokio::spawn(async move {
            storage2.load_identity("test_password").await
        });
        
        let result1 = task1.await.unwrap();
        let result2 = task2.await.unwrap();
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result1.unwrap().is_some());
        assert!(result2.unwrap().is_some());
    }
}