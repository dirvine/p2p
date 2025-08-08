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

//! Adaptive replication manager for content durability
//!
//! This module manages content replication based on:
//! - Network churn rate
//! - Content popularity
//! - Node reliability
//! - Available storage capacity

use super::*;
use crate::adaptive::{
    TrustProvider,
    learning::ChurnPredictor,
    routing::AdaptiveRouter,
    storage::{ContentMetadata, ReplicationConfig},
};
use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Replication manager for adaptive content replication
pub struct ReplicationManager {
    /// Replication configuration
    config: ReplicationConfig,

    /// Trust provider for node selection
    trust_provider: Arc<dyn TrustProvider>,

    /// Churn predictor for proactive replication
    churn_predictor: Arc<ChurnPredictor>,

    /// Routing strategies for node selection
    router: Arc<AdaptiveRouter>,

    /// Content replica tracking
    replica_map: Arc<RwLock<HashMap<ContentHash, ReplicaInfo>>>,

    /// Replication statistics
    stats: Arc<RwLock<ReplicationStats>>,
}

/// Information about content replicas
#[derive(Debug, Clone)]
pub struct ReplicaInfo {
    /// Set of nodes storing this content
    pub storing_nodes: HashSet<NodeId>,

    /// Current replication factor
    pub replication_factor: u32,

    /// Target replication factor
    pub target_factor: u32,

    /// Last replication check
    pub last_check: Instant,

    /// Content metadata
    pub metadata: ContentMetadata,
}

/// Replication strategy selection
#[derive(Debug, Clone, PartialEq)]
pub enum ReplicationStrategy {
    /// Use all available strategies
    Composite,

    /// Kademlia-based replication
    Kademlia,

    /// Trust-based replication
    TrustBased,

    /// Proximity-based replication
    ProximityBased,
}

/// Replication statistics
#[derive(Debug, Default, Clone)]
pub struct ReplicationStats {
    /// Total replications performed
    pub total_replications: u64,

    /// Successful replications
    pub successful_replications: u64,

    /// Failed replications
    pub failed_replications: u64,

    /// Proactive replications (due to churn prediction)
    pub proactive_replications: u64,

    /// Average replication factor
    pub avg_replication_factor: f64,
}

