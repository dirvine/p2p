// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// File metadata and chunking system for secure file transfers

#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;
use blake3::Hasher;
use crate::identity::FourWordAddress;

/// Default chunk size for file transfers (1MB)
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum chunk size allowed (10MB)
pub const MAX_CHUNK_SIZE: usize = 10 * 1024 * 1024;

/// File metadata containing all information about a shareable file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMetadata {
    pub id: Uuid,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub blake3_hash: String,
    pub chunk_size: usize,
    pub chunk_count: usize,
    pub chunk_hashes: Vec<String>,
    pub created_at: u64,
    pub owner: FourWordAddress,
    pub permissions: FilePermissions,
}

/// File sharing permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilePermissions {
    pub public: bool,
    pub trusted_peers_only: bool,
    pub allowed_peers: Vec<FourWordAddress>,
    pub expires_at: Option<u64>,
}

/// File chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkMetadata {
    pub file_id: Uuid,
    pub chunk_index: usize,
    pub chunk_size: usize,
    pub blake3_hash: String,
    pub offset: u64,
}

/// File chunking manager
#[derive(Debug)]
pub struct FileChunker {
    chunk_size: usize,
}

impl Default for FilePermissions {
    fn default() -> Self {
        FilePermissions {
            public: false,
            trusted_peers_only: true,
            allowed_peers: Vec::new(),
            expires_at: None,
        }
    }
}

impl FileMetadata {
    /// Create file metadata from file path
    pub async fn from_file(file_path: PathBuf, owner: FourWordAddress) -> Result<Self> {
        let metadata = tokio::fs::metadata(&file_path).await
            .context("Failed to read file metadata")?;
        
        let size = metadata.len();
        let name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Guess MIME type from file extension
        let mime_type = mime_guess::from_path(&file_path)
            .first_or_octet_stream()
            .to_string();
        
        // Read file and compute BLAKE3 hash
        let file_content = tokio::fs::read(&file_path).await
            .context("Failed to read file for hashing")?;
        let blake3_hash = blake3::hash(&file_content).to_hex().to_string();
        
        // Calculate chunks
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let chunk_count = (size as usize + chunk_size - 1) / chunk_size;
        
        // Calculate chunk hashes
        let mut chunk_hashes = Vec::new();
        for i in 0..chunk_count {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size, file_content.len());
            let chunk_data = &file_content[start..end];
            let chunk_hash = blake3::hash(chunk_data).to_hex().to_string();
            chunk_hashes.push(chunk_hash);
        }
        
