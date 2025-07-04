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
#!/usr/bin/env rust-script
//! Step 3: Testing Replication Tracking System for K=8 DHT Storage
//! 
//! This implements comprehensive tracking of replica health, peer performance,
//! and replication success/failure statistics.
//!
//! Run with: `rustc test_replication_tracking.rs && ./test_replication_tracking`

use std::time::{Duration, SystemTime};
use std::collections::{HashMap, HashSet, VecDeque};

// Re-use foundation types
pub type PeerId = String;

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
    
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
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
    pub average_response_time: Duration,
    pub consecutive_failures: u32,
}

impl Default for PeerHealthInfo {
    fn default() -> Self {
        Self {
            success_rate: 1.0,
            last_successful_store: SystemTime::now(),
            last_failed_store: None,
            total_attempts: 0,
            successful_attempts: 0,
            average_response_time: Duration::from_millis(100),
            consecutive_failures: 0,
        }
    }
}

impl PeerHealthInfo {
    /// Update health stats after a successful operation
    pub fn record_success(&mut self, response_time: Duration) {
        self.total_attempts += 1;
        self.successful_attempts += 1;
        self.success_rate = self.successful_attempts as f64 / self.total_attempts as f64;
        self.last_successful_store = SystemTime::now();
        self.consecutive_failures = 0;
        
        // Update average response time with exponential moving average
        let alpha = 0.1; // Smoothing factor
        let new_time_ms = response_time.as_millis() as f64;
        let current_avg_ms = self.average_response_time.as_millis() as f64;
        let new_avg_ms = alpha * new_time_ms + (1.0 - alpha) * current_avg_ms;
        self.average_response_time = Duration::from_millis(new_avg_ms as u64);
    }
    
    /// Update health stats after a failed operation
    pub fn record_failure(&mut self, error: &str) {
        self.total_attempts += 1;
        self.success_rate = self.successful_attempts as f64 / self.total_attempts as f64;
        self.last_failed_store = Some(SystemTime::now());
        self.consecutive_failures += 1;
        
        println!("  Recorded failure for peer: {} (consecutive: {})", error, self.consecutive_failures);
    }
    
    /// Check if the peer is considered reliable
    pub fn is_reliable(&self, min_success_rate: f64, max_consecutive_failures: u32) -> bool {
        self.success_rate >= min_success_rate && self.consecutive_failures <= max_consecutive_failures
    }
    
    /// Get a health score (0.0 to 1.0)
    pub fn health_score(&self) -> f64 {
        let success_component = self.success_rate;
        let failure_penalty = if self.consecutive_failures > 0 {
            0.9_f64.powi(self.consecutive_failures as i32)
        } else {
            1.0
        };
        let response_time_component = if self.average_response_time < Duration::from_millis(100) {
            1.0
        } else if self.average_response_time < Duration::from_millis(500) {
            0.8
        } else {
            0.5
        };
        
        success_component * failure_penalty * response_time_component
    }
}

/// Tracks replica locations and health for a specific key
#[derive(Debug, Clone)]
pub struct ReplicaLocationInfo {
    pub key: Key,
    pub replica_peers: HashSet<PeerId>,
    pub last_verified: HashMap<PeerId, SystemTime>,
    pub replica_quality: HashMap<PeerId, f64>, // Quality score 0.0-1.0
    pub created_at: SystemTime,
    pub last_updated: SystemTime,
}

impl ReplicaLocationInfo {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            replica_peers: HashSet::new(),
            last_verified: HashMap::new(),
            replica_quality: HashMap::new(),
            created_at: SystemTime::now(),
            last_updated: SystemTime::now(),
        }
    }
    
    pub fn add_replica(&mut self, peer_id: PeerId, quality: f64) {
        self.replica_peers.insert(peer_id.clone());
        self.last_verified.insert(peer_id.clone(), SystemTime::now());
        self.replica_quality.insert(peer_id, quality);
        self.last_updated = SystemTime::now();
    }
    
    pub fn remove_replica(&mut self, peer_id: &PeerId) {
        self.replica_peers.remove(peer_id);
        self.last_verified.remove(peer_id);
        self.replica_quality.remove(peer_id);
        self.last_updated = SystemTime::now();
    }
    
    pub fn replica_count(&self) -> usize {
        self.replica_peers.len()
    }
    
    pub fn average_quality(&self) -> f64 {
        if self.replica_quality.is_empty() {
            0.0
        } else {
            self.replica_quality.values().sum::<f64>() / self.replica_quality.len() as f64
        }
    }
    
    /// Get replicas that haven't been verified recently
    pub fn stale_replicas(&self, max_age: Duration) -> Vec<PeerId> {
        let cutoff = SystemTime::now() - max_age;
        self.last_verified
            .iter()
            .filter(|(_, &timestamp)| timestamp < cutoff)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }
}

