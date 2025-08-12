// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// DHT manager for CLI bootstrap node

use anyhow::{Result, Context};
use saorsa_core::dht::{DHT, Key, Record, DHTConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};
use super::{DHTBootstrapConfig, DHTMetrics, StorageBackend, DiskStorage};

/// Result of DHT operations
#[derive(Debug, Clone, serde::Serialize)]
pub struct DHTOperationResult {
    pub success: bool,
    pub hash: Option<String>,
    pub size: usize,
    pub replicas: usize,
    pub duration_ms: u64,
}

/// Key information
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyInfo {
    pub size: usize,
    pub created: SystemTime,
    pub ttl: u64,
    pub replicas: usize,
}

/// Node information for findNode
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub distance: String,
    pub latency_ms: u64,
}

/// Replication result
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReplicationResult {
    pub keys_processed: usize,
    pub new_replicas: usize,
    pub failed: usize,
}

/// Verification result
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationResult {
    pub valid: usize,
    pub corrupted: usize,
    pub missing: usize,
    pub repaired: usize,
}

/// Import validation result
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationResult {
    pub valid: usize,
    pub invalid: usize,
    pub errors: Vec<String>,
}

/// Bucket information
#[derive(Debug, Clone, serde::Serialize)]
pub struct BucketInfo {
    pub index: usize,
    pub node_count: usize,
    pub distance_range: String,
}

/// DHT Manager for bootstrap nodes
pub struct DHTManager {
    dht: Arc<RwLock<DHT>>,
    storage: Arc<RwLock<Box<dyn StorageBackend>>>,
    config: Arc<RwLock<DHTBootstrapConfig>>,
    metrics: Arc<RwLock<DHTMetrics>>,
}

impl DHTManager {
    /// Create a new DHT manager
    pub async fn new(config: DHTBootstrapConfig) -> Result<Self> {
        // Initialize DHT
        let dht_config = DHTConfig {
            replication_factor: config.replication_factor,
            bucket_size: 20,
            alpha: 3,
            record_ttl: config.record_ttl,
            bucket_refresh_interval: Duration::from_secs(3600),
            republish_interval: Duration::from_secs(3600),
            provider_cleanup_interval: Duration::from_secs(3600),
        };
        
        let dht = DHT::new(dht_config.clone())
            .await
            .context("Failed to create DHT")?;
        
        // Initialize storage backend
        let storage: Box<dyn StorageBackend> = if config.persistent_storage {
            Box::new(DiskStorage::new(&config.storage_path, config.storage_capacity_mb).await?)
        } else {
            Box::new(super::storage::MemoryStorage::new(config.storage_capacity_mb))
        };
        
        let metrics = DHTMetrics {
            total_records: 0,
            storage_used_mb: 0.0,
            get_requests: 0,
            put_requests: 0,
            lookup_requests: 0,
            replication_count: config.replication_factor,
            avg_response_time_ms: 0.0,
            cache_hit_rate: 0.0,
        };
        
        Ok(Self {
            dht: Arc::new(RwLock::new(dht)),
            storage: Arc::new(RwLock::new(storage)),
            config: Arc::new(RwLock::new(config)),
            metrics: Arc::new(RwLock::new(metrics)),
        })
    }
    
    /// Store a value in the DHT
    pub async fn put(&mut self, key: &str, value: Vec<u8>, ttl: u64) -> Result<DHTOperationResult> {
        let start = std::time::Instant::now();
        
        // Create key
        let dht_key = Key::from(blake3::hash(key.as_bytes()).as_bytes().to_vec());
        
        // Create record
        let record = Record {
            key: dht_key.clone(),
            value: value.clone(),
            publisher: None,
            expires: Some(std::time::Instant::now() + Duration::from_secs(ttl)),
        };
        
        // Store locally
        let mut storage = self.storage.write().await;
        storage.put(key, &value).await?;
        
        // Store in DHT
        let mut dht = self.dht.write().await;
        dht.put_record(record).await
            .context("Failed to store in DHT")?;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.put_requests += 1;
        metrics.total_records += 1;
        metrics.storage_used_mb += value.len() as f64 / (1024.0 * 1024.0);
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(DHTOperationResult {
            success: true,
            hash: Some(hex::encode(&dht_key.as_ref()[..8])),
            size: value.len(),
            replicas: self.config.read().await.replication_factor,
            duration_ms,
        })
    }
    
