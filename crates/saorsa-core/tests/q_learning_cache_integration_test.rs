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

//! Integration tests for Q-Learning Cache Management

use saorsa_core::adaptive::{
    ContentHash, QLearningCacheManager, QLearningConfig, q_learning_cache::Experience,
};
use std::collections::HashMap;
use std::time::Instant;
use tokio;

/// Simulate different workload patterns
#[derive(Debug, Clone)]
enum WorkloadPattern {
    /// Uniform random access
    Uniform,
    /// Zipf distribution (power law)
    Zipf { alpha: f64 },
    /// Temporal locality (recent items more likely)
    Temporal { window: usize },
    /// Sequential access
    Sequential,
}

/// Workload generator for testing
struct WorkloadGenerator {
    pattern: WorkloadPattern,
    content_pool: Vec<(ContentHash, u64)>, // (hash, size)
    access_history: Vec<ContentHash>,
    step: usize,
}

impl WorkloadGenerator {
    fn new(pattern: WorkloadPattern, num_items: usize) -> Self {
        let mut content_pool = Vec::new();
        for i in 0..num_items {
            let hash = ContentHash([i as u8; 32]);
            let size = match i % 4 {
                0 => 1024,        // 1KB
                1 => 10 * 1024,   // 10KB
                2 => 100 * 1024,  // 100KB
                _ => 1024 * 1024, // 1MB
            };
            content_pool.push((hash, size));
        }

        Self {
            pattern,
            content_pool,
            access_history: Vec::new(),
            step: 0,
        }
    }

    fn next_access(&mut self) -> (ContentHash, u64) {
        let idx = match &self.pattern {
            WorkloadPattern::Uniform => rand::random::<usize>() % self.content_pool.len(),
            WorkloadPattern::Zipf { alpha } => {
                // Simple Zipf approximation
                let u: f64 = rand::random();
                let n = self.content_pool.len() as f64;
                ((n.powf(1.0 - alpha) - 1.0) * u + 1.0).powf(1.0 / (1.0 - alpha)) as usize - 1
            }
            WorkloadPattern::Temporal { window } => {
                if self.access_history.len() > *window && rand::random::<f64>() < 0.8 {
                    // 80% chance to access recent items
                    let recent_idx =
                        self.access_history.len() - 1 - (rand::random::<usize>() % window);
                    let recent_hash = &self.access_history[recent_idx];
                    self.content_pool
                        .iter()
                        .position(|(h, _)| h == recent_hash)
                        .unwrap()
                } else {
                    rand::random::<usize>() % self.content_pool.len()
                }
            }
            WorkloadPattern::Sequential => {
                let idx = self.step % self.content_pool.len();
                self.step += 1;
                idx
            }
        };

        let (hash, size) = self.content_pool[idx].clone();
        self.access_history.push(hash.clone());
        (hash, size)
    }
}

/// Simple LRU cache for comparison
struct LRUCache {
    capacity: u64,
    usage: u64,
    items: HashMap<ContentHash, (u64, Instant)>, // size, last_access
    hits: u64,
    misses: u64,
}

