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

//! Churn handling and recovery system
//!
//! This module implements churn detection and recovery mechanisms:
//! - Node failure detection with 30-second heartbeat timeout
//! - Proactive content replication based on churn predictions
//! - Routing table repair and maintenance
//! - Trust score updates for unexpected departures
//! - Topology rebalancing for network health
//! - Graceful degradation under high churn

use crate::adaptive::{
    ContentHash, NodeId, TrustProvider,
    gossip::{AdaptiveGossipSub, GossipMessage},
    learning::ChurnPredictor,
    replication::ReplicationManager,
    routing::AdaptiveRouter,
};
use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Churn detection and recovery system
pub struct ChurnHandler {
    /// Local node ID
    node_id: NodeId,

    /// Churn predictor for proactive measures
    predictor: Arc<ChurnPredictor>,

    /// Node monitoring system
    node_monitor: Arc<NodeMonitor>,

    /// Recovery manager for content
    recovery_manager: Arc<RecoveryManager>,

    /// Trust provider for reputation updates
    trust_provider: Arc<dyn TrustProvider>,

    /// Replication manager
    replication_manager: Arc<ReplicationManager>,

    /// Routing system
    router: Arc<AdaptiveRouter>,

    /// Gossip system for announcements
    gossip: Arc<AdaptiveGossipSub>,

    /// Configuration
    config: ChurnConfig,

    /// Churn statistics
    stats: Arc<RwLock<ChurnStats>>,
}

/// Configuration for churn handling
#[derive(Debug, Clone)]
pub struct ChurnConfig {
    /// Heartbeat timeout (default: 30 seconds)
    pub heartbeat_timeout: Duration,

    /// Gossip absence timeout (default: 5 minutes)
    pub gossip_timeout: Duration,

    /// Prediction threshold for proactive measures (default: 0.7)
    pub prediction_threshold: f64,

    /// Monitoring interval (default: 30 seconds)
    pub monitoring_interval: Duration,

    /// Maximum acceptable churn rate (default: 30%)
    pub max_churn_rate: f64,
}

impl Default for ChurnConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout: Duration::from_secs(30),
            gossip_timeout: Duration::from_secs(300),
            prediction_threshold: 0.7,
            monitoring_interval: Duration::from_secs(30),
            max_churn_rate: 0.3,
        }
    }
}

/// Node monitoring system
pub struct NodeMonitor {
    /// Node status tracking
    node_status: Arc<RwLock<HashMap<NodeId, NodeStatus>>>,

    /// Heartbeat tracking
    heartbeats: Arc<RwLock<HashMap<NodeId, Instant>>>,

    /// Configuration
    config: ChurnConfig,
}

/// Status of a monitored node
#[derive(Debug, Clone)]
pub struct NodeStatus {
    /// Node identifier
    pub node_id: NodeId,

    /// Last seen timestamp
    pub last_seen: Instant,

    /// Last heartbeat received
    pub last_heartbeat: Option<Instant>,

    /// Last gossip activity
    pub last_gossip: Option<Instant>,

    /// Current status
    pub status: NodeState,

    /// Reliability score (0.0-1.0)
    pub reliability: f64,

    /// Content stored by this node
    pub stored_content: HashSet<ContentHash>,
}

/// Node state in the network
#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    /// Node is active and healthy
    Active,

    /// Node is suspicious (missed heartbeats)
    Suspicious,

    /// Node is departing (predicted or announced)
    Departing,

    /// Node has failed
    Failed,
}

/// Recovery manager for handling failures
pub struct RecoveryManager {
    /// Content tracking
    content_tracker: Arc<RwLock<HashMap<ContentHash, ContentTracker>>>,

    /// Recovery queue
    recovery_queue: Arc<RwLock<Vec<RecoveryTask>>>,

    /// Active recoveries
    _active_recoveries: Arc<RwLock<HashMap<ContentHash, RecoveryStatus>>>,
}

/// Content tracking information
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ContentTracker {
    /// Content hash
    hash: ContentHash,

    /// Nodes storing this content
    storing_nodes: HashSet<NodeId>,

    /// Target replication factor
    target_replicas: u32,

    /// Last verification time
    last_verified: Instant,
}

/// Recovery task
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RecoveryTask {
    /// Content to recover
    content_hash: ContentHash,

    /// Failed nodes
    failed_nodes: Vec<NodeId>,

    /// Priority level
    priority: RecoveryPriority,

    /// Creation time
    created_at: Instant,
}

/// Recovery priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryPriority {
    /// Low priority - can wait
    Low,

    /// Normal priority
    Normal,

    /// High priority - important content
    High,

    /// Critical - immediate action needed
    Critical,
}

