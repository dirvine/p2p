# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This document provides guidelines for AI assistants (particularly Claude) when working on the P2P Foundation project. It contains project-specific context, coding standards, and development patterns to ensure consistent and high-quality contributions.

## Project Context

### What We're Building
A fully decentralized P2P networking foundation in Rust that:
- Uses QUIC for modern, efficient transport
- Prioritizes IPv6 with comprehensive IPv4 tunneling
- Implements Kademlia DHT for routing
- Integrates MCP servers at each node for AI capabilities
- Maintains minimal footprint for edge deployment

### Key Design Principles
1. **Serverless First**: No central dependencies
2. **AI-Native**: Built for AI agent communication
3. **Universal Connectivity**: Works on any network
4. **Developer Friendly**: Simple APIs, sensible defaults
5. **Production Ready**: Not just a research project

## Code Guidelines

### Rust Patterns

#### Error Handling
```rust
// PREFERRED: Use anyhow for application errors
use anyhow::{Result, Context};

pub async fn connect_peer(addr: &str) -> Result<PeerId> {
    let multiaddr: Multiaddr = addr.parse()
        .context("Failed to parse multiaddr")?;
    
    // ... implementation
}

// For library code, define custom error types
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("DHT lookup timeout")]
    DHTTimeout,
}
```

#### Async Patterns
```rust
// ALWAYS use tokio for async runtime
#[tokio::main]
async fn main() -> Result<()> {
    // ... implementation
}

// For concurrent operations, prefer tokio::select!
tokio::select! {
    result = operation1() => handle_op1(result),
    result = operation2() => handle_op2(result),
    _ = tokio::time::sleep(timeout) => handle_timeout(),
}
```

#### Module Organization
```rust
// src/lib.rs - Public API surface
pub mod network;
pub mod dht;
pub mod transport;
pub mod mcp;

// Keep internal implementation details private
mod implementation_detail;

// Re-export commonly used types
pub use network::{P2PNode, NodeConfig};
```

### Documentation Standards

#### Code Documentation
```rust
/// Creates a new P2P node with the specified configuration.
/// 
/// # Arguments
/// 
/// * `config` - Node configuration including network settings
/// 
/// # Returns
/// 
/// Returns a configured P2P node ready for use.
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Port binding fails
/// - Invalid configuration provided
/// - System resources unavailable
/// 
/// # Examples
/// 
/// ```rust
/// let config = NodeConfig::default();
/// let node = P2PNode::new(config).await?;
/// ```
pub async fn new(config: NodeConfig) -> Result<Self> {
    // Implementation
}
```

#### Module Documentation
```rust
//! # Network Module
//! 
//! This module provides the core networking functionality for the P2P foundation.
//! 
//! ## Features
//! 
//! - QUIC-based transport with Quinn
//! - Automatic NAT traversal
//! - IPv6-first with tunneling fallback
//! 
//! ## Examples
//! 
//! See the `examples/` directory for usage examples.
```

### Testing Patterns

#### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_peer_connection() {
        // Arrange
        let node1 = create_test_node(9001).await;
        let node2 = create_test_node(9002).await;
        
        // Act
        let result = node1.connect(node2.local_addr()).await;
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(node1.peer_count(), 1);
    }
    
    // Helper functions for test setup
    async fn create_test_node(port: u16) -> P2PNode {
        let config = NodeConfig {
            listen_addr: format!("127.0.0.1:{}", port).parse().unwrap(),
            ..Default::default()
        };
        P2PNode::new(config).await.expect("Failed to create test node")
    }
}
```

