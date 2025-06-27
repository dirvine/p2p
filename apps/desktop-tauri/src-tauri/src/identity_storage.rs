//! Identity Storage Module for Tauri
//! 
//! Provides secure local storage for user identities with encryption.
//! Stores identity data in the app's data directory using password-derived encryption.

use ant_core::identity::{
    UserIdentity, UserProfile, EncryptedUserProfile, IdentityChallenge,
    ContactRequest, ProfilePermissions, PrivacySettings, DiscoverabilitySettings,
    UserPreferences, VerificationLevel, IPv6BindingProof,
};
use ant_core::identity::manager::{IdentityManager, IdentityManagerConfig};
use ant_core::{Result, P2PError};
use ed25519_dalek::Keypair;
use serde::{Deserialize, Serialize};
use anyhow;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};
use rand::{RngCore, rngs::OsRng};
use base64::{Engine as _, engine::general_purpose};

/// Identity storage configuration
#[derive(Debug, Clone)]
pub struct IdentityStorageConfig {
    /// Storage file name
    pub file_name: String,
    /// Enable auto-save on changes
    pub auto_save: bool,
    /// Password for encryption (if None, will prompt user)
    pub password: Option<String>,
}

impl Default for IdentityStorageConfig {
    fn default() -> Self {
        Self {
            file_name: "identity.enc".to_string(),
            auto_save: true,
            password: None,
        }
    }
}

/// Encrypted identity data structure for storage
#[derive(Debug, Serialize, Deserialize)]
struct StoredIdentityData {
    /// Encrypted identity bytes
    encrypted_identity: String,
    /// Encrypted keypair bytes
    encrypted_keypair: String,
    /// Encrypted profile bytes
    encrypted_profile: Option<String>,
    /// Password salt (base64 encoded)
    salt: String,
    /// Nonce for identity encryption
    identity_nonce: String,
    /// Nonce for keypair encryption
    keypair_nonce: String,
    /// Nonce for profile encryption
    profile_nonce: Option<String>,
    /// Storage version for migrations
    version: u32,
}

/// Local identity storage for Tauri app
pub struct IdentityStorage {
    /// App handle for accessing app directories
    app_handle: AppHandle,
    /// Storage configuration
    config: IdentityStorageConfig,
    /// Derived encryption key
    encryption_key: RwLock<Option<[u8; 32]>>,
    /// Storage file path
    pub storage_path: PathBuf,
}

impl IdentityStorage {
    /// Create a new identity storage instance
    pub fn new(app_handle: AppHandle, config: IdentityStorageConfig) -> Result<Self> {
        // Get app data directory
        let app_dir = app_handle.path()
            .app_data_dir()
            .map_err(|e| P2PError::IO(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Failed to get app data directory: {}", e))))?;
        
        // Ensure directory exists
        std::fs::create_dir_all(&app_dir)
            .map_err(|e| P2PError::IO(e))?;
        
        let storage_path = app_dir.join(&config.file_name);
        
        Ok(Self {
            app_handle,
            config,
            encryption_key: RwLock::new(None),
            storage_path,
        })
    }
    
    /// Initialize encryption with password
    pub async fn init_encryption(&self, password: &str) -> Result<()> {
        let (_salt, key) = self.derive_key(password, None)?;
        *self.encryption_key.write().await = Some(key);
        Ok(())
    }
    
    /// Derive encryption key from password using SHA256
    /// Note: This is a simplified approach. For production, use a proper KDF like Argon2 or PBKDF2
    fn derive_key(&self, password: &str, salt: Option<Vec<u8>>) -> Result<(Vec<u8>, [u8; 32])> {
        // Generate or use provided salt
        let salt = if let Some(s) = salt {
            s
        } else {
            let mut salt = vec![0u8; 32];
            let mut rng = OsRng;
            rng.fill_bytes(&mut salt);
            salt
        };
        
        // Simple key derivation using SHA256 (not ideal for production)
        // Concatenate password and salt and hash multiple times
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(&salt);
        
        // Iterate to add computational cost
        let mut result = hasher.finalize();
        for _ in 0..10000 {
            let mut hasher = Sha256::new();
            hasher.update(&result);
            hasher.update(&salt);
            result = hasher.finalize();
        }
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        
        Ok((salt, key))
    }
    
    /// Generate a random nonce
    fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        let mut rng = OsRng;
        rng.fill_bytes(&mut nonce);
        nonce
    }
    
    /// Encrypt data using AES-256-GCM
    async fn encrypt_data(&self, data: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let key_guard = self.encryption_key.read().await;
        let key = key_guard.as_ref()
            .ok_or_else(|| P2PError::Cryptography("Encryption not initialized".to_string()))?;
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| P2PError::Cryptography(format!("Failed to create cipher: {}", e)))?;
        
        let nonce = Nonce::from_slice(nonce);
        cipher.encrypt(nonce, data)
            .map_err(|e| P2PError::Cryptography(format!("Encryption failed: {}", e)))
    }
    
