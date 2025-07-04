
//! Performance monitoring and benchmarking utilities
//!
//! Provides tools for measuring and analyzing performance characteristics
//! of P2P operations, including latency, throughput, and resource usage.

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use sysinfo::System;
use tracing::{debug, info, warn};

/// Performance benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of iterations to run
    pub iterations: u32,
    
    /// Number of concurrent operations
    pub concurrency: u32,
    
    /// Warmup iterations (not included in results)
    pub warmup_iterations: u32,
    
    /// Maximum duration for the benchmark
    pub max_duration: Duration,
    
    /// Sample interval for system metrics
    pub sample_interval: Duration,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            concurrency: 10,
            warmup_iterations: 100,
            max_duration: Duration::from_secs(300), // 5 minutes
            sample_interval: Duration::from_millis(100),
        }
    }
}

/// Performance measurement results
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Operation name
    pub name: String,
    
    /// Total number of operations completed
    pub operations_completed: u32,
    
    /// Total benchmark duration
    pub total_duration: Duration,
    
    /// Individual operation durations
    pub operation_durations: Vec<Duration>,
    
    /// System resource usage during benchmark
    pub resource_usage: ResourceUsage,
    
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

impl BenchmarkResult {
    /// Calculate operations per second
    pub fn ops_per_second(&self) -> f64 {
        if self.total_duration.as_secs_f64() > 0.0 {
            self.operations_completed as f64 / self.total_duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Calculate average latency
    pub fn average_latency(&self) -> Duration {
        if !self.operation_durations.is_empty() {
            let total: Duration = self.operation_durations.iter().sum();
            total / self.operation_durations.len() as u32
        } else {
            Duration::ZERO
        }
    }

    /// Calculate percentile latency
    pub fn percentile_latency(&self, percentile: f64) -> Duration {
        if self.operation_durations.is_empty() {
            return Duration::ZERO;
        }

        let mut sorted = self.operation_durations.clone();
        sorted.sort();

        let index = ((percentile / 100.0) * sorted.len() as f64) as usize;
        let index = index.min(sorted.len() - 1);

        sorted[index]
    }

    /// Calculate minimum latency
    pub fn min_latency(&self) -> Duration {
        self.operation_durations.iter().min().copied().unwrap_or(Duration::ZERO)
    }

    /// Calculate maximum latency
    pub fn max_latency(&self) -> Duration {
        self.operation_durations.iter().max().copied().unwrap_or(Duration::ZERO)
    }

    /// Generate summary report
    pub fn summary(&self) -> String {
        format!(
            "Benchmark: {}\n\
             Operations: {} in {:?}\n\
             Throughput: {:.1} ops/sec\n\
             Latency: avg={:?}, p50={:?}, p95={:?}, p99={:?}\n\
             CPU: {:.1}%, Memory: {:.1} MB",
            self.name,
            self.operations_completed,
            self.total_duration,
            self.ops_per_second(),
            self.average_latency(),
            self.percentile_latency(50.0),
            self.percentile_latency(95.0),
            self.percentile_latency(99.0),
            self.resource_usage.average_cpu_percent,
            self.resource_usage.peak_memory_mb,
        )
    }
}

/// System resource usage measurements
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// Average CPU usage percentage
    pub average_cpu_percent: f64,
    
    /// Peak CPU usage percentage
    pub peak_cpu_percent: f64,
    
    /// Peak memory usage in MB
    pub peak_memory_mb: f64,
    
    /// Average memory usage in MB
    pub average_memory_mb: f64,
    
    /// Number of samples taken
    pub sample_count: u32,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            average_cpu_percent: 0.0,
            peak_cpu_percent: 0.0,
            peak_memory_mb: 0.0,
            average_memory_mb: 0.0,
            sample_count: 0,
        }
    }
}

/// Performance monitor for tracking system resources
pub struct PerformanceMonitor {
    system: System,
    process_id: u32,
    samples: Vec<ResourceSample>,
    start_time: Instant,
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        let process_id = std::process::id();
        
