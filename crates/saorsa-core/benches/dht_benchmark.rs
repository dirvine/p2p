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

//! DHT Performance Benchmarks
//!
//! Comprehensive benchmarks for measuring P2P Foundation DHT performance.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use p2p_foundation::dht::Key;

/// Benchmark DHT key operations
fn dht_key_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("dht_key_operations");

    // Benchmark key creation from different data sizes
    for size in [32, 64, 128, 256, 512].iter() {
        let data = vec![0u8; *size];
        group.bench_with_input(BenchmarkId::new("key_creation", size), &data, |b, data| {
            b.iter(|| Key::new(black_box(data)));
        });
    }

    // Benchmark key comparison operations
    let key1 = Key::new(b"benchmark_key_1");
    let key2 = Key::new(b"benchmark_key_2");

    group.bench_function("key_comparison", |b| {
        b.iter(|| black_box(&key1) == black_box(&key2));
    });

    // Benchmark key serialization
    let key = Key::new(b"serialization_test_key");
    group.bench_function("key_serialization", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(black_box(&key)).unwrap();
            black_box(serialized)
        });
    });

    // Benchmark key deserialization
    let serialized = serde_json::to_vec(&key).unwrap();
    group.bench_function("key_deserialization", |b| {
        b.iter(|| {
            let key: Key = serde_json::from_slice(black_box(&serialized)).unwrap();
            black_box(key)
        });
    });

    group.finish();
}

/// Benchmark tunneling protocol operations
fn tunneling_benchmarks(c: &mut Criterion) {
    use p2p_foundation::tunneling::{NetworkCapabilities, TunnelManager, TunnelManagerConfig};

    let mut group = c.benchmark_group("tunneling_operations");

    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: true,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: vec![],
        has_upnp: true,
        interface_mtu: 1500,
    };

    // Benchmark tunnel manager creation
    group.bench_function("tunnel_manager_creation", |b| {
        b.iter(|| {
            let config = TunnelManagerConfig::default();
            let manager = TunnelManager::with_config(black_box(config));
            black_box(manager)
        });
    });

    // Benchmark tunnel selection (synchronous part)
    group.bench_function("tunnel_scoring", |b| {
        let config = TunnelManagerConfig::default();
        let manager = TunnelManager::with_config(config);

        b.iter(|| {
            // This tests the synchronous scoring logic
            let rt = tokio::runtime::Runtime::new().unwrap();
            let selection =
                rt.block_on(async { manager.select_tunnel(black_box(&capabilities)).await });
            black_box(selection)
        });
    });

    // Benchmark ISATAP address generation
    use p2p_foundation::tunneling::IsatapTunnel;
    let prefix: std::net::Ipv6Addr = "fe80::".parse().unwrap();
    group.bench_function("isatap_address_generation", |b| {
        b.iter(|| {
            let ipv4_addr = std::net::Ipv4Addr::new(192, 168, 1, 100);
            let isatap_addr = IsatapTunnel::generate_isatap_address(
                black_box(ipv4_addr),
                black_box(Some(prefix)),
            );
            black_box(isatap_addr)
        });
    });

    // Benchmark MAP address calculation
    use p2p_foundation::tunneling::{
        MapProtocol, MapRule, MapTunnel, PortParameters, TunnelConfig, TunnelProtocol,
    };
    let map_config = TunnelConfig {
        protocol: TunnelProtocol::MapE,
        local_ipv4: Some("192.0.2.100".parse().unwrap()),
        remote_ipv4: None,
        ipv6_prefix: Some("2001:db8::".parse().unwrap()),
        aftr_ipv6: None,
        aftr_name: None,
        mtu: 1460,
        keepalive_interval: std::time::Duration::from_secs(30),
        establishment_timeout: std::time::Duration::from_secs(10),
    };
    let map_tunnel = MapTunnel::new(map_config, MapProtocol::MapE).unwrap();
    let map_rule = MapRule {
        ipv6_prefix: "2001:db8::".parse().unwrap(),
        ipv6_prefix_len: 32,
        ipv4_prefix: "192.0.2.0".parse().unwrap(),
        ipv4_prefix_len: 24,
        port_params: PortParameters {
            psid_offset: 4,
            psid_length: 4,
            excluded_ports: 1024,
        },
        border_relay: Some("2001:db8:ffff::1".parse().unwrap()),
        is_fmr: true,
    };

    group.bench_function("map_ipv6_address_calculation", |b| {
        b.iter(|| {
            let ipv4_addr = std::net::Ipv4Addr::new(192, 0, 2, 100);
            let ipv6_addr = map_tunnel
                .calculate_ipv6_address(black_box(ipv4_addr), black_box(&map_rule))
                .unwrap();
            black_box(ipv6_addr)
        });
    });

    group.bench_function("map_psid_extraction", |b| {
        b.iter(|| {
            let ipv4_addr = std::net::Ipv4Addr::new(192, 0, 2, 100);
            let psid = map_tunnel.extract_psid(black_box(ipv4_addr), black_box(&map_rule));
            black_box(psid)
        });
    });

    group.bench_function("map_port_set_calculation", |b| {
        b.iter(|| {
            let psid = 5u16;
            let port_set = map_tunnel.calculate_port_set(black_box(psid), black_box(&map_rule));
            black_box(port_set)
        });
    });

    group.finish();
}