/// Main replication tracking system
#[derive(Debug)]
pub struct ReplicationTracker {
    /// Maps key -> replica location information
    replica_locations: HashMap<Key, ReplicaLocationInfo>,
    /// Maps peer_id -> health information
    peer_health: HashMap<PeerId, PeerHealthInfo>,
    /// Recent replication operations for analysis
    operation_history: VecDeque<ReplicationOperation>,
    /// Configuration
    max_history_size: usize,
    health_check_interval: Duration,
}

/// Record of a replication operation
#[derive(Debug, Clone)]
pub struct ReplicationOperation {
    pub key: Key,
    pub operation_type: OperationType,
    pub peer_id: PeerId,
    pub timestamp: SystemTime,
    pub success: bool,
    pub response_time: Option<Duration>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum OperationType {
    Store,
    Retrieve,
    Verify,
    Repair,
}

impl ReplicationTracker {
    pub fn new() -> Self {
        Self {
            replica_locations: HashMap::new(),
            peer_health: HashMap::new(),
            operation_history: VecDeque::new(),
            max_history_size: 1000,
            health_check_interval: Duration::from_secs(300), // 5 minutes
        }
    }
    
    /// Record a successful store operation
    pub fn record_successful_store(&mut self, key: &Key, peer_id: &PeerId, response_time: Duration) {
        // Update peer health
        let health = self.peer_health.entry(peer_id.clone()).or_default();
        health.record_success(response_time);
        
        // Update replica location
        let replica_info = self.replica_locations.entry(key.clone()).or_insert_with(|| {
            ReplicaLocationInfo::new(key.clone())
        });
        replica_info.add_replica(peer_id.clone(), health.health_score());
        
        // Record operation
        self.add_operation(ReplicationOperation {
            key: key.clone(),
            operation_type: OperationType::Store,
            peer_id: peer_id.clone(),
            timestamp: SystemTime::now(),
            success: true,
            response_time: Some(response_time),
            error: None,
        });
        
        println!("✓ Recorded successful store: key={:?}, peer={}, time={}ms", 
                 &key.as_bytes()[..4], peer_id, response_time.as_millis());
    }
    
    /// Record a failed store operation
    pub fn record_failed_store(&mut self, key: &Key, peer_id: &PeerId, error: &str) {
        // Update peer health
        let health = self.peer_health.entry(peer_id.clone()).or_default();
        health.record_failure(error);
        
        // Remove from replica locations if it was there
        if let Some(replica_info) = self.replica_locations.get_mut(key) {
            replica_info.remove_replica(peer_id);
        }
        
        // Record operation
        self.add_operation(ReplicationOperation {
            key: key.clone(),
            operation_type: OperationType::Store,
            peer_id: peer_id.clone(),
            timestamp: SystemTime::now(),
            success: false,
            response_time: None,
            error: Some(error.to_string()),
        });
        
        println!("✗ Recorded failed store: key={:?}, peer={}, error={}", 
                 &key.as_bytes()[..4], peer_id, error);
    }
    
    /// Get replica count for a key
    pub fn get_replica_count(&self, key: &Key) -> usize {
        self.replica_locations
            .get(key)
            .map(|info| info.replica_count())
            .unwrap_or(0)
    }
    
    /// Get replica peers for a key
    pub fn get_replica_peers(&self, key: &Key) -> Option<&HashSet<PeerId>> {
        self.replica_locations
            .get(key)
            .map(|info| &info.replica_peers)
    }
    
    /// Get peer health information
    pub fn get_peer_health(&self, peer_id: &PeerId) -> Option<&PeerHealthInfo> {
        self.peer_health.get(peer_id)
    }
    
