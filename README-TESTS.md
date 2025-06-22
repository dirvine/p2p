# P2P Foundation Integration Test Suite

This document describes the comprehensive integration test suite for the P2P Foundation project.

## Overview

The integration test suite provides complete coverage of all P2P Foundation APIs and functionality, serving as both validation and living documentation for the system. The tests are designed to:

- **100% API Coverage**: Every public API method and configuration option is tested
- **Real-world Scenarios**: Tests simulate actual usage patterns and edge cases  
- **Performance Validation**: Benchmarks ensure the system meets performance requirements
- **Reliability Testing**: Stress tests validate system behavior under load
- **Security Verification**: Comprehensive security and cryptographic testing

## Test Architecture

```
tests/integration/
├── common/           # Shared test utilities and helpers
├── network/          # Network layer and peer connectivity tests
├── dht/             # Distributed Hash Table functionality tests
├── transport/       # QUIC/TCP transport layer tests
├── tunneling/       # IPv6/IPv4 tunneling protocol tests
├── mcp/             # Model Context Protocol server tests
├── security/        # Cryptography and security tests
├── scenarios/       # End-to-end integration scenarios
└── main.rs          # Test runner and configuration
```

## Test Categories

### 1. Network Module Tests (`tests/integration/network/`)

Tests core networking functionality:
- Node creation and configuration
- Peer discovery and connection management
- Network topology and routing
- Connection stability and recovery
- IPv4/IPv6 dual stack support
- Connection limits and resource management

**Key Test Cases:**
- `test_node_creation_default()` - Basic node setup
- `test_peer_connection()` - Two-node connectivity
- `test_multiple_peer_connections()` - Multi-node network formation
- `test_network_resilience()` - Node failure recovery
- `test_mixed_ipv4_ipv6_network()` - Mixed IP version networking

### 2. DHT Module Tests (`tests/integration/dht/`)

Tests Distributed Hash Table operations:
- Key-value storage and retrieval
- DHT routing and replication
- Network partition tolerance
- Data consistency and conflict resolution
- Performance and scalability

**Key Test Cases:**
- `test_dht_basic_put_get()` - Basic storage operations
- `test_dht_routing()` - Key routing to closest peers
- `test_dht_replication()` - Data replication across nodes
- `test_dht_network_partition()` - Partition tolerance
- `test_dht_performance()` - Throughput and latency benchmarks

### 3. Transport Layer Tests (`tests/integration/transport/`)

Tests transport protocols:
- QUIC transport with Quinn
- TCP fallback transport
- Transport switching and adaptation
- Performance and reliability
- Security and encryption

**Key Test Cases:**
- `test_quic_transport_basic()` - QUIC functionality
- `test_tcp_transport_basic()` - TCP fallback
- `test_transport_switching()` - Dynamic protocol switching
- `test_transport_security()` - Encrypted communication
- `test_transport_performance_comparison()` - QUIC vs TCP benchmarks

### 4. Tunneling Protocol Tests (`tests/integration/tunneling/`)

Tests IPv6/IPv4 tunneling:
- 6to4 tunneling for IPv6 over IPv4
- Teredo tunneling for NAT traversal
- 6in4 manual tunnels
- Protocol auto-selection and fallback
- Performance and reliability

**Key Test Cases:**
- `test_6to4_tunneling_basic()` - 6to4 tunnel setup
- `test_teredo_tunneling()` - NAT traversal with Teredo
- `test_tunnel_auto_selection()` - Automatic protocol selection
- `test_tunnel_failover()` - Tunnel redundancy and failover
- `test_tunnel_performance()` - Tunneling overhead measurement

### 5. MCP Server Tests (`tests/integration/mcp/`)

Tests Model Context Protocol functionality:
- Tool registration and discovery
- Remote procedure calls
- Service capability advertisement
- Cross-node MCP communication
- Performance and reliability

**Key Test Cases:**
- `test_mcp_server_basic_setup()` - MCP server initialization
- `test_mcp_service_discovery()` - Service discovery across nodes
- `test_remote_mcp_tool_invocation()` - Cross-node RPC calls
- `test_mcp_streaming()` - Streaming operations
- `test_mcp_load_balancing()` - Service load distribution

### 6. Security Module Tests (`tests/integration/security/`)

Tests security and cryptographic functionality:
- Cryptographic key management
- Message signing and verification
- Peer authentication and authorization
- Secure communication channels
- Attack resistance and security properties

**Key Test Cases:**
- `test_key_generation()` - Cryptographic key generation
- `test_message_signing()` - Digital signatures
- `test_peer_authentication()` - Mutual authentication
- `test_secure_channel()` - End-to-end encryption
- `test_replay_attack_protection()` - Security properties

