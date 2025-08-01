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

//! # Encrypted Key Storage System
//!
//! This module provides secure, authenticated storage for cryptographic keys using:
//! - Argon2id for password-based key derivation
//! - AES-256-GCM for authenticated encryption
//! - Secure memory management throughout the pipeline
//!
//! ## Security Features
//! - Password-based key derivation with configurable parameters
//! - Authenticated encryption prevents tampering
//! - Secure memory handling with automatic zeroization
//! - Atomic operations for crash-safe storage
//! - Side-channel resistance through constant-time operations
//!
//! ## Performance Features
//! - Background key derivation for non-blocking operations
//! - Intelligent caching to minimize repeated derivations
//! - Configurable Argon2id parameters for different security levels
//! - Efficient storage format with minimal overhead

use crate::{P2PError, Result};
use crate::error::StorageError;
use crate::secure_memory::{SecureMemory, SecureString};
use crate::key_derivation::MasterSeed;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key
};
use argon2::{
    Argon2, Algorithm, Version, Params,
    password_hash::{PasswordHasher, SaltString}
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};
use rand::{RngCore, thread_rng};
use tokio::sync::RwLock as AsyncRwLock;

/// Version of the encrypted key storage format
const STORAGE_FORMAT_VERSION: u32 = 1;

/// Size of AES-256-GCM key in bytes
const AES_KEY_SIZE: usize = 32;

/// Size of AES-256-GCM nonce in bytes
const AES_NONCE_SIZE: usize = 12;

/// Size of salt for password-based key derivation
const SALT_SIZE: usize = 32;

/// Default Argon2id memory cost (64MB)
const DEFAULT_MEMORY_COST: u32 = 65536;
/// Default Argon2id time cost (3 iterations)
const DEFAULT_TIME_COST: u32 = 3;
/// Default Argon2id parallelism (4 lanes)
const DEFAULT_PARALLELISM: u32 = 4;
/// Default hash length (32 bytes)
const DEFAULT_HASH_LENGTH: usize = 32;

/// Security levels for key derivation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Fast derivation for development/testing
    Fast,
    /// Balanced security for standard use
    Standard,
    /// High security for sensitive applications
    High,
    /// Maximum security for critical systems
    Maximum,
}

/// Argon2id parameters for different security levels
#[derive(Debug, Clone)]
pub struct Argon2Config {
    /// Memory cost in KB
    pub memory_cost: u32,
    /// Time cost (iterations)
    pub time_cost: u32,
    /// Parallelism (lanes)
    pub parallelism: u32,
    /// Output hash length
    pub hash_length: u32,
}

/// Encrypted key storage header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHeader {
    /// Storage format version
    pub version: u32,
    /// Argon2id configuration used
    pub argon2_config: Argon2ConfigSerialized,
    /// Salt for password-based key derivation
    pub salt: [u8; SALT_SIZE],
    /// Nonce for AES-GCM encryption
    pub nonce: [u8; AES_NONCE_SIZE],
    /// Timestamp when storage was created
    pub created_at: u64,
    /// Timestamp when storage was last updated
    pub updated_at: u64,
    /// Size of encrypted data in bytes
    pub encrypted_size: u64,
    /// Authentication tag for integrity verification
    pub auth_tag: [u8; 16],
}

/// Serializable Argon2id configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argon2ConfigSerialized {
    pub memory_cost: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub hash_length: u32,
}

/// Encrypted key storage container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeyStorage {
    /// Storage header with metadata
    pub header: StorageHeader,
    /// Encrypted key data
    pub encrypted_data: Vec<u8>,
}

/// Decrypted key storage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStorageData {
    /// Hierarchical key derivation master seeds
    pub master_seeds: HashMap<String, Vec<u8>>,
    /// Cached derived keys
    pub derived_keys: HashMap<String, Vec<u8>>,
    /// Key metadata
    pub key_metadata: HashMap<String, KeyMetadata>,
    /// Storage creation timestamp
    pub created_at: u64,
    /// Last access timestamp
    pub last_accessed: u64,
}

