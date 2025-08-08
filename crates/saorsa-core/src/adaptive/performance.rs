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

//! Performance optimization utilities for the Adaptive P2P Network
//!
//! This module provides tools and optimizations for improving network performance:
//! - Zero-copy message handling
//! - Optimized serialization
//! - Connection pooling
//! - Caching strategies
//! - Concurrent operation tuning

#![allow(missing_docs)]

use super::*;
use bytes::{Bytes, BytesMut};
use parking_lot::RwLock as PLRwLock;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, mpsc};

/// Performance configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,

    /// Connection pool size
    pub connection_pool_size: usize,

    /// Cache configuration
    pub cache_config: CacheConfig,

    /// Serialization settings
    pub serialization: SerializationConfig,

    /// Batch operation settings
    pub batch_config: BatchConfig,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_ops: 1000,
            connection_pool_size: 100,
            cache_config: CacheConfig::default(),
            serialization: SerializationConfig::default(),
            batch_config: BatchConfig::default(),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum cache entries
    pub max_entries: usize,

    /// Cache TTL
    pub ttl: Duration,

    /// Enable compression
    pub compression: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl: Duration::from_secs(300),
            compression: true,
        }
    }
}

/// Serialization configuration
#[derive(Debug, Clone)]
pub struct SerializationConfig {
    /// Use zero-copy deserialization
    pub zero_copy: bool,

    /// Pre-allocated buffer size
    pub buffer_size: usize,

    /// Use compression
    pub compression: bool,
}

impl Default for SerializationConfig {
    fn default() -> Self {
        Self {
            zero_copy: true,
            buffer_size: 4096,
            compression: false,
        }
    }
}

/// Batch operation configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum batch size
    pub max_batch_size: usize,

    /// Batch timeout
    pub batch_timeout: Duration,

    /// Enable automatic batching
    pub auto_batch: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            batch_timeout: Duration::from_millis(10),
            auto_batch: true,
        }
    }
}

/// Zero-copy message wrapper
pub struct ZeroCopyMessage {
    data: Bytes,
}

impl ZeroCopyMessage {
    /// Create from bytes
    pub fn new(data: Bytes) -> Self {
        Self { data }
    }

    /// Get reference to data
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Deserialize without copying
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        bincode::deserialize(&self.data).map_err(AdaptiveNetworkError::Serialization)
    }
}

/// Optimized serializer with buffer reuse
pub struct OptimizedSerializer {
    buffer_pool: Arc<PLRwLock<Vec<BytesMut>>>,
    config: SerializationConfig,
}

impl OptimizedSerializer {
    pub fn new(config: SerializationConfig) -> Self {
        Self {
            buffer_pool: Arc::new(PLRwLock::new(Vec::new())),
            config,
        }
    }

    /// Serialize with buffer reuse
    pub fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Bytes> {
        // Get buffer from pool or create new
        let mut buffer = self
            .buffer_pool
            .write()
            .pop()
            .unwrap_or_else(|| BytesMut::with_capacity(self.config.buffer_size));

        buffer.clear();

        // Serialize to buffer
        let serialized = bincode::serialize(value).map_err(AdaptiveNetworkError::Serialization)?;
        buffer.extend_from_slice(&serialized);

        // Optional compression
        let bytes = if self.config.compression {
            let compressed = self.compress(&buffer)?;
            // Return buffer to pool after compression
            if buffer.capacity() <= self.config.buffer_size * 2 {
                self.buffer_pool.write().push(buffer);
            }
            compressed
        } else {
            // freeze() consumes the buffer, so we can't return it to the pool
            buffer.freeze()
        };

        Ok(bytes)
    }

    fn compress(&self, data: &[u8]) -> Result<Bytes> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(data)?;
        Ok(Bytes::from(encoder.finish()?))
    }
}

/// Connection pool for reusing connections
pub struct ConnectionPool<T> {
    connections: Arc<PLRwLock<HashMap<String, Vec<T>>>>,
    semaphore: Arc<Semaphore>,
    max_per_host: usize,
}

