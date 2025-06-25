//! Bootstrap Cache System
//!
//! Provides decentralized peer discovery through local caching of known contacts.
//! Eliminates dependency on central bootstrap servers by maintaining a high-quality
//! cache of up to 30,000 peer contacts with automatic conflict resolution for
//! multiple concurrent instances.

pub mod cache;
pub mod contact;
pub mod merge;

pub use cache::{BootstrapCache, CacheConfig, CacheError};
pub use contact::{ContactEntry, QualityMetrics, QualityCalculator};
pub use merge::{MergeCoordinator, MergeResult};

use crate::{Result, P2PError, PeerId};
use std::path::PathBuf;
use std::time::Duration;

/// Default cache configuration
pub const DEFAULT_MAX_CONTACTS: usize = 30_000;
pub const DEFAULT_CACHE_DIR: &str = ".cache/p2p_foundation";
pub const DEFAULT_MERGE_INTERVAL: Duration = Duration::from_secs(30);
pub const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour
pub const DEFAULT_QUALITY_UPDATE_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

/// Bootstrap cache initialization and management
pub struct BootstrapManager {
    cache: BootstrapCache,
    merge_coordinator: MergeCoordinator,
}

impl BootstrapManager {
    /// Create a new bootstrap manager with default configuration
    pub async fn new() -> Result<Self> {
        let cache_dir = home_cache_dir()?;
        let config = CacheConfig::default();
        
        let cache = BootstrapCache::new(cache_dir.clone(), config).await?;
        let merge_coordinator = MergeCoordinator::new(cache_dir)?;
        
        Ok(Self {
            cache,
            merge_coordinator,
        })
    }
    
    /// Create a new bootstrap manager with custom configuration
    pub async fn with_config(config: CacheConfig) -> Result<Self> {
        let cache_dir = home_cache_dir()?;
        
        let cache = BootstrapCache::new(cache_dir.clone(), config).await?;
        let merge_coordinator = MergeCoordinator::new(cache_dir)?;
        
        Ok(Self {
            cache,
            merge_coordinator,
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
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub total_contacts: usize,
    pub high_quality_contacts: usize,
    pub verified_contacts: usize,
    pub last_merge: chrono::DateTime<chrono::Utc>,
    pub last_cleanup: chrono::DateTime<chrono::Utc>,
    pub cache_hit_rate: f64,
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