impl LRUCache {
    fn new(capacity: u64) -> Self {
        Self {
            capacity,
            usage: 0,
            items: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    fn access(&mut self, hash: &ContentHash, size: u64) -> bool {
        if let Some((_, last_access)) = self.items.get_mut(hash) {
            *last_access = Instant::now();
            self.hits += 1;
            true
        } else {
            self.misses += 1;

            // Evict if necessary
            while self.usage + size > self.capacity && !self.items.is_empty() {
                let evict_hash = self
                    .items
                    .iter()
                    .min_by_key(|(_, (_, last))| last)
                    .map(|(h, _)| h.clone())
                    .unwrap();

                if let Some((evict_size, _)) = self.items.remove(&evict_hash) {
                    self.usage -= evict_size;
                }
            }

            // Cache if there's space
            if self.usage + size <= self.capacity {
                self.items.insert(hash.clone(), (size, Instant::now()));
                self.usage += size;
            }

            false
        }
    }

    fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[tokio::test]
async fn test_q_learning_improves_hit_rate() {
    let cache_capacity = 10 * 1024 * 1024; // 10MB
    let config = QLearningConfig {
        learning_rate: 0.1,
        discount_factor: 0.9,
        epsilon: 0.5, // Start with 50% exploration
        epsilon_decay: 0.99,
        epsilon_min: 0.05,
        buffer_size: 1000,
        batch_size: 32,
        learning_frequency: 10,
    };

    let q_manager = QLearningCacheManager::new(config, cache_capacity);
    let mut workload = WorkloadGenerator::new(
        WorkloadPattern::Zipf { alpha: 1.2 }, // Typical web workload
        100,
    );

    // Training phase
    for _ in 0..5000 {
        let (content_hash, content_size) = workload.next_access();

        // Get current state
        let state = q_manager.get_current_state(&content_hash).await.unwrap();

        // Get available actions
        let actions = q_manager
            .get_available_actions(&content_hash, content_size)
            .await
            .unwrap();

        // Select action
        let action = q_manager.select_action(&state, actions).await.unwrap();

        // Execute action and observe outcome
        let old_stats = q_manager.cache_stats.read().await.clone();
        let old_utilization = old_stats.utilization();

        let hit = old_stats.access_frequency.contains_key(&content_hash);

        q_manager
            .update_statistics(&action, &content_hash, content_size, hit)
            .await
            .unwrap();

        let new_stats = q_manager.cache_stats.read().await.clone();
        let new_utilization = new_stats.utilization();

        // Calculate reward
        let reward = q_manager
            .calculate_reward(&action, hit, old_utilization, new_utilization)
            .await;

        // Get next state
        let next_state = q_manager.get_current_state(&content_hash).await.unwrap();

        // Store experience
        let experience = Experience {
            state,
            action,
            reward,
            next_state,
            terminal: false,
        };

        q_manager.add_experience(experience).await.unwrap();
    }

    // Evaluation phase - compare with LRU
    let mut lru_cache = LRUCache::new(cache_capacity);
    let mut q_hits = 0u64;
    let mut q_misses = 0u64;

    // Reset Q-learning statistics for fair comparison
    q_manager.cache_stats.write().await.hits = 0;
    q_manager.cache_stats.write().await.misses = 0;

    for _ in 0..1000 {
        let (content_hash, content_size) = workload.next_access();

        // LRU decision
        lru_cache.access(&content_hash, content_size);

        // Q-learning decision
        let state = q_manager.get_current_state(&content_hash).await.unwrap();
        let actions = q_manager
            .get_available_actions(&content_hash, content_size)
            .await
            .unwrap();
        let action = q_manager.select_action(&state, actions).await.unwrap();

        let hit = q_manager
            .cache_stats
            .read()
            .await
            .access_frequency
            .contains_key(&content_hash);

        if hit {
            q_hits += 1;
        } else {
            q_misses += 1;
        }

        q_manager
            .update_statistics(&action, &content_hash, content_size, hit)
            .await
            .unwrap();
    }

    let q_hit_rate = q_hits as f64 / (q_hits + q_misses) as f64;
    let lru_hit_rate = lru_cache.hit_rate();

    println!("Q-Learning hit rate: {:.2}%", q_hit_rate * 100.0);
    println!("LRU hit rate: {:.2}%", lru_hit_rate * 100.0);

    // Q-learning should perform at least as well as LRU
    assert!(q_hit_rate >= lru_hit_rate * 0.95); // Allow 5% margin
}

#[tokio::test]
async fn test_q_learning_adapts_to_workload_changes() {
    let cache_capacity = 5 * 1024 * 1024; // 5MB
    let config = QLearningConfig {
        learning_rate: 0.2, // Higher learning rate for faster adaptation
        discount_factor: 0.9,
        epsilon: 0.3,
        epsilon_decay: 0.995,
        epsilon_min: 0.05,
        buffer_size: 500,
        batch_size: 16,
        learning_frequency: 5,
    };

    let q_manager = QLearningCacheManager::new(config, cache_capacity);

    // Phase 1: Sequential workload
    let mut workload = WorkloadGenerator::new(WorkloadPattern::Sequential, 50);

    for _ in 0..1000 {
        let (content_hash, content_size) = workload.next_access();
        let state = q_manager.get_current_state(&content_hash).await.unwrap();
        let actions = q_manager
            .get_available_actions(&content_hash, content_size)
            .await
            .unwrap();
        let action = q_manager.select_action(&state, actions).await.unwrap();

        let hit = q_manager
            .cache_stats
            .read()
            .await
            .access_frequency
            .contains_key(&content_hash);

        q_manager
            .update_statistics(&action, &content_hash, content_size, hit)
            .await
            .unwrap();

        let old_utilization = q_manager.cache_stats.read().await.utilization();
        let reward = q_manager
            .calculate_reward(&action, hit, old_utilization, old_utilization)
            .await;
        let next_state = q_manager.get_current_state(&content_hash).await.unwrap();

        q_manager
            .add_experience(Experience {
                state,
                action,
                reward,
                next_state,
                terminal: false,
            })
            .await
            .unwrap();
    }

    let phase1_stats = q_manager.cache_stats.read().await.clone();
    let phase1_hit_rate = phase1_stats.hit_rate();

    // Phase 2: Switch to temporal locality workload
    let mut workload = WorkloadGenerator::new(WorkloadPattern::Temporal { window: 10 }, 50);

    // Reset hit/miss counters
    q_manager.cache_stats.write().await.hits = 0;
    q_manager.cache_stats.write().await.misses = 0;

    for _ in 0..1000 {
        let (content_hash, content_size) = workload.next_access();
        let state = q_manager.get_current_state(&content_hash).await.unwrap();
        let actions = q_manager
            .get_available_actions(&content_hash, content_size)
            .await
            .unwrap();
        let action = q_manager.select_action(&state, actions).await.unwrap();

        let hit = q_manager
            .cache_stats
            .read()
            .await
            .access_frequency
            .contains_key(&content_hash);

        q_manager
            .update_statistics(&action, &content_hash, content_size, hit)
            .await
            .unwrap();

        let old_utilization = q_manager.cache_stats.read().await.utilization();
        let reward = q_manager
            .calculate_reward(&action, hit, old_utilization, old_utilization)
            .await;
        let next_state = q_manager.get_current_state(&content_hash).await.unwrap();

        q_manager
            .add_experience(Experience {
                state,
                action,
                reward,
                next_state,
                terminal: false,
            })
            .await
            .unwrap();
    }

    let phase2_stats = q_manager.cache_stats.read().await.clone();
    let phase2_hit_rate = phase2_stats.hit_rate();

    println!(
        "Sequential workload hit rate: {:.2}%",
        phase1_hit_rate * 100.0
    );
    println!(
        "Temporal locality hit rate: {:.2}%",
        phase2_hit_rate * 100.0
    );

    // Should adapt and improve hit rate for temporal locality
    assert!(phase2_hit_rate > phase1_hit_rate);
}

#[tokio::test]
async fn test_q_learning_handles_mixed_content_sizes() {
    let cache_capacity = 2 * 1024 * 1024; // 2MB
    let config = QLearningConfig::default();
    let q_manager = QLearningCacheManager::new(config, cache_capacity);

    // Create content with very different sizes
    let small_content = ContentHash([1u8; 32]);
    let medium_content = ContentHash([2u8; 32]);
    let large_content = ContentHash([3u8; 32]);

    let small_size = 1024; // 1KB
    let medium_size = 100 * 1024; // 100KB
    let large_size = 1024 * 1024; // 1MB

    // Train the system
    for i in 0..1000 {
        let (content_hash, content_size) = match i % 10 {
            0..=5 => (small_content.clone(), small_size), // 60% small
            6..=8 => (medium_content.clone(), medium_size), // 30% medium
            _ => (large_content.clone(), large_size),     // 10% large
        };

        let state = q_manager.get_current_state(&content_hash).await.unwrap();
        let actions = q_manager
            .get_available_actions(&content_hash, content_size)
            .await
            .unwrap();
        let action = q_manager.select_action(&state, actions).await.unwrap();

        let hit = q_manager
            .cache_stats
            .read()
            .await
            .access_frequency
            .contains_key(&content_hash);

        let old_stats = q_manager.cache_stats.read().await.clone();
        let old_utilization = old_stats.utilization();

        q_manager
            .update_statistics(&action, &content_hash, content_size, hit)
            .await
            .unwrap();

        let new_utilization = q_manager.cache_stats.read().await.utilization();
        let reward = q_manager
            .calculate_reward(&action, hit, old_utilization, new_utilization)
            .await;
        let next_state = q_manager.get_current_state(&content_hash).await.unwrap();

        q_manager
            .add_experience(Experience {
                state,
                action,
                reward,
                next_state,
                terminal: false,
            })
            .await
            .unwrap();
    }

    // Check learned behavior
    let final_stats = q_manager.cache_stats.read().await.clone();

    // Should prefer caching small items due to better space efficiency
    assert!(final_stats.access_frequency.contains_key(&small_content));

    // Large item should rarely be cached (takes 50% of capacity)
    let large_cached = final_stats.access_frequency.contains_key(&large_content);
    if large_cached {
        // If cached, should have very high access frequency
        let large_info = &final_stats.access_frequency[&large_content];
        assert!(large_info.count > 50); // High threshold
    }

    println!(
        "Final cache utilization: {:.2}%",
        final_stats.utilization() * 100.0
    );
    println!("Hit rate: {:.2}%", final_stats.hit_rate() * 100.0);
}

#[tokio::test]
async fn test_q_learning_convergence() {
    let cache_capacity = 1024 * 1024; // 1MB
    let config = QLearningConfig {
        learning_rate: 0.1,
        discount_factor: 0.9,
        epsilon: 0.5,
        epsilon_decay: 0.99,
        epsilon_min: 0.01,
        buffer_size: 1000,
        batch_size: 32,
        learning_frequency: 10,
    };

    let q_manager = QLearningCacheManager::new(config, cache_capacity);

    // Fixed workload for testing convergence
    let content_items = vec![
        (ContentHash([1u8; 32]), 100 * 1024), // 100KB, accessed frequently
        (ContentHash([2u8; 32]), 200 * 1024), // 200KB, accessed moderately
        (ContentHash([3u8; 32]), 300 * 1024), // 300KB, accessed rarely
        (ContentHash([4u8; 32]), 400 * 1024), // 400KB, accessed very rarely
    ];

    let access_probabilities = vec![0.5, 0.3, 0.15, 0.05];

    // Track Q-values over time
    let mut q_value_history = Vec::new();

    for epoch in 0..100 {
        let mut epoch_q_values = Vec::new();

        for _ in 0..100 {
            // Select content based on probabilities
            let r = rand::random::<f64>();
            let mut cumulative = 0.0;
            let mut selected_idx = 0;

            for (i, &prob) in access_probabilities.iter().enumerate() {
                cumulative += prob;
                if r < cumulative {
                    selected_idx = i;
                    break;
                }
            }

            let (content_hash, content_size) = content_items[selected_idx].clone();

            let state = q_manager.get_current_state(&content_hash).await.unwrap();
            let actions = q_manager
                .get_available_actions(&content_hash, content_size)
                .await
                .unwrap();
            let action = q_manager.select_action(&state, actions).await.unwrap();

            // Record Q-value
            let q_value = q_manager.get_q_value(&state, action.action_type()).await;
            epoch_q_values.push(q_value);

            let hit = q_manager
                .cache_stats
                .read()
                .await
                .access_frequency
                .contains_key(&content_hash);

            let old_utilization = q_manager.cache_stats.read().await.utilization();
            q_manager
                .update_statistics(&action, &content_hash, content_size, hit)
                .await
                .unwrap();
            let new_utilization = q_manager.cache_stats.read().await.utilization();

            let reward = q_manager
                .calculate_reward(&action, hit, old_utilization, new_utilization)
                .await;
            let next_state = q_manager.get_current_state(&content_hash).await.unwrap();

            q_manager
                .add_experience(Experience {
                    state,
                    action,
                    reward,
                    next_state,
                    terminal: false,
                })
                .await
                .unwrap();
        }

        let avg_q_value = epoch_q_values.iter().sum::<f64>() / epoch_q_values.len() as f64;
        q_value_history.push(avg_q_value);
    }

    // Check for convergence - Q-values should stabilize
    let last_10_avg = q_value_history[90..].iter().sum::<f64>() / 10.0;
    let prev_10_avg = q_value_history[80..90].iter().sum::<f64>() / 10.0;

    let change_rate = (last_10_avg - prev_10_avg).abs() / prev_10_avg.abs().max(0.001);

    println!("Q-value convergence rate: {:.4}", change_rate);
    println!("Final average Q-value: {:.4}", last_10_avg);
    println!(
        "Final epsilon: {:.4}",
        *q_manager.current_epsilon.read().await
    );

    // Should converge (small change rate)
    assert!(change_rate < 0.05); // Less than 5% change

    // Final policy check - should cache high-frequency items
    let final_cache = q_manager.cache_stats.read().await.access_frequency.clone();
    assert!(final_cache.contains_key(&content_items[0].0)); // Most frequent
    assert!(final_cache.contains_key(&content_items[1].0)); // Second most frequent
}