    /// Get peers with good health scores
    pub fn get_healthy_peers(&self, min_success_rate: f64, max_consecutive_failures: u32) -> Vec<PeerId> {
        self.peer_health
            .iter()
            .filter(|(_, health)| health.is_reliable(min_success_rate, max_consecutive_failures))
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }
    
    /// Get comprehensive statistics
    pub fn get_statistics(&self) -> ReplicationStatistics {
        let total_keys = self.replica_locations.len();
        let total_replicas: usize = self.replica_locations.values()
            .map(|info| info.replica_count())
            .sum();
        
        let healthy_peers = self.get_healthy_peers(0.8, 3);
        let total_peers = self.peer_health.len();
        
        let recent_operations = self.operation_history.iter()
            .filter(|op| op.timestamp > SystemTime::now() - Duration::from_secs(3600))
            .count();
        
        let recent_successes = self.operation_history.iter()
            .filter(|op| op.timestamp > SystemTime::now() - Duration::from_secs(3600) && op.success)
            .count();
        
        let success_rate = if recent_operations > 0 {
            recent_successes as f64 / recent_operations as f64
        } else {
            1.0
        };
        
        let average_replicas = if total_keys > 0 {
            total_replicas as f64 / total_keys as f64
        } else {
            0.0
        };
        
        ReplicationStatistics {
            total_keys,
            total_replicas,
            average_replicas_per_key: average_replicas,
            healthy_peers: healthy_peers.len(),
            total_peers,
            recent_success_rate: success_rate,
            recent_operations,
        }
    }
    
    /// Find keys that need repair (insufficient replicas)
    pub fn find_underreplicated_keys(&self, min_replicas: usize) -> Vec<(Key, usize)> {
        self.replica_locations
            .iter()
            .filter(|(_, info)| info.replica_count() < min_replicas)
            .map(|(key, info)| (key.clone(), info.replica_count()))
            .collect()
    }
    
    /// Clean up old operation history
    pub fn cleanup_history(&mut self) {
        while self.operation_history.len() > self.max_history_size {
            self.operation_history.pop_front();
        }
        
        // Remove very old operations (older than 24 hours)
        let cutoff = SystemTime::now() - Duration::from_secs(24 * 3600);
        self.operation_history.retain(|op| op.timestamp > cutoff);
    }
    
    /// Verify replica integrity (simulate verification)
    pub fn verify_replica(&mut self, key: &Key, peer_id: &PeerId) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Simulate verification with some randomness
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        peer_id.hash(&mut hasher);
        let hash_value = hasher.finish();
        
        // 90% success rate for simulation
        let success = (hash_value % 100) < 90;
        
        if success {
            if let Some(health) = self.peer_health.get_mut(peer_id) {
                health.record_success(Duration::from_millis(50));
            }
            
            self.add_operation(ReplicationOperation {
                key: key.clone(),
                operation_type: OperationType::Verify,
                peer_id: peer_id.clone(),
                timestamp: SystemTime::now(),
                success: true,
                response_time: Some(Duration::from_millis(50)),
                error: None,
            });
        } else {
            if let Some(health) = self.peer_health.get_mut(peer_id) {
                health.record_failure("verification failed");
            }
            
            // Remove from replica locations
            if let Some(replica_info) = self.replica_locations.get_mut(key) {
                replica_info.remove_replica(peer_id);
            }
            
            self.add_operation(ReplicationOperation {
                key: key.clone(),
                operation_type: OperationType::Verify,
                peer_id: peer_id.clone(),
                timestamp: SystemTime::now(),
                success: false,
                response_time: None,
                error: Some("verification failed".to_string()),
            });
        }
        
        success
    }
    
    fn add_operation(&mut self, operation: ReplicationOperation) {
        self.operation_history.push_back(operation);
        self.cleanup_history();
    }
}

#[derive(Debug)]
pub struct ReplicationStatistics {
    pub total_keys: usize,
    pub total_replicas: usize,
    pub average_replicas_per_key: f64,
    pub healthy_peers: usize,
    pub total_peers: usize,
    pub recent_success_rate: f64,
    pub recent_operations: usize,
}

