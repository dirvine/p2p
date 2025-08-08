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

//! Adaptive GossipSub implementation
//!
//! Enhanced gossip protocol with adaptive mesh degree, peer scoring,
//! and priority message types

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};

/// Topic identifier for gossip messages
pub type Topic = String;

/// Message identifier
pub type MessageId = [u8; 32];

/// Control messages for gossip protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    Graft {
        topic: Topic,
    },
    Prune {
        topic: Topic,
        backoff: Duration,
    },
    IHave {
        topic: Topic,
        message_ids: Vec<MessageId>,
    },
    IWant {
        message_ids: Vec<MessageId>,
    },
}

/// Topic priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TopicPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Message validation trait
#[async_trait::async_trait]
pub trait MessageValidator: Send + Sync {
    /// Validate a message before propagation
    async fn validate(&self, message: &GossipMessage) -> Result<bool>;
}

/// Gossip statistics
#[derive(Debug, Clone, Default)]
pub struct GossipStats {
    /// Total messages sent
    pub messages_sent: u64,

    /// Total messages received
    pub messages_received: u64,

    /// Current mesh size
    pub mesh_size: usize,

    /// Number of active topics
    pub topic_count: usize,

    /// Total peers
    pub peer_count: usize,

    /// Messages by topic
    pub messages_by_topic: HashMap<Topic, u64>,
}

/// Adaptive GossipSub implementation
pub struct AdaptiveGossipSub {
    /// Local node ID
    _local_id: NodeId,

    /// Mesh peers for each topic
    mesh: Arc<RwLock<HashMap<Topic, HashSet<NodeId>>>>,

    /// Fanout peers for topics we're not subscribed to
    fanout: Arc<RwLock<HashMap<Topic, HashSet<NodeId>>>>,

    /// Seen messages cache
    seen_messages: Arc<RwLock<HashMap<MessageId, Instant>>>,

    /// Message cache for IWANT requests
    message_cache: Arc<RwLock<HashMap<MessageId, GossipMessage>>>,

    /// Peer scores
    peer_scores: Arc<RwLock<HashMap<NodeId, PeerScore>>>,

    /// Topic parameters
    topics: Arc<RwLock<HashMap<Topic, TopicParams>>>,

    /// Topic priorities
    topic_priorities: Arc<RwLock<HashMap<Topic, TopicPriority>>>,

    /// Heartbeat interval
    _heartbeat_interval: Duration,

    /// Message validators by topic
    message_validators: Arc<RwLock<HashMap<Topic, Box<dyn MessageValidator + Send + Sync>>>>,

    /// Trust provider for peer scoring
    trust_provider: Arc<dyn TrustProvider>,

    /// Message receiver channel
    _message_rx: Arc<RwLock<Option<mpsc::Receiver<(NodeId, GossipMessage)>>>>,

    /// Control message sender
    control_tx: Arc<RwLock<Option<mpsc::Sender<(NodeId, ControlMessage)>>>>,

    /// Churn detector
    churn_detector: Arc<RwLock<ChurnDetector>>,

    /// Statistics
    stats: Arc<RwLock<GossipStats>>,
}

/// Gossip message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub topic: Topic,
    pub data: Vec<u8>,
    pub from: NodeId,
    pub seqno: u64,
    pub timestamp: u64,
}

/// Peer score tracking
#[derive(Debug, Clone)]
pub struct PeerScore {
    pub time_in_mesh: Duration,
    pub first_message_deliveries: u64,
    pub mesh_message_deliveries: u64,
    pub invalid_messages: u64,
    pub behavior_penalty: f64,
    pub app_specific_score: f64, // From trust system
}

impl PeerScore {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            time_in_mesh: Duration::ZERO,
            first_message_deliveries: 0,
            mesh_message_deliveries: 0,
            invalid_messages: 0,
            behavior_penalty: 0.0,
            app_specific_score: 0.5,
        }
    }

    fn score(&self) -> f64 {
        let time_score = (self.time_in_mesh.as_secs() as f64 / 60.0).min(10.0) * 0.5;
        let delivery_score = (self.first_message_deliveries as f64).min(100.0) / 100.0;
        let mesh_score = (self.mesh_message_deliveries as f64).min(1000.0) / 1000.0 * 0.2;
        let invalid_penalty = self.invalid_messages as f64 * -10.0;

        time_score
            + delivery_score
            + mesh_score
            + invalid_penalty
            + self.behavior_penalty
            + self.app_specific_score
    }
}

