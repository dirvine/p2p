// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Enhanced identity management with P2P integration

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::address::FourWordAddress;
use super::trust::{TrustManager, TrustRelationship, TrustLevel};

/// Address book entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBookEntry {
    pub address: FourWordAddress,
    pub name: String,
    pub notes: Option<String>,
    pub public_key: Option<Vec<u8>>,
    pub added_at: u64,
    pub last_seen: Option<u64>,
}

/// Enhanced identity manager with full P2P integration
#[derive(Debug)]
pub struct EnhancedIdentityManager {
    storage_path: PathBuf,
    current_identity: Option<Identity>,
    address_book: Vec<AddressBookEntry>,
    trust_manager: TrustManager,
    #[cfg(feature = "network")]
    saorsa_identity: Option<saorsa_core::Identity>,
}

/// Local identity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub four_word_address: FourWordAddress,
    pub public_key: Vec<u8>,
    pub created_at: u64,
}

impl EnhancedIdentityManager {
    /// Create a new enhanced identity manager
    pub fn new<P: AsRef<Path>>(storage_path: P) -> Self {
        Self {
            storage_path: storage_path.as_ref().to_path_buf(),
            current_identity: None,
            address_book: Vec::new(),
            trust_manager: TrustManager::new(),
            #[cfg(feature = "network")]
            saorsa_identity: None,
        }
    }
    
    /// Initialize with network identity
    #[cfg(feature = "network")]
    pub async fn initialize_with_network(&mut self) -> Result<()> {
        use saorsa_core::identity::{IdentityManager, IdentityCreationParams, SecurityLevel};
        use saorsa_core::secure_memory::SecureString;
        
        // Create identity manager
        let mut identity_manager = IdentityManager::new();
        identity_manager.initialize()
            .await
            .context("Failed to initialize identity manager")?;
        
        // Create identity with parameters
        let password = SecureString::from_str("default-password")
            .context("Failed to create secure password")?;
        
        let params = IdentityCreationParams {
            display_name: Some("Communitas User".to_string()),
            avatar_url: None,
            bio: None,
            ..Default::default()
        };
        
        // Try to create the identity, handling the known issue
        match identity_manager.create_identity(&password, params).await {
            Ok(identity) => {
                // Use the four-word address directly from saorsa
                let four_word = FourWordAddress::from_string(&identity.four_word_address)?;
                
                let local_identity = Identity {
                    name: "Network User".to_string(),
                    four_word_address: four_word,
                    public_key: vec![], // Would be populated from actual key
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                self.current_identity = Some(local_identity);
                self.saorsa_identity = Some(identity);
                Ok(())
            }
            Err(e) => {
                // Known issue with saorsa-core, create a mock identity for now
                eprintln!("Warning: Could not create network identity: {}", e);
                self.create_local_identity("Network User").await
            }
        }
    }
    
    /// Create a local identity without network
    pub async fn create_local_identity(&mut self, name: &str) -> Result<()> {
        // Generate a random four-word address for local identity
        let address = FourWordAddress::generate()
            .context("Failed to generate four-word address")?;
        
        let identity = Identity {
            name: name.to_string(),
            four_word_address: address,
            public_key: vec![0u8; 32], // Placeholder key
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.current_identity = Some(identity);
        self.save().await?;
        Ok(())
    }
    
    /// Get current identity
    pub fn current(&self) -> Option<&Identity> {
        self.current_identity.as_ref()
    }
    
    /// Get current four-word address
    pub fn get_address(&self) -> Option<&FourWordAddress> {
        self.current_identity.as_ref().map(|i| &i.four_word_address)
    }
    
    /// Add entry to address book
    pub fn add_to_address_book(&mut self, entry: AddressBookEntry) {
        // Check if address already exists
        if !self.address_book.iter().any(|e| e.address == entry.address) {
            self.address_book.push(entry);
        }
    }
    
    /// Get address book entry
    pub fn get_address_book_entry(&self, address: &FourWordAddress) -> Option<&AddressBookEntry> {
        self.address_book.iter().find(|e| &e.address == address)
    }
    
    /// Remove from address book
    pub fn remove_from_address_book(&mut self, address: &FourWordAddress) -> bool {
        let len_before = self.address_book.len();
        self.address_book.retain(|e| &e.address != address);
        self.address_book.len() < len_before
    }
    
    /// Get all address book entries
    pub fn get_address_book(&self) -> &[AddressBookEntry] {
        &self.address_book
    }
    
    /// Get trust manager
    pub fn trust_manager(&self) -> &TrustManager {
        &self.trust_manager
    }
    
    /// Get mutable trust manager
    pub fn trust_manager_mut(&mut self) -> &mut TrustManager {
        &mut self.trust_manager
    }
    
    /// Save identity data to storage
    pub async fn save(&self) -> Result<()> {
        // Ensure storage directory exists
        fs::create_dir_all(&self.storage_path)
            .await
            .context("Failed to create identity storage directory")?;
        
        // Save current identity
        if let Some(identity) = &self.current_identity {
            let identity_path = self.storage_path.join("identity.json");
            let json = serde_json::to_string_pretty(identity)
                .context("Failed to serialize identity")?;
            fs::write(identity_path, json)
                .await
                .context("Failed to save identity")?;
        }
        
        // Save address book
        if !self.address_book.is_empty() {
            let address_book_path = self.storage_path.join("address_book.json");
            let json = serde_json::to_string_pretty(&self.address_book)
                .context("Failed to serialize address book")?;
            fs::write(address_book_path, json)
                .await
                .context("Failed to save address book")?;
        }
        
        // Save trust relationships
        let relationships = self.trust_manager.export_to_vec();
        if !relationships.is_empty() {
            let trust_path = self.storage_path.join("trust.json");
            let json = serde_json::to_string_pretty(&relationships)
                .context("Failed to serialize trust relationships")?;
            fs::write(trust_path, json)
                .await
                .context("Failed to save trust relationships")?;
        }
        
        Ok(())
    }
    
    /// Load identity data from storage
    pub async fn load(&mut self) -> Result<()> {
        // Load current identity
        let identity_path = self.storage_path.join("identity.json");
        if identity_path.exists() {
            let json = fs::read_to_string(identity_path)
                .await
                .context("Failed to read identity file")?;
            self.current_identity = Some(
                serde_json::from_str(&json)
                    .context("Failed to deserialize identity")?
            );
        }
        
        // Load address book
        let address_book_path = self.storage_path.join("address_book.json");
        if address_book_path.exists() {
            let json = fs::read_to_string(address_book_path)
                .await
                .context("Failed to read address book")?;
            self.address_book = serde_json::from_str(&json)
                .context("Failed to deserialize address book")?;
        }
        
        // Load trust relationships
        let trust_path = self.storage_path.join("trust.json");
        if trust_path.exists() {
            let json = fs::read_to_string(trust_path)
                .await
                .context("Failed to read trust file")?;
            let relationships: Vec<TrustRelationship> = serde_json::from_str(&json)
                .context("Failed to deserialize trust relationships")?;
            self.trust_manager.load_from_vec(relationships);
        }
        
        Ok(())
    }
    
    /// Verify a peer's identity
    pub async fn verify_peer(&mut self, address: &FourWordAddress, method: &str) -> Result<()> {
        self.trust_manager.verify_peer(address, method.to_string())?;
        self.save().await?;
        Ok(())
    }
    
    /// Update trust level for a peer
    pub async fn update_trust(&mut self, address: &FourWordAddress, level: TrustLevel) -> Result<()> {
        self.trust_manager.update_trust_level(address, level)?;
        self.save().await?;
        Ok(())
    }
    
    /// Check if we can interact with a peer
    pub fn can_interact_with(&self, address: &FourWordAddress) -> bool {
        self.trust_manager.get_trust_level(address).allows_interaction()
    }
    
    /// Check if we can share files with a peer
    pub fn can_share_files_with(&self, address: &FourWordAddress) -> bool {
        self.trust_manager.get_trust_level(address).allows_file_sharing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_identity_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = EnhancedIdentityManager::new(temp_dir.path());
        
        assert!(manager.current().is_none());
        assert!(manager.get_address().is_none());
        assert_eq!(manager.get_address_book().len(), 0);
    }
    
    #[tokio::test]
    async fn test_local_identity_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = EnhancedIdentityManager::new(temp_dir.path());
        
        manager.create_local_identity("Test User").await.unwrap();
        
        assert!(manager.current().is_some());
        assert!(manager.get_address().is_some());
        
        let identity = manager.current().unwrap();
        assert_eq!(identity.name, "Test User");
        assert!(identity.four_word_address.is_valid());
    }
    
    #[tokio::test]
    async fn test_address_book_management() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = EnhancedIdentityManager::new(temp_dir.path());
        
        let address = FourWordAddress::generate().unwrap();
        let entry = AddressBookEntry {
            address: address.clone(),
            name: "Friend".to_string(),
            notes: Some("Test friend".to_string()),
            public_key: None,
            added_at: 0,
            last_seen: None,
        };
        
        manager.add_to_address_book(entry.clone());
        assert_eq!(manager.get_address_book().len(), 1);
        
        let retrieved = manager.get_address_book_entry(&address).unwrap();
        assert_eq!(retrieved.name, "Friend");
        
        assert!(manager.remove_from_address_book(&address));
        assert_eq!(manager.get_address_book().len(), 0);
    }
    
