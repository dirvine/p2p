# P2P Foundation Repository Overview

## Purpose

The P2P Foundation is a comprehensive peer-to-peer networking platform that implements revolutionary distributed systems technologies. It provides a fully decentralized network with quantum-resistant cryptography, human-readable addressing, and adaptive machine learning optimization. The repository contains both the core networking library (published as `saorsa-core`) and the flagship Saorsa chat application.

## Key Features

### Core Networking
- **Adaptive P2P Network**: Self-optimizing network that learns and improves over time
- **Three/Four-Word Addresses**: Human-readable network addresses (e.g., "forest.lightning.compass")
- **Quantum-Resistant Crypto**: ML-KEM and ML-DSA cryptographic primitives
- **Git-like DHT**: Distributed hash table with content addressing
- **QUIC Transport**: Modern, efficient network protocol with 0-RTT connections
- **MCP Integration**: Model Context Protocol for AI-native capabilities

### Adaptive Network Components
- **Secure Kademlia (S/Kademlia)**: Foundational DHT with cryptographic security
- **Hyperbolic Geometry Routing**: Efficient O(1) greedy routing
- **Self-Organizing Maps (SOM)**: Content and capability clustering
- **EigenTrust++ Reputation**: Decentralized trust management
- **Adaptive GossipSub**: Scalable pub/sub messaging
- **Machine Learning Optimization**: Thompson Sampling, Q-Learning, LSTM prediction

### Applications
- **Saorsa Desktop**: Cross-platform chat application (Tauri-based)
- **Terminal Chat**: Command-line messaging application
- **Network Tester**: Testing and debugging tools

## Architecture

The project follows a modular monorepo structure:

```
p2p/
├── crates/                    # Rust libraries
│   ├── p2p-core/             # Core networking library (saorsa-core)
│   │   ├── src/
│   │   │   ├── adaptive/     # Adaptive P2P network implementation
│   │   │   ├── bootstrap/    # Network bootstrapping
│   │   │   ├── dht/          # Distributed hash table
│   │   │   ├── identity/     # Identity management
│   │   │   ├── mcp/          # Model Context Protocol
│   │   │   └── network/      # Core networking
│   │   ├── benches/          # Performance benchmarks
│   │   └── examples/         # Usage examples
│   ├── p2p-cli/              # Command-line tools
│   ├── ant-test-suite/       # Comprehensive test framework
│   └── p2p-integration-tests/# Integration test suite
├── apps/                      # Applications
│   ├── saorsa/               # Tauri desktop/mobile/web app
│   ├── saorsa-terminal-chat/ # Terminal chat client
│   └── saorsa-network-tester/# Network testing utility
├── docs/                      # Documentation
│   ├── architecture/         # System design docs
│   ├── api/                  # API references
│   ├── deployment/           # Deployment guides
│   ├── examples/             # Example applications
│   └── guides/               # User guides
└── scripts/                   # Build and test scripts
```

## Main Components

### 1. Core Library (`crates/p2p-core`)
The heart of the system, implementing:
- Cryptographic identity management (Ed25519)
- Multi-protocol transport (TCP, QUIC, WebRTC)
- Adaptive routing with ML optimization
- Distributed storage with replication
- Security features (rate limiting, attack detection)

### 2. Adaptive Network (`crates/p2p-core/src/adaptive/`)
Revolutionary P2P implementation with:
- 19 integrated subsystems working in harmony
- Machine learning for continuous optimization
- Self-healing and adaptation to network conditions
- Performance targets: <200ms latency, 10K+ req/s

### 3. Applications (`apps/`)
User-facing applications:
- **Saorsa**: Full-featured desktop app with AI wallet management
- **Terminal Chat**: Lightweight CLI messaging
- **Network Tester**: Diagnostic and testing tools

### 4. Testing Infrastructure
- Unit tests throughout all modules
- Integration test framework
- Performance benchmarks
- Security test scenarios
- CI/CD pipelines

## Project Structure

### Core Modules
```
crates/p2p-core/src/
├── adaptive/           # Adaptive P2P network (new)
│   ├── identity.rs     # Cryptographic identities
│   ├── transport.rs    # Multi-protocol transport
│   ├── routing.rs      # Adaptive routing
│   ├── dht_integration.rs # Kademlia integration
│   ├── hyperbolic.rs   # Hyperbolic routing
│   ├── som.rs          # Self-organizing maps
│   ├── trust.rs        # EigenTrust++ reputation
│   ├── gossip.rs       # Adaptive GossipSub
│   ├── learning.rs     # ML components
│   ├── storage.rs      # Distributed storage
│   ├── security.rs     # Security hardening
│   └── monitoring.rs   # Prometheus metrics
├── bootstrap/          # Network discovery
├── dht/               # Base DHT implementation
├── identity/          # User identity management
├── mcp/               # AI integration
└── network/           # Core networking
```

## Dependencies

### Core Dependencies
- **tokio**: Async runtime (v1.35+)
- **quinn**: QUIC implementation
- **ed25519-dalek**: Cryptographic signatures
- **libp2p**: P2P networking primitives
- **serde**: Serialization framework

### Adaptive Network Dependencies
- **criterion**: Performance benchmarking
- **prometheus**: Metrics collection
- **parking_lot**: High-performance synchronization
- **flate2**: Compression support

### Application Dependencies
- **tauri**: Cross-platform app framework (v2.0)
- **clap**: Command-line parsing
- **sqlx**: Database access

## APIs

### Client API
High-level async API for applications:
```rust
// Connect to network
let client = Client::connect(ClientConfig::default()).await?;

// Store data
let hash = client.store(data).await?;

// Retrieve data
let data = client.retrieve(&hash).await?;

// Publish messages
client.publish(topic, message).await?;

// Subscribe to topics
let stream = client.subscribe(topic).await?;
```

### Network Statistics
```rust
let stats = client.get_network_stats().await?;
// Returns: connected_peers, latency, throughput, etc.
```

## Data Flow

### Storage Flow
1. Client stores data → Chunking → Hashing → Replication
2. Adaptive routing selects optimal nodes
3. Data distributed to k=20 closest nodes
4. Acknowledgments collected
5. Content hash returned to client

### Retrieval Flow
1. Client requests by hash
2. Parallel strategies: Kademlia, Hyperbolic, SOM
3. First successful response wins
4. Integrity verification
5. Q-learning cache decision
6. Data returned to client

### Message Flow
1. Publisher sends to topic
2. GossipSub mesh construction
3. Adaptive fanout based on conditions
4. Message deduplication
5. Subscribers receive messages

## Performance Characteristics

- **Lookup Latency**: <200ms (P50), <500ms (P99)
- **Throughput**: 10,000+ requests/second
- **Storage Overhead**: 20-30% for replication
- **Churn Tolerance**: 50% hourly with <15% degradation
- **Network Scale**: Tested to 1M+ nodes

## Security Model

- **Identity**: Ed25519 cryptographic identities
- **Transport**: Mandatory TLS 1.3 encryption
- **Access Control**: Friend-based permissions
- **Rate Limiting**: Per-node and global limits
- **Attack Detection**: Eclipse, Sybil, DoS protection
- **Data Integrity**: Content addressing and verification