// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Identity management module for P2P network integration

pub mod address;
pub mod trust;
pub mod manager;

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// User identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub three_word_address: Option<String>,
    pub public_key: Option<Vec<u8>>,
    pub created_at: u64,
}

/// Identity manager for handling user identities
#[derive(Debug)]
pub struct IdentityManager {
    storage_path: PathBuf,
    current_identity: Option<Identity>,
    #[cfg(feature = "network")]
    #[allow(dead_code)]
    identity_handle: Option<saorsa_core::Identity>,
}

impl IdentityManager {
    /// Create a new identity manager
    pub fn new(storage_path: PathBuf) -> Self {
        IdentityManager {
            storage_path,
            current_identity: None,
            #[cfg(feature = "network")]
            identity_handle: None,
        }
    }

    /// Initialize with saorsa-core identity when network feature is enabled
    #[cfg(feature = "network")]
    pub async fn with_saorsa_identity() -> Result<Self> {
        let storage_path = directories::ProjectDirs::from("com", "saorsa", "communitas")
            .context("Failed to get project directories")?
            .data_dir()
            .to_path_buf();

        // Create identity manager using saorsa-core
        let identity_manager = saorsa_core::IdentityManager::new(
            storage_path.clone(),
            saorsa_core::SecurityLevel::Standard,
        )
        .await
        .context("Failed to create identity manager")?;
        
        // Initialize with a secure password
        let password = saorsa_core::SecureString::from_str("default_password")
            .context("Failed to create secure password")?;
        identity_manager.initialize(&password).await
            .context("Failed to initialize identity manager")?;
        
        // Create identity with parameters
        let params = saorsa_core::IdentityCreationParams {
            display_name: Some("User".to_string()),
            avatar_url: None,
            bio: None,
            ..Default::default()
        };
        
        let identity = identity_manager.create_identity(&password, params)
            .await
            .context("Failed to create identity")?;
        
        let user_identity = Identity {
            name: "User".to_string(),
            three_word_address: Some(identity.four_word_address.clone()),
            public_key: Some(vec![]), // Will be populated from actual identity
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        Ok(IdentityManager {
            storage_path,
            current_identity: Some(user_identity),
            identity_handle: Some(identity),
        })
    }

    /// Load identity from storage
    pub async fn load(&mut self) -> Result<()> {
        let identity_file = self.storage_path.join("identity.json");
        
        if identity_file.exists() {
            let data = tokio::fs::read_to_string(&identity_file)
                .await
                .context("Failed to read identity file")?;
            
            self.current_identity = Some(
                serde_json::from_str(&data)
                    .context("Failed to parse identity data")?
            );
        }
        
        Ok(())
    }

    /// Save identity to storage
    pub async fn save(&self) -> Result<()> {
        if let Some(ref identity) = self.current_identity {
            // Ensure directory exists
            tokio::fs::create_dir_all(&self.storage_path)
                .await
                .context("Failed to create storage directory")?;
            
            let identity_file = self.storage_path.join("identity.json");
            let data = serde_json::to_string_pretty(identity)
                .context("Failed to serialize identity")?;
            
            tokio::fs::write(&identity_file, data)
                .await
                .context("Failed to write identity file")?;
        }
        
        Ok(())
    }

    /// Get current identity
    pub fn current(&self) -> Option<&Identity> {
        self.current_identity.as_ref()
    }

    /// Create a new identity
    pub async fn create_identity(&mut self, name: String) -> Result<()> {
        let identity = Identity {
            name,
            three_word_address: None,
            public_key: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.current_identity = Some(identity);
        self.save().await?;
        
        Ok(())
    }

    /// Get three-word address
    pub fn get_address(&self) -> Option<String> {
        self.current_identity.as_ref()
            .and_then(|id| id.three_word_address.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_identity_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = IdentityManager::new(temp_dir.path().to_path_buf());
        
        manager.create_identity("Alice".to_string()).await.unwrap();
        
        let identity = manager.current().unwrap();
        assert_eq!(identity.name, "Alice");
    }

    #[tokio::test]
    async fn test_identity_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();
        
        // Create and save identity
        {
            let mut manager = IdentityManager::new(path.clone());
            manager.create_identity("Bob".to_string()).await.unwrap();
        }
        
        // Load identity
        {
            let mut manager = IdentityManager::new(path);
            manager.load().await.unwrap();
            
            let identity = manager.current().unwrap();
            assert_eq!(identity.name, "Bob");
        }
    }

    #[test]
    fn test_identity_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = IdentityManager::new(temp_dir.path().to_path_buf());
        
        assert!(manager.current().is_none());
    }
}

// Re-export enhanced identity components
pub use address::FourWordAddress;
pub use trust::{TrustLevel, TrustManager, TrustRelationship};
pub use manager::{EnhancedIdentityManager, AddressBookEntry};