/// Topic parameters
#[derive(Debug, Clone)]
pub struct TopicParams {
    pub d: usize,                // Target mesh degree
    pub d_low: usize,            // Lower bound
    pub d_high: usize,           // Upper bound
    pub d_out: usize,            // Outbound degree for neighbor exchange
    pub graylist_threshold: f64, // Score below which peers are graylisted
    pub mesh_message_deliveries_threshold: f64,
    pub gossip_factor: f64, // % of peers to send IHave to
    pub priority: TopicPriority,
}

impl Default for TopicParams {
    fn default() -> Self {
        Self {
            d: 8,
            d_low: 6,
            d_high: 12,
            d_out: 2,
            graylist_threshold: -1.0,
            mesh_message_deliveries_threshold: 0.5,
            gossip_factor: 0.25,
            priority: TopicPriority::Normal,
        }
    }
}

/// Churn detection and tracking
#[derive(Debug, Clone)]
pub struct ChurnDetector {
    /// Recent peer join/leave events
    events: VecDeque<(Instant, ChurnEvent)>,
    /// Window size for churn calculation
    window: Duration,
    /// Current churn rate
    churn_rate: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ChurnEvent {
    PeerJoined(NodeId),
    PeerLeft(NodeId),
}

/// Churn statistics for a time window
#[derive(Debug)]
pub struct ChurnStats {
    /// Number of nodes that joined
    pub joins: usize,
    /// Number of nodes that left
    pub leaves: usize,
    /// Average session duration
    pub avg_session_duration: Duration,
    /// Node join times for uptime calculation
    node_join_times: HashMap<NodeId, Instant>,
}

impl ChurnStats {
    /// Get uptime for a specific node
    pub fn get_node_uptime(&self, node_id: &NodeId) -> Duration {
        self.node_join_times
            .get(node_id)
            .map(|join_time| Instant::now().duration_since(*join_time))
            .unwrap_or(Duration::from_secs(0))
    }
}

impl ChurnDetector {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
            window: Duration::from_secs(300), // 5 minute window
            churn_rate: 0.0,
        }
    }

    fn record_join(&mut self, peer: NodeId) {
        self.events
            .push_back((Instant::now(), ChurnEvent::PeerJoined(peer)));
        self.update_rate();
    }

    fn record_leave(&mut self, peer: NodeId) {
        self.events
            .push_back((Instant::now(), ChurnEvent::PeerLeft(peer)));
        self.update_rate();
    }

    fn update_rate(&mut self) {
        let cutoff = Instant::now() - self.window;
        self.events.retain(|(time, _)| *time > cutoff);

        let joins = self
            .events
            .iter()
            .filter(|(_, event)| matches!(event, ChurnEvent::PeerJoined(_)))
            .count();
        let leaves = self
            .events
            .iter()
            .filter(|(_, event)| matches!(event, ChurnEvent::PeerLeft(_)))
            .count();

        // Churn rate as percentage of changes
        self.churn_rate = (joins + leaves) as f64 / self.window.as_secs() as f64;
    }

    fn get_rate(&self) -> f64 {
        self.churn_rate
    }

    pub async fn get_hourly_rates(&self, hours: usize) -> Vec<f64> {
        let now = Instant::now();
        let mut hourly_rates = vec![0.0; hours];

        for (time, event) in &self.events {
            let age = now.duration_since(*time);
            let hour_index = (age.as_secs() / 3600) as usize;

            if hour_index < hours {
                match event {
                    ChurnEvent::PeerJoined(_) | ChurnEvent::PeerLeft(_) => {
                        hourly_rates[hour_index] += 1.0;
                    }
                }
            }
        }

        // Normalize to rates
        for rate in &mut hourly_rates {
            *rate /= 3600.0; // Events per second
        }

        hourly_rates
    }

    pub async fn get_recent_stats(&self, window: Duration) -> ChurnStats {
        let now = Instant::now();
        let mut joins = 0;
        let mut leaves = 0;
        let mut _session_durations = Vec::new();
        let mut _node_join_times = HashMap::new();

        for (time, event) in &self.events {
            if now.duration_since(*time) <= window {
                match event {
                    ChurnEvent::PeerJoined(node_id) => {
                        joins += 1;
                        _node_join_times.insert(node_id.clone(), *time);
                    }
                    ChurnEvent::PeerLeft(_) => leaves += 1,
                }
            }
        }

        let avg_session_duration = if _session_durations.is_empty() {
            Duration::from_secs(3600) // Default 1 hour
        } else {
            Duration::from_secs(
                _session_durations
                    .iter()
                    .map(|d: &Duration| d.as_secs())
                    .sum::<u64>()
                    / _session_durations.len() as u64,
            )
        };

        ChurnStats {
            joins,
            leaves,
            avg_session_duration,
            node_join_times: _node_join_times,
        }
    }
}

