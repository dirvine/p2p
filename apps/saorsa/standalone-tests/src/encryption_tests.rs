// Copyright 2024 MaidSafe Limited
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

//! Encryption and security tests

use sha2::{Sha256, Digest};

/// Derive encryption key from password (simplified version)
#[allow(dead_code)]
fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    
    // Iterate to add computational cost
    let mut result = hasher.finalize();
    for _ in 0..10000 {
        let mut hasher = Sha256::new();
        hasher.update(result);
        hasher.update(salt);
        result = hasher.finalize();
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use rand::{RngCore, rngs::OsRng};
    use ed25519_dalek::{SigningKey, Signer, Verifier};
    
    #[test]
    fn test_key_derivation() {
        let password = "test_password_123!";
        let salt = b"random_salt_value";
        
        // Derive key
        let key1 = derive_key(password, salt);
        let key2 = derive_key(password, salt);
        
        // Same password and salt should produce same key
        assert_eq!(key1, key2);
        
        // Different password should produce different key
        let key3 = derive_key("different_password", salt);
        assert_ne!(key1, key3);
        
        // Different salt should produce different key
        let key4 = derive_key(password, b"different_salt");
        assert_ne!(key1, key4);
    }
    
    #[test]
    fn test_aes_encryption_decryption() {
        let key = [42u8; 32]; // Test key
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        
        let plaintext = b"Secret message to encrypt";
        let nonce_bytes = [1u8; 12]; // Test nonce
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
        
        // Ciphertext should be different from plaintext
        assert_ne!(&ciphertext[..], plaintext);
        
        // Decrypt
        let decrypted = cipher.decrypt(nonce, ciphertext.as_ref()).unwrap();
        
        // Decrypted should match original
        assert_eq!(&decrypted[..], plaintext);
    }
    
    #[test]
    fn test_aes_authentication() {
        let key = [42u8; 32];
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        
        let plaintext = b"Secret message";
        let nonce = Nonce::from_slice(&[1u8; 12]);
        
        // Encrypt
        let mut ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
        
        // Tamper with ciphertext
        ciphertext[0] ^= 1;
        
        // Decryption should fail due to authentication
        let result = cipher.decrypt(nonce, ciphertext.as_ref());
        assert!(result.is_err());
    }
    
    #[test]
    fn test_ed25519_signatures() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        
        let message = b"Message to sign";
        
        // Sign message
        let signature = signing_key.sign(message);
        
        // Verify with correct public key
        assert!(verifying_key.verify(message, &signature).is_ok());
        
        // Verify with tampered message should fail
        let tampered = b"Tampered message";
        assert!(verifying_key.verify(tampered, &signature).is_err());
        
        // Verify with different keypair should fail
        let other_signing_key = SigningKey::generate(&mut csprng);
        let other_verifying_key = other_signing_key.verifying_key();
        assert!(other_verifying_key.verify(message, &signature).is_err());
    }
    
    #[test]
    fn test_secure_random_generation() {
        let mut rng = OsRng;
        
        // Generate multiple random values
        let mut values = Vec::new();
        for _ in 0..10 {
            let mut buffer = [0u8; 32];
            rng.fill_bytes(&mut buffer);
            values.push(buffer);
        }
        
        // All values should be unique (extremely high probability)
        for i in 0..values.len() {
            for j in (i + 1)..values.len() {
                assert_ne!(values[i], values[j], "Random values should be unique");
            }
        }
    }
    
    #[test]
    fn test_nonce_uniqueness() {
        let mut nonces = Vec::new();
        let mut rng = OsRng;
        
        // Generate multiple nonces
        for _ in 0..100 {
            let mut nonce = [0u8; 12];
            rng.fill_bytes(&mut nonce);
            nonces.push(nonce);
        }
        
        // Check uniqueness
        for i in 0..nonces.len() {
            for j in (i + 1)..nonces.len() {
                assert_ne!(nonces[i], nonces[j], "Nonces must be unique");
            }
        }
    }
    
    #[test]
    fn test_password_key_derivation_timing() {
        use std::time::Instant;
        
        let password = "test_password";
        let salt = b"salt_value";
        
        let start = Instant::now();
        let _key = derive_key(password, salt);
        let duration = start.elapsed();
        
        // Key derivation should take some time (due to iterations)
        // but not too long
        assert!(duration.as_millis() > 1, "Key derivation too fast");
        assert!(duration.as_secs() < 1, "Key derivation too slow");
    }
}