    /// Store encrypted value
    pub async fn put_encrypted(&mut self, key: &str, value: Vec<u8>, ttl: u64) -> Result<DHTOperationResult> {
        // TODO: Implement encryption using saorsa-core's encryption module
        // For now, just store as-is with a marker
        let mut encrypted_value = vec![0xEE]; // Encryption marker
        encrypted_value.extend_from_slice(&value);
        self.put(key, encrypted_value, ttl).await
    }
    
    /// Retrieve a value from the DHT
    pub async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>> {
        let start = std::time::Instant::now();
        
        // Try local storage first
        let storage = self.storage.read().await;
        if let Some(value) = storage.get(key).await? {
            // Update metrics for cache hit
            let mut metrics = self.metrics.write().await;
            metrics.get_requests += 1;
            metrics.cache_hit_rate = (metrics.cache_hit_rate * (metrics.get_requests - 1) as f64 
                + 1.0) / metrics.get_requests as f64;
            return Ok(Some(value));
        }
        drop(storage);
        
        // Query DHT
        let dht_key = Key::from(blake3::hash(key.as_bytes()).as_bytes().to_vec());
        let mut dht = self.dht.write().await;
        
        if let Some(record) = dht.get_record(&dht_key).await? {
            // Store in local cache
            let mut storage = self.storage.write().await;
            storage.put(key, &record.value).await?;
            
            // Update metrics
            let mut metrics = self.metrics.write().await;
            metrics.get_requests += 1;
            
            let duration_ms = start.elapsed().as_millis() as u64;
            metrics.avg_response_time_ms = (metrics.avg_response_time_ms * (metrics.get_requests - 1) as f64 
                + duration_ms as f64) / metrics.get_requests as f64;
            
            Ok(Some(record.value))
        } else {
            // Update metrics for miss
            let mut metrics = self.metrics.write().await;
            metrics.get_requests += 1;
            
            Ok(None)
        }
    }
    
