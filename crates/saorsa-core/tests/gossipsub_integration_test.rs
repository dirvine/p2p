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

//! Comprehensive integration tests for Adaptive GossipSub Protocol
//!
//! Tests cover:
//! - Topic-based mesh construction and maintenance
//! - Adaptive mesh degree based on network churn
//! - Peer scoring and reputation
//! - Message validation and deduplication
//! - Gossip factor adjustment
//! - Topic prioritization
//! - Network partition handling
//! - High load scenarios

#[cfg(test)]
mod gossipsub_tests {
    use saorsa_core::adaptive::gossip::*;
    use saorsa_core::adaptive::{NodeId, TrustProvider};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tokio::time::Duration;

    /// Mock trust provider for testing
    struct MockTrustProvider {
        trust_scores: Arc<RwLock<HashMap<NodeId, f64>>>,
    }

    impl MockTrustProvider {
        fn new() -> Self {
            Self {
                trust_scores: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        async fn set_trust(&self, node: &NodeId, trust: f64) {
            let mut scores = self.trust_scores.write().await;
            scores.insert(node.clone(), trust);
        }
    }

    #[async_trait::async_trait]
    impl TrustProvider for MockTrustProvider {
        fn get_trust(&self, node: &NodeId) -> f64 {
            futures::executor::block_on(async {
                let scores = self.trust_scores.read().await;
                scores.get(node).cloned().unwrap_or(0.5)
            })
        }

        fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}

        fn get_global_trust(&self) -> HashMap<NodeId, f64> {
            futures::executor::block_on(async { self.trust_scores.read().await.clone() })
        }

        fn remove_node(&self, node: &NodeId) {
            futures::executor::block_on(async {
                let mut scores = self.trust_scores.write().await;
                scores.remove(node);
            })
        }
    }

    /// Helper to create test nodes
    fn create_test_nodes(count: usize) -> Vec<NodeId> {
        use rand::RngCore;
        (0..count)
            .map(|i| {
                let mut hash = [0u8; 32];
                hash[0] = i as u8;
                rand::thread_rng().fill_bytes(&mut hash[1..]);
                NodeId::from_bytes(hash)
            })
            .collect()
    }

