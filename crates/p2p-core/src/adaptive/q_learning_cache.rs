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

//! # Q-Learning Cache Management
//!
//! This module implements Q-Learning for intelligent cache management in the P2P network.
//! It learns optimal caching policies based on content access patterns, node capabilities,
//! and network conditions.
//!
//! ## Features
//! - State discretization for continuous cache metrics
//! - Action space: cache, evict, replicate, do nothing
//! - ε-greedy exploration strategy
//! - Experience replay for stable learning
//! - Adaptive to changing access patterns

// use super::*; // Removed unused import
use super::eviction::{EvictionStrategy, EvictionStrategyType, CacheState};
use crate::Result;
use crate::identity::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use super::ContentHash;

/// State vector representing cache state in discrete buckets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateVector {
    /// Cache utilization: 0-10 (0%, 10%, ..., 100%)
    pub utilization_bucket: u8,
    /// Access frequency: 0-5 (very low to very high)
    pub frequency_bucket: u8,
    /// Recency of access: 0-5 (very old to very recent)
    pub recency_bucket: u8,
    /// Content size: 0-4 (tiny, small, medium, large, huge)
    pub content_size_bucket: u8,
}

impl StateVector {
    /// Create a new state vector from continuous values
    pub fn from_metrics(
        utilization: f64,      // 0.0 to 1.0
        frequency: f64,        // accesses per hour
        recency_seconds: u64,  // seconds since last access
        content_size: u64,     // bytes
    ) -> Self {
        // Discretize utilization (0-100% -> 0-10)
        let utilization_bucket = (utilization * 10.0).min(10.0) as u8;
        
        // Discretize frequency (log scale)
        let frequency_bucket = match frequency {
            f if f < 1.0 => 0,    // < 1/hour
            f if f < 5.0 => 1,    // 1-5/hour
            f if f < 20.0 => 2,   // 5-20/hour
            f if f < 100.0 => 3,  // 20-100/hour
            f if f < 500.0 => 4,  // 100-500/hour
            _ => 5,               // 500+/hour
        };
        
        // Discretize recency (log scale)
        let recency_bucket = match recency_seconds {
            r if r < 60 => 5,           // < 1 minute
            r if r < 600 => 4,          // 1-10 minutes
            r if r < 3600 => 3,         // 10-60 minutes
            r if r < 86400 => 2,        // 1-24 hours
            r if r < 604800 => 1,       // 1-7 days
            _ => 0,                     // > 7 days
        };
        
        // Discretize content size
        let content_size_bucket = match content_size {
            s if s < 1024 => 0,              // < 1KB (tiny)
            s if s < 1024 * 100 => 1,        // 1-100KB (small)
            s if s < 1024 * 1024 => 2,       // 100KB-1MB (medium)
            s if s < 1024 * 1024 * 10 => 3,  // 1-10MB (large)
            _ => 4,                          // > 10MB (huge)
        };
        
        Self {
            utilization_bucket,
            frequency_bucket,
            recency_bucket,
            content_size_bucket,
        }
    }
    
    /// Get the total number of possible states
    pub fn state_space_size() -> usize {
        11 * 6 * 6 * 5  // utilization * frequency * recency * size
    }
}

/// Actions that can be taken by the cache manager
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheAction {
    /// Cache the content locally
    Cache(ContentHash),
    /// Evict content from cache
    Evict(ContentHash),
    /// Replicate content to another node
    Replicate { content: ContentHash, target: NodeId },
    /// Do nothing
    DoNothing,
}

impl CacheAction {
    /// Get a simplified action type for Q-table indexing
    pub fn action_type(&self) -> ActionType {
        match self {
            CacheAction::Cache(_) => ActionType::Cache,
            CacheAction::Evict(_) => ActionType::Evict,
            CacheAction::Replicate { .. } => ActionType::Replicate,
            CacheAction::DoNothing => ActionType::DoNothing,
        }
    }
}

/// Simplified action types for Q-table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionType {
    Cache,
    Evict,
    Replicate,
    DoNothing,
}

