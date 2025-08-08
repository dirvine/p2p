// Copyright (c) 2025 Saorsa Labs Limited

// This file is part of the Saorsa P2P network.

// Licensed under the AGPL-3.0 license:
// <https://www.gnu.org/licenses/agpl-3.0.html>

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


//! Integration Performance Benchmarks
//!
//! Measures performance of integration test scenarios for regression detection.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Duration;
use tokio::runtime::Runtime;

// Import our test frameworks
use saorsa_core::*;

fn benchmark_network_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("network_performance");
    group.measurement_time(Duration::from_secs(30));

    for node_count in [3, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::new("multi_node_messaging", node_count),
            node_count,
            |b, &node_count| {
                b.to_async(&rt)
                    .iter(|| async move { benchmark_multi_node_messaging(node_count).await });
            },
        );
    }

    group.finish();
}

fn benchmark_storage_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("storage_performance");
    group.measurement_time(Duration::from_secs(20));

    for data_size in [1024, 10240, 102400].iter() {
        group.bench_with_input(
            BenchmarkId::new("store_retrieve", data_size),
            data_size,
            |b, &data_size| {
                b.to_async(&rt)
                    .iter(|| async move { benchmark_storage_operations(data_size).await });
            },
        );
    }

    group.finish();
}

fn benchmark_crypto_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("crypto_performance");

    for message_size in [1024, 10240].iter() {
        group.bench_with_input(
            BenchmarkId::new("encrypt_decrypt", message_size),
            message_size,
            |b, &message_size| {
                b.to_async(&rt)
                    .iter(|| async move { benchmark_encryption_operations(message_size).await });
            },
        );
    }

    group.finish();
}

async fn benchmark_multi_node_messaging(
    node_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Simplified version of network test for benchmarking
    let mut nodes = Vec::new();

    for i in 0..node_count {
        let mut config = Config::default();
        config.network.listen_port = 4000 + i as u16;
        let node = Node::new(config).await?;
        nodes.push(node);
    }

    // Start nodes and connect them
    for node in &nodes {
        node.start().await?;
    }

    // Connect in chain
    for i in 1..nodes.len() {
        let peer_addr = format!("/ip4/127.0.0.1/tcp/{}", 4000 + (i - 1));
        nodes[i].connect_to_peer(&peer_addr).await?;
    }

    // Send messages
    for i in 0..10 {
        let message = format!("benchmark_message_{}", i);
        let recipient = format!("node_{}", (i + 1) % nodes.len());
        nodes[0]
            .send_message(&recipient, message.into_bytes())
            .await?;
    }

    // Cleanup
    for node in nodes {
        let _ = node.shutdown().await;
    }

    Ok(())
}

async fn benchmark_storage_operations(data_size: usize) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let node = Node::new(config).await?;
    node.start().await?;

    let key = StorageKey::from_str("benchmark_key")?;
    let data = vec![0xAB; data_size];
    let value = StorageValue::from_bytes(data)?;

    // Benchmark store
    node.store(key.clone(), value).await?;

    // Benchmark retrieve
    let _retrieved = node.retrieve(&key).await?;

    node.shutdown().await?;
    Ok(())
}

async fn benchmark_encryption_operations(
    message_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let sender_identity = Identity::generate()?;
    let receiver_identity = Identity::generate()?;

    let config = Config::default();
    let sender_node = Node::new_with_identity(config.clone(), sender_identity.clone()).await?;
    let receiver_node = Node::new_with_identity(config, receiver_identity.clone()).await?;

    sender_node.start().await?;
    receiver_node.start().await?;

    let message = vec![0xCD; message_size];

    // Benchmark encryption
    let encrypted = sender_node
        .encrypt_message(&message, &receiver_identity)
        .await?;

    // Benchmark decryption
    let _decrypted = receiver_node
        .decrypt_message(encrypted, &sender_identity)
        .await?;

    sender_node.shutdown().await?;
    receiver_node.shutdown().await?;
    Ok(())
}

criterion_group!(
    benches,
    benchmark_network_performance,
    benchmark_storage_performance,
    benchmark_crypto_performance
);
criterion_main!(benches);