    /// Decrypt data using AES-256-GCM
    async fn decrypt_data(&self, encrypted_data: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let key_guard = self.encryption_key.read().await;
        let key = key_guard.as_ref()
            .ok_or_else(|| P2PError::Cryptography("Encryption not initialized".to_string()))?;
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| P2PError::Cryptography(format!("Failed to create cipher: {}", e)))?;
        
        let nonce = Nonce::from_slice(nonce);
        cipher.decrypt(nonce, encrypted_data)
            .map_err(|e| P2PError::Cryptography(format!("Decryption failed: {}", e)))
    }
    
    /// Save identity to encrypted storage
    pub async fn save_identity(
        &self,
        identity: &UserIdentity,
        keypair: &Keypair,
        profile: Option<&EncryptedUserProfile>,
        password: &str,
    ) -> Result<()> {
        info!("Saving identity to encrypted storage");
        
        // Initialize encryption if not already done
        if self.encryption_key.read().await.is_none() {
            self.init_encryption(password).await?;
        }
        
        // Derive key with new salt
        let (salt, _) = self.derive_key(password, None)?;
        
        // Serialize data
        let identity_bytes = bincode::serialize(identity)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to serialize identity: {}", e)))?;
        let keypair_bytes = bincode::serialize(keypair)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to serialize keypair: {}", e)))?;
        let profile_bytes = if let Some(p) = profile {
            Some(bincode::serialize(p)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to serialize profile: {}", e)))?)
        } else {
            None
        };
        
        // Generate nonces
        let identity_nonce = Self::generate_nonce();
        let keypair_nonce = Self::generate_nonce();
        let profile_nonce = profile_bytes.as_ref().map(|_| Self::generate_nonce());
        
        // Encrypt data
        let encrypted_identity = self.encrypt_data(&identity_bytes, &identity_nonce).await?;
        let encrypted_keypair = self.encrypt_data(&keypair_bytes, &keypair_nonce).await?;
        let encrypted_profile = if let Some(pb) = profile_bytes {
            Some(self.encrypt_data(&pb, profile_nonce.as_ref().unwrap()).await?)
        } else {
            None
        };
        
        // Create storage structure
        let stored_data = StoredIdentityData {
            encrypted_identity: general_purpose::STANDARD.encode(&encrypted_identity),
            encrypted_keypair: general_purpose::STANDARD.encode(&encrypted_keypair),
            encrypted_profile: encrypted_profile.map(|d| general_purpose::STANDARD.encode(&d)),
            salt: general_purpose::STANDARD.encode(&salt),
            identity_nonce: general_purpose::STANDARD.encode(&identity_nonce),
            keypair_nonce: general_purpose::STANDARD.encode(&keypair_nonce),
            profile_nonce: profile_nonce.map(|n| general_purpose::STANDARD.encode(&n)),
            version: 1,
        };
        
        // Write to file
        let json = serde_json::to_string_pretty(&stored_data)
            .map_err(|e| P2PError::Serialization(e))?;
        
        std::fs::write(&self.storage_path, json)
            .map_err(|e| P2PError::IO(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to write identity file: {}", e))))?;
        
        info!("Identity saved successfully");
        Ok(())
    }
    
    /// Load identity from encrypted storage
    pub async fn load_identity(
        &self,
        password: &str,
    ) -> Result<Option<(UserIdentity, Keypair, Option<EncryptedUserProfile>)>> {
        if !self.storage_path.exists() {
            debug!("No identity file found");
            return Ok(None);
        }
        
        info!("Loading identity from encrypted storage");
        
        // Read storage file
        let json = std::fs::read_to_string(&self.storage_path)
            .map_err(|e| P2PError::IO(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to read identity file: {}", e))))?;
        
        let stored_data: StoredIdentityData = serde_json::from_str(&json)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to parse storage data: {}", e)))?;
        
        // Decode salt and derive key
        let salt = general_purpose::STANDARD.decode(&stored_data.salt)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode salt: {}", e)))?;
        let (_, key) = self.derive_key(password, Some(salt))?;
        *self.encryption_key.write().await = Some(key);
        
        // Decode base64 data
        let encrypted_identity = general_purpose::STANDARD.decode(&stored_data.encrypted_identity)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode identity: {}", e)))?;
        let encrypted_keypair = general_purpose::STANDARD.decode(&stored_data.encrypted_keypair)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode keypair: {}", e)))?;
        
        // Decode nonces
        let identity_nonce_bytes = general_purpose::STANDARD.decode(&stored_data.identity_nonce)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode identity nonce: {}", e)))?;
        let keypair_nonce_bytes = general_purpose::STANDARD.decode(&stored_data.keypair_nonce)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode keypair nonce: {}", e)))?;
        
        let mut identity_nonce = [0u8; 12];
        let mut keypair_nonce = [0u8; 12];
        identity_nonce.copy_from_slice(&identity_nonce_bytes);
        keypair_nonce.copy_from_slice(&keypair_nonce_bytes);
        
        // Decrypt data
        let identity_bytes = self.decrypt_data(&encrypted_identity, &identity_nonce).await?;
        let keypair_bytes = self.decrypt_data(&encrypted_keypair, &keypair_nonce).await?;
        
        // Deserialize
        let identity: UserIdentity = bincode::deserialize(&identity_bytes)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to deserialize identity: {}", e)))?;
        let keypair: Keypair = bincode::deserialize(&keypair_bytes)
            .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to deserialize keypair: {}", e)))?;
        
        // Handle profile if present
        let profile = if let Some(encrypted_profile_str) = stored_data.encrypted_profile {
            let encrypted_profile = general_purpose::STANDARD.decode(&encrypted_profile_str)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode profile: {}", e)))?;
            
            let profile_nonce_str = stored_data.profile_nonce
                .ok_or_else(|| P2PError::Generic(anyhow::anyhow!("Missing profile nonce")))?;
            let profile_nonce_bytes = general_purpose::STANDARD.decode(&profile_nonce_str)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode profile nonce: {}", e)))?;
            
            let mut profile_nonce = [0u8; 12];
            profile_nonce.copy_from_slice(&profile_nonce_bytes);
            
            let profile_bytes = self.decrypt_data(&encrypted_profile, &profile_nonce).await?;
            let profile: EncryptedUserProfile = bincode::deserialize(&profile_bytes)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to deserialize profile: {}", e)))?;
            
            Some(profile)
        } else {
            None
        };
        
        info!("Identity loaded successfully");
        Ok(Some((identity, keypair, profile)))
    }
    
    /// Delete stored identity
    pub async fn delete_identity(&self) -> Result<()> {
        if self.storage_path.exists() {
            std::fs::remove_file(&self.storage_path)
                .map_err(|e| P2PError::IO(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to delete identity file: {}", e))))?;
            info!("Identity deleted from storage");
        }
        
        // Clear encryption key
        *self.encryption_key.write().await = None;
        
        Ok(())
    }
    
    /// Check if identity exists
    pub fn identity_exists(&self) -> bool {
        self.storage_path.exists()
    }
}