### 7. End-to-End Scenarios (`tests/integration/scenarios/`)

Tests complete application workflows:
- AI agent collaboration scenarios
- Distributed file storage and retrieval
- Network partition and recovery
- Real-time collaborative editing
- Performance under realistic load

**Key Test Cases:**
- `test_ai_agent_collaboration()` - Multi-agent AI workflow
- `test_distributed_file_storage()` - Large file distribution
- `test_network_partition_healing()` - Network split-brain recovery
- `test_collaborative_editing()` - Real-time document editing
- `test_realistic_load_performance()` - Mixed workload simulation

## Running Tests

### Quick Start

```bash
# Run all tests with default configuration
./test-runner.sh

# Run specific test module
./test-runner.sh dev --module network

# Run with different environment
./test-runner.sh ci
```

### Test Environments

#### Development Environment (`dev`)
- **Purpose**: Comprehensive testing during development
- **Configuration**: 5 nodes, 600s timeout, all features enabled
- **Usage**: `./test-runner.sh dev`

#### CI Environment (`ci`) 
- **Purpose**: Fast testing in continuous integration
- **Configuration**: 3 nodes, 180s timeout, minimal features
- **Usage**: `./test-runner.sh ci`

#### Benchmark Environment (`bench`)
- **Purpose**: Performance testing and benchmarking
- **Configuration**: 10 nodes, 900s timeout, benchmarks enabled
- **Usage**: `./test-runner.sh bench`

#### Stress Environment (`stress`)
- **Purpose**: Reliability testing under extreme conditions
- **Configuration**: 8 nodes, 1200s timeout, stress tests enabled
- **Usage**: `./test-runner.sh stress`

### Command Line Options

```bash
./test-runner.sh [ENVIRONMENT] [OPTIONS]

Options:
  -n, --node-count NUM      Number of test nodes (default: 3)
  -p, --port NUM           Base port number (default: 9000)
  -t, --timeout NUM        Test timeout in seconds (default: 300)
  -l, --log-level LEVEL    Log level: debug,info,warn,error
  --ipv6 / --no-ipv6      Enable/disable IPv6 tests
  --benchmarks             Enable benchmark tests
  --stress                 Enable stress tests
  --module MODULE          Run specific test module only
  --features FEATURES      Cargo features to enable
  --clean                  Clean build before running
  --release                Run in release mode
  --single-thread          Run tests single-threaded
```

### Environment Variables

```bash
export P2P_TEST_ENV=dev                    # Test environment
export P2P_TEST_NODE_COUNT=5               # Number of nodes
export P2P_TEST_BASE_PORT=9000             # Base port
export P2P_TEST_TIMEOUT=300                # Timeout in seconds
export P2P_TEST_LOG_LEVEL=info             # Log level
export P2P_TEST_ENABLE_IPV6=true           # IPv6 support
export P2P_TEST_ENABLE_BENCHMARKS=true     # Benchmarks
export P2P_TEST_ENABLE_STRESS=false        # Stress tests
```

## Test Utilities

### TestNetwork

Central utility for creating test networks:

```rust
use crate::common::TestNetwork;

// Create simple test network
let network = TestNetwork::simple(3).await?;

// Create with custom configuration
let config = TestNetworkConfig {
    node_count: 5,
    base_port: 9000,
    enable_ipv6: true,
    ..Default::default()
};
let network = TestNetwork::new(config).await?;

// Access nodes
let node = network.node(0)?;
let peer_id = node.peer_id();
```

### TestNodeConfig

Builder for test node configurations:

```rust
use crate::common::TestNodeConfig;

let config = TestNodeConfig::builder()
    .port(9001)
    .enable_ipv6(true)
    .enable_mcp(true)
    .bootstrap_peers(bootstrap_addrs)
    .build();

let node = P2PNode::new(config).await?;
```

### TestDataGen

Utilities for generating test data:

```rust
use crate::common::TestDataGen;

// Generate random test data
let data = TestDataGen::random_bytes(1024);

// Generate DHT keys
let key = TestDataGen::dht_key("test_prefix");

// Generate MCP tool configurations
let tool = TestDataGen::mcp_tool("test_tool");
```

### TestAssertions

Helper functions for common assertions:

```rust
use crate::common::TestAssertions;

// Assert network connectivity
TestAssertions::assert_full_connectivity(&network).await?;

// Assert DHT convergence
TestAssertions::assert_dht_convergence(&network, &key, &value).await?;

// Assert MCP service availability
TestAssertions::assert_mcp_availability(&network).await?;
```

### PerformanceTest

Performance measurement utilities:

```rust
use crate::common::PerformanceTest;

let mut perf = PerformanceTest::new();

// Measure synchronous operation
let result = perf.measure("operation_name", || {
    expensive_operation()
});

// Measure asynchronous operation
let result = perf.measure_async("async_operation", async {
    async_operation().await
}).await;

// Print results
perf.print_results();
```

## Continuous Integration

The test suite integrates with GitHub Actions for automated testing:

### Workflow Configuration (`.github/workflows/integration-tests.yml`)

- **Matrix Testing**: Tests against stable and beta Rust versions
- **Feature Testing**: Tests default and all-features configurations
- **Incremental Testing**: Runs test modules separately for faster feedback
- **Comprehensive Testing**: Nightly runs with full feature set
- **Security Auditing**: Automated security vulnerability scanning
- **Coverage Reporting**: Test coverage analysis and reporting

### CI Test Strategy

1. **Pull Request Tests**: Fast subset for quick feedback
2. **Main Branch Tests**: Full test suite on merge
3. **Nightly Tests**: Comprehensive testing with stress tests
4. **Release Tests**: Performance benchmarking and validation

## Performance Benchmarks

### Network Performance
- **Connection Establishment**: < 100ms for local connections
- **Peer Discovery**: < 5s for 10-node network convergence
- **Throughput**: > 100 Mbps for large message transmission

### DHT Performance  
- **Single Operations**: < 50ms for put/get operations
- **Replication**: < 2s for 3-replica convergence
- **Concurrent Load**: > 1000 ops/sec sustained throughput

### MCP Performance
- **Service Discovery**: < 3s for cross-node service advertisement  
- **RPC Latency**: < 20ms for simple remote calls
- **Concurrent Calls**: > 100 concurrent RPC operations

### Transport Performance
- **QUIC vs TCP**: QUIC should provide 10-20% better throughput
- **Tunnel Overhead**: < 20% performance penalty for tunneling
- **Connection Pooling**: > 50% reduction in connection establishment time

## Development Workflow

### Test-Driven Development

1. **Write Tests First**: Create integration tests for new features
2. **Implement to Pass**: Develop code to make tests pass
3. **Refactor**: Improve implementation while maintaining test coverage
4. **Document**: Update test documentation with new test cases

### Adding New Tests

1. **Identify Test Category**: Determine which module the test belongs to
2. **Create Test Function**: Add test function with descriptive name
3. **Use Test Utilities**: Leverage existing utilities for common operations
4. **Add to Test Runner**: Include in appropriate test module
5. **Update Documentation**: Document new test case

### Debugging Test Failures

1. **Check Logs**: Review test output with appropriate log level
2. **Isolate Issue**: Run specific test module or single test
3. **Use Debug Mode**: Run with `--log-level debug` for verbose output
4. **Check Resources**: Verify ports, memory, and system requirements
5. **Environment Variables**: Ensure test environment is configured correctly

## Best Practices

### Test Design
- **Independent Tests**: Each test should be self-contained
- **Deterministic**: Tests should produce consistent results
- **Fast Feedback**: Optimize for quick test execution
- **Realistic**: Test real-world scenarios and edge cases
- **Comprehensive**: Cover all public APIs and configurations

### Resource Management
- **Clean Shutdown**: Always properly shut down test nodes
- **Port Management**: Use unique ports to avoid conflicts
- **Memory Usage**: Monitor memory consumption in large tests
- **Timeout Handling**: Set appropriate timeouts for operations
- **Error Propagation**: Ensure errors are properly reported

### Maintenance
- **Regular Updates**: Keep tests current with API changes
- **Performance Monitoring**: Track test execution time trends
- **Flaky Test Investigation**: Address intermittent test failures
- **Documentation**: Maintain test documentation accuracy
- **Cleanup**: Remove obsolete tests and utilities

## Troubleshooting

### Common Issues

#### Port Conflicts
```bash
# Check for port usage
netstat -tuln | grep :9000

# Use different base port
./test-runner.sh dev --port 8000
```

#### IPv6 Issues
```bash
# Check IPv6 support
ip -6 addr show lo

# Disable IPv6 tests
./test-runner.sh dev --no-ipv6
```

#### Memory Issues
```bash
# Check available memory
free -h

# Reduce node count
./test-runner.sh dev --node-count 3
```

#### Timeout Issues
```bash
# Increase timeout
./test-runner.sh dev --timeout 600

# Run single-threaded
./test-runner.sh dev --single-thread
```

### Getting Help

- **Test Logs**: Review detailed logs with debug level
- **GitHub Issues**: Report persistent failures or bugs
- **Documentation**: Check module-specific test documentation
- **Community**: Discuss testing approaches and issues

---

For more information about the P2P Foundation project, see the main [README.md](README.md) and [SPECIFICATION.md](SPECIFICATION.md) files.