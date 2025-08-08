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

//! Content-addressed storage system with adaptive replication
//!
//! This module implements the storage and retrieval system for the adaptive P2P network,
//! featuring:
//! - Content-addressed storage using SHA-256
//! - Parallel retrieval strategies
//! - Adaptive replication based on network churn
//! - Efficient chunk management
//! - RocksDB persistence layer

use super::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::sync::RwLock;

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Path to RocksDB database
    pub db_path: String,

    /// Maximum chunk size in bytes
    pub chunk_size: usize,

    /// Replication configuration
    pub replication_config: ReplicationConfig,

    /// Cache size in bytes
    pub cache_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/storage".to_string(),
            chunk_size: 1024 * 1024, // 1MB chunks
            replication_config: ReplicationConfig::default(),
            cache_size: 100 * 1024 * 1024, // 100MB cache
        }
    }
}

/// Replication configuration
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// Minimum replication factor
    pub min_replicas: u32,

    /// Maximum replication factor
    pub max_replicas: u32,

    /// Base replication factor
    pub base_replicas: u32,

    /// Churn threshold for increasing replication
    pub churn_threshold: f64,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            min_replicas: 5,
            max_replicas: 20,
            base_replicas: 8,
            churn_threshold: 0.3, // 30% churn rate
        }
    }
}

/// Content store for local storage
pub struct ContentStore {
    /// Storage backend (using in-memory for now, would be RocksDB in production)
    db: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,

    /// In-memory cache
    cache: Arc<RwLock<HashMap<ContentHash, CachedContent>>>,

    /// Storage configuration
    config: StorageConfig,

    /// Storage statistics
    stats: Arc<RwLock<StorageStats>>,
}

/// Cached content with metadata
#[derive(Debug, Clone)]
pub struct CachedContent {
    /// Content data
    pub data: Vec<u8>,

    /// Last access time
    pub last_access: Instant,

    /// Access count
    pub access_count: u64,

    /// Content metadata
    pub metadata: ContentMetadata,
}

/// Content metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetadata {
    /// Content size
    pub size: usize,

    /// Content type
    pub content_type: ContentType,

    /// Creation timestamp (Unix timestamp in seconds)
    pub created_at: u64,

    /// Number of chunks (if chunked)
    pub chunk_count: Option<u32>,

    /// Replication factor
    pub replication_factor: u32,
}

impl Default for ContentMetadata {
    fn default() -> Self {
        Self {
            size: 0,
            content_type: ContentType::DataRetrieval,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            chunk_count: None,
            replication_factor: 8,
        }
    }
}

/// Storage statistics
#[derive(Debug, Default, Clone)]
pub struct StorageStats {
    /// Total content stored
    pub total_content: u64,

    /// Total bytes stored
    pub total_bytes: u64,

    /// Cache hits
    pub cache_hits: u64,

    /// Cache misses
    pub cache_misses: u64,

    /// Storage operations
    pub store_operations: u64,

    /// Retrieval operations
    pub retrieve_operations: u64,

    /// Total number of items (alias for total_content)
    pub total_items: u64,

    /// Storage capacity in bytes
    pub capacity_bytes: u64,
}

impl StorageStats {
    /// Calculate storage utilization
    pub fn utilization(&self) -> f64 {
        if self.capacity_bytes == 0 {
            0.0
        } else {
            self.total_bytes as f64 / self.capacity_bytes as f64
        }
    }
}

