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

//! Production Module Performance Benchmarks
//!
//! Benchmarks for production hardening features including resource management,
//! rate limiting, health checks, and performance monitoring.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use p2p_foundation::production::{ProductionConfig, RateLimitConfig, ResourceManager};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark resource manager operations
fn resource_manager_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resource_manager");

    // Benchmark resource manager creation
    group.bench_function("resource_manager_creation", |b| {
        b.iter(|| {
            let config = ProductionConfig::default();
            let manager = ResourceManager::new(black_box(config));
            black_box(manager)
        });
    });

    // Benchmark rate limiting checks
    let config = ProductionConfig {
        rate_limits: RateLimitConfig {
            dht_ops_per_sec: 100,
            mcp_calls_per_sec: 50,
            messages_per_sec: 200,
            burst_capacity: 10,
            window_duration: Duration::from_secs(1),
        },
        ..ProductionConfig::default()
    };
    let manager = Arc::new(ResourceManager::new(config));

    rt.block_on(async {
        manager.start().await.unwrap();
    });

    group.bench_function("rate_limit_check_dht", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                let result = manager.check_rate_limit("benchmark_peer", "dht").await;
                black_box(result)
            })
        });
    });

    group.bench_function("rate_limit_check_mcp", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                let result = manager.check_rate_limit("benchmark_peer", "mcp").await;
                black_box(result)
            })
        });
    });

    group.bench_function("rate_limit_check_message", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                let result = manager.check_rate_limit("benchmark_peer", "message").await;
                black_box(result)
            })
        });
    });

    // Benchmark bandwidth recording
    group.bench_function("bandwidth_recording", |b| {
        let manager = manager.clone();
        b.iter(|| {
            manager.record_bandwidth(black_box(1024), black_box(2048));
        });
    });

    // Benchmark metrics collection
    group.bench_function("metrics_collection", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                let metrics = manager.get_metrics().await;
                black_box(metrics)
            })
        });
    });

    // Benchmark health check
    group.bench_function("health_check", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                let result = manager.health_check().await;
                black_box(result)
            })
        });
    });

    // Benchmark connection acquisition
    group.bench_function("connection_acquisition", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                if let Ok(guard) = manager.acquire_connection().await {
                    drop(guard); // Release immediately for benchmark
                }
            })
        });
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        let _ = manager.shutdown().await;
    });
}

/// Benchmark concurrent resource manager operations
fn concurrent_resource_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_resource_operations");

    let config = ProductionConfig {
        max_connections: 100,
        rate_limits: RateLimitConfig {
            dht_ops_per_sec: 1000,
            mcp_calls_per_sec: 500,
            messages_per_sec: 2000,
            burst_capacity: 50,
            window_duration: Duration::from_secs(1),
        },
        ..ProductionConfig::default()
    };
    let manager = Arc::new(ResourceManager::new(config));

    rt.block_on(async {
        manager.start().await.unwrap();
    });

    // Benchmark concurrent rate limit checks
    for concurrency in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_rate_checks", concurrency),
            concurrency,
            |b, &concurrency| {
                let manager = manager.clone();
                b.iter(|| {
                    let manager = manager.clone();
                    rt.block_on(async move {
                        let handles: Vec<_> = (0..concurrency)
                            .map(|i| {
                                let manager = manager.clone();
                                tokio::spawn(async move {
                                    let peer_id = format!("peer_{}", i);
                                    let operation = match i % 3 {
                                        0 => "dht",
                                        1 => "mcp",
                                        _ => "message",
                                    };
                                    manager.check_rate_limit(&peer_id, operation).await
                                })
                            })
                            .collect();

                        for handle in handles {
                            let _ = handle.await;
                        }
                    })
                });
            },
        );
    }

    // Benchmark concurrent connection acquisition
    for concurrency in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_connections", concurrency),
            concurrency,
            |b, &concurrency| {
                let manager = manager.clone();
                b.iter(|| {
                    let manager = manager.clone();
                    rt.block_on(async move {
                        let handles: Vec<_> = (0..concurrency)
                            .map(|_| {
                                let manager = manager.clone();
                                tokio::spawn(async move {
                                    if let Ok(guard) = manager.acquire_connection().await {
                                        // Hold connection briefly then release
                                        tokio::time::sleep(Duration::from_millis(1)).await;
                                        drop(guard);
                                    }
                                })
                            })
                            .collect();

                        for handle in handles {
                            let _ = handle.await;
                        }
                    })
                });
            },
        );
    }

    // Benchmark concurrent bandwidth recording
    group.bench_function("concurrent_bandwidth_recording", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                let handles: Vec<_> = (0..10)
                    .map(|i| {
                        let manager = manager.clone();
                        tokio::spawn(async move {
                            manager.record_bandwidth(1024 + i * 100, 2048 + i * 200);
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            })
        });
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        let _ = manager.shutdown().await;
    });
}