/// Recovery status
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RecoveryStatus {
    /// Start time
    started_at: Instant,

    /// Nodes contacted for recovery
    contacted_nodes: Vec<NodeId>,

    /// Successful recoveries
    successful_nodes: Vec<NodeId>,

    /// Failed attempts
    failed_attempts: u32,
}

/// Churn handling statistics
#[derive(Debug, Default, Clone)]
pub struct ChurnStats {
    /// Total nodes monitored
    pub total_nodes: u64,

    /// Active nodes
    pub active_nodes: u64,

    /// Failed nodes
    pub failed_nodes: u64,

    /// Suspicious nodes
    pub suspicious_nodes: u64,

    /// Current churn rate
    pub churn_rate: f64,

    /// Successful recoveries
    pub successful_recoveries: u64,

    /// Failed recoveries
    pub failed_recoveries: u64,

    /// Proactive replications
    pub proactive_replications: u64,

    /// Average detection time
    pub avg_detection_time_ms: f64,
}

impl ChurnHandler {
    /// Create a new churn handler
    pub fn new(
        node_id: NodeId,
        predictor: Arc<ChurnPredictor>,
        trust_provider: Arc<dyn TrustProvider>,
        replication_manager: Arc<ReplicationManager>,
        router: Arc<AdaptiveRouter>,
        gossip: Arc<AdaptiveGossipSub>,
        config: ChurnConfig,
    ) -> Self {
        let node_monitor = Arc::new(NodeMonitor::new(config.clone()));
        let recovery_manager = Arc::new(RecoveryManager::new());

        Self {
            node_id,
            predictor,
            node_monitor,
            recovery_manager,
            trust_provider,
            replication_manager,
            router,
            gossip,
            config,
            stats: Arc::new(RwLock::new(ChurnStats::default())),
        }
    }