/// Benchmark MCP operations
fn mcp_benchmarks(c: &mut Criterion) {
    use p2p_foundation::mcp::{MCPServer, MCPServerConfig};
    use serde_json::json;

    let mut group = c.benchmark_group("mcp_operations");

    // Benchmark MCP server creation
    group.bench_function("mcp_server_creation", |b| {
        b.iter(|| {
            let config = MCPServerConfig::default();
            let server = MCPServer::new(black_box(config));
            black_box(server)
        });
    });

    // Benchmark JSON message processing
    let messages = vec![
        ("small", json!({"method": "list_tools", "params": {}})),
        (
            "medium",
            json!({
                "method": "call_tool",
                "params": {
                    "name": "test",
                    "args": {"data": [1, 2, 3, 4, 5]}
                }
            }),
        ),
        (
            "large",
            json!({
                "method": "call_tool",
                "params": {
                    "name": "process_data",
                    "args": {
                        "data": (0..1000).collect::<Vec<i32>>(),
                        "options": {"format": "json", "compress": true}
                    }
                }
            }),
        ),
    ];

    for (size, message) in messages {
        group.bench_with_input(
            BenchmarkId::new("json_serialization", size),
            &message,
            |b, message| {
                b.iter(|| {
                    let serialized = serde_json::to_vec(black_box(message)).unwrap();
                    black_box(serialized)
                });
            },
        );

        let serialized = serde_json::to_vec(&message).unwrap();
        group.bench_with_input(
            BenchmarkId::new("json_deserialization", size),
            &serialized,
            |b, serialized| {
                b.iter(|| {
                    let parsed: serde_json::Value =
                        serde_json::from_slice(black_box(serialized)).unwrap();
                    black_box(parsed)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark network operations
fn network_benchmarks(c: &mut Criterion) {
    use p2p_foundation::{NodeBuilder, NodeConfig};

    let mut group = c.benchmark_group("network_operations");

    // Benchmark node configuration creation
    group.bench_function("node_config_creation", |b| {
        b.iter(|| {
            let config = NodeConfig::default();
            black_box(config)
        });
    });

    // Benchmark node builder creation
    group.bench_function("node_builder_creation", |b| {
        b.iter(|| {
            let builder = NodeBuilder::new();
            black_box(builder)
        });
    });

    group.finish();
}

/// Benchmark concurrent operations
fn concurrent_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // Benchmark concurrent key operations
    for thread_count in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_key_creation", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let handles: Vec<_> = (0..thread_count)
                            .map(|i| {
                                tokio::spawn(async move {
                                    let key = Key::new(format!("key_{}", i).as_bytes());
                                    black_box(key)
                                })
                            })
                            .collect();

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }

    // Benchmark concurrent JSON processing
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_json_processing", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let handles: Vec<_> = (0..thread_count)
                            .map(|i| {
                                tokio::spawn(async move {
                                    let data = serde_json::json!({
                                        "id": i,
                                        "data": (0..100).collect::<Vec<i32>>()
                                    });
                                    let serialized = serde_json::to_vec(&data).unwrap();
                                    let _parsed: serde_json::Value =
                                        serde_json::from_slice(&serialized).unwrap();
                                    black_box(())
                                })
                            })
                            .collect();

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cryptographic operations
fn crypto_benchmarks(c: &mut Criterion) {
    use ed25519_dalek::{Keypair, Signer, Verifier};
    use rand::rngs::OsRng;
    use sha2::{Digest, Sha256};

    let mut group = c.benchmark_group("crypto_operations");

    // Benchmark SHA256 hashing
    for size in [64, 256, 1024, 4096].iter() {
        let data = vec![0u8; *size];
        group.bench_with_input(BenchmarkId::new("sha256_hash", size), &data, |b, data| {
            b.iter(|| {
                let mut hasher = Sha256::new();
                hasher.update(black_box(data));
                let hash = hasher.finalize();
                black_box(hash)
            });
        });
    }

    // Benchmark Ed25519 key generation
    group.bench_function("ed25519_keygen", |b| {
        b.iter(|| {
            let mut csprng = OsRng {};
            let keypair = Keypair::generate(&mut csprng);
            black_box(keypair)
        });
    });

    // Benchmark Ed25519 signing
    let mut csprng = OsRng {};
    let keypair = Keypair::generate(&mut csprng);
    let message = b"benchmark message for signing";

    group.bench_function("ed25519_sign", |b| {
        b.iter(|| {
            let signature = keypair.sign(black_box(message));
            black_box(signature)
        });
    });

    // Benchmark Ed25519 verification
    let signature = keypair.sign(message);

    group.bench_function("ed25519_verify", |b| {
        b.iter(|| {
            let result = keypair
                .public
                .verify(black_box(message), black_box(&signature));
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    dht_key_benchmarks,
    tunneling_benchmarks,
    mcp_benchmarks,
    network_benchmarks,
    concurrent_benchmarks,
    crypto_benchmarks
);

criterion_main!(benches);