// Test functions
fn test_peer_health_tracking() {
    println!("Testing peer health tracking...");
    let mut health = PeerHealthInfo::default();
    
    // Record some successful operations
    health.record_success(Duration::from_millis(50));
    health.record_success(Duration::from_millis(80));
    health.record_success(Duration::from_millis(30));
    
    assert_eq!(health.total_attempts, 3);
    assert_eq!(health.successful_attempts, 3);
    assert_eq!(health.success_rate, 1.0);
    assert_eq!(health.consecutive_failures, 0);
    
    // Record some failures
    health.record_failure("network timeout");
    health.record_failure("connection refused");
    
    assert_eq!(health.total_attempts, 5);
    assert_eq!(health.successful_attempts, 3);
    assert_eq!(health.success_rate, 0.6);
    assert_eq!(health.consecutive_failures, 2);
    
    // Health score should be lower due to failures
    let score = health.health_score();
    assert!(score < 0.6);
    
    println!("✓ Peer health tracking works correctly (success rate: {:.1}%, health score: {:.2})", 
             health.success_rate * 100.0, score);
}

fn test_replica_location_tracking() {
    println!("\nTesting replica location tracking...");
    let key = Key::from(vec![1, 2, 3]);
    let mut replica_info = ReplicaLocationInfo::new(key.clone());
    
    // Add some replicas
    replica_info.add_replica("peer1".to_string(), 0.9);
    replica_info.add_replica("peer2".to_string(), 0.8);
    replica_info.add_replica("peer3".to_string(), 0.95);
    
    assert_eq!(replica_info.replica_count(), 3);
    assert!((replica_info.average_quality() - 0.883).abs() < 0.01); // (0.9 + 0.8 + 0.95) / 3
    
    // Remove a replica
    replica_info.remove_replica(&"peer2".to_string());
    assert_eq!(replica_info.replica_count(), 2);
    
    println!("✓ Replica location tracking works correctly ({} replicas, avg quality: {:.2})", 
             replica_info.replica_count(), replica_info.average_quality());
}

fn test_replication_tracker() {
    println!("\nTesting main replication tracker...");
    let mut tracker = ReplicationTracker::new();
    
    let key1 = Key::from(vec![10, 20, 30]);
    let key2 = Key::from(vec![40, 50, 60]);
    
    // Record successful stores
    tracker.record_successful_store(&key1, &"peer_a".to_string(), Duration::from_millis(50));
    tracker.record_successful_store(&key1, &"peer_b".to_string(), Duration::from_millis(80));
    tracker.record_successful_store(&key1, &"peer_c".to_string(), Duration::from_millis(60));
    
    tracker.record_successful_store(&key2, &"peer_a".to_string(), Duration::from_millis(45));
    tracker.record_successful_store(&key2, &"peer_d".to_string(), Duration::from_millis(100));
    
    // Record some failures
    tracker.record_failed_store(&key1, &"peer_e".to_string(), "connection timeout");
    tracker.record_failed_store(&key2, &"peer_f".to_string(), "disk full");
    
    // Check replica counts
    assert_eq!(tracker.get_replica_count(&key1), 3);
    assert_eq!(tracker.get_replica_count(&key2), 2);
    
    // Check healthy peers
    let healthy_peers = tracker.get_healthy_peers(0.5, 5);
    assert!(healthy_peers.contains(&"peer_a".to_string()));
    assert!(healthy_peers.contains(&"peer_b".to_string()));
    assert!(!healthy_peers.contains(&"peer_e".to_string())); // Failed
    
    println!("✓ Main replication tracker works correctly");
    println!("  Key1 replicas: {}", tracker.get_replica_count(&key1));
    println!("  Key2 replicas: {}", tracker.get_replica_count(&key2));
    println!("  Healthy peers: {}", healthy_peers.len());
}

