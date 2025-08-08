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

//! Performance benchmarks for Q-Learning Cache Management

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use saorsa_core::adaptive::{
    ActionType, CacheAction, ContentHash, Experience, NodeId, QLearningCacheManager,
    QLearningConfig, StateVector,
};
use tokio::runtime::Runtime;

fn create_test_manager(capacity: u64) -> QLearningCacheManager {
    let config = QLearningConfig {
        learning_rate: 0.1,
        discount_factor: 0.9,
        epsilon: 0.1,
        epsilon_decay: 0.995,
        epsilon_min: 0.01,
        buffer_size: 1000,
        batch_size: 32,
        learning_frequency: 10,
    };

    QLearningCacheManager::new(config, capacity)
}

fn benchmark_state_discretization(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_discretization");

    group.bench_function("create_state_vector", |b| {
        b.iter(|| {
            StateVector::from_metrics(
                black_box(0.75),      // utilization
                black_box(25.0),      // frequency
                black_box(300),       // recency
                black_box(1_048_576), // size
            )
        })
    });

    group.bench_function("state_space_calculation", |b| {
        b.iter(|| black_box(StateVector::state_space_size()))
    });

    group.finish();
}

fn benchmark_q_learning_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("q_learning_ops");
    let rt = Runtime::new().unwrap();
    let manager = create_test_manager(10 * 1024 * 1024); // 10MB

    // Pre-populate Q-table
    rt.block_on(async {
        for i in 0..100 {
            let state = StateVector::from_metrics(
                (i as f64) / 100.0,
                i as f64,
                i as u64 * 60,
                i as u64 * 1024,
            );
            for action in &[ActionType::Cache, ActionType::Evict, ActionType::DoNothing] {
                manager
                    .update_q_value(&state, *action, rand::random::<f64>(), &state, false)
                    .await
                    .unwrap();
            }
        }
    });

    group.bench_function("get_q_value", |b| {
        let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
        b.iter(|| {
            rt.block_on(async {
                manager
                    .get_q_value(black_box(&state), black_box(ActionType::Cache))
                    .await
            })
        })
    });

    group.bench_function("update_q_value", |b| {
        let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
        let next_state = StateVector::from_metrics(0.6, 11.0, 60, 1024);
        b.iter(|| {
            rt.block_on(async {
                manager
                    .update_q_value(
                        black_box(&state),
                        black_box(ActionType::Cache),
                        black_box(0.5),
                        black_box(&next_state),
                        black_box(false),
                    )
                    .await
                    .unwrap()
            })
        })
    });

    group.finish();
}

fn benchmark_action_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("action_selection");
    let rt = Runtime::new().unwrap();

    for epsilon in [0.0, 0.1, 0.5, 1.0].iter() {
        let config = QLearningConfig {
            epsilon: *epsilon,
            ..Default::default()
        };
        let manager = QLearningCacheManager::new(config, 10 * 1024 * 1024);

        // Set up some Q-values
        rt.block_on(async {
            let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
            manager
                .update_q_value(&state, ActionType::Cache, 1.0, &state, true)
                .await
                .unwrap();
            manager
                .update_q_value(&state, ActionType::Evict, 0.5, &state, true)
                .await
                .unwrap();
            manager
                .update_q_value(&state, ActionType::DoNothing, 0.7, &state, true)
                .await
                .unwrap();
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("epsilon_{}", epsilon)),
            epsilon,
            |b, _| {
                let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
                let content_hash = ContentHash([1u8; 32]);
                let actions = vec![
                    CacheAction::Cache(content_hash.clone()),
                    CacheAction::Evict(content_hash.clone()),
                    CacheAction::DoNothing,
                ];

                b.iter(|| {
                    rt.block_on(async {
                        manager
                            .select_action(black_box(&state), black_box(actions.clone()))
                            .await
                            .unwrap()
                    })
                })
            },
        );
    }

    group.finish();
}

fn benchmark_experience_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("experience_replay");
    let rt = Runtime::new().unwrap();

    for buffer_size in [100, 1000, 10000].iter() {
        let config = QLearningConfig {
            buffer_size: *buffer_size,
            batch_size: 32,
            learning_frequency: 10,
            ..Default::default()
        };
        let manager = QLearningCacheManager::new(config, 10 * 1024 * 1024);

        // Fill buffer to capacity
        rt.block_on(async {
            for i in 0..*buffer_size {
                let state = StateVector::from_metrics(
                    (i as f64) / (*buffer_size as f64),
                    i as f64,
                    i as u64,
                    i as u64 * 1024,
                );
                let experience = Experience {
                    state,
                    action: CacheAction::DoNothing,
                    reward: rand::random(),
                    next_state: state,
                    terminal: false,
                };
                manager
                    .experience_buffer
                    .write()
                    .await
                    .push_back(experience);
            }
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("buffer_{}", buffer_size)),
            buffer_size,
            |b, _| b.iter(|| rt.block_on(async { manager.learn_from_replay().await.unwrap() })),
        );
    }

    group.finish();
}

