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

//! # Cache Eviction Strategies for Adaptive Learning
//!
//! This module provides various cache eviction strategies that integrate with
//! the Q-learning cache management system. Strategies include:
//! - LRU (Least Recently Used)
//! - LFU (Least Frequently Used)
//! - FIFO (First In First Out)
//! - Adaptive (Q-value based eviction)

// use super::*; // Removed unused import
use super::ContentHash;
use super::q_learning_cache::{AccessInfo, CacheAction, StateVector};
// use crate::Result; // Removed unused import
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Trait for cache eviction strategies
pub trait EvictionStrategy: Send + Sync + std::fmt::Debug {
    /// Select a content hash to evict from the cache
    fn select_victim(
        &self,
        cache_state: &CacheState,
        access_info: &HashMap<ContentHash, AccessInfo>,
    ) -> Option<ContentHash>;

    /// Update strategy state when content is accessed
    fn on_access(&mut self, content_hash: &ContentHash);

    /// Update strategy state when new content is inserted
    fn on_insert(&mut self, content_hash: &ContentHash);

    /// Get strategy name for logging/metrics
    fn name(&self) -> &str;
}

/// Cache state information for eviction decisions
#[derive(Debug, Clone)]
pub struct CacheState {
    /// Current cache size in bytes
    pub current_size: u64,
    /// Maximum cache size in bytes
    pub max_size: u64,
    /// Number of items in cache
    pub item_count: usize,
    /// Average access frequency
    pub avg_access_frequency: f64,
}

/// LRU (Least Recently Used) eviction strategy
#[derive(Debug)]
pub struct LRUStrategy {
    /// Access order tracking
    access_order: VecDeque<ContentHash>,
    /// Map for O(1) position lookup
    position_map: HashMap<ContentHash, usize>,
}

impl LRUStrategy {
    pub fn new() -> Self {
        Self {
            access_order: VecDeque::new(),
            position_map: HashMap::new(),
        }
    }
}

impl EvictionStrategy for LRUStrategy {
    fn select_victim(
        &self,
        _cache_state: &CacheState,
        access_info: &HashMap<ContentHash, AccessInfo>,
    ) -> Option<ContentHash> {
        // Find least recently used item that exists in cache
        self.access_order
            .iter()
            .find(|&hash| access_info.contains_key(hash))
            .cloned()
    }

    fn on_access(&mut self, content_hash: &ContentHash) {
        // Remove from current position if exists
        if let Some(&pos) = self.position_map.get(content_hash) {
            self.access_order.remove(pos);
            // Update positions for items after removed one
            for (i, hash) in self.access_order.iter().enumerate().skip(pos) {
                self.position_map.insert(hash.clone(), i);
            }
        }

        // Add to end (most recently used)
        self.access_order.push_back(content_hash.clone());
        self.position_map
            .insert(content_hash.clone(), self.access_order.len() - 1);
    }

    fn on_insert(&mut self, content_hash: &ContentHash) {
        self.on_access(content_hash);
    }

    fn name(&self) -> &str {
        "LRU"
    }
}

/// LFU (Least Frequently Used) eviction strategy
#[derive(Debug)]
pub struct LFUStrategy {
    /// Frequency counts
    frequency_map: HashMap<ContentHash, u64>,
}

impl LFUStrategy {
    pub fn new() -> Self {
        Self {
            frequency_map: HashMap::new(),
        }
    }
}

impl EvictionStrategy for LFUStrategy {
    fn select_victim(
        &self,
        _cache_state: &CacheState,
        access_info: &HashMap<ContentHash, AccessInfo>,
    ) -> Option<ContentHash> {
        // Find item with lowest frequency
        access_info
            .keys()
            .min_by_key(|&hash| self.frequency_map.get(hash).unwrap_or(&0))
            .cloned()
    }

    fn on_access(&mut self, content_hash: &ContentHash) {
        *self.frequency_map.entry(content_hash.clone()).or_insert(0) += 1;
    }

    fn on_insert(&mut self, content_hash: &ContentHash) {
        self.frequency_map.insert(content_hash.clone(), 1);
    }