impl AdaptiveGossipSub {
    /// Create a new adaptive gossipsub instance
    pub fn new(local_id: NodeId, trust_provider: Arc<dyn TrustProvider>) -> Self {
        let (control_tx, _control_rx) = mpsc::channel(1000);
        let (_message_tx, message_rx) = mpsc::channel(1000);

        Self {
            _local_id: local_id,
            mesh: Arc::new(RwLock::new(HashMap::new())),
            fanout: Arc::new(RwLock::new(HashMap::new())),
            seen_messages: Arc::new(RwLock::new(HashMap::new())),
            message_cache: Arc::new(RwLock::new(HashMap::new())),
            peer_scores: Arc::new(RwLock::new(HashMap::new())),
            topics: Arc::new(RwLock::new(HashMap::new())),
            topic_priorities: Arc::new(RwLock::new(HashMap::new())),
            _heartbeat_interval: Duration::from_secs(1),
            message_validators: Arc::new(RwLock::new(HashMap::new())),
            trust_provider,
            _message_rx: Arc::new(RwLock::new(Some(message_rx))),
            control_tx: Arc::new(RwLock::new(Some(control_tx))),
            churn_detector: Arc::new(RwLock::new(ChurnDetector::new())),
            stats: Arc::new(RwLock::new(GossipStats::default())),
        }
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: &str) -> Result<()> {
        let mut topics = self.topics.write().await;
        topics
            .entry(topic.to_string())
            .or_insert_with(TopicParams::default);

        let mut mesh = self.mesh.write().await;
        mesh.insert(topic.to_string(), HashSet::new());

        Ok(())
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: &str) -> Result<()> {
        let mut mesh = self.mesh.write().await;
        mesh.remove(topic);

        Ok(())
    }

    /// Publish a message to a topic
    pub async fn publish(&self, topic: &str, message: GossipMessage) -> Result<()> {
        // Validate message before publishing
        if !self.validate_message(&message).await? {
            return Err(AdaptiveNetworkError::Gossip(
                "Message validation failed".to_string().into(),
            )
            .into());
        }

        let msg_id = self.compute_message_id(&message);

        // Add to seen messages and cache
        {
            let mut seen = self.seen_messages.write().await;
            seen.insert(msg_id, Instant::now());

            let mut cache = self.message_cache.write().await;
            cache.insert(msg_id, message.clone());
        }

        // Send to mesh peers
        let mesh = self.mesh.read().await;
        if let Some(mesh_peers) = mesh.get(topic) {
            for peer in mesh_peers {
                // In real implementation, send via network
                self.send_message(peer, &message).await?;
            }
        } else {
            // Use fanout if not subscribed
            let fanout = self.fanout.read().await;
            let fanout_peers = fanout
                .get(topic)
                .cloned()
                .unwrap_or_else(|| self.get_fanout_peers(topic).unwrap_or_default());

            for peer in &fanout_peers {
                self.send_message(peer, &message).await?;
            }
        }

        Ok(())
    }

    /// Send GRAFT control message
    pub async fn send_graft(&self, peer: &NodeId, topic: &str) -> Result<()> {
        let control_tx = self.control_tx.read().await;
        if let Some(tx) = control_tx.as_ref() {
            let msg = ControlMessage::Graft {
                topic: topic.to_string(),
            };
            tx.send((peer.clone(), msg)).await.map_err(|_| {
                AdaptiveNetworkError::Other("Failed to send GRAFT".to_string().into())
            })?;
        }
        Ok(())
    }

    /// Send PRUNE control message
    pub async fn send_prune(&self, peer: &NodeId, topic: &str, backoff: Duration) -> Result<()> {
        let control_tx = self.control_tx.read().await;
        if let Some(tx) = control_tx.as_ref() {
            let msg = ControlMessage::Prune {
                topic: topic.to_string(),
                backoff,
            };
            tx.send((peer.clone(), msg)).await.map_err(|_| {
                AdaptiveNetworkError::Other("Failed to send PRUNE".to_string().into())
            })?;
        }
        Ok(())
    }

