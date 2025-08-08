//! Enhanced Content Addressing System with BLAKE3 hashing
//!
//! Provides deterministic content addressing with efficient chunking,
//! deduplication, and integrity verification capabilities.

use anyhow::{Result, anyhow};
use blake3::Hasher;
use bytes::{Bytes, BytesMut};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::RwLock;

/// Content address uniquely identifying stored content
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentAddress {
    /// BLAKE3 hash of the content manifest
    pub root_hash: [u8; 32],
    /// Individual chunk hashes
    pub chunk_hashes: Vec<[u8; 32]>,
    /// Total content size in bytes
    pub total_size: u64,
    /// Number of chunks
    pub chunk_count: u32,
}

impl ContentAddress {
    /// Create a new content address
    pub fn new(root_hash: [u8; 32], chunk_hashes: Vec<[u8; 32]>, total_size: u64) -> Self {
        Self {
            root_hash,
            chunk_count: chunk_hashes.len() as u32,
            chunk_hashes,
            total_size,
        }
    }
    
    /// Create a content address from bytes (using the bytes as root hash)
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut root_hash = [0u8; 32];
        let len = bytes.len().min(32);
        root_hash[..len].copy_from_slice(&bytes[..len]);
        
        Self {
            root_hash,
            chunk_count: 1,
            chunk_hashes: vec![root_hash],
            total_size: bytes.len() as u64,
        }
    }
}

/// Metadata about stored chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_sizes: Vec<u32>,
    pub created_at: SystemTime,
    pub access_count: u32,
    pub dedup_count: u32,
}

/// Statistics about deduplication efficiency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupStatistics {
    pub total_chunks: u64,
    pub unique_chunks: u64,
    pub dedup_ratio: f64,
    pub space_saved: u64,
}

/// Configuration for content-defined chunking
#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    pub min_chunk_size: usize,
    pub target_chunk_size: usize,
    pub max_chunk_size: usize,
    pub window_size: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: 1024,        // 1KB
            target_chunk_size: 65536,    // 64KB
            max_chunk_size: 1048576,     // 1MB
            window_size: 48,
        }
    }
}

/// Content-defined chunker using rolling hash
pub struct ContentDefinedChunker {
    config: ChunkingConfig,
    buffer: BytesMut,
}

impl ContentDefinedChunker {
    pub fn new(config: ChunkingConfig) -> Self {
        let buffer_capacity = config.max_chunk_size;
        Self {
            config,
            buffer: BytesMut::with_capacity(buffer_capacity),
        }
    }
    
    /// Find next chunk boundary using rolling hash
    pub fn find_boundary(&self, data: &[u8]) -> Option<usize> {
        if data.len() < self.config.min_chunk_size {
            return None;
        }
        
        let mut hash = 0u32;
        let window = self.config.window_size;
        
        // Skip minimum chunk size
        let search_start = self.config.min_chunk_size;
        let search_end = data.len().min(self.config.max_chunk_size);
        
        for i in search_start..search_end {
            // Simple rolling hash (Buzhash)
            if i >= window {
                let old_byte = data[i - window];
                hash = hash.rotate_left(1) ^ u32::from(old_byte);
            }
            
            let new_byte = data[i];
            hash = hash.rotate_left(1) ^ u32::from(new_byte);
            
            // Check if we found a boundary
            let mask = (1 << 13) - 1; // Target 8KB average
            if (hash & mask) == 0 {
                return Some(i);
            }
        }
        
        // Force boundary at max chunk size
        if data.len() >= self.config.max_chunk_size {
            Some(self.config.max_chunk_size)
        } else {
            None
        }
    }
    
    /// Chunk data into variable-size chunks
    pub async fn chunk_data(&mut self, mut reader: impl AsyncRead + Unpin) -> Result<Vec<Bytes>> {
        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; self.config.max_chunk_size];
        
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            
            self.buffer.extend_from_slice(&buffer[..n]);
            
            while self.buffer.len() >= self.config.min_chunk_size {
                if let Some(boundary) = self.find_boundary(&self.buffer) {
                    let chunk = self.buffer.split_to(boundary);
                    chunks.push(chunk.freeze());
                } else {
                    break;
                }
            }
        }
        
        // Handle remaining data
        if !self.buffer.is_empty() {
            chunks.push(self.buffer.split().freeze());
        }
        
        Ok(chunks)
    }
}

/// Reference to a stored chunk
#[derive(Debug, Clone)]
struct ChunkRef {
    size: u32,
    created_at: SystemTime,
    access_count: u32,
    reference_count: u32,
}

/// Global deduplication index
pub struct DedupIndex {
    chunk_refs: Arc<RwLock<HashMap<[u8; 32], ChunkRef>>>,
    total_dedup_savings: Arc<RwLock<u64>>,
}

