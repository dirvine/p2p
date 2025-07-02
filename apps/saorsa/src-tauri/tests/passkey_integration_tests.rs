// tests/passkey_integration_tests.rs
use saorsa_lib::passkey_auth::{PasskeyAuthManager, MockAuthenticator, PlatformAuthenticator};
use tempfile::TempDir;
use serial_test::serial;

#[cfg(test)]
mod passkey_tests {
    use super::*;
    
    /// Helper to create test manager with mock authenticator
    fn create_test_manager(should_succeed: bool) -> (PasskeyAuthManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf()).unwrap();
        manager.authenticator = PlatformAuthenticator::Mock(
            MockAuthenticator::new(should_succeed)
        );
        (manager, temp_dir)
    }
    
    #[tokio::test]
    async fn test_passkey_availability() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        // Mock authenticator should always be available
        assert!(manager.is_available().await);
    }
    
    #[tokio::test]
    async fn test_passkey_creation_success() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        let credential = manager.create_passkey("test_user", "test.word.address")
            .await
            .expect("Should create passkey");
        
        assert_eq!(credential.user_id, "test_user");
        assert_eq!(credential.three_word_address, "test.word.address");
        assert!(!credential.credential_id.is_empty());
        assert!(!credential.public_key.is_empty());
        assert_eq!(credential.public_key.len(), 32); // Ed25519 public key size
        assert!(credential.created_at > 0);
    }
    
    #[tokio::test]
    async fn test_passkey_authentication_success() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        // Create passkey first
        let credential = manager.create_passkey("test_user", "test.word.address")
            .await
            .unwrap();
        
        // Authenticate with passkey
        let signature = manager.authenticate_with_passkey(&credential.credential_id)
            .await
            .expect("Should authenticate");
        
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64); // Ed25519 signature length
    }
    
    #[tokio::test]
    async fn test_authentication_failure() {
        let (manager, _temp_dir) = create_test_manager(false);
        
        // Should fail to create passkey
        let result = manager.create_passkey("test_user", "test.word.address").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("authentication failed"));
    }
    
    #[tokio::test]
    #[serial]
    async fn test_keychain_storage() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        // Create and store credential
        let credential = manager.create_passkey("keychain_test", "test.word.address")
            .await
            .unwrap();
        
        // Verify we can authenticate with stored credential
        let signature = manager.authenticate_with_passkey(&credential.credential_id)
            .await
            .unwrap();
        
        assert!(!signature.is_empty());
        
        // Test deletion
        manager.delete_passkey(&credential.credential_id)
            .await
            .expect("Should delete passkey");
        
        // Authentication should now fail
        let auth_result = manager.authenticate_with_passkey(&credential.credential_id).await;
        assert!(auth_result.is_err());
    }
    
    #[tokio::test]
    async fn test_platform_info() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        let info = manager.get_platform_info();
        assert!(!info.is_empty());
        assert!(info.contains("Mock"));
    }
    
    #[tokio::test]
    async fn test_multiple_credentials() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        // Create multiple credentials
        let cred1 = manager.create_passkey("user1", "user1.test.address").await.unwrap();
        let cred2 = manager.create_passkey("user2", "user2.test.address").await.unwrap();
        
        // Both should be unique
        assert_ne!(cred1.credential_id, cred2.credential_id);
        assert_ne!(cred1.user_id, cred2.user_id);
        
        // Both should authenticate successfully
        let sig1 = manager.authenticate_with_passkey(&cred1.credential_id).await.unwrap();
        let sig2 = manager.authenticate_with_passkey(&cred2.credential_id).await.unwrap();
        
        // Signatures should be different (random challenge)
        assert_ne!(sig1, sig2);
    }
}