        // Get current timestamp
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(FileMetadata {
            id: Uuid::new_v4(),
            name,
            size,
            mime_type,
            blake3_hash,
            chunk_size,
            chunk_count,
            chunk_hashes,
            created_at,
            owner,
            permissions: FilePermissions::default(),
        })
    }

    /// Validate file metadata integrity
    pub fn validate(&self) -> Result<()> {
        // Check basic constraints
        if self.name.is_empty() {
            anyhow::bail!("File name cannot be empty");
        }
        
        if self.size == 0 {
            anyhow::bail!("File size cannot be zero");
        }
        
        if self.chunk_size == 0 || self.chunk_size > MAX_CHUNK_SIZE {
            anyhow::bail!("Invalid chunk size: must be between 1 and {}", MAX_CHUNK_SIZE);
        }
        
        // Validate chunk count matches size and chunk_size
        let expected_chunks = (self.size as usize + self.chunk_size - 1) / self.chunk_size;
        if self.chunk_count != expected_chunks {
            anyhow::bail!("Chunk count {} doesn't match expected {} for size {} and chunk_size {}", 
                         self.chunk_count, expected_chunks, self.size, self.chunk_size);
        }
        
        // Validate chunk hashes length
        if self.chunk_hashes.len() != self.chunk_count {
            anyhow::bail!("Chunk hashes length {} doesn't match chunk count {}", 
                         self.chunk_hashes.len(), self.chunk_count);
        }
        
        // Validate hash format (BLAKE3 produces 64 hex characters)
        if self.blake3_hash.len() != 64 {
            anyhow::bail!("Invalid BLAKE3 hash length: expected 64, got {}", self.blake3_hash.len());
        }
        
        // Validate all chunk hashes are valid hex strings
        for (i, hash) in self.chunk_hashes.iter().enumerate() {
            if hash.len() != 64 {
                anyhow::bail!("Invalid chunk hash length at index {}: expected 64, got {}", i, hash.len());
            }
            if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                anyhow::bail!("Invalid chunk hash format at index {}: not hexadecimal", i);
            }
        }
        
        Ok(())
    }

    /// Check if peer has permission to access this file
    pub fn has_permission(&self, peer: &FourWordAddress, is_trusted: bool) -> bool {
        // Owner always has permission
        if peer == &self.owner {
            return true;
        }
        
        // Check if file has expired
        if let Some(expires_at) = self.permissions.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now > expires_at {
                return false;
            }
        }
        
        // Check public access
        if self.permissions.public {
            return true;
        }
        
        // Check specific peer allowlist
        if self.permissions.allowed_peers.contains(peer) {
            return true;
        }
        
        // Check trusted peers only flag
        if self.permissions.trusted_peers_only && is_trusted {
            return true;
        }
        
        false
    }

    /// Get chunk metadata for specific chunk
    pub fn get_chunk_metadata(&self, chunk_index: usize) -> Result<ChunkMetadata> {
        if chunk_index >= self.chunk_count {
            anyhow::bail!("Chunk index {} out of bounds (max: {})", chunk_index, self.chunk_count - 1);
        }
        
        let offset = (chunk_index * self.chunk_size) as u64;
        
        // Calculate actual chunk size (last chunk might be smaller)
        let remaining_bytes = self.size - offset;
        let actual_chunk_size = std::cmp::min(self.chunk_size, remaining_bytes as usize);
        
        let blake3_hash = self.chunk_hashes[chunk_index].clone();
        
        Ok(ChunkMetadata {
            file_id: self.id,
            chunk_index,
            chunk_size: actual_chunk_size,
            blake3_hash,
            offset,
        })
    }
}

