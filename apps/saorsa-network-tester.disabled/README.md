# Saorsa Network Tester

A comprehensive network testing tool for the Saorsa P2P platform, featuring real-world tests of QUIC transport, DHT operations, and peer connectivity.

## Features

- **Quick Network Test**: Comprehensive test of P2P node creation and basic operations
- **DHT Storage Test**: Test distributed hash table storage and retrieval
- **Peer Connection Test**: Test connectivity between multiple nodes
- **Network Info**: Display detailed network configuration and capabilities

## Building

From the project root:

```bash
cargo build --release -p saorsa-network-tester
```

Or use the build script:

```bash
./build-terminal-apps.sh
```

## Usage

Run the network tester:

```bash
./target/release/saorsa-network-tester
```

## Test Options

### 1. Quick P2P Network Test

Tests:
- P2P node creation with QUIC transport
- Network address binding (IPv6/IPv4)
- DHT operations (put/get)
- Event system subscription

### 2. DHT Storage Test

Tests:
- Multiple key-value pair storage
- Data retrieval and verification
- Large value handling
- DHT replication

### 3. Peer Connection Test

Tests:
- Multi-node creation
- Peer-to-peer connectivity
- Message sending between nodes
- Network routing

### 4. Network Info

Displays:
- Node peer ID
- Listen addresses
- Transport capabilities (QUIC/TCP)
- IPv6/IPv4 support
- Enabled features (DHT, MCP, crypto)

## Technical Architecture

The network tester validates:

- **Transport Layer**: QUIC with automatic fallback to TCP
- **IPv6 Support**: Native IPv6 with tunneling (Teredo/6to4/DS-Lite)
- **DHT Operations**: Kademlia-based distributed storage
- **Event System**: Async event handling with broadcast channels
- **Production Features**: Rate limiting, connection pooling, health checks

## Test Results

Each test provides:
- Pass/fail status for individual checks
- Performance metrics (test duration)
- Detailed error messages for failures
- Overall network health assessment

## Use Cases

- Verify P2P network setup
- Test network connectivity
- Benchmark DHT performance
- Validate production configuration
- Debug connectivity issues