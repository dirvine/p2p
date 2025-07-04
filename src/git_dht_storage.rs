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

//! Git-DHT Storage Integration
//!
//! This module provides the GitDhtStorage layer that integrates git-like content
//! addressing with the existing DHT storage system, enabling git operations
//! to work seamlessly with the distributed hash table.

use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use crate::git_content_addressing::{
    ContentHash, ObjectType, GitObject, GitResult, GitContentError, GitContentType,
};
use crate::git_objects::{
    BlobObject, TreeObject, CommitObject, TagObject, Reference, ReferenceType,
    CommitAuthor, CommitType, BranchState, TreeEntry,
};
use crate::storage::{
    DataAccessLevel, EncryptedData, ThresholdEncryptionMeta, Key,
    EnhancedDhtRecord, VersionVector, IntegrityProof, SerializationFormat,
    SerializationManager, ContentType, AccessContext,
};

/// Git-aware DHT storage manager
pub struct GitDhtStorage {
    /// Reference to the DHT storage system
    pub dht: Arc<dyn DhtStorageProvider>,
    /// LRU cache for git objects
    pub object_cache: Arc<RwLock<LruCache<ContentHash, GitObject>>>,
    /// LRU cache for references
    pub ref_cache: Arc<RwLock<LruCache<String, Reference>>>,
    /// Serialization manager
    pub serialization: SerializationManager,
    /// Local peer ID
    pub local_peer_id: String,
}

/// Trait for DHT storage providers
pub trait DhtStorageProvider: Send + Sync {
    /// Store a record in the DHT
    async fn store_secure_record(&self, record: EnhancedDhtRecord) -> GitResult<()>;
    
    /// Retrieve a record from DHT with K-consistency
    async fn get_secure_record_with_k_consistency(
        &self,
        key: &Key,
        requester: &str,
        context: &AccessContext,
    ) -> GitResult<Option<EnhancedDhtRecord>>;
    
    /// Get local peer ID
    fn local_id(&self) -> &str;
}

impl GitDhtStorage {
    /// Create new git-DHT storage
    pub fn new(
        dht: Arc<dyn DhtStorageProvider>,
        cache_size: usize,
        local_peer_id: String,
    ) -> Self {
        let object_cache_size = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());
        let ref_cache_size = NonZeroUsize::new(cache_size / 4).unwrap_or(NonZeroUsize::new(250).unwrap());
        
