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

// Unit tests for passkey authentication module

use saorsa_lib::passkey_auth::{
    MockAuthenticator, PasskeyAuthManager, PlatformAuthenticator, StoredPasskeyCredential,
};
use std::path::PathBuf;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test passkey manager
    fn create_test_manager() -> (PasskeyAuthManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Use mock authenticator for testing
        manager.authenticator = PlatformAuthenticator::Mock(MockAuthenticator::new(true));

        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_passkey_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let result = PasskeyAuthManager::new(temp_dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_is_available() {
        let (manager, _temp_dir) = create_test_manager();
        let available = manager.is_available().await;
        assert!(available); // Mock always returns true
    }

    #[tokio::test]
    async fn test_create_passkey_success() {
        let (manager, _temp_dir) = create_test_manager();

        let result = manager
            .create_passkey("test_user", "test.word.address")
            .await;
        assert!(result.is_ok());

        let credential = result.unwrap();
        assert_eq!(credential.user_id, "test_user");
        assert_eq!(credential.three_word_address, "test.word.address");
        assert!(!credential.credential_id.is_empty());
        assert!(!credential.public_key.is_empty());
        assert_eq!(credential.counter, 0);
    }

    #[tokio::test]
    async fn test_create_passkey_with_failed_auth() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf()).unwrap();

        // Use mock authenticator that fails
        manager.authenticator = PlatformAuthenticator::Mock(MockAuthenticator::new(false));

        let result = manager
            .create_passkey("test_user", "test.word.address")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_authenticate_with_passkey() {
        let (manager, _temp_dir) = create_test_manager();

        // First create a passkey
        let credential = manager
            .create_passkey("test_user", "test.word.address")
            .await
            .unwrap();

        // Then authenticate with it
        let result = manager
            .authenticate_with_passkey(&credential.credential_id)
            .await;
        assert!(result.is_ok());

        let signature = result.unwrap();
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64); // Ed25519 signature length
    }

    #[tokio::test]
    async fn test_authenticate_with_invalid_credential() {
        let (manager, _temp_dir) = create_test_manager();

        // Try to authenticate with non-existent credential
        let result = manager
            .authenticate_with_passkey("invalid_credential_id")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_passkey() {
        let (manager, _temp_dir) = create_test_manager();

        // Create a passkey
        let credential = manager
            .create_passkey("test_user", "test.word.address")
            .await
            .unwrap();

        // Delete it
        let result = manager.delete_passkey(&credential.credential_id).await;
        assert!(result.is_ok());

        // Try to authenticate with deleted credential - should fail
        let auth_result = manager
            .authenticate_with_passkey(&credential.credential_id)
            .await;
        assert!(auth_result.is_err());
    }

    #[tokio::test]
    async fn test_get_platform_info() {
        let (manager, _temp_dir) = create_test_manager();
        let info = manager.get_platform_info();
        assert_eq!(info, "Mock (Testing)");
    }

    #[tokio::test]
    async fn test_multiple_passkeys() {
        let (manager, _temp_dir) = create_test_manager();

        // Create multiple passkeys
        let cred1 = manager
            .create_passkey("user1", "user1.test.address")
            .await
            .unwrap();
        let cred2 = manager
            .create_passkey("user2", "user2.test.address")
            .await
            .unwrap();

        // Both should have unique credential IDs
        assert_ne!(cred1.credential_id, cred2.credential_id);

        // Both should authenticate successfully
        let auth1 = manager
            .authenticate_with_passkey(&cred1.credential_id)
            .await;
        let auth2 = manager
            .authenticate_with_passkey(&cred2.credential_id)
            .await;

        assert!(auth1.is_ok());
        assert!(auth2.is_ok());
    }

    #[tokio::test]
    async fn test_stored_credential_serialization() {
        let credential = StoredPasskeyCredential {
            credential_id: "test_cred_id".to_string(),
            public_key: vec![1, 2, 3, 4, 5],
            counter: 42,
            created_at: 1234567890,
            three_word_address: "test.word.address".to_string(),
            user_id: "test_user".to_string(),
        };

        // Serialize
        let serialized = serde_json::to_string(&credential).unwrap();

        // Deserialize
        let deserialized: StoredPasskeyCredential = serde_json::from_str(&serialized).unwrap();

        assert_eq!(credential.credential_id, deserialized.credential_id);
        assert_eq!(credential.public_key, deserialized.public_key);
        assert_eq!(credential.counter, deserialized.counter);
        assert_eq!(credential.created_at, deserialized.created_at);
        assert_eq!(
            credential.three_word_address,
            deserialized.three_word_address
        );
        assert_eq!(credential.user_id, deserialized.user_id);
    }

    #[tokio::test]
    async fn test_concurrent_authentication() {
        let (manager, _temp_dir) = create_test_manager();
        let manager = std::sync::Arc::new(manager);

        // Create a passkey
        let credential = manager
            .create_passkey("test_user", "test.word.address")
            .await
            .unwrap();

        // Try concurrent authentications
        let manager1 = manager.clone();
        let cred_id1 = credential.credential_id.clone();
        let task1 =
            tokio::spawn(async move { manager1.authenticate_with_passkey(&cred_id1).await });

        let manager2 = manager.clone();
        let cred_id2 = credential.credential_id.clone();
        let task2 =
            tokio::spawn(async move { manager2.authenticate_with_passkey(&cred_id2).await });

        let result1 = task1.await.unwrap();
        let result2 = task2.await.unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
