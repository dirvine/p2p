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

//! Enhanced DHT Storage with K=8 Replication
//!
//! This module extends the basic DHT functionality with advanced replication,
//! peer selection, and repair mechanisms for multi-user P2P applications.

use crate::PeerId;
use crate::dht::Key;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Configuration for the K=8 replication system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Number of replicas to maintain (K=8 for production)
    pub replication_factor: usize,
    /// Minimum acceptable replicas before triggering emergency repair
    pub min_replication_factor: usize,
    /// XOR distance preference factor (0.0 = pure distance, 1.0 = pure random)
    pub preferred_distance_factor: f64,
    /// Whether to consider geographic distribution in peer selection
    pub geographic_awareness: bool,
    /// Trigger repair when replicas fall below this threshold
    pub repair_threshold: usize,
    /// How often to check for needed repairs
    pub repair_interval: Duration,
    /// Maximum concurrent repair operations
    pub max_repair_concurrent: usize,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            replication_factor: 8,
            min_replication_factor: 3,
            preferred_distance_factor: 0.3,
            geographic_awareness: true,
            repair_threshold: 5,
            repair_interval: Duration::from_secs(300), // 5 minutes
            max_repair_concurrent: 3,
        }
    }
}

/// Result of a replication operation
#[derive(Debug, Clone)]
pub struct ReplicationResult {
    pub key: Key,
    pub successful_replicas: usize,
    pub failed_replicas: usize,
    pub target_replicas: usize,
    pub successful_peers: Vec<PeerId>,
    pub failed_peers: Vec<(PeerId, String)>, // Simplified error representation
    pub is_sufficient: bool,
}

impl ReplicationResult {
    /// Check if replication meets minimum requirements
    pub fn is_healthy(&self, min_replicas: usize) -> bool {
        self.successful_replicas >= min_replicas
    }

    /// Calculate replication success rate
    pub fn success_rate(&self) -> f64 {
        if self.target_replicas == 0 {
            0.0
        } else {
            self.successful_replicas as f64 / self.target_replicas as f64
        }
    }

    /// Get a summary of the replication attempt
    pub fn summary(&self) -> String {
        format!(
            "Replication: {}/{} successful ({}% success rate), {} failed",
            self.successful_replicas,
            self.target_replicas,
            (self.success_rate() * 100.0) as u32,
            self.failed_replicas
        )
    }
}

/// Errors that can occur during replication
#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("Insufficient peers available: required {required}, available {available}")]
    InsufficientPeers { required: usize, available: usize },

    #[error("No peers available for replication")]
    NoPeersAvailable,

    #[error("Network error during replication: {0}")]
    NetworkError(String),

    #[error("DHT operation failed: {0}")]
    DhtError(String),

    #[error("Geographic information unavailable")]
    GeographicInfoUnavailable,

    #[error("Timeout during replication operation")]
    Timeout,

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Repair operation failed: {0}")]
    RepairFailed(String),
}

/// Geographic information for peer distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerGeographicInfo {
    pub peer_id: PeerId,
    pub region: String,        // Geographic region (e.g., "us-east", "eu-west")
    pub country_code: String,  // ISO country code
    pub latitude: Option<f64>, // Approximate coordinates
    pub longitude: Option<f64>,
    pub network_provider: Option<String>, // ISP or cloud provider
    pub estimated_rtt: Option<Duration>,  // Round-trip time estimate
}

/// Health information for individual peers
#[derive(Debug, Clone)]
pub struct PeerHealthInfo {
    pub success_rate: f64,
    pub last_successful_store: SystemTime,
    pub last_failed_store: Option<SystemTime>,
    pub total_attempts: u64,
    pub successful_attempts: u64,
}

impl Default for PeerHealthInfo {
    fn default() -> Self {
        Self {
            success_rate: 1.0,
            last_successful_store: SystemTime::now(),
            last_failed_store: None,
            total_attempts: 0,
            successful_attempts: 0,
        }
    }
}

/// Priority levels for repair operations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepairPriority {
    Low,      // Replicas above threshold but below target
    Medium,   // Replicas at threshold
    High,     // Replicas below threshold
    Critical, // Very few replicas remaining
}

/// A repair task for maintaining replication levels
#[derive(Debug, Clone)]
pub struct RepairTask {
    pub key: Key,
    pub current_replicas: Vec<PeerId>,
    pub required_replicas: usize,
    pub priority: RepairPriority,
    pub scheduled_at: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replication_config_default() {
        let config = ReplicationConfig::default();
        assert_eq!(config.replication_factor, 8);
        assert_eq!(config.min_replication_factor, 3);
        assert_eq!(config.repair_threshold, 5);
        assert!(config.geographic_awareness);
    }

    #[test]
    fn test_replication_result_health_check() {
        let result = ReplicationResult {
            key: Key::new(&[1, 2, 3]),
            successful_replicas: 6,
            failed_replicas: 2,
            target_replicas: 8,
            successful_peers: vec![],
            failed_peers: vec![],
            is_sufficient: true,
        };

        assert!(result.is_healthy(3));
        assert!(result.is_healthy(5));
        assert!(!result.is_healthy(7));
    }

    #[test]
    fn test_replication_result_success_rate() {
        let result = ReplicationResult {
            key: Key::new(&[1, 2, 3]),
            successful_replicas: 6,
            failed_replicas: 2,
            target_replicas: 8,
            successful_peers: vec![],
            failed_peers: vec![],
            is_sufficient: true,
        };

        assert_eq!(result.success_rate(), 0.75);
        assert_eq!(
            result.summary(),
            "Replication: 6/8 successful (75% success rate), 2 failed"
        );
    }

    #[test]
    fn test_peer_health_info_default() {
        let health = PeerHealthInfo::default();
        assert_eq!(health.success_rate, 1.0);
        assert_eq!(health.total_attempts, 0);
        assert_eq!(health.successful_attempts, 0);
    }

    #[test]
    fn test_repair_priority_ordering() {
        assert!(RepairPriority::Critical > RepairPriority::High);
        assert!(RepairPriority::High > RepairPriority::Medium);
        assert!(RepairPriority::Medium > RepairPriority::Low);
    }
}