impl<T: Send> ConnectionPool<T> {
    pub fn new(max_connections: usize, max_per_host: usize) -> Self {
        Self {
            connections: Arc::new(PLRwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            max_per_host,
        }
    }

    /// Get connection from pool
    pub async fn get(&self, host: &str) -> Option<T> {
        let _permit = self.semaphore.acquire().await.ok()?;
        self.connections
            .write()
            .get_mut(host)
            .and_then(|conns| conns.pop())
    }

    /// Return connection to pool
    pub fn put(&self, host: String, conn: T) {
        let mut pool = self.connections.write();
        let conns = pool.entry(host).or_default();

        if conns.len() < self.max_per_host {
            conns.push(conn);
        }
    }
}

/// High-performance cache with TTL
pub struct PerformanceCache<K, V> {
    entries: Arc<PLRwLock<HashMap<K, CacheEntry<V>>>>,
    config: CacheConfig,
}

#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> PerformanceCache<K, V> {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(PLRwLock::new(HashMap::new())),
            config,
        }
    }

    /// Get value from cache
    pub fn get(&self, key: &K) -> Option<V> {
        let entries = self.entries.read();
        entries.get(key).and_then(|entry| {
            if entry.inserted_at.elapsed() < self.config.ttl {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    /// Insert value into cache
    pub fn insert(&self, key: K, value: V) {
        let mut entries = self.entries.write();

        // Evict old entries if at capacity
        if entries.len() >= self.config.max_entries {
            let _now = Instant::now();
            entries.retain(|_, entry| entry.inserted_at.elapsed() < self.config.ttl);

            // If still over capacity, remove oldest
            if entries.len() >= self.config.max_entries {
                if let Some(oldest_key) = entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.inserted_at)
                    .map(|(k, _)| k.clone())
                {
                    entries.remove(&oldest_key);
                }
            }
        }

        entries.insert(
            key,
            CacheEntry {
                value,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Clear expired entries
    pub fn evict_expired(&self) {
        let mut entries = self.entries.write();
        let _now = Instant::now();
        entries.retain(|_, entry| entry.inserted_at.elapsed() < self.config.ttl);
    }
}

/// Batch processor for aggregating operations
pub struct BatchProcessor<T> {
    config: BatchConfig,
    tx: mpsc::Sender<T>,
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<T>>>,
}

impl<T: Send + 'static> BatchProcessor<T> {
    pub fn new(config: BatchConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.max_batch_size * 10);
        Self {
            config,
            tx,
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }

    /// Add item to batch
    pub async fn add(&self, item: T) -> Result<()> {
        self.tx.send(item).await.map_err(|_| {
            AdaptiveNetworkError::Other("Batch processor closed".to_string().into())
        })?;
        Ok(())
    }

    /// Process batch with given function
    pub async fn process_batch<F, Fut>(&self, mut f: F) -> Result<()>
    where
        F: FnMut(Vec<T>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut rx = self.rx.lock().await;
        let mut batch = Vec::with_capacity(self.config.max_batch_size);

        // Collect items up to batch size or timeout
        let timeout = tokio::time::sleep(self.config.batch_timeout);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(item) = rx.recv() => {
                    batch.push(item);
                    if batch.len() >= self.config.max_batch_size {
                        break;
                    }
                }
                _ = &mut timeout => {
                    if !batch.is_empty() {
                        break;
                    }
                }
            }
        }

        if !batch.is_empty() {
            f(batch).await?;
        }

        Ok(())
    }
}

/// Concurrent operation limiter
#[derive(Clone)]
pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Execute operation with concurrency limit
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| AdaptiveNetworkError::Other("Semaphore closed".to_string().into()))?;
        f().await
    }

    /// Execute many operations with concurrency limit
    pub async fn execute_many<F, Fut, T>(&self, operations: Vec<F>) -> Vec<Result<T>>
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        let futures = operations.into_iter().map(|op| {
            let semaphore = self.semaphore.clone();
            async move {
                let _permit = semaphore.acquire().await.ok()?;
                Some(op().await)
            }
        });

        futures::future::join_all(futures)
            .await
            .into_iter()
            .flatten()
            .collect()
    }
}