    /// Send IHAVE control message
    pub async fn send_ihave(
        &self,
        peer: &NodeId,
        topic: &str,
        message_ids: Vec<MessageId>,
    ) -> Result<()> {
        let control_tx = self.control_tx.read().await;
        if let Some(tx) = control_tx.as_ref() {
            let msg = ControlMessage::IHave {
                topic: topic.to_string(),
                message_ids,
            };
            tx.send((peer.clone(), msg)).await.map_err(|_| {
                AdaptiveNetworkError::Other("Failed to send IHAVE".to_string().into())
            })?;
        }
        Ok(())
    }

    /// Send IWANT control message
    pub async fn send_iwant(&self, peer: &NodeId, message_ids: Vec<MessageId>) -> Result<()> {
        let control_tx = self.control_tx.read().await;
        if let Some(tx) = control_tx.as_ref() {
            let msg = ControlMessage::IWant { message_ids };
            tx.send((peer.clone(), msg)).await.map_err(|_| {
                AdaptiveNetworkError::Other("Failed to send IWANT".to_string().into())
            })?;
        }
        Ok(())
    }

    /// Handle periodic heartbeat
    pub async fn heartbeat(&self) {
        let mesh = self.mesh.read().await.clone();

        for (topic, mesh_peers) in mesh {
            let params = {
                let topics = self.topics.read().await;
                topics.get(&topic).cloned().unwrap_or_default()
            };

            // Calculate adaptive mesh size based on churn
            let target_size = self.calculate_adaptive_mesh_size(&topic).await;

            // Remove low-scoring peers
            let mut peers_to_remove = Vec::new();
            {
                let scores = self.peer_scores.read().await;
                for peer in &mesh_peers {
                    if let Some(score) = scores.get(peer) {
                        if score.score() < params.graylist_threshold {
                            peers_to_remove.push(peer.clone());
                        }
                    }
                }
            }

            // Update mesh
            let mut mesh_write = self.mesh.write().await;
            if let Some(topic_mesh) = mesh_write.get_mut(&topic) {
                // Send PRUNE messages and update churn detector
                for peer in peers_to_remove {
                    topic_mesh.remove(&peer);
                    let _ = self
                        .send_prune(&peer, &topic, Duration::from_secs(60))
                        .await;

                    // Record peer leaving mesh
                    let mut churn = self.churn_detector.write().await;
                    churn.record_leave(peer);
                }

                // Add high-scoring peers if below target
                while topic_mesh.len() < target_size {
                    if let Some(peer) = self.select_peer_for_mesh(&topic, topic_mesh).await {
                        topic_mesh.insert(peer.clone());
                        let _ = self.send_graft(&peer, &topic).await;

                        // Record peer joining mesh
                        let mut churn = self.churn_detector.write().await;
                        churn.record_join(peer);
                    } else {
                        break;
                    }
                }
            }
        }

        // Update peer scores
        self.update_peer_scores().await;

        // Clean old seen messages
        self.clean_seen_messages().await;
    }

    /// Calculate adaptive mesh size based on network conditions
    async fn calculate_adaptive_mesh_size(&self, topic: &str) -> usize {
        let base_size = 8;

        // Get churn rate from detector
        let churn_rate = {
            let churn = self.churn_detector.read().await;
            churn.get_rate()
        };

        // Get topic priority
        let priority_factor = {
            let priorities = self.topic_priorities.read().await;
            match priorities.get(topic) {
                Some(TopicPriority::Critical) => 2.0,
                Some(TopicPriority::High) => 1.5,
                Some(TopicPriority::Normal) => 1.0,
                Some(TopicPriority::Low) => 0.8,
                None => 1.0,
            }
        };

        // Increase mesh size based on churn and priority
        let churn_factor = 1.0 + (churn_rate * 0.1).min(0.5); // Max 50% increase

        (base_size as f64 * churn_factor * priority_factor).round() as usize
    }

