//! Benchmarks for the validation framework

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use saorsa_core::validation::*;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

fn benchmark_peer_id_validation(c: &mut Criterion) {
    c.bench_function("validate_peer_id_valid", |b| {
        b.iter(|| validate_peer_id(black_box("valid_peer_id_123456")));
    });

    c.bench_function("validate_peer_id_invalid", |b| {
        b.iter(|| {
            let _ = validate_peer_id(black_box("short"));
        });
    });
}

fn benchmark_network_address_validation(c: &mut Criterion) {
    let ctx = ValidationContext::default();
    let addr: SocketAddr = "8.8.8.8:53".parse().unwrap();

    c.bench_function("validate_network_address", |b| {
        b.iter(|| validate_network_address(black_box(&addr), &ctx));
    });
}

fn benchmark_message_size_validation(c: &mut Criterion) {
    let max_size = 16 * 1024 * 1024; // 16MB

    c.bench_function("validate_message_size", |b| {
        b.iter(|| validate_message_size(black_box(1024 * 1024), black_box(max_size)));
    });
}

fn benchmark_file_path_validation(c: &mut Criterion) {
    let path = Path::new("/usr/local/bin/application");

    c.bench_function("validate_file_path", |b| {
        b.iter(|| validate_file_path(black_box(path)));
    });
}

fn benchmark_dht_validation(c: &mut Criterion) {
    let ctx = ValidationContext::default();
    let key = vec![0u8; 32];
    let value = vec![0u8; 1024];

    c.bench_function("validate_dht_key", |b| {
        b.iter(|| validate_dht_key(black_box(&key), &ctx));
    });

    c.bench_function("validate_dht_value", |b| {
        b.iter(|| validate_dht_value(black_box(&value), &ctx));
    });
}

fn benchmark_rate_limiter(c: &mut Criterion) {
    let config = RateLimitConfig {
        window: Duration::from_secs(60),
        max_requests: 10000,
        burst_size: 1000,
        ..Default::default()
    };

    let limiter = Arc::new(RateLimiter::new(config));
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    c.bench_function("rate_limiter_check", |b| {
        b.iter(|| {
            let _ = limiter.check_ip(black_box(&ip));
        });
    });
}

fn benchmark_complex_validation(c: &mut Criterion) {
    let ctx = ValidationContext::default();

    c.bench_function("network_message_validation", |b| {
        b.iter_batched(
            || NetworkMessage {
                peer_id: "valid_peer_id_123456".to_string(),
                payload: vec![0u8; 1024],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
            |msg| msg.validate(&ctx),
            BatchSize::SmallInput,
        );
    });
}

fn benchmark_sanitization(c: &mut Criterion) {
    c.bench_function("sanitize_string", |b| {
        b.iter(|| sanitize_string(black_box("hello@world#123$test%"), black_box(20)));
    });
}

criterion_group!(
    benches,
    benchmark_peer_id_validation,
    benchmark_network_address_validation,
    benchmark_message_size_validation,
    benchmark_file_path_validation,
    benchmark_dht_validation,
    benchmark_rate_limiter,
    benchmark_complex_validation,
    benchmark_sanitization
);

criterion_main!(benches);
