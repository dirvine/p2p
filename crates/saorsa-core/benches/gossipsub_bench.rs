// Copyright (c) 2025 Saorsa Labs Limited

// This file is part of the Saorsa P2P network.

// Licensed under the AGPL-3.0 license:
// <https://www.gnu.org/licenses/agpl-3.0.html>

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use p2p_core::adaptive::gossip::*;
use p2p_core::adaptive::{NodeId, TrustProvider};
use std::collections::HashMap;
use std::sync::Arc;

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

fn create_test_gossipsub(num_peers: usize) -> (AdaptiveGossipSub, Vec<NodeId>) {
    use rand::RngCore;

    let mut hash = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut hash);
    let local_id = NodeId::from_bytes(hash);

    let trust_provider = Arc::new(MockTrustProvider);
    let gossipsub = AdaptiveGossipSub::new(local_id, trust_provider);

    let peers: Vec<NodeId> = (0..num_peers)
        .map(|i| {
            let mut hash = [0u8; 32];
            hash[0] = i as u8;
            rand::thread_rng().fill_bytes(&mut hash[1..]);
            NodeId::from_bytes(hash)
        })
        .collect();

    (gossipsub, peers)
}

fn bench_mesh_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("gossipsub_mesh_construction");

    for num_peers in [10, 50, 100, 500] {
        group.bench_function(format!("peers_{}", num_peers), |b| {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            b.iter_batched(
                || create_test_gossipsub(num_peers),
                |(gossipsub, peers)| {
                    runtime.block_on(async {
                        gossipsub.subscribe("test_topic").await.unwrap();

                        // Add peers to scores
                        {
                            let mut scores = gossipsub.peer_scores.write().await;
                            for peer in &peers {
                                scores.insert(peer.clone(), PeerScore::new());
                            }
                        }

                        black_box(gossipsub.heartbeat().await);
                    });
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_message_publishing(c: &mut Criterion) {
    let mut group = c.benchmark_group("gossipsub_message_publishing");

    for num_peers in [10, 50, 100] {
        group.bench_function(format!("peers_{}", num_peers), |b| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let (gossipsub, peers) = create_test_gossipsub(num_peers);

            runtime.block_on(async {
                gossipsub.subscribe("bench_topic").await.unwrap();

                // Set up mesh
                let mut scores = gossipsub.peer_scores.write().await;
                for peer in &peers {
                    scores.insert(peer.clone(), PeerScore::new());
                }
                drop(scores);

                let mut mesh = gossipsub.mesh.write().await;
                let topic_mesh = mesh.get_mut("bench_topic").unwrap();
                for peer in peers.iter().take(8) {
                    topic_mesh.insert(peer.clone());
                }
            });

            b.iter(|| {
                runtime.block_on(async {
                    let message = GossipMessage {
                        topic: "bench_topic".to_string(),
                        data: vec![1, 2, 3, 4],
                        from: peers[0].clone(),
                        seqno: 1,
                        timestamp: 12345,
                    };

                    black_box(gossipsub.publish("bench_topic", message).await.unwrap());
                });
            });
        });
    }

    group.finish();
}

fn bench_control_message_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("gossipsub_control_messages");

    group.bench_function("graft_handling", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (gossipsub, peers) = create_test_gossipsub(10);

        runtime.block_on(async {
            gossipsub.subscribe("test_topic").await.unwrap();
            let mut scores = gossipsub.peer_scores.write().await;
            scores.insert(peers[0].clone(), PeerScore::new());
        });

        b.iter(|| {
            runtime.block_on(async {
                let msg = ControlMessage::Graft {
                    topic: "test_topic".to_string(),
                };
                black_box(
                    gossipsub
                        .handle_control_message(&peers[0], msg)
                        .await
                        .unwrap(),
                );
            });
        });
    });

    group.bench_function("iwant_handling", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (gossipsub, peers) = create_test_gossipsub(10);

        runtime.block_on(async {
            // Add messages to cache
            let mut cache = gossipsub.message_cache.write().await;
            for i in 0..10 {
                let msg = GossipMessage {
                    topic: "test".to_string(),
                    data: vec![i],
                    from: peers[0].clone(),
                    seqno: i as u64,
                    timestamp: 12345,
                };
                let msg_id = gossipsub.compute_message_id(&msg);
                cache.insert(msg_id, msg);
            }
        });

        b.iter(|| {
            runtime.block_on(async {
                let msg = ControlMessage::IWant {
                    message_ids: vec![[1u8; 32], [2u8; 32]],
                };
                black_box(
                    gossipsub
                        .handle_control_message(&peers[0], msg)
                        .await
                        .unwrap(),
                );
            });
        });
    });

    group.finish();
}

fn bench_adaptive_mesh_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("gossipsub_adaptive_mesh");

    group.bench_function("calculate_adaptive_size", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (gossipsub, _) = create_test_gossipsub(100);

        runtime.block_on(async {
            gossipsub
                .set_topic_priority("critical", TopicPriority::Critical)
                .await;
            gossipsub
                .set_topic_priority("normal", TopicPriority::Normal)
                .await;
            gossipsub
                .set_topic_priority("low", TopicPriority::Low)
                .await;
        });

        b.iter(|| {
            runtime.block_on(async {
                black_box(gossipsub.calculate_adaptive_mesh_size("critical").await);
                black_box(gossipsub.calculate_adaptive_mesh_size("normal").await);
                black_box(gossipsub.calculate_adaptive_mesh_size("low").await);
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mesh_construction,
    bench_message_publishing,
    bench_control_message_handling,
    bench_adaptive_mesh_calculation
);

criterion_main!(benches);