        Self {
            dht,
            object_cache: Arc::new(RwLock::new(LruCache::new(object_cache_size))),
            ref_cache: Arc::new(RwLock::new(LruCache::new(ref_cache_size))),
            serialization: SerializationManager::new(),
            local_peer_id,
        }
    }
    
    /// Store git object with content-addressed key
    pub async fn store_object(&self, object: GitObject) -> GitResult<ContentHash> {
        let hash = object.hash.clone();
        
        // Verify object integrity
        if !object.verify_integrity() {
            return Err(GitContentError::CorruptedData(
                "Object hash does not match content".to_string()
            ));
        }
        
        // Serialize object
        let serialized = self.serialization
            .serialize(&object, Some(SerializationFormat::Bincode))
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        
        // Create enhanced DHT record
        let record = EnhancedDhtRecord {
            key: hash.to_dht_key(),
            value: serialized,
            publisher: object.creator.clone(),
            created_at: object.created_at,
            expires_at: object.created_at + object.ttl.unwrap_or(Duration::from_secs(365 * 24 * 60 * 60)),
            access_level: object.access_level.clone(),
            content_type: self.object_type_to_content_type(&object.object_type),
            version_vector: VersionVector::new(),
            parent_hash: None,
            application_metadata: self.create_git_metadata(&object)?,
            integrity_proof: IntegrityProof::ContentAddressed { hash: hash.clone() },
            threshold_signatures: Vec::new(),
        };
        
        // Store with K=8 replication
        self.dht.store_secure_record(record).await
            .map_err(|e| GitContentError::StorageError(e.to_string()))?;
        
        // Cache locally
        if let Ok(mut cache) = self.object_cache.write() {
            cache.put(hash.clone(), object);
        }
        
        Ok(hash)
    }
    
    /// Retrieve git object by content hash
    pub async fn get_object(&self, hash: &ContentHash) -> GitResult<Option<GitObject>> {
        // Check cache first
        if let Ok(mut cache) = self.object_cache.write() {
            if let Some(object) = cache.get(hash) {
                return Ok(Some(object.clone()));
            }
        }
        
        // Query DHT with K=8 consistency
        let dht_key = hash.to_dht_key();
        let access_context = AccessContext::default();
        
        match self.dht.get_secure_record_with_k_consistency(&dht_key, &self.local_peer_id, &access_context).await {
            Ok(Some(record)) => {
                let object: GitObject = self.serialization
                    .deserialize(&record.value, SerializationFormat::Bincode)
                    .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
                
                // Verify content hash integrity
                if object.hash != *hash {
                    return Err(GitContentError::CorruptedData(
                        format!("Hash mismatch: expected {}, got {}", hash.hex(), object.hash.hex())
                    ));
                }
                
                // Cache the result
                if let Ok(mut cache) = self.object_cache.write() {
                    cache.put(hash.clone(), object.clone());
                }
                
                Ok(Some(object))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    /// Store/update reference (branch, tag, etc.)
    pub async fn store_reference(&self, reference: Reference) -> GitResult<()> {
        let ref_key = format!("ref:{}:{}", reference.namespace, reference.name);
        
        // Determine access level based on reference type
        let access_level = match reference.ref_type {
            ReferenceType::Branch | ReferenceType::Head => DataAccessLevel::GroupShared {
                encrypted_data: EncryptedData::default(),
                threshold_metadata: ThresholdEncryptionMeta::default(),
                group_id: reference.namespace.clone(),
                required_shares: 2,
            },
            ReferenceType::Tag => DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            ReferenceType::Remote => DataAccessLevel::UserPrivate {
                encrypted_data: EncryptedData::default(),
                ml_kem_session_key: Vec::new(),
                user_key_id: reference.updated_by.clone(),
            },
        };
        
        // Serialize reference
        let serialized = self.serialization
            .serialize(&reference, Some(SerializationFormat::Bincode))
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        
        // Create DHT record for reference
        let record = EnhancedDhtRecord {
            key: Key::from_string(&ref_key),
            value: serialized,
            publisher: reference.updated_by.clone(),
            created_at: reference.last_updated,
            expires_at: reference.last_updated + Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            access_level,
            content_type: ContentType::GitReference,
            version_vector: VersionVector::new(),
            parent_hash: None,
            application_metadata: HashMap::new(),
            integrity_proof: IntegrityProof::None,
            threshold_signatures: Vec::new(),
        };
        
        // Store reference
        self.dht.store_secure_record(record).await
            .map_err(|e| GitContentError::StorageError(e.to_string()))?;
        
        // Cache reference
        if let Ok(mut cache) = self.ref_cache.write() {
            cache.put(ref_key, reference);
        }
        
        Ok(())
    }
    
    /// Get reference by name
    pub async fn get_reference(&self, namespace: &str, name: &str) -> GitResult<Option<Reference>> {
        let ref_key = format!("ref:{}:{}", namespace, name);
        
        // Check cache
        if let Ok(mut cache) = self.ref_cache.write() {
            if let Some(reference) = cache.get(&ref_key) {
                return Ok(Some(reference.clone()));
            }
        }
        
        // Query DHT
        let key = Key::from_string(&ref_key);
        let access_context = AccessContext::default();
        
        match self.dht.get_secure_record_with_k_consistency(&key, &self.local_peer_id, &access_context).await {
            Ok(Some(record)) => {
                let reference: Reference = self.serialization
                    .deserialize(&record.value, SerializationFormat::Bincode)
                    .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
                
                // Cache the result
                if let Ok(mut cache) = self.ref_cache.write() {
                    cache.put(ref_key, reference.clone());
                }
                
                Ok(Some(reference))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    /// Traverse git object tree (like git ls-tree)
    pub async fn traverse_tree(&self, tree_hash: &ContentHash) -> GitResult<Vec<(String, GitObject)>> {
        let tree_object = self.get_object(tree_hash).await?
            .ok_or_else(|| GitContentError::ObjectNotFound(tree_hash.hex()))?;
        
        if tree_object.object_type != ObjectType::Tree {
            return Err(GitContentError::InvalidObjectType {
                expected: "tree".to_string(),
                actual: tree_object.object_type.to_string(),
            });
        }
        
        let tree: TreeObject = bincode::deserialize(&tree_object.content)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        
        let mut results = Vec::new();
        
        for entry in tree.entries {
            if let Some(object) = self.get_object(&entry.hash).await? {
                results.push((entry.name, object));
            }
        }
        
        Ok(results)
    }
    
    /// Get commit history (like git log)
    pub async fn get_commit_history(&self, start_hash: &ContentHash, limit: usize) -> GitResult<Vec<CommitObject>> {
        let mut history = Vec::new();
        let mut current_hash = *start_hash;
        let mut visited = HashSet::new();
        
        while history.len() < limit && !visited.contains(&current_hash) {
            visited.insert(current_hash);
            
            let commit_object = self.get_object(&current_hash).await?
                .ok_or_else(|| GitContentError::ObjectNotFound(current_hash.hex()))?;
            
            if commit_object.object_type != ObjectType::Commit {
                break;
            }
            
            let commit: CommitObject = bincode::deserialize(&commit_object.content)
                .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
            
            history.push(commit.clone());
            
            // Follow first parent for linear history
            if let Some(parent_hash) = commit.parents.first() {
                current_hash = *parent_hash;
            } else {
                break;
            }
        }
        
        Ok(history)
    }
    
    /// Create new commit (like git commit)
    pub async fn create_commit(
        &self,
        tree: TreeObject,
        parents: Vec<ContentHash>,
        message: String,
        author: CommitAuthor,
        application: String,
        namespace: String,
        commit_type: CommitType,
    ) -> GitResult<ContentHash> {
        // Store tree object first
        let tree_content = bincode::serialize(&tree)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let tree_hash = ContentHash::from_typed_content(ObjectType::Tree, &tree_content);
        
        let tree_object = GitObject::new(
            ObjectType::Tree,
            tree_content,
            DataAccessLevel::GroupShared {
                encrypted_data: EncryptedData::default(),
                threshold_metadata: ThresholdEncryptionMeta::default(),
                group_id: namespace.clone(),
                required_shares: 2,
            },
            author.peer_id.clone(),
            Some(Duration::from_secs(365 * 24 * 60 * 60)),
        );
        
        self.store_object(tree_object).await?;
        
        // Create commit object
        let commit = CommitObject::new(
            tree_hash,
            parents,
            message,
            author.clone(),
            application,
            namespace.clone(),
            commit_type,
        );
        
        let commit_content = bincode::serialize(&commit)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let commit_hash = ContentHash::from_typed_content(ObjectType::Commit, &commit_content);
        
        let commit_object = GitObject::new(
            ObjectType::Commit,
            commit_content,
            DataAccessLevel::GroupShared {
                encrypted_data: EncryptedData::default(),
                threshold_metadata: ThresholdEncryptionMeta::default(),
                group_id: namespace,
                required_shares: 2,
            },
            author.peer_id,
            Some(Duration::from_secs(365 * 24 * 60 * 60)),
        );
        
        self.store_object(commit_object).await?;
        
        Ok(commit_hash)
    }
    
    /// Update branch head (like git branch update)
    pub async fn update_branch(&self, namespace: &str, branch_name: &str, new_head: ContentHash) -> GitResult<()> {
        // Get current branch state
        let mut reference = if let Some(existing) = self.get_reference(namespace, branch_name).await? {
            existing
        } else {
            // Create new branch
            Reference::new_branch(
                branch_name.to_string(),
                new_head,
                namespace.to_string(),
                self.local_peer_id.clone(),
            )
        };
        
        // Update branch head
        reference.update(new_head, self.local_peer_id.clone());
        
        // Store updated reference
        self.store_reference(reference).await?;
        
        Ok(())
    }
    
    // Helper methods
    
    fn object_type_to_content_type(&self, object_type: &ObjectType) -> ContentType {
        match object_type {
            ObjectType::Blob => ContentType::GitBlob,
            ObjectType::Tree => ContentType::GitTree,
            ObjectType::Commit => ContentType::GitCommit,
            ObjectType::Tag => ContentType::GitTag,
            ObjectType::Index => ContentType::GitIndex,
            ObjectType::Manifest => ContentType::GitManifest,
        }
    }
    
    fn create_git_metadata(&self, object: &GitObject) -> GitResult<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        
        metadata.insert("git_object_type".to_string(), object.object_type.to_string());
        metadata.insert("git_hash".to_string(), object.hash.hex());
        metadata.insert("git_size".to_string(), object.size.to_string());
        metadata.insert("git_creator".to_string(), object.creator.clone());
        metadata.insert("git_replication_factor".to_string(), object.replication_factor.to_string());
        
        if let Some(ttl) = object.ttl {
            metadata.insert("git_ttl_seconds".to_string(), ttl.as_secs().to_string());
        }
        
        Ok(metadata)
    }
    
    /// Clear caches
    pub fn clear_caches(&self) {
        if let Ok(mut cache) = self.object_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.ref_cache.write() {
            cache.clear();
        }
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> GitCacheStats {
        let object_stats = if let Ok(cache) = self.object_cache.read() {
            (cache.len(), cache.cap().get())
        } else {
            (0, 0)
        };
        
        let ref_stats = if let Ok(cache) = self.ref_cache.read() {
            (cache.len(), cache.cap().get())
        } else {
            (0, 0)
        };
        
        GitCacheStats {
            object_cache_size: object_stats.0,
            object_cache_capacity: object_stats.1,
            ref_cache_size: ref_stats.0,
            ref_cache_capacity: ref_stats.1,
        }
    }
}

/// Cache statistics for git storage
#[derive(Debug, Clone)]
pub struct GitCacheStats {
    pub object_cache_size: usize,
    pub object_cache_capacity: usize,
    pub ref_cache_size: usize,
    pub ref_cache_capacity: usize,
}

/// Mock DHT storage provider for testing
#[cfg(test)]
pub struct MockDhtStorage {
    pub storage: Arc<RwLock<HashMap<String, EnhancedDhtRecord>>>,
    pub local_peer_id: String,
}

#[cfg(test)]
impl MockDhtStorage {
    pub fn new(local_peer_id: String) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            local_peer_id,
        }
    }
}

#[cfg(test)]
impl DhtStorageProvider for MockDhtStorage {
    async fn store_secure_record(&self, record: EnhancedDhtRecord) -> GitResult<()> {
        let key = hex::encode(record.key.as_bytes());
        if let Ok(mut storage) = self.storage.write() {
            storage.insert(key, record);
        }
        Ok(())
    }
    
    async fn get_secure_record_with_k_consistency(
        &self,
        key: &Key,
        _requester: &str,
        _context: &AccessContext,
    ) -> GitResult<Option<EnhancedDhtRecord>> {
        let key_str = hex::encode(key.as_bytes());
        if let Ok(storage) = self.storage.read() {
            Ok(storage.get(&key_str).cloned())
        } else {
            Ok(None)
        }
    }
    
    fn local_id(&self) -> &str {
        &self.local_peer_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_objects::*;

    async fn create_test_storage() -> GitDhtStorage {
        let mock_dht = Arc::new(MockDhtStorage::new("test_peer".to_string()));
        GitDhtStorage::new(mock_dht, 100, "test_peer".to_string())
    }

    #[tokio::test]
    async fn test_store_and_retrieve_blob() {
        let storage = create_test_storage().await;
        
        let blob = BlobObject::from_text("Hello, World!");
        let blob_content = bincode::serialize(&blob).unwrap();
        let blob_object = GitObject::new(
            ObjectType::Blob,
            blob_content,
            DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            "test_peer".to_string(),
            None,
        );
        
        let hash = storage.store_object(blob_object.clone()).await.unwrap();
        let retrieved = storage.get_object(&hash).await.unwrap().unwrap();
        
        assert_eq!(retrieved.hash, hash);
        assert_eq!(retrieved.object_type, ObjectType::Blob);
        
        // Verify content
        let retrieved_blob: BlobObject = bincode::deserialize(&retrieved.content).unwrap();
        assert_eq!(retrieved_blob.as_string().unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_store_and_retrieve_reference() {
        let storage = create_test_storage().await;
        
        let target_hash = ContentHash::from_content(b"commit content");
        let reference = Reference::new_branch(
            "main".to_string(),
            target_hash.clone(),
            "test_namespace".to_string(),
            "test_peer".to_string(),
        );
        
        storage.store_reference(reference.clone()).await.unwrap();
        let retrieved = storage.get_reference("test_namespace", "main").await.unwrap().unwrap();
        
        assert_eq!(retrieved.name, "main");
        assert_eq!(retrieved.target, target_hash);
        assert_eq!(retrieved.namespace, "test_namespace");
    }

    #[tokio::test]
    async fn test_create_commit() {
        let storage = create_test_storage().await;
        
        // Create a simple tree
        let mut tree = TreeObject::new();
        let blob_hash = ContentHash::from_content(b"file content");
        tree.add_blob("file.txt".to_string(), blob_hash, 12);
        
        let author = CommitAuthor {
            peer_id: "test_peer".to_string(),
            name: "Test User".to_string(),
            email: None,
            timestamp: SystemTime::now(),
        };
        
        let commit_hash = storage.create_commit(
            tree,
            vec![],
            "Initial commit".to_string(),
            author,
            "test_app".to_string(),
            "test_namespace".to_string(),
            CommitType::DocumentCreated,
        ).await.unwrap();
        
        // Verify commit was stored
        let commit_object = storage.get_object(&commit_hash).await.unwrap().unwrap();
        assert_eq!(commit_object.object_type, ObjectType::Commit);
        
        let commit: CommitObject = bincode::deserialize(&commit_object.content).unwrap();
        assert_eq!(commit.message, "Initial commit");
        assert!(commit.is_root());
    }

    #[tokio::test]
    async fn test_update_branch() {
        let storage = create_test_storage().await;
        
        let initial_hash = ContentHash::from_content(b"initial commit");
        let new_hash = ContentHash::from_content(b"new commit");
        
        // Update branch (creates if doesn't exist)
        storage.update_branch("test_namespace", "main", initial_hash).await.unwrap();
        
        let reference = storage.get_reference("test_namespace", "main").await.unwrap().unwrap();
        assert_eq!(reference.target, initial_hash);
        
        // Update to new commit
        storage.update_branch("test_namespace", "main", new_hash).await.unwrap();
        
        let updated_reference = storage.get_reference("test_namespace", "main").await.unwrap().unwrap();
        assert_eq!(updated_reference.target, new_hash);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let storage = create_test_storage().await;
        
        let blob_object = GitObject::new(
            ObjectType::Blob,
            b"test content".to_vec(),
            DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            "test_peer".to_string(),
            None,
        );
        
        let hash = storage.store_object(blob_object).await.unwrap();
        
        // First retrieval should hit DHT and cache
        let _obj1 = storage.get_object(&hash).await.unwrap().unwrap();
        
        // Second retrieval should hit cache
        let _obj2 = storage.get_object(&hash).await.unwrap().unwrap();
        
        let stats = storage.cache_stats();
        assert!(stats.object_cache_size > 0);
    }
}