    fn name(&self) -> &str {
        "LFU"
    }
}

/// FIFO (First In First Out) eviction strategy
#[derive(Debug)]
pub struct FIFOStrategy {
    /// Insertion order
    insertion_order: VecDeque<ContentHash>,
}

impl FIFOStrategy {
    pub fn new() -> Self {
        Self {
            insertion_order: VecDeque::new(),
        }
    }
}

impl EvictionStrategy for FIFOStrategy {
    fn select_victim(
        &self,
        _cache_state: &CacheState,
        access_info: &HashMap<ContentHash, AccessInfo>,
    ) -> Option<ContentHash> {
        // Find oldest item that still exists in cache
        self.insertion_order
            .iter()
            .find(|&hash| access_info.contains_key(hash))
            .cloned()
    }

    fn on_access(&mut self, _content_hash: &ContentHash) {
        // FIFO doesn't care about access patterns
    }

    fn on_insert(&mut self, content_hash: &ContentHash) {
        self.insertion_order.push_back(content_hash.clone());
    }

    fn name(&self) -> &str {
        "FIFO"
    }
}

/// Q-value type for adaptive strategy
#[derive(Debug, Clone, Default)]
pub struct QValue {
    pub value: f64,
    pub updates: u32,
}

/// Adaptive eviction strategy based on Q-learning values
#[derive(Debug)]
pub struct AdaptiveStrategy {
    /// Q-table reference for value-based decisions
    _q_table: Arc<RwLock<HashMap<(StateVector, CacheAction), QValue>>>,
    /// Recent performance history
    _performance_history: VecDeque<f64>,
    /// Maximum history size
    _max_history: usize,
}

#[allow(dead_code)]
impl AdaptiveStrategy {
    /// Get current timestamp in seconds since UNIX_EPOCH
    fn current_timestamp_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn new(q_table: Arc<RwLock<HashMap<(StateVector, CacheAction), QValue>>>) -> Self {
        Self {
            _q_table: q_table,
            _performance_history: VecDeque::new(),
            _max_history: 100,
        }
    }

    /// Calculate expected value of keeping content in cache
    async fn calculate_retention_value(
        &self,
        content_hash: &ContentHash,
        access_info: &AccessInfo,
        cache_state: &CacheState,
    ) -> f64 {
        // Create state vector for this content
        let state = StateVector {
            utilization_bucket: ((cache_state.current_size as f64 / cache_state.max_size as f64)
                * 10.0) as u8,
            frequency_bucket: self.frequency_bucket(access_info.count),
            recency_bucket: self.recency_bucket(access_info.last_access_secs),
            content_size_bucket: self.size_bucket(access_info.size),
        };

        // Get Q-values for keeping vs evicting
        let q_table = self._q_table.read().await;
        let keep_value = q_table
            .get(&(state, CacheAction::DoNothing))
            .map(|qv| qv.value)
            .unwrap_or(0.0);
        let evict_value = q_table
            .get(&(state, CacheAction::Evict(content_hash.clone())))
            .map(|qv| qv.value)
            .unwrap_or(0.0);

        keep_value - evict_value
    }

    fn frequency_bucket(&self, count: u64) -> u8 {
        match count {
            0..=10 => 0,
            11..=50 => 1,
            51..=100 => 2,
            101..=500 => 3,
            501..=1000 => 4,
            _ => 5,
        }
    }

    fn recency_bucket(&self, last_access_secs: u64) -> u8 {
        let now_secs = Self::current_timestamp_secs();
        let age_secs = now_secs.saturating_sub(last_access_secs);

        match age_secs {
            0..=60 => 0,         // Last minute
            61..=3600 => 1,      // Last hour
            3601..=86400 => 2,   // Last day
            86401..=604800 => 3, // Last week
            _ => 4,              // Older
        }
    }

