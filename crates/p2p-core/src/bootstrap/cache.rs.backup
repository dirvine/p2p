
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

//! Bootstrap Cache Implementation
//!
//! Manages a persistent cache of peer contacts with quality-based selection,
//! automatic cleanup, and multi-instance coordination.

use crate::{PeerId, Result, P2PError};
use crate::bootstrap::{ContactEntry, QualityMetrics, QualityCalculator, CacheStats};
use crate::error::BootstrapError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

/// Bootstrap cache configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheConfig {
    /// Directory where cache files are stored
    pub cache_dir: PathBuf,
    /// Maximum number of contacts to keep in cache
    pub max_contacts: usize,
    /// Interval between cache merge operations
    pub merge_interval: Duration,
    /// Interval between cache cleanup operations
    pub cleanup_interval: Duration,
    /// Interval between quality score updates
    pub quality_update_interval: Duration,
    /// Age threshold for considering contacts stale
    pub stale_threshold: Duration,
    /// Interval between connectivity checks
    pub connectivity_check_interval: Duration,
    /// Number of peers to check connectivity with
    pub connectivity_check_count: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from(".cache/p2p_foundation"),
            max_contacts: crate::bootstrap::DEFAULT_MAX_CONTACTS,
            merge_interval: crate::bootstrap::DEFAULT_MERGE_INTERVAL,
            cleanup_interval: crate::bootstrap::DEFAULT_CLEANUP_INTERVAL,
            quality_update_interval: crate::bootstrap::DEFAULT_QUALITY_UPDATE_INTERVAL,
            stale_threshold: Duration::from_secs(86400 * 7), // 7 days
            connectivity_check_interval: Duration::from_secs(900), // 15 minutes
            connectivity_check_count: 100, // Check top 100 peers
        }
    }
}

/// Bootstrap cache errors
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// File I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// JSON serialization/deserialization failed
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Failed to acquire lock on cache
    #[error("Lock error: {0}")]
    Lock(String),
    
    /// Cache file corruption detected
    #[error("Cache corruption: {0}")]
    Corruption(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Main bootstrap cache implementation
#[derive(Clone)]
pub struct BootstrapCache {
    config: CacheConfig,
    contacts: Arc<RwLock<HashMap<PeerId, ContactEntry>>>,
    instance_id: String,
    cache_file: PathBuf,
    instance_cache_file: PathBuf,
    lock_file: PathBuf,
    metadata_file: PathBuf,
    quality_calculator: QualityCalculator,
    stats: Arc<RwLock<CacheStats>>,
}

/// Cached data structure for persistence
#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    version: u32,
    instance_id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    contacts: HashMap<PeerId, ContactEntry>,
    checksum: u64,
}

/// Cache metadata for health monitoring
#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    last_merge: chrono::DateTime<chrono::Utc>,
    last_cleanup: chrono::DateTime<chrono::Utc>,
    last_quality_update: chrono::DateTime<chrono::Utc>,
    total_merges: u64,
    total_cleanups: u64,
    corruption_count: u64,
    instance_count: u64,
}

impl BootstrapCache {
    /// Create a new bootstrap cache
    pub async fn new(cache_dir: PathBuf, config: CacheConfig) -> Result<Self> {
        // Ensure cache directory exists
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to create cache directory: {e}"))))?;
        
        let instance_id = generate_instance_id();
        
        let cache_file = cache_dir.join("bootstrap_cache.json");
        let instance_cache_file = cache_dir.join("instance_caches").join(format!("{instance_id}.cache"));
        let lock_file = cache_dir.join("bootstrap_cache.lock");
        let metadata_file = cache_dir.join("metadata.json");
        
        // Ensure instance cache directory exists
        std::fs::create_dir_all(instance_cache_file.parent().unwrap())
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to create instance cache directory: {e}"))))?;
        
        let mut cache = Self {
            config: config.clone(),
            contacts: Arc::new(RwLock::new(HashMap::new())),
            instance_id,
            cache_file,
            instance_cache_file,
            lock_file,
            metadata_file,
            quality_calculator: QualityCalculator::new(),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        };
        
        // Load existing cache
        cache.load_from_disk().await?;
        
        info!("Bootstrap cache initialized with {} contacts", cache.contacts.read().await.len());
        
        Ok(cache)
    }
    
