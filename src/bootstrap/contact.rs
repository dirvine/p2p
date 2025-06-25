//! Contact Entry and Quality Scoring
//!
//! Manages peer contact information with comprehensive quality metrics for
//! intelligent bootstrap peer selection.

use crate::PeerId;
use std::time::Duration;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A contact entry representing a known peer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactEntry {
    /// Unique identifier for this peer
    pub peer_id: PeerId,
    /// List of network addresses where this peer can be reached
    pub addresses: Vec<String>,
    /// Timestamp when this peer was last seen online
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Quality metrics for connection performance evaluation
    pub quality_metrics: QualityMetrics,
    /// List of capabilities supported by this peer
    pub capabilities: Vec<String>,
    /// Whether this peer's IPv6 identity has been verified
    pub ipv6_identity_verified: bool,
    /// Overall reputation score (0.0 to 1.0)
    pub reputation_score: f64,
    /// Historical connection data for this peer
    pub connection_history: ConnectionHistory,
}

/// Quality metrics for peer evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualityMetrics {
    /// Connection success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Average connection latency in milliseconds
    pub avg_latency_ms: f64,
    /// Computed overall quality score (0.0 to 1.0)
    pub quality_score: f64,
    /// Timestamp of the last connection attempt
    pub last_connection_attempt: chrono::DateTime<chrono::Utc>,
    /// Timestamp of the last successful connection
    pub last_successful_connection: chrono::DateTime<chrono::Utc>,
    /// Estimated uptime reliability score (0.0 to 1.0)
    pub uptime_score: f64,
}

/// Connection history tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionHistory {
    /// Total number of connection attempts made
    pub total_attempts: u64,
    /// Number of successful connections established
    pub successful_connections: u64,
    /// Number of failed connection attempts
    pub failed_connections: u64,
    /// Total time spent in successful sessions
    pub total_session_time: Duration,
    /// Last 10 latency measurements in milliseconds
    pub recent_latencies: Vec<u64>,
    /// Failure reasons and their occurrence counts
    pub connection_failures: HashMap<String, u64>,
}

impl ContactEntry {
    /// Create a new contact entry
    pub fn new(peer_id: PeerId, addresses: Vec<String>) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            peer_id,
            addresses,
            last_seen: now,
            quality_metrics: QualityMetrics::new(),
            capabilities: Vec::new(),
            ipv6_identity_verified: false,
            reputation_score: 0.5, // Neutral starting score
            connection_history: ConnectionHistory::new(),
        }
    }
    
    /// Update quality metrics based on connection result
    pub fn update_connection_result(&mut self, success: bool, latency_ms: Option<u64>, error: Option<String>) {
        let now = chrono::Utc::now();
        self.last_seen = now;
        self.quality_metrics.last_connection_attempt = now;
        self.connection_history.total_attempts += 1;
        
        if success {
            self.connection_history.successful_connections += 1;
            self.quality_metrics.last_successful_connection = now;
            
            if let Some(latency) = latency_ms {
                self.add_latency_measurement(latency);
            }
        } else {
            self.connection_history.failed_connections += 1;
            
            if let Some(err) = error {
                *self.connection_history.connection_failures.entry(err).or_insert(0) += 1;
            }
        }
        
        self.update_success_rate();
        self.update_latency_average();
        self.recalculate_quality_score();
    }
    
    /// Add a latency measurement
    fn add_latency_measurement(&mut self, latency_ms: u64) {
        self.connection_history.recent_latencies.push(latency_ms);
        
        // Keep only last 10 measurements
        if self.connection_history.recent_latencies.len() > 10 {
            self.connection_history.recent_latencies.remove(0);
        }
    }
    
    /// Update success rate
    pub fn update_success_rate(&mut self) {
        if self.connection_history.total_attempts > 0 {
            self.quality_metrics.success_rate = 
                self.connection_history.successful_connections as f64 / 
                self.connection_history.total_attempts as f64;
        }
    }
    
    /// Update average latency
    fn update_latency_average(&mut self) {
        if !self.connection_history.recent_latencies.is_empty() {
            let sum: u64 = self.connection_history.recent_latencies.iter().sum();
            self.quality_metrics.avg_latency_ms = sum as f64 / self.connection_history.recent_latencies.len() as f64;
        }
    }
    
    /// Recalculate overall quality score
    pub fn recalculate_quality_score(&mut self) {
        let quality_calculator = QualityCalculator::new();
        self.quality_metrics.quality_score = quality_calculator.calculate_quality(self);
    }
    
    /// Update capabilities
    pub fn update_capabilities(&mut self, capabilities: Vec<String>) {
        self.capabilities = capabilities;
        self.recalculate_quality_score();
    }
    
    /// Update reputation score
    pub fn update_reputation(&mut self, reputation: f64) {
        self.reputation_score = reputation.clamp(0.0, 1.0);
        self.recalculate_quality_score();
    }
    
    /// Mark IPv6 identity as verified
    pub fn mark_ipv6_verified(&mut self) {
        self.ipv6_identity_verified = true;
        self.recalculate_quality_score();
    }
    
    /// Check if contact is considered stale
    pub fn is_stale(&self, max_age: Duration) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.last_seen);
        age.to_std().unwrap_or(Duration::MAX) > max_age
    }
    
    /// Get contact age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.last_seen);
        age.num_seconds().max(0) as u64
    }
    
    /// Check if contact has essential capabilities
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }
    
    /// Get a summary string for debugging
    pub fn summary(&self) -> String {
        format!(
            "Peer {} (Quality: {:.2}, Success: {:.1}%, Latency: {:.0}ms, Verified: {})",
            self.peer_id.to_string().chars().take(8).collect::<String>(),
            self.quality_metrics.quality_score,
            self.quality_metrics.success_rate * 100.0,
            self.quality_metrics.avg_latency_ms,
            self.ipv6_identity_verified
        )
    }
}