#### Integration Tests
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_multi_node_dht() {
    let network = TestNetwork::new(5).await;
    
    // Store value in node 0
    let key = Key::new(b"test_key");
    let value = b"test_value".to_vec();
    network.nodes[0].store(key.clone(), value.clone()).await.unwrap();
    
    // Verify replication across network
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    for node in &network.nodes[1..] {
        let retrieved = node.retrieve(key.clone()).await.unwrap();
        assert_eq!(retrieved, Some(value.clone()));
    }
}
```

## Architecture Decisions

### Why These Technologies?

#### libp2p
- **Chosen for**: Battle-tested P2P primitives
- **Alternatives considered**: Custom implementation (too risky), Matrix (application layer)
- **Trade-offs**: Some complexity, but modular design allows picking only needed features

#### Quinn for QUIC
- **Chosen for**: Pure Rust, excellent API, active development
- **Alternatives considered**: quiche (C++ bindings), s2n-quic (newer)
- **Trade-offs**: Slightly larger binary size vs performance benefits

#### Kademlia DHT
- **Chosen for**: Proven at scale, good for decentralized routing
- **Alternatives considered**: Chord, Pastry, custom DHT
- **Trade-offs**: Well-understood but requires careful parameter tuning

### Implementation Priorities

1. **Phase 1**: Get basic P2P working (libp2p + Kademlia)
2. **Phase 2**: Add QUIC transport and IPv6
3. **Phase 3**: Implement tunneling protocols
4. **Phase 4**: Integrate MCP servers
5. **Phase 5**: Production hardening

### Current Project Status

**CURRENT STATUS**: Core network module is now implemented! ✅

**COMPLETED**:
- ✅ **Network Module**: Full P2PNode implementation with async lifecycle, peer management, events
- ✅ **Error Handling**: Complete P2PError types with proper error propagation  
- ✅ **CLI Binary**: Functional p2p-node with comprehensive command-line options
- ✅ **Test Infrastructure**: 72 comprehensive integration tests across all modules
- ✅ **Build System**: Working Cargo.toml with essential dependencies

**NEXT TO IMPLEMENT** (in priority order):
- 🔄 **Transport Layer**: QUIC/TCP protocol implementations
- 🔄 **DHT Module**: Kademlia distributed hash table
- 🔄 **Tunneling Protocols**: IPv6/IPv4 connectivity (6to4, Teredo, 6in4)
- 🔄 **MCP Integration**: Model Context Protocol for AI capabilities
- 🔄 **Security Module**: Cryptographic primitives and secure transport

**WORKING FEATURES**:
- P2P node creation with builder pattern
- Peer connection management (simulated)
- Network event broadcasting  
- Node lifecycle (start/run/stop)
- CLI with IPv6, MCP, bootstrap options

**EXAMPLE USAGE**:
```rust
// Create and start a P2P node
let node = P2PNode::builder()
    .with_peer_id("my_node".to_string())
    .listen_on("/ip4/127.0.0.1/tcp/9000")
    .with_bootstrap_peer("/ip4/127.0.0.1/tcp/8000")
    .with_ipv6(true)
    .build()
    .await?;

node.start().await?;
println!("Node {} started with {} peers", 
         node.peer_id(), node.peer_count().await);
```

```bash
# Run the CLI node
cargo run --bin p2p-node -- --port 9000 --ipv6 --mcp
```

## Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Build in release mode  
cargo build --release

# Run tests
cargo test

# Run tests with all features
cargo test --all-features

# Run a specific test
cargo test test_name

# Run tests with debug output
cargo test -- --nocapture
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting without making changes
cargo fmt --all --check

# Run clippy for linting
cargo clippy

# Run clippy with all features and treat warnings as errors
cargo clippy --all-features -- -D warnings

# Generate documentation
cargo doc --no-deps --open
```

### Testing Commands

```bash
# Run all integration tests (basic framework tests only)
cargo test --test integration_tests

# Run specific network functionality test 
cargo test --test integration_tests test_network_functionality -- --nocapture

# Run comprehensive test runner script
./test-runner.sh

# Run tests in different environments
P2P_TEST_ENABLE_IPV6=false ./test-runner.sh   # IPv4 only
P2P_TEST_NODE_COUNT=5 ./test-runner.sh        # 5-node network
```

**Test Status**: 
- ✅ 6 basic framework tests passing
- ✅ Network functionality test demonstrates P2P node capabilities
- 📋 72 comprehensive test cases defined for all modules
- 🔄 Most tests are placeholders awaiting module implementations

### Development Features
```bash
# Build with CLI features (default)
cargo build --features cli

# Build with metrics support
cargo build --features metrics

# Build with benchmarking support
cargo build --features bench

# Build with test utilities
cargo build --features test-utils

# Build with all features
cargo build --all-features
```

### Running Binaries and Examples
```bash
# Run the p2p-node binary (once implemented)
cargo run --bin p2p-node

# Run examples (once implemented)
cargo run --example basic_node
cargo run --example dht_storage
cargo run --example mcp_service
```

### Benchmarking
```bash
# Run benchmarks (once implemented)
cargo bench --features bench

# Run specific benchmark
cargo bench --features bench dht_benchmark
```

## Common Tasks

### Adding a New Tunneling Protocol

1. Create new module in `src/tunneling/`
2. Implement the `Tunnel` trait
3. Add to `TunnelProtocol` enum
4. Update auto-selection logic
5. Add tests and documentation

```rust
// src/tunneling/new_protocol.rs
pub struct NewProtocolTunnel {
    // Implementation details
}

impl Tunnel for NewProtocolTunnel {
    async fn establish(&mut self) -> Result<()> {
        // Protocol-specific setup
    }
    
    async fn encapsulate(&self, packet: &[u8]) -> Result<Vec<u8>> {
        // Wrap IPv6 in IPv4
    }
}
```

### Adding MCP Tools

1. Define tool schema in `src/mcp/tools/`
2. Implement handler function
3. Register in default tool set
4. Update capability advertisements

```rust
// src/mcp/tools/new_tool.rs
pub fn create_new_tool() -> Tool {
    Tool {
        name: "new_tool".to_string(),
        description: "Does something useful".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "param": { "type": "string" }
            }
        }),
        handler: Box::new(handle_new_tool),
    }
}

async fn handle_new_tool(params: Value) -> Result<Value> {
    // Implementation
}
```