    /// Get bootstrap peers for initial connection
    pub async fn get_bootstrap_peers(&self, count: usize) -> Result<Vec<ContactEntry>> {
        let contacts = self.contacts.read().await;
        
        let mut sorted_contacts: Vec<&ContactEntry> = contacts.values().collect();
        
        // Sort by quality score in descending order
        sorted_contacts.sort_by(|a, b| {
            b.quality_metrics.quality_score
                .partial_cmp(&a.quality_metrics.quality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let selected: Vec<ContactEntry> = sorted_contacts
            .into_iter()
            .take(count)
            .cloned()
            .collect();
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.cache_hit_rate = if !contacts.is_empty() {
                selected.len() as f64 / count.min(contacts.len()) as f64
            } else {
                0.0
            };
        }
        
        debug!("Selected {} bootstrap peers from {} available contacts", selected.len(), contacts.len());
        
        Ok(selected)
    }
    
    /// Get bootstrap peers that support QUIC networking
    pub async fn get_quic_bootstrap_peers(&self, count: usize) -> Result<Vec<ContactEntry>> {
        let contacts = self.contacts.read().await;
        
        // Filter for contacts with QUIC information
        let mut quic_contacts: Vec<&ContactEntry> = contacts.values()
            .filter(|contact| contact.quic_contact.is_some())
            .collect();
        
        // Sort by combined quality (regular + QUIC quality)
        quic_contacts.sort_by(|a, b| {
            let score_a = a.quality_metrics.quality_score + a.quic_quality_score() * 0.3;
            let score_b = b.quality_metrics.quality_score + b.quic_quality_score() * 0.3;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let selected: Vec<ContactEntry> = quic_contacts
            .into_iter()
            .take(count)
            .cloned()
            .collect();
        
        debug!("Selected {} QUIC bootstrap peers from {} available QUIC contacts", 
               selected.len(), contacts.values().filter(|c| c.quic_contact.is_some()).count());
        
        Ok(selected)
    }
    
    /// Get contacts by QUIC direct addresses
    pub async fn get_contact_by_addresses(&self, target_addresses: &Vec<SocketAddr>) -> Option<ContactEntry> {
        let contacts = self.contacts.read().await;
        contacts.values()
            .find(|contact| {
                if let Some(quic_addrs) = contact.quic_direct_addresses() {
                    quic_addrs.iter().any(|addr| target_addresses.contains(addr))
                } else {
                    false
                }
            })
            .cloned()
    }
    
    /// Update QUIC connection metrics for a contact
    pub async fn update_quic_metrics(
        &mut self, 
        peer_id: &PeerId, 
        connection_type: crate::bootstrap::contact::QuicConnectionType,
        success: bool,
        setup_time_ms: Option<u64>
    ) -> Result<()> {
        let mut contacts = self.contacts.write().await;
        
        if let Some(contact) = contacts.get_mut(peer_id) {
            contact.update_quic_connection_result(
                connection_type,
                success,
                setup_time_ms
            );
            
            debug!("Updated QUIC metrics for peer {}: {}", peer_id, contact.summary());
        }
        
        Ok(())
    }
    
    /// Add or update a contact
    pub async fn add_contact(&mut self, contact: ContactEntry) -> Result<()> {
        let mut contacts = self.contacts.write().await;
        
        // Check if we need to evict contacts
        if contacts.len() >= self.config.max_contacts && !contacts.contains_key(&contact.peer_id) {
            self.evict_lowest_quality_contacts(&mut contacts).await?;
        }
        
        contacts.insert(contact.peer_id.clone(), contact.clone());
        drop(contacts);
        
        // Save to instance cache
        self.save_to_instance_cache().await?;
        
        debug!("Added contact: {}", contact.summary());
        
        Ok(())
    }
    
    /// Update contact metrics
    pub async fn update_contact_metrics(&mut self, peer_id: &PeerId, metrics: QualityMetrics) -> Result<()> {
        let mut contacts = self.contacts.write().await;
        
        if let Some(contact) = contacts.get_mut(peer_id) {
            contact.quality_metrics = metrics;
            contact.recalculate_quality_score();
            
            debug!("Updated metrics for peer {}: {}", peer_id, contact.summary());
        }
        
        Ok(())
    }
    
    /// Update quality scores for all contacts
    pub async fn update_quality_scores(&self) -> Result<()> {
        let mut contacts = self.contacts.write().await;
        let mut updated_count = 0;
        
        for contact in contacts.values_mut() {
            let old_score = contact.quality_metrics.quality_score;
            
            // Apply age decay
            let age_seconds = contact.age_seconds() as f64;
            let decay_factor = (-age_seconds / 86400.0).exp(); // 24 hour half-life
            contact.quality_metrics.apply_age_decay(decay_factor);
            
            // Recalculate quality score
            contact.recalculate_quality_score();
            
            if (contact.quality_metrics.quality_score - old_score).abs() > 0.01 {
                updated_count += 1;
            }
        }
        
        // Update metadata
        self.update_metadata(|meta| {
            meta.last_quality_update = chrono::Utc::now();
        }).await?;
        
        debug!("Updated quality scores for {} contacts", updated_count);
        
        Ok(())
    }
    
    /// Clean up stale entries
    pub async fn cleanup_stale_entries(&self) -> Result<()> {
        let mut contacts = self.contacts.write().await;
        let initial_count = contacts.len();
        
        // Remove stale contacts
        contacts.retain(|_peer_id, contact| {
            !contact.is_stale(self.config.stale_threshold)
        });
        
        let removed_count = initial_count - contacts.len();
        
        if removed_count > 0 {
            info!("Cleaned up {} stale contacts", removed_count);
            
            // Save updated cache
            drop(contacts);
            self.save_to_disk().await?;
        }
        
        // Update metadata
        self.update_metadata(|meta| {
            meta.last_cleanup = chrono::Utc::now();
            meta.total_cleanups += 1;
        }).await?;
        
        Ok(())
    }
    
    /// Get all contacts (for merge operations)
    pub async fn get_all_contacts(&self) -> HashMap<PeerId, ContactEntry> {
        self.contacts.read().await.clone()
    }
    
    /// Set all contacts (for merge operations)
    pub async fn set_all_contacts(&self, contacts: HashMap<PeerId, ContactEntry>) {
        let mut current_contacts = self.contacts.write().await;
        *current_contacts = contacts;
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let contacts = self.contacts.read().await;
        let mut stats = self.stats.write().await;
        
        stats.total_contacts = contacts.len();
        stats.high_quality_contacts = contacts.values()
            .filter(|c| c.quality_metrics.quality_score > 0.7)
            .count();
        stats.verified_contacts = contacts.values()
            .filter(|c| c.ipv6_identity_verified)
            .count();
        
        // QUIC-specific statistics
        stats.iroh_contacts = contacts.values()
            .filter(|c| c.quic_contact.is_some())
            .count();
        stats.nat_traversal_contacts = 0; // NAT traversal not tracked in simplified QUIC implementation
        
        // Calculate average QUIC setup time
        let quic_setup_times: Vec<f64> = contacts.values()
            .filter_map(|c| c.quic_contact.as_ref())
            .filter(|quic| quic.quic_quality.avg_connection_setup_time_ms > 0.0)
            .map(|quic| quic.quic_quality.avg_connection_setup_time_ms)
            .collect();
        stats.avg_iroh_setup_time_ms = if !quic_setup_times.is_empty() {
            quic_setup_times.iter().sum::<f64>() / quic_setup_times.len() as f64
        } else {
            0.0
        };
        
        // Find most successful connection type
        let mut connection_type_counts = std::collections::HashMap::new();
        for contact in contacts.values() {
            if let Some(ref quic) = contact.quic_contact {
                for conn_type in &quic.successful_connection_types {
                    *connection_type_counts.entry(format!("{conn_type:?}")).or_insert(0) += 1;
                }
            }
        }
        stats.preferred_iroh_connection_type = connection_type_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(conn_type, _)| conn_type);
        
        if !contacts.is_empty() {
            stats.average_quality_score = contacts.values()
                .map(|c| c.quality_metrics.quality_score)
                .sum::<f64>() / contacts.len() as f64;
        }
        
        Ok(stats.clone())
    }
    
    /// Load cache from disk
    async fn load_from_disk(&mut self) -> Result<()> {
        if !self.cache_file.exists() {
            debug!("No existing cache file found, starting with empty cache");
            return Ok(());
        }
        
        let _lock = self.acquire_file_lock().await?;
        
        match self.load_cache_data().await {
            Ok(cache_data) => {
                if self.verify_cache_integrity(&cache_data) {
                    let mut contacts = self.contacts.write().await;
                    *contacts = cache_data.contacts;
                    info!("Loaded {} contacts from cache", contacts.len());
                } else {
                    warn!("Cache integrity check failed, starting with empty cache");
                    self.handle_cache_corruption().await?;
                }
            }
            Err(e) => {
                warn!("Failed to load cache: {}, starting with empty cache", e);
                self.handle_cache_corruption().await?;
            }
        }
        
        Ok(())
    }
    
    /// Save cache to disk
    pub async fn save_to_disk(&self) -> Result<()> {
        let _lock = self.acquire_file_lock().await?;
        
        let contacts = self.contacts.read().await;
        let cache_data = CacheData {
            version: 1,
            instance_id: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
            contacts: contacts.clone(),
            checksum: self.calculate_checksum(&contacts),
        };
        
        // Write to temporary file first for atomic operation
        let temp_file = self.cache_file.with_extension("tmp");
        let json_data = serde_json::to_string_pretty(&cache_data)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to serialize cache: {e}"))))?;
        
        std::fs::write(&temp_file, json_data)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to write cache file: {e}"))))?;
        
        // Atomic rename
        std::fs::rename(temp_file, &self.cache_file)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to rename cache file: {e}"))))?;
        
        debug!("Saved {} contacts to cache", contacts.len());
        
        Ok(())
    }
    
    /// Save to instance-specific cache
    async fn save_to_instance_cache(&self) -> Result<()> {
        let contacts = self.contacts.read().await;
        let cache_data = CacheData {
            version: 1,
            instance_id: self.instance_id.clone(),
            timestamp: chrono::Utc::now(),
            contacts: contacts.clone(),
            checksum: self.calculate_checksum(&contacts),
        };
        
        let json_data = serde_json::to_string(&cache_data)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to serialize instance cache: {e}"))))?;
        
        std::fs::write(&self.instance_cache_file, json_data)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to write instance cache: {e}"))))?;
        
        Ok(())
    }
    
    /// Acquire file lock for atomic operations
    async fn acquire_file_lock(&self) -> Result<FileLock> {
        FileLock::acquire(&self.lock_file).await
    }
    
    /// Load cache data from file
    async fn load_cache_data(&self) -> Result<CacheData> {
        let json_data = std::fs::read_to_string(&self.cache_file)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to read cache file: {e}"))))?;
        
        let cache_data: CacheData = serde_json::from_str(&json_data)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::InvalidData(format!("Failed to parse cache file: {e}"))))?;
        
        Ok(cache_data)
    }
    
    /// Verify cache integrity
    fn verify_cache_integrity(&self, cache_data: &CacheData) -> bool {
        let calculated_checksum = self.calculate_checksum(&cache_data.contacts);
        cache_data.checksum == calculated_checksum
    }
    
    /// Calculate checksum for cache integrity
    fn calculate_checksum(&self, contacts: &HashMap<PeerId, ContactEntry>) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Sort by peer ID for consistent hashing
        let mut sorted_contacts: Vec<_> = contacts.iter().collect();
        sorted_contacts.sort_by_key(|(peer_id, _)| *peer_id);
        
        for (peer_id, contact) in sorted_contacts {
            peer_id.hash(&mut hasher);
            contact.quality_metrics.success_rate.to_bits().hash(&mut hasher);
            contact.addresses.len().hash(&mut hasher);
        }
        
        hasher.finish()
    }
    
