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

//! Git-Like Content-Addressed DHT Storage
//!
//! This module implements a git-like content addressing system on top of the DHT,
//! providing universal version control for all P2P applications.
//!
//! Key Features:
//! - BLAKE3-based content addressing for integrity and performance
//! - Git-like object model (blobs, trees, commits, tags)
//! - Branching and reference management
//! - Integration with existing DHT storage and security

use blake3;
use hex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use crate::storage::{
    DataAccessLevel, EncryptedData, ThresholdEncryptionMeta, Key,
    EnhancedDhtRecord, VersionVector, IntegrityProof, SerializationFormat,
};

/// Content hash using BLAKE3 for fast, collision-resistant addressing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Create hash from raw content (like git's SHA-1, but using BLAKE3)
    pub fn from_content(data: &[u8]) -> Self {
        Self(blake3::hash(data).into())
    }
    
    /// Create hash with type prefix for git-like object typing
    pub fn from_typed_content(object_type: ObjectType, data: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(object_type.as_bytes());
        hasher.update(b"\0");
        hasher.update(data);
        Self(hasher.finalize().into())
    }
    
    /// Convert to DHT key for storage
    pub fn to_dht_key(&self) -> Key {
        Key::from_hash(self.0)
    }
    
    /// Short form for display (like git short hashes)
    pub fn short(&self) -> String {
        hex::encode(&self.0[..8])
    }
    
    /// Full hex representation
    pub fn hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, GitContentError> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| GitContentError::InvalidHash("Invalid hex encoding".to_string()))?;
        
        if bytes.len() != 32 {
            return Err(GitContentError::InvalidHash("Hash must be 32 bytes".to_string()));
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(Self(hash))
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex())
    }
}

/// Git-like object types for different data structures
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    Blob,       // Raw content (files, messages, documents)
    Tree,       // Directory-like structure (channels, folders)
    Commit,     // State changes with history (message sends, edits)
    Tag,        // Named references (releases, bookmarks)
    Index,      // Indexes for discovery and querying
    Manifest,   // Application-specific manifests
}

impl ObjectType {
    /// Get type as byte slice for hashing
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            ObjectType::Blob => b"blob",
            ObjectType::Tree => b"tree", 
            ObjectType::Commit => b"commit",
            ObjectType::Tag => b"tag",
            ObjectType::Index => b"index",
            ObjectType::Manifest => b"manifest",
        }
    }
    
    /// Get type as string
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
            ObjectType::Tag => "tag",
            ObjectType::Index => "index",
            ObjectType::Manifest => "manifest",
        }
    }
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Universal git-like object stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitObject {
    /// Content hash of this object
    pub hash: ContentHash,
    /// Type of git object
    pub object_type: ObjectType,
    /// Size of content in bytes
    pub size: u64,
    /// Serialized object content
    pub content: Vec<u8>,
    
    // P2P specific fields
    /// Access control level for this object
    pub access_level: DataAccessLevel,
    /// When this object was created
    pub created_at: SystemTime,
    /// Who created this object
    pub creator: String, // PeerId as string
    
    // DHT replication metadata
    /// Replication factor (K=8 by default)
    pub replication_factor: u8,
    /// Time-to-live for this object
    pub ttl: Option<Duration>,
}

impl GitObject {
    /// Create a new git object with computed hash
    pub fn new(
        object_type: ObjectType,
        content: Vec<u8>,
        access_level: DataAccessLevel,
        creator: String,
        ttl: Option<Duration>,
    ) -> Self {
        let hash = ContentHash::from_typed_content(object_type.clone(), &content);
        let size = content.len() as u64;
        
        Self {
            hash,
            object_type,
            size,
            content,
            access_level,
            created_at: SystemTime::now(),
            creator,
            replication_factor: 8, // K=8 replication
            ttl,
        }
    }
    
    /// Verify content hash integrity
    pub fn verify_integrity(&self) -> bool {
        let computed_hash = ContentHash::from_typed_content(self.object_type.clone(), &self.content);
        computed_hash == self.hash
    }
    
    /// Get object age
    pub fn age(&self) -> Duration {
        SystemTime::now().duration_since(self.created_at).unwrap_or_default()
    }
    
    /// Check if object has expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            self.age() > ttl
        } else {
            false
        }
    }
}

/// Errors specific to git content addressing
#[derive(Debug, thiserror::Error)]
pub enum GitContentError {
    #[error("Invalid hash: {0}")]
    InvalidHash(String),
    
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    
    #[error("Invalid object type: expected {expected}, got {actual}")]
    InvalidObjectType { expected: String, actual: String },
    
