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

//! Adaptive P2P Network Implementation
//! 
//! This module implements the adaptive P2P network architecture described in the
//! network documentation, combining multiple distributed systems technologies:
//! - Secure Kademlia (S/Kademlia) as the foundational DHT layer
//! - Hyperbolic geometry routing for efficient greedy routing
//! - Self-Organizing Maps (SOM) for content and capability clustering
//! - EigenTrust++ for decentralized reputation management
//! - Adaptive GossipSub for scalable message propagation
//! - Machine learning systems for routing optimization, caching, and churn prediction

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod identity;
pub mod transport;
pub mod dht_integration;
pub mod routing;
pub mod hyperbolic;
pub mod som;
pub mod trust;
pub mod gossip;
pub mod learning;
pub mod storage;
pub mod replication;
pub mod retrieval;
pub mod churn;
pub mod monitoring;
pub mod client;
pub mod security;
pub mod performance;

// Re-export commonly used types
pub use identity::{NodeIdentity, SignedMessage, ProofOfWork, StoredIdentity};
pub use transport::{Transport, TransportManager, TransportProtocol, ConnectionInfo};
pub use dht_integration::{AdaptiveDHT, KademliaRoutingStrategy};
pub use routing::AdaptiveRouter;
pub use hyperbolic::{HyperbolicSpace, HyperbolicRoutingStrategy};
pub use som::{SelfOrganizingMap, SOMRoutingStrategy, FeatureExtractor};
pub use trust::{EigenTrustEngine, TrustBasedRoutingStrategy, NodeStatistics, NodeStatisticsUpdate, MockTrustProvider};
pub use gossip::AdaptiveGossipSub;
pub use learning::{ThompsonSampling, QLearnCacheManager, ChurnPredictor};
pub use storage::{ContentStore, StorageConfig, ChunkManager, ReplicationConfig};
pub use replication::{ReplicationManager, ReplicationStrategy, ReplicaInfo};
pub use retrieval::{RetrievalManager, RetrievalStrategy};
pub use churn::{ChurnHandler, ChurnConfig, NodeMonitor, RecoveryManager, NodeState};
pub use monitoring::{MonitoringSystem, MonitoringConfig, NetworkHealth, DashboardData, AlertManager, Alert};
pub use client::{Client, ClientConfig, ClientProfile, AdaptiveP2PClient, NetworkStats as ClientNetworkStats};
pub use security::{SecurityManager, SecurityConfig, RateLimiter, BlacklistManager, EclipseDetector, SecurityAuditor};
pub use performance::{PerformanceConfig, OptimizedSerializer, ConnectionPool, PerformanceCache, BatchProcessor, ConcurrencyLimiter};

/// Result type for adaptive network operations
pub type Result<T> = std::result::Result<T, AdaptiveNetworkError>;

/// Core error type for the adaptive network
#[derive(Debug, thiserror::Error)]
pub enum AdaptiveNetworkError {
    #[error("Routing error: {0}")]
    Routing(String),
    
    #[error("Trust calculation error: {0}")]
    Trust(String),
    
    #[error("Learning system error: {0}")]
    Learning(String),
    
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Content hash type used throughout the network
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);


/// Network message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// Message ID
    pub id: String,
    /// Sender node ID
    pub sender: NodeId,
    /// Message content
    pub content: Vec<u8>,
    /// Message type
    pub msg_type: ContentType,
    /// Timestamp (Unix timestamp in seconds)
    pub timestamp: u64,
}

/// Node ID type alias
pub type NodeId = crate::peer_record::UserId;

/// Node descriptor containing all information about a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDescriptor {
    pub id: NodeId,
    pub public_key: ed25519_dalek::VerifyingKey,
    pub addresses: Vec<String>,
    pub hyperbolic: Option<HyperbolicCoordinate>,
    pub som_position: Option<[f64; 4]>,
    pub trust: f64,
    pub capabilities: NodeCapabilities,
}

/// Hyperbolic coordinate in Poincaré disk model
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HyperbolicCoordinate {
    pub r: f64,     // Radial coordinate [0, 1)
    pub theta: f64, // Angular coordinate [0, 2π)
}

/// Node capabilities for resource discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub storage: u64,    // GB available
    pub compute: u64,    // Benchmark score
    pub bandwidth: u64,  // Mbps available
}

/// Core trait for adaptive P2P network nodes
#[async_trait]
pub trait AdaptiveNetworkNode: Send + Sync {
    /// Join the network using bootstrap nodes
    async fn join(&mut self, bootstrap: Vec<NodeDescriptor>) -> Result<()>;
    
