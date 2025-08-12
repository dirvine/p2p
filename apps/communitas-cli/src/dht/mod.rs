// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// DHT integration for CLI bootstrap node

pub mod commands;
pub mod manager;
pub mod storage;

pub use commands::{DHTCommands, execute_dht_command};
pub use manager::{DHTManager, DHTOperationResult};
pub use storage::{StorageBackend, DiskStorage, StorageStats};

use saorsa_core::dht::{Key, Record};
use std::time::Duration;

/// DHT configuration for bootstrap nodes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DHTBootstrapConfig {
    /// Replication factor
    pub replication_factor: usize,
    /// Storage capacity in MB
    pub storage_capacity_mb: usize,
    /// Enable persistent storage
    pub persistent_storage: bool,
    /// Storage directory path
    pub storage_path: String,
    /// Record expiration time
    pub record_ttl: Duration,
    /// Enable geographic routing optimization
    pub geographic_routing: bool,
    /// Enable automatic data rebalancing
    pub auto_rebalance: bool,
    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,
}

impl Default for DHTBootstrapConfig {
    fn default() -> Self {
        Self {
            replication_factor: 8,
            storage_capacity_mb: 1024, // 1GB default
            persistent_storage: true,
            storage_path: "./dht_storage".to_string(),
            record_ttl: Duration::from_secs(86400), // 24 hours
            geographic_routing: true,
            auto_rebalance: true,
            max_concurrent_ops: 100,
        }
    }
}

/// DHT operation metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DHTMetrics {
    pub total_records: usize,
    pub storage_used_mb: f64,
    pub get_requests: u64,
    pub put_requests: u64,
    pub lookup_requests: u64,
    pub replication_count: usize,
    pub avg_response_time_ms: f64,
    pub cache_hit_rate: f64,
}