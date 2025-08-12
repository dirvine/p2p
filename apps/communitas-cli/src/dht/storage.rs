// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Storage backend for DHT data

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Storage statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageStats {
    pub total_files: usize,
    pub total_bytes: usize,
    pub capacity_mb: usize,
    pub used_mb: f64,
}

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a value
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<()>;
    
    /// Retrieve a value
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    
    /// Delete a value
    async fn delete(&mut self, key: &str) -> Result<bool>;
    
    /// List keys with optional prefix
    async fn list_keys(&self, prefix: Option<&str>, limit: usize) -> Result<Vec<String>>;
    
    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats>;
    
    /// Clear all data
    async fn clear(&mut self) -> Result<()>;
}

/// In-memory storage backend
pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    capacity_mb: usize,
}

impl MemoryStorage {
    pub fn new(capacity_mb: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            capacity_mb,
        }
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<()> {
        let mut data = self.data.write().await;
        
        // Check capacity
        let current_size: usize = data.values().map(|v| v.len()).sum();
        let new_size = current_size + value.len();
        
        if new_size > self.capacity_mb * 1024 * 1024 {
            return Err(anyhow::anyhow!("Storage capacity exceeded"));
        }
        
        data.insert(key.to_string(), value.to_vec());
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }
    
    async fn delete(&mut self, key: &str) -> Result<bool> {
        let mut data = self.data.write().await;
        Ok(data.remove(key).is_some())
    }
    
    async fn list_keys(&self, prefix: Option<&str>, limit: usize) -> Result<Vec<String>> {
        let data = self.data.read().await;
        
        let keys: Vec<String> = if let Some(prefix) = prefix {
            data.keys()
                .filter(|k| k.starts_with(prefix))
                .take(limit)
                .cloned()
                .collect()
        } else {
            data.keys()
                .take(limit)
                .cloned()
                .collect()
        };
        
        Ok(keys)
    }
    
    async fn get_stats(&self) -> Result<StorageStats> {
        let data = self.data.read().await;
        let total_bytes: usize = data.values().map(|v| v.len()).sum();
        
        Ok(StorageStats {
            total_files: data.len(),
            total_bytes,
            capacity_mb: self.capacity_mb,
            used_mb: total_bytes as f64 / (1024.0 * 1024.0),
        })
    }
    
    async fn clear(&mut self) -> Result<()> {
        let mut data = self.data.write().await;
        data.clear();
        Ok(())
    }
}

/// Disk-based storage backend
pub struct DiskStorage {
    base_path: PathBuf,
    capacity_mb: usize,
    index: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl DiskStorage {
    pub async fn new(base_path: &str, capacity_mb: usize) -> Result<Self> {
        let base_path = PathBuf::from(base_path);
        
        // Create directory if it doesn't exist
        tokio::fs::create_dir_all(&base_path).await?;
        
        // Load existing index
        let mut index = HashMap::new();
        let mut entries = tokio::fs::read_dir(&base_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let file_name = entry.file_name();
                if let Some(name) = file_name.to_str() {
                    if name.ends_with(".dht") {
                        let key = name.trim_end_matches(".dht");
                        index.insert(key.to_string(), entry.path());
                    }
                }
            }
        }
        
        Ok(Self {
            base_path,
            capacity_mb,
            index: Arc::new(RwLock::new(index)),
        })
    }
    
    fn get_file_path(&self, key: &str) -> PathBuf {
        // Hash the key to create a safe filename
        let hash = blake3::hash(key.as_bytes());
        let filename = format!("{}.dht", hex::encode(&hash.as_bytes()[..16]));
        self.base_path.join(filename)
    }
}

#[async_trait]
impl StorageBackend for DiskStorage {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<()> {
        // Check capacity
        let stats = self.get_stats().await?;
        let new_size = stats.total_bytes + value.len();
        
        if new_size > self.capacity_mb * 1024 * 1024 {
            return Err(anyhow::anyhow!("Storage capacity exceeded"));
        }
        
        let file_path = self.get_file_path(key);
        
        // Write data to file
        tokio::fs::write(&file_path, value).await?;
        
        // Update index
        let mut index = self.index.write().await;
        index.insert(key.to_string(), file_path);
        
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let index = self.index.read().await;
        
        if let Some(path) = index.get(key) {
            match tokio::fs::read(path).await {
                Ok(data) => Ok(Some(data)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            }
        } else {
            // Try direct path in case index is out of sync
            let file_path = self.get_file_path(key);
            match tokio::fs::read(&file_path).await {
                Ok(data) => {
                    // Update index
                    drop(index);
                    let mut index = self.index.write().await;
                    index.insert(key.to_string(), file_path);
                    Ok(Some(data))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            }
        }
    }
    
    async fn delete(&mut self, key: &str) -> Result<bool> {
        let mut index = self.index.write().await;
        
        if let Some(path) = index.remove(key) {
            match tokio::fs::remove_file(&path).await {
                Ok(_) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(e.into()),
            }
        } else {
            // Try direct path
            let file_path = self.get_file_path(key);
            match tokio::fs::remove_file(&file_path).await {
                Ok(_) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(e.into()),
            }
        }
    }
    
    async fn list_keys(&self, prefix: Option<&str>, limit: usize) -> Result<Vec<String>> {
        let index = self.index.read().await;
        
        let keys: Vec<String> = if let Some(prefix) = prefix {
            index.keys()
                .filter(|k| k.starts_with(prefix))
                .take(limit)
                .cloned()
                .collect()
        } else {
            index.keys()
                .take(limit)
                .cloned()
                .collect()
        };
        
        Ok(keys)
    }
    
    async fn get_stats(&self) -> Result<StorageStats> {
        let index = self.index.read().await;
        let mut total_bytes = 0;
        
        for path in index.values() {
            if let Ok(metadata) = tokio::fs::metadata(path).await {
                total_bytes += metadata.len() as usize;
            }
        }
        
        Ok(StorageStats {
            total_files: index.len(),
            total_bytes,
            capacity_mb: self.capacity_mb,
            used_mb: total_bytes as f64 / (1024.0 * 1024.0),
        })
    }
    
    async fn clear(&mut self) -> Result<()> {
        let mut index = self.index.write().await;
        
        // Delete all files
        for path in index.values() {
            let _ = tokio::fs::remove_file(path).await;
        }
        
        index.clear();
        Ok(())
    }
}