    /// Handle cache corruption
    async fn handle_cache_corruption(&self) -> Result<()> {
        warn!("Handling cache corruption, backing up corrupted file");
        
        if self.cache_file.exists() {
            let backup_file = self.cache_file.with_extension("corrupted");
            if let Err(e) = std::fs::rename(&self.cache_file, backup_file) {
                error!("Failed to backup corrupted cache: {}", e);
            }
        }
        
        // Update corruption count in metadata
        self.update_metadata(|meta| {
            meta.corruption_count += 1;
        }).await?;
        
        Ok(())
    }
    
    /// Evict lowest quality contacts to make room
    async fn evict_lowest_quality_contacts(&self, contacts: &mut HashMap<PeerId, ContactEntry>) -> Result<()> {
        let eviction_count = (self.config.max_contacts / 10).max(1); // Evict 10% or at least 1
        
        let mut sorted_contacts: Vec<_> = contacts.iter().collect();
        sorted_contacts.sort_by(|a, b| {
            a.1.quality_metrics.quality_score
                .partial_cmp(&b.1.quality_metrics.quality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let to_evict: Vec<PeerId> = sorted_contacts
            .into_iter()
            .take(eviction_count)
            .map(|(peer_id, _)| peer_id.clone())
            .collect();
        
        for peer_id in to_evict {
            contacts.remove(&peer_id);
        }
        
        debug!("Evicted {} lowest quality contacts", eviction_count);
        
        Ok(())
    }
    
    /// Update metadata
    async fn update_metadata<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut CacheMetadata),
    {
        let mut metadata = if self.metadata_file.exists() {
            let json_data = std::fs::read_to_string(&self.metadata_file)?;
            serde_json::from_str(&json_data).unwrap_or_default()
        } else {
            CacheMetadata::default()
        };
        
        updater(&mut metadata);
        
        let json_data = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&self.metadata_file, json_data)?;
        
        Ok(())
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            total_contacts: 0,
            high_quality_contacts: 0,
            verified_contacts: 0,
            last_merge: chrono::Utc::now(),
            last_cleanup: chrono::Utc::now(),
            cache_hit_rate: 0.0,
            average_quality_score: 0.0,
            iroh_contacts: 0,
            nat_traversal_contacts: 0,
            avg_iroh_setup_time_ms: 0.0,
            preferred_iroh_connection_type: None,
        }
    }
}