impl ContentStore {
    /// Create a new content store
    pub async fn new(config: StorageConfig) -> Result<Self> {
        // For now, use in-memory storage (would be RocksDB in production)
        let db = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(StorageStats::default())),
        })
    }

    /// Store content
    pub async fn store(&self, content: Vec<u8>, metadata: ContentMetadata) -> Result<ContentHash> {
        // Calculate content hash
        let hash = Self::calculate_hash(&content);

        // Store in database
        let mut db = self.db.write().await;
        db.insert(hash.0.to_vec(), content.clone());

        // Store metadata
        let metadata_key = Self::metadata_key(&hash);
        let metadata_bytes = bincode::serialize(&metadata)?;
        db.insert(metadata_key, metadata_bytes);

        // Update cache
        let content_size = metadata.size;
        let mut cache = self.cache.write().await;
        cache.insert(
            hash.clone(),
            CachedContent {
                data: content,
                last_access: Instant::now(),
                access_count: 0,
                metadata,
            },
        );

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_content += 1;
        stats.total_bytes += content_size as u64;
        stats.store_operations += 1;

        Ok(hash)
    }

    /// Retrieve content
    pub async fn retrieve(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>> {
        // Check cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(cached) = cache.get_mut(hash) {
                cached.last_access = Instant::now();
                cached.access_count += 1;

                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;
                stats.retrieve_operations += 1;

                return Ok(Some(cached.data.clone()));
            }
        }

        // Cache miss - fetch from RocksDB
        let mut stats = self.stats.write().await;
        stats.cache_misses += 1;
        stats.retrieve_operations += 1;
        drop(stats);

        let db = self.db.read().await;
        match db.get(&hash.0.to_vec()) {
            Some(data) => {
                // Get metadata
                let metadata_key = Self::metadata_key(hash);
                let metadata = if let Some(metadata_bytes) = db.get(&metadata_key) {
                    bincode::deserialize(metadata_bytes)?
                } else {
                    // Create default metadata if missing
                    ContentMetadata {
                        size: data.len(),
                        content_type: ContentType::DataRetrieval,
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                        chunk_count: None,
                        replication_factor: 8,
                    }
                };

                // Update cache
                let mut cache = self.cache.write().await;
                cache.insert(
                    hash.clone(),
                    CachedContent {
                        data: data.clone(),
                        last_access: Instant::now(),
                        access_count: 1,
                        metadata,
                    },
                );

                Ok(Some(data.clone()))
            }
            None => Ok(None),
        }
    }

    /// Check if content exists
    pub async fn exists(&self, hash: &ContentHash) -> Result<bool> {
        // Check cache
        if self.cache.read().await.contains_key(hash) {
            return Ok(true);
        }

        // Check database
        let db = self.db.read().await;
        Ok(db.contains_key(&hash.0.to_vec()))
    }

    /// Delete content
    pub async fn delete(&self, hash: &ContentHash) -> Result<()> {
        // Remove from cache
        self.cache.write().await.remove(hash);

        // Remove from database
        let mut db = self.db.write().await;
        db.remove(&hash.0.to_vec());

        // Remove metadata
        let metadata_key = Self::metadata_key(hash);
        db.remove(&metadata_key);

        Ok(())
    }

    /// Calculate SHA-256 hash of content
    pub fn calculate_hash(content: &[u8]) -> ContentHash {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&result);
        ContentHash(hash_bytes)
    }

    /// Get metadata key for a content hash
    fn metadata_key(hash: &ContentHash) -> Vec<u8> {
        let mut key = vec![0u8]; // Prefix for metadata
        key.extend_from_slice(&hash.0);
        key
    }

    /// Get storage configuration
    pub fn get_config(&self) -> &StorageConfig {
        &self.config
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageStats {
        let mut stats = self.stats.read().await.clone();
        // Ensure total_items is synchronized with total_content
        stats.total_items = stats.total_content;
        // Set capacity (in real implementation would get from disk)
        stats.capacity_bytes = 10 * 1024 * 1024 * 1024; // 10GB default
        stats
    }
}

/// Stored content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredContent {
    /// Content hash
    pub hash: ContentHash,

    /// Content data
    pub data: Vec<u8>,

    /// Content metadata
    pub metadata: ContentMetadata,
}

/// Chunk metadata for large content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Parent content hash
    pub parent_hash: ContentHash,

    /// Chunk index
    pub chunk_index: u32,

    /// Total chunks
    pub total_chunks: u32,

    /// Chunk size
    pub chunk_size: usize,

    /// Chunk hash
    pub chunk_hash: ContentHash,
}

/// Chunk of large content
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Chunk metadata
    pub metadata: ChunkMetadata,

    /// Chunk data
    pub data: Vec<u8>,
}

/// Chunk manager for handling large content
pub struct ChunkManager {
    /// Maximum chunk size
    chunk_size: usize,
}