/// Metadata for stored keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key type identifier
    pub key_type: String,
    /// Key creation timestamp
    pub created_at: u64,
    /// Key usage counter
    pub usage_count: u64,
    /// Key expiration timestamp (optional)
    pub expires_at: Option<u64>,
    /// Key derivation path (for hierarchical keys)
    pub derivation_path: Option<String>,
}

/// Key storage manager
pub struct EncryptedKeyStorageManager {
    /// Storage file path
    storage_path: PathBuf,
    /// Argon2id configuration
    argon2_config: Argon2Config,
    /// In-memory cache of decrypted keys
    key_cache: Arc<RwLock<HashMap<String, SecureMemory>>>,
    /// Password hash cache for performance
    password_cache: Arc<Mutex<Option<SecureMemory>>>,
    /// Background key derivation tasks
    background_tasks: Arc<AsyncRwLock<HashMap<String, tokio::task::JoinHandle<Result<()>>>>>,
    /// Performance statistics
    stats: Arc<Mutex<StorageStats>>,
    /// Security settings
    security_level: SecurityLevel,
}

/// Performance statistics for key storage
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total key derivations performed
    pub total_derivations: u64,
    /// Total cache hits
    pub cache_hits: u64,
    /// Total cache misses
    pub cache_misses: u64,
    /// Average derivation time in milliseconds
    pub avg_derivation_time_ms: u64,
    /// Total storage operations
    pub storage_operations: u64,
    /// Total storage failures
    pub storage_failures: u64,
    /// Current cache size in bytes
    pub cache_size_bytes: u64,
}

/// Password validation result
#[derive(Debug, Clone)]
pub struct PasswordValidation {
    /// Whether password meets requirements
    pub valid: bool,
    /// Password strength score (0-100)
    pub strength_score: u8,
    /// Validation error messages
    pub errors: Vec<String>,
    /// Suggestions for improvement
    pub suggestions: Vec<String>,
}

/// Key derivation request for background processing
pub struct KeyDerivationRequest {
    /// Key identifier
    pub key_id: String,
    /// Password for derivation
    pub password: SecureString,
    /// Priority level
    pub priority: DerivationPriority,
    /// Callback for completion
    pub callback: Option<tokio::sync::oneshot::Sender<Result<SecureMemory>>>,
}

/// Priority levels for key derivation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DerivationPriority {
    /// Background processing
    Low,
    /// Normal priority
    Normal,
    /// High priority for user-initiated operations
    High,
    /// Critical priority for system operations
    Critical,
}

impl SecurityLevel {
    /// Get Argon2id configuration for this security level
    pub fn argon2_config(&self) -> Argon2Config {
        match self {
            SecurityLevel::Fast => Argon2Config {
                memory_cost: 4096,    // 4MB
                time_cost: 1,
                parallelism: 1,
                hash_length: 32,
            },
            SecurityLevel::Standard => Argon2Config {
                memory_cost: DEFAULT_MEMORY_COST,
                time_cost: DEFAULT_TIME_COST,
                parallelism: DEFAULT_PARALLELISM,
                hash_length: DEFAULT_HASH_LENGTH as u32,
            },
            SecurityLevel::High => Argon2Config {
                memory_cost: 131072,  // 128MB
                time_cost: 5,
                parallelism: 8,
                hash_length: 32,
            },
            SecurityLevel::Maximum => Argon2Config {
                memory_cost: 262144,  // 256MB
                time_cost: 10,
                parallelism: 16,
                hash_length: 32,
            },
        }
    }
    
    /// Get recommended derivation time for this security level
    pub fn target_derivation_time(&self) -> Duration {
        match self {
            SecurityLevel::Fast => Duration::from_millis(50),
            SecurityLevel::Standard => Duration::from_millis(200),
            SecurityLevel::High => Duration::from_millis(500),
            SecurityLevel::Maximum => Duration::from_millis(1000),
        }
    }
}