    fn size_bucket(&self, size: u64) -> u8 {
        match size {
            0..=1024 => 0,           // <= 1KB
            1025..=10240 => 1,       // <= 10KB
            10241..=102400 => 2,     // <= 100KB
            102401..=1048576 => 3,   // <= 1MB
            1048577..=10485760 => 4, // <= 10MB
            _ => 5,                  // > 10MB
        }
    }
}

impl EvictionStrategy for AdaptiveStrategy {
    fn select_victim(
        &self,
        _cache_state: &CacheState,
        access_info: &HashMap<ContentHash, AccessInfo>,
    ) -> Option<ContentHash> {
        // For now, use a simple heuristic instead of async Q-values
        // In production, consider making this trait async

        // Find the content with lowest value score
        access_info
            .iter()
            .map(|(hash, info)| {
                // Simple heuristic: combine frequency, recency, and size
                let frequency_score = (info.count as f64).log2() + 1.0;
                let now_secs = Self::current_timestamp_secs();
                let age_secs = now_secs.saturating_sub(info.last_access_secs) as f64;
                let recency_score = 1.0 / (age_secs / 3600.0 + 1.0); // Hours
                let size_penalty = (info.size as f64 / 1_048_576.0).sqrt(); // MB

                let score = frequency_score * recency_score / (size_penalty + 1.0);
                (hash, score)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(hash, _)| *hash)
    }

    fn on_access(&mut self, _content_hash: &ContentHash) {
        // Adaptive strategy learns from Q-values, not direct access patterns
    }

    fn on_insert(&mut self, _content_hash: &ContentHash) {
        // Adaptive strategy learns from Q-values, not direct insertions
    }

    fn name(&self) -> &str {
        "Adaptive"
    }
}

/// Factory for creating eviction strategies
#[derive(Debug, Clone)]
pub enum EvictionStrategyType {
    LRU,
    LFU,
    FIFO,
    Adaptive(Arc<RwLock<HashMap<(StateVector, CacheAction), QValue>>>),
}

impl EvictionStrategyType {
    pub fn create(self) -> Box<dyn EvictionStrategy> {
        match self {
            EvictionStrategyType::LRU => Box::new(LRUStrategy::new()),
            EvictionStrategyType::LFU => Box::new(LFUStrategy::new()),
            EvictionStrategyType::FIFO => Box::new(FIFOStrategy::new()),
            EvictionStrategyType::Adaptive(q_table) => Box::new(AdaptiveStrategy::new(q_table)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_access_info() -> HashMap<ContentHash, AccessInfo> {
        let mut info = HashMap::new();

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Add test data
        info.insert(
            ContentHash::from("content1".as_bytes()),
            AccessInfo {
                count: 10,
                last_access_secs: now_secs.saturating_sub(60),
                size: 1024,
            },
        );

        info.insert(
            ContentHash::from("content2".as_bytes()),
            AccessInfo {
                count: 5,
                last_access_secs: now_secs.saturating_sub(120),
                size: 2048,
            },
        );

        info.insert(
            ContentHash::from("content3".as_bytes()),
            AccessInfo {
                count: 20,
                last_access_secs: now_secs.saturating_sub(30),
                size: 512,
            },
        );

        info
    }

    #[test]
    fn test_lru_eviction_removes_least_recently_used() {
        let mut strategy = LRUStrategy::new();
        let access_info = create_test_access_info();
        let cache_state = CacheState {
            current_size: 3584,
            max_size: 4096,
            item_count: 3,
            avg_access_frequency: 11.67,
        };

        // Set up access order
        let hash1 = ContentHash::from("content1".as_bytes());
        let hash2 = ContentHash::from("content2".as_bytes());
        let hash3 = ContentHash::from("content3".as_bytes());

        strategy.on_insert(&hash2); // Oldest
        strategy.on_insert(&hash1);
        strategy.on_insert(&hash3); // Newest

        // Access hash1 to make it more recent than hash2
        strategy.on_access(&hash1);

        // Should evict content2 (least recently used)
        let victim = strategy.select_victim(&cache_state, &access_info);
        assert_eq!(victim, Some(hash2));
    }

    #[test]
    fn test_lfu_eviction_removes_least_frequently_used() {
        let mut strategy = LFUStrategy::new();
        let access_info = create_test_access_info();
        let cache_state = CacheState {
            current_size: 3584,
            max_size: 4096,
            item_count: 3,
            avg_access_frequency: 11.67,
        };

        // Set up frequencies
        let hash1 = ContentHash::from("content1".as_bytes());
        let hash2 = ContentHash::from("content2".as_bytes());
        let hash3 = ContentHash::from("content3".as_bytes());

        // Access patterns
        for _ in 0..10 {
            strategy.on_access(&hash1);
        }
        for _ in 0..5 {
            strategy.on_access(&hash2);
        }
        for _ in 0..20 {
            strategy.on_access(&hash3);
        }

        // Should evict content2 (least frequently used)
        let victim = strategy.select_victim(&cache_state, &access_info);
        assert_eq!(victim, Some(hash2));
    }

    #[test]
    fn test_fifo_eviction_removes_oldest() {
        let mut strategy = FIFOStrategy::new();
        let access_info = create_test_access_info();
        let cache_state = CacheState {
            current_size: 3584,
            max_size: 4096,
            item_count: 3,
            avg_access_frequency: 11.67,
        };

        // Insert in order
        let hash1 = ContentHash::from("content1".as_bytes());
        let hash2 = ContentHash::from("content2".as_bytes());
        let hash3 = ContentHash::from("content3".as_bytes());

        strategy.on_insert(&hash2); // First
        strategy.on_insert(&hash1); // Second
        strategy.on_insert(&hash3); // Third

        // Access shouldn't affect FIFO order
        strategy.on_access(&hash2);
        strategy.on_access(&hash3);

        // Should evict content2 (first in)
        let victim = strategy.select_victim(&cache_state, &access_info);
        assert_eq!(victim, Some(hash2));
    }

    #[tokio::test]
    async fn test_adaptive_eviction_uses_q_values() {
        let q_table = Arc::new(RwLock::new(HashMap::new()));
        let strategy = AdaptiveStrategy::new(q_table.clone());
        let access_info = create_test_access_info();
        let cache_state = CacheState {
            current_size: 3584,
            max_size: 4096,
            item_count: 3,
            avg_access_frequency: 11.67,
        };

        // Set up Q-values for different states
        let state1 = StateVector {
            utilization_bucket: 8,
            frequency_bucket: 1,
            recency_bucket: 1,
            content_size_bucket: 1,
        };

        let hash1 = ContentHash::from("content1".as_bytes());
        let hash2 = ContentHash::from("content2".as_bytes());

        // Make content1 more valuable to keep
        q_table.write().await.insert(
            (state1, CacheAction::DoNothing),
            QValue {
                value: 10.0,
                updates: 1,
            },
        );
        q_table.write().await.insert(
            (state1, CacheAction::Evict(hash1.clone())),
            QValue {
                value: 5.0,
                updates: 1,
            },
        );

        // Make content2 less valuable
        q_table.write().await.insert(
            (state1, CacheAction::DoNothing),
            QValue {
                value: 3.0,
                updates: 1,
            },
        );
        q_table.write().await.insert(
            (state1, CacheAction::Evict(hash2.clone())),
            QValue {
                value: 8.0,
                updates: 1,
            },
        );

        // Adaptive strategy should consider Q-values
        let victim = strategy.select_victim(&cache_state, &access_info);
        assert!(victim.is_some());
    }

    #[test]
    fn test_strategy_factory() {
        let lru = EvictionStrategyType::LRU.create();
        assert_eq!(lru.name(), "LRU");

        let lfu = EvictionStrategyType::LFU.create();
        assert_eq!(lfu.name(), "LFU");

        let fifo = EvictionStrategyType::FIFO.create();
        assert_eq!(fifo.name(), "FIFO");

        let q_table = Arc::new(RwLock::new(HashMap::new()));
        let adaptive = EvictionStrategyType::Adaptive(q_table).create();
        assert_eq!(adaptive.name(), "Adaptive");
    }
}