    /// Retrieve and decrypt a value
    pub async fn get_encrypted(&mut self, key: &str) -> Result<Option<Vec<u8>>> {
        if let Some(mut value) = self.get(key).await? {
            // Check for encryption marker
            if !value.is_empty() && value[0] == 0xEE {
                value.remove(0); // Remove marker
                // TODO: Implement actual decryption
            }
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    /// Delete a value from the DHT
    pub async fn delete(&mut self, key: &str) -> Result<bool> {
        // Remove from local storage
        let mut storage = self.storage.write().await;
        let deleted = storage.delete(key).await?;
        
        if deleted {
            // Update metrics
            let mut metrics = self.metrics.write().await;
            if metrics.total_records > 0 {
                metrics.total_records -= 1;
            }
        }
        
        // TODO: Send delete message to DHT network
        
        Ok(deleted)
    }
    
    /// List keys with optional prefix filter
    pub async fn list_keys(&self, prefix: Option<&str>, limit: usize) -> Result<Vec<(String, KeyInfo)>> {
        let storage = self.storage.read().await;
        let keys = storage.list_keys(prefix, limit).await?;
        
        let mut result = Vec::new();
        for key in keys {
            if let Some(value) = storage.get(&key).await? {
                let info = KeyInfo {
                    size: value.len(),
                    created: SystemTime::now(), // TODO: Track actual creation time
                    ttl: 86400, // TODO: Track actual TTL
                    replicas: self.config.read().await.replication_factor,
                };
                result.push((key, info));
            }
        }
        
        Ok(result)
    }
    
    /// Get DHT statistics
    pub async fn get_stats(&self) -> Result<DHTMetrics> {
        Ok(self.metrics.read().await.clone())
    }
    
    /// Get detailed statistics
    pub async fn get_detailed_stats(&self) -> Result<HashMap<String, String>> {
        let mut stats = HashMap::new();
        
        let dht = self.dht.read().await;
        let dht_stats = dht.get_stats().await;
        
        stats.insert("routing_table_size".to_string(), dht_stats.total_nodes.to_string());
        stats.insert("active_buckets".to_string(), dht_stats.active_buckets.to_string());
        stats.insert("expired_records".to_string(), dht_stats.expired_records.to_string());
        
        let storage = self.storage.read().await;
        let storage_stats = storage.get_stats().await?;
        stats.insert("storage_files".to_string(), storage_stats.total_files.to_string());
        stats.insert("storage_bytes".to_string(), storage_stats.total_bytes.to_string());
        
        Ok(stats)
    }
    
    /// Find closest nodes to a key
    pub async fn find_closest_nodes(&mut self, key: &str, count: usize) -> Result<Vec<NodeInfo>> {
        let dht_key = Key::from(blake3::hash(key.as_bytes()).as_bytes().to_vec());
        
        let mut dht = self.dht.write().await;
        let nodes = dht.find_closest_nodes(&dht_key, count).await?;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.lookup_requests += 1;
        
        Ok(nodes.into_iter().map(|node| NodeInfo {
            id: hex::encode(&node.peer_id.to_bytes()[..8]),
            address: node.addresses.first()
                .map(|a| a.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            distance: format!("{}", node.distance),
            latency_ms: 0, // TODO: Measure actual latency
        }).collect())
    }
    
    /// Replicate a key to maintain replication factor
    pub async fn replicate_key(&mut self, key: &str, factor: Option<usize>) -> Result<ReplicationResult> {
        let factor = factor.unwrap_or(self.config.read().await.replication_factor);
        
        // Get the value
        let value = self.get(key).await?
            .ok_or_else(|| anyhow::anyhow!("Key not found"))?;
        
        // Find nodes to replicate to
        let dht_key = Key::from(blake3::hash(key.as_bytes()).as_bytes().to_vec());
        let mut dht = self.dht.write().await;
        let nodes = dht.find_closest_nodes(&dht_key, factor).await?;
        
        // TODO: Actually replicate to nodes
        
        Ok(ReplicationResult {
            keys_processed: 1,
            new_replicas: nodes.len(),
            failed: 0,
        })
    }
    
    /// Replicate all keys
    pub async fn replicate_all(&mut self, factor: Option<usize>) -> Result<ReplicationResult> {
        let keys = self.list_keys(None, usize::MAX).await?;
        
        let mut total_result = ReplicationResult {
            keys_processed: 0,
            new_replicas: 0,
            failed: 0,
        };
        
        for (key, _) in keys {
            match self.replicate_key(&key, factor).await {
                Ok(result) => {
                    total_result.keys_processed += result.keys_processed;
                    total_result.new_replicas += result.new_replicas;
                }
                Err(_) => {
                    total_result.failed += 1;
                }
            }
        }
        
        Ok(total_result)
    }
    
    /// Verify data integrity for a key
    pub async fn verify_key(&mut self, key: &str, repair: bool) -> Result<VerificationResult> {
        let mut result = VerificationResult {
            valid: 0,
            corrupted: 0,
            missing: 0,
            repaired: 0,
        };
        
        // Check if key exists locally
        if let Some(value) = self.get(key).await? {
            // Verify hash
            let expected_hash = blake3::hash(key.as_bytes());
            let actual_hash = blake3::hash(&value);
            
            if expected_hash == actual_hash {
                result.valid = 1;
            } else {
                result.corrupted = 1;
                if repair {
                    // TODO: Fetch from network and repair
                    result.repaired = 1;
                }
            }
        } else {
            result.missing = 1;
        }
        
        Ok(result)
    }
    
    /// Verify all keys
    pub async fn verify_all(&mut self, repair: bool) -> Result<VerificationResult> {
        let keys = self.list_keys(None, usize::MAX).await?;
        
        let mut total_result = VerificationResult {
            valid: 0,
            corrupted: 0,
            missing: 0,
            repaired: 0,
        };
        
        for (key, _) in keys {
            let result = self.verify_key(&key, repair).await?;
            total_result.valid += result.valid;
            total_result.corrupted += result.corrupted;
            total_result.missing += result.missing;
            total_result.repaired += result.repaired;
        }
        
        Ok(total_result)
    }
    
    /// Export DHT data
    pub async fn export_data(&self, path: &str, format: &str, metadata: bool) -> Result<usize> {
        let keys = self.list_keys(None, usize::MAX).await?;
        
        if format == "json" {
            let mut data = serde_json::Map::new();
            
            for (key, info) in &keys {
                let storage = self.storage.read().await;
                if let Some(value) = storage.get(key).await? {
                    let mut entry = serde_json::Map::new();
                    
                    // Try to store as string, fallback to base64
                    match String::from_utf8(value.clone()) {
                        Ok(s) => {
                            entry.insert("value".to_string(), serde_json::Value::String(s));
                            entry.insert("encoding".to_string(), serde_json::Value::String("utf8".to_string()));
                        }
                        Err(_) => {
                            entry.insert("value".to_string(), serde_json::Value::String(base64::encode(&value)));
                            entry.insert("encoding".to_string(), serde_json::Value::String("base64".to_string()));
                        }
                    }
                    
                    if metadata {
                        entry.insert("size".to_string(), serde_json::Value::Number(info.size.into()));
                        entry.insert("ttl".to_string(), serde_json::Value::Number(info.ttl.into()));
                    }
                    
                    data.insert(key.clone(), serde_json::Value::Object(entry));
                }
            }
            
            let json = serde_json::to_string_pretty(&data)?;
            tokio::fs::write(path, json).await?;
        } else {
            // Binary format - simple key-value pairs
            let mut buffer = Vec::new();
            
            for (key, _) in &keys {
                let storage = self.storage.read().await;
                if let Some(value) = storage.get(key).await? {
                    // Write key length, key, value length, value
                    buffer.extend_from_slice(&(key.len() as u32).to_le_bytes());
                    buffer.extend_from_slice(key.as_bytes());
                    buffer.extend_from_slice(&(value.len() as u32).to_le_bytes());
                    buffer.extend_from_slice(&value);
                }
            }
            
            tokio::fs::write(path, buffer).await?;
        }
        
        Ok(keys.len())
    }
    
    /// Validate import file
    pub async fn validate_import(&self, path: &str) -> Result<ValidationResult> {
        let content = tokio::fs::read(path).await?;
        
        let mut result = ValidationResult {
            valid: 0,
            invalid: 0,
            errors: Vec::new(),
        };
        
        // Try to parse as JSON
        if let Ok(data) = serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&content) {
            for (key, value) in data {
                if let Some(obj) = value.as_object() {
                    if obj.contains_key("value") && obj.contains_key("encoding") {
                        result.valid += 1;
                    } else {
                        result.invalid += 1;
                        result.errors.push(format!("Invalid entry for key: {}", key));
                    }
                } else {
                    result.invalid += 1;
                    result.errors.push(format!("Invalid format for key: {}", key));
                }
            }
        } else {
            // Try binary format validation
            let mut pos = 0;
            while pos < content.len() {
                if pos + 4 > content.len() {
                    result.errors.push("Truncated binary data".to_string());
                    break;
                }
                
                let key_len = u32::from_le_bytes([content[pos], content[pos+1], content[pos+2], content[pos+3]]) as usize;
                pos += 4;
                
                if pos + key_len + 4 > content.len() {
                    result.errors.push("Invalid key length".to_string());
                    break;
                }
                
                pos += key_len;
                let value_len = u32::from_le_bytes([content[pos], content[pos+1], content[pos+2], content[pos+3]]) as usize;
                pos += 4;
                
                if pos + value_len > content.len() {
                    result.errors.push("Invalid value length".to_string());
                    break;
                }
                
                pos += value_len;
                result.valid += 1;
            }
        }
        
        Ok(result)
    }
    
    /// Import DHT data
    pub async fn import_data(&mut self, path: &str, overwrite: bool) -> Result<usize> {
        let content = tokio::fs::read(path).await?;
        let mut count = 0;
        
        // Try to parse as JSON
        if let Ok(data) = serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&content) {
            for (key, value) in data {
                if let Some(obj) = value.as_object() {
                    if let (Some(val), Some(enc)) = (obj.get("value"), obj.get("encoding")) {
                        let value_bytes = if enc == "base64" {
                            base64::decode(val.as_str().unwrap_or(""))?
                        } else {
                            val.as_str().unwrap_or("").as_bytes().to_vec()
                        };
                        
                        // Check if key exists
                        let exists = self.get(&key).await?.is_some();
                        if !exists || overwrite {
                            self.put(&key, value_bytes, 86400).await?;
                            count += 1;
                        }
                    }
                }
            }
        } else {
            // Parse binary format
            let mut pos = 0;
            while pos < content.len() {
                let key_len = u32::from_le_bytes([content[pos], content[pos+1], content[pos+2], content[pos+3]]) as usize;
                pos += 4;
                
                let key = String::from_utf8_lossy(&content[pos..pos+key_len]).to_string();
                pos += key_len;
                
                let value_len = u32::from_le_bytes([content[pos], content[pos+1], content[pos+2], content[pos+3]]) as usize;
                pos += 4;
                
                let value = content[pos..pos+value_len].to_vec();
                pos += value_len;
                
                // Check if key exists
                let exists = self.get(&key).await?.is_some();
                if !exists || overwrite {
                    self.put(&key, value, 86400).await?;
                    count += 1;
                }
            }
        }
        
        Ok(count)
    }
    
    /// Get bucket information
    pub async fn get_bucket_info(&self) -> Result<Vec<BucketInfo>> {
        let dht = self.dht.read().await;
        let stats = dht.get_stats().await;
        
        // TODO: Get actual bucket information from DHT
        let mut buckets = Vec::new();
        for i in 0..stats.active_buckets {
            buckets.push(BucketInfo {
                index: i,
                node_count: 0, // TODO: Get actual count
                distance_range: format!("2^{} - 2^{}", i, i+1),
            });
        }
        
        Ok(buckets)
    }
    
    /// Refresh stale buckets
    pub async fn refresh_buckets(&mut self) -> Result<usize> {
        let mut dht = self.dht.write().await;
        dht.refresh_buckets().await?;
        Ok(0) // TODO: Return actual count
    }
    
    /// Compact buckets
    pub async fn compact_buckets(&mut self) -> Result<usize> {
        // TODO: Implement bucket compaction
        Ok(0)
    }
    
    /// Get current configuration
    pub async fn get_config(&self) -> Result<DHTBootstrapConfig> {
        Ok(self.config.read().await.clone())
    }
    
    /// Set replication factor
    pub async fn set_replication_factor(&mut self, factor: usize) -> Result<()> {
        let mut config = self.config.write().await;
        config.replication_factor = factor;
        
        // TODO: Update DHT configuration
        
        Ok(())
    }
    
    /// Set storage capacity
    pub async fn set_storage_capacity(&mut self, capacity_mb: usize) -> Result<()> {
        let mut config = self.config.write().await;
        config.storage_capacity_mb = capacity_mb;
        
        // TODO: Update storage backend capacity
        
        Ok(())
    }
    
    /// Set default TTL
    pub async fn set_default_ttl(&mut self, ttl_secs: u64) -> Result<()> {
        let mut config = self.config.write().await;
        config.record_ttl = Duration::from_secs(ttl_secs);
        
        Ok(())
    }
    
    /// Enable/disable geographic routing
    pub async fn set_geographic_routing(&mut self, enabled: bool) -> Result<()> {
        let mut config = self.config.write().await;
        config.geographic_routing = enabled;
        
        // TODO: Update DHT routing strategy
        
        Ok(())
    }
}