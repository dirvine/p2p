// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Four-Word profile data structures

#![allow(unused_variables)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::identity::FourWordAddress;

/// Content types for profile entries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProfileContent {
    Website(String),     // Markdown website content
    Blog(String),        // Markdown blog content  
    BitcoinAddress(String),
    EthereumAddress(String),
    CustomData(String, String), // key, value pairs for future extension
}

/// Four-word personal profile 
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FourWordProfile {
    /// The original four words that generate the DHT hash
    pub four_words: FourWordAddress,
    /// Profile content entries
    pub content: Vec<ProfileContent>,
    /// Profile creation timestamp (Unix epoch)
    pub created_at: u64,
    /// Last update timestamp (Unix epoch) 
    pub updated_at: u64,
    /// Cryptographic signature for integrity
    pub signature: Vec<u8>,
    /// Profile version for updates
    pub version: u32,
}

impl FourWordProfile {
    /// Create a new profile with four words
    pub fn new(four_words: FourWordAddress) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        Self {
            four_words,
            content: Vec::new(),
            created_at: now,
            updated_at: now,
            signature: Vec::new(),
            version: 1,
        }
    }
    
    /// Add website content (markdown)
    pub fn with_website(mut self, website: String) -> Self {
        self.content.retain(|c| !matches!(c, ProfileContent::Website(_)));
        self.content.push(ProfileContent::Website(website));
        self.update_timestamp();
        self
    }
    
    /// Add blog content (markdown)
    pub fn with_blog(mut self, blog: String) -> Self {
        self.content.retain(|c| !matches!(c, ProfileContent::Blog(_)));
        self.content.push(ProfileContent::Blog(blog));
        self.update_timestamp();
        self
    }
    
    /// Add Bitcoin address
    pub fn with_bitcoin_address(mut self, address: String) -> Self {
        self.content.retain(|c| !matches!(c, ProfileContent::BitcoinAddress(_)));
        self.content.push(ProfileContent::BitcoinAddress(address));
        self.update_timestamp();
        self
    }
    
    /// Add Ethereum address  
    pub fn with_ethereum_address(mut self, address: String) -> Self {
        self.content.retain(|c| !matches!(c, ProfileContent::EthereumAddress(_)));
        self.content.push(ProfileContent::EthereumAddress(address));
        self.update_timestamp();
        self
    }
    
    /// Get website content if present
    pub fn get_website(&self) -> Option<&str> {
        self.content.iter().find_map(|c| match c {
            ProfileContent::Website(content) => Some(content.as_str()),
            _ => None,
        })
    }
    
    /// Get blog content if present
    pub fn get_blog(&self) -> Option<&str> {
        self.content.iter().find_map(|c| match c {
            ProfileContent::Blog(content) => Some(content.as_str()),
            _ => None,
        })
    }
    
    /// Get Bitcoin address if present
    pub fn get_bitcoin_address(&self) -> Option<&str> {
        self.content.iter().find_map(|c| match c {
            ProfileContent::BitcoinAddress(addr) => Some(addr.as_str()),
            _ => None,
        })
    }
    
    /// Get Ethereum address if present
    pub fn get_ethereum_address(&self) -> Option<&str> {
        self.content.iter().find_map(|c| match c {
            ProfileContent::EthereumAddress(addr) => Some(addr.as_str()),
            _ => None,
        })
    }
    
    /// Generate the DHT packet hash from four words
    pub fn generate_dht_hash(&self) -> Result<[u8; 32]> {
        use blake3::Hasher;
        
        // Hash the four words directly for deterministic addressing
        let mut hasher = Hasher::new();
        
        // Use the four words as the input for hash generation
        // This ensures the same four words always produce the same hash
        let four_words_str = self.four_words.as_string();
        
        hasher.update(four_words_str.as_bytes());
        let hash = hasher.finalize();
        
        Ok(*hash.as_bytes())
    }
    
    /// Sign the profile with a private key
    pub fn sign(&mut self, private_key: &[u8]) -> Result<()> {
        use blake3::Hasher;
        
        // For now, create a simple signature using BLAKE3 hash
        // In production, this should use proper ML-DSA signing
        let mut hasher = Hasher::new();
        
        // Include all profile fields except the signature itself
        hasher.update(self.four_words.to_string().as_bytes());
        
        // Serialize content entries
        for content in &self.content {
            match content {
                ProfileContent::Website(s) => {
                    hasher.update(b"website:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::Blog(s) => {
                    hasher.update(b"blog:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::BitcoinAddress(s) => {
                    hasher.update(b"bitcoin:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::EthereumAddress(s) => {
                    hasher.update(b"ethereum:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::CustomData(k, v) => {
                    hasher.update(b"custom:");
                    hasher.update(k.as_bytes());
                    hasher.update(b":");
                    hasher.update(v.as_bytes());
                }
            }
        }
        
        // Include timestamps and version
        hasher.update(&self.created_at.to_le_bytes());
        hasher.update(&self.updated_at.to_le_bytes());
        hasher.update(&self.version.to_le_bytes());
        
        // Mix in the private key
        hasher.update(private_key);
        
        // Generate signature
        let hash = hasher.finalize();
        self.signature = hash.as_bytes().to_vec();
        
        Ok(())
    }
    
    /// Verify the profile signature
    pub fn verify_signature(&self, public_key: &[u8]) -> Result<bool> {
        use blake3::Hasher;
        
        // For now, recreate the signature and compare
        // In production, this should use proper ML-DSA verification
        let mut hasher = Hasher::new();
        
        // Recreate the exact same data that was signed
        hasher.update(self.four_words.to_string().as_bytes());
        
        // Serialize content entries
        for content in &self.content {
            match content {
                ProfileContent::Website(s) => {
                    hasher.update(b"website:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::Blog(s) => {
                    hasher.update(b"blog:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::BitcoinAddress(s) => {
                    hasher.update(b"bitcoin:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::EthereumAddress(s) => {
                    hasher.update(b"ethereum:");
                    hasher.update(s.as_bytes());
                }
                ProfileContent::CustomData(k, v) => {
                    hasher.update(b"custom:");
                    hasher.update(k.as_bytes());
                    hasher.update(b":");
                    hasher.update(v.as_bytes());
                }
            }
        }
        
        // Include timestamps and version
        hasher.update(&self.created_at.to_le_bytes());
        hasher.update(&self.updated_at.to_le_bytes());
        hasher.update(&self.version.to_le_bytes());
        
        // Mix in the public key (in production would use proper verification)
        hasher.update(public_key);
        
        // Generate expected signature
        let hash = hasher.finalize();
        let expected_signature = hash.as_bytes().to_vec();
        
        // Verify signatures match
        Ok(self.signature == expected_signature)
    }
    
    /// Serialize to bytes for DHT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        // Use JSON serialization for now
        let json_str = serde_json::to_string(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize profile: {}", e))?;
        Ok(json_str.into_bytes())
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        // Use JSON deserialization for now
        let json_str = std::str::from_utf8(data)
            .map_err(|e| anyhow::anyhow!("Failed to parse bytes as UTF-8: {}", e))?;
        serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize profile: {}", e))
    }
    
    /// Update the timestamp
    fn update_timestamp(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.version += 1;
    }
    
    /// Validate profile data integrity
    pub fn validate(&self) -> Result<()> {
        // Check timestamps are valid
        if self.updated_at < self.created_at {
            return Err(anyhow::anyhow!("Updated timestamp cannot be before created timestamp"));
        }
        
        // Check version is non-zero
        if self.version == 0 {
            return Err(anyhow::anyhow!("Profile version must be greater than 0"));
        }
        
        // Validate four-word address
        if !self.four_words.is_valid() {
            return Err(anyhow::anyhow!("Four-word address is invalid"));
        }
        
        // Validate content entries
        for content in &self.content {
            match content {
                ProfileContent::BitcoinAddress(addr) => {
                    // Basic Bitcoin address validation (starts with 1, 3, or bc1)
                    if !addr.starts_with('1') && !addr.starts_with('3') && !addr.starts_with("bc1") {
                        return Err(anyhow::anyhow!("Invalid Bitcoin address format"));
                    }
                }
                ProfileContent::EthereumAddress(addr) => {
                    // Basic Ethereum address validation (starts with 0x and is 42 chars)
                    if !addr.starts_with("0x") || addr.len() != 42 {
                        return Err(anyhow::anyhow!("Invalid Ethereum address format"));
                    }
                }
                ProfileContent::Website(content) | ProfileContent::Blog(content) => {
                    // Check for excessive size (10MB limit)
                    if content.len() > 10 * 1024 * 1024 {
                        return Err(anyhow::anyhow!("Content exceeds maximum size of 10MB"));
                    }
                }
                ProfileContent::CustomData(key, value) => {
                    if key.is_empty() {
                        return Err(anyhow::anyhow!("Custom data key cannot be empty"));
                    }
                    if value.len() > 1024 * 1024 { // 1MB limit for custom data
                        return Err(anyhow::anyhow!("Custom data value exceeds 1MB limit"));
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_creation() {
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words.clone());
        
        assert_eq!(profile.four_words, four_words);
        assert!(profile.content.is_empty());
        assert_eq!(profile.version, 1);
        assert!(profile.signature.is_empty());
    }

    #[test]
    fn test_profile_with_website() {
        let four_words = FourWordAddress::generate().unwrap();
        let website_content = "# My Website\n\nWelcome to my site!".to_string();
        let profile = FourWordProfile::new(four_words)
            .with_website(website_content.clone());
        
        assert_eq!(profile.get_website(), Some(website_content.as_str()));
        assert_eq!(profile.version, 2); // Updated when content added
    }

    #[test]
    fn test_profile_with_blog() {
        let four_words = FourWordAddress::generate().unwrap();
        let blog_content = "# My Blog\n\n## First Post\n\nHello world!".to_string();
        let profile = FourWordProfile::new(four_words)
            .with_blog(blog_content.clone());
        
        assert_eq!(profile.get_blog(), Some(blog_content.as_str()));
    }

    #[test]
    fn test_profile_with_crypto_addresses() {
        let four_words = FourWordAddress::generate().unwrap();
        let bitcoin_addr = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string();
        let ethereum_addr = "0x742d35Cc6098A1E24E9b8c61AE4c2c2A8B68C4db".to_string();
        
        let profile = FourWordProfile::new(four_words)
            .with_bitcoin_address(bitcoin_addr.clone())
            .with_ethereum_address(ethereum_addr.clone());
        
        assert_eq!(profile.get_bitcoin_address(), Some(bitcoin_addr.as_str()));
        assert_eq!(profile.get_ethereum_address(), Some(ethereum_addr.as_str()));
    }

    #[test]
    fn test_profile_content_replacement() {
        let four_words = FourWordAddress::generate().unwrap();
        let website1 = "First website".to_string();
        let website2 = "Updated website".to_string();
        
        let profile = FourWordProfile::new(four_words)
            .with_website(website1)
            .with_website(website2.clone());
        
        // Should only have the most recent website
        assert_eq!(profile.get_website(), Some(website2.as_str()));
        assert_eq!(profile.content.len(), 1);
    }

    #[test]
    fn test_profile_full_content() {
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words)
            .with_website("# Website".to_string())
            .with_blog("# Blog".to_string())
            .with_bitcoin_address("1ABC123".to_string())
            .with_ethereum_address("0xABC123".to_string());
        
        assert_eq!(profile.content.len(), 4);
        assert!(profile.get_website().is_some());
        assert!(profile.get_blog().is_some());
        assert!(profile.get_bitcoin_address().is_some());
        assert!(profile.get_ethereum_address().is_some());
    }

    #[test]
    fn test_dht_hash_generation() {
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words.clone());
        
        let result = profile.generate_dht_hash();
        assert!(result.is_ok());
        
        let hash = result.unwrap();
        assert_eq!(hash.len(), 32); // BLAKE3 produces 32-byte hashes
        
        // Same four words should produce same hash
        let profile2 = FourWordProfile::new(four_words);
        let hash2 = profile2.generate_dht_hash().unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_profile_signing() {
        let four_words = FourWordAddress::generate().unwrap();
        let mut profile = FourWordProfile::new(four_words)
            .with_website("# My Website".to_string())
            .with_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string());
        
        let private_key = vec![0x42u8; 32]; // Test private key
        
        let result = profile.sign(&private_key);
        assert!(result.is_ok());
        assert!(!profile.signature.is_empty());
        assert_eq!(profile.signature.len(), 32); // BLAKE3 signature size
    }

    #[test]
    fn test_signature_verification() {
        let four_words = FourWordAddress::generate().unwrap();
        let mut profile = FourWordProfile::new(four_words)
            .with_website("# My Website".to_string());
        
        let private_key = vec![0x42u8; 32]; // Test private key
        profile.sign(&private_key).unwrap();
        
        // With matching key (our simplified scheme uses same key for sign/verify)
        let result = profile.verify_signature(&private_key);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // With different key should fail
        let wrong_key = vec![0x99u8; 32];
        let result = profile.verify_signature(&wrong_key);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_profile_serialization() {
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words.clone())
            .with_website("# Test".to_string())
            .with_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string());
        
        let result = profile.to_bytes();
        assert!(result.is_ok());
        
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        
        // Should be valid JSON
        let json_str = std::str::from_utf8(&bytes).unwrap();
        assert!(json_str.contains("four_words"));
        assert!(json_str.contains("content"));
    }

    #[test]
    fn test_profile_deserialization() {
        let four_words = FourWordAddress::generate().unwrap();
        let original = FourWordProfile::new(four_words.clone())
            .with_website("# Test".to_string());
        
        let bytes = original.to_bytes().unwrap();
        let result = FourWordProfile::from_bytes(&bytes);
        assert!(result.is_ok());
        
        let deserialized = result.unwrap();
        assert_eq!(deserialized.four_words, original.four_words);
        assert_eq!(deserialized.get_website(), original.get_website());
        
        // Invalid data should fail
        let bad_data = vec![0u8; 100];
        let result = FourWordProfile::from_bytes(&bad_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_validation() {
        let four_words = FourWordAddress::generate().unwrap();
        let mut profile = FourWordProfile::new(four_words);
        
        // Valid profile should pass
        let result = profile.validate();
        assert!(result.is_ok());
        
        // Invalid timestamp should fail
        profile.updated_at = profile.created_at - 1000;
        let result = profile.validate();
        assert!(result.is_err());
        
        // Reset and test invalid version
        profile.updated_at = profile.created_at;
        profile.version = 0;
        let result = profile.validate();
        assert!(result.is_err());
        
        // Test invalid Bitcoin address
        profile.version = 1;
        profile = profile.with_bitcoin_address("invalid_address".to_string());
        let result = profile.validate();
        assert!(result.is_err());
    }
}