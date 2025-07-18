# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The P2P Foundation is a fully decentralized networking platform built in Rust, featuring:
- **Three-word network addresses** for human-readable connectivity
- **Quantum-resistant cryptography** (ML-KEM/ML-DSA)
- **Git-like DHT** for universal version control
- **MCP integration** for AI-native capabilities
- **Cross-platform applications** via Tauri (desktop, mobile, and web)

## Build & Development Commands

### Core Library
```bash
# Build entire workspace
cargo build --release

# Run all tests
cargo test

# Run comprehensive test suite with specific modules
./test-runner.sh
./test-runner.sh --module dht
./test-runner.sh --module network

# Run specific integration tests
cargo test --test dht_network_integration_test
```

### Saorsa Desktop App
```bash
cd apps/saorsa

# Development mode with hot reload
cargo tauri dev

# Production build
cargo tauri build

# Quick start (Python server + Tauri)
./run.sh
```

### Quality Checks
```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy --all-features -- -D warnings

# Generate documentation
cargo doc --no-deps --open
```

## High-Level Architecture

### Workspace Structure
```
p2p/
├── crates/                 # Core Rust libraries
│   ├── p2p-core/          # Main P2P library (published as saorsa-core)
│   ├── p2p-cli/           # Command-line tools
│   └── ant-test-suite/    # Comprehensive testing framework
├── apps/
│   ├── saorsa/            # Tauri cross-platform app (desktop, mobile, web)
│   ├── saorsa-terminal-chat/    # Terminal chat application
│   └── saorsa-network-tester/   # Network testing utility
└── docs/                  # Comprehensive documentation
```

### Core Components

1. **Network Layer** (`crates/p2p-core/src/network/`)
   - QUIC/TCP transport with automatic fallback
   - IPv6-first with comprehensive IPv4 tunneling
   - Connection pooling and load balancing

2. **DHT System** (`crates/p2p-core/src/dht/`)
   - Kademlia routing with K=8 replication
   - Git-like content addressing with BLAKE3
   - Quantum-resistant encryption for stored data

3. **Identity Management** (`crates/p2p-core/src/identity/`)
   - Three-word address system
   - ML-KEM/ML-DSA cryptographic foundation
   - FROST threshold cryptography

4. **MCP Integration** (`crates/p2p-core/src/mcp/`)
   - Model Context Protocol servers at each node
   - Tool registry and service discovery
   - AI-native communication capabilities

### Key Design Patterns

**Error Handling**: Use `anyhow::Result` for applications, custom error types for libraries
```rust
use anyhow::{Result, Context};
pub async fn operation() -> Result<Value> {
    something.await.context("Failed to perform operation")?;
    Ok(value)
}
```

**Async Programming**: Always use Tokio runtime
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Implementation
}
```

**Testing**: Comprehensive unit and integration tests
```rust
#[tokio::test]
async fn test_feature() {
    // Arrange, Act, Assert pattern
}
```

## Current Development Focus

The project is actively developing:
1. **Passkey authentication** for the Saorsa app (see recent commits)
2. **Platform-specific implementations** for mobile support
3. **Enhanced identity management** with DHT integration

## Important Notes

- **Test Coverage**: 1400+ lines of comprehensive tests - always run tests before commits
- **Dual Licensing**: AGPL-3.0 for open source, commercial license available
- **Security First**: All data encrypted by default, quantum-resistant cryptography throughout
- **Production Ready**: Not just research - includes connection pooling, load balancing, fault tolerance

## Common Tasks

### Running Multiple P2P Nodes
```bash
# Terminal 1: Bootstrap node
cargo run --bin saorsa -- --port 9001 --bootstrap-file bootstrap.json

# Terminal 2: Additional node
cargo run --bin saorsa -- --port 9002 --bootstrap /ip6/::1/tcp/9001

# Terminal 3: Desktop app
cd apps/saorsa && cargo tauri dev
```

### Working with Tests
```bash
# Run specific test categories
cd crates/saorsa-test-suite
cargo test network_tests
cargo test identity_tests
cargo test crypto_tests

# Run with environment variables
NODES=50 cargo test network_tests
RUST_LOG=debug cargo test -- --nocapture
```

## Additional Resources

- Detailed technical guidance: `/docs/CLAUDE.md`
- Product requirements: `/docs/PRD.md`
- Technical specification: `/docs/SPECIFICATION.md`
- Architecture diagrams: `/docs/architecture/`