    /// Start monitoring network for churn
    pub async fn start_monitoring(&self) {
        let monitoring_interval = self.config.monitoring_interval;
        let handler = self.clone_for_task();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(monitoring_interval);

            loop {
                interval.tick().await;

                if let Err(_e) = handler.monitor_cycle().await {
                    // log::error!("Churn monitoring error: {}", e);
                }
            }
        });
    }

    /// Perform one monitoring cycle
    async fn monitor_cycle(&self) -> Result<()> {
        let nodes = self.node_monitor.get_all_nodes().await;
        let mut stats = self.stats.write().await;

        stats.total_nodes = nodes.len() as u64;
        stats.active_nodes = 0;
        stats.suspicious_nodes = 0;
        stats.failed_nodes = 0;

        for node_id in nodes {
            let status = self.node_monitor.get_node_status(&node_id).await;

            match status.status {
                NodeState::Active => {
                    stats.active_nodes += 1;

                    // Check churn prediction
                    let prediction = self.predictor.predict(&node_id).await;

                    if prediction.probability_1h > self.config.prediction_threshold {
                        self.handle_imminent_departure(&node_id).await?;
                        stats.proactive_replications += 1;
                    }
                }
                NodeState::Suspicious => {
                    stats.suspicious_nodes += 1;

                    // Check if node should be marked as failed
                    if status.last_seen.elapsed() > self.config.heartbeat_timeout {
                        self.handle_node_failure(&node_id).await?;
                    }
                }
                NodeState::Failed => {
                    stats.failed_nodes += 1;
                }
                _ => {}
            }
        }

        // Calculate churn rate
        if stats.total_nodes > 0 {
            stats.churn_rate = stats.failed_nodes as f64 / stats.total_nodes as f64;
        }

        // Check if network is experiencing high churn
        if stats.churn_rate > self.config.max_churn_rate {
            self.handle_high_churn().await?;
        }

        Ok(())
    }

    /// Handle imminent node departure (predicted)
    async fn handle_imminent_departure(&self, node_id: &NodeId) -> Result<()> {
        // log::info!("Handling imminent departure for node {:?}", node_id);

        // 1. Mark node as departing
        self.node_monitor
            .update_node_state(node_id, NodeState::Departing)
            .await;

        // 2. Get content stored by this node
        let stored_content = self.get_content_stored_by(node_id).await?;

        // 3. Start aggressive replication
        for content_hash in stored_content {
            self.recovery_manager
                .increase_replication(&content_hash, RecoveryPriority::High)
                .await?;
        }

        // 4. Reroute ongoing connections
        self.router.mark_node_unreliable(node_id).await;

        // 5. Notify network via gossip
        let message = GossipMessage {
            topic: "node_departing".to_string(),
            data: bincode::serialize(&node_id)
                .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?,
            from: self.node_id.clone(),
            seqno: 0, // Will be set by gossip subsystem
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.gossip.publish("node_departing", message).await?;

        Ok(())
    }

    /// Handle confirmed node failure
    async fn handle_node_failure(&self, node_id: &NodeId) -> Result<()> {
        let start_time = Instant::now();
        // log::warn!("Handling node failure for {:?}", node_id);

        // 1. Mark node as failed
        self.node_monitor
            .update_node_state(node_id, NodeState::Failed)
            .await;

        // 2. Remove from all routing structures
        self.remove_from_routing_tables(node_id).await?;

        // 3. Identify lost content
        let lost_content = self.identify_lost_content(node_id).await?;

        // 4. Queue recovery tasks
        for content_hash in lost_content {
            self.recovery_manager
                .queue_recovery(
                    content_hash,
                    vec![node_id.clone()],
                    RecoveryPriority::Critical,
                )
                .await?;
        }

        // 5. Update trust scores
        self.penalize_unexpected_departure(node_id).await;

        // 6. Trigger topology rebalancing
        self.trigger_topology_rebalance().await?;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.failed_nodes += 1;
        let detection_time = start_time.elapsed().as_millis() as f64;
        stats.avg_detection_time_ms =
            (stats.avg_detection_time_ms * (stats.failed_nodes - 1) as f64 + detection_time)
                / stats.failed_nodes as f64;

        Ok(())
    }

    /// Handle high churn conditions
    async fn handle_high_churn(&self) -> Result<()> {
        // log::warn!("Network experiencing high churn, entering defensive mode");

        // 1. Increase replication factors globally
        self.replication_manager
            .increase_global_replication(1.5)
            .await;

        // 2. Reduce gossip fanout to conserve bandwidth
        self.gossip.reduce_fanout(0.75).await;

        // 3. Enable aggressive caching
        self.router.enable_aggressive_caching().await;

        // 4. Notify applications of degraded conditions
        let churn_rate = self.stats.read().await.churn_rate;
        let message = GossipMessage {
            topic: "high_churn_alert".to_string(),
            data: bincode::serialize(&churn_rate)
                .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?,
            from: self.node_id.clone(),
            seqno: 0, // Will be set by gossip subsystem
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.gossip.publish("high_churn_alert", message).await?;

        Ok(())
    }

    /// Remove failed node from routing tables
    async fn remove_from_routing_tables(&self, node_id: &NodeId) -> Result<()> {
        // Remove from Kademlia routing table
        self.router.remove_node(node_id).await;

        // Remove from hyperbolic space
        self.router.remove_hyperbolic_coordinate(node_id).await;

        // Remove from SOM
        self.router.remove_from_som(node_id).await;

        // Remove from trust system
        self.trust_provider.remove_node(node_id);

        Ok(())
    }

    /// Get content stored by a specific node
    async fn get_content_stored_by(&self, node_id: &NodeId) -> Result<Vec<ContentHash>> {
        let status = self.node_monitor.get_node_status(node_id).await;
        Ok(status.stored_content.into_iter().collect())
    }

    /// Identify content that may be lost due to node failure
    async fn identify_lost_content(&self, failed_node: &NodeId) -> Result<Vec<ContentHash>> {
        let all_content = self.get_content_stored_by(failed_node).await?;
        let mut at_risk_content = Vec::new();

        for content_hash in all_content {
            let remaining_replicas = self
                .recovery_manager
                .get_remaining_replicas(&content_hash, failed_node)
                .await?;

            // If below minimum replication factor, mark as at risk
            if remaining_replicas < 5 {
                at_risk_content.push(content_hash);
            }
        }

        Ok(at_risk_content)
    }

    /// Penalize node for unexpected departure
    async fn penalize_unexpected_departure(&self, node_id: &NodeId) {
        self.trust_provider.update_trust(
            &NodeId { hash: [0u8; 32] }, // System node
            node_id,
            false, // Negative interaction
        );
    }

    /// Trigger topology rebalancing after failures
    async fn trigger_topology_rebalance(&self) -> Result<()> {
        // Adjust hyperbolic coordinates
        self.router.rebalance_hyperbolic_space().await;

        // Update SOM grid if needed
        self.router.update_som_grid().await;

        // Recompute trust scores
        self.router.trigger_trust_recomputation().await;

        Ok(())
    }

    /// Handle heartbeat from a node
    pub async fn handle_heartbeat(&self, node_id: &NodeId) -> Result<()> {
        self.node_monitor.record_heartbeat(node_id).await;
        Ok(())
    }

    /// Handle gossip activity from a node
    pub async fn handle_gossip_activity(&self, node_id: &NodeId) -> Result<()> {
        self.node_monitor.record_gossip_activity(node_id).await;
        Ok(())
    }

    /// Get churn statistics
    pub async fn get_stats(&self) -> ChurnStats {
        self.stats.read().await.clone()
    }

    /// Clone for spawning tasks
    fn clone_for_task(&self) -> Self {
        Self {
            node_id: self.node_id.clone(),
            predictor: self.predictor.clone(),
            node_monitor: self.node_monitor.clone(),
            recovery_manager: self.recovery_manager.clone(),
            trust_provider: self.trust_provider.clone(),
            replication_manager: self.replication_manager.clone(),
            router: self.router.clone(),
            gossip: self.gossip.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}

impl NodeMonitor {
    /// Create a new node monitor
    pub fn new(config: ChurnConfig) -> Self {
        Self {
            node_status: Arc::new(RwLock::new(HashMap::new())),
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Get all monitored nodes
    pub async fn get_all_nodes(&self) -> Vec<NodeId> {
        self.node_status.read().await.keys().cloned().collect()
    }

    /// Get node status
    pub async fn get_node_status(&self, node_id: &NodeId) -> NodeStatus {
        self.node_status
            .read()
            .await
            .get(node_id)
            .cloned()
            .unwrap_or(NodeStatus {
                node_id: node_id.clone(),
                last_seen: Instant::now(),
                last_heartbeat: None,
                last_gossip: None,
                status: NodeState::Failed,
                reliability: 0.0,
                stored_content: HashSet::new(),
            })
    }

    /// Update node state
    pub async fn update_node_state(&self, node_id: &NodeId, state: NodeState) {
        if let Some(status) = self.node_status.write().await.get_mut(node_id) {
            status.status = state;
        }
    }

    /// Record heartbeat from node
    pub async fn record_heartbeat(&self, node_id: &NodeId) {
        let now = Instant::now();
        self.heartbeats.write().await.insert(node_id.clone(), now);

        if let Some(status) = self.node_status.write().await.get_mut(node_id) {
            status.last_heartbeat = Some(now);
            status.last_seen = now;
            status.status = NodeState::Active;
        }
    }

    /// Record gossip activity
    pub async fn record_gossip_activity(&self, node_id: &NodeId) {
        let now = Instant::now();

        if let Some(status) = self.node_status.write().await.get_mut(node_id) {
            status.last_gossip = Some(now);
            status.last_seen = now;
        }
    }

    /// Check if node is alive
    pub async fn is_alive(&self, node_id: &NodeId) -> bool {
        if let Some(last_heartbeat) = self.heartbeats.read().await.get(node_id) {
            last_heartbeat.elapsed() < self.config.heartbeat_timeout
        } else {
            false
        }
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new() -> Self {
        Self {
            content_tracker: Arc::new(RwLock::new(HashMap::new())),
            recovery_queue: Arc::new(RwLock::new(Vec::new())),
            _active_recoveries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Increase replication for content
    pub async fn increase_replication(
        &self,
        content_hash: &ContentHash,
        priority: RecoveryPriority,
    ) -> Result<()> {
        // Queue a replication task
        self.queue_recovery(content_hash.clone(), vec![], priority)
            .await
    }

    /// Queue content for recovery
    pub async fn queue_recovery(
        &self,
        content_hash: ContentHash,
        failed_nodes: Vec<NodeId>,
        priority: RecoveryPriority,
    ) -> Result<()> {
        let task = RecoveryTask {
            content_hash,
            failed_nodes,
            priority,
            created_at: Instant::now(),
        };

        let mut queue = self.recovery_queue.write().await;
        queue.push(task);

        // Sort by priority
        queue.sort_by_key(|task| std::cmp::Reverse(task.priority));

        Ok(())
    }

    /// Get remaining replicas for content
    pub async fn get_remaining_replicas(
        &self,
        content_hash: &ContentHash,
        exclude_node: &NodeId,
    ) -> Result<u32> {
        if let Some(tracker) = self.content_tracker.read().await.get(content_hash) {
            let remaining = tracker
                .storing_nodes
                .iter()
                .filter(|&n| n != exclude_node)
                .count() as u32;
            Ok(remaining)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::som::{GridSize, SomConfig};
    use crate::adaptive::trust::MockTrustProvider;
    use rand::RngCore;

    async fn create_test_churn_handler() -> ChurnHandler {
        let predictor = Arc::new(ChurnPredictor::new());
        let trust_provider = Arc::new(MockTrustProvider::new());
        let replication_manager = Arc::new(ReplicationManager::new(
            Default::default(),
            trust_provider.clone(),
            predictor.clone(),
            Arc::new(AdaptiveRouter::new(
                trust_provider.clone(),
                Arc::new(crate::adaptive::hyperbolic::HyperbolicSpace::new()),
                Arc::new(crate::adaptive::som::SelfOrganizingMap::new(SomConfig {
                    initial_learning_rate: 0.3,
                    initial_radius: 5.0,
                    iterations: 100,
                    grid_size: GridSize::Fixed(10, 10),
                })),
            )),
        ));
        let router = Arc::new(AdaptiveRouter::new(
            trust_provider.clone(),
            Arc::new(crate::adaptive::hyperbolic::HyperbolicSpace::new()),
            Arc::new(crate::adaptive::som::SelfOrganizingMap::new(SomConfig {
                initial_learning_rate: 0.3,
                initial_radius: 5.0,
                iterations: 100,
                grid_size: GridSize::Fixed(10, 10),
            })),
        ));
        // Create a test NodeId
        use crate::peer_record::UserId;
        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let node_id = UserId::from_bytes(hash);

        let gossip = Arc::new(AdaptiveGossipSub::new(
            node_id.clone(),
            trust_provider.clone(),
        ));

        // Create default ChurnConfig
        let config = ChurnConfig {
            heartbeat_timeout: Duration::from_secs(30),
            gossip_timeout: Duration::from_secs(300),
            monitoring_interval: Duration::from_secs(5),
            prediction_threshold: 0.7,
            max_churn_rate: 0.2,
        };

        ChurnHandler::new(
            node_id,
            predictor,
            trust_provider,
            replication_manager,
            router,
            gossip,
            config,
        )
    }

    #[tokio::test]
    async fn test_node_monitoring() {
        let handler = create_test_churn_handler().await;
        let node_id = NodeId { hash: [1u8; 32] };

        // Record heartbeat
        handler.handle_heartbeat(&node_id).await.unwrap();

        // Check node is alive
        assert!(handler.node_monitor.is_alive(&node_id).await);

        // Check status
        let status = handler.node_monitor.get_node_status(&node_id).await;
        assert_eq!(status.status, NodeState::Active);
        assert!(status.last_heartbeat.is_some());
    }

    #[tokio::test]
    async fn test_failure_detection() {
        let mut config = ChurnConfig::default();
        config.heartbeat_timeout = Duration::from_millis(100); // Short timeout for testing

        let handler = create_test_churn_handler().await;
        let node_id = NodeId { hash: [1u8; 32] };

        // Record initial heartbeat
        handler.handle_heartbeat(&node_id).await.unwrap();

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Node should no longer be alive
        assert!(!handler.node_monitor.is_alive(&node_id).await);
    }

    #[tokio::test]
    async fn test_proactive_replication() {
        let handler = create_test_churn_handler().await;
        let node_id = NodeId { hash: [1u8; 32] };

        // Add node with high churn probability
        handler
            .predictor
            .update_node_features(
                &node_id,
                vec![
                    0.1, // Low online duration
                    0.9, // High response time
                    0.1, // Low resource contribution
                    0.1, // Low message frequency
                    0.0, // Time of day
                    0.0, // Day of week
                    0.1, // Low historical reliability
                    0.0, 0.0, 0.0,
                ],
            )
            .await
            .unwrap();

        // Handle imminent departure
        handler.handle_imminent_departure(&node_id).await.unwrap();

        // Check node marked as departing
        let status = handler.node_monitor.get_node_status(&node_id).await;
        assert_eq!(status.status, NodeState::Departing);
    }

    #[tokio::test]
    async fn test_churn_rate_calculation() {
        let handler = create_test_churn_handler().await;

        // Add some nodes
        for i in 0..10 {
            let node_id = NodeId { hash: [i; 32] };
            handler.handle_heartbeat(&node_id).await.unwrap();
        }

        // Mark some as failed
        for i in 0..3 {
            let node_id = NodeId { hash: [i; 32] };
            handler
                .node_monitor
                .update_node_state(&node_id, NodeState::Failed)
                .await;
        }

        // Run monitoring cycle
        handler.monitor_cycle().await.unwrap();

        // Check stats
        let stats = handler.get_stats().await;
        assert_eq!(stats.total_nodes, 10);
        assert_eq!(stats.failed_nodes, 3);
        assert!((stats.churn_rate - 0.3).abs() < 0.01); // 30% churn rate
    }
}