fn benchmark_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");
    let rt = Runtime::new().unwrap();
    let manager = create_test_manager(10 * 1024 * 1024);

    // Pre-populate cache
    rt.block_on(async {
        for i in 0..50 {
            let content_hash = ContentHash([i as u8; 32]);
            manager
                .update_statistics(
                    &CacheAction::Cache(content_hash.clone()),
                    &content_hash,
                    100 * 1024, // 100KB each
                    false,
                )
                .await
                .unwrap();
        }
    });

    group.bench_function("get_current_state", |b| {
        let content_hash = ContentHash([25u8; 32]);
        b.iter(|| {
            rt.block_on(async {
                manager
                    .get_current_state(black_box(&content_hash))
                    .await
                    .unwrap()
            })
        })
    });

    group.bench_function("get_available_actions", |b| {
        let content_hash = ContentHash([99u8; 32]); // Not cached
        b.iter(|| {
            rt.block_on(async {
                manager
                    .get_available_actions(black_box(&content_hash), black_box(50 * 1024))
                    .await
                    .unwrap()
            })
        })
    });

    group.bench_function("update_statistics", |b| {
        let content_hash = ContentHash([50u8; 32]);
        b.iter(|| {
            rt.block_on(async {
                manager
                    .update_statistics(
                        black_box(&CacheAction::DoNothing),
                        black_box(&content_hash),
                        black_box(100 * 1024),
                        black_box(true),
                    )
                    .await
                    .unwrap()
            })
        })
    });

    group.bench_function("calculate_reward", |b| {
        b.iter(|| {
            rt.block_on(async {
                manager
                    .calculate_reward(
                        black_box(&CacheAction::Cache(ContentHash([1u8; 32]))),
                        black_box(false),
                        black_box(0.7),
                        black_box(0.8),
                    )
                    .await
            })
        })
    });

    group.finish();
}

fn benchmark_decision_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("decision_latency");
    let rt = Runtime::new().unwrap();

    for cache_size in [1, 10, 100, 1000].iter() {
        let manager = create_test_manager(100 * 1024 * 1024); // 100MB

        // Populate cache with different numbers of items
        rt.block_on(async {
            for i in 0..*cache_size {
                let content_hash = ContentHash([(i % 256) as u8; 32]);
                manager
                    .update_statistics(
                        &CacheAction::Cache(content_hash.clone()),
                        &content_hash,
                        10 * 1024, // 10KB each
                        false,
                    )
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("items_{}", cache_size)),
            cache_size,
            |b, _| {
                let content_hash = ContentHash([255u8; 32]);
                b.iter(|| {
                    rt.block_on(async {
                        // Full decision pipeline
                        let state = manager.get_current_state(&content_hash).await.unwrap();
                        let actions = manager
                            .get_available_actions(&content_hash, 10 * 1024)
                            .await
                            .unwrap();
                        let action = manager.select_action(&state, actions).await.unwrap();
                        black_box(action);
                    })
                })
            },
        );
    }

    group.finish();
}

fn benchmark_learning_convergence(c: &mut Criterion) {
    let mut group = c.benchmark_group("learning_convergence");
    group.measurement_time(std::time::Duration::from_secs(10));

    let rt = Runtime::new().unwrap();

    for learning_rate in [0.01, 0.1, 0.5].iter() {
        let config = QLearningConfig {
            learning_rate: *learning_rate,
            ..Default::default()
        };
        let manager = QLearningCacheManager::new(config, 10 * 1024 * 1024);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("lr_{}", learning_rate)),
            learning_rate,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        // Simulate 100 learning iterations
                        for _ in 0..100 {
                            let state = StateVector::from_metrics(
                                rand::random::<f64>(),
                                rand::random::<f64>() * 100.0,
                                rand::random::<u64>() % 3600,
                                rand::random::<u64>() % 1_000_000,
                            );

                            let experience = Experience {
                                state,
                                action: CacheAction::DoNothing,
                                reward: rand::random::<f64>() * 2.0 - 0.5,
                                next_state: state,
                                terminal: false,
                            };

                            manager.add_experience(experience).await.unwrap();
                        }
                    })
                })
            },
        );
    }

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    let rt = Runtime::new().unwrap();

    // Benchmark memory scaling with state space size
    for num_states in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("states_{}", num_states)),
            num_states,
            |b, &num| {
                b.iter(|| {
                    let manager = create_test_manager(100 * 1024 * 1024);

                    rt.block_on(async {
                        // Populate Q-table with many states
                        for i in 0..num {
                            let state = StateVector {
                                utilization_bucket: (i % 11) as u8,
                                frequency_bucket: ((i / 11) % 6) as u8,
                                recency_bucket: ((i / 66) % 6) as u8,
                                content_size_bucket: ((i / 396) % 5) as u8,
                            };

                            for action in
                                &[ActionType::Cache, ActionType::Evict, ActionType::DoNothing]
                            {
                                manager
                                    .update_q_value(&state, *action, rand::random(), &state, false)
                                    .await
                                    .unwrap();
                            }
                        }

                        // Return Q-table size for memory estimation
                        let q_table_size = manager.q_table.read().await.len();
                        black_box(q_table_size);
                    })
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_state_discretization,
    benchmark_q_learning_operations,
    benchmark_action_selection,
    benchmark_experience_replay,
    benchmark_cache_operations,
    benchmark_decision_latency,
    benchmark_learning_convergence,
    benchmark_memory_usage
);
criterion_main!(benches);
