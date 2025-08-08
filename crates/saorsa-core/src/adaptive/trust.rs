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

//! EigenTrust++ implementation for decentralized reputation management
//!
//! Provides global trust scores based on local peer interactions with
//! pre-trusted nodes and time decay

use super::*;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// EigenTrust++ engine for reputation management
pub struct EigenTrustEngine {
    /// Local trust scores between pairs of nodes
    local_trust: Arc<RwLock<HashMap<(NodeId, NodeId), LocalTrustData>>>,

    /// Global trust scores
    global_trust: Arc<RwLock<HashMap<NodeId, f64>>>,

    /// Pre-trusted nodes
    pre_trusted_nodes: Arc<RwLock<HashSet<NodeId>>>,

    /// Node statistics for multi-factor trust
    node_stats: Arc<RwLock<HashMap<NodeId, NodeStatistics>>>,

    /// Teleportation probability (alpha parameter)
    alpha: f64,

    /// Trust decay rate
    decay_rate: f64,

    /// Last update timestamp
    last_update: RwLock<Instant>,

    /// Update interval for batch processing
    update_interval: Duration,

    /// Cached trust scores for fast synchronous access
    trust_cache: Arc<RwLock<HashMap<NodeId, f64>>>,
}

/// Local trust data with interaction history
#[derive(Debug, Clone)]
struct LocalTrustData {
    /// Current trust value
    value: f64,
    /// Number of interactions
    interactions: u64,
    /// Last interaction time
    last_interaction: Instant,
}

/// Node statistics for multi-factor trust calculation
#[derive(Debug, Clone, Default)]
pub struct NodeStatistics {
    /// Total uptime in seconds
    pub uptime: u64,
    /// Number of correct responses
    pub correct_responses: u64,
    /// Number of failed responses
    pub failed_responses: u64,
    /// Storage contributed (GB)
    pub storage_contributed: u64,
    /// Bandwidth contributed (GB)
    pub bandwidth_contributed: u64,
    /// Compute cycles contributed
    pub compute_contributed: u64,
}

/// Statistics update type
#[derive(Debug, Clone)]
pub enum NodeStatisticsUpdate {
    Uptime(u64),
    CorrectResponse,
    FailedResponse,
    StorageContributed(u64),
    BandwidthContributed(u64),
    ComputeContributed(u64),
}

impl EigenTrustEngine {
    /// Create a new EigenTrust++ engine
    pub fn new(pre_trusted_nodes: HashSet<NodeId>) -> Self {
        let mut initial_cache = HashMap::new();
        // Pre-trusted nodes start with high trust
        for node in &pre_trusted_nodes {
            initial_cache.insert(node.clone(), 0.9);
        }

        Self {
            local_trust: Arc::new(RwLock::new(HashMap::new())),
            global_trust: Arc::new(RwLock::new(HashMap::new())),
            pre_trusted_nodes: Arc::new(RwLock::new(pre_trusted_nodes)),
            node_stats: Arc::new(RwLock::new(HashMap::new())),
            alpha: 0.15,
            decay_rate: 0.99,
            last_update: RwLock::new(Instant::now()),
            update_interval: Duration::from_secs(300), // 5 minutes
            trust_cache: Arc::new(RwLock::new(initial_cache)),
        }
    }