/// Experience tuple for replay buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    /// State before action
    pub state: StateVector,
    /// Action taken
    pub action: CacheAction,
    /// Reward received
    pub reward: f64,
    /// State after action
    pub next_state: StateVector,
    /// Whether this was a terminal state
    pub terminal: bool,
}

/// Cache statistics for tracking performance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Total cache capacity in bytes
    pub capacity: u64,
    /// Current cache usage in bytes
    pub usage: u64,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Content access frequency map
    pub access_frequency: HashMap<ContentHash, AccessInfo>,
    /// Total number of evictions
    pub evictions: u64,
    /// Total number of replications
    pub replications: u64,
}

impl CacheStatistics {
    /// Calculate cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
    
    /// Calculate cache utilization
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.usage as f64 / self.capacity as f64
        }
    }
}

/// Information about content access patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessInfo {
    /// Number of accesses
    pub count: u64,
    /// Last access time (as Unix timestamp for serialization)
    pub last_access_secs: u64,
    /// Content size
    pub size: u64,
}

/// Configuration for Q-Learning cache manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QLearningConfig {
    /// Learning rate (alpha)
    pub learning_rate: f64,
    /// Discount factor (gamma)
    pub discount_factor: f64,
    /// Exploration rate (epsilon)
    pub epsilon: f64,
    /// Epsilon decay rate
    pub epsilon_decay: f64,
    /// Minimum epsilon value
    pub epsilon_min: f64,
    /// Experience replay buffer size
    pub buffer_size: usize,
    /// Batch size for learning
    pub batch_size: usize,
    /// Learning frequency (episodes)
    pub learning_frequency: u32,
    /// Eviction strategy type
    #[serde(skip)]
    pub eviction_strategy: Option<EvictionStrategyType>,
}

impl Default for QLearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount_factor: 0.9,
            epsilon: 1.0,
            epsilon_decay: 0.995,
            epsilon_min: 0.01,
            buffer_size: 10000,
            batch_size: 32,
            learning_frequency: 10,
            eviction_strategy: None,
        }
    }
}

/// Q-Learning based cache manager
pub struct QLearnCacheManager {
    /// Configuration
    config: QLearningConfig,
    /// Q-table: state -> action -> value
    q_table: Arc<RwLock<HashMap<StateVector, HashMap<ActionType, f64>>>>,
    /// Experience replay buffer
    experience_buffer: Arc<RwLock<VecDeque<Experience>>>,
    /// Cache statistics
    cache_stats: Arc<RwLock<CacheStatistics>>,
    /// Episode counter
    episode_count: Arc<RwLock<u32>>,
    /// Current epsilon value
    current_epsilon: Arc<RwLock<f64>>,
    /// Eviction strategy
    eviction_strategy: Arc<RwLock<Box<dyn EvictionStrategy>>>,
}