/// Performance monitoring and metrics
pub struct PerformanceMonitor {
    operation_times: Arc<PLRwLock<HashMap<String, Vec<Duration>>>>,
    start_times: Arc<PLRwLock<HashMap<String, Instant>>>,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            operation_times: Arc::new(PLRwLock::new(HashMap::new())),
            start_times: Arc::new(PLRwLock::new(HashMap::new())),
        }
    }

    /// Start timing an operation
    pub fn start_operation(&self, name: &str) {
        self.start_times
            .write()
            .insert(name.to_string(), Instant::now());
    }

    /// End timing an operation
    pub fn end_operation(&self, name: &str) {
        if let Some(start) = self.start_times.write().remove(name) {
            let duration = start.elapsed();
            self.operation_times
                .write()
                .entry(name.to_string())
                .or_default()
                .push(duration);
        }
    }

    /// Get performance statistics
    pub fn get_stats(&self, name: &str) -> Option<PerformanceStats> {
        let times = self.operation_times.read();
        times.get(name).map(|durations| {
            let total: Duration = durations.iter().sum();
            let count = durations.len();
            let avg = total / count as u32;

            let mut sorted = durations.clone();
            sorted.sort();

            PerformanceStats {
                count,
                avg_duration: avg,
                min_duration: sorted.first().copied().unwrap_or_default(),
                max_duration: sorted.last().copied().unwrap_or_default(),
                p50_duration: sorted.get(count / 2).copied().unwrap_or_default(),
                p99_duration: sorted.get(count * 99 / 100).copied().unwrap_or_default(),
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub count: usize,
    pub avg_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub p50_duration: Duration,
    pub p99_duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_message() {
        let data = Bytes::from(vec![1, 2, 3, 4]);
        let msg = ZeroCopyMessage::new(data);
        assert_eq!(msg.as_bytes(), &[1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_batch_processor() {
        let processor = BatchProcessor::new(BatchConfig {
            max_batch_size: 3,
            batch_timeout: Duration::from_millis(100),
            auto_batch: true,
        });

        // Add items
        processor.add(1).await.unwrap();
        processor.add(2).await.unwrap();
        processor.add(3).await.unwrap();

        // Process batch
        processor
            .process_batch(|batch| async move {
                assert_eq!(batch.len(), 3);
                assert_eq!(batch, vec![1, 2, 3]);
                Ok::<_, AdaptiveNetworkError>(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_concurrency_limiter() {
        let limiter = ConcurrencyLimiter::new(2);
        let counter = Arc::new(tokio::sync::Mutex::new(0));

        let operations: Vec<_> = (0..5)
            .map(|_| {
                let counter = counter.clone();
                let limiter = limiter.clone();

                tokio::spawn(async move {
                    limiter
                        .execute(|| async {
                            let mut count = counter.lock().await;
                            *count += 1;
                            Ok::<_, AdaptiveNetworkError>(())
                        })
                        .await
                })
            })
            .collect();

        for op in operations {
            op.await.unwrap().unwrap();
        }

        assert_eq!(*counter.lock().await, 5);
    }

    #[test]
    fn test_performance_cache() {
        let cache = PerformanceCache::new(CacheConfig {
            max_entries: 2,
            ttl: Duration::from_secs(1),
            compression: false,
        });

        cache.insert("key1", "value1");
        cache.insert("key2", "value2");

        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));

        // Add third item, should evict oldest
        cache.insert("key3", "value3");
        assert_eq!(cache.get(&"key3"), Some("value3"));
    }

    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();

        monitor.start_operation("test_op");
        std::thread::sleep(Duration::from_millis(10));
        monitor.end_operation("test_op");

        let stats = monitor.get_stats("test_op").unwrap();
        assert_eq!(stats.count, 1);
        assert!(stats.avg_duration >= Duration::from_millis(10));
    }
}