impl Default for CacheMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            last_merge: now,
            last_cleanup: now,
            last_quality_update: now,
            total_merges: 0,
            total_cleanups: 0,
            corruption_count: 0,
            instance_count: 0,
        }
    }
}

/// File locking for atomic operations
struct FileLock {
    _file: std::fs::File,
}

impl FileLock {
    async fn acquire(lock_file: &PathBuf) -> Result<Self> {
        use std::fs::OpenOptions;
        
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(lock_file)
            .map_err(|e| P2PError::Bootstrap(BootstrapError::CacheError(format!("Failed to create lock file: {e}"))))?;
        
        // In a production system, you'd use proper file locking here
        // For now, we'll rely on atomic file operations
        
        Ok(Self { _file: file })
    }
}

/// Generate unique instance ID
fn generate_instance_id() -> String {
    format!("{}_{}", std::process::id(), SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_contacts: 100,
            ..CacheConfig::default()
        };
        
        let cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await;
        assert!(cache.is_ok());
    }
    
    #[tokio::test]
    async fn test_add_and_retrieve_contacts() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_contacts: 100,
            ..CacheConfig::default()
        };
        
        let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await.unwrap();
        
        let contact = ContactEntry::new(
            PeerId::from("test-peer"),
            vec!["127.0.0.1:9000".parse().unwrap()]
        );
        
        cache.add_contact(contact).await.unwrap();
        
        let bootstrap_peers = cache.get_bootstrap_peers(10).await.unwrap();
        assert_eq!(bootstrap_peers.len(), 1);
    }
    
    #[tokio::test]
    async fn test_cache_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_contacts: 100,
            ..CacheConfig::default()
        };
        
        // Create cache and add contact
        {
            let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config.clone()).await.unwrap();
            let contact = ContactEntry::new(
                PeerId::from("test-peer"),
                vec!["127.0.0.1:9000".parse().unwrap()]
            );
            cache.add_contact(contact).await.unwrap();
            cache.save_to_disk().await.unwrap();
        }
        
        // Create new cache and verify contact is loaded
        {
            let cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await.unwrap();
            let bootstrap_peers = cache.get_bootstrap_peers(10).await.unwrap();
            assert_eq!(bootstrap_peers.len(), 1);
        }
    }
    
    #[tokio::test]
    async fn test_cache_eviction() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_contacts: 5,
            ..CacheConfig::default()
        };
        
        let mut cache = BootstrapCache::new(temp_dir.path().to_path_buf(), config).await.unwrap();
        
        // Add contacts exceeding the limit
        for i in 0..10 {
            let contact = ContactEntry::new(
                PeerId::from(format!("test-peer-{}", i)),
                vec![format!("127.0.0.1:{}", 9000 + i).parse().unwrap()]
            );
            cache.add_contact(contact).await.unwrap();
        }
        
        let stats = cache.get_stats().await.unwrap();
        assert!(stats.total_contacts <= 5);
    }
}