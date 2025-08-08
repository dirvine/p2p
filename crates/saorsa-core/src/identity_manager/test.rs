#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_encryption_roundtrip() {
        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::High)
            .await
            .unwrap();
        
        // Test derive_encryption_key
        let password = SecureString::from_str("test_password").unwrap();
        let salt = b"test_salt_32_bytes_test_salt_32b";
        let key = manager.derive_encryption_key(&password, salt).unwrap();
        assert_eq!(key.len(), 32);
        
        // Test encrypt/decrypt
        let plaintext = b"Hello, World! This is a test message.";
        let nonce = [0u8; 12];
        
        let ciphertext = manager.encrypt_data(plaintext, &key, &nonce).unwrap();
        assert_ne!(ciphertext, plaintext);
        
        let decrypted = manager.decrypt_data(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}