    /// Store data with adaptive replication
    async fn store(&self, data: Vec<u8>) -> Result<ContentHash>;
    
    /// Retrieve data using parallel strategies
    async fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>>;
    
    /// Publish a message to a gossip topic
    async fn publish(&self, topic: &str, message: Vec<u8>) -> Result<()>;
    
    /// Subscribe to a gossip topic
    async fn subscribe(&self, topic: &str) -> Result<Box<dyn futures::Stream<Item = Vec<u8>> + Send>>;
    
    /// Get current node information
    async fn node_info(&self) -> Result<NodeDescriptor>;
    
    /// Get network statistics
    async fn network_stats(&self) -> Result<NetworkStats>;
    
    /// Gracefully shutdown the node
    async fn shutdown(self) -> Result<()>;
}

/// Network statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub connected_peers: usize,
    pub routing_success_rate: f64,
    pub average_trust_score: f64,
    pub cache_hit_rate: f64,
    pub churn_rate: f64,
    pub total_storage: u64,
    pub total_bandwidth: u64,
}

/// Routing strategy trait for different routing algorithms
#[async_trait]
pub trait RoutingStrategy: Send + Sync {
    /// Find a path to the target node
    async fn find_path(&self, target: &NodeId) -> Result<Vec<NodeId>>;
    
    /// Calculate routing score for a neighbor towards a target
    fn route_score(&self, neighbor: &NodeId, target: &NodeId) -> f64;
    
    /// Update routing metrics based on success/failure
    fn update_metrics(&mut self, path: &[NodeId], success: bool);
    
    /// Find closest nodes to a content hash
    async fn find_closest_nodes(&self, content_hash: &ContentHash, _count: usize) -> Result<Vec<NodeId>> {
        // Default implementation uses node ID from content hash
        let target = NodeId { hash: content_hash.0 };
        self.find_path(&target).await
    }
}

/// Trust provider trait for reputation queries
pub trait TrustProvider: Send + Sync {
    /// Get trust score for a node
    fn get_trust(&self, node: &NodeId) -> f64;
    
    /// Update trust based on interaction
    fn update_trust(&self, from: &NodeId, to: &NodeId, success: bool);
    
    /// Get global trust vector
    fn get_global_trust(&self) -> std::collections::HashMap<NodeId, f64>;
    
    /// Remove a node from the trust system
    fn remove_node(&self, node: &NodeId);
    
    /// Get trust score (alias for get_trust)
    fn get_trust_score(&self, node: &NodeId) -> f64 {
        self.get_trust(node)
    }
}

/// Learning system trait for adaptive behavior
#[async_trait]
pub trait LearningSystem: Send + Sync {
    /// Select optimal strategy based on context
    async fn select_strategy(&self, context: &LearningContext) -> StrategyChoice;
    
    /// Update learning model with outcome
    async fn update(&mut self, context: &LearningContext, choice: &StrategyChoice, outcome: &Outcome);
    
    /// Get current model performance metrics
    async fn metrics(&self) -> LearningMetrics;
}

/// Context for learning decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningContext {
    pub content_type: ContentType,
    pub network_conditions: NetworkConditions,
    pub historical_performance: Vec<f64>,
}

/// Content type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    DHTLookup,
    DataRetrieval,
    ComputeRequest,
    RealtimeMessage,
}

/// Current network conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConditions {
    pub connected_peers: usize,
    pub avg_latency_ms: f64,
    pub churn_rate: f64,
}

/// Strategy choice made by learning system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StrategyChoice {
    Kademlia,
    Hyperbolic,
    TrustPath,
    SOMRegion,
}

/// Outcome of a strategy choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub latency_ms: u64,
    pub hops: usize,
}

/// Learning system performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub total_decisions: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub strategy_performance: std::collections::HashMap<StrategyChoice, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_hash_serialization() {
        let hash = ContentHash([42u8; 32]);
        let serialized = bincode::serialize(&hash).unwrap();
        let deserialized: ContentHash = bincode::deserialize(&serialized).unwrap();
        assert_eq!(hash, deserialized);
    }
    
    #[test]
    fn test_hyperbolic_coordinate_bounds() {
        let coord = HyperbolicCoordinate { r: 0.5, theta: std::f64::consts::PI };
        assert!(coord.r >= 0.0 && coord.r < 1.0);
        assert!(coord.theta >= 0.0 && coord.theta < 2.0 * std::f64::consts::PI);
    }
}