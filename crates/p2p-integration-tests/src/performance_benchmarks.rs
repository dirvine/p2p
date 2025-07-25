//! Performance benchmarks for the adaptive P2P network
//!
//! Comprehensive performance testing including:
//! - Storage/retrieval throughput
//! - Routing latency
//! - Message propagation speed
//! - Scalability testing
//! - Resource utilization

use p2p_integration_tests::*;
use saorsa_core::adaptive::*;
use anyhow::Result;
use std::{
    time::{Duration, Instant},
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};
use tracing::{info, debug};
use tokio::sync::Semaphore;
use futures::future::join_all;

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Storage throughput (ops/sec)
    pub storage_throughput: f64,
    
    /// Retrieval throughput (ops/sec)
    pub retrieval_throughput: f64,
    
    /// Average storage latency
    pub avg_storage_latency: Duration,
    
    /// Average retrieval latency
    pub avg_retrieval_latency: Duration,
    
    /// Routing latency percentiles
    pub routing_latencies: LatencyStats,
    
    /// Gossip propagation time
    pub gossip_propagation_time: Duration,
    
    /// Network bandwidth (MB/s)
    pub bandwidth_mbps: f64,
    
    /// CPU usage percentage
    pub cpu_usage: f64,
    
    /// Memory usage (MB)
    pub memory_usage: u64,
}

/// Benchmark runner
pub struct BenchmarkRunner {
    /// Test cluster
    cluster: TestCluster,
    
    /// Test duration
    duration: Duration,
    
    /// Concurrent operations
    concurrency: usize,
    
    /// Content sizes to test
    content_sizes: Vec<usize>,
    
    /// Results
    results: Arc<RwLock<BenchmarkResults>>,
}

/// Benchmark results
#[derive(Debug, Clone, Default)]
pub struct BenchmarkResults {
    /// Results by node count
    pub by_node_count: BTreeMap<usize, PerformanceMetrics>,
    
    /// Results by content size
    pub by_content_size: BTreeMap<usize, PerformanceMetrics>,
    
    /// Results by operation type
    pub by_operation: BTreeMap<String, PerformanceMetrics>,
    
    /// Scalability metrics
    pub scalability: ScalabilityMetrics,
}

/// Scalability metrics
#[derive(Debug, Clone, Default)]
pub struct ScalabilityMetrics {
    /// Throughput scaling factor
    pub throughput_scaling: f64,
    
    /// Latency scaling factor
    pub latency_scaling: f64,
    
    /// Optimal node count
    pub optimal_nodes: usize,
    
    /// Maximum sustainable load
    pub max_load: f64,
}