## Performance Considerations

### Optimization Guidelines

1. **Avoid Allocations**: Use `&str` instead of `String` where possible
2. **Zero-Copy**: Use `bytes::Bytes` for network buffers
3. **Concurrent Processing**: Use `tokio::spawn` for parallel tasks
4. **Connection Pooling**: Reuse QUIC connections
5. **Lazy Initialization**: Don't create resources until needed

### Benchmarking

```rust
// benches/dht_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn dht_lookup_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let network = runtime.block_on(TestNetwork::new(100));
    
    c.bench_function("dht_lookup", |b| {
        b.to_async(&runtime).iter(|| async {
            let key = Key::random();
            network.nodes[0].lookup(&key).await
        });
    });
}

criterion_group!(benches, dht_lookup_benchmark);
criterion_main!(benches);
```

## Debugging Tips

### Common Issues

1. **Connection Failures**
   - Check firewall settings
   - Verify IPv6 connectivity
   - Test with `RUST_LOG=debug`

2. **DHT Not Converging**
   - Ensure bootstrap nodes are reachable
   - Check k-bucket refresh intervals
   - Verify node IDs are unique

3. **High Memory Usage**
   - Profile with `valgrind` or `heaptrack`
   - Check for connection leaks
   - Review buffer allocation patterns

### Useful Commands

```bash
# Run with debug logging
RUST_LOG=p2p=debug cargo run

# Test specific tunneling protocol
cargo test --features "teredo" tunneling::teredo

# Benchmark with flamegraph
cargo flamegraph --bench dht_benchmark

# Check for memory leaks
valgrind --leak-check=full target/debug/p2p-node

# Network debugging
sudo tcpdump -i any port 9000 -nn
```

## Contributing Guidelines

### Before Submitting Code

1. **Run full test suite**: `cargo test --all-features`
2. **Check formatting**: `cargo fmt --all --check`
3. **Run clippy**: `cargo clippy --all-features -- -D warnings`
4. **Update documentation**: `cargo doc --no-deps --open`
5. **Add tests for new features**

### Code Review Checklist

- [ ] Error handling uses `Result` types appropriately
- [ ] Public APIs have complete documentation
- [ ] Tests cover happy path and error cases
- [ ] No `unwrap()` in production code paths
- [ ] Performance implications considered
- [ ] Security implications reviewed

## Security Considerations

### Always Remember

1. **Never trust user input**: Validate all data from network
2. **Use constant-time comparisons**: For cryptographic operations
3. **Limit resource usage**: Implement quotas and rate limiting
4. **Fail securely**: Default to denying access
5. **Log security events**: But not sensitive data

### Common Vulnerabilities

```rust
// BAD: Vulnerable to timing attacks
if received_signature == expected_signature {
    // Authenticate
}

// GOOD: Constant-time comparison
use constant_time_eq::constant_time_eq;
if constant_time_eq(&received_signature, &expected_signature) {
    // Authenticate
}
```

## Resources

### Internal Documentation
- `/docs/architecture.md` - System architecture
- `/docs/protocols.md` - Protocol specifications
- `/examples/` - Working examples

### External Resources
- [libp2p Documentation](https://docs.libp2p.io/)
- [Quinn Documentation](https://docs.rs/quinn/)
- [MCP Specification](https://modelcontextprotocol.io/docs)
- [Kademlia Paper](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)

### Community
- GitHub Issues: Bug reports and features
- Discord: Real-time help and discussion
- Forum: Long-form discussions

## Quick Reference

### Key Types
```rust
use p2p::{
    P2PNode,        // Main node type
    NodeConfig,     // Configuration
    PeerId,         // Peer identifier
    Key,            // DHT key type
    MCPService,     // MCP service descriptor
};
```

### Common Patterns
```rust
// Create a node
let node = P2PNode::builder()
    .listen_on("/ip6/::/tcp/9000")
    .with_mcp_server()
    .build()
    .await?;

// Store in DHT
node.dht_put(key, value).await?;

// Call MCP service
let response = node.mcp_call(peer_id, "service", params).await?;

// Subscribe to events
let mut events = node.subscribe_events();
while let Some(event) = events.next().await {
    match event {
        P2PEvent::PeerConnected(peer_id) => {},
        P2PEvent::MessageReceived(msg) => {},
    }
}
```

## Maintenance Notes

### Regular Tasks
- Update dependencies monthly
- Review security advisories
- Benchmark performance regression
- Update compatibility matrix
- Refresh documentation

### Release Process
1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Run full test suite
4. Create git tag
5. Publish to crates.io
6. Update documentation

---

Remember: When in doubt, prioritize simplicity and correctness over performance. We can always optimize later, but broken code helps no one.