    /// Select a peer to add to mesh
    async fn select_peer_for_mesh(
        &self,
        _topic: &str,
        current_mesh: &HashSet<NodeId>,
    ) -> Option<NodeId> {
        // Select from known peers not in mesh, sorted by score
        let scores = self.peer_scores.read().await;
        let mut candidates: Vec<_> = scores
            .iter()
            .filter(|(peer_id, _)| !current_mesh.contains(peer_id))
            .map(|(peer_id, score)| (peer_id.clone(), score.score()))
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.first().map(|(peer, _)| peer.clone())
    }

    /// Update peer scores
    async fn update_peer_scores(&self) {
        let mut scores = self.peer_scores.write().await;
        for (peer_id, score) in scores.iter_mut() {
            // Update app-specific score from trust system
            score.app_specific_score = self.trust_provider.get_trust(peer_id);

            // Decay behavior penalty
            score.behavior_penalty *= 0.99;
        }
    }

    /// Clean old seen messages
    async fn clean_seen_messages(&self) {
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes
        let mut seen = self.seen_messages.write().await;
        seen.retain(|_, timestamp| *timestamp > cutoff);
    }

    /// Compute message ID
    fn compute_message_id(&self, message: &GossipMessage) -> MessageId {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(message.topic.as_bytes());
        hasher.update(message.from.hash);
        hasher.update(message.seqno.to_le_bytes());
        hasher.update(&message.data);

        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        id
    }

    /// Send message to a peer (placeholder)
    async fn send_message(&self, _peer: &NodeId, _message: &GossipMessage) -> Result<()> {
        // In real implementation, send via network layer
        Ok(())
    }

    /// Get fanout peers for a topic
    fn get_fanout_peers(&self, _topic: &str) -> Option<HashSet<NodeId>> {
        // In real implementation, select high-scoring peers
        None
    }