impl ReplicationManager {
    /// Create a new replication manager
    pub fn new(
        config: ReplicationConfig,
        trust_provider: Arc<dyn TrustProvider>,
        churn_predictor: Arc<ChurnPredictor>,
        router: Arc<AdaptiveRouter>,
    ) -> Self {
        Self {
            config,
            trust_provider,
            churn_predictor,
            router,
            replica_map: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ReplicationStats::default())),
        }
    }

    /// Calculate adaptive replication factor based on network conditions
    pub async fn calculate_replication_factor(&self, content_hash: &ContentHash) -> u32 {
        // Get current network churn rate (would be calculated from network stats)
        let churn_rate = self.estimate_churn_rate().await;

        // Base factor
        let mut factor = self.config.base_replicas;

        // Adjust based on churn rate
        if churn_rate > self.config.churn_threshold {
            // Increase replication linearly with churn rate
            let churn_multiplier = 1.0 + (churn_rate - self.config.churn_threshold) * 2.0;
            factor = (factor as f64 * churn_multiplier) as u32;
        }

        // Check content popularity (would track access patterns)
        let popularity = self.get_content_popularity(content_hash).await;
        if popularity > 0.8 {
            factor = (factor as f64 * 1.5) as u32;
        }

        // Clamp to configured range
        factor
            .max(self.config.min_replicas)
            .min(self.config.max_replicas)
    }

    /// Select nodes for replication based on composite scoring
    pub async fn select_replication_nodes(
        &self,
        _content_hash: &ContentHash,
        count: usize,
        exclude: &HashSet<NodeId>,
    ) -> Result<Vec<NodeId>> {
        // Get candidate nodes using different strategies
        let mut candidates = HashMap::new();

        // Get nodes from each routing strategy
        let strategies = self.router.get_all_strategies();
        for (strategy_name, strategy) in strategies {
            let nodes = strategy
                .find_closest_nodes(_content_hash, count * 2)
                .await?;
            for node in nodes {
                if !exclude.contains(&node) {
                    candidates
                        .entry(node)
                        .or_insert(Vec::new())
                        .push(strategy_name.clone());
                }
            }
        }

        // Score nodes based on composite criteria
        let mut scored_nodes: Vec<(NodeId, f64)> = Vec::new();
        for (node, strategies_found) in candidates {
            let score = self
                .calculate_node_score(&node, _content_hash, &strategies_found)
                .await;
            scored_nodes.push((node, score));
        }

        // Sort by score (descending) and take top nodes
        scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored_nodes
            .into_iter()
            .take(count)
            .map(|(node, _)| node)
            .collect())
    }

    /// Calculate composite node score for replication
    async fn calculate_node_score(
        &self,
        node: &NodeId,
        _content_hash: &ContentHash,
        strategies_found: &[String],
    ) -> f64 {
        // Base score from specification:
        // Score = 0.4×(XOR_proximity) + 0.3×(trust) + 0.2×(hyperbolic_distance) + 0.1×(SOM_similarity)

        let mut score = 0.0;

        // XOR proximity (if found by Kademlia)
        if strategies_found.contains(&"Kademlia".to_string()) {
            score += 0.4;
        }

        // Trust score
        let trust = self.trust_provider.get_trust_score(node);
        score += 0.3 * trust;

        // Hyperbolic proximity (if found by Hyperbolic routing)
        if strategies_found.contains(&"Hyperbolic".to_string()) {
            score += 0.2;
        }

        // SOM similarity (if found by SOM routing)
        if strategies_found.contains(&"SOM".to_string()) {
            score += 0.1;
        }

        // Bonus for being found by multiple strategies
        score += 0.1 * (strategies_found.len() as f64 / 4.0);

        // Penalty for high churn probability
        let churn_probability = self.churn_predictor.predict(node).await.probability_1h;
        score *= 1.0 - (churn_probability * 0.5);

        score
    }

    /// Replicate content to maintain target replication factor
    pub async fn replicate_content(
        &self,
        _content_hash: &ContentHash,
        content: &[u8],
        metadata: ContentMetadata,
    ) -> Result<ReplicaInfo> {
        // Calculate target replication factor
        let target_factor = self.calculate_replication_factor(_content_hash).await;

        // Get current replicas
        let mut replica_map = self.replica_map.write().await;
        let current_replicas = replica_map
            .get(_content_hash)
            .map(|info| info.storing_nodes.clone())
            .unwrap_or_default();

        let current_count = current_replicas.len() as u32;

        // If we need more replicas
        if current_count < target_factor {
            let needed = (target_factor - current_count) as usize;
            let new_nodes = self
                .select_replication_nodes(_content_hash, needed, &current_replicas)
                .await?;

            // Replicate to selected nodes (in real implementation, would send via network)
            let mut successful_nodes = current_replicas.clone();
            let mut stats = self.stats.write().await;

            for node in new_nodes {
                // Simulate replication (would actually send content to node)
                if self
                    .send_replica_to_node(&node, _content_hash, content)
                    .await
                {
                    successful_nodes.insert(node);
                    stats.successful_replications += 1;
                } else {
                    stats.failed_replications += 1;
                }
                stats.total_replications += 1;
            }

            // Update replica info
            let replication_factor = successful_nodes.len() as u32;
            let replica_info = ReplicaInfo {
                storing_nodes: successful_nodes,
                replication_factor,
                target_factor,
                last_check: Instant::now(),
                metadata,
            };

            replica_map.insert(_content_hash.clone(), replica_info.clone());
            Ok(replica_info)
        } else {
            // Already have enough replicas
            let replica_info = replica_map
                .get(_content_hash)
                .cloned()
                .unwrap_or(ReplicaInfo {
                    storing_nodes: current_replicas,
                    replication_factor: current_count,
                    target_factor,
                    last_check: Instant::now(),
                    metadata,
                });
            Ok(replica_info)
        }
    }

    /// Check and maintain replication for stored content
    pub async fn maintain_replications(&self) -> Result<()> {
        let replica_map = self.replica_map.read().await;
        let content_to_check: Vec<_> = replica_map
            .iter()
            .filter(|(_, info)| {
                // Check content that hasn't been checked recently
                info.last_check.elapsed() > Duration::from_secs(300) // 5 minutes
            })
            .map(|(hash, info)| (hash.clone(), info.clone()))
            .collect();
        drop(replica_map);

        for (content_hash, mut replica_info) in content_to_check {
            // Check if any storing nodes are at risk of churning
            let mut at_risk_nodes = Vec::new();
            for node in &replica_info.storing_nodes {
                if self.churn_predictor.should_replicate(node).await {
                    at_risk_nodes.push(node.clone());
                }
            }

            // Proactively replicate if nodes are at risk
            if !at_risk_nodes.is_empty() {
                let mut stats = self.stats.write().await;
                stats.proactive_replications += 1;
                drop(stats);

                // Select new nodes to replace at-risk ones
                let replacement_nodes = self
                    .select_replication_nodes(
                        &content_hash,
                        at_risk_nodes.len(),
                        &replica_info.storing_nodes,
                    )
                    .await?;

                // Update storing nodes (in real implementation, would trigger actual replication)
                for (old_node, new_node) in at_risk_nodes.iter().zip(replacement_nodes.iter()) {
                    replica_info.storing_nodes.remove(old_node);
                    replica_info.storing_nodes.insert(new_node.clone());
                }
            }

            // Update last check time
            replica_info.last_check = Instant::now();
            self.replica_map
                .write()
                .await
                .insert(content_hash, replica_info);
        }

        Ok(())
    }

    /// Handle node departure by checking affected content
    pub async fn handle_node_departure(&self, departed_node: &NodeId) -> Result<()> {
        let replica_map = self.replica_map.read().await;
        let affected_content: Vec<_> = replica_map
            .iter()
            .filter(|(_, info)| info.storing_nodes.contains(departed_node))
            .map(|(hash, info)| (hash.clone(), info.clone()))
            .collect();
        drop(replica_map);

        for (content_hash, mut replica_info) in affected_content {
            // Remove departed node
            replica_info.storing_nodes.remove(departed_node);
            replica_info.replication_factor = replica_info.storing_nodes.len() as u32;

            // Check if we need to replicate to maintain factor
            if replica_info.replication_factor < replica_info.target_factor {
                let needed =
                    (replica_info.target_factor - replica_info.replication_factor) as usize;
                let new_nodes = self
                    .select_replication_nodes(&content_hash, needed, &replica_info.storing_nodes)
                    .await?;

                // Add new replicas (in real implementation)
                for node in new_nodes {
                    replica_info.storing_nodes.insert(node);
                }
                replica_info.replication_factor = replica_info.storing_nodes.len() as u32;
            }

            self.replica_map
                .write()
                .await
                .insert(content_hash, replica_info);
        }

        Ok(())
    }

    /// Simulate sending replica to a node (would use network in real implementation)
    async fn send_replica_to_node(
        &self,
        node: &NodeId,
        _content_hash: &ContentHash,
        _content: &[u8],
    ) -> bool {
        // In real implementation, this would:
        // 1. Establish connection to node
        // 2. Send STORE_REPLICA message
        // 3. Wait for acknowledgment
        // 4. Return success/failure

        // For now, simulate with trust-based success probability
        let trust = self.trust_provider.get_trust_score(node);
        rand::random::<f64>() < trust
    }

    /// Estimate current network churn rate
    async fn estimate_churn_rate(&self) -> f64 {
        // In real implementation, would calculate from network statistics
        // For now, return a simulated value
        0.2 // 20% churn rate
    }

    /// Get content popularity score
    async fn get_content_popularity(&self, _content_hash: &ContentHash) -> f64 {
        // In real implementation, would track access patterns
        // For now, return a simulated value
        0.5
    }

    /// Get replication statistics
    pub async fn get_stats(&self) -> ReplicationStats {
        let stats = self.stats.read().await;
        let replica_map = self.replica_map.read().await;

        // Calculate average replication factor
        let avg_factor = if replica_map.is_empty() {
            0.0
        } else {
            let total_factor: u32 = replica_map
                .values()
                .map(|info| info.replication_factor)
                .sum();
            total_factor as f64 / replica_map.len() as f64
        };

        ReplicationStats {
            avg_replication_factor: avg_factor,
            ..stats.clone()
        }
    }

    /// Increase global replication factor during high churn
    pub async fn increase_global_replication(&self, _multiplier: f64) {
        // This would increase the replication factor for all content
        // For now, just log the action
        // log::info!("Increasing global replication by {:.2}x due to high churn", multiplier);

        // In a real implementation, would update config and trigger re-replication
        // self.config.base_replicas = (self.config.base_replicas as f64 * multiplier) as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::trust::MockTrustProvider;
    use std::sync::Arc;

    fn create_test_replication_manager() -> ReplicationManager {
        let config = ReplicationConfig::default();
        let trust_provider = Arc::new(MockTrustProvider::new());
        let churn_predictor = Arc::new(ChurnPredictor::new());
        let router = Arc::new(AdaptiveRouter::new(
            trust_provider.clone(),
            Arc::new(crate::adaptive::hyperbolic::HyperbolicSpace::new()),
            Arc::new(crate::adaptive::som::SelfOrganizingMap::new(
                crate::adaptive::som::SomConfig {
                    initial_learning_rate: 0.5,
                    initial_radius: 3.0,
                    iterations: 1000,
                    grid_size: crate::adaptive::som::GridSize::Fixed(10, 10),
                },
            )),
        ));

        ReplicationManager::new(config, trust_provider, churn_predictor, router)
    }

    #[tokio::test]
    async fn test_adaptive_replication_factor() {
        let manager = create_test_replication_manager();
        let content_hash = ContentHash([1u8; 32]);

        // Test base replication factor
        let factor = manager.calculate_replication_factor(&content_hash).await;
        assert!(factor >= manager.config.min_replicas);
        assert!(factor <= manager.config.max_replicas);
    }

    #[tokio::test]
    async fn test_node_selection_excludes_nodes() {
        let manager = create_test_replication_manager();
        let content_hash = ContentHash([1u8; 32]);
        let mut exclude = HashSet::new();
        exclude.insert(NodeId { hash: [1u8; 32] });
        exclude.insert(NodeId { hash: [2u8; 32] });

        let nodes = manager
            .select_replication_nodes(&content_hash, 5, &exclude)
            .await
            .unwrap();

        // Verify excluded nodes are not in results
        for node in nodes {
            assert!(!exclude.contains(&node));
        }
    }

    #[tokio::test]
    async fn test_replication_tracking() {
        let manager = create_test_replication_manager();
        let content_hash = ContentHash([1u8; 32]);
        let content = b"Test content";
        let metadata = ContentMetadata {
            size: content.len(),
            content_type: ContentType::DataRetrieval,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            chunk_count: None,
            replication_factor: 8,
        };

        // Perform replication
        let replica_info = manager
            .replicate_content(&content_hash, content, metadata)
            .await
            .unwrap();

        // Check that replicas were tracked
        assert!(replica_info.replication_factor > 0);
        assert!(!replica_info.storing_nodes.is_empty());

        // Check stats
        let stats = manager.get_stats().await;
        assert!(stats.total_replications > 0);
    }

    #[tokio::test]
    async fn test_proactive_replication() {
        let manager = create_test_replication_manager();

        // Add some content to track
        let content_hash = ContentHash([1u8; 32]);
        let mut replica_info = ReplicaInfo {
            storing_nodes: HashSet::new(),
            replication_factor: 3,
            target_factor: 5,
            last_check: Instant::now() - Duration::from_secs(400), // Old check
            metadata: ContentMetadata {
                size: 100,
                content_type: ContentType::DataRetrieval,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                chunk_count: None,
                replication_factor: 5,
            },
        };

        // Add some nodes
        for i in 0..3 {
            replica_info.storing_nodes.insert(NodeId {
                hash: [i as u8; 32],
            });
        }

        manager
            .replica_map
            .write()
            .await
            .insert(content_hash, replica_info);

        // Run maintenance
        manager.maintain_replications().await.unwrap();

        // Check that maintenance was performed
        let updated = manager
            .replica_map
            .read()
            .await
            .get(&content_hash)
            .unwrap()
            .clone();
        assert!(updated.last_check.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_node_departure_handling() {
        let manager = create_test_replication_manager();
        let departed_node = NodeId { hash: [1u8; 32] };

        // Add content that includes the departed node
        let content_hash = ContentHash([1u8; 32]);
        let mut storing_nodes = HashSet::new();
        storing_nodes.insert(departed_node.clone());
        storing_nodes.insert(NodeId { hash: [2u8; 32] });
        storing_nodes.insert(NodeId { hash: [3u8; 32] });

        let replica_info = ReplicaInfo {
            storing_nodes,
            replication_factor: 3,
            target_factor: 5,
            last_check: Instant::now(),
            metadata: ContentMetadata {
                size: 100,
                content_type: ContentType::DataRetrieval,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                chunk_count: None,
                replication_factor: 5,
            },
        };

        manager
            .replica_map
            .write()
            .await
            .insert(content_hash.clone(), replica_info);

        // Handle departure
        manager.handle_node_departure(&departed_node).await.unwrap();

        // Check that node was removed and replication adjusted
        let updated = manager
            .replica_map
            .read()
            .await
            .get(&content_hash)
            .unwrap()
            .clone();
        assert!(!updated.storing_nodes.contains(&departed_node));
    }
}
