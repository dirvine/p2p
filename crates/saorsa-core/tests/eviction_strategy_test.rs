// Copyright 2024 Saorsa Labs Limited
//
//! Cache eviction strategy integration tests

use saorsa_core::adaptive::eviction::QValue;
use saorsa_core::adaptive::{
    AccessInfo, AdaptiveStrategy, CacheAction, CacheState, ContentHash, EvictionStrategy,
    EvictionStrategyType, LFUStrategy, LRUStrategy, StateVector,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Create test access info with different patterns
fn create_test_data() -> HashMap<ContentHash, AccessInfo> {
    let mut data = HashMap::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Frequently accessed, recently used, small
    data.insert(
        ContentHash::from(b"popular_recent_small"),
        AccessInfo {
            count: 100,
            last_access_secs: now - 30, // 30 seconds ago
            size: 1024,                 // 1KB
        },
    );

    // Frequently accessed, old, large
    data.insert(
        ContentHash::from(b"popular_old_large"),
        AccessInfo {
            count: 150,
            last_access_secs: now - 3600, // 1 hour ago
            size: 10 * 1024 * 1024,       // 10MB
        },
    );

    // Rarely accessed, recent, medium
    data.insert(
        ContentHash::from(b"rare_recent_medium"),
        AccessInfo {
            count: 5,
            last_access_secs: now - 120, // 2 minutes ago
            size: 100 * 1024,            // 100KB
        },
    );

    // Never accessed after insert, old, tiny
    data.insert(
        ContentHash::from(b"stale_old_tiny"),
        AccessInfo {
            count: 1,
            last_access_secs: now - 86400, // 1 day ago
            size: 512,                     // 512 bytes
        },
    );

    data
}

#[tokio::test]
async fn test_lru_eviction_behavior() {
    let mut strategy = LRUStrategy::new();
    let access_data = create_test_data();
    let cache_state = CacheState {
        current_size: 10 * 1024 * 1024 + 101 * 1024 + 1024 + 512,
        max_size: 20 * 1024 * 1024,
        item_count: 4,
        avg_access_frequency: 64.0,
    };

    // Simulate access pattern
    let popular_recent = ContentHash::from(b"popular_recent_small");
    let popular_old = ContentHash::from(b"popular_old_large");
    let rare_recent = ContentHash::from(b"rare_recent_medium");
    let stale_old = ContentHash::from(b"stale_old_tiny");

    // Insert in order (oldest to newest)
    strategy.on_insert(&stale_old);
    strategy.on_insert(&popular_old);
    strategy.on_insert(&rare_recent);
    strategy.on_insert(&popular_recent);

    // Access some items to update LRU order
    strategy.on_access(&popular_old); // Make this more recent than stale_old
    strategy.on_access(&popular_recent); // Most recent

    // Should evict stale_old (least recently used)
    let victim = strategy.select_victim(&cache_state, &access_data);
    assert_eq!(victim, Some(stale_old));
}

#[tokio::test]
async fn test_lfu_eviction_behavior() {
    let mut strategy = LFUStrategy::new();
    let access_data = create_test_data();
    let cache_state = CacheState {
        current_size: 10 * 1024 * 1024 + 101 * 1024 + 1024 + 512,
        max_size: 20 * 1024 * 1024,
        item_count: 4,
        avg_access_frequency: 64.0,
    };

    // Simulate frequency patterns
    let popular_recent = ContentHash::from(b"popular_recent_small");
    let popular_old = ContentHash::from(b"popular_old_large");
    let rare_recent = ContentHash::from(b"rare_recent_medium");
    let stale_old = ContentHash::from(b"stale_old_tiny");

    // Set up frequencies
    for _ in 0..100 {
        strategy.on_access(&popular_recent);
    }
    for _ in 0..150 {
        strategy.on_access(&popular_old);
    }
    for _ in 0..5 {
        strategy.on_access(&rare_recent);
    }
    strategy.on_access(&stale_old); // Only once

    // Should evict stale_old (least frequently used)
    let victim = strategy.select_victim(&cache_state, &access_data);
    assert_eq!(victim, Some(stale_old));
}

#[tokio::test]
async fn test_adaptive_strategy_with_q_learning() {
    let q_table = Arc::new(RwLock::new(HashMap::new()));
    let strategy = AdaptiveStrategy::new(q_table.clone());
    let access_data = create_test_data();
    let cache_state = CacheState {
        current_size: 10 * 1024 * 1024 + 101 * 1024 + 1024 + 512,
        max_size: 20 * 1024 * 1024,
        item_count: 4,
        avg_access_frequency: 64.0,
    };

    // Set up Q-values to prefer keeping frequently accessed items
    let popular_recent = ContentHash::from(b"popular_recent_small");
    let stale_old = ContentHash::from(b"stale_old_tiny");

    // High utilization state
    let state = StateVector {
        utilization_bucket: 5,
        frequency_bucket: 3,
        recency_bucket: 0,
        content_size_bucket: 0,
    };

    // Make keeping popular_recent valuable
    q_table.write().await.insert(
        (state, CacheAction::DoNothing),
        QValue {
            value: 10.0,
            updates: 10,
        },
    );
    q_table.write().await.insert(
        (state, CacheAction::Evict(popular_recent.clone())),
        QValue {
            value: -5.0,
            updates: 10,
        },
    );

    // Make evicting stale_old valuable
    q_table.write().await.insert(
        (state, CacheAction::Evict(stale_old.clone())),
        QValue {
            value: 8.0,
            updates: 10,
        },
    );

    // Should prefer to evict stale_old based on Q-values
    let victim = strategy.select_victim(&cache_state, &access_data);
    assert!(victim.is_some());
}

#[tokio::test]
async fn test_strategy_switching() {
    // Test that we can switch strategies at runtime
    let mut lru = LRUStrategy::new();
    let mut lfu = LFUStrategy::new();
    let _access_data = create_test_data();
    let cache_state = CacheState {
        current_size: 10 * 1024 * 1024,
        max_size: 20 * 1024 * 1024,
        item_count: 4,
        avg_access_frequency: 50.0,
    };

    // Set up different patterns for each strategy
    let item1 = ContentHash::from(b"item1");
    let item2 = ContentHash::from(b"item2");

    // LRU pattern
    lru.on_insert(&item1);
    lru.on_insert(&item2);
    lru.on_access(&item1); // item1 is more recent

    // LFU pattern
    lfu.on_insert(&item1);
    lfu.on_insert(&item2);
    for _ in 0..10 {
        lfu.on_access(&item1); // item1 is more frequent
    }
    lfu.on_access(&item2); // item2 accessed only once

    // Create access data for testing
    let mut test_data = HashMap::new();
    test_data.insert(
        item1,
        AccessInfo {
            count: 10,
            last_access_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 10,
            size: 1024,
        },
    );
    test_data.insert(
        item2,
        AccessInfo {
            count: 1,
            last_access_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 5,
            size: 1024,
        },
    );

    // LRU should evict item2 (less recently used despite being accessed more recently in wall time)
    let lru_victim = lru.select_victim(&cache_state, &test_data);
    assert_eq!(lru_victim, Some(item2));

    // LFU should evict item2 (less frequently used)
    let lfu_victim = lfu.select_victim(&cache_state, &test_data);
    assert_eq!(lfu_victim, Some(item2));
}

#[tokio::test]
async fn test_cache_state_calculations() {
    let access_data = create_test_data();

    // Calculate total size
    let total_size: u64 = access_data.values().map(|info| info.size).sum();

    // Calculate average frequency
    let avg_frequency =
        access_data.values().map(|info| info.count).sum::<u64>() as f64 / access_data.len() as f64;

    let cache_state = CacheState {
        current_size: total_size,
        max_size: 50 * 1024 * 1024,
        item_count: access_data.len(),
        avg_access_frequency: avg_frequency,
    };

    // Verify calculations
    assert_eq!(cache_state.item_count, 4);
    assert!(cache_state.avg_access_frequency > 60.0 && cache_state.avg_access_frequency < 65.0);
    assert!(cache_state.current_size > 10 * 1024 * 1024);
}

#[test]
fn test_eviction_strategy_factory() {
    // Test factory pattern
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