/// Benchmark production configuration operations
fn config_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("production_config");

    // Benchmark config creation
    group.bench_function("config_default", |b| {
        b.iter(|| {
            let config = ProductionConfig::default();
            black_box(config)
        });
    });

    // Benchmark config cloning
    let config = ProductionConfig::default();
    group.bench_function("config_clone", |b| {
        b.iter(|| {
            let cloned = black_box(&config).clone();
            black_box(cloned)
        });
    });

    // Benchmark rate limit config creation
    group.bench_function("rate_limit_config_default", |b| {
        b.iter(|| {
            let config = RateLimitConfig::default();
            black_box(config)
        });
    });

    // Benchmark config serialization
    group.bench_function("config_serialization", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(black_box(&config)).unwrap();
            black_box(serialized)
        });
    });

    // Benchmark config deserialization
    let serialized = serde_json::to_vec(&config).unwrap();
    group.bench_function("config_deserialization", |b| {
        b.iter(|| {
            let config: ProductionConfig = serde_json::from_slice(black_box(&serialized)).unwrap();
            black_box(config)
        });
    });

    group.finish();
}

/// Benchmark memory and performance tracking
fn performance_tracking_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("performance_tracking");

    let config = ProductionConfig {
        enable_performance_tracking: true,
        metrics_interval: Duration::from_millis(100),
        ..ProductionConfig::default()
    };
    let manager = Arc::new(ResourceManager::new(config));

    rt.block_on(async {
        manager.start().await.unwrap();
    });

    // Simulate some activity
    rt.block_on(async {
        for i in 0..50 {
            manager.record_bandwidth(i * 100, i * 150);
            if i % 10 == 0 {
                let _ = manager.acquire_connection().await;
            }
        }
    });

    // Benchmark metrics retrieval under load
    group.bench_function("metrics_under_load", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                // Simulate concurrent activity while measuring
                let activity_handle = tokio::spawn({
                    let manager = manager.clone();
                    async move {
                        for i in 0..10 {
                            manager.record_bandwidth(i * 50, i * 75);
                            let _ = manager.check_rate_limit("load_test", "dht").await;
                        }
                    }
                });

                // Get metrics while activity is happening
                let metrics = manager.get_metrics().await;
                let _ = activity_handle.await;

                black_box(metrics)
            })
        });
    });

    // Benchmark health check under load
    group.bench_function("health_check_under_load", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                // Simulate load
                let load_handles: Vec<_> = (0..5)
                    .map(|i| {
                        let manager = manager.clone();
                        tokio::spawn(async move {
                            manager.record_bandwidth(i * 200, i * 300);
                            let _ = manager
                                .check_rate_limit(&format!("peer_{}", i), "mcp")
                                .await;
                        })
                    })
                    .collect();

                // Health check while under load
                let health_result = manager.health_check().await;

                for handle in load_handles {
                    let _ = handle.await;
                }

                black_box(health_result)
            })
        });
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        let _ = manager.shutdown().await;
    });
}

/// Benchmark memory usage patterns
fn memory_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_usage");

    // Benchmark memory usage of multiple resource managers
    for manager_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("multiple_managers", manager_count),
            manager_count,
            |b, &manager_count| {
                b.iter(|| {
                    let mut managers = Vec::new();

                    for _ in 0..manager_count {
                        let config = ProductionConfig {
                            max_connections: 10,
                            max_memory_bytes: 10 * 1024 * 1024, // 10MB each
                            ..ProductionConfig::default()
                        };
                        let manager = ResourceManager::new(config);
                        managers.push(manager);
                    }

                    black_box(managers)
                });
            },
        );
    }

    // Benchmark resource usage tracking
    let config = ProductionConfig::default();
    let manager = Arc::new(ResourceManager::new(config));

    rt.block_on(async {
        manager.start().await.unwrap();
    });

    group.bench_function("resource_usage_tracking", |b| {
        let manager = manager.clone();
        b.iter(|| {
            let manager = manager.clone();
            rt.block_on(async move {
                // Simulate varying resource usage patterns
                for pattern in 0..10 {
                    match pattern % 3 {
                        0 => {
                            // High bandwidth usage
                            manager.record_bandwidth(10_000, 15_000);
                        }
                        1 => {
                            // High connection usage
                            let _guard = manager.acquire_connection().await;
                        }
                        _ => {
                            // High rate limiting usage
                            for i in 0..5 {
                                let _ = manager
                                    .check_rate_limit(&format!("burst_{}", i), "dht")
                                    .await;
                            }
                        }
                    }
                }

                let metrics = manager.get_metrics().await;
                black_box(metrics)
            })
        });
    });

    group.finish();

    // Cleanup
    rt.block_on(async {
        let _ = manager.shutdown().await;
    });
}

criterion_group!(
    production_benches,
    resource_manager_benchmarks,
    concurrent_resource_benchmarks,
    config_benchmarks,
    performance_tracking_benchmarks,
    memory_benchmarks
);

criterion_main!(production_benches);
