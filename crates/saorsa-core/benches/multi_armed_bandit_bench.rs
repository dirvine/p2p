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

//! Performance benchmarks for Multi-Armed Bandit routing optimization

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use saorsa_core::adaptive::{
    ContentType, MABConfig, MultiArmedBandit, NodeId, Outcome, RouteId, StrategyChoice,
};
use std::time::Duration;
use tokio::runtime::Runtime;

fn create_test_mab() -> MultiArmedBandit {
    let config = MABConfig {
        epsilon: 0.1,
        min_samples: 10,
        decay_factor: 0.99,
        storage_path: None,
        persist_interval: Duration::from_secs(300),
        max_stats_age: Duration::from_secs(3600),
    };

    let rt = Runtime::new().unwrap();
    rt.block_on(async { MultiArmedBandit::new(config).await.unwrap() })
}

fn benchmark_route_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("route_selection");
    let rt = Runtime::new().unwrap();

    for num_strategies in [2, 4, 8].iter() {
        let mab = create_test_mab();
        let destination = NodeId::from_bytes([1u8; 32]);

        let strategies: Vec<StrategyChoice> = match *num_strategies {
            2 => vec![StrategyChoice::Kademlia, StrategyChoice::Hyperbolic],
            4 => vec![
                StrategyChoice::Kademlia,
                StrategyChoice::Hyperbolic,
                StrategyChoice::TrustPath,
                StrategyChoice::SOMRegion,
            ],
            _ => vec![
                StrategyChoice::Kademlia,
                StrategyChoice::Hyperbolic,
                StrategyChoice::TrustPath,
                StrategyChoice::SOMRegion,
                StrategyChoice::Kademlia, // Duplicates to reach 8
                StrategyChoice::Hyperbolic,
                StrategyChoice::TrustPath,
                StrategyChoice::SOMRegion,
            ],
        };

        // Pre-populate with some statistics
        rt.block_on(async {
            for _ in 0..100 {
                for strategy in &strategies {
                    let route_id = RouteId::new(destination.clone(), *strategy);
                    let outcome = Outcome {
                        success: rand::random(),
                        latency_ms: 50,
                        hops: 3,
                    };
                    mab.update_route(&route_id, ContentType::DHTLookup, &outcome)
                        .await
                        .unwrap();
                }
            }
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(num_strategies),
            num_strategies,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let decision = mab
                            .select_route(
                                black_box(&destination),
                                black_box(ContentType::DHTLookup),
                                black_box(&strategies),
                            )
                            .await
                            .unwrap();
                        black_box(decision);
                    })
                })
            },
        );
    }
    group.finish();
}

fn benchmark_route_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("route_update");
    let rt = Runtime::new().unwrap();
    let mab = create_test_mab();
    let destination = NodeId::from_bytes([1u8; 32]);

    group.bench_function("single_update", |b| {
        b.iter(|| {
            rt.block_on(async {
                let route_id = RouteId::new(destination.clone(), StrategyChoice::Kademlia);
                let outcome = Outcome {
                    success: black_box(true),
                    latency_ms: black_box(50),
                    hops: black_box(3),
                };
                mab.update_route(
                    black_box(&route_id),
                    black_box(ContentType::DHTLookup),
                    black_box(&outcome),
                )
                .await
                .unwrap();
            })
        })
    });

    group.finish();
}

