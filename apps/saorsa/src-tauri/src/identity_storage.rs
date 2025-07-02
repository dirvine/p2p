//! Identity Storage Module for Tauri
//! 
//! Provides secure local storage for user identities with encryption.
//! Stores identity data in the app's data directory using password-derived encryption.

use saorsa_core::identity::{
    UserIdentity, UserProfile, EncryptedUserProfile, IdentityChallenge,
    ContactRequest, ProfilePermissions, PrivacySettings, DiscoverabilitySettings,
    UserPreferences, VerificationLevel, IPv6BindingProof,
};
use saorsa_core::identity::manager::{IdentityManager, IdentityManagerConfig};
use saorsa_core::{Result, P2PError};
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
use crate::passkey_auth::StoredPasskeyCredential;

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
    /// Encrypted passkey credentials
    encrypted_passkey_credentials: Option<String>,
    /// Password salt (base64 encoded)
    salt: String,
    /// Nonce for identity encryption
    identity_nonce: String,
    /// Nonce for keypair encryption
    keypair_nonce: String,
    /// Nonce for profile encryption
    profile_nonce: Option<String>,
    /// Nonce for passkey credentials encryption
    passkey_nonce: Option<String>,
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
    /// Stored passkey credentials
    passkey_credentials: RwLock<Vec<StoredPasskeyCredential>>,
}