impl QLearnCacheManager {
    /// Get current timestamp in seconds since UNIX_EPOCH
    fn current_timestamp_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    
    /// Create a new Q-Learning cache manager
    pub fn new(config: QLearningConfig, capacity: u64) -> Self {
        let cache_stats = CacheStatistics {
            capacity,
            ..Default::default()
        };
        
        // Create eviction strategy
        let eviction_strategy: Box<dyn EvictionStrategy> = match &config.eviction_strategy {
            Some(strategy_type) => match strategy_type {
                EvictionStrategyType::LRU => Box::new(super::eviction::LRUStrategy::new()),
                EvictionStrategyType::LFU => Box::new(super::eviction::LFUStrategy::new()),
                EvictionStrategyType::FIFO => Box::new(super::eviction::FIFOStrategy::new()),
                EvictionStrategyType::Adaptive(q_table) => Box::new(super::eviction::AdaptiveStrategy::new(q_table.clone())),
            },
            None => Box::new(super::eviction::LRUStrategy::new()), // Default to LRU
        };
        
        Self {
            config: config.clone(),
            q_table: Arc::new(RwLock::new(HashMap::new())),
            experience_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(config.buffer_size))),
            cache_stats: Arc::new(RwLock::new(cache_stats)),
            episode_count: Arc::new(RwLock::new(0)),
            current_epsilon: Arc::new(RwLock::new(config.epsilon)),
            eviction_strategy: Arc::new(RwLock::new(eviction_strategy)),
        }
    }
    
    /// Get Q-value for a state-action pair
    pub async fn get_q_value(&self, state: &StateVector, action: ActionType) -> f64 {
        let q_table = self.q_table.read().await;
        q_table
            .get(state)
            .and_then(|actions| actions.get(&action))
            .copied()
            .unwrap_or(0.0)
    }
    
    /// Update Q-value using Bellman equation
    pub async fn update_q_value(
        &self,
        state: &StateVector,
        action: ActionType,
        reward: f64,
        next_state: &StateVector,
        terminal: bool,
    ) -> Result<()> {
        let mut q_table = self.q_table.write().await;
        
        // Get current Q-value
        let current_q = q_table
            .get(state)
            .and_then(|actions| actions.get(&action))
            .copied()
            .unwrap_or(0.0);
        
        // Calculate target value
        let target = if terminal {
            reward
        } else {
            // Get max Q-value for next state
            let max_next_q = self.get_max_q_value_locked(&q_table, next_state);
            reward + self.config.discount_factor * max_next_q
        };
        
        // Update Q-value using Bellman equation
        let new_q = current_q + self.config.learning_rate * (target - current_q);
        
        // Store updated value
        q_table
            .entry(*state)
            .or_insert_with(HashMap::new)
            .insert(action, new_q);
        
        Ok(())
    }
    
    /// Get maximum Q-value for a state (with lock already held)
    fn get_max_q_value_locked(
        &self,
        q_table: &HashMap<StateVector, HashMap<ActionType, f64>>,
        state: &StateVector,
    ) -> f64 {
        q_table
            .get(state)
            .map(|actions| {
                actions.values().copied().fold(0.0f64, |a, b| a.max(b))
            })
            .unwrap_or(0.0)
    }
    
    /// Select action using ε-greedy policy
    pub async fn select_action(
        &self,
        state: &StateVector,
        available_actions: Vec<CacheAction>,
    ) -> Result<CacheAction> {
        if available_actions.is_empty() {
            return Ok(CacheAction::DoNothing);
        }
        
        let epsilon = *self.current_epsilon.read().await;
        
        // Exploration: random action
        if rand::random::<f64>() < epsilon {
            let idx = rand::random::<usize>() % available_actions.len();
            return Ok(available_actions[idx].clone());
        }
        
        // Exploitation: best Q-value
        let q_table = self.q_table.read().await;
        let mut best_action = &available_actions[0];
        let mut best_q = f64::NEG_INFINITY;
        
        for action in &available_actions {
            let action_type = action.action_type();
            let q_value = q_table
                .get(state)
                .and_then(|actions| actions.get(&action_type))
                .copied()
                .unwrap_or(0.0);
            
            if q_value > best_q {
                best_q = q_value;
                best_action = action;
            }
        }
        
        Ok(best_action.clone())
    }
    
    /// Add experience to replay buffer
    pub async fn add_experience(&self, experience: Experience) -> Result<()> {
        let mut buffer = self.experience_buffer.write().await;
        
        // Remove oldest if buffer is full
        if buffer.len() >= self.config.buffer_size {
            buffer.pop_front();
        }
        
        buffer.push_back(experience);
        
        // Check if we should learn from replay
        let episode_count = {
            let mut count = self.episode_count.write().await;
            *count += 1;
            *count
        };
        
        if episode_count % self.config.learning_frequency == 0 {
            drop(buffer); // Release lock before learning
            self.learn_from_replay().await?;
        }
        
        Ok(())
    }
    
    /// Learn from experience replay buffer
    pub async fn learn_from_replay(&self) -> Result<()> {
        let buffer = self.experience_buffer.read().await;
        
        if buffer.len() < self.config.batch_size {
            return Ok(());
        }
        
        // Sample random batch
        let mut batch = Vec::new();
        for _ in 0..self.config.batch_size {
            let idx = rand::random::<usize>() % buffer.len();
            batch.push(buffer[idx].clone());
        }
        
        drop(buffer); // Release lock before updating Q-values
        
        // Update Q-values for batch
        for experience in batch {
            self.update_q_value(
                &experience.state,
                experience.action.action_type(),
                experience.reward,
                &experience.next_state,
                experience.terminal,
            ).await?;
        }
        
        // Decay epsilon
        let mut epsilon = self.current_epsilon.write().await;
        *epsilon = (*epsilon * self.config.epsilon_decay).max(self.config.epsilon_min);
        
        Ok(())
    }
    
    /// Calculate reward for a cache action
    pub async fn calculate_reward(
        &self,
        action: &CacheAction,
        hit: bool,
        old_utilization: f64,
        new_utilization: f64,
    ) -> f64 {
        let mut reward = 0.0;
        
        // Reward for cache hits
        if hit {
            reward += 1.0;
        }
        
        // Penalty for cache misses
        if !hit && matches!(action, CacheAction::DoNothing) {
            reward -= 0.5;
        }
        
        // Reward for good cache decisions
        match action {
            CacheAction::Cache(_) => {
                // Reward if caching when utilization is low
                if old_utilization < 0.8 {
                    reward += 0.2;
                } else {
                    reward -= 0.1; // Penalty for caching when almost full
                }
            }
            CacheAction::Evict(_) => {
                // Reward for evicting when cache is full
                if old_utilization > 0.9 {
                    reward += 0.3;
                } else if old_utilization < 0.5 {
                    reward -= 0.2; // Penalty for evicting when plenty of space
                }
            }
            CacheAction::Replicate { .. } => {
                // Small reward for replication (helps availability)
                reward += 0.1;
            }
            CacheAction::DoNothing => {
                // Neutral unless it caused a miss
            }
        }
        
        // Penalty for extreme utilization
        if new_utilization > 0.95 {
            reward -= 0.3;
        } else if new_utilization < 0.3 {
            reward -= 0.1; // Slight penalty for underutilization
        }
        
        reward
    }
    
    /// Update cache statistics after an action
    pub async fn update_statistics(
        &self,
        action: &CacheAction,
        content_hash: &ContentHash,
        content_size: u64,
        hit: bool,
    ) -> Result<()> {
        let mut stats = self.cache_stats.write().await;
        
        if hit {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }
        
        match action {
            CacheAction::Cache(_) => {
                stats.usage = (stats.usage + content_size).min(stats.capacity);
                stats.access_frequency.insert(
                    content_hash.clone(),
                    AccessInfo {
                        count: 1,
                        last_access_secs: Self::current_timestamp_secs(),
                        size: content_size,
                    },
                );
                // Notify eviction strategy
                drop(stats);
                self.eviction_strategy.write().await.on_insert(content_hash);
                stats = self.cache_stats.write().await;
            }
            CacheAction::Evict(hash) => {
                if let Some(info) = stats.access_frequency.get(hash) {
                    stats.usage = stats.usage.saturating_sub(info.size);
                    stats.evictions += 1;
                }
                stats.access_frequency.remove(hash);
            }
            CacheAction::Replicate { .. } => {
                stats.replications += 1;
            }
            CacheAction::DoNothing => {}
        }
        
        // Update access info for existing content
        if let Some(info) = stats.access_frequency.get_mut(content_hash) {
            info.count += 1;
            info.last_access_secs = Self::current_timestamp_secs();
            // Notify eviction strategy
            drop(stats);
            self.eviction_strategy.write().await.on_access(content_hash);
        }
        
        Ok(())
    }
    
    /// Get current cache state as a state vector
    pub async fn get_current_state(&self, content_hash: &ContentHash) -> Result<StateVector> {
        let stats = self.cache_stats.read().await;
        
        let utilization = stats.utilization();
        
        let (frequency, recency_seconds, content_size) = if let Some(info) = stats.access_frequency.get(content_hash) {
            let now_secs = Self::current_timestamp_secs();
            let elapsed_secs = now_secs.saturating_sub(info.last_access_secs);
            let hours_elapsed = elapsed_secs as f64 / 3600.0;
            let frequency = if hours_elapsed > 0.0 {
                info.count as f64 / hours_elapsed
            } else {
                info.count as f64
            };
            (frequency, elapsed_secs, info.size)
        } else {
            (0.0, u64::MAX, 0)
        };
        
        Ok(StateVector::from_metrics(
            utilization,
            frequency,
            recency_seconds,
            content_size,
        ))
    }
    
    /// Get available actions for current state
    pub async fn get_available_actions(
        &self,
        content_hash: &ContentHash,
        content_size: u64,
    ) -> Result<Vec<CacheAction>> {
        let stats = self.cache_stats.read().await;
        let mut actions = vec![CacheAction::DoNothing];
        
        // Can cache if not already cached and have space
        if !stats.access_frequency.contains_key(content_hash) {
            if stats.usage + content_size <= stats.capacity {
                actions.push(CacheAction::Cache(content_hash.clone()));
            } else {
                // Need to evict something first - use eviction strategy
                let cache_state = CacheState {
                    current_size: stats.usage,
                    max_size: stats.capacity,
                    item_count: stats.access_frequency.len(),
                    avg_access_frequency: if stats.access_frequency.is_empty() {
                        0.0
                    } else {
                        stats.access_frequency.values()
                            .map(|info| info.count as f64)
                            .sum::<f64>() / stats.access_frequency.len() as f64
                    },
                };
                
                let eviction_strategy = self.eviction_strategy.read().await;
                if let Some(victim) = eviction_strategy.select_victim(&cache_state, &stats.access_frequency) {
                    actions.push(CacheAction::Evict(victim));
                }
            }
        }
        
        // Can always replicate (if we have peers)
        // In real implementation, would check available peers
        
        Ok(actions)
    }
    
    /// Set eviction strategy at runtime
    pub async fn set_eviction_strategy(&self, strategy: Box<dyn EvictionStrategy>) {
        *self.eviction_strategy.write().await = strategy;
    }
    
    /// Get current eviction strategy name
    pub async fn get_eviction_strategy_name(&self) -> String {
        self.eviction_strategy.read().await.name().to_string()
    }
    
    /// Reset Q-table and statistics
    pub async fn reset(&self) {
        *self.q_table.write().await = HashMap::new();
        *self.experience_buffer.write().await = VecDeque::with_capacity(self.config.buffer_size);
        *self.episode_count.write().await = 0;
        *self.current_epsilon.write().await = self.config.epsilon;
        
        let mut stats = self.cache_stats.write().await;
        let capacity = stats.capacity;
        *stats = CacheStatistics {
            capacity,
            ..Default::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_discretization() {
        // Test utilization discretization
        let state1 = StateVector::from_metrics(0.0, 0.0, 0, 0);
        assert_eq!(state1.utilization_bucket, 0);
        
        let state2 = StateVector::from_metrics(0.55, 0.0, 0, 0);
        assert_eq!(state2.utilization_bucket, 5);
        
        let state3 = StateVector::from_metrics(1.0, 0.0, 0, 0);
        assert_eq!(state3.utilization_bucket, 10);
        
        // Test frequency discretization
        let state4 = StateVector::from_metrics(0.0, 0.5, 0, 0);
        assert_eq!(state4.frequency_bucket, 0);
        
        let state5 = StateVector::from_metrics(0.0, 50.0, 0, 0);
        assert_eq!(state5.frequency_bucket, 3);
        
        let state6 = StateVector::from_metrics(0.0, 1000.0, 0, 0);
        assert_eq!(state6.frequency_bucket, 5);
        
        // Test recency discretization
        let state7 = StateVector::from_metrics(0.0, 0.0, 30, 0);
        assert_eq!(state7.recency_bucket, 5); // < 1 minute
        
        let state8 = StateVector::from_metrics(0.0, 0.0, 3600, 0);
        assert_eq!(state8.recency_bucket, 3); // 1 hour
        
        let state9 = StateVector::from_metrics(0.0, 0.0, 1000000, 0);
        assert_eq!(state9.recency_bucket, 0); // > 7 days
        
        // Test content size discretization
        let state10 = StateVector::from_metrics(0.0, 0.0, 0, 500);
        assert_eq!(state10.content_size_bucket, 0); // tiny
        
        let state11 = StateVector::from_metrics(0.0, 0.0, 0, 500_000);
        assert_eq!(state11.content_size_bucket, 2); // medium
        
        let state12 = StateVector::from_metrics(0.0, 0.0, 0, 20_000_000);
        assert_eq!(state12.content_size_bucket, 4); // huge
    }
    
    #[test]
    fn test_state_features() {
        // Test complete state creation
        let state = StateVector::from_metrics(
            0.75,      // 75% utilization
            25.0,      // 25 accesses/hour
            300,       // 5 minutes ago
            1_048_576, // 1MB
        );
        
        assert_eq!(state.utilization_bucket, 7);
        assert_eq!(state.frequency_bucket, 3);
        assert_eq!(state.recency_bucket, 4);
        assert_eq!(state.content_size_bucket, 2);
        
        // Test state space size calculation
        assert_eq!(StateVector::state_space_size(), 11 * 6 * 6 * 5);
    }
    
    #[test]
    fn test_action_types() {
        let content_hash = ContentHash([1u8; 32]);
        let node_id = NodeId::from_bytes([2u8; 32]);
        
        let cache_action = CacheAction::Cache(content_hash.clone());
        assert_eq!(cache_action.action_type(), ActionType::Cache);
        
        let evict_action = CacheAction::Evict(content_hash.clone());
        assert_eq!(evict_action.action_type(), ActionType::Evict);
        
        let replicate_action = CacheAction::Replicate {
            content: content_hash,
            target: node_id,
        };
        assert_eq!(replicate_action.action_type(), ActionType::Replicate);
        
        let do_nothing = CacheAction::DoNothing;
        assert_eq!(do_nothing.action_type(), ActionType::DoNothing);
    }
    
    #[test]
    fn test_cache_statistics() {
        let mut stats = CacheStatistics {
            capacity: 1000,
            usage: 750,
            hits: 80,
            misses: 20,
            ..Default::default()
        };
        
        // Test hit rate calculation
        assert_eq!(stats.hit_rate(), 0.8);
        
        // Test utilization calculation
        assert_eq!(stats.utilization(), 0.75);
        
        // Test with zero total
        stats.hits = 0;
        stats.misses = 0;
        assert_eq!(stats.hit_rate(), 0.0);
        
        // Test with zero capacity
        stats.capacity = 0;
        assert_eq!(stats.utilization(), 0.0);
    }
    
    #[test]
    fn test_q_learning_config() {
        let config = QLearningConfig::default();
        
        assert_eq!(config.learning_rate, 0.1);
        assert_eq!(config.discount_factor, 0.9);
        assert_eq!(config.epsilon, 1.0);
        assert_eq!(config.epsilon_decay, 0.995);
        assert_eq!(config.epsilon_min, 0.01);
        assert_eq!(config.buffer_size, 10000);
        assert_eq!(config.batch_size, 32);
        assert_eq!(config.learning_frequency, 10);
    }
    
    #[tokio::test]
    async fn test_q_table_initialization() {
        let config = QLearningConfig::default();
        let manager = QLearnCacheManager::new(config, 1000);
        
        // Q-table should be empty initially
        let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
        let q_value = manager.get_q_value(&state, ActionType::Cache).await;
        assert_eq!(q_value, 0.0);
    }
    
    #[tokio::test]
    async fn test_q_value_updates() -> Result<()> {
        let config = QLearningConfig {
            learning_rate: 0.5,
            discount_factor: 0.9,
            ..Default::default()
        };
        let manager = QLearnCacheManager::new(config, 1000);
        
        let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
        let next_state = StateVector::from_metrics(0.6, 11.0, 60, 1024);
        
        // Update Q-value
        manager.update_q_value(
            &state,
            ActionType::Cache,
            1.0,  // reward
            &next_state,
            false, // not terminal
        ).await?;
        
        // Check Q-value was updated
        let q_value = manager.get_q_value(&state, ActionType::Cache).await;
        assert!(q_value > 0.0);
        assert_eq!(q_value, 0.5); // learning_rate * reward = 0.5 * 1.0
        
        // Update again to test Bellman equation
        manager.update_q_value(
            &state,
            ActionType::Cache,
            0.5,
            &next_state,
            false,
        ).await?;
        
        let new_q_value = manager.get_q_value(&state, ActionType::Cache).await;
        // Should incorporate previous value: 0.5 + 0.5 * (0.5 - 0.5) = 0.5
        assert_eq!(new_q_value, 0.5);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_epsilon_greedy_selection() -> Result<()> {
        let config = QLearningConfig {
            epsilon: 1.0,  // Always explore
            ..Default::default()
        };
        let manager = QLearnCacheManager::new(config, 1000);
        
        let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
        let content_hash = ContentHash([1u8; 32]);
        let actions = vec![
            CacheAction::Cache(content_hash.clone()),
            CacheAction::DoNothing,
        ];
        
        // With epsilon=1.0, should randomly select
        let mut cache_count = 0;
        let mut nothing_count = 0;
        
        for _ in 0..100 {
            let action = manager.select_action(&state, actions.clone()).await?;
            match action {
                CacheAction::Cache(_) => cache_count += 1,
                CacheAction::DoNothing => nothing_count += 1,
                _ => panic!("Unexpected action"),
            }
        }
        
        // Both actions should be selected sometimes
        assert!(cache_count > 0);
        assert!(nothing_count > 0);
        
        // Now test exploitation
        *manager.current_epsilon.write().await = 0.0;
        
        // Set Q-values so Cache has higher value
        manager.update_q_value(
            &state,
            ActionType::Cache,
            10.0,
            &state,
            true,
        ).await?;
        
        // Should always select Cache action now
        for _ in 0..10 {
            let action = manager.select_action(&state, actions.clone()).await?;
            assert!(matches!(action, CacheAction::Cache(_)));
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_experience_storage() -> Result<()> {
        let config = QLearningConfig {
            buffer_size: 5,
            batch_size: 2,
            learning_frequency: 3,
            ..Default::default()
        };
        let manager = QLearnCacheManager::new(config, 1000);
        
        let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
        let content_hash = ContentHash([1u8; 32]);
        
        // Add experiences
        for i in 0..7 {
            let experience = Experience {
                state,
                action: CacheAction::Cache(content_hash.clone()),
                reward: i as f64,
                next_state: state,
                terminal: false,
            };
            manager.add_experience(experience).await?;
        }
        
        // Buffer should be at capacity (5)
        let buffer_len = manager.experience_buffer.read().await.len();
        assert_eq!(buffer_len, 5);
        
        // Epsilon should have decayed (learning happens every 3 episodes)
        let epsilon = *manager.current_epsilon.read().await;
        assert!(epsilon < 1.0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_reward_calculation() {
        let config = QLearningConfig::default();
        let manager = QLearnCacheManager::new(config, 1000);
        
        let content_hash = ContentHash([1u8; 32]);
        
        // Test cache hit reward
        let reward = manager.calculate_reward(
            &CacheAction::DoNothing,
            true,  // hit
            0.5,   // old utilization
            0.5,   // new utilization
        ).await;
        assert_eq!(reward, 1.0);
        
        // Test cache miss penalty
        let reward = manager.calculate_reward(
            &CacheAction::DoNothing,
            false, // miss
            0.5,
            0.5,
        ).await;
        assert_eq!(reward, -0.5);
        
        // Test good caching decision
        let reward = manager.calculate_reward(
            &CacheAction::Cache(content_hash.clone()),
            false,
            0.3,  // low utilization
            0.4,
        ).await;
        assert_eq!(reward, 0.2);
        
        // Test bad caching decision (cache full)
        let reward = manager.calculate_reward(
            &CacheAction::Cache(content_hash.clone()),
            false,
            0.85, // high utilization
            0.96, // very high after
        ).await;
        assert_eq!(reward, -0.1 - 0.3); // penalty for caching when full + extreme utilization
        
        // Test good eviction
        let reward = manager.calculate_reward(
            &CacheAction::Evict(content_hash.clone()),
            false,
            0.95, // very high utilization
            0.85,
        ).await;
        assert_eq!(reward, 0.3);
        
        // Test bad eviction
        let reward = manager.calculate_reward(
            &CacheAction::Evict(content_hash),
            false,
            0.4,  // low utilization
            0.3,
        ).await;
        assert_eq!(reward, -0.2);
    }
    
    #[tokio::test]
    async fn test_cache_statistics_updates() -> Result<()> {
        let config = QLearningConfig::default();
        let manager = QLearnCacheManager::new(config, 1000);
        
        let content_hash = ContentHash([1u8; 32]);
        
        // Test cache action
        manager.update_statistics(
            &CacheAction::Cache(content_hash.clone()),
            &content_hash,
            100,
            false,
        ).await?;
        
        let stats = manager.cache_stats.read().await;
        assert_eq!(stats.usage, 100);
        assert_eq!(stats.misses, 1);
        assert!(stats.access_frequency.contains_key(&content_hash));
        drop(stats);
        
        // Test hit
        manager.update_statistics(
            &CacheAction::DoNothing,
            &content_hash,
            100,
            true,
        ).await?;
        
        let stats = manager.cache_stats.read().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.access_frequency[&content_hash].count, 2);
        drop(stats);
        
        // Test eviction
        manager.update_statistics(
            &CacheAction::Evict(content_hash.clone()),
            &content_hash,
            100,
            false,
        ).await?;
        
        let stats = manager.cache_stats.read().await;
        assert_eq!(stats.usage, 0);
        assert_eq!(stats.evictions, 1);
        assert!(!stats.access_frequency.contains_key(&content_hash));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_available_actions() -> Result<()> {
        let config = QLearningConfig::default();
        let manager = QLearnCacheManager::new(config, 200);
        
        let content1 = ContentHash([1u8; 32]);
        let content2 = ContentHash([2u8; 32]);
        
        // Empty cache - can cache
        let actions = manager.get_available_actions(&content1, 100).await?;
        assert_eq!(actions.len(), 2); // DoNothing, Cache
        assert!(actions.iter().any(|a| matches!(a, CacheAction::Cache(_))));
        
        // Add content to cache
        manager.update_statistics(
            &CacheAction::Cache(content1.clone()),
            &content1,
            150,
            false,
        ).await?;
        
        // Cache nearly full - should suggest eviction
        let actions = manager.get_available_actions(&content2, 100).await?;
        assert!(actions.iter().any(|a| matches!(a, CacheAction::Evict(_))));
        
        // Already cached content - only DoNothing
        let actions = manager.get_available_actions(&content1, 150).await?;
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], CacheAction::DoNothing));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_reset() -> Result<()> {
        let config = QLearningConfig::default();
        let manager = QLearnCacheManager::new(config, 1000);
        
        // Add some data
        let state = StateVector::from_metrics(0.5, 10.0, 300, 1024);
        manager.update_q_value(
            &state,
            ActionType::Cache,
            1.0,
            &state,
            true,
        ).await?;
        
        let content_hash = ContentHash([1u8; 32]);
        manager.update_statistics(
            &CacheAction::Cache(content_hash),
            &content_hash,
            100,
            false,
        ).await?;
        
        // Reset
        manager.reset().await;
        
        // Check everything is cleared
        assert_eq!(manager.get_q_value(&state, ActionType::Cache).await, 0.0);
        assert_eq!(*manager.episode_count.read().await, 0);
        assert_eq!(*manager.current_epsilon.read().await, 1.0);
        
        let stats = manager.cache_stats.read().await;
        assert_eq!(stats.usage, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert!(stats.access_frequency.is_empty());
        
        Ok(())
    }
}