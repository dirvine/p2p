// Copyright 2024 MaidSafe Limited
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
#!/usr/bin/env rust-script
//! Standalone test for enhanced DHT storage components
//! 
//! Run with: `rust-script test_enhanced_storage.rs`
//! Or with: `rustc test_enhanced_storage.rs && ./test_enhanced_storage`

use std::time::{Duration, SystemTime};
use std::collections::HashMap;

// Minimal PeerId for testing
pub type PeerId = String;

// Minimal Key implementation for testing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    hash: [u8; 32],
}

impl Key {
    pub fn from(data: Vec<u8>) -> Self {
        let mut hash = [0u8; 32];
        hash[..data.len().min(32)].copy_from_slice(&data[..data.len().min(32)]);
        Self { hash }
    }
}

/// Configuration for the K=8 replication system
#[derive(Debug, Clone)]
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
    pub failed_peers: Vec<(PeerId, String)>,
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

fn test_replication_config_default() {
    println!("Testing ReplicationConfig::default()...");
    let config = ReplicationConfig::default();
    assert_eq!(config.replication_factor, 8);
    assert_eq!(config.min_replication_factor, 3);
    assert_eq!(config.repair_threshold, 5);
    assert!(config.geographic_awareness);
    println!("✓ ReplicationConfig defaults are correct");
}

fn test_replication_result_health_check() {
    println!("Testing ReplicationResult health checks...");
    let result = ReplicationResult {
        key: Key::from(vec![1, 2, 3]),
        successful_replicas: 6,
        failed_replicas: 2,
        target_replicas: 8,
        successful_peers: vec!["peer1".to_string(), "peer2".to_string()],
        failed_peers: vec![("peer3".to_string(), "network error".to_string())],
        is_sufficient: true,
    };

    assert!(result.is_healthy(3));
    assert!(result.is_healthy(5));
    assert!(result.is_healthy(6));
    assert!(!result.is_healthy(7));
    assert!(!result.is_healthy(8));
    println!("✓ Health checks work correctly");
}

fn test_replication_result_success_rate() {
    println!("Testing ReplicationResult success rate calculation...");
    let result = ReplicationResult {
        key: Key::from(vec![1, 2, 3]),
        successful_replicas: 6,
        failed_replicas: 2,
        target_replicas: 8,
        successful_peers: vec![],
        failed_peers: vec![],
        is_sufficient: true,
    };

    assert_eq!(result.success_rate(), 0.75);
    assert_eq!(result.summary(), "Replication: 6/8 successful (75% success rate), 2 failed");
    println!("✓ Success rate calculation is correct");
}

fn test_peer_health_info() {
    println!("Testing PeerHealthInfo...");
    let mut health = PeerHealthInfo::default();
    assert_eq!(health.success_rate, 1.0);
    assert_eq!(health.total_attempts, 0);
    assert_eq!(health.successful_attempts, 0);
    
    // Simulate some operations
    health.total_attempts = 10;
    health.successful_attempts = 8;
    health.success_rate = health.successful_attempts as f64 / health.total_attempts as f64;
    
    assert_eq!(health.success_rate, 0.8);
    println!("✓ PeerHealthInfo works correctly");
}

fn test_repair_priority_ordering() {
    println!("Testing RepairPriority ordering...");
    assert!(RepairPriority::Critical > RepairPriority::High);
    assert!(RepairPriority::High > RepairPriority::Medium);
    assert!(RepairPriority::Medium > RepairPriority::Low);
    
    let mut priorities = vec![
        RepairPriority::Low,
        RepairPriority::Critical,
        RepairPriority::Medium,
        RepairPriority::High,
    ];
    
    priorities.sort();
    
    assert_eq!(priorities, vec![
        RepairPriority::Low,
        RepairPriority::Medium,
        RepairPriority::High,
        RepairPriority::Critical,
    ]);
    println!("✓ RepairPriority ordering is correct");
}

fn test_comprehensive_replication_scenario() {
    println!("Testing comprehensive replication scenario...");
    let config = ReplicationConfig {
        replication_factor: 5,
        min_replication_factor: 2,
        preferred_distance_factor: 0.3,
        geographic_awareness: true,
        repair_threshold: 3,
        repair_interval: Duration::from_secs(300),
        max_repair_concurrent: 2,
    };
    
    let key = Key::from(vec![10, 20, 30]);
    
    // Simulate partial replication success
    let result = ReplicationResult {
        key: key.clone(),
        successful_replicas: 3,
        failed_replicas: 2,
        target_replicas: config.replication_factor,
        successful_peers: vec![
            "peer_a".to_string(),
            "peer_b".to_string(),
            "peer_c".to_string(),
        ],
        failed_peers: vec![
            ("peer_d".to_string(), "timeout".to_string()),
            ("peer_e".to_string(), "connection refused".to_string()),
        ],
        is_sufficient: true,
    };
    
    // Verify the result meets minimum requirements
    assert!(result.is_healthy(config.min_replication_factor));
    assert!(!result.is_healthy(config.replication_factor));
    
    // Check if repair should be triggered
    let needs_repair = result.successful_replicas < config.repair_threshold;
    assert!(!needs_repair); // 3 >= 3, so no repair needed
    
    // Test success rate calculation
    assert_eq!(result.success_rate(), 0.6); // 3/5
    
    // Verify summary format
    let summary = result.summary();
    assert!(summary.contains("3/5 successful"));
    assert!(summary.contains("60% success rate"));
    assert!(summary.contains("2 failed"));
    
    println!("✓ Comprehensive scenario: {}", summary);
}

fn test_key_functionality() {
    println!("Testing Key functionality...");
    let key1 = Key::from(vec![1, 2, 3, 4, 5]);
    let key2 = Key::from(vec![1, 2, 3, 4, 5]);
    let key3 = Key::from(vec![6, 7, 8, 9, 10]);
    
    assert_eq!(key1, key2);
    assert_ne!(key1, key3);
    
    // Test that keys can be used in hash maps
    let mut key_map = HashMap::new();
    key_map.insert(key1.clone(), "value1");
    key_map.insert(key3.clone(), "value3");
    
    assert_eq!(key_map.get(&key2), Some(&"value1"));
    assert_eq!(key_map.len(), 2);
    println!("✓ Key functionality works correctly");
}

fn main() {
    println!("🧪 Running Enhanced DHT Storage Tests\n");
    
    test_replication_config_default();
    test_replication_result_health_check();
    test_replication_result_success_rate();
    test_peer_health_info();
    test_repair_priority_ordering();
    test_key_functionality();
    test_comprehensive_replication_scenario();
    
    println!("\n🎉 All tests passed! Enhanced DHT Storage foundation is working correctly.");
    println!("✅ Step 1 Complete: Basic replication configuration and types");
    
    println!("\n📋 Next Steps:");
    println!("  2. Implement peer selection strategy");
    println!("  3. Add replication tracking system");
    println!("  4. Implement repair scheduler");
    println!("  5. Create enhanced record manager");
    println!("  6. Write integration tests");
}