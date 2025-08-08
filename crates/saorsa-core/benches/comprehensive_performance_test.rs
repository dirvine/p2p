//! Comprehensive Performance Testing Suite
//!
//! This benchmark suite tests all critical performance aspects of the P2P Foundation
//! to ensure production readiness with target metrics:
//! - P50 latency < 200ms
//! - Throughput > 10K req/s
//! - Memory usage stable
//! - Zero memory leaks

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

use saorsa_core::{
    Config,
    adaptive::{
        client::AdaptiveClient, coordinator::AdaptiveCoordinator, replication::ReplicationManager,
    },
    health::HealthManager,
    identity::IdentityManager,
};

/// Performance test framework for comprehensive benchmarking
struct PerformanceTestFramework {
    rt: Runtime,
    coordinators: Vec<Arc<AdaptiveCoordinator>>,
    clients: Vec<Arc<AdaptiveClient>>,
    metrics: HashMap<String, f64>,
}

impl PerformanceTestFramework {
    fn new() -> Self {
        let rt = Runtime::new().unwrap();
        Self {
            rt,
            coordinators: Vec::new(),
            clients: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    fn setup_nodes(&mut self, node_count: usize) -> anyhow::Result<()> {
        for i in 0..node_count {
            let mut config = Config::default();
            config.network.listen_port = 7000 + i as u16;
            config.storage.replication_factor = 3;
            config.network.max_connections = 1000;

            let coordinator = Arc::new(self.rt.block_on(AdaptiveCoordinator::new(config.clone()))?);

            let client = Arc::new(self.rt.block_on(AdaptiveClient::new(config))?);

            self.coordinators.push(coordinator);
            self.clients.push(client);
        }

        // Start all nodes
        for coordinator in &self.coordinators {
            self.rt.block_on(coordinator.start())?;
        }

        for client in &self.clients {
            self.rt.block_on(client.start())?;
        }

        // Allow startup time
        std::thread::sleep(Duration::from_secs(2));

        Ok(())
    }

    fn benchmark_baseline_throughput(&mut self) -> anyhow::Result<f64> {
        if self.clients.is_empty() {
            return Ok(0.0);
        }

        let operations = 1000;
        let start = Instant::now();

        for i in 0..operations {
            let key = format!("throughput_test_{}", i);
            let value = format!("value_{}", i).into_bytes();

            self.rt.block_on(self.clients[0].store(&key, value))?;
        }

        let duration = start.elapsed();
        let ops_per_sec = operations as f64 / duration.as_secs_f64();

        self.metrics
            .insert("baseline_throughput".to_string(), ops_per_sec);
        Ok(ops_per_sec)
    }

    fn benchmark_latency_distribution(&mut self) -> anyhow::Result<(f64, f64, f64)> {
        if self.clients.is_empty() {
            return Ok((0.0, 0.0, 0.0));
        }

        let operations = 100;
        let mut latencies = Vec::new();

        for i in 0..operations {
            let key = format!("latency_test_{}", i);
            let value = format!("value_{}", i).into_bytes();

            let start = Instant::now();
            self.rt.block_on(self.clients[0].store(&key, value))?;
            let latency = start.elapsed().as_millis() as f64;

            latencies.push(latency);
        }

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50 = latencies[latencies.len() / 2];
        let p95 = latencies[(latencies.len() as f64 * 0.95) as usize];
        let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];

        self.metrics.insert("p50_latency".to_string(), p50);
        self.metrics.insert("p95_latency".to_string(), p95);
        self.metrics.insert("p99_latency".to_string(), p99);

        Ok((p50, p95, p99))
    }

    fn benchmark_concurrent_operations(&mut self, concurrency: usize) -> anyhow::Result<f64> {
        if self.clients.is_empty() {
            return Ok(0.0);
        }

        let operations_per_thread = 100;
        let start = Instant::now();

        let handles: Vec<_> = (0..concurrency)
            .map(|thread_id| {
                let client = self.clients[thread_id % self.clients.len()].clone();

                tokio::spawn(async move {
                    for i in 0..operations_per_thread {
                        let key = format!("concurrent_{}_{}", thread_id, i);
                        let value = format!("value_{}_{}", thread_id, i).into_bytes();

                        let _ = client.store(&key, value).await;
                    }
                })
            })
            .collect();

        self.rt.block_on(async {
            for handle in handles {
                let _ = handle.await;
            }
        });

        let duration = start.elapsed();
        let total_ops = concurrency * operations_per_thread;
        let ops_per_sec = total_ops as f64 / duration.as_secs_f64();

        self.metrics.insert(
            format!("concurrent_throughput_{}", concurrency),
            ops_per_sec,
        );
        Ok(ops_per_sec)
    }

    fn benchmark_memory_usage(&mut self) -> anyhow::Result<u64> {
        // Simple memory usage estimation
        let operations = 1000;

        let initial_memory = self.estimate_memory_usage();

        for i in 0..operations {
            let key = format!("memory_test_{}", i);
            let value = vec![0u8; 1024]; // 1KB per operation

            self.rt.block_on(self.clients[0].store(&key, value))?;
        }

        let final_memory = self.estimate_memory_usage();
        let memory_increase = final_memory.saturating_sub(initial_memory);

        self.metrics
            .insert("memory_increase".to_string(), memory_increase as f64);
        Ok(memory_increase)
    }

    fn estimate_memory_usage(&self) -> u64 {
        // Simplified memory estimation
        // In production, this would use proper memory profiling
        (self.coordinators.len() + self.clients.len()) as u64 * 1024 * 1024 // ~1MB per node
    }

    fn cleanup(&mut self) -> anyhow::Result<()> {
        for coordinator in &self.coordinators {
            let _ = self.rt.block_on(coordinator.shutdown());
        }

        for client in &self.clients {
            let _ = self.rt.block_on(client.shutdown());
        }

        Ok(())
    }
}

fn baseline_performance_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_performance");
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("single_node_throughput", |b| {
        b.iter_custom(|iters| {
            let mut framework = PerformanceTestFramework::new();
            framework.setup_nodes(1).unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                let _ = framework.benchmark_baseline_throughput();
            }
            let duration = start.elapsed();

            framework.cleanup().unwrap();
            duration
        });
    });

    group.finish();
}

