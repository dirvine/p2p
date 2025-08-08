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


//! Comprehensive benchmarks for adaptive P2P network
//!
//! This benchmark suite establishes performance baselines
//! for all critical operations in the adaptive network.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use saorsa_core::adaptive::*;
use std::time::Duration;

fn bench_identity_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("identity");

    group.bench_function("generate_new", |b| {
        b.iter(|| {
            let identity = NodeIdentity::generate().unwrap();
            black_box(identity);
        });
    });

    group.bench_function("from_seed", |b| {
        let seed = [42u8; 32];
        b.iter(|| {
            let identity = NodeIdentity::from_seed(&seed).unwrap();
            black_box(identity);
        });
    });

    group.bench_function("proof_of_work_16", |b| {
        let node_id = NodeId { hash: [1u8; 32] };
        b.iter(|| {
            let pow = ProofOfWork::compute(&node_id, 16).unwrap();
            black_box(pow);
        });
    });

    group.finish();
}

fn bench_routing_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing");

    // Hyperbolic routing
    group.bench_function("hyperbolic_distance", |b| {
        let coord1 = HyperbolicCoordinate { r: 0.5, theta: 1.0 };
        let coord2 = HyperbolicCoordinate { r: 0.7, theta: 2.0 };

        b.iter(|| {
            let dist = hyperbolic_distance(&coord1, &coord2);
            black_box(dist);
        });
    });

    // Kademlia XOR distance
    group.bench_function("kademlia_xor_distance", |b| {
        let id1 = NodeId { hash: [1u8; 32] };
        let id2 = NodeId { hash: [2u8; 32] };

        b.iter(|| {
            let dist = xor_distance(&id1, &id2);
            black_box(dist);
        });
    });

    // SOM operations
    for size in [10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::new("som_find_bmu", size), size, |b, &size| {
            let som = SelfOrganizingMap::new(size, size, 4);
            let input = vec![0.5; 4];

            b.iter(|| {
                let (x, y) = som.find_bmu(&input);
                black_box((x, y));
            });
        });
    }

    group.finish();
}

fn bench_trust_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("trust");

    for num_nodes in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("eigentrust_iteration", num_nodes),
            num_nodes,
            |b, &num_nodes| {
                let engine = setup_trust_network(num_nodes);

                b.iter(|| {
                    engine.compute_iteration();
                });
            },
        );
    }

    group.bench_function("trust_update", |b| {
        let engine = EigenTrustEngine::new(NodeId { hash: [0u8; 32] }).unwrap();
        let from = NodeId { hash: [1u8; 32] };
        let to = NodeId { hash: [2u8; 32] };

        b.iter(|| {
            engine.update_trust(&from, &to, true);
        });
    });

    group.finish();
}

fn bench_storage_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage");

    for size in [1024, 10240, 102400].iter() {
        group.bench_with_input(BenchmarkId::new("store_data", size), size, |b, &size| {
            let storage = create_test_storage();
            let data = vec![0u8; size];

            b.iter(|| {
                let hash = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(storage.store(data.clone()))
                    .unwrap();
                black_box(hash);
            });
        });
    }

    group.bench_function("retrieve_cached", |b| {
        let storage = create_test_storage();
        let data = vec![42u8; 1024];
        let hash = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(storage.store(data))
            .unwrap();

        b.iter(|| {
            let retrieved = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(storage.retrieve(&hash))
                .unwrap();
            black_box(retrieved);
        });
    });

    group.finish();
}

fn bench_ml_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("machine_learning");

    // Multi-armed bandit
    group.bench_function("mab_selection", |b| {
        let mut mab = setup_mab_with_arms(10);

        b.iter(|| {
            let selected = mab.select_arm();
            black_box(selected);
        });
    });

    // Q-Learning cache
    group.bench_function("q_learning_decision", |b| {
        let q_cache = QLearningCacheManager::new(QLearningConfig::default());
        let state = create_test_state();

        b.iter(|| {
            let action = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(q_cache.select_action(&state));
            black_box(action);
        });
    });

    // LSTM churn prediction
    group.bench_function("lstm_prediction", |b| {
        let predictor = create_test_lstm_predictor();
        let features = create_test_features();

        b.iter(|| {
            let prediction = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(predictor.predict(&features))
                .unwrap();
            black_box(prediction);
        });
    });

    group.finish();
}

fn bench_gossip_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("gossip");

    for num_peers in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("message_propagation", num_peers),
            num_peers,
            |b, &num_peers| {
                let gossip = setup_gossip_network(num_peers);
                let message = create_test_gossip_message();

                b.iter(|| {
                    tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(gossip.propagate(message.clone()))
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_coordinator_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinator");

    group.bench_function("full_store_retrieve_cycle", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let coordinator = runtime.block_on(create_test_coordinator());
        let data = vec![42u8; 1024];

        b.iter(|| {
            let hash = runtime.block_on(coordinator.store(data.clone())).unwrap();
            let retrieved = runtime.block_on(coordinator.retrieve(&hash)).unwrap();
            black_box(retrieved);
        });
    });

    group.bench_function("message_routing", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let coordinator = runtime.block_on(create_test_coordinator());
        let message = create_test_network_message();

        b.iter(|| {
            runtime
                .block_on(coordinator.route_message(message.clone()))
                .ok();
        });
    });

    group.finish();
}

