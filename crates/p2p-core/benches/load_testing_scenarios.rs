//\! Load Testing Scenarios for Production Readiness
//\! 
//\! These scenarios test the system under various load conditions
//\! to identify bottlenecks and ensure production performance.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use tokio::runtime::Runtime;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use saorsa_core::{
    Config,
    adaptive::{
        coordinator::AdaptiveCoordinator,
        client::AdaptiveClient,
    },
};

/// Load testing framework for stress scenarios
struct LoadTestFramework {
    rt: Runtime,
    coordinators: Vec<Arc<AdaptiveCoordinator>>,
    clients: Vec<Arc<AdaptiveClient>>,
    error_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
}

impl LoadTestFramework {
    fn new() -> Self {
        Self {
            rt: Runtime::new().unwrap(),
            coordinators: Vec::new(),
            clients: Vec::new(),
            error_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
        }
    }
    
    fn setup_cluster(&mut self, node_count: usize) -> anyhow::Result<()> {
        for i in 0..node_count {
            let mut config = Config::default();
            config.network.listen_port = 6000 + i as u16;
            config.storage.replication_factor = 3.min(node_count);
            config.network.max_connections = 1000;
            config.network.connection_timeout = Duration::from_secs(30);
            
            let coordinator = Arc::new(self.rt.block_on(
                AdaptiveCoordinator::new(config.clone())
            )?);
            
            let client = Arc::new(self.rt.block_on(
                AdaptiveClient::new(config)
            )?);
            
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
        
        // Connect clients to coordinators
        for (i, client) in self.clients.iter().enumerate() {
            for (j, _) in self.coordinators.iter().enumerate() {
                if i \!= j {
                    let addr = format\!("127.0.0.1:{}", 6000 + j);
                    let _ = self.rt.block_on(client.connect_to_peer(&addr));
                }
            }
        }
        
        // Allow cluster formation time
        std::thread::sleep(Duration::from_secs(3));
        
        Ok(())
    }
    
    fn sustained_load_test(&mut self, duration: Duration, ops_per_second: usize) -> anyhow::Result<LoadTestResults> {
        let start_time = Instant::now();
        let interval = Duration::from_nanos(1_000_000_000 / ops_per_second as u64);
        
        let mut operation_count = 0;
        let mut last_op_time = start_time;
        
        while start_time.elapsed() < duration {
            let now = Instant::now();
            if now.duration_since(last_op_time) >= interval {
                self.execute_operation(operation_count)?;
                operation_count += 1;
                last_op_time = now;
            }
            
            // Small sleep to prevent busy waiting
            std::thread::sleep(Duration::from_millis(1));
        }
        
        Ok(LoadTestResults {
            duration: start_time.elapsed(),
            total_operations: operation_count,
            successful_operations: self.success_count.load(Ordering::Relaxed),
            failed_operations: self.error_count.load(Ordering::Relaxed),
        })
    }
    
    fn burst_load_test(&mut self, burst_size: usize, burst_count: usize) -> anyhow::Result<LoadTestResults> {
        let start_time = Instant::now();
        let mut total_operations = 0;
        
        for burst_id in 0..burst_count {
            println\!("Executing burst {} of {}", burst_id + 1, burst_count);
            
            // Execute burst of operations
            let handles: Vec<_> = (0..burst_size).map(|op_id| {
                let client = self.clients[op_id % self.clients.len()].clone();
                let success_count = self.success_count.clone();
                let error_count = self.error_count.clone();
                
                tokio::spawn(async move {
                    let key = format\!("burst_{}_{}", burst_id, op_id);
                    let value = format\!("burst_value_{}_{}", burst_id, op_id).into_bytes();
                    
                    match client.store(&key, value).await {
                        Ok(_) => success_count.fetch_add(1, Ordering::Relaxed),
                        Err(_) => error_count.fetch_add(1, Ordering::Relaxed),
                    };
                })
            }).collect();
            
            self.rt.block_on(async {
                for handle in handles {
                    let _ = handle.await;
                }
            });
            
            total_operations += burst_size;
            
            // Brief pause between bursts
            std::thread::sleep(Duration::from_millis(100));
        }
        
        Ok(LoadTestResults {
            duration: start_time.elapsed(),
            total_operations,
            successful_operations: self.success_count.load(Ordering::Relaxed),
            failed_operations: self.error_count.load(Ordering::Relaxed),
        })
    }
    
    fn ramp_up_test(&mut self, max_ops_per_second: usize, ramp_duration: Duration) -> anyhow::Result<LoadTestResults> {
        let start_time = Instant::now();
        let mut total_operations = 0;
        
        while start_time.elapsed() < ramp_duration {
            let elapsed_ratio = start_time.elapsed().as_secs_f64() / ramp_duration.as_secs_f64();
            let current_ops_per_sec = (max_ops_per_second as f64 * elapsed_ratio) as usize;
            
            if current_ops_per_sec > 0 {
                let interval = Duration::from_nanos(1_000_000_000 / current_ops_per_sec as u64);
                
                if total_operations == 0 || start_time.elapsed() >= Duration::from_nanos(total_operations as u64 * interval.as_nanos() as u64) {
                    self.execute_operation(total_operations)?;
                    total_operations += 1;
                }
            }
            
            std::thread::sleep(Duration::from_millis(10));
        }
        
        Ok(LoadTestResults {
            duration: start_time.elapsed(),
            total_operations,
            successful_operations: self.success_count.load(Ordering::Relaxed),
            failed_operations: self.error_count.load(Ordering::Relaxed),
        })
    }
    
    fn execute_operation(&mut self, operation_id: usize) -> anyhow::Result<()> {
        let client = &self.clients[operation_id % self.clients.len()];
        let key = format\!("load_test_{}", operation_id);
        let value = format\!("load_value_{}", operation_id).into_bytes();
        
        match self.rt.block_on(client.store(&key, value)) {
            Ok(_) => {
                self.success_count.fetch_add(1, Ordering::Relaxed);
            },
            Err(_) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        Ok(())
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

#[derive(Debug)]
struct LoadTestResults {
    duration: Duration,
    total_operations: usize,
    successful_operations: u64,
    failed_operations: u64,
}

impl LoadTestResults {
    fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            return 0.0;
        }
        self.successful_operations as f64 / self.total_operations as f64 * 100.0
    }
    
    fn throughput(&self) -> f64 {
        self.successful_operations as f64 / self.duration.as_secs_f64()
    }
}

fn sustained_load_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_load");
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(10);
    
    for ops_per_sec in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("sustained_ops", ops_per_sec),
            ops_per_sec,
            |b, &ops_per_sec| {
                b.iter_custom(|_iters| {
                    let mut framework = LoadTestFramework::new();
                    framework.setup_cluster(3).unwrap();
                    
                    let start = Instant::now();
                    let results = framework.sustained_load_test(
                        Duration::from_secs(30), 
                        ops_per_sec
                    ).unwrap();
                    
                    println\!("Sustained load results: {} ops/sec, {:.1}% success rate", 
                            results.throughput(), results.success_rate());
                    
                    framework.cleanup().unwrap();
                    start.elapsed()
                });
            },
        );
    }
    
    group.finish();
}