        Self {
            system,
            process_id,
            samples: Vec::new(),
            start_time: Instant::now(),
        }
    }

    /// Start monitoring (call this before benchmark)
    pub fn start(&mut self) {
        self.samples.clear();
        self.start_time = Instant::now();
        self.system.refresh_all();
    }

    /// Take a resource usage sample
    pub fn sample(&mut self) {
        self.system.refresh_all();
        
        let cpu_percent = self.system.global_cpu_info().cpu_usage() as f64;
        
        let memory_mb = if let Some(process) = self.system.process(sysinfo::Pid::from(self.process_id as usize)) {
            process.memory() as f64 / 1024.0 / 1024.0 // Convert to MB
        } else {
            0.0
        };

        self.samples.push(ResourceSample {
            timestamp: self.start_time.elapsed(),
            cpu_percent,
            memory_mb,
        });
    }

    /// Stop monitoring and return resource usage summary
    pub fn stop(&self) -> ResourceUsage {
        if self.samples.is_empty() {
            return ResourceUsage::default();
        }

        let cpu_values: Vec<f64> = self.samples.iter().map(|s| s.cpu_percent).collect();
        let memory_values: Vec<f64> = self.samples.iter().map(|s| s.memory_mb).collect();

        let average_cpu = cpu_values.iter().sum::<f64>() / cpu_values.len() as f64;
        let peak_cpu = cpu_values.iter().fold(0.0f64, |a, &b| a.max(b));
        
        let average_memory = memory_values.iter().sum::<f64>() / memory_values.len() as f64;
        let peak_memory = memory_values.iter().fold(0.0f64, |a, &b| a.max(b));

        ResourceUsage {
            average_cpu_percent: average_cpu,
            peak_cpu_percent: peak_cpu,
            peak_memory_mb: peak_memory,
            average_memory_mb: average_memory,
            sample_count: self.samples.len() as u32,
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct ResourceSample {
    timestamp: Duration,
    cpu_percent: f64,
    memory_mb: f64,
}

/// Benchmark executor for running performance tests
pub struct BenchmarkExecutor {
    monitor: PerformanceMonitor,
}

impl BenchmarkExecutor {
    pub fn new() -> Self {
        Self {
            monitor: PerformanceMonitor::new(),
        }
    }

    /// Run a benchmark with the given configuration
    pub async fn run_benchmark<F, Fut>(
        &mut self,
        name: String,
        config: BenchmarkConfig,
        operation: F,
    ) -> Result<BenchmarkResult>
    where
        F: Fn() -> Fut + Clone,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!("Starting benchmark: {} with {} iterations", name, config.iterations);

        // Warmup phase
        if config.warmup_iterations > 0 {
            debug!("Running {} warmup iterations", config.warmup_iterations);
            for _ in 0..config.warmup_iterations {
                if let Err(e) = operation().await {
                    warn!("Warmup iteration failed: {:?}", e);
                }
            }
        }

        // Start monitoring
        self.monitor.start();
        let benchmark_start = Instant::now();
        
        // Start background resource sampling
        let sample_interval = config.sample_interval;
        let monitor_handle = {
            let mut monitor_clone = PerformanceMonitor::new();
            monitor_clone.start();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(sample_interval);
                loop {
                    interval.tick().await;
                    monitor_clone.sample();
                    
                    // Stop sampling if benchmark should be done
                    if monitor_clone.start_time.elapsed() > sample_interval * 1000 {
                        break;
                    }
                }
                monitor_clone.stop()
            })
        };

        // Run benchmark iterations
        let mut operation_durations = Vec::with_capacity(config.iterations as usize);
        let mut completed_operations = 0u32;
        let mut custom_metrics = HashMap::new();

        // Sequential execution for now (can be extended for concurrent)
        for i in 0..config.iterations {
            if benchmark_start.elapsed() > config.max_duration {
                warn!("Benchmark timeout reached after {} iterations", i);
                break;
            }

            let operation_start = Instant::now();
            match operation().await {
                Ok(_) => {
                    let operation_duration = operation_start.elapsed();
                    operation_durations.push(operation_duration);
                    completed_operations += 1;
                }
                Err(e) => {
                    warn!("Operation {} failed: {:?}", i, e);
                }
            }

            // Progress reporting
            if i % 100 == 0 && i > 0 {
                debug!("Completed {}/{} iterations", i, config.iterations);
            }
        }

        let total_duration = benchmark_start.elapsed();
        
        // Stop monitoring
        let resource_usage = if let Ok(usage) = monitor_handle.await {
            usage
        } else {
            self.monitor.stop()
        };

        let result = BenchmarkResult {
            name,
            operations_completed: completed_operations,
            total_duration,
            operation_durations,
            resource_usage,
            custom_metrics,
        };

        info!("Benchmark completed: {}", result.summary());

        Ok(result)
    }

    /// Run concurrent benchmark
    pub async fn run_concurrent_benchmark<F, Fut>(
        &mut self,
        name: String,
        config: BenchmarkConfig,
        operation: F,
    ) -> Result<BenchmarkResult>
    where
        F: Fn() -> Fut + Clone + Send + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        info!(
            "Starting concurrent benchmark: {} with {} iterations, {} concurrency",
            name, config.iterations, config.concurrency
        );

        self.monitor.start();
        let benchmark_start = Instant::now();

        let mut handles = Vec::new();
        let operations_per_task = config.iterations / config.concurrency;
        let remaining_operations = config.iterations % config.concurrency;

        // Spawn concurrent tasks
        for task_id in 0..config.concurrency {
            let operations_for_this_task = if task_id < remaining_operations {
                operations_per_task + 1
            } else {
                operations_per_task
            };

            let operation_clone = operation.clone();
            let max_duration = config.max_duration;
            
            let handle = tokio::spawn(async move {
                let mut durations = Vec::new();
                let mut completed = 0u32;
                let task_start = Instant::now();

                for _ in 0..operations_for_this_task {
                    if task_start.elapsed() > max_duration {
                        break;
                    }

                    let op_start = Instant::now();
                    if operation_clone().await.is_ok() {
                        durations.push(op_start.elapsed());
                        completed += 1;
                    }
                }

                (durations, completed)
            });

            handles.push(handle);
        }

        // Collect results from all tasks
        let mut all_durations = Vec::new();
        let mut total_completed = 0u32;

        for handle in handles {
            if let Ok((durations, completed)) = handle.await {
                all_durations.extend(durations);
                total_completed += completed;
            }
        }

        let total_duration = benchmark_start.elapsed();
        let resource_usage = self.monitor.stop();

        let result = BenchmarkResult {
            name,
            operations_completed: total_completed,
            total_duration,
            operation_durations: all_durations,
            resource_usage,
            custom_metrics: HashMap::new(),
        };

        info!("Concurrent benchmark completed: {}", result.summary());

        Ok(result)
    }
}

