
//! Passkey authentication tests

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Passkey credential for storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredPasskeyCredential {
    pub credential_id: String,
    pub public_key: Vec<u8>,
    pub counter: u32,
    pub created_at: u64,
    pub three_word_address: String,
    pub user_id: String,
}

/// Mock authenticator for testing
pub struct MockAuthenticator {
    pub should_succeed: bool,
    pub credentials: Arc<Mutex<HashMap<String, StoredPasskeyCredential>>>,
}

impl MockAuthenticator {
    pub fn new(should_succeed: bool) -> Self {
        Self {
            should_succeed,
            credentials: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn create_credential(&self, user_id: &str, three_word_address: &str) -> Result<StoredPasskeyCredential, String> {
        if !self.should_succeed {
            return Err("Authentication failed".to_string());
        }
        
        let credential = StoredPasskeyCredential {
            credential_id: format!("cred_{}", uuid::Uuid::new_v4()),
            public_key: vec![1, 2, 3, 4, 5], // Mock public key
            counter: 0,
            created_at: chrono::Utc::now().timestamp() as u64,
            three_word_address: three_word_address.to_string(),
            user_id: user_id.to_string(),
        };
        
        self.credentials.lock().unwrap()
            .insert(credential.credential_id.clone(), credential.clone());
        
        Ok(credential)
    }
    
    pub fn authenticate(&self, credential_id: &str) -> Result<Vec<u8>, String> {
        if !self.should_succeed {
            return Err("Authentication failed".to_string());
        }
        
        let mut creds = self.credentials.lock().unwrap();
        if let Some(cred) = creds.get_mut(credential_id) {
            cred.counter += 1;
            Ok(vec![0; 64]) // Mock signature
        } else {
            Err("Credential not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_authenticator_success() {
        let auth = MockAuthenticator::new(true);
        
        // Create credential
        let cred = auth.create_credential("user123", "alice.test.address").unwrap();
        assert_eq!(cred.user_id, "user123");
        assert_eq!(cred.three_word_address, "alice.test.address");
        assert_eq!(cred.counter, 0);
        
        // Authenticate
        let signature = auth.authenticate(&cred.credential_id).unwrap();
        assert_eq!(signature.len(), 64);
        
        // Counter should increment
        let creds = auth.credentials.lock().unwrap();
        assert_eq!(creds.get(&cred.credential_id).unwrap().counter, 1);
    }
    
    #[test]
    fn test_mock_authenticator_failure() {
        let auth = MockAuthenticator::new(false);
        
        // Create credential should fail
        let result = auth.create_credential("user123", "alice.test.address");
        assert!(result.is_err());
        
        // Authenticate should fail
        let result = auth.authenticate("non_existent");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_credential_serialization() {
        let cred = StoredPasskeyCredential {
            credential_id: "test_cred_123".to_string(),
            public_key: vec![1, 2, 3, 4, 5],
            counter: 42,
            created_at: 1234567890,
            three_word_address: "test.word.address".to_string(),
            user_id: "test_user".to_string(),
        };
        
        // Serialize
        let json = serde_json::to_string(&cred).unwrap();
        
        // Deserialize
        let deserialized: StoredPasskeyCredential = serde_json::from_str(&json).unwrap();
        
        // Verify
        assert_eq!(cred, deserialized);
    }
    
    #[test]
    fn test_multiple_credentials() {
        let auth = MockAuthenticator::new(true);
        
        // Create multiple credentials
        let cred1 = auth.create_credential("user1", "user1.test.address").unwrap();
        let cred2 = auth.create_credential("user2", "user2.test.address").unwrap();
        
        // Verify different IDs
        assert_ne!(cred1.credential_id, cred2.credential_id);
        
        // Authenticate both
        assert!(auth.authenticate(&cred1.credential_id).is_ok());
        assert!(auth.authenticate(&cred2.credential_id).is_ok());
        
        // Verify both exist
        let creds = auth.credentials.lock().unwrap();
        assert_eq!(creds.len(), 2);
    }
    
    #[test]
    fn test_credential_not_found() {
        let auth = MockAuthenticator::new(true);
        
        // Try to authenticate non-existent credential
        let result = auth.authenticate("non_existent_id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Credential not found");
    }
}