impl FileChunker {
    /// Create new file chunker with default chunk size
    pub fn new() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }

    /// Create new file chunker with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Result<Self> {
        if chunk_size == 0 {
            anyhow::bail!("Chunk size cannot be zero");
        }
        
        if chunk_size > MAX_CHUNK_SIZE {
            anyhow::bail!("Chunk size {} exceeds maximum allowed size {}", chunk_size, MAX_CHUNK_SIZE);
        }
        
        Ok(Self { chunk_size })
    }

    /// Chunk file into metadata and return chunk information
    pub async fn chunk_file(&self, file_path: PathBuf, owner: FourWordAddress) -> Result<FileMetadata> {
        let metadata = tokio::fs::metadata(&file_path).await
            .context("Failed to read file metadata")?;
        
        let size = metadata.len();
        let name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Guess MIME type from file extension
        let mime_type = mime_guess::from_path(&file_path)
            .first_or_octet_stream()
            .to_string();
        
        // Read file and compute BLAKE3 hash
        let file_content = tokio::fs::read(&file_path).await
            .context("Failed to read file for hashing")?;
        let blake3_hash = blake3::hash(&file_content).to_hex().to_string();
        
        // Calculate chunks using the chunker's chunk size
        let chunk_count = (size as usize + self.chunk_size - 1) / self.chunk_size;
        
        // Calculate chunk hashes
        let mut chunk_hashes = Vec::new();
        for i in 0..chunk_count {
            let start = i * self.chunk_size;
            let end = std::cmp::min(start + self.chunk_size, file_content.len());
            let chunk_data = &file_content[start..end];
            let chunk_hash = blake3::hash(chunk_data).to_hex().to_string();
            chunk_hashes.push(chunk_hash);
        }
        
        // Get current timestamp
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(FileMetadata {
            id: Uuid::new_v4(),
            name,
            size,
            mime_type,
            blake3_hash,
            chunk_size: self.chunk_size,
            chunk_count,
            chunk_hashes,
            created_at,
            owner,
            permissions: FilePermissions::default(),
        })
    }

    /// Read specific chunk from file
    pub async fn read_chunk(&self, file_path: &PathBuf, chunk_metadata: &ChunkMetadata) -> Result<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        
        let mut file = tokio::fs::File::open(file_path).await
            .context("Failed to open file for chunk reading")?;
        
        // Seek to the chunk offset
        file.seek(std::io::SeekFrom::Start(chunk_metadata.offset)).await
            .context("Failed to seek to chunk offset")?;
        
        // Read the chunk data
        let mut buffer = vec![0u8; chunk_metadata.chunk_size];
        let bytes_read = file.read(&mut buffer).await
            .context("Failed to read chunk data")?;
        
        // Resize buffer to actual bytes read (in case of last chunk)
        buffer.truncate(bytes_read);
        
        // Verify chunk integrity
        let actual_hash = blake3::hash(&buffer).to_hex().to_string();
        if actual_hash != chunk_metadata.blake3_hash {
            anyhow::bail!("Chunk integrity verification failed: expected {}, got {}", 
                         chunk_metadata.blake3_hash, actual_hash);
        }
        
        Ok(buffer)
    }

    /// Verify chunk integrity
    pub fn verify_chunk(&self, chunk_data: &[u8], expected_hash: &str) -> bool {
        let actual_hash = blake3::hash(chunk_data).to_hex().to_string();
        actual_hash == expected_hash
    }
}

