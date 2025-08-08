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

pub use cache::{BootstrapCache, CacheConfig, CacheError};
pub use contact::{
    ContactEntry, QualityCalculator, QualityMetrics, QuicConnectionType, QuicContactInfo,
    QuicQualityMetrics,
};
pub use discovery::{BootstrapConfig, BootstrapDiscovery, ConfigurableBootstrapDiscovery};
pub use merge::{MergeCoordinator, MergeResult};
// TODO: Re-enable when four_word_networking crate is available
// pub use four_word_networking::{FourWordAddress, WordDictionary, WordEncoder};

/// Placeholder for FourWordAddress
#[derive(Debug, Clone)]
pub struct FourWordAddress(pub String);

impl FourWordAddress {
    /// Create from a string
    pub fn from_string(s: &str) -> Result<Self> {
        // Simple validation: ensure it has exactly 4 words separated by dots or hyphens
        let parts: Vec<&str> = s.split(['.', '-']).collect();
        if parts.len() != 4 {
            return Err(P2PError::Bootstrap(
                crate::error::BootstrapError::InvalidData(
                    "Four-word address must have exactly 4 words"
                        .to_string()
                        .into(),
                ),
            ));
        }

        // Basic validation: each word should be non-empty and contain only letters
        for part in &parts {
            if part.is_empty() || !part.chars().all(|c| c.is_alphabetic()) {
                return Err(P2PError::Bootstrap(
                    crate::error::BootstrapError::InvalidData(
                        "Invalid word in four-word address".to_string().into(),
                    ),
                ));
            }
        }

        Ok(FourWordAddress(s.to_string()))
    }

    /// Validate against a word encoder
    pub fn validate(&self, _encoder: &WordEncoder) -> bool {
        // Placeholder validation - in real implementation would check against dictionary
        let parts: Vec<&str> = self.0.split(['.', '-']).collect();
        parts.len() == 4 && parts.iter().all(|part| !part.is_empty())
    }
}

/// Placeholder for WordDictionary
#[derive(Debug, Clone)]
pub struct WordDictionary;

/// Placeholder for WordEncoder
#[derive(Debug, Clone)]
pub struct WordEncoder;

impl Default for WordEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl WordEncoder {
    pub fn new() -> Self {
        Self
    }

    /// Encode a multiaddr string to a four-word address
    pub fn encode_multiaddr_string(&self, multiaddr: &str) -> Result<FourWordAddress> {
        // Placeholder implementation: generate deterministic words from multiaddr
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        multiaddr.hash(&mut hasher);
        let hash = hasher.finish();

        // Simple word lists for demonstration
        let words1 = [
            "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
        ];
        let words2 = [
            "red", "blue", "green", "yellow", "purple", "orange", "pink", "brown",
        ];
        let words3 = [
            "cat", "dog", "bird", "fish", "lion", "bear", "wolf", "eagle",
        ];
        let words4 = [
            "one", "two", "three", "four", "five", "six", "seven", "eight",
        ];

        let word1 = words1[(hash % words1.len() as u64) as usize];
        let word2 = words2[((hash >> 16) % words2.len() as u64) as usize];
        let word3 = words3[((hash >> 32) % words3.len() as u64) as usize];
        let word4 = words4[((hash >> 48) % words4.len() as u64) as usize];

        Ok(FourWordAddress(
            format!("{word1}.{word2}.{word3}.{word4}").into(),
        ))
    }

