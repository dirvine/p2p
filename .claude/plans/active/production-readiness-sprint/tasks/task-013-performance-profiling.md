# Task 013: Performance Profiling and Optimization

## Overview
Profile the system to identify performance bottlenecks introduced by error handling and monitoring. Optimize critical paths while maintaining safety.

## Acceptance Criteria
- [ ] Performance profiling completed
- [ ] No regression > 5% from baseline
- [ ] Hot paths optimized
- [ ] Memory usage analyzed
- [ ] Profiling integrated into CI

## Technical Details

### 1. Profiling Infrastructure

```rust
// benches/production_benchmarks.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use p2p_core::*;

fn bench_error_handling(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("error_handling");
    
    // Benchmark successful operations
    group.bench_function("dht_get_success", |b| {
        b.to_async(&runtime).iter(|| async {
            let dht = create_test_dht().await;
            dht.store(b"key", b"value").await.unwrap();
            dht.get(b"key").await.unwrap()
        })
    });
    
    // Benchmark error paths
    group.bench_function("dht_get_not_found", |b| {
        b.to_async(&runtime).iter(|| async {
            let dht = create_test_dht().await;
            dht.get(b"nonexistent").await.ok()
        })
    });
    
    group.finish();
}

fn bench_monitoring_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("monitoring");
    
    // Without metrics
    group.bench_function("send_no_metrics", |b| {
        b.iter(|| {
            let msg = create_test_message();
            send_without_metrics(msg);
        })
    });
    
    // With metrics
    group.bench_function("send_with_metrics", |b| {
        b.iter(|| {
            let msg = create_test_message();
            send_with_metrics(msg);
        })
    });
    
    group.finish();
}
```

### 2. Memory Profiling

```rust
// src/bin/memory_profile.rs
use jemalloc_ctl::{stats, epoch};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

async fn profile_memory_usage() {
    // Force collection
    epoch::advance().unwrap();
    
    let baseline = stats::allocated::read().unwrap();
    println!("Baseline memory: {} bytes", baseline);
    
    // Create network with error handling
    let network = Network::new(Config::default()).await.unwrap();
    
    epoch::advance().unwrap();
    let after_network = stats::allocated::read().unwrap();
    println!("After network creation: {} bytes (+{})", 
        after_network, 
        after_network - baseline
    );
    
    // Perform operations
    for i in 0..1000 {
        let _ = network.connect(&format!("peer-{}", i)).await;
    }
    
    epoch::advance().unwrap();
    let after_operations = stats::allocated::read().unwrap();
    println!("After 1000 operations: {} bytes (+{})",
        after_operations,
        after_operations - after_network
    );
}
```

### 3. CPU Profiling

```rust
// Profile with perf or flamegraph
use pprof::{ProfilerGuard, Report};

fn profile_cpu_usage() -> Result<Report> {
    let guard = ProfilerGuard::new(100)?;
    
    // Run performance-critical code
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let network = create_test_network(10).await;
        
        // Stress test
        for _ in 0..10000 {
            let _ = network.broadcast_message(b"test").await;
        }
    });
    
    guard.report().build()
}
```

### 4. Hot Path Optimizations

#### Optimization 1: Error Type Size
```rust
// Before: Large error enums
#[derive(Error, Debug)]
pub enum NetworkError {
    Io(#[from] std::io::Error),        // 16 bytes
    Timeout(Duration),                   // 16 bytes
    LargeVariant(String, Vec<u8>, i32), // 56 bytes!
}

// After: Box large variants
#[derive(Error, Debug)]
pub enum NetworkError {
    Io(#[from] std::io::Error),
    Timeout(Duration),
    LargeVariant(Box<LargeError>),      // 8 bytes
}

struct LargeError {
    message: String,
    data: Vec<u8>,
    code: i32,
}
```

#### Optimization 2: Metrics Collection
```rust
// Before: Metrics on every operation
pub async fn send(&self, msg: Message) -> Result<()> {
    METRICS.messages_sent.inc();
    METRICS.bytes_sent.inc_by(msg.len() as u64);
    self.send_internal(msg).await
}

// After: Batch metrics updates
pub struct MetricsBatcher {
    messages: AtomicU64,
    bytes: AtomicU64,
    last_flush: Instant,
}

impl MetricsBatcher {
    pub fn record(&self, msg_size: usize) {
        self.messages.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(msg_size as u64, Ordering::Relaxed);
    }
    
    pub async fn flush_if_needed(&self) {
        if self.last_flush.elapsed() > Duration::from_secs(1) {
            let messages = self.messages.swap(0, Ordering::Relaxed);
            let bytes = self.bytes.swap(0, Ordering::Relaxed);
            
            METRICS.messages_sent.inc_by(messages);
            METRICS.bytes_sent.inc_by(bytes);
            
            self.last_flush = Instant::now();
        }
    }
}
```

#### Optimization 3: Allocation Reduction
```rust
// Before: Allocating for error context
pub fn validate_message(msg: &[u8]) -> Result<()> {
    if msg.len() > MAX_SIZE {
        return Err(anyhow!("Message too large: {} bytes", msg.len()));
    }
    Ok(())
}

// After: Pre-allocated error messages
lazy_static! {
    static ref MSG_TOO_LARGE: NetworkError = 
        NetworkError::MessageTooLarge { max: MAX_SIZE };
}

pub fn validate_message(msg: &[u8]) -> Result<(), &'static NetworkError> {
    if msg.len() > MAX_SIZE {
        return Err(&MSG_TOO_LARGE);
    }
    Ok(())
}
```

### 5. Async Performance

```rust
// Optimize async boundaries
#[inline(always)]
pub async fn get_cached(&self, key: &[u8]) -> Option<Vec<u8>> {
    // Fast path - no async if in cache
    if let Some(value) = self.cache.get(key) {
        return Some(value.clone());
    }
    
    // Slow path - async DHT lookup
    self.get_from_dht(key).await
}

// Use FuturesUnordered for concurrent operations
pub async fn batch_get(&self, keys: Vec<Vec<u8>>) -> Vec<Option<Vec<u8>>> {
    use futures::stream::{FuturesUnordered, StreamExt};
    
    let mut futures = FuturesUnordered::new();
    
    for key in keys {
        futures.push(self.get(key));
    }
    
    let mut results = Vec::new();
    while let Some(result) = futures.next().await {
        results.push(result);
    }
    
    results
}
```

### 6. Continuous Profiling

```toml
# .github/workflows/performance.yml
name: Performance Regression Check

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Run benchmarks
        run: |
          cargo bench --bench production_benchmarks -- --save-baseline pr
          
      - name: Compare with main
        run: |
          git checkout main
          cargo bench --bench production_benchmarks -- --save-baseline main
          cargo bench --bench production_benchmarks -- --baseline main --compare pr
```

## Testing Requirements
- Benchmark suite covering all critical paths
- Memory leak detection
- Profile under realistic load
- Compare against baseline
- Document optimization decisions

## Dependencies
- Previous: All implementation tasks
- External: criterion, pprof, flamegraph

## Time Estimate
- Profiling setup: 3 hours
- Performance analysis: 4 hours
- Optimizations: 6 hours
- CI integration: 2 hours
- Total: 15 hours

## Definition of Done
- [ ] All benchmarks passing
- [ ] No performance regression > 5%
- [ ] Memory usage acceptable
- [ ] Hot paths optimized
- [ ] CI performance tracking active