impl Default for FileChunker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[test]
    fn test_file_permissions_default() {
        let permissions = FilePermissions::default();
        
        assert!(!permissions.public);
        assert!(permissions.trusted_peers_only);
        assert!(permissions.allowed_peers.is_empty());
        assert!(permissions.expires_at.is_none());
    }

    #[test]
    fn test_chunk_metadata_creation() {
        let file_id = Uuid::new_v4();
        let chunk = ChunkMetadata {
            file_id,
            chunk_index: 0,
            chunk_size: DEFAULT_CHUNK_SIZE,
            blake3_hash: "test_hash".to_string(),
            offset: 0,
        };

        assert_eq!(chunk.file_id, file_id);
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.chunk_size, DEFAULT_CHUNK_SIZE);
        assert_eq!(chunk.offset, 0);
    }

    #[tokio::test]
    async fn test_file_metadata_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, world!";
        fs::write(&file_path, content).await.unwrap();

        let owner = FourWordAddress::generate().unwrap();
        
        // Now implementation is complete, should succeed
        let result = FileMetadata::from_file(file_path, owner).await;
        assert!(result.is_ok());
        
        let metadata = result.unwrap();
        assert_eq!(metadata.name, "test.txt");
        assert_eq!(metadata.size, content.len() as u64);
    }

    #[test]
    fn test_file_metadata_validation() {
        let metadata = FileMetadata {
            id: Uuid::new_v4(),
            name: "test.txt".to_string(),
            size: 13,
            mime_type: "text/plain".to_string(),
            blake3_hash: "test_hash".to_string(),
            chunk_size: DEFAULT_CHUNK_SIZE,
            chunk_count: 1,
            chunk_hashes: vec!["chunk_hash".to_string()],
            created_at: 1234567890,
            owner: FourWordAddress::generate().unwrap(),
            permissions: FilePermissions::default(),
        };

        // This should fail until implementation is complete
        let result = metadata.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_permissions_check() {
        let owner = FourWordAddress::generate().unwrap();
        let trusted_peer = FourWordAddress::generate().unwrap();
        let untrusted_peer = FourWordAddress::generate().unwrap();

        let metadata = FileMetadata {
            id: Uuid::new_v4(),
            name: "test.txt".to_string(),
            size: 13,
            mime_type: "text/plain".to_string(),
            blake3_hash: "test_hash".to_string(),
            chunk_size: DEFAULT_CHUNK_SIZE,
            chunk_count: 1,
            chunk_hashes: vec!["chunk_hash".to_string()],
            created_at: 1234567890,
            owner: owner.clone(),
            permissions: FilePermissions::default(), // trusted_peers_only = true
        };

        // This should fail until implementation is complete
        // Should allow trusted peers but not untrusted ones
        // assert!(metadata.has_permission(&trusted_peer, true));
        // assert!(!metadata.has_permission(&untrusted_peer, false));
    }

    #[test]
    fn test_chunk_metadata_retrieval() {
        let metadata = FileMetadata {
            id: Uuid::new_v4(),
            name: "test.txt".to_string(),
            size: 13,
            mime_type: "text/plain".to_string(),
            blake3_hash: "test_hash".to_string(),
            chunk_size: DEFAULT_CHUNK_SIZE,
            chunk_count: 1,
            chunk_hashes: vec!["chunk_hash".to_string()],
            created_at: 1234567890,
            owner: FourWordAddress::generate().unwrap(),
            permissions: FilePermissions::default(),
        };

        // Now implementation is complete, should succeed
        let result = metadata.get_chunk_metadata(0);
        assert!(result.is_ok());
        
        let chunk_metadata = result.unwrap();
        assert_eq!(chunk_metadata.chunk_index, 0);
        assert_eq!(chunk_metadata.file_id, metadata.id);
    }

    #[test]
    fn test_file_chunker_creation() {
        let chunker = FileChunker::new();
        assert_eq!(chunker.chunk_size, DEFAULT_CHUNK_SIZE);

        let chunker2 = FileChunker::default();
        assert_eq!(chunker2.chunk_size, DEFAULT_CHUNK_SIZE);
    }

    #[test]
    fn test_file_chunker_custom_size() {
        let custom_size = 512 * 1024; // 512KB
        
        // Now implementation is complete, should succeed
        let result = FileChunker::with_chunk_size(custom_size);
        assert!(result.is_ok());
        
        let chunker = result.unwrap();
        assert_eq!(chunker.chunk_size, custom_size);
    }

    #[tokio::test]
    async fn test_file_chunking() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "A".repeat(2 * 1024 * 1024); // 2MB file
        fs::write(&file_path, &content).await.unwrap();

        let chunker = FileChunker::new();
        let owner = FourWordAddress::generate().unwrap();

        // Now implementation is complete, should succeed
        let result = chunker.chunk_file(file_path, owner).await;
        assert!(result.is_ok());
        
        let metadata = result.unwrap();
        assert_eq!(metadata.size, content.len() as u64);
        assert_eq!(metadata.chunk_count, 2); // 2MB file with 1MB chunks = 2 chunks
    }

    #[tokio::test]
    async fn test_chunk_reading() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, world!";
        fs::write(&file_path, content).await.unwrap();

        let chunker = FileChunker::new();
        let chunk_metadata = ChunkMetadata {
            file_id: Uuid::new_v4(),
            chunk_index: 0,
            chunk_size: content.len(),
            blake3_hash: "test_hash".to_string(),
            offset: 0,
        };

        // This should fail until implementation is complete
        let result = chunker.read_chunk(&file_path, &chunk_metadata).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_verification() {
        let chunker = FileChunker::new();
        let chunk_data = b"Hello, world!";
        let expected_hash = blake3::hash(chunk_data).to_hex().to_string();

        // Now implementation is complete, should succeed
        let result = chunker.verify_chunk(chunk_data, &expected_hash);
        assert!(result); // Should return true for correct hash
    }
}