impl QualityMetrics {
    /// Create new quality metrics with default values
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        
        Self {
            success_rate: 0.0,
            avg_latency_ms: 0.0,
            quality_score: 0.0,
            last_connection_attempt: now,
            last_successful_connection: now,
            uptime_score: 0.5, // Neutral starting score
        }
    }
    
    /// Apply age decay to quality metrics
    pub fn apply_age_decay(&mut self, decay_factor: f64) {
        // Decay quality score over time to favor recent connections
        self.quality_score *= decay_factor;
        self.uptime_score *= decay_factor;
    }
}

impl ConnectionHistory {
    /// Create new connection history
    pub fn new() -> Self {
        Self {
            total_attempts: 0,
            successful_connections: 0,
            failed_connections: 0,
            total_session_time: Duration::from_secs(0),
            recent_latencies: Vec::new(),
            connection_failures: HashMap::new(),
        }
    }
    
    /// Add session time
    pub fn add_session_time(&mut self, duration: Duration) {
        self.total_session_time = self.total_session_time.saturating_add(duration);
    }
    
    /// Get failure rate for specific error type
    pub fn get_failure_rate(&self, error_type: &str) -> f64 {
        let failures = self.connection_failures.get(error_type).copied().unwrap_or(0);
        if self.total_attempts > 0 {
            failures as f64 / self.total_attempts as f64
        } else {
            0.0
        }
    }
}

/// Quality calculator for computing peer scores
pub struct QualityCalculator {
    success_weight: f64,
    latency_weight: f64,
    recency_weight: f64,
    reputation_weight: f64,
    verification_bonus: f64,
    capability_bonus: f64,
}

impl QualityCalculator {
    /// Create new quality calculator with default weights
    pub fn new() -> Self {
        Self {
            success_weight: 0.40,   // 40% - Connection success rate
            latency_weight: 0.30,   // 30% - Network performance
            recency_weight: 0.20,   // 20% - How recently seen
            reputation_weight: 0.10, // 10% - Reputation score
            verification_bonus: 0.05, // 5% bonus for verified identity
            capability_bonus: 0.02,  // 2% bonus per important capability
        }
    }
    