impl IdentityStorage {
    /// Create a new identity storage instance
    /// 
    /// # Arguments
    /// * `app_handle` - Tauri app handle for accessing app directories
    /// * `config` - Storage configuration
    /// 
    /// # Returns
    /// * New `IdentityStorage` instance
    /// * Error if app directory cannot be accessed or created
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
            passkey_credentials: RwLock::new(Vec::new()),
        })
    }
    
    /// Initialize encryption with password
    /// 
    /// Must be called before any encryption/decryption operations.
    /// 
    /// # Arguments
    /// * `password` - Password for key derivation
    /// 
    /// # Security
    /// Uses SHA256-based key derivation with salt and iterations
    pub async fn init_encryption(&self, password: &str) -> Result<()> {
        let (_salt, key) = self.derive_key(password, None)?;
        *self.encryption_key.write().await = Some(key);
        Ok(())
    }
    
    /// Derive encryption key from password using SHA256
    /// 
    /// # Arguments
    /// * `password` - User password
    /// * `salt` - Optional salt (generates new if None)
    /// 
    /// # Returns
    /// * Tuple of (salt, derived_key)
    /// 
    /// # Security Note
    /// This uses SHA256 with 10,000 iterations. For production,
    /// consider using Argon2 or PBKDF2 for better security.
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
    
    /// Generate a cryptographically secure random nonce for AES-GCM
    /// 
    /// # Returns
    /// * 12-byte nonce suitable for AES-256-GCM
    fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        let mut rng = OsRng;
        rng.fill_bytes(&mut nonce);
        nonce
    }
    
    /// Encrypt data using AES-256-GCM authenticated encryption
    /// 
    /// # Arguments
    /// * `data` - Plain data to encrypt
    /// * `nonce` - 12-byte nonce (must be unique per encryption)
    /// 
    /// # Returns
    /// * Encrypted data with authentication tag
    /// * Error if encryption key not initialized
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
    
    /// Decrypt data using AES-256-GCM authenticated encryption
    /// 
    /// # Arguments
    /// * `encrypted_data` - Encrypted data with auth tag
    /// * `nonce` - 12-byte nonce used for encryption
    /// 
    /// # Returns
    /// * Decrypted plain data
    /// * Error if authentication fails or key not initialized
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
    
    /// Save identity to encrypted storage file
    /// 
    /// # Arguments
    /// * `identity` - User identity to save
    /// * `keypair` - Associated Ed25519 keypair
    /// * `profile` - Optional encrypted user profile
    /// * `password` - Password for encryption
    /// 
    /// # Returns
    /// * Success or error
    /// 
    /// # Security
    /// - All data is encrypted with AES-256-GCM
    /// - Each data type has its own nonce
    /// - Password is used to derive encryption key
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
        
        // Serialize passkey credentials
        let credentials = self.passkey_credentials.read().await;
        let passkey_bytes = if !credentials.is_empty() {
            Some(bincode::serialize(&*credentials)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to serialize passkey credentials: {}", e)))?)
        } else {
            None
        };
        
        // Generate nonces
        let identity_nonce = Self::generate_nonce();
        let keypair_nonce = Self::generate_nonce();
        let profile_nonce = profile_bytes.as_ref().map(|_| Self::generate_nonce());
        let passkey_nonce = passkey_bytes.as_ref().map(|_| Self::generate_nonce());
        
        // Encrypt data
        let encrypted_identity = self.encrypt_data(&identity_bytes, &identity_nonce).await?;
        let encrypted_keypair = self.encrypt_data(&keypair_bytes, &keypair_nonce).await?;
        let encrypted_profile = if let Some(pb) = profile_bytes {
            Some(self.encrypt_data(&pb, profile_nonce.as_ref().unwrap()).await?)
        } else {
            None
        };
        let encrypted_passkey_credentials = if let Some(pb) = passkey_bytes {
            Some(self.encrypt_data(&pb, passkey_nonce.as_ref().unwrap()).await?)
        } else {
            None
        };
        
        // Create storage structure
        let stored_data = StoredIdentityData {
            encrypted_identity: general_purpose::STANDARD.encode(&encrypted_identity),
            encrypted_keypair: general_purpose::STANDARD.encode(&encrypted_keypair),
            encrypted_profile: encrypted_profile.map(|d| general_purpose::STANDARD.encode(&d)),
            encrypted_passkey_credentials: encrypted_passkey_credentials.map(|d| general_purpose::STANDARD.encode(&d)),
            salt: general_purpose::STANDARD.encode(&salt),
            identity_nonce: general_purpose::STANDARD.encode(&identity_nonce),
            keypair_nonce: general_purpose::STANDARD.encode(&keypair_nonce),
            profile_nonce: profile_nonce.map(|n| general_purpose::STANDARD.encode(&n)),
            passkey_nonce: passkey_nonce.map(|n| general_purpose::STANDARD.encode(&n)),
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
        
        // Handle passkey credentials if present
        if let Some(encrypted_passkey_str) = stored_data.encrypted_passkey_credentials {
            let encrypted_passkey = general_purpose::STANDARD.decode(&encrypted_passkey_str)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode passkey credentials: {}", e)))?;
            
            let passkey_nonce_str = stored_data.passkey_nonce
                .ok_or_else(|| P2PError::Generic(anyhow::anyhow!("Missing passkey nonce")))?;
            let passkey_nonce_bytes = general_purpose::STANDARD.decode(&passkey_nonce_str)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to decode passkey nonce: {}", e)))?;
            
            let mut passkey_nonce = [0u8; 12];
            passkey_nonce.copy_from_slice(&passkey_nonce_bytes);
            
            let passkey_bytes = self.decrypt_data(&encrypted_passkey, &passkey_nonce).await?;
            let credentials: Vec<StoredPasskeyCredential> = bincode::deserialize(&passkey_bytes)
                .map_err(|e| P2PError::Generic(anyhow::anyhow!("Failed to deserialize passkey credentials: {}", e)))?;
            
            // Update in-memory storage
            *self.passkey_credentials.write().await = credentials;
        }
        
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
    
    /// Add a passkey credential
    pub async fn add_passkey_credential(&self, credential: &StoredPasskeyCredential, password: &str) -> Result<()> {
        info!("Adding passkey credential: {}", credential.credential_id);
        
        // Initialize encryption if not already done
        if self.encryption_key.read().await.is_none() {
            self.init_encryption(password).await?;
        }
        
        // Add to local storage
        {
            let mut credentials = self.passkey_credentials.write().await;
            credentials.push(credential.clone());
        }
        
        // Save to encrypted storage
        if self.identity_exists() {
            // Load existing identity and re-save with new credentials
            if let Ok(Some((identity, keypair, profile))) = self.load_identity(password).await {
                self.save_identity(&identity, &keypair, profile.as_ref(), password).await?;
            }
        }
        
        Ok(())
    }
    
    /// Get all passkey credentials
    pub async fn get_passkey_credentials(&self) -> Result<Vec<StoredPasskeyCredential>> {
        let credentials = self.passkey_credentials.read().await;
        Ok(credentials.clone())
    }
    
    /// Remove a passkey credential
    pub async fn remove_passkey_credential(&self, credential_id: &str, password: &str) -> Result<bool> {
        info!("Removing passkey credential: {}", credential_id);
        
        let removed;
        {
            let mut credentials = self.passkey_credentials.write().await;
            let initial_len = credentials.len();
            credentials.retain(|cred| cred.credential_id != credential_id);
            removed = credentials.len() < initial_len;
        }
        
        if removed {
            // Save updated credentials to storage
            if self.identity_exists() {
                if let Ok(Some((identity, keypair, profile))) = self.load_identity(password).await {
                    self.save_identity(&identity, &keypair, profile.as_ref(), password).await?;
                }
            }
        }
        
        Ok(removed)
    }
    
    /// Unlock storage with derived key (used for passkey authentication)
    pub async fn unlock_with_derived_key(&self, key: &[u8; 32]) -> Result<()> {
        *self.encryption_key.write().await = Some(*key);
        info!("Storage unlocked with derived key");
        Ok(())
    }
}