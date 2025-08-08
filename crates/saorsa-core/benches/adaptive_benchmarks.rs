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

//! Performance benchmarks for the Adaptive P2P Network
//!
//! Run with: cargo bench --bench adaptive_benchmarks

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;
use saorsa_core::adaptive::*;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark lookup operations
fn benchmark_lookup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("lookup");
    group.measurement_time(Duration::from_secs(10));

    // Setup test network
    let client = rt.block_on(async {
        Client::connect(ClientConfig {
            profile: ClientProfile::Full,
            ..Default::default()
        })
        .await
        .unwrap()
    });

    // Pre-store test data
    let test_data: Vec<(ContentHash, Vec<u8>)> = (0..100)
        .map(|i| {
            let data = format!("test_data_{}", i).into_bytes();
            let hash = rt.block_on(client.store(data.clone())).unwrap();
            (hash, data)
        })
        .collect();

    // Benchmark different lookup scenarios
    group.bench_function("single_lookup", |b| {
        b.to_async(&rt).iter(|| async {
            let (hash, _) = &test_data[0];
            client.retrieve(black_box(hash)).await.unwrap()
        });
    });

    group.bench_function("parallel_lookups_10", |b| {
        b.to_async(&rt).iter(|| async {
            let futures: Vec<_> = test_data
                .iter()
                .take(10)
                .map(|(hash, _)| client.retrieve(hash))
                .collect();
            futures::future::join_all(futures).await
        });
    });

    group.bench_function("random_lookup", |b| {
        let mut rng = rand::thread_rng();
        b.to_async(&rt).iter(|| async {
            let idx = rng.gen_range(0..test_data.len());
            let (hash, _) = &test_data[idx];
            client.retrieve(black_box(hash)).await.unwrap()
        });
    });

    group.finish();
}

/// Benchmark storage operations
fn benchmark_storage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("storage");
    group.measurement_time(Duration::from_secs(10));

    let client = rt.block_on(async { Client::connect(ClientConfig::default()).await.unwrap() });

    // Benchmark different data sizes
    for size in [1_024, 10_240, 102_400, 1_048_576].iter() {
        group.bench_with_input(BenchmarkId::new("store_bytes", size), size, |b, &size| {
            let data = vec![0u8; size];
            b.to_async(&rt)
                .iter(|| async { client.store(black_box(data.clone())).await.unwrap() });
        });
    }

    group.finish();
}

/// Benchmark message serialization
fn benchmark_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // Create test messages
    let small_msg = NetworkMessage {
        id: "test-123".to_string(),
        sender: NodeId { hash: [0u8; 32] },
        content: vec![0u8; 100],
        msg_type: ContentType::DataRetrieval,
        timestamp: 1234567890,
    };

    let large_msg = NetworkMessage {
        id: "test-456".to_string(),
        sender: NodeId { hash: [1u8; 32] },
        content: vec![0u8; 10_000],
        msg_type: ContentType::DataRetrieval,
        timestamp: 1234567890,
    };

    // Benchmark bincode serialization
    group.bench_function("bincode_small", |b| {
        b.iter(|| bincode::serialize(black_box(&small_msg)).unwrap());
    });

    group.bench_function("bincode_large", |b| {
        b.iter(|| bincode::serialize(black_box(&large_msg)).unwrap());
    });

    // Benchmark deserialization
    let small_bytes = bincode::serialize(&small_msg).unwrap();
    let large_bytes = bincode::serialize(&large_msg).unwrap();

    group.bench_function("bincode_deserialize_small", |b| {
        b.iter(|| bincode::deserialize::<NetworkMessage>(black_box(&small_bytes)).unwrap());
    });

    group.bench_function("bincode_deserialize_large", |b| {
        b.iter(|| bincode::deserialize::<NetworkMessage>(black_box(&large_bytes)).unwrap());
    });

    group.finish();
}

/// Benchmark routing operations
fn benchmark_routing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("routing");

    // Create mock routing components
    let router = rt.block_on(async {
        // This would use the actual AdaptiveRouter in real benchmarks
        // For now, we'll create a simplified version
        AdaptiveRouter::new(NodeId { hash: [0u8; 32] }, Default::default()).await
    });

    // Benchmark route calculation
    group.bench_function("find_path", |b| {
        let target = NodeId { hash: [255u8; 32] };
        b.to_async(&rt)
            .iter(|| async { router.find_path(black_box(&target)).await.unwrap() });
    });

    // Benchmark routing table operations
    group.bench_function("routing_table_insert", |b| {
        b.iter(|| {
            let node = NodeDescriptor {
                id: NodeId {
                    hash: rand::random(),
                },
                public_key: ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32]).unwrap(),
                addresses: vec!["127.0.0.1:8000".to_string()],
                hyperbolic: None,
                som_position: None,
                trust: 0.5,
                capabilities: NodeCapabilities {
                    storage: 100,
                    compute: 50,
                    bandwidth: 10,
                },
            };
            // Add to routing table (mock operation)
            black_box(node);
        });
    });

    group.finish();
}

/// Benchmark concurrent operations
fn benchmark_concurrency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrency");
    group.measurement_time(Duration::from_secs(20));

    let client = rt.block_on(async { Client::connect(ClientConfig::default()).await.unwrap() });

    // Benchmark different concurrency levels
    for concurrency in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_stores", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let data = format!("concurrent_data_{}", i).into_bytes();
                            client.store(data)
                        })
                        .collect();
                    futures::future::join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn benchmark_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");

    // Benchmark object allocation
    group.bench_function("node_descriptor_allocation", |b| {
        b.iter(|| {
            let node = NodeDescriptor {
                id: NodeId { hash: [0u8; 32] },
                public_key: ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32]).unwrap(),
                addresses: vec!["127.0.0.1:8000".to_string(); 10],
                hyperbolic: Some(HyperbolicCoordinate { r: 0.5, theta: 1.0 }),
                som_position: Some([0.1, 0.2, 0.3, 0.4]),
                trust: 0.75,
                capabilities: NodeCapabilities {
                    storage: 1000,
                    compute: 100,
                    bandwidth: 100,
                },
            };
            black_box(node);
        });
    });

    // Benchmark collection operations
    group.bench_function("hashmap_operations", |b| {
        let mut map = std::collections::HashMap::new();
        b.iter(|| {
            for i in 0..100 {
                let key = NodeId {
                    hash: [i as u8; 32],
                };
                map.insert(key, i);
            }
            map.clear();
        });
    });

    group.finish();
}

/// Benchmark cryptographic operations
fn benchmark_crypto(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto");

    // Generate test keys
    let identity = NodeIdentity::generate_blocking();
    let message = b"Hello, P2P Network!";

    // Benchmark signing
    group.bench_function("ed25519_sign", |b| {
        b.iter(|| identity.sign(black_box(message)));
    });

    // Benchmark verification
    let signature = identity.sign(message);
    group.bench_function("ed25519_verify", |b| {
        b.iter(|| identity.verify(black_box(message), black_box(&signature)));
    });

    // Benchmark hashing
    group.bench_function("sha256_hash", |b| {
        use sha2::{Digest, Sha256};
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(message));
            hasher.finalize()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_lookup,
    benchmark_storage,
    benchmark_serialization,
    benchmark_routing,
    benchmark_concurrency,
    benchmark_memory,
    benchmark_crypto
);
criterion_main!(benches);