    #[tokio::test]
    async fn test_trust_integration() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = EnhancedIdentityManager::new(temp_dir.path());
        
        let address = FourWordAddress::generate().unwrap();
        
        // Initially unknown
        assert!(!manager.can_interact_with(&address));
        assert!(!manager.can_share_files_with(&address));
        
        // Update trust
        manager.update_trust(&address, TrustLevel::Verified).await.unwrap();
        assert!(manager.can_interact_with(&address));
        assert!(manager.can_share_files_with(&address));
        
        // Verify peer
        manager.verify_peer(&address, "manual").await.unwrap();
        let trust_level = manager.trust_manager().get_trust_level(&address);
        assert_eq!(trust_level, TrustLevel::Verified);
    }
    
    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();
        
        // Create and save
        let address = {
            let mut manager = EnhancedIdentityManager::new(&path);
            manager.create_local_identity("Test User").await.unwrap();
            
            let address = FourWordAddress::generate().unwrap();
            let entry = AddressBookEntry {
                address: address.clone(),
                name: "Friend".to_string(),
                notes: None,
                public_key: None,
                added_at: 0,
                last_seen: None,
            };
            
            manager.add_to_address_book(entry);
            manager.update_trust(&address, TrustLevel::Trusted).await.unwrap();
            manager.save().await.unwrap();
            address
        };
        
        // Load and verify
        {
            let mut manager = EnhancedIdentityManager::new(&path);
            manager.load().await.unwrap();
            
            assert!(manager.current().is_some());
            assert_eq!(manager.current().unwrap().name, "Test User");
            assert_eq!(manager.get_address_book().len(), 1);
            
            assert_eq!(manager.trust_manager().get_trust_level(&address), TrustLevel::Trusted);
        }
    }
}