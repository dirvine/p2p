// Copyright 2024 Saorsa Labs Limited
//
// Benchmark for cache eviction strategies

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use saorsa_core::adaptive::{
    AccessInfo, CacheState, ContentHash, EvictionStrategy, EvictionStrategyType,
};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn create_test_cache(size: usize) -> HashMap<ContentHash, AccessInfo> {
    let mut cache = HashMap::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..size {
        let hash = ContentHash::from(format!("content_{}", i).as_bytes());
        let info = AccessInfo {
            count: (i as u64 % 100) + 1,
            last_access_secs: now - (i as u64 * 60), // Older items have older access times
            size: ((i % 10) + 1) as u64 * 1024 * 1024, // 1-10 MB
        };
        cache.insert(hash, info);
    }

    cache
}

fn bench_lru_eviction(c: &mut Criterion) {
    let cache_data = create_test_cache(1000);
    let cache_state = CacheState {
        current_size: 500 * 1024 * 1024,
        max_size: 1024 * 1024 * 1024,
        item_count: 1000,
        avg_access_frequency: 50.0,
    };

    c.bench_function("lru_eviction_1000", |b| {
        let mut strategy = EvictionStrategyType::LRU.create();
        // Populate access order
        for (hash, _) in &cache_data {
            strategy.on_insert(hash);
        }

        b.iter(|| black_box(strategy.select_victim(&cache_state, &cache_data)));
    });
}

fn bench_lfu_eviction(c: &mut Criterion) {
    let cache_data = create_test_cache(1000);
    let cache_state = CacheState {
        current_size: 500 * 1024 * 1024,
        max_size: 1024 * 1024 * 1024,
        item_count: 1000,
        avg_access_frequency: 50.0,
    };

    c.bench_function("lfu_eviction_1000", |b| {
        let mut strategy = EvictionStrategyType::LFU.create();
        // Populate frequencies
        for (hash, info) in &cache_data {
            for _ in 0..info.count {
                strategy.on_access(hash);
            }
        }

        b.iter(|| black_box(strategy.select_victim(&cache_state, &cache_data)));
    });
}

fn bench_adaptive_eviction(c: &mut Criterion) {
    let cache_data = create_test_cache(1000);
    let cache_state = CacheState {
        current_size: 500 * 1024 * 1024,
        max_size: 1024 * 1024 * 1024,
        item_count: 1000,
        avg_access_frequency: 50.0,
    };

    c.bench_function("adaptive_eviction_1000", |b| {
        let q_table = std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let strategy = EvictionStrategyType::Adaptive(q_table).create();

        b.iter(|| black_box(strategy.select_victim(&cache_state, &cache_data)));
    });
}

criterion_group!(
    benches,
    bench_lru_eviction,
    bench_lfu_eviction,
    bench_adaptive_eviction
);
criterion_main!(benches);