    /// Start background trust computation task
    pub fn start_background_updates(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(self.update_interval).await;
                let _ = self.compute_global_trust().await;
            }
        });
    }

    /// Update local trust based on interaction
    pub async fn update_local_trust(&self, from: &NodeId, to: &NodeId, success: bool) {
        let key = (from.clone(), to.clone());
        let new_value = if success { 1.0 } else { 0.0 };

        let mut trust_map = self.local_trust.write().await;
        trust_map
            .entry(key)
            .and_modify(|data| {
                // Exponential moving average
                data.value = 0.9 * data.value + 0.1 * new_value;
                data.interactions += 1;
                data.last_interaction = Instant::now();
            })
            .or_insert(LocalTrustData {
                value: new_value,
                interactions: 1,
                last_interaction: Instant::now(),
            });
    }

    /// Update node statistics
    pub async fn update_node_stats(&self, node_id: &NodeId, stats_update: NodeStatisticsUpdate) {
        let mut stats = self.node_stats.write().await;
        let node_stats = stats.entry(node_id.clone()).or_default();

        match stats_update {
            NodeStatisticsUpdate::Uptime(seconds) => node_stats.uptime += seconds,
            NodeStatisticsUpdate::CorrectResponse => node_stats.correct_responses += 1,
            NodeStatisticsUpdate::FailedResponse => node_stats.failed_responses += 1,
            NodeStatisticsUpdate::StorageContributed(gb) => node_stats.storage_contributed += gb,
            NodeStatisticsUpdate::BandwidthContributed(gb) => {
                node_stats.bandwidth_contributed += gb
            }
            NodeStatisticsUpdate::ComputeContributed(cycles) => {
                node_stats.compute_contributed += cycles
            }
        }
    }

    /// Compute global trust scores
    pub async fn compute_global_trust(&self) -> HashMap<NodeId, f64> {
        // Collect all nodes
        let local_trust = self.local_trust.read().await;
        let node_stats = self.node_stats.read().await;
        let pre_trusted = self.pre_trusted_nodes.read().await;

        let nodes: Vec<NodeId> = local_trust
            .keys()
            .flat_map(|(from, to)| vec![from.clone(), to.clone()])
            .chain(node_stats.keys().cloned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if nodes.is_empty() {
            return HashMap::new();
        }

        // Initialize trust vector
        let mut trust_vector: HashMap<NodeId, f64> = HashMap::new();
        for node in &nodes {
            trust_vector.insert(node.clone(), 1.0 / nodes.len() as f64);
        }

        // Power iteration
        for _ in 0..50 {
            let mut new_trust = HashMap::new();

            for node in &nodes {
                let mut trust_sum = 0.0;

                for other in &nodes {
                    if let Some(local_trust_val) =
                        self.get_normalized_trust(&local_trust, other, node)
                    {
                        trust_sum += local_trust_val * trust_vector.get(other).unwrap_or(&0.0);
                    }
                }

                // Add pre-trusted component
                let pre_trust = if pre_trusted.contains(node) {
                    1.0 / pre_trusted.len().max(1) as f64
                } else {
                    0.0
                };

                let new_value = (1.0 - self.alpha) * trust_sum + self.alpha * pre_trust;
                new_trust.insert(node.clone(), new_value);
            }

            // Check convergence
            let diff: f64 = trust_vector
                .iter()
                .map(|(node, old_trust)| (old_trust - new_trust.get(node).unwrap_or(&0.0)).abs())
                .sum();

            trust_vector = new_trust;

            if diff < 0.001 {
                break;
            }
        }

        // Apply multi-factor trust adjustments
        for (node, trust) in trust_vector.iter_mut() {
            if let Some(stats) = node_stats.get(node) {
                let factor = self.compute_multi_factor_adjustment(stats);
                *trust *= factor;
            }
        }

        // Apply time decay
        let last_update = self.last_update.read().await;
        let elapsed = last_update.elapsed().as_secs() as f64 / 3600.0; // hours

        for (_, trust) in trust_vector.iter_mut() {
            *trust *= self.decay_rate.powf(elapsed);
        }

        // Normalize trust scores
        let total_trust: f64 = trust_vector.values().sum();
        if total_trust > 0.0 {
            for (_, trust) in trust_vector.iter_mut() {
                *trust /= total_trust;
            }
        }

        // Update caches
        let mut global_trust = self.global_trust.write().await;
        let mut trust_cache = self.trust_cache.write().await;

        for (node, trust) in &trust_vector {
            global_trust.insert(node.clone(), *trust);
            trust_cache.insert(node.clone(), *trust);
        }

        // Update timestamp
        *self.last_update.write().await = Instant::now();

        trust_vector
    }

    /// Compute multi-factor trust adjustment based on node statistics
    fn compute_multi_factor_adjustment(&self, stats: &NodeStatistics) -> f64 {
        let response_rate = if stats.correct_responses + stats.failed_responses > 0 {
            stats.correct_responses as f64
                / (stats.correct_responses + stats.failed_responses) as f64
        } else {
            0.5
        };

        // Normalize contributions (log scale for large values)
        let storage_factor = (1.0 + stats.storage_contributed as f64).ln() / 10.0;
        let bandwidth_factor = (1.0 + stats.bandwidth_contributed as f64).ln() / 10.0;
        let compute_factor = (1.0 + stats.compute_contributed as f64).ln() / 10.0;
        let uptime_factor = (stats.uptime as f64 / 86400.0).min(1.0); // Max 1 day

        // Weighted combination
        0.4 * response_rate
            + 0.2 * uptime_factor
            + 0.15 * storage_factor
            + 0.15 * bandwidth_factor
            + 0.1 * compute_factor
    }

    /// Get normalized local trust
    fn get_normalized_trust(
        &self,
        local_trust: &HashMap<(NodeId, NodeId), LocalTrustData>,
        from: &NodeId,
        to: &NodeId,
    ) -> Option<f64> {
        let key = (from.clone(), to.clone());
        let trust_data = local_trust.get(&key)?;

        // Normalize by total outgoing trust
        let total_outgoing: f64 = local_trust
            .iter()
            .filter(|((f, _), _)| f == from)
            .map(|(_, data)| data.value.max(0.0))
            .sum();

        if total_outgoing > 0.0 {
            Some(trust_data.value.max(0.0) / total_outgoing)
        } else {
            None
        }
    }

    /// Add a pre-trusted node
    pub async fn add_pre_trusted(&self, node_id: NodeId) {
        let mut pre_trusted = self.pre_trusted_nodes.write().await;
        pre_trusted.insert(node_id.clone());

        // Update cache with high initial trust
        let mut cache = self.trust_cache.write().await;
        cache.insert(node_id, 0.9);
    }

    /// Remove a pre-trusted node
    pub async fn remove_pre_trusted(&self, node_id: &NodeId) {
        let mut pre_trusted = self.pre_trusted_nodes.write().await;
        pre_trusted.remove(node_id);
    }

    /// Get current trust score (fast synchronous access)
    pub async fn get_trust_async(&self, node_id: &NodeId) -> f64 {
        let cache = self.trust_cache.read().await;
        cache.get(node_id).copied().unwrap_or(0.5)
    }
}