    /// Handle incoming control message
    pub async fn handle_control_message(
        &self,
        from: &NodeId,
        message: ControlMessage,
    ) -> Result<()> {
        match message {
            ControlMessage::Graft { topic } => {
                // Peer wants to join our mesh
                let mut mesh = self.mesh.write().await;
                if let Some(topic_mesh) = mesh.get_mut(&topic) {
                    // Check peer score before accepting
                    let score = {
                        let scores = self.peer_scores.read().await;
                        scores.get(from).map(|s| s.score()).unwrap_or(0.0)
                    };

                    if score > 0.0 {
                        topic_mesh.insert(from.clone());
                    } else {
                        // Send PRUNE back if we don't want them
                        let _ = self.send_prune(from, &topic, Duration::from_secs(60)).await;
                    }
                }
            }
            ControlMessage::Prune { topic, backoff: _ } => {
                // Peer is removing us from their mesh
                let mut mesh = self.mesh.write().await;
                if let Some(topic_mesh) = mesh.get_mut(&topic) {
                    topic_mesh.remove(from);
                }
            }
            ControlMessage::IHave {
                topic: _,
                message_ids,
            } => {
                // Peer is announcing messages they have
                let seen = self.seen_messages.read().await;
                let mut want = Vec::new();

                for msg_id in message_ids {
                    if !seen.contains_key(&msg_id) {
                        want.push(msg_id);
                    }
                }

                if !want.is_empty() {
                    let _ = self.send_iwant(from, want).await;
                }
            }
            ControlMessage::IWant { message_ids } => {
                // Peer wants specific messages
                let cache = self.message_cache.read().await;
                for msg_id in message_ids {
                    if let Some(message) = cache.get(&msg_id) {
                        let _ = self.send_message(from, message).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Set topic priority
    pub async fn set_topic_priority(&self, topic: &str, priority: TopicPriority) {
        let mut priorities = self.topic_priorities.write().await;
        priorities.insert(topic.to_string(), priority);
    }

    /// Register a message validator for a topic
    pub async fn register_validator(
        &self,
        topic: &str,
        validator: Box<dyn MessageValidator + Send + Sync>,
    ) -> Result<()> {
        let mut validators = self.message_validators.write().await;
        validators.insert(topic.to_string(), validator);
        Ok(())
    }

    /// Validate a message before processing
    async fn validate_message(&self, message: &GossipMessage) -> Result<bool> {
        let validators = self.message_validators.read().await;

        if let Some(validator) = validators.get(&message.topic) {
            validator.validate(message).await
        } else {
            // No validator registered, accept by default
            Ok(true)
        }
    }

    /// Reduce gossip fanout during high churn
    pub async fn reduce_fanout(&self, factor: f64) {
        // In a real implementation, would reduce mesh degree based on factor
        // This would involve updating the target degree for mesh maintenance
        let _ = factor; // Suppress unused warning
    }

    /// Get gossip statistics
    pub async fn get_stats(&self) -> GossipStats {
        let mut stats = self.stats.read().await.clone();

        // Update current values
        let mesh = self.mesh.read().await;
        stats.mesh_size = mesh.values().map(|peers| peers.len()).sum();
        stats.topic_count = mesh.len();

        let peer_scores = self.peer_scores.read().await;
        stats.peer_count = peer_scores.len();

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gossipsub_subscribe() {
        struct MockTrustProvider;
        impl TrustProvider for MockTrustProvider {
            fn get_trust(&self, _node: &NodeId) -> f64 {
                0.5
            }
            fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}
            fn get_global_trust(&self) -> HashMap<NodeId, f64> {
                HashMap::new()
            }
            fn remove_node(&self, _node: &NodeId) {}
        }

        use crate::peer_record::UserId;
        use rand::RngCore;

        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let local_id = UserId::from_bytes(hash);

        let trust_provider = Arc::new(MockTrustProvider);
        let gossip = AdaptiveGossipSub::new(local_id, trust_provider);

        gossip.subscribe("test-topic").await.unwrap();

        let mesh = gossip.mesh.read().await;
        assert!(mesh.contains_key("test-topic"));
    }

    #[test]
    fn test_peer_score() {
        let mut score = PeerScore::new();
        assert!(score.score() > 0.0);

        score.invalid_messages = 5;
        assert!(score.score() < 0.0);
    }

    #[test]
    fn test_message_id() {
        struct MockTrustProvider;
        impl TrustProvider for MockTrustProvider {
            fn get_trust(&self, _node: &NodeId) -> f64 {
                0.5
            }
            fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}
            fn get_global_trust(&self) -> HashMap<NodeId, f64> {
                HashMap::new()
            }
            fn remove_node(&self, _node: &NodeId) {}
        }

        use crate::peer_record::UserId;
        use rand::RngCore;

        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let local_id = UserId::from_bytes(hash);

        let trust_provider = Arc::new(MockTrustProvider);
        let gossip = AdaptiveGossipSub::new(local_id, trust_provider);

        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);

        let msg = GossipMessage {
            topic: "test".to_string(),
            data: vec![1, 2, 3],
            from: UserId::from_bytes(hash2),
            seqno: 1,
            timestamp: 12345,
        };

        let id1 = gossip.compute_message_id(&msg);
        let id2 = gossip.compute_message_id(&msg);

        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn test_adaptive_mesh_size() {
        use crate::peer_record::UserId;
        use rand::RngCore;

        struct MockTrustProvider;
        impl TrustProvider for MockTrustProvider {
            fn get_trust(&self, _node: &NodeId) -> f64 {
                0.5
            }
            fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}
            fn get_global_trust(&self) -> HashMap<NodeId, f64> {
                HashMap::new()
            }
            fn remove_node(&self, _node: &NodeId) {}
        }

        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let local_id = UserId::from_bytes(hash);

        let trust_provider = Arc::new(MockTrustProvider);
        let gossip = AdaptiveGossipSub::new(local_id, trust_provider);

        // Set topic priority
        gossip
            .set_topic_priority("critical-topic", TopicPriority::Critical)
            .await;
        gossip
            .set_topic_priority("low-topic", TopicPriority::Low)
            .await;

        // Test mesh size calculation
        let critical_size = gossip.calculate_adaptive_mesh_size("critical-topic").await;
        let normal_size = gossip.calculate_adaptive_mesh_size("normal-topic").await;
        let low_size = gossip.calculate_adaptive_mesh_size("low-topic").await;

        assert!(critical_size > normal_size);
        assert!(normal_size > low_size);
    }

    #[test]
    fn test_churn_detector() {
        use crate::peer_record::UserId;
        use rand::RngCore;

        let mut detector = ChurnDetector::new();

        // Add some join/leave events
        for i in 0..10 {
            let mut hash = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut hash);
            hash[0] = i;
            let peer = UserId::from_bytes(hash);

            if i % 2 == 0 {
                detector.record_join(peer);
            } else {
                detector.record_leave(peer);
            }
        }

        let rate = detector.get_rate();
        assert!(rate > 0.0);
    }

    #[tokio::test]
    async fn test_control_messages() {
        use crate::peer_record::UserId;
        use rand::RngCore;

        struct MockTrustProvider;
        impl TrustProvider for MockTrustProvider {
            fn get_trust(&self, _node: &NodeId) -> f64 {
                0.8
            }
            fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}
            fn get_global_trust(&self) -> HashMap<NodeId, f64> {
                HashMap::new()
            }
            fn remove_node(&self, _node: &NodeId) {}
        }

        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let local_id = UserId::from_bytes(hash);

        let trust_provider = Arc::new(MockTrustProvider);
        let gossip = AdaptiveGossipSub::new(local_id, trust_provider);

        // Subscribe to a topic
        gossip.subscribe("test-topic").await.unwrap();

        // Test GRAFT handling
        let mut peer_hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut peer_hash);
        let peer_id = UserId::from_bytes(peer_hash);

        let graft_msg = ControlMessage::Graft {
            topic: "test-topic".to_string(),
        };
        gossip
            .handle_control_message(&peer_id, graft_msg)
            .await
            .unwrap();

        // Peer should be in mesh due to good trust score
        let mesh = gossip.mesh.read().await;
        assert!(mesh.get("test-topic").unwrap().contains(&peer_id));
    }