    /// Decode a four-word address to a socket address
    pub fn decode_to_socket_addr(&self, words: &FourWordAddress) -> Result<std::net::SocketAddr> {
        // Placeholder implementation: deterministic mapping from words to IP+port
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        words.0.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate deterministic IP and port from hash
        let ip_bytes = [
            ((hash >> 24) & 0xFF) as u8,
            ((hash >> 16) & 0xFF) as u8,
            ((hash >> 8) & 0xFF) as u8,
            (hash & 0xFF) as u8,
        ];

        // Use private IP ranges for placeholder (10.x.x.x)
        let ip = std::net::Ipv4Addr::new(10, ip_bytes[1], ip_bytes[2], ip_bytes[3]);
        let port = 9000 + ((hash >> 32) & 0xFFFF) as u16 % 1000; // Port range 9000-9999

        Ok(std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            ip, port,
        )))
    }

    /// Encode a socket address to a four-word address
    pub fn encode_socket_addr(&self, addr: &std::net::SocketAddr) -> Result<FourWordAddress> {
        // Use the existing encode_multiaddr_string logic but adapted for socket addr
        let addr_string = format!("{addr}");
        self.encode_multiaddr_string(&addr_string)
    }
}

use crate::error::BootstrapError;
use crate::{P2PError, PeerId, Result};
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
    pub async fn update_contact_metrics(
        &mut self,
        peer_id: &PeerId,
        metrics: QualityMetrics,
    ) -> Result<()> {
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
        self.merge_coordinator
            .merge_instance_caches(&self.cache)
            .await
    }

    /// Convert socket address to four-word address
    pub fn encode_address(&self, socket_addr: &std::net::SocketAddr) -> Result<FourWordAddress> {
        self.word_encoder
            .encode_socket_addr(socket_addr)
            .map_err(|e| {
                crate::P2PError::Bootstrap(crate::error::BootstrapError::InvalidData(
                    format!("Failed to encode socket address: {e}").into(),
                ))
            })
    }

    /// Convert four-word address to socket address
    pub fn decode_address(&self, words: &FourWordAddress) -> Result<std::net::SocketAddr> {
        self.word_encoder.decode_to_socket_addr(words).map_err(|e| {
            crate::P2PError::Bootstrap(crate::error::BootstrapError::InvalidData(
                format!("Failed to decode four-word address: {e}").into(),
            ))
        })
    }

    /// Validate four-word address format
    pub fn validate_words(&self, words: &FourWordAddress) -> Result<()> {
        if words.validate(&self.word_encoder) {
            Ok(())
        } else {
            Err(crate::P2PError::Bootstrap(
                crate::error::BootstrapError::InvalidData(
                    "Invalid four-word address format".to_string().into(),
                ),
            ))
        }
    }

    /// Get the word encoder for direct access
    pub fn word_encoder(&self) -> &WordEncoder {
        &self.word_encoder
    }

    /// Get well-known bootstrap addresses as four-word addresses
    pub fn get_well_known_word_addresses(&self) -> Vec<(FourWordAddress, std::net::SocketAddr)> {
        let well_known_addrs = vec![
            // Primary bootstrap nodes with well-known addresses
            std::net::SocketAddr::from(([0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888], 9000)),
            std::net::SocketAddr::from(([0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8844], 9001)),
            std::net::SocketAddr::from(([0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111], 9002)),
        ];

        well_known_addrs
            .into_iter()
            .filter_map(|socket_addr| {
                if let Ok(words) = self.encode_address(&socket_addr) {
                    Some((words, socket_addr))
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

    // QUIC-specific statistics
    /// Number of contacts with QUIC networking support
    pub iroh_contacts: usize,
    /// Number of contacts with successful NAT traversal (deprecated)
    pub nat_traversal_contacts: usize,
    /// Average QUIC connection setup time (milliseconds)
    pub avg_iroh_setup_time_ms: f64,
    /// Most successful QUIC connection type
    pub preferred_iroh_connection_type: Option<String>,
}

/// Get the home cache directory
fn home_cache_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| {
            P2PError::Bootstrap(BootstrapError::CacheError(
                "Unable to determine home directory".to_string().into(),
            ))
        })?;

    let cache_dir = PathBuf::from(home).join(DEFAULT_CACHE_DIR);

    // Ensure cache directory exists
    std::fs::create_dir_all(&cache_dir).map_err(|e| {
        P2PError::Bootstrap(BootstrapError::CacheError(
            format!("Failed to create cache directory: {e}").into(),
        ))
    })?;

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