impl TrustProvider for EigenTrustEngine {
    fn get_trust(&self, node: &NodeId) -> f64 {
        // Use cached value for synchronous access
        // The cache is updated by background task
        if let Ok(cache) = self.trust_cache.try_read() {
            cache.get(node).copied().unwrap_or(0.5)
        } else {
            // If we can't get the lock, return default trust
            0.5
        }
    }

    fn update_trust(&self, from: &NodeId, to: &NodeId, success: bool) {
        // Spawn a task to handle async update
        let local_trust = self.local_trust.clone();
        let from = from.clone();
        let to = to.clone();

        tokio::spawn(async move {
            let key = (from, to);
            let new_value = if success { 1.0 } else { 0.0 };

            let mut trust_map = local_trust.write().await;
            trust_map
                .entry(key)
                .and_modify(|data| {
                    data.value = 0.9 * data.value + 0.1 * new_value;
                    data.interactions += 1;
                    data.last_interaction = Instant::now();
                })
                .or_insert(LocalTrustData {
                    value: new_value,
                    interactions: 1,
                    last_interaction: Instant::now(),
                });
        });
    }

    fn get_global_trust(&self) -> HashMap<NodeId, f64> {
        // Return cached values for synchronous access
        if let Ok(cache) = self.trust_cache.try_read() {
            cache.clone()
        } else {
            HashMap::new()
        }
    }

    fn remove_node(&self, node: &NodeId) {
        // Schedule removal in background task
        let node_id = node.clone();
        let local_trust = self.local_trust.clone();
        let trust_cache = self.trust_cache.clone();

        tokio::spawn(async move {
            // Remove from local trust matrix
            let mut trust_map = local_trust.write().await;
            trust_map.retain(|(from, to), _| from != &node_id && to != &node_id);

            // Remove from cache
            let mut cache = trust_cache.write().await;
            cache.remove(&node_id);
        });
    }
}

/// Trust-based routing strategy
pub struct TrustBasedRoutingStrategy {
    /// Reference to the trust engine
    trust_engine: Arc<EigenTrustEngine>,