    #[tokio::test]
    async fn test_message_validation() {
        use crate::peer_record::UserId;
        use rand::RngCore;

        struct MockTrustProvider;
        impl TrustProvider for MockTrustProvider {
            fn get_trust(&self, _node: &NodeId) -> f64 {
                0.8
            }
            fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}
            fn get_global_trust(&self) -> HashMap<NodeId, f64> {
                HashMap::new()
            }
            fn remove_node(&self, _node: &NodeId) {}
        }

        // Custom validator that rejects messages with "bad" in the data
        struct TestValidator;
        #[async_trait::async_trait]
        impl MessageValidator for TestValidator {
            async fn validate(&self, message: &GossipMessage) -> Result<bool> {
                Ok(!message.data.windows(3).any(|w| w == b"bad"))
            }
        }

        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let local_id = UserId::from_bytes(hash);

        let trust_provider = Arc::new(MockTrustProvider);
        let gossip = AdaptiveGossipSub::new(local_id, trust_provider);

        // Register validator
        gossip
            .register_validator("test-topic", Box::new(TestValidator))
            .await
            .unwrap();

        // Test valid message
        let valid_message = GossipMessage {
            topic: "test-topic".to_string(),
            data: vec![1, 2, 3, 4], // No "bad" in data
            from: UserId::from_bytes([0; 32]),
            seqno: 1,
            timestamp: 12345,
        };

        // Should succeed
        assert!(gossip.publish("test-topic", valid_message).await.is_ok());

        // Test invalid message
        let invalid_message = GossipMessage {
            topic: "test-topic".to_string(),
            data: vec![b'b', b'a', b'd', b'!'], // Contains "bad"
            from: UserId::from_bytes([0; 32]),
            seqno: 2,
            timestamp: 12346,
        };

        // Should fail validation
        assert!(gossip.publish("test-topic", invalid_message).await.is_err());
    }

    #[tokio::test]
    async fn test_ihave_iwant_flow() {
        use crate::peer_record::UserId;
        use rand::RngCore;

        struct MockTrustProvider;
        impl TrustProvider for MockTrustProvider {
            fn get_trust(&self, _node: &NodeId) -> f64 {
                0.8
            }
            fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}
            fn get_global_trust(&self) -> HashMap<NodeId, f64> {
                HashMap::new()
            }
            fn remove_node(&self, _node: &NodeId) {}
        }

        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let local_id = UserId::from_bytes(hash);

        let trust_provider = Arc::new(MockTrustProvider);
        let gossip = AdaptiveGossipSub::new(local_id, trust_provider);

        // Create a test message
        let mut peer_hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut peer_hash);
        let from_peer = UserId::from_bytes(peer_hash);

        let message = GossipMessage {
            topic: "test-topic".to_string(),
            data: vec![1, 2, 3, 4],
            from: from_peer.clone(),
            seqno: 1,
            timestamp: 12345,
        };

        // Publish message (adds to cache)
        gossip.publish("test-topic", message.clone()).await.unwrap();

        // Message should be in cache
        let msg_id = gossip.compute_message_id(&message);
        let cache = gossip.message_cache.read().await;
        assert!(cache.contains_key(&msg_id));
    }
}