    #[error("Corrupted data: {0}")]
    CorruptedData(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Invalid reference: {0}")]
    InvalidReference(String),
}

/// Content types for git objects in DHT storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitContentType {
    GitBlob,
    GitTree,
    GitCommit,
    GitTag,
    GitReference,
    GitIndex,
    GitManifest,
}

impl From<ObjectType> for GitContentType {
    fn from(object_type: ObjectType) -> Self {
        match object_type {
            ObjectType::Blob => GitContentType::GitBlob,
            ObjectType::Tree => GitContentType::GitTree,
            ObjectType::Commit => GitContentType::GitCommit,
            ObjectType::Tag => GitContentType::GitTag,
            ObjectType::Index => GitContentType::GitIndex,
            ObjectType::Manifest => GitContentType::GitManifest,
        }
    }
}

/// Result type for git operations
pub type GitResult<T> = Result<T, GitContentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_creation() {
        let data = b"Hello, World!";
        let hash = ContentHash::from_content(data);
        
        // BLAKE3 should be deterministic
        let hash2 = ContentHash::from_content(data);
        assert_eq!(hash, hash2);
        
        // Different data should produce different hashes
        let hash3 = ContentHash::from_content(b"Different data");
        assert_ne!(hash, hash3);
    }
    
    #[test]
    fn test_typed_content_hash() {
        let data = b"Hello, World!";
        let blob_hash = ContentHash::from_typed_content(ObjectType::Blob, data);
        let tree_hash = ContentHash::from_typed_content(ObjectType::Tree, data);
        
        // Same data with different types should produce different hashes
        assert_ne!(blob_hash, tree_hash);
    }
    
    #[test]
    fn test_hash_display() {
        let data = b"test data";
        let hash = ContentHash::from_content(data);
        
        let short = hash.short();
        let full = hash.hex();
        
        assert_eq!(short.len(), 16); // 8 bytes = 16 hex chars
        assert_eq!(full.len(), 64);  // 32 bytes = 64 hex chars
        assert!(full.starts_with(&short));
    }
    
    #[test]
    fn test_hash_round_trip() {
        let data = b"test data";
        let original_hash = ContentHash::from_content(data);
        let hex_str = original_hash.hex();
        let parsed_hash = ContentHash::from_hex(&hex_str).unwrap();
        
        assert_eq!(original_hash, parsed_hash);
    }
    
    #[test]
    fn test_object_type_bytes() {
        assert_eq!(ObjectType::Blob.as_bytes(), b"blob");
        assert_eq!(ObjectType::Tree.as_bytes(), b"tree");
        assert_eq!(ObjectType::Commit.as_bytes(), b"commit");
        assert_eq!(ObjectType::Tag.as_bytes(), b"tag");
    }
    
    #[test]
    fn test_git_object_creation() {
        let content = b"test content".to_vec();
        let access_level = DataAccessLevel::Public {
            signature: Default::default(),
            content_hash: [0u8; 32],
        };
        
        let obj = GitObject::new(
            ObjectType::Blob,
            content.clone(),
            access_level,
            "test_peer".to_string(),
            Some(Duration::from_secs(3600)),
        );
        
        assert_eq!(obj.object_type, ObjectType::Blob);
        assert_eq!(obj.content, content);
        assert_eq!(obj.size, content.len() as u64);
        assert!(obj.verify_integrity());
    }
    
    #[test]
    fn test_git_object_integrity() {
        let content = b"test content".to_vec();
        let access_level = DataAccessLevel::Public {
            signature: Default::default(),
            content_hash: [0u8; 32],
        };
        
        let mut obj = GitObject::new(
            ObjectType::Blob,
            content,
            access_level,
            "test_peer".to_string(),
            None,
        );
        
        assert!(obj.verify_integrity());
        
        // Corrupt the content
        obj.content[0] = obj.content[0].wrapping_add(1);
        assert!(!obj.verify_integrity());
    }
    
    #[test]
    fn test_git_object_expiration() {
        let content = b"test content".to_vec();
        let access_level = DataAccessLevel::Public {
            signature: Default::default(),
            content_hash: [0u8; 32],
        };
        
        // Object with very short TTL
        let obj = GitObject::new(
            ObjectType::Blob,
            content,
            access_level,
            "test_peer".to_string(),
            Some(Duration::from_millis(1)),
        );
        
        std::thread::sleep(Duration::from_millis(2));
        assert!(obj.is_expired());
        
        // Object with no TTL should never expire
        let obj_no_ttl = GitObject::new(
            ObjectType::Blob,
            b"test".to_vec(),
            DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            "test_peer".to_string(),
            None,
        );
        
        assert!(!obj_no_ttl.is_expired());
    }
}