    /// Local node ID
    local_id: NodeId,

    /// Minimum trust threshold for routing
    min_trust_threshold: f64,
}

impl TrustBasedRoutingStrategy {
    /// Create a new trust-based routing strategy
    pub fn new(trust_engine: Arc<EigenTrustEngine>, local_id: NodeId) -> Self {
        Self {
            trust_engine,
            local_id,
            min_trust_threshold: 0.3,
        }
    }
}

#[async_trait]
impl RoutingStrategy for TrustBasedRoutingStrategy {
    async fn find_path(&self, target: &NodeId) -> Result<Vec<NodeId>> {
        // Get global trust scores
        let trust_scores = self.trust_engine.get_global_trust();

        // Filter nodes by minimum trust
        let mut trusted_nodes: Vec<(NodeId, f64)> = trust_scores
            .into_iter()
            .filter(|(id, trust)| {
                id != &self.local_id && id != target && *trust >= self.min_trust_threshold
            })
            .collect();

        // Sort by trust descending
        trusted_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Create path through highest trust nodes
        let path: Vec<NodeId> = trusted_nodes
            .into_iter()
            .take(3) // Max 3 intermediate hops
            .map(|(id, _)| id)
            .chain(std::iter::once(target.clone()))
            .collect();

        if path.len() == 1 {
            // Only target, no trusted intermediaries
            Err(AdaptiveNetworkError::Routing(
                "No trusted path found".to_string(),
            ))
        } else {
            Ok(path)
        }
    }

    fn route_score(&self, neighbor: &NodeId, _target: &NodeId) -> f64 {
        self.trust_engine.get_trust(neighbor)
    }

    fn update_metrics(&mut self, path: &[NodeId], success: bool) {
        // Update trust based on routing outcome
        if path.len() >= 2 {
            for window in path.windows(2) {
                self.trust_engine
                    .update_trust(&window[0], &window[1], success);
            }
        }
    }
}

/// Mock trust provider for testing
pub struct MockTrustProvider {
    trust_scores: Arc<RwLock<HashMap<NodeId, f64>>>,
}

impl Default for MockTrustProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTrustProvider {
    pub fn new() -> Self {
        Self {
            trust_scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl TrustProvider for MockTrustProvider {
    fn get_trust(&self, node: &NodeId) -> f64 {
        self.trust_scores
            .blocking_read()
            .get(node)
            .copied()
            .unwrap_or(0.5)
    }

    fn update_trust(&self, _from: &NodeId, to: &NodeId, success: bool) {
        let mut scores = self.trust_scores.blocking_write();
        let current = scores.get(to).copied().unwrap_or(0.5);
        let new_score = if success {
            (current + 0.1).min(1.0)
        } else {
            (current - 0.1).max(0.0)
        };
        scores.insert(to.clone(), new_score);
    }

    fn get_global_trust(&self) -> HashMap<NodeId, f64> {
        self.trust_scores.blocking_read().clone()
    }

    fn remove_node(&self, node: &NodeId) {
        self.trust_scores.blocking_write().remove(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_eigentrust_basic() {
        use rand::RngCore;

        let mut hash_pre = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash_pre);
        let pre_trusted = HashSet::from([NodeId::from_bytes(hash_pre)]);

        let engine = EigenTrustEngine::new(pre_trusted.clone());

        // Add some trust relationships
        let mut hash1 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash1);
        let node1 = NodeId { hash: hash1 };

        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);
        let node2 = NodeId { hash: hash2 };

        let pre_trusted_node = pre_trusted.iter().next().unwrap();

        engine
            .update_local_trust(pre_trusted_node, &node1, true)
            .await;
        engine.update_local_trust(&node1, &node2, true).await;
        engine.update_local_trust(&node2, &node1, false).await;

        // Compute global trust
        let global_trust = engine.compute_global_trust().await;

        // Pre-trusted node should have highest trust
        let pre_trust = global_trust.get(pre_trusted_node).unwrap_or(&0.0);
        let node1_trust = global_trust.get(&node1).unwrap_or(&0.0);

        assert!(pre_trust > node1_trust);
    }

    #[tokio::test]
    async fn test_trust_normalization() {
        use rand::RngCore;

        let engine = EigenTrustEngine::new(HashSet::new());

        let mut hash1 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash1);
        let node1 = NodeId { hash: hash1 };

        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);
        let node2 = NodeId { hash: hash2 };

        let mut hash3 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash3);
        let node3 = NodeId { hash: hash3 };