    /// Calculate overall quality score for a contact
    pub fn calculate_quality(&self, contact: &ContactEntry) -> f64 {
        let mut score = 0.0;
        
        // Success rate component (0.0 to 1.0)
        let success_component = contact.quality_metrics.success_rate * self.success_weight;
        score += success_component;
        
        // Latency component (inverse of latency, normalized)
        let latency_component = if contact.quality_metrics.avg_latency_ms > 0.0 {
            let normalized_latency = (1000.0 / (contact.quality_metrics.avg_latency_ms + 100.0)).min(1.0);
            normalized_latency * self.latency_weight
        } else {
            0.0
        };
        score += latency_component;
        
        // Recency component (exponential decay)
        let age_seconds = contact.age_seconds() as f64;
        let recency_component = (-age_seconds / 86400.0).exp() * self.recency_weight; // 24 hour half-life
        score += recency_component;
        
        // Reputation component
        let reputation_component = contact.reputation_score * self.reputation_weight;
        score += reputation_component;
        
        // IPv6 verification bonus
        if contact.ipv6_identity_verified {
            score += self.verification_bonus;
        }
        
        // Capability bonuses
        let important_capabilities = ["dht", "mcp", "relay"];
        let capability_count = important_capabilities.iter()
            .filter(|&cap| contact.has_capability(cap))
            .count();
        score += capability_count as f64 * self.capability_bonus;
        
        // Clamp to valid range
        score.clamp(0.0, 1.0)
    }
    
    /// Calculate quality with custom weights
    pub fn calculate_with_weights(
        &self,
        contact: &ContactEntry,
        success_weight: f64,
        latency_weight: f64,
        recency_weight: f64,
        reputation_weight: f64,
    ) -> f64 {
        let mut calculator = self.clone();
        calculator.success_weight = success_weight;
        calculator.latency_weight = latency_weight;
        calculator.recency_weight = recency_weight;
        calculator.reputation_weight = reputation_weight;
        
        calculator.calculate_quality(contact)
    }
}

impl Clone for QualityCalculator {
    fn clone(&self) -> Self {
        Self {
            success_weight: self.success_weight,
            latency_weight: self.latency_weight,
            recency_weight: self.recency_weight,
            reputation_weight: self.reputation_weight,
            verification_bonus: self.verification_bonus,
            capability_bonus: self.capability_bonus,
        }
    }
}

impl Default for QualityCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contact_entry_creation() {
        let peer_id = PeerId::from("test-peer");
        let addresses = vec!["/ip4/127.0.0.1/tcp/9000".to_string()];
        
        let contact = ContactEntry::new(peer_id.clone(), addresses.clone());
        
        assert_eq!(contact.peer_id, peer_id);
        assert_eq!(contact.addresses, addresses);
        assert_eq!(contact.quality_metrics.success_rate, 0.0);
        assert!(!contact.ipv6_identity_verified);
    }
    
    #[test]
    fn test_quality_calculation() {
        let mut contact = ContactEntry::new(
            PeerId::from("test-peer"),
            vec!["/ip4/127.0.0.1/tcp/9000".to_string()]
        );
        
        // Simulate successful connections
        contact.update_connection_result(true, Some(50), None);
        contact.update_connection_result(true, Some(60), None);
        contact.update_connection_result(false, None, Some("timeout".to_string()));
        
        assert!(contact.quality_metrics.success_rate > 0.5);
        assert!(contact.quality_metrics.avg_latency_ms > 0.0);
        assert!(contact.quality_metrics.quality_score > 0.0);
    }
    
    #[test]
    fn test_capability_bonus() {
        let mut contact = ContactEntry::new(
            PeerId::from("test-peer"),
            vec!["/ip4/127.0.0.1/tcp/9000".to_string()]
        );
        
        let initial_score = contact.quality_metrics.quality_score;
        
        contact.update_capabilities(vec!["dht".to_string(), "mcp".to_string()]);
        
        assert!(contact.quality_metrics.quality_score > initial_score);
    }
    
    #[test]
    fn test_stale_detection() {
        let mut contact = ContactEntry::new(
            PeerId::from("test-peer"),
            vec!["/ip4/127.0.0.1/tcp/9000".to_string()]
        );
        
        // Set last seen to 2 hours ago
        contact.last_seen = chrono::Utc::now() - chrono::Duration::hours(2);
        
        assert!(contact.is_stale(Duration::from_secs(3600))); // 1 hour threshold
        assert!(!contact.is_stale(Duration::from_secs(10800))); // 3 hour threshold
    }
    
    #[test]
    fn test_quality_decay() {
        let mut metrics = QualityMetrics::new();
        metrics.quality_score = 0.8;
        metrics.uptime_score = 0.9;
        
        metrics.apply_age_decay(0.9);
        
        assert!(metrics.quality_score < 0.8);
        assert!(metrics.uptime_score < 0.9);
    }
}