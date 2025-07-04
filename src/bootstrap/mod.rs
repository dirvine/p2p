
//! Bootstrap Cache System
//!
//! Provides decentralized peer discovery through local caching of known contacts.
//! Eliminates dependency on central bootstrap servers by maintaining a high-quality
//! cache of up to 30,000 peer contacts with automatic conflict resolution for
//! multiple concurrent instances.

pub mod cache;
pub mod contact;
pub mod discovery;
pub mod merge;
pub mod words;

pub use cache::{BootstrapCache, CacheConfig, CacheError};
pub use contact::{ContactEntry, QualityMetrics, QualityCalculator};
pub use discovery::{BootstrapDiscovery, BootstrapConfig, ConfigurableBootstrapDiscovery};
pub use merge::{MergeCoordinator, MergeResult};
pub use words::{ThreeWordAddress, WordDictionary, WordEncoder};

use crate::{Result, P2PError, PeerId};
use std::path::PathBuf;
use std::time::Duration;

/// Default cache configuration
pub const DEFAULT_MAX_CONTACTS: usize = 30_000;
/// Default directory for storing bootstrap cache files
pub const DEFAULT_CACHE_DIR: &str = ".cache/p2p_foundation";
/// Default interval for merging instance cache files
pub const DEFAULT_MERGE_INTERVAL: Duration = Duration::from_secs(30);
/// Default interval for cleaning up stale contacts (1 hour)
pub const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);
/// Default interval for updating contact quality scores (5 minutes)
pub const DEFAULT_QUALITY_UPDATE_INTERVAL: Duration = Duration::from_secs(300);

/// Bootstrap cache initialization and management
pub struct BootstrapManager {
    cache: BootstrapCache,
    merge_coordinator: MergeCoordinator,
    word_encoder: WordEncoder,
}

impl BootstrapManager {
    /// Create a new bootstrap manager with default configuration
    pub async fn new() -> Result<Self> {
        let cache_dir = home_cache_dir()?;
        let config = CacheConfig::default();
        
        let cache = BootstrapCache::new(cache_dir.clone(), config).await?;
        let merge_coordinator = MergeCoordinator::new(cache_dir)?;
        let word_encoder = WordEncoder::new();
        
        Ok(Self {
            cache,
            merge_coordinator,
            word_encoder,
        })
    }
    
    /// Create a new bootstrap manager with custom configuration
    pub async fn with_config(config: CacheConfig) -> Result<Self> {
        let cache_dir = home_cache_dir()?;
        
        let cache = BootstrapCache::new(cache_dir.clone(), config).await?;
        let merge_coordinator = MergeCoordinator::new(cache_dir)?;
        let word_encoder = WordEncoder::new();
        
        Ok(Self {
            cache,
            merge_coordinator,
            word_encoder,
        })
    }
    
    /// Get bootstrap peers for initial connection
    pub async fn get_bootstrap_peers(&self, count: usize) -> Result<Vec<ContactEntry>> {
        self.cache.get_bootstrap_peers(count).await
    }
    
    /// Add a discovered peer to the cache
    pub async fn add_contact(&mut self, contact: ContactEntry) -> Result<()> {
        self.cache.add_contact(contact).await
    }
    
    /// Update contact performance metrics
    pub async fn update_contact_metrics(&mut self, peer_id: &PeerId, metrics: QualityMetrics) -> Result<()> {
        self.cache.update_contact_metrics(peer_id, metrics).await
    }
    
    /// Start background maintenance tasks
    pub async fn start_background_tasks(&mut self) -> Result<()> {
        // Start periodic merge of instance caches
        let cache_clone = self.cache.clone();
        let merge_coordinator = self.merge_coordinator.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(DEFAULT_MERGE_INTERVAL);
            loop {
                interval.tick().await;
                if let Err(e) = merge_coordinator.merge_instance_caches(&cache_clone).await {
                    tracing::warn!("Failed to merge instance caches: {}", e);
                }
            }
        });
        
        // Start quality score updates
        let cache_clone = self.cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(DEFAULT_QUALITY_UPDATE_INTERVAL);
            loop {
                interval.tick().await;
                if let Err(e) = cache_clone.update_quality_scores().await {
                    tracing::warn!("Failed to update quality scores: {}", e);
                }
            }
        });
        
        // Start cleanup task
        let cache_clone = self.cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(DEFAULT_CLEANUP_INTERVAL);
            loop {
                interval.tick().await;
                if let Err(e) = cache_clone.cleanup_stale_entries().await {
                    tracing::warn!("Failed to cleanup stale entries: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        self.cache.get_stats().await
    }
    
    /// Force a cache merge operation
    pub async fn force_merge(&self) -> Result<MergeResult> {
        self.merge_coordinator.merge_instance_caches(&self.cache).await
    }
    
    /// Convert multiaddr to three-word address
    pub fn encode_address(&self, multiaddr: &crate::Multiaddr) -> Result<ThreeWordAddress> {
        self.word_encoder.encode_multiaddr(multiaddr)
    }
    
    /// Convert three-word address to multiaddr (requires registry lookup)
    pub fn decode_address(&self, words: &ThreeWordAddress) -> Result<crate::Multiaddr> {
        self.word_encoder.decode_to_multiaddr(words)
    }
    
    /// Validate three-word address format
    pub fn validate_words(&self, words: &ThreeWordAddress) -> Result<()> {
        words.validate(&self.word_encoder)
    }
    
    /// Get the word encoder for direct access
    pub fn word_encoder(&self) -> &WordEncoder {
        &self.word_encoder
    }
    
    /// Get well-known bootstrap addresses as three-word addresses
    pub fn get_well_known_word_addresses(&self) -> Vec<(ThreeWordAddress, crate::Multiaddr)> {
        let well_known_addrs = vec![
            // Primary bootstrap nodes with well-known addresses
            "/ip6/2001:4860:4860::8888/udp/9000/quic", // Example - would be real bootstrap nodes
            "/ip6/2001:4860:4860::8844/udp/9001/quic",
            "/ip6/2606:4700:4700::1111/udp/9002/quic",
        ];
        
        well_known_addrs
            .into_iter()
            .filter_map(|addr_str| {
                if let Ok(multiaddr) = addr_str.parse() {
                    if let Ok(words) = self.encode_address(&multiaddr) {
                        Some((words, multiaddr))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    /// Total number of contacts in the cache
    pub total_contacts: usize,
    /// Number of contacts with high quality scores
    pub high_quality_contacts: usize,
    /// Number of contacts with verified IPv6 identity
    pub verified_contacts: usize,
    /// Timestamp of the last cache merge operation
    pub last_merge: chrono::DateTime<chrono::Utc>,
    /// Timestamp of the last cache cleanup operation
    pub last_cleanup: chrono::DateTime<chrono::Utc>,
    /// Cache hit rate for peer discovery operations
    pub cache_hit_rate: f64,
    /// Average quality score across all contacts
    pub average_quality_score: f64,
}

/// Get the home cache directory
fn home_cache_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| P2PError::Bootstrap("Unable to determine home directory".to_string()))?;
    
    let cache_dir = PathBuf::from(home).join(DEFAULT_CACHE_DIR);
    
    // Ensure cache directory exists
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| P2PError::Bootstrap(format!("Failed to create cache directory: {}", e)))?;
    
    Ok(cache_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_bootstrap_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_contacts: 1000,
            ..CacheConfig::default()
        };
        
        let manager = BootstrapManager::with_config(config).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_home_cache_dir() {
        let result = home_cache_dir();
        assert!(result.is_ok());
        
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }
}