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

//! Security Module Performance Benchmarks
//!
//! Benchmarks for security features including IPv6 node identity generation,
//! IP diversity enforcement, reputation management, and cryptographic operations.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use ed25519_dalek::SigningKey;
use p2p_foundation::security::{
    IPDiversityConfig, IPDiversityEnforcer, IPv6NodeID, ReputationManager,
};
use std::net::Ipv6Addr;
use std::time::Duration;

/// Benchmark IPv6 node identity operations
fn ipv6_identity_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipv6_identity");

    // Create test data
    let mut csprng = rand::rngs::OsRng {};
    let keypair = SigningKey::generate(&mut csprng);
    let ipv6_addrs = vec![
        "2001:db8:85a3::8a2e:370:7334".parse::<Ipv6Addr>().unwrap(),
        "2001:db8:85a3:1234:5678:8a2e:370:7334".parse().unwrap(),
        "fe80::1234:5678:8a2e:370".parse().unwrap(),
        "2001:db8::1".parse().unwrap(),
        "::1".parse().unwrap(),
    ];

    // Benchmark IPv6 node ID generation
    for (i, addr) in ipv6_addrs.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("node_id_generation", i),
            addr,
            |b, addr| {
                b.iter(|| {
                    let node_id =
                        IPv6NodeID::generate(black_box(*addr), black_box(&keypair)).unwrap();
                    black_box(node_id)
                });
            },
        );
    }

    // Benchmark node ID verification
    let test_addr = ipv6_addrs[0];
    let node_id = IPv6NodeID::generate(test_addr, &keypair).unwrap();

    group.bench_function("node_id_verification_valid", |b| {
        b.iter(|| {
            let result = node_id.verify().unwrap();
            black_box(result)
        });
    });

    // Benchmark verification with wrong key (should fail)
    let wrong_keypair = SigningKey::generate(&mut csprng);
    group.bench_function("node_id_verification_invalid", |b| {
        b.iter(|| {
            let result = node_id.verify();
            black_box(result)
        });
    });

    // Benchmark node ID serialization
    group.bench_function("node_id_serialization", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(black_box(&node_id)).unwrap();
            black_box(serialized)
        });
    });

    // Benchmark node ID deserialization
    let serialized = serde_json::to_vec(&node_id).unwrap();
    group.bench_function("node_id_deserialization", |b| {
        b.iter(|| {
            let deserialized: IPv6NodeID = serde_json::from_slice(black_box(&serialized)).unwrap();
            black_box(deserialized)
        });
    });

    group.finish();
}

/// Benchmark IP diversity enforcement operations
fn ip_diversity_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("ip_diversity");

    // Create test configuration and enforcer
    let config = IPDiversityConfig {
        max_nodes_per_64: 5,
        max_nodes_per_48: 15,
        max_nodes_per_32: 50,
        max_nodes_per_asn: 100,
        enable_geolocation_check: true,
        min_geographic_diversity: 3,
    };
    let mut enforcer = IPDiversityEnforcer::new(config);

    // Generate test IPv6 addresses in different subnets
    let test_addresses = vec![
        "2001:db8:85a3:1234:5678:8a2e:370:7334"
            .parse::<Ipv6Addr>()
            .unwrap(),
        "2001:db8:85a3:1234:5678:8a2e:370:7335".parse().unwrap(), // Same /64
        "2001:db8:85a3:1235:5678:8a2e:370:7334".parse().unwrap(), // Different /64, same /48
        "2001:db8:85a4:1234:5678:8a2e:370:7334".parse().unwrap(), // Different /48, same /32
        "2001:db9:85a3:1234:5678:8a2e:370:7334".parse().unwrap(), // Different /32
        "fe80::1234:5678:8a2e:370".parse().unwrap(),              // Link-local
    ];

    // Benchmark IP analysis
    for (i, addr) in test_addresses.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("ip_analysis", i), addr, |b, addr| {
            b.iter(|| {
                let analysis = enforcer.analyze_ip(black_box(*addr)).unwrap();
                black_box(analysis)
            });
        });
    }

    // Benchmark subnet prefix extraction
    let test_addr = test_addresses[0];
    for prefix_len in [32, 48, 56, 64, 96, 128].iter() {
        group.bench_with_input(
            BenchmarkId::new("subnet_extraction", prefix_len),
            prefix_len,
            |b, &prefix_len| {
                b.iter(|| {
                    let subnet = IPDiversityEnforcer::extract_subnet_prefix(
                        black_box(test_addr),
                        black_box(prefix_len),
                    );
                    black_box(subnet)
                });
            },
        );
    }

    // Benchmark can_accept_node checks
    let analysis = enforcer.analyze_ip(test_addresses[0]).unwrap();
    group.bench_function("can_accept_node_empty", |b| {
        b.iter(|| {
            let result = enforcer.can_accept_node(black_box(&analysis));
            black_box(result)
        });
    });

    // Add some nodes to test performance with existing state
    for addr in &test_addresses[0..3] {
        let analysis = enforcer.analyze_ip(*addr).unwrap();
        let _ = enforcer.add_node(&analysis);
    }

    group.bench_function("can_accept_node_with_state", |b| {
        let analysis = enforcer.analyze_ip(test_addresses[4]).unwrap();
        b.iter(|| {
            let result = enforcer.can_accept_node(black_box(&analysis));
            black_box(result)
        });
    });

    // Benchmark add_node operation
    group.bench_function("add_node", |b| {
        let enforcer = IPDiversityEnforcer::new(IPDiversityConfig::default());
        let analysis = enforcer.analyze_ip(test_addresses[0]).unwrap();
        b.iter(|| {
            let mut enforcer_clone = IPDiversityEnforcer::new(IPDiversityConfig::default());
            let result = enforcer_clone.add_node(black_box(&analysis));
            black_box(result)
        });
    });

    // Benchmark remove_node operation
    group.bench_function("remove_node", |b| {
        let analysis = enforcer.analyze_ip(test_addresses[0]).unwrap();
        b.iter(|| {
            let mut enforcer_clone = IPDiversityEnforcer::new(IPDiversityConfig::default());
            let _ = enforcer_clone.add_node(&analysis);
            enforcer_clone.remove_node(black_box(&analysis));
        });
    });

    // Benchmark diversity stats generation
    group.bench_function("get_diversity_stats", |b| {
        b.iter(|| {
            let stats = enforcer.get_diversity_stats();
            black_box(stats)
        });
    });

    group.finish();
}