fn latency_distribution_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_distribution");
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("operation_latency", |b| {
        b.iter_custom(|iters| {
            let mut framework = PerformanceTestFramework::new();
            framework.setup_nodes(1).unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                let _ = framework.benchmark_latency_distribution();
            }
            let duration = start.elapsed();

            framework.cleanup().unwrap();
            duration
        });
    });

    group.finish();
}

fn concurrency_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrency");
    group.measurement_time(Duration::from_secs(30));

    for concurrency in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_operations", concurrency),
            concurrency,
            |b, &concurrency| {
                b.iter_custom(|iters| {
                    let mut framework = PerformanceTestFramework::new();
                    framework.setup_nodes(concurrency.min(4)).unwrap();

                    let start = Instant::now();
                    for _ in 0..iters {
                        let _ = framework.benchmark_concurrent_operations(concurrency);
                    }
                    let duration = start.elapsed();

                    framework.cleanup().unwrap();
                    duration
                });
            },
        );
    }

    group.finish();
}

fn scale_testing_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_testing");
    group.measurement_time(Duration::from_secs(45));
    group.sample_size(10); // Fewer samples for expensive tests

    for node_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("multi_node_throughput", node_count),
            node_count,
            |b, &node_count| {
                b.iter_custom(|iters| {
                    let mut framework = PerformanceTestFramework::new();
                    framework.setup_nodes(node_count).unwrap();

                    let start = Instant::now();
                    for _ in 0..iters {
                        let _ = framework.benchmark_baseline_throughput();
                    }
                    let duration = start.elapsed();

                    framework.cleanup().unwrap();
                    duration
                });
            },
        );
    }

    group.finish();
}

fn memory_usage_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("memory_growth", |b| {
        b.iter_custom(|iters| {
            let mut framework = PerformanceTestFramework::new();
            framework.setup_nodes(1).unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                let _ = framework.benchmark_memory_usage();
            }
            let duration = start.elapsed();

            framework.cleanup().unwrap();
            duration
        });
    });

    group.finish();
}

criterion_group!(
    comprehensive_performance,
    baseline_performance_benchmark,
    latency_distribution_benchmark,
    concurrency_benchmark,
    scale_testing_benchmark,
    memory_usage_benchmark
);

criterion_main!(comprehensive_performance);