fn burst_load_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("burst_load");
    group.measurement_time(Duration::from_secs(45));
    group.sample_size(10);
    
    for burst_size in [50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("burst_size", burst_size),
            burst_size,
            |b, &burst_size| {
                b.iter_custom(|_iters| {
                    let mut framework = LoadTestFramework::new();
                    framework.setup_cluster(4).unwrap();
                    
                    let start = Instant::now();
                    let results = framework.burst_load_test(burst_size, 5).unwrap();
                    
                    println\!("Burst load results: {} total ops, {:.1}% success rate", 
                            results.total_operations, results.success_rate());
                    
                    framework.cleanup().unwrap();
                    start.elapsed()
                });
            },
        );
    }
    
    group.finish();
}

fn ramp_up_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("ramp_up_load");
    group.measurement_time(Duration::from_secs(90));
    group.sample_size(5);
    
    group.bench_function("ramp_to_1000_ops", |b| {
        b.iter_custom(|_iters| {
            let mut framework = LoadTestFramework::new();
            framework.setup_cluster(5).unwrap();
            
            let start = Instant::now();
            let results = framework.ramp_up_test(1000, Duration::from_secs(60)).unwrap();
            
            println\!("Ramp-up results: {} total ops, {:.1}% success rate, {:.0} ops/sec", 
                    results.total_operations, results.success_rate(), results.throughput());
            
            framework.cleanup().unwrap();
            start.elapsed()
        });
    });
    
    group.finish();
}

criterion_group\!(
    load_testing,
    sustained_load_benchmark,
    burst_load_benchmark,
    ramp_up_benchmark
);

criterion_main\!(load_testing);