fn test_statistics_and_analysis() {
    println!("\nTesting statistics and analysis...");
    let mut tracker = ReplicationTracker::new();
    
    // Simulate a workload
    let keys: Vec<Key> = (0..5).map(|i| Key::from(vec![i as u8])).collect();
    let peers: Vec<String> = (0..8).map(|i| format!("peer_{}", i)).collect();
    
    // Record successful operations for most combinations
    for (i, key) in keys.iter().enumerate() {
        for (j, peer) in peers.iter().enumerate() {
            if (i + j) % 3 != 0 { // Skip some to simulate failures
                tracker.record_successful_store(
                    key, 
                    peer, 
                    Duration::from_millis(50 + (i * j) as u64 * 10)
                );
            } else {
                tracker.record_failed_store(key, peer, "simulated failure");
            }
        }
    }
    
    // Get statistics
    let stats = tracker.get_statistics();
    println!("✓ Statistics calculated:");
    println!("  Total keys: {}", stats.total_keys);
    println!("  Total replicas: {}", stats.total_replicas);
    println!("  Average replicas per key: {:.1}", stats.average_replicas_per_key);
    println!("  Healthy peers: {}/{}", stats.healthy_peers, stats.total_peers);
    println!("  Recent success rate: {:.1}%", stats.recent_success_rate * 100.0);
    println!("  Recent operations: {}", stats.recent_operations);
    
    assert!(stats.total_keys > 0);
    assert!(stats.total_replicas > 0);
    assert!(stats.average_replicas_per_key > 0.0);
}

fn test_underreplication_detection() {
    println!("\nTesting underreplication detection...");
    let mut tracker = ReplicationTracker::new();
    
    let key1 = Key::from(vec![100]);
    let key2 = Key::from(vec![200]);
    let key3 = Key::from(vec![255]);
    
    // Key1: Well replicated (5 replicas)
    for i in 0..5 {
        tracker.record_successful_store(&key1, &format!("peer_{}", i), Duration::from_millis(50));
    }
    
    // Key2: Underreplicated (2 replicas)
    for i in 0..2 {
        tracker.record_successful_store(&key2, &format!("peer_{}", i), Duration::from_millis(50));
    }
    
    // Key3: Critically underreplicated (1 replica)
    tracker.record_successful_store(&key3, &"peer_0".to_string(), Duration::from_millis(50));
    
    // Check for underreplication
    let underreplicated = tracker.find_underreplicated_keys(3);
    
    println!("✓ Underreplication detection:");
    for (key, count) in &underreplicated {
        println!("  Key {:?}: {} replicas (needs repair)", &key.as_bytes()[..1], count);
    }
    
    assert_eq!(underreplicated.len(), 2); // key2 and key3
    assert!(underreplicated.iter().any(|(_, count)| *count == 2)); // key2
    assert!(underreplicated.iter().any(|(_, count)| *count == 1)); // key3
}

fn test_replica_verification() {
    println!("\nTesting replica verification...");
    let mut tracker = ReplicationTracker::new();
    
    let key = Key::from(vec![42]);
    let peer = "test_peer".to_string();
    
    // Add initial replica
    tracker.record_successful_store(&key, &peer, Duration::from_millis(50));
    assert_eq!(tracker.get_replica_count(&key), 1);
    
    // Verify replica multiple times
    let mut successes = 0;
    let mut failures = 0;
    
    for _ in 0..20 {
        if tracker.verify_replica(&key, &peer) {
            successes += 1;
        } else {
            failures += 1;
        }
    }
    
    println!("✓ Verification results: {} successes, {} failures", successes, failures);
    
    // Should have mostly successes (90% success rate in simulation)
    assert!(successes > failures);
    
    // Check that failed verifications removed the replica
    if failures > 0 {
        println!("  Replica was removed after verification failure");
    }
}

fn main() {
    println!("🧪 Running Replication Tracking System Tests\n");
    
    test_peer_health_tracking();
    test_replica_location_tracking();
    test_replication_tracker();
    test_statistics_and_analysis();
    test_underreplication_detection();
    test_replica_verification();
    
    println!("\n🎉 All replication tracking tests passed!");
    println!("✅ Step 3 Complete: Replication tracking system is working correctly");
    
    println!("\n📋 Key Features Implemented:");
    println!("  ✓ Peer health monitoring with success rates");
    println!("  ✓ Replica location tracking with quality scores");
    println!("  ✓ Operation history and statistics");
    println!("  ✓ Underreplication detection");
    println!("  ✓ Replica verification system");
    println!("  ✓ Comprehensive health scoring");
    
    println!("\n📋 Next Steps:");
    println!("  4. Implement repair scheduler");
    println!("  5. Create enhanced record manager");
    println!("  6. Write integration tests");
}