// Helper functions
fn hyperbolic_distance(a: &HyperbolicCoordinate, b: &HyperbolicCoordinate) -> f64 {
    let delta = ((a.r * a.theta.cos() - b.r * b.theta.cos()).powi(2)
        + (a.r * a.theta.sin() - b.r * b.theta.sin()).powi(2))
    .sqrt();

    let cosh_d = 1.0 + 2.0 * delta.powi(2) / ((1.0 - a.r.powi(2)) * (1.0 - b.r.powi(2)));
    cosh_d.max(1.0).acosh()
}

fn xor_distance(a: &NodeId, b: &NodeId) -> [u8; 32] {
    let mut dist = [0u8; 32];
    for i in 0..32 {
        dist[i] = a.hash[i] ^ b.hash[i];
    }
    dist
}

fn setup_trust_network(num_nodes: usize) -> EigenTrustEngine {
    let engine = EigenTrustEngine::new(NodeId { hash: [0u8; 32] }).unwrap();

    // Add nodes and some interactions
    for i in 0..num_nodes {
        for j in 0..num_nodes {
            if i != j && (i + j) % 3 == 0 {
                let from = NodeId {
                    hash: [i as u8; 32],
                };
                let to = NodeId {
                    hash: [j as u8; 32],
                };
                engine.update_trust(&from, &to, true);
            }
        }
    }

    engine
}

fn create_test_storage() -> ContentStore {
    let config = StorageConfig {
        max_size: 100 * 1024 * 1024, // 100MB
        chunk_size: 1024 * 1024,     // 1MB
        compression_enabled: false,
        encryption_enabled: false,
    };
    ContentStore::new(config).unwrap()
}

fn setup_mab_with_arms(num_arms: usize) -> MultiArmedBandit {
    let config = MABConfig::default();
    let mut mab = MultiArmedBandit::new(config);

    for i in 0..num_arms {
        mab.add_arm(RouteId::from(format!("arm_{}", i)));
    }

    // Warm up with some initial pulls
    for _ in 0..100 {
        let arm = mab.select_arm();
        let reward = rand::random::<f64>();
        mab.update(arm, reward);
    }

    mab
}

fn create_test_state() -> StateVector {
    StateVector {
        cache_size: 0.5,
        hit_rate: 0.7,
        network_load: 0.3,
        available_space: 0.8,
        time_of_day: 0.5,
        content_popularity: 0.6,
    }
}

fn create_test_lstm_predictor() -> LSTMChurnPredictor {
    LSTMChurnPredictor::new()
}

fn create_test_features() -> ChurnFeatures {
    ChurnFeatures {
        churn_history: vec![0.1; 24],
        hour_of_day: 12,
        day_of_week: 3,
        network_size: 1000,
        avg_session_duration: Duration::from_secs(7200),
        node_uptime: Duration::from_secs(86400),
        join_leave_ratio: 1.0,
    }
}

fn setup_gossip_network(num_peers: usize) -> AdaptiveGossipSub {
    let trust_provider = Arc::new(MockTrustProvider::new());
    let gossip = AdaptiveGossipSub::new(NodeId { hash: [0u8; 32] }, trust_provider);

    // Add peers
    for i in 0..num_peers {
        let peer_id = NodeId {
            hash: [i as u8; 32],
        };
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(gossip.add_peer(peer_id))
            .unwrap();
    }

    gossip
}

fn create_test_gossip_message() -> GossipMessage {
    GossipMessage {
        topic: "test-topic".to_string(),
        data: vec![42u8; 256],
        from: NodeId { hash: [0u8; 32] },
        seqno: 1,
        timestamp: 0,
    }
}

async fn create_test_coordinator() -> NetworkCoordinator {
    let identity = NodeIdentity::generate().unwrap();
    let config = NetworkConfig::default();
    NetworkCoordinator::new(identity, config).await.unwrap()
}

fn create_test_network_message() -> NetworkMessage {
    NetworkMessage {
        id: "test-msg".to_string(),
        sender: NodeId { hash: [0u8; 32] },
        content: vec![1, 2, 3, 4],
        msg_type: ContentType::DHTLookup,
        timestamp: 0,
    }
}

// Benchmark groups
criterion_group!(
    benches,
    bench_identity_generation,
    bench_routing_operations,
    bench_trust_computation,
    bench_storage_operations,
    bench_ml_operations,
    bench_gossip_operations,
    bench_coordinator_operations
);

criterion_main!(benches);