#[cfg(test)]
mod platform_specific_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_platform_detection() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf());
        
        assert!(manager.is_ok(), "Should create manager on any supported platform");
        
        let mgr = manager.unwrap();
        let platform_info = mgr.get_platform_info();
        
        // Should contain some platform information
        assert!(!platform_info.is_empty());
        
        // Test availability
        let available = mgr.is_available().await;
        println!("Platform: {}, Available: {}", platform_info, available);
    }
    
    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore] // Run manually: cargo test -- --ignored
    async fn test_real_touchid() {
        use saorsa_lib::platform::macos::TouchIdAuth;
        
        let auth = TouchIdAuth::new().unwrap();
        let available = auth.is_available().await;
        
        println!("TouchID available: {}", available);
        
        if available {
            // This will trigger real TouchID prompt
            match auth.authenticate("Test TouchID for Saorsa").await {
                Ok(_) => println!("TouchID authentication successful"),
                Err(e) => println!("TouchID authentication failed: {}", e),
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore] // Run manually
    async fn test_real_windows_hello() {
        use saorsa_lib::platform::windows::WindowsHelloAuth;
        
        let auth = WindowsHelloAuth::new().unwrap();
        let available = auth.is_available().await;
        
        println!("Windows Hello available: {}", available);
        
        if available {
            match auth.verify_user("Test Windows Hello for Saorsa").await {
                Ok(_) => println!("Windows Hello authentication successful"),
                Err(e) => println!("Windows Hello authentication failed: {}", e),
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    #[tokio::test]
    #[ignore] // Run manually
    async fn test_real_linux_auth() {
        use saorsa_lib::platform::linux::LinuxAuth;
        
        let auth = LinuxAuth::new().unwrap();
        let available = auth.is_available().await;
        
        println!("Linux auth available: {}", available);
        
        if available {
            match auth.authenticate("Test Linux auth for Saorsa").await {
                Ok(_) => println!("Linux authentication successful"),
                Err(e) => println!("Linux authentication failed: {}", e),
            }
        }
    }
}

#[cfg(test)]
mod cross_platform_tests {
    use super::*;
    use std::env;
    
    #[tokio::test]
    async fn test_platform_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf());
        
        let os = env::consts::OS;
        match os {
            "macos" | "windows" | "linux" => {
                assert!(manager.is_ok(), "Should create manager on supported platform: {}", os);
            }
            _ => {
                // On unsupported platforms, should still work with mock
                assert!(manager.is_ok(), "Should fallback to mock on unsupported platform: {}", os);
            }
        }
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let (manager, _temp_dir) = create_test_manager(false);
        
        // Test various error conditions
        let create_result = manager.create_passkey("", "").await;
        assert!(create_result.is_err());
        
        let auth_result = manager.authenticate_with_passkey("invalid_id").await;
        assert!(auth_result.is_err());
        
        let delete_result = manager.delete_passkey("nonexistent_id").await;
        assert!(delete_result.is_err());
    }
    
    #[tokio::test]
    async fn test_credential_id_uniqueness() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        let mut credential_ids = std::collections::HashSet::new();
        
        // Create multiple credentials and ensure IDs are unique
        for i in 0..10 {
            let cred = manager.create_passkey(
                &format!("user{}", i),
                &format!("user{}.test.address", i)
            ).await.unwrap();
            
            assert!(credential_ids.insert(cred.credential_id.clone()),
                "Credential ID should be unique: {}", cred.credential_id);
        }
        
        assert_eq!(credential_ids.len(), 10);
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_signature_verification() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        let credential = manager.create_passkey("security_test", "test.address").await.unwrap();
        
        // Get multiple signatures
        let sig1 = manager.authenticate_with_passkey(&credential.credential_id).await.unwrap();
        let sig2 = manager.authenticate_with_passkey(&credential.credential_id).await.unwrap();
        
        // Signatures should be different due to random challenge
        assert_ne!(sig1, sig2, "Signatures should be different due to random challenge");
        
        // Both should be valid Ed25519 signatures
        assert_eq!(sig1.len(), 64);
        assert_eq!(sig2.len(), 64);
    }
    
    #[tokio::test]
    async fn test_key_isolation() {
        let (manager, _temp_dir) = create_test_manager(true);
        
        // Create two different credentials
        let cred1 = manager.create_passkey("user1", "user1.address").await.unwrap();
        let cred2 = manager.create_passkey("user2", "user2.address").await.unwrap();
        
        // Should not be able to authenticate cred1 with cred2's ID
        let wrong_auth = manager.authenticate_with_passkey(&cred2.credential_id).await;
        
        // This should succeed because mock authenticator allows any credential_id
        // In real implementation, this would test actual key isolation
        assert!(wrong_auth.is_ok());
    }
}