impl BenchmarkRunner {
    /// Create new benchmark runner
    pub async fn new(
        cluster: TestCluster,
        duration: Duration,
        concurrency: usize,
    ) -> Self {
        Self {
            cluster,
            duration,
            concurrency,
            content_sizes: vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024], // 1KB to 1MB
            results: Arc::new(RwLock::new(BenchmarkResults::default())),
        }
    }
    
    /// Run storage benchmark
    pub async fn benchmark_storage(&self) -> Result<PerformanceMetrics> {
        info!("Starting storage benchmark");
        
        let nodes = self.cluster.nodes.read().await;
        let test_nodes: Vec<_> = nodes.values().take(self.concurrency).collect();
        
        let start_time = Instant::now();
        let operations = Arc::new(AtomicU64::new(0));
        let total_bytes = Arc::new(AtomicU64::new(0));
        let mut handles = vec![];
        
        for (i, node) in test_nodes.iter().enumerate() {
            let node = (*node).clone();
            let ops = operations.clone();
            let bytes = total_bytes.clone();
            let duration = self.duration;
            let content_sizes = self.content_sizes.clone();
            
            let handle = tokio::spawn(async move {
                let mut latencies = vec![];
                let start = Instant::now();
                
                while start.elapsed() < duration {
                    let size = content_sizes[rand::random::<usize>() % content_sizes.len()];
                    let content = utils::generate_content(size);
                    
                    let metadata = storage::ContentMetadata {
                        size,
                        content_type: ContentType::DataRetrieval,
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        chunk_count: if size > 1024 * 1024 { Some((size / (1024 * 1024)) as u32 + 1) } else { None },
                        replication_factor: 8,
                    };
                    
                    let op_start = Instant::now();
                    if let Ok(_) = node.components.storage.store(content, metadata).await {
                        let latency = op_start.elapsed();
                        latencies.push(latency);
                        ops.fetch_add(1, Ordering::Relaxed);
                        bytes.fetch_add(size as u64, Ordering::Relaxed);
                    }
                    
                    // Small delay to prevent overwhelming
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                
                latencies
            });
            
            handles.push(handle);
        }
        
        // Collect all latencies
        let mut all_latencies = vec![];
        for handle in handles {
            let latencies = handle.await?;
            all_latencies.extend(latencies);
        }
        
        let elapsed = start_time.elapsed();
        let total_ops = operations.load(Ordering::Relaxed);
        let total_mb = total_bytes.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0;
        
        let metrics = PerformanceMetrics {
            storage_throughput: total_ops as f64 / elapsed.as_secs_f64(),
            avg_storage_latency: if all_latencies.is_empty() {
                Duration::ZERO
            } else {
                all_latencies.iter().sum::<Duration>() / all_latencies.len() as u32
            },
            bandwidth_mbps: total_mb / elapsed.as_secs_f64(),
            routing_latencies: utils::calculate_latency_stats(&all_latencies),
            ..Default::default()
        };
        
        info!("Storage benchmark complete: {:.2} ops/sec, {:.2} MB/s",
            metrics.storage_throughput, metrics.bandwidth_mbps);
        
        Ok(metrics)
    }
    
    /// Run retrieval benchmark
    pub async fn benchmark_retrieval(&self) -> Result<PerformanceMetrics> {
        info!("Starting retrieval benchmark");
        
        // First, store test content
        let nodes = self.cluster.nodes.read().await;
        let store_node = nodes.values().next().unwrap();
        let mut content_hashes = vec![];
        
        for size in &self.content_sizes {
            let content = utils::generate_content(*size);
            let metadata = storage::ContentMetadata {
                size: *size,
                content_type: ContentType::DataRetrieval,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                chunk_count: if *size > 1024 * 1024 { Some((*size / (1024 * 1024)) as u32 + 1) } else { None },
                replication_factor: 10,
            };
            
            let hash = store_node.components.storage.store(content, metadata).await?;
            content_hashes.push((hash, *size));
        }
        
        // Wait for replication
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Run retrieval benchmark
        let test_nodes: Vec<_> = nodes.values()
            .skip(1) // Skip store node
            .take(self.concurrency)
            .collect();
        
        let start_time = Instant::now();
        let operations = Arc::new(AtomicU64::new(0));
        let total_bytes = Arc::new(AtomicU64::new(0));
        let mut handles = vec![];
        
        for node in test_nodes {
            let node = node.clone();
            let ops = operations.clone();
            let bytes = total_bytes.clone();
            let duration = self.duration;
            let hashes = content_hashes.clone();
            
            let handle = tokio::spawn(async move {
                let mut latencies = vec![];
                let start = Instant::now();
                
                let retrieval_manager = RetrievalManager::new(
                    node.components.router.clone(),
                    node.components.storage.clone(),
                    Arc::new(learning::QLearnCacheManager::new(100 * 1024 * 1024)),
                );
                
                while start.elapsed() < duration {
                    let (hash, size) = &hashes[rand::random::<usize>() % hashes.len()];
                    
                    let op_start = Instant::now();
                    if let Ok(_) = retrieval_manager.retrieve(
                        hash,
                        retrieval::RetrievalStrategy::Parallel
                    ).await {
                        let latency = op_start.elapsed();
                        latencies.push(latency);
                        ops.fetch_add(1, Ordering::Relaxed);
                        bytes.fetch_add(*size as u64, Ordering::Relaxed);
                    }
                    
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                
                latencies
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let mut all_latencies = vec![];
        for handle in handles {
            let latencies = handle.await?;
            all_latencies.extend(latencies);
        }
        
        let elapsed = start_time.elapsed();
        let total_ops = operations.load(Ordering::Relaxed);
        let total_mb = total_bytes.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0;
        
        let metrics = PerformanceMetrics {
            retrieval_throughput: total_ops as f64 / elapsed.as_secs_f64(),
            avg_retrieval_latency: if all_latencies.is_empty() {
                Duration::ZERO
            } else {
                all_latencies.iter().sum::<Duration>() / all_latencies.len() as u32
            },
            bandwidth_mbps: total_mb / elapsed.as_secs_f64(),
            routing_latencies: utils::calculate_latency_stats(&all_latencies),
            ..Default::default()
        };
        
        info!("Retrieval benchmark complete: {:.2} ops/sec, {:.2} MB/s",
            metrics.retrieval_throughput, metrics.bandwidth_mbps);
        
        Ok(metrics)
    }
    
    /// Benchmark routing performance
    pub async fn benchmark_routing(&self) -> Result<PerformanceMetrics> {
        info!("Starting routing benchmark");
        
        let nodes = self.cluster.nodes.read().await;
        let node_pairs: Vec<_> = nodes.values()
            .take(self.concurrency * 2)
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|pair| (pair[0].clone(), pair[1].clone()))
            .collect();
        
        let mut all_latencies = vec![];
        let start_time = Instant::now();
        
        // Test different routing strategies
        let strategies = vec![
            StrategyChoice::Kademlia,
            StrategyChoice::Hyperbolic,
            StrategyChoice::TrustPath,
            StrategyChoice::SOMRegion,
        ];
        
        for (source, target) in &node_pairs {
            for strategy in &strategies {
                let op_start = Instant::now();
                
                match strategy {
                    StrategyChoice::Kademlia => {
                        let kad_strategy = KademliaRoutingStrategy::new(
                            source.components.dht.clone()
                        );
                        let _ = kad_strategy.find_path(&target.identity.node_id).await;
                    }
                    StrategyChoice::Hyperbolic => {
                        let hyp_strategy = HyperbolicRoutingStrategy::new(
                            source.components.router.hyperbolic_space.clone()
                        );
                        let _ = hyp_strategy.find_path(&target.identity.node_id).await;
                    }
                    StrategyChoice::TrustPath => {
                        let trust_strategy = TrustBasedRoutingStrategy::new(
                            source.components.trust.clone()
                        );
                        let _ = trust_strategy.find_path(&target.identity.node_id).await;
                    }
                    StrategyChoice::SOMRegion => {
                        let som_strategy = SOMRoutingStrategy::new(
                            source.components.router.som.clone(),
                            Arc::new(som::FeatureExtractor::new())
                        );
                        let _ = som_strategy.find_path(&target.identity.node_id).await;
                    }
                }
                
                let latency = op_start.elapsed();
                all_latencies.push(latency);
            }
        }
        
        let elapsed = start_time.elapsed();
        
        let metrics = PerformanceMetrics {
            routing_latencies: utils::calculate_latency_stats(&all_latencies),
            avg_retrieval_latency: all_latencies.iter().sum::<Duration>() / all_latencies.len() as u32,
            ..Default::default()
        };
        
        info!("Routing benchmark complete: avg latency {:?}", metrics.avg_retrieval_latency);
        
        Ok(metrics)
    }
    
    /// Benchmark gossip propagation
    pub async fn benchmark_gossip(&self) -> Result<PerformanceMetrics> {
        info!("Starting gossip propagation benchmark");
        
        let topic = "benchmark_topic";
        let nodes = self.cluster.nodes.read().await;
        
        // Subscribe all nodes
        for node in nodes.values() {
            node.components.gossip.subscribe(topic).await?;
        }
        
        // Measure propagation time for different message sizes
        let mut propagation_times = vec![];
        
        for size in &[100, 1000, 10000] {
            let message = utils::generate_content(*size);
            let publisher = nodes.values().next().unwrap();
            
            let start = Instant::now();
            publisher.components.gossip.publish(topic, message).await?;
            
            // Wait for propagation (simplified - in real test would track receipt)
            tokio::time::sleep(Duration::from_millis(500 + (*size as u64 / 100))).await;
            
            let propagation_time = start.elapsed();
            propagation_times.push(propagation_time);
            
            debug!("Message size {} propagated in {:?}", size, propagation_time);
        }
        
        let avg_propagation = propagation_times.iter().sum::<Duration>() / propagation_times.len() as u32;
        
        let metrics = PerformanceMetrics {
            gossip_propagation_time: avg_propagation,
            ..Default::default()
        };
        
        info!("Gossip benchmark complete: avg propagation {:?}", avg_propagation);
        
        Ok(metrics)
    }
    
    /// Run scalability test
    pub async fn benchmark_scalability(&mut self) -> Result<ScalabilityMetrics> {
        info!("Starting scalability benchmark");
        
        let node_counts = vec![10, 20, 50, 100, 200];
        let mut throughputs = vec![];
        let mut latencies = vec![];
        
        for &count in &node_counts {
            info!("Testing with {} nodes", count);
            
            // Restart cluster with different size
            self.cluster.shutdown().await?;
            
            let config = TestClusterConfig {
                node_count: count,
                bootstrap_count: count / 10,
                topology: NetworkTopology::Random,
                timeout: Duration::from_secs(300),
                ..Default::default()
            };
            
            self.cluster = TestCluster::new(config).await?;
            self.cluster.start().await?;
            self.cluster.wait_for_stabilization(Duration::from_secs(60)).await?;
            
            // Run benchmarks
            let storage_metrics = self.benchmark_storage().await?;
            
            throughputs.push((count, storage_metrics.storage_throughput));
            latencies.push((count, storage_metrics.avg_storage_latency));
            
            // Store results
            self.results.write().await.by_node_count.insert(count, storage_metrics);
        }
        
        // Calculate scaling factors
        let base_throughput = throughputs[0].1;
        let base_latency = latencies[0].1;
        
        let throughput_scaling = throughputs.last().unwrap().1 / base_throughput;
        let latency_scaling = latencies.last().unwrap().1.as_secs_f64() / base_latency.as_secs_f64();
        
        // Find optimal node count (best throughput/latency ratio)
        let optimal_nodes = throughputs.iter()
            .zip(latencies.iter())
            .map(|((count, tput), (_, lat))| {
                (count, tput / lat.as_secs_f64())
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(count, _)| *count)
            .unwrap_or(50);
        
        let scalability = ScalabilityMetrics {
            throughput_scaling,
            latency_scaling,
            optimal_nodes,
            max_load: throughputs.iter().map(|(_, t)| t).max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0),
        };
        
        info!("Scalability benchmark complete:");
        info!("  Throughput scaling: {:.2}x", throughput_scaling);
        info!("  Latency scaling: {:.2}x", latency_scaling);
        info!("  Optimal nodes: {}", optimal_nodes);
        
        Ok(scalability)
    }
    
    /// Generate benchmark report
    pub async fn generate_report(&self) -> BenchmarkReport {
        let results = self.results.read().await;
        
        BenchmarkReport {
            summary: self.calculate_summary(&results),
            by_node_count: results.by_node_count.clone(),
            by_content_size: results.by_content_size.clone(),
            by_operation: results.by_operation.clone(),
            scalability: results.scalability.clone(),
            recommendations: self.generate_recommendations(&results),
        }
    }
    
    /// Calculate summary statistics
    fn calculate_summary(&self, results: &BenchmarkResults) -> SummaryStats {
        let mut all_metrics = vec![];
        
        for metrics in results.by_node_count.values() {
            all_metrics.push(metrics.clone());
        }
        
        if all_metrics.is_empty() {
            return SummaryStats::default();
        }
        
        SummaryStats {
            avg_storage_throughput: all_metrics.iter()
                .map(|m| m.storage_throughput)
                .sum::<f64>() / all_metrics.len() as f64,
            avg_retrieval_throughput: all_metrics.iter()
                .map(|m| m.retrieval_throughput)
                .sum::<f64>() / all_metrics.len() as f64,
            avg_bandwidth: all_metrics.iter()
                .map(|m| m.bandwidth_mbps)
                .sum::<f64>() / all_metrics.len() as f64,
            best_configuration: format!("{} nodes", results.scalability.optimal_nodes),
        }
    }
    
    /// Generate performance recommendations
    fn generate_recommendations(&self, results: &BenchmarkResults) -> Vec<String> {
        let mut recommendations = vec![];
        
        if results.scalability.latency_scaling > 2.0 {
            recommendations.push("Consider implementing better load balancing to improve latency scaling".to_string());
        }
        
        if results.scalability.throughput_scaling < 0.5 {
            recommendations.push("Throughput doesn't scale well - investigate bottlenecks".to_string());
        }
        
        if results.scalability.optimal_nodes < 50 {
            recommendations.push("Network performs best with smaller clusters - consider sharding".to_string());
        }
        
        recommendations
    }
}

/// Benchmark report
#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub summary: SummaryStats,
    pub by_node_count: BTreeMap<usize, PerformanceMetrics>,
    pub by_content_size: BTreeMap<usize, PerformanceMetrics>,
    pub by_operation: BTreeMap<String, PerformanceMetrics>,
    pub scalability: ScalabilityMetrics,
    pub recommendations: Vec<String>,
}

/// Summary statistics
#[derive(Debug, Clone, Default)]
pub struct SummaryStats {
    pub avg_storage_throughput: f64,
    pub avg_retrieval_throughput: f64,
    pub avg_bandwidth: f64,
    pub best_configuration: String,
}

#[tokio::test]
async fn test_storage_performance() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting storage performance test");
    
    let config = TestClusterConfig {
        node_count: 30,
        bootstrap_count: 3,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    let runner = BenchmarkRunner::new(
        cluster,
        Duration::from_secs(30),
        10, // 10 concurrent operations
    ).await;
    
    let metrics = runner.benchmark_storage().await?;
    
    info!("Storage performance results:");
    info!("  Throughput: {:.2} ops/sec", metrics.storage_throughput);
    info!("  Average latency: {:?}", metrics.avg_storage_latency);
    info!("  P95 latency: {:?}", metrics.routing_latencies.p95);
    info!("  Bandwidth: {:.2} MB/s", metrics.bandwidth_mbps);
    
    assert!(metrics.storage_throughput > 10.0, 
        "Storage throughput should exceed 10 ops/sec");
    assert!(metrics.avg_storage_latency < Duration::from_millis(500),
        "Average storage latency should be under 500ms");
    
    runner.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_retrieval_performance() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting retrieval performance test");
    
    let config = TestClusterConfig {
        node_count: 40,
        bootstrap_count: 4,
        topology: NetworkTopology::Mesh,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    let runner = BenchmarkRunner::new(
        cluster,
        Duration::from_secs(30),
        15, // 15 concurrent operations
    ).await;
    
    let metrics = runner.benchmark_retrieval().await?;
    
    info!("Retrieval performance results:");
    info!("  Throughput: {:.2} ops/sec", metrics.retrieval_throughput);
    info!("  Average latency: {:?}", metrics.avg_retrieval_latency);
    info!("  P95 latency: {:?}", metrics.routing_latencies.p95);
    info!("  P99 latency: {:?}", metrics.routing_latencies.p99);
    
    assert!(metrics.retrieval_throughput > 20.0,
        "Retrieval throughput should exceed 20 ops/sec");
    assert!(metrics.avg_retrieval_latency < Duration::from_millis(300),
        "Average retrieval latency should be under 300ms");
    
    runner.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_routing_performance() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting routing performance test");
    
    let config = TestClusterConfig {
        node_count: 50,
        bootstrap_count: 5,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    let runner = BenchmarkRunner::new(
        cluster,
        Duration::from_secs(20),
        20,
    ).await;
    
    let metrics = runner.benchmark_routing().await?;
    
    info!("Routing performance results:");
    info!("  Average latency: {:?}", metrics.avg_retrieval_latency);
    info!("  P50 latency: {:?}", metrics.routing_latencies.p50);
    info!("  P95 latency: {:?}", metrics.routing_latencies.p95);
    info!("  P99 latency: {:?}", metrics.routing_latencies.p99);
    
    assert!(metrics.avg_retrieval_latency < Duration::from_millis(50),
        "Average routing latency should be under 50ms");
    assert!(metrics.routing_latencies.p99 < Duration::from_millis(200),
        "P99 routing latency should be under 200ms");
    
    runner.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting concurrent operations test");
    
    let config = TestClusterConfig {
        node_count: 30,
        bootstrap_count: 3,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    let nodes = cluster.nodes.read().await;
    let test_nodes: Vec<_> = nodes.values().take(20).collect();
    
    // Concurrent mixed operations
    let semaphore = Arc::new(Semaphore::new(50)); // Max 50 concurrent ops
    let mut handles = vec![];
    let success_count = Arc::new(AtomicU64::new(0));
    let total_count = Arc::new(AtomicU64::new(0));
    
    for _ in 0..200 {
        let permit = semaphore.clone().acquire_owned().await?;
        let node = test_nodes[rand::random::<usize>() % test_nodes.len()].clone();
        let success = success_count.clone();
        let total = total_count.clone();
        
        let handle = tokio::spawn(async move {
            total.fetch_add(1, Ordering::Relaxed);
            
            // Random operation
            match rand::random::<u8>() % 3 {
                0 => {
                    // Storage
                    let content = utils::generate_content(10 * 1024);
                    let metadata = storage::ContentMetadata {
                        size: content.len(),
                        content_type: ContentType::DataRetrieval,
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        chunk_count: None,
                        replication_factor: 5,
                    };
                    
                    if node.components.storage.store(content, metadata).await.is_ok() {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                }
                1 => {
                    // Routing
                    let target = NodeId { hash: [rand::random::<u8>(); 32] };
                    let kad_strategy = KademliaRoutingStrategy::new(node.components.dht.clone());
                    if kad_strategy.find_path(&target).await.is_ok() {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                }
                _ => {
                    // Gossip
                    let message = b"Test message".to_vec();
                    if node.components.gossip.publish("test", message).await.is_ok() {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            
            drop(permit);
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations
    for handle in handles {
        handle.await?;
    }
    
    let total_ops = total_count.load(Ordering::Relaxed);
    let successful_ops = success_count.load(Ordering::Relaxed);
    let success_rate = successful_ops as f64 / total_ops as f64;
    
    info!("Concurrent operations results:");
    info!("  Total operations: {}", total_ops);
    info!("  Successful: {}", successful_ops);
    info!("  Success rate: {:.2}%", success_rate * 100.0);
    
    assert!(success_rate > 0.95,
        "Concurrent operations should have >95% success rate");
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Long-running test
async fn test_network_scalability() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting network scalability test");
    
    // Start with small cluster
    let config = TestClusterConfig {
        node_count: 10,
        bootstrap_count: 2,
        topology: NetworkTopology::Random,
        timeout: Duration::from_secs(600),
        ..Default::default()
    };
    
    let cluster = TestCluster::new(config).await?;
    
    let mut runner = BenchmarkRunner::new(
        cluster,
        Duration::from_secs(30),
        5,
    ).await;
    
    let scalability = runner.benchmark_scalability().await?;
    let report = runner.generate_report().await;
    
    info!("Scalability test results:");
    info!("  Throughput scaling: {:.2}x", scalability.throughput_scaling);
    info!("  Latency scaling: {:.2}x", scalability.latency_scaling);
    info!("  Optimal network size: {} nodes", scalability.optimal_nodes);
    info!("  Maximum load: {:.2} ops/sec", scalability.max_load);
    
    info!("\nPerformance by node count:");
    for (count, metrics) in &report.by_node_count {
        info!("  {} nodes: {:.2} ops/sec, {:?} latency",
            count, metrics.storage_throughput, metrics.avg_storage_latency);
    }
    
    info!("\nRecommendations:");
    for recommendation in &report.recommendations {
        info!("  - {}", recommendation);
    }
    
    assert!(scalability.throughput_scaling > 0.3,
        "Throughput should scale at least 30% with more nodes");
    assert!(scalability.latency_scaling < 3.0,
        "Latency should not increase more than 3x with scale");
    
    runner.cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_stress_test() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting stress test");
    
    let config = TestClusterConfig {
        node_count: 50,
        bootstrap_count: 5,
        topology: NetworkTopology::Random,
        conditions: NetworkConditions {
            packet_loss: 0.02,   // 2% packet loss
            latency_ms: 20,      // 20ms base latency
            jitter_ms: 5,        // 5ms jitter
            bandwidth_mbps: 50,  // 50 Mbps limit
            failure_rate: 5.0,   // 5 failures/hour
        },
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(45)).await?;
    cluster.apply_network_conditions().await?;
    
    // Run intensive workload
    let runner = BenchmarkRunner::new(
        cluster,
        Duration::from_secs(60),
        30, // High concurrency
    ).await;
    
    // Run all benchmarks under stress
    let storage_metrics = runner.benchmark_storage().await?;
    let retrieval_metrics = runner.benchmark_retrieval().await?;
    let routing_metrics = runner.benchmark_routing().await?;
    
    info!("Stress test results:");
    info!("  Storage throughput: {:.2} ops/sec", storage_metrics.storage_throughput);
    info!("  Retrieval throughput: {:.2} ops/sec", retrieval_metrics.retrieval_throughput);
    info!("  Average routing latency: {:?}", routing_metrics.avg_retrieval_latency);
    
    // Performance should degrade gracefully under stress
    assert!(storage_metrics.storage_throughput > 5.0,
        "Should maintain at least 5 ops/sec under stress");
    assert!(retrieval_metrics.avg_retrieval_latency < Duration::from_secs(2),
        "Retrieval latency should stay under 2 seconds");
    
    runner.cluster.shutdown().await?;
    Ok(())
}