impl ChunkManager {
    /// Create a new chunk manager
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Split content into chunks
    pub fn create_chunks(&self, content: &[u8], parent_hash: ContentHash) -> Vec<Chunk> {
        let total_chunks = content.len().div_ceil(self.chunk_size);

        content
            .chunks(self.chunk_size)
            .enumerate()
            .map(|(i, chunk_data)| {
                let chunk_hash = ContentStore::calculate_hash(chunk_data);
                Chunk {
                    metadata: ChunkMetadata {
                        parent_hash: parent_hash.clone(),
                        chunk_index: i as u32,
                        total_chunks: total_chunks as u32,
                        chunk_size: chunk_data.len(),
                        chunk_hash,
                    },
                    data: chunk_data.to_vec(),
                }
            })
            .collect()
    }

    /// Reassemble chunks into content
    pub fn reassemble_chunks(chunks: Vec<Chunk>) -> Result<Vec<u8>> {
        // Sort chunks by index
        let mut sorted_chunks = chunks;
        sorted_chunks.sort_by_key(|c| c.metadata.chunk_index);

        // Verify we have all chunks
        if sorted_chunks.is_empty() {
            return Err(anyhow::anyhow!("No chunks provided"));
        }

        let total_chunks = sorted_chunks[0].metadata.total_chunks;
        if sorted_chunks.len() != total_chunks as usize {
            return Err(anyhow::anyhow!(
                "Missing chunks: have {}, need {}",
                sorted_chunks.len(),
                total_chunks
            ));
        }

        // Reassemble
        let mut content = Vec::new();
        for chunk in sorted_chunks {
            content.extend(chunk.data);
        }

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_content_hash_generation() {
        let content1 = b"Hello, World!";
        let content2 = b"Hello, World!";
        let content3 = b"Different content";

        let hash1 = ContentStore::calculate_hash(content1);
        let hash2 = ContentStore::calculate_hash(content2);
        let hash3 = ContentStore::calculate_hash(content3);

        // Same content should produce same hash
        assert_eq!(hash1, hash2);

        // Different content should produce different hash
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_content_storage() {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            db_path: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let store = ContentStore::new(config).await.unwrap();
        let content = b"Test content".to_vec();
        let metadata = ContentMetadata {
            size: content.len(),
            content_type: ContentType::DataRetrieval,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            chunk_count: None,
            replication_factor: 8,
        };

        // Store content
        let hash = store.store(content.clone(), metadata).await.unwrap();

        // Retrieve content
        let retrieved = store.retrieve(&hash).await.unwrap();
        assert_eq!(retrieved, Some(content));

        // Check existence
        assert!(store.exists(&hash).await.unwrap());

        // Check stats
        let stats = store.get_stats().await;
        assert_eq!(stats.total_content, 1);
        assert_eq!(stats.store_operations, 1);
        assert_eq!(stats.retrieve_operations, 1);
        assert_eq!(stats.cache_hits, 1);
    }

    #[tokio::test]
    async fn test_chunk_manager() {
        let chunk_size = 10;
        let manager = ChunkManager::new(chunk_size);
        let content = b"This is a test content that will be chunked".to_vec();
        let parent_hash = ContentStore::calculate_hash(&content);

        // Create chunks
        let chunks = manager.create_chunks(&content, parent_hash);
        assert_eq!(chunks.len(), 5); // 44 bytes / 10 = 5 chunks

        // Verify chunk metadata
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i as u32);
            assert_eq!(chunk.metadata.total_chunks, 5);
            assert_eq!(chunk.metadata.parent_hash, parent_hash);
        }

        // Reassemble chunks
        let reassembled = ChunkManager::reassemble_chunks(chunks).unwrap();
        assert_eq!(reassembled, content);
    }

    #[tokio::test]
    async fn test_cache_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            db_path: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let store = ContentStore::new(config).await.unwrap();
        let content = b"Cached content".to_vec();
        let metadata = ContentMetadata {
            size: content.len(),
            content_type: ContentType::DataRetrieval,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            chunk_count: None,
            replication_factor: 8,
        };

        // Store content
        let hash = store.store(content.clone(), metadata).await.unwrap();

        // First retrieval (from cache)
        let _ = store.retrieve(&hash).await.unwrap();

        // Second retrieval (should hit cache)
        let _ = store.retrieve(&hash).await.unwrap();

        let stats = store.get_stats().await;
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.cache_misses, 0);
    }
}