impl DedupIndex {
    pub fn new() -> Self {
        Self {
            chunk_refs: Arc::new(RwLock::new(HashMap::new())),
            total_dedup_savings: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Check if chunk exists and update reference count
    pub async fn check_and_update(&self, hash: &[u8; 32], size: u32) -> bool {
        let mut refs = self.chunk_refs.write().await;
        
        if let Some(chunk_ref) = refs.get_mut(hash) {
            chunk_ref.reference_count += 1;
            chunk_ref.access_count += 1;
            
            let mut savings = self.total_dedup_savings.write().await;
            *savings += size as u64;
            
            true
        } else {
            refs.insert(*hash, ChunkRef {
                size,
                created_at: SystemTime::now(),
                access_count: 1,
                reference_count: 1,
            });
            false
        }
    }
    
    /// Get deduplication statistics
    pub async fn get_stats(&self) -> DedupStatistics {
        let refs = self.chunk_refs.read().await;
        let savings = *self.total_dedup_savings.read().await;
        
        let total_chunks: u64 = refs.values()
            .map(|r| r.reference_count as u64)
            .sum();
        let unique_chunks = refs.len() as u64;
        
        DedupStatistics {
            total_chunks,
            unique_chunks,
            dedup_ratio: if total_chunks > 0 {
                1.0 - (unique_chunks as f64 / total_chunks as f64)
            } else {
                0.0
            },
            space_saved: savings,
        }
    }
}

/// Storage backend for chunks
pub struct ChunkStorage {
    chunks: Arc<RwLock<HashMap<[u8; 32], Bytes>>>,
}

impl ChunkStorage {
    pub fn new() -> Self {
        Self {
            chunks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Store a chunk
    pub async fn store(&self, hash: [u8; 32], data: Bytes) -> Result<()> {
        let mut chunks = self.chunks.write().await;
        chunks.insert(hash, data);
        Ok(())
    }
    
    /// Retrieve a chunk
    pub async fn retrieve(&self, hash: &[u8; 32]) -> Result<Bytes> {
        let chunks = self.chunks.read().await;
        chunks.get(hash)
            .cloned()
            .ok_or_else(|| anyhow!("Chunk not found"))
    }
    
    /// Verify chunk integrity
    pub async fn verify(&self, hash: &[u8; 32]) -> Result<bool> {
        let chunks = self.chunks.read().await;
        if let Some(data) = chunks.get(hash) {
            let computed_hash = blake3::hash(data);
            Ok(computed_hash.as_bytes() == hash)
        } else {
            Ok(false)
        }
    }
}

/// Main content addressing system
pub struct ContentAddressingSystem {
    chunker: ContentDefinedChunker,
    dedup_index: DedupIndex,
    chunk_store: ChunkStorage,
    metadata: Arc<RwLock<HashMap<[u8; 32], ChunkMetadata>>>,
}

impl ContentAddressingSystem {
    /// Create a new content addressing system
    pub fn new() -> Self {
        Self {
            chunker: ContentDefinedChunker::new(ChunkingConfig::default()),
            dedup_index: DedupIndex::new(),
            chunk_store: ChunkStorage::new(),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Store content and return its address
    pub async fn store_content(
        &mut self,
        content: impl AsyncRead + Unpin,
    ) -> Result<ContentAddress> {
        // Chunk the content
        let chunks = self.chunker.chunk_data(content).await?;
        
        let mut chunk_hashes = Vec::new();
        let mut total_size = 0u64;
        let mut chunk_sizes = Vec::new();
        
        // Process each chunk
        for chunk in chunks {
            let hash = blake3::hash(&chunk);
            let hash_bytes = *hash.as_bytes();
            
            chunk_hashes.push(hash_bytes);
            chunk_sizes.push(chunk.len() as u32);
            total_size += chunk.len() as u64;
            
            // Check deduplication
            let is_duplicate = self.dedup_index
                .check_and_update(&hash_bytes, chunk.len() as u32)
                .await;
            
            // Store if new
            if !is_duplicate {
                self.chunk_store.store(hash_bytes, chunk).await?;
            }
        }
        
        // Generate root hash from manifest
        let mut hasher = Hasher::new();
        for hash in &chunk_hashes {
            hasher.update(hash);
        }
        hasher.update(&total_size.to_le_bytes());
        let root_hash = *hasher.finalize().as_bytes();
        
        // Store metadata
        let metadata = ChunkMetadata {
            chunk_sizes,
            created_at: SystemTime::now(),
            access_count: 0,
            dedup_count: 0,
        };
        
        self.metadata.write().await.insert(root_hash, metadata);
        
        Ok(ContentAddress::new(root_hash, chunk_hashes, total_size))
    }
    
    /// Retrieve content by address
    pub async fn retrieve_content(
        &self,
        address: &ContentAddress,
    ) -> Result<Vec<u8>> {
        let mut content = Vec::with_capacity(address.total_size as usize);
        
        for chunk_hash in &address.chunk_hashes {
            let chunk = self.chunk_store.retrieve(chunk_hash).await?;
            content.extend_from_slice(&chunk);
        }
        
        // Update access count
        if let Some(metadata) = self.metadata.write().await.get_mut(&address.root_hash) {
            metadata.access_count += 1;
        }
        
        Ok(content)
    }
    
    /// Verify content integrity without retrieval
    pub async fn verify_integrity(&self, address: &ContentAddress) -> Result<bool> {
        // Verify all chunks exist and match their hashes
        for chunk_hash in &address.chunk_hashes {
            if !self.chunk_store.verify(chunk_hash).await? {
                return Ok(false);
            }
        }
        
        // Verify manifest hash
        let mut hasher = Hasher::new();
        for hash in &address.chunk_hashes {
            hasher.update(hash);
        }
        hasher.update(&address.total_size.to_le_bytes());
        let computed_root = hasher.finalize();
        
        Ok(computed_root.as_bytes() == &address.root_hash)
    }
    
    /// Get chunk metadata
    pub async fn get_chunk_info(&self, address: &ContentAddress) -> Result<ChunkMetadata> {
        let metadata = self.metadata.read().await;
        metadata.get(&address.root_hash)
            .cloned()
            .ok_or_else(|| anyhow!("Metadata not found"))
    }
    
    /// Get deduplication statistics
    pub async fn get_dedup_stats(&self) -> DedupStatistics {
        self.dedup_index.get_stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::Cursor;
    
    #[tokio::test]
    async fn test_deterministic_addressing() {
        let mut system1 = ContentAddressingSystem::new();
        let mut system2 = ContentAddressingSystem::new();
        let content = b"test content for deterministic addressing";
        
        let addr1 = system1.store_content(Cursor::new(content)).await.unwrap();
        let addr2 = system2.store_content(Cursor::new(content)).await.unwrap();
        
        assert_eq!(addr1, addr2, "Same content should produce same address");
    }
    
    #[tokio::test]
    async fn test_content_integrity_verification() {
        let mut system = ContentAddressingSystem::new();
        let content = b"test content for integrity verification";
        
        let addr = system.store_content(Cursor::new(content)).await.unwrap();
        let is_valid = system.verify_integrity(&addr).await.unwrap();
        
        assert!(is_valid, "Content integrity should be valid");
        
        let retrieved = system.retrieve_content(&addr).await.unwrap();
        assert_eq!(retrieved, content, "Retrieved content should match original");
    }
    
    #[tokio::test]
    async fn test_chunking_boundaries() {
        let config = ChunkingConfig::default();
        let chunker = ContentDefinedChunker::new(config.clone());
        
        // Test minimum chunk size
        let small_data = vec![0u8; config.min_chunk_size - 1];
        assert_eq!(chunker.find_boundary(&small_data), None);
        
        // Test maximum chunk size
        let large_data = vec![1u8; config.max_chunk_size + 100];
        let boundary = chunker.find_boundary(&large_data);
        assert_eq!(boundary, Some(config.max_chunk_size));
    }
    
    #[tokio::test]
    async fn test_deduplication_efficiency() {
        let mut system = ContentAddressingSystem::new();
        
        // Store same content twice
        let content = b"duplicate content for dedup testing";
        let addr1 = system.store_content(Cursor::new(content)).await.unwrap();
        let addr2 = system.store_content(Cursor::new(content)).await.unwrap();
        
        assert_eq!(addr1, addr2, "Duplicate content should have same address");
        
        let stats = system.get_dedup_stats().await;
        assert!(stats.dedup_ratio > 0.0, "Should have deduplication");
        assert_eq!(stats.unique_chunks, 1, "Should only store one unique chunk");
    }
    
    #[tokio::test]
    async fn test_empty_content() {
        let mut system = ContentAddressingSystem::new();
        let content = b"";
        
        let addr = system.store_content(Cursor::new(content)).await.unwrap();
        assert_eq!(addr.total_size, 0);
        assert_eq!(addr.chunk_count, 0);
        
        let retrieved = system.retrieve_content(&addr).await.unwrap();
        assert_eq!(retrieved.len(), 0);
    }
    
    #[tokio::test]
    async fn test_large_content_streaming() {
        let mut system = ContentAddressingSystem::new();
        let large_content = vec![42u8; 10_000_000]; // 10MB
        
        let addr = system.store_content(Cursor::new(&large_content)).await.unwrap();
        assert_eq!(addr.total_size, large_content.len() as u64);
        assert!(addr.chunk_count > 1, "Large content should be chunked");
        
        let retrieved = system.retrieve_content(&addr).await.unwrap();
        assert_eq!(retrieved.len(), large_content.len());
        assert_eq!(retrieved, large_content);
    }
}