/// Benchmark reputation management operations
fn reputation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("reputation_management");

    // Create test reputation manager
    let mut manager = ReputationManager::new(0.1, 0.1);

    // Benchmark reputation update
    group.bench_function("reputation_update_success", |b| {
        let peer_id = "benchmark_peer_success".to_string();
        b.iter(|| {
            manager.update_reputation(
                black_box(&peer_id),
                black_box(true),
                black_box(Duration::from_millis(100)),
            );
        });
    });

    group.bench_function("reputation_update_failure", |b| {
        let peer_id = "benchmark_peer_failure".to_string();
        b.iter(|| {
            manager.update_reputation(
                black_box(&peer_id),
                black_box(false),
                black_box(Duration::from_millis(500)),
            );
        });
    });

    // Add some test data
    for i in 0..100 {
        let peer_id = format!("test_peer_{}", i);
        manager.update_reputation(&peer_id, i % 3 != 0, Duration::from_millis(100 + i * 2));
    }

    // Benchmark reputation retrieval
    group.bench_function("get_reputation", |b| {
        let peer_id = "test_peer_50".to_string();
        b.iter(|| {
            let reputation = manager.get_reputation(black_box(&peer_id));
            black_box(reputation)
        });
    });

    // Benchmark reputation retrieval for non-existent peer
    group.bench_function("get_reputation_nonexistent", |b| {
        let peer_id = "nonexistent_peer".to_string();
        b.iter(|| {
            let reputation = manager.get_reputation(black_box(&peer_id));
            black_box(reputation)
        });
    });

    // Benchmark reputation decay
    group.bench_function("apply_decay", |b| {
        b.iter(|| {
            let mut manager_clone = ReputationManager::new(0.1, 0.1);
            // Add some test reputations
            for i in 0..50 {
                let peer_id = format!("decay_test_peer_{}", i);
                manager_clone.update_reputation(&peer_id, i % 2 == 0, Duration::from_millis(100));
            }
            manager_clone.apply_decay();
            black_box(manager_clone)
        });
    });

    // Benchmark performance with many peers
    for peer_count in [100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("many_peers_update", peer_count),
            peer_count,
            |b, &peer_count| {
                b.iter(|| {
                    let mut manager = ReputationManager::new(0.1, 0.1);
                    for i in 0..peer_count {
                        let peer_id = format!("peer_{}", i);
                        manager.update_reputation(&peer_id, i % 3 != 0, Duration::from_millis(100));
                    }
                    black_box(manager)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent security operations
fn concurrent_security_benchmarks(c: &mut Criterion) {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let mut group = c.benchmark_group("concurrent_security");

    // Benchmark concurrent IPv6 node ID generation
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_node_id_generation", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|i| {
                            thread::spawn(move || {
                                let mut csprng = rand::rngs::OsRng {};
                                let keypair = SigningKey::generate(&mut csprng);
                                let addr: Ipv6Addr =
                                    format!("2001:db8:85a3::{}:7334", i).parse().unwrap();
                                let node_id = IPv6NodeID::generate(addr, &keypair).unwrap();
                                black_box(node_id)
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    // Benchmark concurrent IP diversity operations
    let enforcer = Arc::new(Mutex::new(IPDiversityEnforcer::new(
        IPDiversityConfig::default(),
    )));

    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_ip_analysis", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|i| {
                            let enforcer = enforcer.clone();
                            thread::spawn(move || {
                                let addr: Ipv6Addr =
                                    format!("2001:db8:85a3:{}::7334", i).parse().unwrap();
                                let enforcer = enforcer.lock().unwrap();
                                let analysis = enforcer.analyze_ip(addr).unwrap();
                                black_box(analysis)
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    // Benchmark concurrent reputation updates
    let manager = Arc::new(Mutex::new(ReputationManager::new(0.1, 0.1)));

    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_reputation_updates", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|i| {
                            let manager = manager.clone();
                            thread::spawn(move || {
                                let peer_id = format!("concurrent_peer_{}", i);
                                let mut manager = manager.lock().unwrap();
                                manager.update_reputation(
                                    &peer_id,
                                    i % 2 == 0,
                                    Duration::from_millis(100),
                                );
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cryptographic operations in security context
fn security_crypto_benchmarks(c: &mut Criterion) {
    use ed25519_dalek::{Signer, Verifier};
    use sha2::{Digest, Sha256};

    let mut group = c.benchmark_group("security_crypto");

    let mut csprng = rand::rngs::OsRng {};
    let keypair = SigningKey::generate(&mut csprng);

    // Benchmark IPv6 address hash computation (used in node ID generation)
    let ipv6_addresses = vec![
        "2001:db8:85a3::8a2e:370:7334".parse::<Ipv6Addr>().unwrap(),
        "fe80::1234:5678:8a2e:370".parse().unwrap(),
        "::1".parse().unwrap(),
    ];

    for (i, addr) in ipv6_addresses.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("ipv6_hash", i), addr, |b, addr| {
            b.iter(|| {
                let mut hasher = Sha256::new();
                hasher.update(addr.octets());
                hasher.update(keypair.public.as_bytes());
                hasher.update(b"salt");
                let hash = hasher.finalize();
                black_box(hash)
            });
        });
    }

    // Benchmark signature generation for node ID
    for addr in &ipv6_addresses {
        let mut hasher = Sha256::new();
        hasher.update(addr.octets());
        hasher.update(keypair.public.as_bytes());
        let data_to_sign = hasher.finalize();

        group.bench_function("node_id_signature", |b| {
            b.iter(|| {
                let signature = keypair.sign(black_box(&data_to_sign));
                black_box(signature)
            });
        });
        break; // Only benchmark one iteration
    }

    // Benchmark signature verification for node ID
    let mut hasher = Sha256::new();
    hasher.update(ipv6_addresses[0].octets());
    hasher.update(keypair.public.as_bytes());
    let data_to_verify = hasher.finalize();
    let signature = keypair.sign(&data_to_verify);

    group.bench_function("node_id_verification", |b| {
        b.iter(|| {
            let result = keypair
                .public
                .verify(black_box(&data_to_verify), black_box(&signature));
            black_box(result)
        });
    });

    // Benchmark batch verification (simulating multiple node IDs)
    let signatures: Vec<_> = (0..10)
        .map(|i| {
            let mut hasher = Sha256::new();
            hasher.update(format!("data_{}", i).as_bytes());
            let data = hasher.finalize();
            let sig = keypair.sign(&data);
            (data.to_vec(), sig)
        })
        .collect();

    group.bench_function("batch_verification", |b| {
        b.iter(|| {
            for (data, sig) in black_box(&signatures) {
                let result = keypair.public.verify(data, sig);
                black_box(result).unwrap();
            }
        });
    });

    group.finish();
}

criterion_group!(
    security_benches,
    ipv6_identity_benchmarks,
    ip_diversity_benchmarks,
    reputation_benchmarks,
    concurrent_security_benchmarks,
    security_crypto_benchmarks
);

criterion_main!(security_benches);