        engine.update_local_trust(&node1, &node2, true).await;
        engine.update_local_trust(&node1, &node3, true).await;

        // Both should have normalized trust of 0.5
        let local_trust = engine.local_trust.read().await;
        let trust2 = engine.get_normalized_trust(&local_trust, &node1, &node2);
        let trust3 = engine.get_normalized_trust(&local_trust, &node1, &node3);

        assert_eq!(trust2, Some(0.5));
        assert_eq!(trust3, Some(0.5));
    }

    #[tokio::test]
    async fn test_multi_factor_trust() {
        use rand::RngCore;

        let engine = Arc::new(EigenTrustEngine::new(HashSet::new()));

        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let node = NodeId { hash: hash };

        // Update node statistics
        engine
            .update_node_stats(&node, NodeStatisticsUpdate::Uptime(3600))
            .await;
        engine
            .update_node_stats(&node, NodeStatisticsUpdate::CorrectResponse)
            .await;
        engine
            .update_node_stats(&node, NodeStatisticsUpdate::CorrectResponse)
            .await;
        engine
            .update_node_stats(&node, NodeStatisticsUpdate::FailedResponse)
            .await;
        engine
            .update_node_stats(&node, NodeStatisticsUpdate::StorageContributed(100))
            .await;

        // Add some trust relationships
        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);
        let other = NodeId { hash: hash2 };

        engine.update_local_trust(&other, &node, true).await;

        // Compute global trust
        let global_trust = engine.compute_global_trust().await;

        // Node should have trust affected by its statistics
        let trust = global_trust.get(&node).unwrap_or(&0.0);
        assert!(*trust > 0.0);
    }

    #[tokio::test]
    async fn test_trust_decay() {
        use rand::RngCore;

        let mut engine = EigenTrustEngine::new(HashSet::new());
        engine.decay_rate = 0.5; // Fast decay for testing

        let mut hash1 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash1);
        let node1 = NodeId { hash: hash1 };

        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);
        let node2 = NodeId { hash: hash2 };

        engine.update_local_trust(&node1, &node2, true).await;

        // First computation
        let trust1 = engine.compute_global_trust().await;
        let initial_trust = trust1.get(&node2).copied().unwrap_or(0.0);

        // Simulate time passing by manually updating the timestamp
        *engine.last_update.write().await = Instant::now() - Duration::from_secs(3600);

        // Second computation should show decay
        let trust2 = engine.compute_global_trust().await;
        let decayed_trust = trust2.get(&node2).copied().unwrap_or(0.0);

        assert!(decayed_trust < initial_trust);
        assert!((decayed_trust - initial_trust * 0.5).abs() < 0.1); // Should be ~50% of original
    }

    #[tokio::test]
    async fn test_trust_based_routing() {
        use rand::RngCore;

        // Create pre-trusted nodes
        let mut hash_pre = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash_pre);
        let pre_trusted_id = NodeId::from_bytes(hash_pre);

        let engine = Arc::new(EigenTrustEngine::new(HashSet::from([
            pre_trusted_id.clone()
        ])));

        // Create some nodes
        let mut hash_local = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash_local);
        let local_id = NodeId::from_bytes(hash_local);

        let mut hash_target = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash_target);
        let target_id = NodeId::from_bytes(hash_target);

        // Build trust relationships
        engine
            .update_local_trust(&pre_trusted_id, &local_id, true)
            .await;
        engine.update_local_trust(&local_id, &target_id, true).await;

        // Compute trust
        engine.compute_global_trust().await;

        // Create routing strategy
        let strategy = TrustBasedRoutingStrategy::new(engine.clone(), local_id);

        // Try to find path
        let result = strategy.find_path(&target_id).await;

        // Should find a path through trusted nodes
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains(&target_id));
    }
}