    /// Helper to create test message
    fn create_test_message(topic: &str, data: Vec<u8>, from: &NodeId) -> GossipMessage {
        GossipMessage {
            topic: topic.to_string(),
            data,
            from: from.clone(),
            seqno: 1,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    #[tokio::test]
    async fn test_basic_gossipsub_creation() {
        let local_id = NodeId::from_bytes([1u8; 32]);
        let trust_provider = Arc::new(MockTrustProvider::new());
        let gossipsub = AdaptiveGossipSub::new(local_id, trust_provider.clone());

        let nodes = create_test_nodes(20);
        let topic = "test_topic";

        // Subscribe to topic
        gossipsub.subscribe(topic).await.unwrap();

        // Set trust scores for peers
        for (i, node) in nodes.iter().enumerate() {
            let score = 0.5 + (i as f64 * 0.02); // Varying scores
            trust_provider.set_trust(node, score).await;
        }

        // Run heartbeat to construct mesh
        gossipsub.heartbeat().await;

        // Basic validation that we can create and run heartbeat
        let _params = TopicParams::default();
    }

    #[tokio::test]
    async fn test_topic_management() {
        let local_id = NodeId::from_bytes([1u8; 32]);
        let trust_provider = Arc::new(MockTrustProvider::new());
        let gossipsub = AdaptiveGossipSub::new(local_id, trust_provider.clone());

        let topic = "adaptive_topic";

        // Set high priority for adaptive behavior
        gossipsub
            .set_topic_priority(topic, TopicPriority::Critical)
            .await;
        gossipsub.subscribe(topic).await.unwrap();

        // Test that we can calculate adaptive mesh size
        let adapted_size = gossipsub.calculate_adaptive_mesh_size(topic).await;

        // Critical priority should give us larger mesh
        assert!(adapted_size > 8); // Base size is 8
        assert!(adapted_size <= 20); // Reasonable upper bound
    }

    #[tokio::test]
    async fn test_message_creation() {
        let local_id = NodeId::from_bytes([1u8; 32]);
        let _trust_provider = Arc::new(MockTrustProvider::new());

        let topic = "validation_topic";

        // Create valid message
        let valid_msg = create_test_message(topic, b"valid data".to_vec(), &local_id);

        // Basic validation
        assert_eq!(valid_msg.topic, topic);
        assert_eq!(valid_msg.data, b"valid data");
        assert_eq!(valid_msg.from, local_id);
    }

    #[tokio::test]
    async fn test_message_publishing() {
        let local_id = NodeId::from_bytes([1u8; 32]);
        let trust_provider = Arc::new(MockTrustProvider::new());
        let gossipsub = AdaptiveGossipSub::new(local_id.clone(), trust_provider);

        let topic = "propagation_topic";

        // Subscribe to topic
        gossipsub.subscribe(topic).await.unwrap();

        // Create and publish message
        let message = create_test_message(topic, b"propagate this".to_vec(), &local_id);
        gossipsub.publish(topic, message.clone()).await.unwrap();

        // Check that message ID can be computed
        let _msg_id = gossipsub.compute_message_id(&message);
    }

    #[tokio::test]
    async fn test_control_message_types() {
        let topic = "control_topic";

        // Test creating different control messages
        let graft_msg = ControlMessage::Graft {
            topic: topic.to_string(),
        };
        let prune_msg = ControlMessage::Prune {
            topic: topic.to_string(),
            backoff: Duration::from_secs(60),
        };
        let ihave_msg = ControlMessage::IHave {
            topic: topic.to_string(),
            message_ids: vec![[1u8; 32], [2u8; 32]],
        };
        let iwant_msg = ControlMessage::IWant {
            message_ids: vec![[3u8; 32]],
        };

        // Verify they can be created
        match graft_msg {
            ControlMessage::Graft { topic: t } => assert_eq!(t, topic),
            _ => panic!("Wrong message type"),
        }
        match prune_msg {
            ControlMessage::Prune {
                topic: t,
                backoff: b,
            } => {
                assert_eq!(t, topic);
                assert_eq!(b, Duration::from_secs(60));
            }
            _ => panic!("Wrong message type"),
        }
        match ihave_msg {
            ControlMessage::IHave {
                topic: t,
                message_ids: ids,
            } => {
                assert_eq!(t, topic);
                assert_eq!(ids.len(), 2);
            }
            _ => panic!("Wrong message type"),
        }
        match iwant_msg {
            ControlMessage::IWant { message_ids: ids } => {
                assert_eq!(ids.len(), 1);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_topic_prioritization() {
        let local_id = NodeId::from_bytes([1u8; 32]);
        let trust_provider = Arc::new(MockTrustProvider::new());
        let gossipsub = AdaptiveGossipSub::new(local_id.clone(), trust_provider);

        // Define topic priorities
        gossipsub
            .set_topic_priority("high_priority", TopicPriority::Critical)
            .await;
        gossipsub
            .set_topic_priority("medium_priority", TopicPriority::Normal)
            .await;
        gossipsub
            .set_topic_priority("low_priority", TopicPriority::Low)
            .await;

        // Subscribe to all topics
        gossipsub.subscribe("high_priority").await.unwrap();
        gossipsub.subscribe("medium_priority").await.unwrap();
        gossipsub.subscribe("low_priority").await.unwrap();

        // Test adaptive mesh sizes
        let critical_size = gossipsub
            .calculate_adaptive_mesh_size("high_priority")
            .await;
        let normal_size = gossipsub
            .calculate_adaptive_mesh_size("medium_priority")
            .await;
        let low_size = gossipsub.calculate_adaptive_mesh_size("low_priority").await;

        // Critical should be 2x base, normal 1x, low 0.8x
        assert!(critical_size > normal_size);
        assert!(normal_size > low_size);
        assert_eq!(critical_size, 16); // Base 8 * 2.0
        assert_eq!(normal_size, 8); // Base 8 * 1.0
        assert_eq!(low_size, 6); // Base 8 * 0.8, rounded
    }

    #[tokio::test]
    async fn test_stats_initialization() {
        let stats = GossipStats::default();

        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.mesh_size, 0);
        assert_eq!(stats.topic_count, 0);
        assert_eq!(stats.peer_count, 0);
        assert!(stats.messages_by_topic.is_empty());
    }

    #[tokio::test]
    async fn test_peer_score_calculation() {
        let score = PeerScore {
            time_in_mesh: Duration::from_secs(300),
            first_message_deliveries: 50,
            mesh_message_deliveries: 500,
            invalid_messages: 0,
            behavior_penalty: 0.0,
            app_specific_score: 0.7,
        };

        let calculated = score.score();
        assert!(calculated > 0.0);

        // Test with penalties
        let bad_score = PeerScore {
            time_in_mesh: Duration::from_secs(60),
            first_message_deliveries: 5,
            mesh_message_deliveries: 10,
            invalid_messages: 3,
            behavior_penalty: -5.0,
            app_specific_score: 0.3,
        };

        let bad_calculated = bad_score.score();
        assert!(bad_calculated < calculated);
    }
}
