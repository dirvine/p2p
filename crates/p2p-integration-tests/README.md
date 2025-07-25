# P2P Integration Tests

Comprehensive integration test suite for the adaptive P2P network, providing thorough testing of distributed system behavior, resilience, and performance.

## Overview

This test suite validates the P2P network's behavior under various conditions:
- Multi-node network formation and operation
- Network resilience under churn
- Security against various attack vectors
- Performance characteristics and scalability
- System behavior under chaos conditions

## Test Categories

### 1. Multi-Node Tests (`multi_node.rs`)
Tests basic network functionality with multiple nodes:
- Network formation and peer discovery
- Content storage and retrieval across nodes
- Gossip message propagation
- Trust establishment between peers
- Different network topologies (random, ring, star, mesh, hierarchical)

### 2. Churn Simulation (`churn_simulation.rs`)
Tests network resilience under node churn:
- Random node failures and recoveries
- Correlated failures (datacenter outages)
- Flash crowd behavior
- Network partitions and healing
- Diurnal patterns
- Extreme churn scenarios

### 3. Attack Scenarios (`attack_scenarios.rs`)
Tests security defenses against attacks:
- Eclipse attacks (isolating nodes)
- Sybil attacks (fake identities)
- Content poisoning
- Denial of Service (DoS)
- Routing attacks
- Trust manipulation
- Combined attack scenarios

### 4. Performance Benchmarks (`performance_benchmarks.rs`)
Measures system performance:
- Storage/retrieval throughput
- Routing latency
- Message propagation speed
- Concurrent operation handling
- Scalability analysis
- Stress testing under adverse conditions

### 5. Chaos Testing (`chaos_testing.rs`)
Tests system resilience through controlled chaos:
- Component failures
- Resource exhaustion
- Byzantine behavior
- Clock skew
- Network chaos (packet loss, reordering)
- Combined chaos scenarios

## Running Tests

### Basic Test Execution
```bash
# Run all integration tests
cargo test

# Run specific test category
cargo test --test multi_node
cargo test --test churn_simulation
cargo test --test attack_scenarios
cargo test --test performance_benchmarks
cargo test --test chaos_testing

# Run specific test
cargo test --test multi_node test_network_formation

# Run with logging
RUST_LOG=info cargo test -- --nocapture

# Run ignored (long-running) tests
cargo test -- --ignored
```

### Test Configuration
Many tests can be configured through environment variables:
- `RUST_LOG`: Set logging level (error, warn, info, debug, trace)
- `NODES`: Number of nodes to spawn (for scalability tests)
- `TEST_DURATION`: Duration for long-running tests

### Parallel Execution
Some tests require serial execution due to resource constraints:
```bash
# Run tests serially
cargo test -- --test-threads=1

# Run with limited parallelism
cargo test -- --test-threads=2
```

## CI Integration

The test suite is integrated with GitHub Actions for continuous testing:

### Workflows
1. **PR Tests**: Run on every pull request
   - Basic multi-node tests
   - Security attack scenarios
   - Performance regression checks

2. **Nightly Tests**: Extended test suite
   - Large-scale network tests (200+ nodes)
   - Long-running stress tests
   - Extreme churn scenarios
   - Full scalability analysis

3. **Release Tests**: Comprehensive validation
   - All test categories
   - Multiple OS/architecture matrix
   - Performance benchmarking
   - Security audit

### CI Configuration
See `.github/workflows/integration_tests.yml` for CI setup.

## Test Framework Architecture

### Core Components

1. **TestCluster**: Manages multi-node test networks
   - Node lifecycle management
   - Network topology configuration
   - Statistics collection

2. **TestNode**: Wrapper around P2P node for testing
   - Component access
   - State tracking
   - Statistics collection

3. **Network Conditions**: Simulates real-world conditions
   - Packet loss
   - Latency and jitter
   - Bandwidth limitations
   - Node failures

### Utilities

- `generate_content()`: Creates random test data
- `measure_latency()`: Times operations
- `parallel_operations()`: Runs concurrent tests
- `calculate_latency_stats()`: Computes percentiles

## Writing New Tests

### Test Structure
```rust
#[tokio::test]
async fn test_new_scenario() -> Result<()> {
    // 1. Setup
    let config = TestClusterConfig {
        node_count: 50,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // 2. Execute test scenario
    // ... test logic ...
    
    // 3. Verify results
    assert!(condition, "Explanation");
    
    // 4. Cleanup
    cluster.shutdown().await?;
    Ok(())
}
```

### Best Practices
1. Use appropriate timeouts for operations
2. Clean up resources in test teardown
3. Use descriptive assertion messages
4. Log important test events
5. Consider test isolation requirements

## Performance Considerations

### Resource Usage
- Each test node consumes ~50-100MB RAM
- Large-scale tests may require 8GB+ RAM
- CPU usage scales with node count and activity

### Optimization Tips
1. Reuse test clusters when possible
2. Use smaller node counts for quick tests
3. Run resource-intensive tests with `--release`
4. Limit concurrent operations to avoid overwhelm

## Troubleshooting

### Common Issues

1. **Port conflicts**: Tests use ports 4000-5000
   - Solution: Ensure ports are free or adjust base port

2. **Timeout failures**: Operations taking too long
   - Solution: Increase timeouts or reduce node count

3. **Resource exhaustion**: Out of memory/file descriptors
   - Solution: Reduce test scale or increase limits

4. **Flaky tests**: Intermittent failures
   - Solution: Add retries or increase stabilization time

### Debug Techniques
```bash
# Verbose logging
RUST_LOG=debug cargo test -- --nocapture

# Single test with full output
cargo test test_name -- --exact --nocapture

# Generate flamegraph (requires cargo-flamegraph)
cargo flamegraph --test performance_benchmarks

# Memory profiling (requires valgrind)
valgrind --leak-check=full cargo test
```

## Contributing

When adding new integration tests:
1. Choose appropriate test category
2. Document test purpose and requirements
3. Use existing utilities and patterns
4. Ensure tests are deterministic
5. Add to CI workflow if needed
6. Update this README

## License

Licensed under AGPL-3.0 - see LICENSE file for details.