impl Default for BenchmarkExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance thresholds for validation
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    pub max_average_latency: Duration,
    pub max_p99_latency: Duration,
    pub min_throughput: f64,
    pub max_cpu_percent: f64,
    pub max_memory_mb: f64,
}

impl PerformanceThresholds {
    /// Validate benchmark results against thresholds
    pub fn validate(&self, result: &BenchmarkResult) -> Vec<String> {
        let mut violations = Vec::new();

        if result.average_latency() > self.max_average_latency {
            violations.push(format!(
                "Average latency {:?} exceeds threshold {:?}",
                result.average_latency(),
                self.max_average_latency
            ));
        }

        if result.percentile_latency(99.0) > self.max_p99_latency {
            violations.push(format!(
                "P99 latency {:?} exceeds threshold {:?}",
                result.percentile_latency(99.0),
                self.max_p99_latency
            ));
        }

        if result.ops_per_second() < self.min_throughput {
            violations.push(format!(
                "Throughput {:.1} ops/sec below threshold {:.1}",
                result.ops_per_second(),
                self.min_throughput
            ));
        }

        if result.resource_usage.average_cpu_percent > self.max_cpu_percent {
            violations.push(format!(
                "CPU usage {:.1}% exceeds threshold {:.1}%",
                result.resource_usage.average_cpu_percent,
                self.max_cpu_percent
            ));
        }

        if result.resource_usage.peak_memory_mb > self.max_memory_mb {
            violations.push(format!(
                "Memory usage {:.1} MB exceeds threshold {:.1} MB",
                result.resource_usage.peak_memory_mb,
                self.max_memory_mb
            ));
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_executor() {
        let mut executor = BenchmarkExecutor::new();
        let config = BenchmarkConfig {
            iterations: 10,
            concurrency: 1,
            warmup_iterations: 2,
            max_duration: Duration::from_secs(10),
            sample_interval: Duration::from_millis(100),
        };

        let result = executor.run_benchmark(
            "test_benchmark".to_string(),
            config,
            || async {
                tokio::time::sleep(Duration::from_millis(1)).await;
                Ok(())
            },
        ).await.unwrap();

        assert_eq!(result.operations_completed, 10);
        assert!(result.total_duration > Duration::ZERO);
        assert!(!result.operation_durations.is_empty());
    }

    #[test]
    fn test_benchmark_result_calculations() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];

        let result = BenchmarkResult {
            name: "test".to_string(),
            operations_completed: 5,
            total_duration: Duration::from_secs(1),
            operation_durations: durations,
            resource_usage: ResourceUsage::default(),
            custom_metrics: HashMap::new(),
        };

        assert_eq!(result.ops_per_second(), 5.0);
        assert_eq!(result.average_latency(), Duration::from_millis(30));
        assert_eq!(result.min_latency(), Duration::from_millis(10));
        assert_eq!(result.max_latency(), Duration::from_millis(50));
    }
}