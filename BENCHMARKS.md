# P2P Foundation Benchmarks

This document describes the benchmark suite for the P2P Foundation project.

## Running Benchmarks

### Basic Usage

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark file
cargo bench --bench dht_benchmark

# Run with faster settings for testing
cargo bench -- --measurement-time 1 --warm-up-time 1

# Run specific benchmark group
cargo bench dht_key_operations

# Run with smaller sample size for faster execution
cargo bench -- --sample-size 10
```

### Benchmark Output

The benchmarks generate HTML reports in `target/criterion/` directory with detailed performance charts and statistics.

## Benchmark Categories

### 1. DHT Key Operations (`dht_key_operations`)
- **Key Creation**: Performance of DHT key creation from different data sizes (32-512 bytes)
- **Key Comparison**: Speed of key equality comparisons
- **Key Serialization**: JSON serialization performance
- **Key Deserialization**: JSON deserialization performance

**Typical Results:**
- Key creation (32 bytes): ~170 ns
- Key creation (512 bytes): ~1.47 µs
- Key comparison: ~845 ps
- Key serialization: ~151 ns
- Key deserialization: ~302 ns

### 2. Tunneling Operations (`tunneling_operations`)
- **Tunnel Manager Creation**: Speed of creating tunnel managers
- **Tunnel Scoring**: Performance of intelligent protocol selection algorithm

**Typical Results:**
- Tunnel manager creation: ~44 ns
- Tunnel scoring: ~268 µs

### 3. MCP Operations (`mcp_operations`)
- **MCP Server Creation**: Speed of creating MCP servers
- **JSON Serialization**: Message serialization for different sizes (small/medium/large)
- **JSON Deserialization**: Message parsing performance

**Typical Results:**
- MCP server creation: ~941 ns
- JSON serialization (small): ~50 ns
- JSON serialization (large): ~6.08 µs
- JSON deserialization (small): ~185 ns
- JSON deserialization (large): ~11.75 µs

### 4. Network Operations (`network_operations`)
- **Node Config Creation**: Performance of configuration object creation
- **Node Builder Creation**: Speed of builder pattern initialization

**Typical Results:**
- Node config creation: ~86 ns
- Node builder creation: ~86 ns

### 5. Concurrent Operations (`concurrent_operations`)
- **Concurrent Key Creation**: Multi-threaded key creation performance (1-16 threads)
- **Concurrent JSON Processing**: Multi-threaded JSON processing (1-8 threads)

**Typical Results:**
- Single-threaded: ~295 µs
- Multi-threaded (16 threads): ~302 µs (minimal overhead)

### 6. Cryptographic Operations (`crypto_operations`)
- **SHA256 Hashing**: Hash performance for different data sizes (64-4096 bytes)
- **Ed25519 Key Generation**: Cryptographic key pair generation
- **Ed25519 Signing**: Digital signature creation
- **Ed25519 Verification**: Signature verification

**Typical Results:**
- SHA256 (64 bytes): ~365 ns
- SHA256 (4096 bytes): ~10.98 µs
- Ed25519 key generation: ~12.08 µs
- Ed25519 signing: ~11.87 µs
- Ed25519 verification: ~28.21 µs

## Performance Analysis

### Key Insights

1. **DHT Operations**: Extremely fast key operations with linear scaling based on data size
2. **Tunneling**: Intelligent protocol selection adds ~268µs overhead but provides significant network optimization
3. **MCP**: JSON processing scales well with message size, suitable for real-time AI tool communication
4. **Concurrency**: Excellent multi-threading performance with minimal overhead
5. **Cryptography**: Ed25519 operations are fast enough for high-frequency P2P communications

### Performance Goals

- **DHT Key Operations**: < 2µs for all key sizes
- **Tunnel Selection**: < 500µs for protocol selection
- **MCP Message Processing**: < 50µs for typical messages
- **Cryptographic Operations**: < 50µs for signing/verification

### Optimization Opportunities

1. **JSON Processing**: Consider binary protocols for very high-throughput scenarios
2. **Tunnel Selection**: Cache network capability detection results
3. **Concurrent Operations**: Further optimize for NUMA architectures

## Adding New Benchmarks

To add new benchmarks, edit `benches/dht_benchmark.rs`:

```rust
fn new_benchmark_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("new_operations");
    
    group.bench_function("operation_name", |b| {
        b.iter(|| {
            // Benchmark code here
            black_box(your_function(black_box(input)))
        });
    });
    
    group.finish();
}

// Add to criterion_group! macro
criterion_group!(
    benches,
    // ... existing benchmarks
    new_benchmark_group
);
```

## Continuous Integration

Benchmarks should be run regularly to detect performance regressions:

```bash
# In CI, run with consistent settings
cargo bench -- --output-format json > benchmark_results.json

# Compare with baseline
cargo bench -- --save-baseline main
cargo bench -- --baseline main
```

## Hardware Considerations

Benchmark results may vary significantly based on:
- CPU architecture (x86_64 vs ARM64)
- Memory hierarchy (L1/L2/L3 cache sizes)
- System load and thermal throttling
- Compiler optimizations and Rust version

For consistent results, run benchmarks on dedicated hardware with consistent conditions.