fn benchmark_thompson_sampling_convergence(c: &mut Criterion) {
    let mut group = c.benchmark_group("thompson_sampling_convergence");
    let rt = Runtime::new().unwrap();

    for num_iterations in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_iterations),
            num_iterations,
            |b, &num_iter| {
                b.iter(|| {
                    rt.block_on(async {
                        let mab = create_test_mab();
                        let destination = NodeId::from_bytes([1u8; 32]);
                        let strategies = vec![StrategyChoice::Kademlia, StrategyChoice::Hyperbolic];

                        // Simulate learning process
                        for _ in 0..num_iter {
                            let decision = mab
                                .select_route(&destination, ContentType::DHTLookup, &strategies)
                                .await
                                .unwrap();

                            // Simulate different success rates
                            let success = match decision.route_id.strategy {
                                StrategyChoice::Kademlia => rand::random::<f64>() < 0.8,
                                _ => rand::random::<f64>() < 0.3,
                            };

                            let outcome = Outcome {
                                success,
                                latency_ms: 50,
                                hops: 3,
                            };

                            mab.update_route(&decision.route_id, ContentType::DHTLookup, &outcome)
                                .await
                                .unwrap();
                        }

                        black_box(mab.get_metrics().await);
                    })
                })
            },
        );
    }

    group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    let rt = Runtime::new().unwrap();

    for num_concurrent in [1, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_concurrent),
            num_concurrent,
            |b, &num| {
                b.iter(|| {
                    rt.block_on(async {
                        let mab = create_test_mab();
                        let strategies = vec![
                            StrategyChoice::Kademlia,
                            StrategyChoice::Hyperbolic,
                            StrategyChoice::TrustPath,
                            StrategyChoice::SOMRegion,
                        ];

                        let mut handles = vec![];

                        for i in 0..num {
                            let mab_clone = &mab;
                            let strategies_clone = strategies.clone();

                            let handle = tokio::spawn(async move {
                                let destination = NodeId::from_bytes([i as u8; 32]);

                                for _ in 0..10 {
                                    let decision = mab_clone
                                        .select_route(
                                            &destination,
                                            ContentType::DHTLookup,
                                            &strategies_clone,
                                        )
                                        .await
                                        .unwrap();

                                    let outcome = Outcome {
                                        success: rand::random(),
                                        latency_ms: 50,
                                        hops: 3,
                                    };

                                    mab_clone
                                        .update_route(
                                            &decision.route_id,
                                            ContentType::DHTLookup,
                                            &outcome,
                                        )
                                        .await
                                        .unwrap();
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    })
                })
            },
        );
    }

    group.finish();
}

fn benchmark_statistics_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("statistics_size");
    let rt = Runtime::new().unwrap();

    for num_routes in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_routes),
            num_routes,
            |b, &num| {
                b.iter(|| {
                    rt.block_on(async {
                        let mab = create_test_mab();

                        // Populate with many routes
                        for i in 0..num {
                            let destination = NodeId::from_bytes([(i % 256) as u8; 32]);
                            let strategy = match i % 4 {
                                0 => StrategyChoice::Kademlia,
                                1 => StrategyChoice::Hyperbolic,
                                2 => StrategyChoice::TrustPath,
                                _ => StrategyChoice::SOMRegion,
                            };
                            let content_type = match i % 4 {
                                0 => ContentType::DHTLookup,
                                1 => ContentType::DataRetrieval,
                                2 => ContentType::RealtimeMessage,
                                _ => ContentType::ComputeRequest,
                            };

                            let route_id = RouteId::new(destination, strategy);
                            let outcome = Outcome {
                                success: true,
                                latency_ms: 50,
                                hops: 3,
                            };

                            mab.update_route(&route_id, content_type, &outcome)
                                .await
                                .unwrap();
                        }

                        // Measure retrieval time
                        let stats = mab.get_all_statistics().await;
                        black_box(stats.len());
                    })
                })
            },
        );
    }

    group.finish();
}

fn benchmark_decision_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("decision_overhead");
    let rt = Runtime::new().unwrap();

    // Compare MAB decision vs simple random selection
    group.bench_function("mab_decision", |b| {
        let mab = create_test_mab();
        let destination = NodeId::from_bytes([1u8; 32]);
        let strategies = vec![
            StrategyChoice::Kademlia,
            StrategyChoice::Hyperbolic,
            StrategyChoice::TrustPath,
            StrategyChoice::SOMRegion,
        ];

        // Pre-populate with statistics
        rt.block_on(async {
            for _ in 0..100 {
                for strategy in &strategies {
                    let route_id = RouteId::new(destination.clone(), *strategy);
                    let outcome = Outcome {
                        success: rand::random(),
                        latency_ms: 50,
                        hops: 3,
                    };
                    mab.update_route(&route_id, ContentType::DHTLookup, &outcome)
                        .await
                        .unwrap();
                }
            }
        });

        b.iter(|| {
            rt.block_on(async {
                let decision = mab
                    .select_route(
                        black_box(&destination),
                        black_box(ContentType::DHTLookup),
                        black_box(&strategies),
                    )
                    .await
                    .unwrap();
                black_box(decision);
            })
        })
    });

    group.bench_function("random_selection", |b| {
        let strategies = vec![
            StrategyChoice::Kademlia,
            StrategyChoice::Hyperbolic,
            StrategyChoice::TrustPath,
            StrategyChoice::SOMRegion,
        ];

        b.iter(|| {
            let idx = rand::random::<usize>() % strategies.len();
            let strategy = black_box(strategies[idx]);
            black_box(strategy);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_route_selection,
    benchmark_route_update,
    benchmark_thompson_sampling_convergence,
    benchmark_concurrent_operations,
    benchmark_statistics_size,
    benchmark_decision_overhead
);
criterion_main!(benches);