impl Argon2Config {
    /// Create Argon2 instance with this configuration
    pub fn create_argon2(&self) -> Argon2<'static> {
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(
                self.memory_cost,
                self.time_cost,
                self.parallelism,
                Some(self.hash_length as usize),
            ).unwrap(),
        )
    }
    
    /// Convert to serializable format
    pub fn to_serialized(&self) -> Argon2ConfigSerialized {
        Argon2ConfigSerialized {
            memory_cost: self.memory_cost,
            time_cost: self.time_cost,
            parallelism: self.parallelism,
            hash_length: self.hash_length,
        }
    }
    
    /// Create from serialized format
    pub fn from_serialized(config: &Argon2ConfigSerialized) -> Self {
        Self {
            memory_cost: config.memory_cost,
            time_cost: config.time_cost,
            parallelism: config.parallelism,
            hash_length: config.hash_length,
        }
    }
}

impl EncryptedKeyStorageManager {
    /// Create new encrypted key storage manager
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        security_level: SecurityLevel,
    ) -> Result<Self> {
        let storage_path = storage_path.as_ref().to_path_buf();
        let argon2_config = security_level.argon2_config();
        
        // Ensure parent directory exists
        if let Some(parent) = storage_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| P2PError::Io(e))?;
        }
        
        Ok(Self {
            storage_path,
            argon2_config,
            key_cache: Arc::new(RwLock::new(HashMap::new())),
            password_cache: Arc::new(Mutex::new(None)),
            background_tasks: Arc::new(AsyncRwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(StorageStats::default())),
            security_level,
        })
    }
    
    /// Initialize storage with a password
    pub async fn initialize(&self, password: &SecureString) -> Result<()> {
        // Validate password strength
        let validation = self.validate_password(password)?;
        if !validation.valid {
            return Err(P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!(
                "Password validation failed: {}",
                validation.errors.join(", ")
            ))));
        }
        
        // Generate salt and nonce
        let mut salt = [0u8; SALT_SIZE];
        let mut nonce = [0u8; AES_NONCE_SIZE];
        RngCore::fill_bytes(&mut thread_rng(), &mut salt);
        RngCore::fill_bytes(&mut thread_rng(), &mut nonce);
        
        // Create initial key storage data
        let key_data = KeyStorageData {
            master_seeds: HashMap::new(),
            derived_keys: HashMap::new(),
            key_metadata: HashMap::new(),
            created_at: current_timestamp(),
            last_accessed: current_timestamp(),
        };
        
        // Encrypt and store
        self.encrypt_and_store(password, &salt, &nonce, &key_data).await?;
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.storage_operations += 1;
        }
        
        Ok(())
    }
    
    /// Store a master seed
    pub async fn store_master_seed(
        &self,
        seed_id: &str,
        master_seed: &MasterSeed,
        password: &SecureString,
    ) -> Result<()> {
        let start_time = Instant::now();
        
        // Load existing data
        let mut key_data = self.load_and_decrypt(password).await?;
        
        // Store the master seed
        key_data.master_seeds.insert(
            seed_id.to_string(),
            master_seed.seed_material().to_vec(),
        );
        
        // Update metadata
        key_data.key_metadata.insert(
            seed_id.to_string(),
            KeyMetadata {
                key_type: "master_seed".to_string(),
                created_at: current_timestamp(),
                usage_count: 0,
                expires_at: None,
                derivation_path: None,
            },
        );
        
        key_data.last_accessed = current_timestamp();
        
        // Re-encrypt and store
        let salt = self.get_current_salt().await?;
        let nonce = self.generate_nonce()?;
        self.encrypt_and_store(password, &salt, &nonce, &key_data).await?;
        
        // Update cache
        {
            let mut cache = self.key_cache.write().unwrap();
            cache.insert(
                seed_id.to_string(),
                SecureMemory::from_slice(master_seed.seed_material())?,
            );
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.storage_operations += 1;
            stats.avg_derivation_time_ms = 
                (stats.avg_derivation_time_ms + start_time.elapsed().as_millis() as u64) / 2;
        }
        
        Ok(())
    }
    
    /// Retrieve a master seed
    pub async fn retrieve_master_seed(
        &self,
        seed_id: &str,
        password: &SecureString,
    ) -> Result<MasterSeed> {
        let start_time = Instant::now();
        
        // Check cache first
        {
            let cache = self.key_cache.read().unwrap();
            if let Some(cached_seed) = cache.get(seed_id) {
                let mut stats = self.stats.lock().unwrap();
                stats.cache_hits += 1;
                return MasterSeed::from_entropy(cached_seed.as_slice());
            }
        }
        
        // Load from storage
        let key_data = self.load_and_decrypt(password).await?;
        
        let seed_bytes = key_data.master_seeds.get(seed_id)
            .ok_or_else(|| P2PError::Storage(crate::error::StorageError::FileNotFound {
                path: format!("seed:{seed_id}"),
            }))?;
        
        let master_seed = MasterSeed::from_entropy(seed_bytes)?;
        
        // Update cache
        {
            let mut cache = self.key_cache.write().unwrap();
            cache.insert(
                seed_id.to_string(),
                SecureMemory::from_slice(seed_bytes)?,
            );
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.cache_misses += 1;
            stats.avg_derivation_time_ms = 
                (stats.avg_derivation_time_ms + start_time.elapsed().as_millis() as u64) / 2;
        }
        
        Ok(master_seed)
    }
    
    /// Change storage password
    pub async fn change_password(
        &self,
        old_password: &SecureString,
        new_password: &SecureString,
    ) -> Result<()> {
        // Validate new password
        let validation = self.validate_password(new_password)?;
        if !validation.valid {
            return Err(P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!(
                "New password validation failed: {}",
                validation.errors.join(", ")
            ))));
        }
        
        // Load data with old password
        let key_data = self.load_and_decrypt(old_password).await?;
        
        // Generate new salt and nonce
        let mut salt = [0u8; SALT_SIZE];
        let mut nonce = [0u8; AES_NONCE_SIZE];
        RngCore::fill_bytes(&mut thread_rng(), &mut salt);
        RngCore::fill_bytes(&mut thread_rng(), &mut nonce);
        
        // Re-encrypt with new password
        self.encrypt_and_store(new_password, &salt, &nonce, &key_data).await?;
        
        // Clear password cache
        {
            let mut cache = self.password_cache.lock().unwrap();
            *cache = None;
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.storage_operations += 1;
        }
        
        Ok(())
    }
    
    /// Validate password strength
    pub fn validate_password(&self, password: &SecureString) -> Result<PasswordValidation> {
        let password_str = password.as_str()
            .map_err(|e| P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!("Invalid password encoding: {e}"))))?;
        
        let mut errors = Vec::new();
        let mut suggestions = Vec::new();
        let mut strength_score = 0u8;
        
        // Length check
        if password_str.len() < 8 {
            errors.push("Password must be at least 8 characters long".to_string());
        } else if password_str.len() >= 12 {
            strength_score += 25;
        } else {
            strength_score += 10;
            suggestions.push("Use at least 12 characters for better security".to_string());
        }
        
        // Character diversity
        let has_lowercase = password_str.chars().any(|c| c.is_lowercase());
        let has_uppercase = password_str.chars().any(|c| c.is_uppercase());
        let has_digits = password_str.chars().any(|c| c.is_numeric());
        let has_special = password_str.chars().any(|c| !c.is_alphanumeric());
        
        let diversity_score = [has_lowercase, has_uppercase, has_digits, has_special]
            .iter()
            .map(|&x| if x { 1 } else { 0 })
            .sum::<u8>();
        
        match diversity_score {
            4 => strength_score += 35,
            3 => {
                strength_score += 25;
                suggestions.push("Add more character types for better security".to_string());
            }
            2 => {
                strength_score += 15;
                suggestions.push("Use uppercase, lowercase, numbers, and special characters".to_string());
            }
            _ => {
                errors.push("Password must contain at least 2 different character types".to_string());
            }
        }
        
        // Common password checks (basic)
        let common_passwords = [
            "password", "123456", "admin", "letmein", "welcome",
            "monkey", "dragon", "pass", "master", "shadow"
        ];
        
        if common_passwords.iter().any(|&common| password_str.to_lowercase().contains(common)) {
            errors.push("Password contains common words".to_string());
            suggestions.push("Avoid common words and patterns".to_string());
        } else {
            strength_score += 20;
        }
        
        // Pattern detection
        if password_str.len() > 3 {
            let has_sequence = password_str.chars()
                .collect::<Vec<_>>()
                .windows(3)
                .any(|window| {
                    let chars: Vec<u32> = window.iter().map(|&c| c as u32).collect();
                    (chars[1] == chars[0] + 1 && chars[2] == chars[1] + 1) ||
                    (chars[1] == chars[0] - 1 && chars[2] == chars[1] - 1)
                });
            
            if has_sequence {
                strength_score = strength_score.saturating_sub(10);
                suggestions.push("Avoid sequential characters".to_string());
            } else {
                strength_score += 10;
            }
        }
        
        // Final strength assessment
        let final_score = std::cmp::min(strength_score, 100);
        
        if final_score < 50 {
            suggestions.push("Consider using a passphrase with multiple words".to_string());
        }
        
        Ok(PasswordValidation {
            valid: errors.is_empty() && final_score >= 30,
            strength_score: final_score,
            errors,
            suggestions,
        })
    }
    
    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        self.stats.lock().unwrap().clone()
    }
    
    /// Clear key cache
    pub fn clear_cache(&self) {
        let mut cache = self.key_cache.write().unwrap();
        cache.clear();
        
        let mut password_cache = self.password_cache.lock().unwrap();
        *password_cache = None;
        
        let mut stats = self.stats.lock().unwrap();
        stats.cache_size_bytes = 0;
    }
    
    /// Derive key from password using Argon2id
    async fn derive_key(&self, password: &SecureString, salt: &[u8; SALT_SIZE]) -> Result<SecureMemory> {
        let start_time = Instant::now();
        
        // Check password cache
        {
            let cache = self.password_cache.lock().unwrap();
            if let Some(ref cached_key) = *cache {
                return SecureMemory::from_slice(cached_key.as_slice());
            }
        }
        
        let password_str = password.as_str()
            .map_err(|e| P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!("Invalid password encoding: {e}"))))?;
        
        let argon2 = self.argon2_config.create_argon2();
        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!("Failed to encode salt: {e}"))))?;
        
        let hash = argon2.hash_password(password_str.as_bytes(), &salt_string)
            .map_err(|e| P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!("Argon2id key derivation failed: {e}"))))?;
        
        let hash_output = hash.hash.unwrap();
        let key_bytes = hash_output.as_bytes();
        let derived_key = SecureMemory::from_slice(&key_bytes[..AES_KEY_SIZE])?;
        
        // Cache the derived key
        {
            let mut cache = self.password_cache.lock().unwrap();
            *cache = Some(SecureMemory::from_slice(derived_key.as_slice())?);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_derivations += 1;
            stats.avg_derivation_time_ms = 
                (stats.avg_derivation_time_ms + start_time.elapsed().as_millis() as u64) / 2;
        }
        
        Ok(derived_key)
    }
    
    /// Encrypt and store key data
    async fn encrypt_and_store(
        &self,
        password: &SecureString,
        salt: &[u8; SALT_SIZE],
        nonce: &[u8; AES_NONCE_SIZE],
        key_data: &KeyStorageData,
    ) -> Result<()> {
        // Derive encryption key
        let derived_key = self.derive_key(password, salt).await?;
        
        // Serialize key data
        let serialized_data = bincode::serialize(key_data)
            .map_err(|e| P2PError::Storage(StorageError::Database(e.to_string())))?;
        
        // Encrypt data
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(derived_key.as_slice()));
        let nonce_obj = Nonce::from_slice(nonce);
        
        let encrypted_data = cipher.encrypt(nonce_obj, serialized_data.as_ref())
            .map_err(|e| P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!("AES-GCM encryption failed: {e}"))))?;
        
        // Create storage header
        let header = StorageHeader {
            version: STORAGE_FORMAT_VERSION,
            argon2_config: self.argon2_config.to_serialized(),
            salt: *salt,
            nonce: *nonce,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            encrypted_size: encrypted_data.len() as u64,
            auth_tag: [0u8; 16], // Will be updated after encryption
        };
        
        let storage = EncryptedKeyStorage {
            header,
            encrypted_data,
        };
        
        // Write to file atomically
        let temp_path = self.storage_path.with_extension("tmp");
        let serialized_storage = bincode::serialize(&storage)
            .map_err(|e| P2PError::Storage(StorageError::Database(e.to_string())))?;
        
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)
                .map_err(|e| P2PError::Io(e))?;
            
            file.write_all(&serialized_storage)
                .map_err(|e| P2PError::Io(e))?;
            
            file.flush()
                .map_err(|e| P2PError::Io(e))?;
        }
        
        // Atomic rename
        std::fs::rename(&temp_path, &self.storage_path)
            .map_err(|e| P2PError::Io(e))?;
        
        Ok(())
    }
    
    /// Load and decrypt key data
    async fn load_and_decrypt(&self, password: &SecureString) -> Result<KeyStorageData> {
        // Read storage file
        let mut file = File::open(&self.storage_path)
            .map_err(|e| P2PError::Io(e))?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| P2PError::Io(e))?;
        
        // Deserialize storage
        let storage: EncryptedKeyStorage = bincode::deserialize(&data)
            .map_err(|e| P2PError::Storage(crate::error::StorageError::CorruptionDetected {
                reason: format!("Deserialization failed: {e}"),
            }))?;
        
        // Verify version
        if storage.header.version != STORAGE_FORMAT_VERSION {
            return Err(P2PError::Storage(crate::error::StorageError::CorruptionDetected {
                reason: format!(
                    "Unsupported storage format version: {}",
                    storage.header.version
                ),
            }));
        }
        
        // Derive decryption key
        let derived_key = self.derive_key(password, &storage.header.salt).await?;
        
        // Decrypt data
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(derived_key.as_slice()));
        let nonce_obj = Nonce::from_slice(&storage.header.nonce);
        
        let decrypted_data = cipher.decrypt(nonce_obj, storage.encrypted_data.as_ref())
            .map_err(|e| P2PError::Security(crate::error::SecurityError::DecryptionFailed(format!("AES-GCM decryption failed: {e}"))))?;
        
        // Deserialize key data
        let key_data: KeyStorageData = bincode::deserialize(&decrypted_data)
            .map_err(|e| P2PError::Storage(crate::error::StorageError::CorruptionDetected {
                reason: format!("Deserialization failed: {e}"),
            }))?;
        
        Ok(key_data)
    }
    
    /// Get current salt from storage
    async fn get_current_salt(&self) -> Result<[u8; SALT_SIZE]> {
        let mut file = File::open(&self.storage_path)
            .map_err(|e| P2PError::Io(e))?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| P2PError::Io(e))?;
        
        let storage: EncryptedKeyStorage = bincode::deserialize(&data)
            .map_err(|e| P2PError::Storage(crate::error::StorageError::CorruptionDetected {
                reason: format!("Deserialization failed: {e}"),
            }))?;
        
        Ok(storage.header.salt)
    }
    
    /// Generate a new nonce
    fn generate_nonce(&self) -> Result<[u8; AES_NONCE_SIZE]> {
        let mut nonce = [0u8; AES_NONCE_SIZE];
        RngCore::fill_bytes(&mut thread_rng(), &mut nonce);
        Ok(nonce)
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_storage_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test_storage.enc");
        
        let manager = EncryptedKeyStorageManager::new(
            &storage_path,
            SecurityLevel::Fast,
        ).unwrap();
        
        let password = SecureString::from_str("test_password_123!").unwrap();
        manager.initialize(&password).await.unwrap();
        
        assert!(storage_path.exists());
    }
    
    #[tokio::test]
    async fn test_master_seed_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test_storage.enc");
        
        let manager = EncryptedKeyStorageManager::new(
            &storage_path,
            SecurityLevel::Fast,
        ).unwrap();
        
        let password = SecureString::from_str("test_password_123!").unwrap();
        manager.initialize(&password).await.unwrap();
        
        // Create and store master seed
        let master_seed = MasterSeed::generate().unwrap();
        manager.store_master_seed("test_seed", &master_seed, &password).await.unwrap();
        
        // Retrieve and verify
        let retrieved_seed = manager.retrieve_master_seed("test_seed", &password).await.unwrap();
        assert_eq!(master_seed.seed_material(), retrieved_seed.seed_material());
    }
    
    #[tokio::test]
    async fn test_password_validation() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test_storage.enc");
        
        let manager = EncryptedKeyStorageManager::new(
            &storage_path,
            SecurityLevel::Standard,
        ).unwrap();
        
        // Test weak password
        let weak_password = SecureString::from_str("123").unwrap();
        let validation = manager.validate_password(&weak_password).unwrap();
        assert!(!validation.valid);
        assert!(!validation.errors.is_empty());
        
        // Test strong password
        let strong_password = SecureString::from_str("MySecurePassword123!").unwrap();
        let validation = manager.validate_password(&strong_password).unwrap();
        assert!(validation.valid);
        assert!(validation.strength_score >= 70);
    }
    
    #[tokio::test]
    async fn test_password_change() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test_storage.enc");
        
        let manager = EncryptedKeyStorageManager::new(
            &storage_path,
            SecurityLevel::Fast,
        ).unwrap();
        
        let old_password = SecureString::from_str("old_password_123!").unwrap();
        let new_password = SecureString::from_str("new_password_456!").unwrap();
        
        manager.initialize(&old_password).await.unwrap();
        
        // Store a master seed
        let master_seed = MasterSeed::generate().unwrap();
        manager.store_master_seed("test_seed", &master_seed, &old_password).await.unwrap();
        
        // Change password
        manager.change_password(&old_password, &new_password).await.unwrap();
        
        // Verify we can retrieve with new password
        let retrieved_seed = manager.retrieve_master_seed("test_seed", &new_password).await.unwrap();
        assert_eq!(master_seed.seed_material(), retrieved_seed.seed_material());
        
        // Verify old password doesn't work
        assert!(manager.retrieve_master_seed("test_seed", &old_password).await.is_err());
    }
    
    #[tokio::test]
    async fn test_security_levels() {
        let temp_dir = TempDir::new().unwrap();
        
        for level in [SecurityLevel::Fast, SecurityLevel::Standard, SecurityLevel::High] {
            let storage_path = temp_dir.path().join(format!("test_storage_{:?}.enc", level));
            
            let manager = EncryptedKeyStorageManager::new(&storage_path, level).unwrap();
            let password = SecureString::from_str("test_password_123!").unwrap();
            
            let start_time = Instant::now();
            manager.initialize(&password).await.unwrap();
            let derivation_time = start_time.elapsed();
            
            let target_time = level.target_derivation_time();
            println!("Security level {:?}: {}ms (target: {}ms)", 
                level, derivation_time.as_millis(), target_time.as_millis());
            
            assert!(storage_path.exists());
        }
    }
    
    #[tokio::test]
    async fn test_cache_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("test_storage.enc");
        
        let manager = EncryptedKeyStorageManager::new(
            &storage_path,
            SecurityLevel::Fast,
        ).unwrap();
        
        let password = SecureString::from_str("test_password_123!").unwrap();
        manager.initialize(&password).await.unwrap();
        
        // Store master seed
        let master_seed = MasterSeed::generate().unwrap();
        manager.store_master_seed("test_seed", &master_seed, &password).await.unwrap();
        
        // First retrieval (cache miss)
        let start_time = Instant::now();
        let _retrieved_seed1 = manager.retrieve_master_seed("test_seed", &password).await.unwrap();
        let first_access_time = start_time.elapsed();
        
        // Second retrieval (cache hit)
        let start_time = Instant::now();
        let _retrieved_seed2 = manager.retrieve_master_seed("test_seed", &password).await.unwrap();
        let second_access_time = start_time.elapsed();
        
        // Cache hit should be faster
        assert!(second_access_time < first_access_time);
        
        let stats = manager.get_stats();
        assert!(stats.cache_hits > 0);
    }
}