# P2P Foundation Architecture Overview

## System Architecture

### Layer Architecture

```
┌─────────────────────────────────────────────┐
│         Application Layer (Apps)             │
│  Saorsa, Terminal Chat, Network Tester      │
└─────────────────────────────────────────────┘
                      ↕
┌─────────────────────────────────────────────┐
│           API Layer (p2p-client)            │
│    High-level APIs, SDK, FFI Bindings       │
└─────────────────────────────────────────────┘
                      ↕
┌─────────────────────────────────────────────┐
│         Service Layer (p2p-core)            │
│   Node Management, MCP, Orchestration       │
└─────────────────────────────────────────────┘
                      ↕
┌─────────────────────────────────────────────┐
│          Protocol Layer                      │
│   DHT, Gossipsub, Trust, Learning          │
└─────────────────────────────────────────────┘
                      ↕
┌─────────────────────────────────────────────┐
│         Transport Layer                      │
│    QUIC, TCP, IPv6/IPv4 Tunneling          │
└─────────────────────────────────────────────┘
                      ↕
┌─────────────────────────────────────────────┐
│         Foundation Layer                     │
│   Identity, Crypto, Storage, Config         │
└─────────────────────────────────────────────┘
```

## Core Components

### 1. Network Architecture

#### Transport Layer
- **Multi-Protocol Support**
  - QUIC (primary): Low latency, multiplexing
  - TCP (fallback): Universal compatibility
  - Automatic protocol selection

#### IPv6/IPv4 Handling
- **IPv6-First Design**
  - Native IPv6 support
  - Automatic IPv4 tunneling
- **Tunneling Protocols**
  - 6to4: IPv6 over IPv4
  - Teredo: NAT traversal
  - DS-Lite: Dual-stack lite
  - Automatic tunnel selection

#### Connection Management
- **Connection Pooling**
  - Persistent connections
  - Connection reuse
  - Resource optimization
- **Load Balancing**
  - Round-robin distribution
  - Weighted selection
  - Health-based routing

### 2. DHT Architecture

#### Kademlia Implementation
```
Node ID Space (256-bit)
├── Bucket 0: Distance 2^0
├── Bucket 1: Distance 2^1
├── ...
└── Bucket 255: Distance 2^255

Replication: K=8 replicas
Lookup: α=3 parallel queries
```

#### Content Addressing
- **BLAKE3 Hashing**: Fast, secure content addressing
- **Git-Like Semantics**: Version control for all data
- **Automatic Replication**: K=8 replicas for fault tolerance

#### Storage Layers
1. **Memory Cache**: Hot data, fast access
2. **Persistent Storage**: Encrypted at rest
3. **Network Storage**: Distributed replicas

### 3. Identity System

#### Cryptographic Foundation
```
Identity
├── ML-KEM-768 (Kyber)
│   ├── Public Key
│   └── Private Key
├── ML-DSA-65 (Dilithium)
│   ├── Signing Key
│   └── Verification Key
└── Three-Word Address
    └── Deterministic Mapping
```

#### Three-Word Addressing
- **Format**: "word1-word2-word3"
- **Uniqueness**: 50+ million combinations
- **Mapping**: Deterministic from public key
- **Collision Resistance**: Birthday paradox considered

#### Authentication
- **Passkey Support**: WebAuthn integration
- **FROST**: Threshold signatures (t-of-n)
- **Session Management**: Ephemeral keys

### 4. MCP Integration

#### Architecture
```
P2P Node
├── MCP Server
│   ├── Tool Registry
│   ├── Service Discovery
│   └── Request Handler
├── Tool Providers
│   ├── Storage Tools
│   ├── Network Tools
│   └── Custom Tools
└── AI Interface
    ├── Context Management
    └── Response Generation
```

#### Tool Ecosystem
- **Built-in Tools**: Storage, messaging, discovery
- **Custom Tools**: User-defined extensions
- **Tool Discovery**: Automatic registration
- **Permission System**: Capability-based access

### 5. Adaptive Learning

#### Q-Learning Cache
```
State: (content_type, access_pattern, time)
Action: (cache, evict, priority)
Reward: hit_rate * value - miss_cost
Update: Q(s,a) ← Q(s,a) + α[r + γ max Q(s',a') - Q(s,a)]
```

#### Thompson Sampling
- **Route Selection**: Probabilistic exploration
- **Peer Selection**: Reputation-weighted
- **Dynamic Adaptation**: Online learning

#### Self-Organizing Maps
- **Network Topology**: 2D/3D visualization
- **Peer Clustering**: Similarity-based groups
- **Anomaly Detection**: Outlier identification

### 6. Storage Architecture

#### Eviction Strategies
1. **LRU (Least Recently Used)**: Time-based
2. **LFU (Least Frequently Used)**: Access-based
3. **Q-Learning**: Adaptive optimization
4. **Thompson Sampling**: Exploration/exploitation
5. **Hybrid**: Combined strategies

#### Data Persistence
```
Storage Pipeline
├── Input: Raw Data
├── Encryption: ML-KEM
├── Compression: Zstd
├── Chunking: Content-defined
├── Storage: Key-Value Store
└── Replication: K=8 copies
```

#### Repair Mechanism
- **Periodic Verification**: Merkle tree validation
- **Automatic Repair**: Missing chunk recovery
- **Proactive Replication**: Maintain K replicas

## Security Architecture

### Threat Model
- **Network Adversary**: Can observe/modify traffic
- **Storage Adversary**: Can access disk
- **Quantum Adversary**: Has quantum computer

### Defense Mechanisms

#### Cryptographic Protection
- **Quantum-Resistant**: ML-KEM, ML-DSA
- **Forward Secrecy**: Ephemeral keys
- **Perfect Forward Secrecy**: Key rotation

#### Network Security
- **TLS 1.3**: Transport encryption
- **Noise Protocol**: Alternative encryption
- **Rate Limiting**: DoS protection

#### Storage Security
- **Encryption at Rest**: All data encrypted
- **Key Management**: Hardware security module
- **Access Control**: Capability-based

## Performance Architecture

### Optimization Strategies

#### Network Optimization
- **Connection Pooling**: Reuse connections
- **Multiplexing**: QUIC streams
- **Compression**: Protocol buffers

#### Storage Optimization
- **Caching**: Multi-level cache
- **Indexing**: B-tree/LSM-tree
- **Compression**: Zstd/LZ4

#### Compute Optimization
- **Async/Await**: Non-blocking I/O
- **Thread Pooling**: Work stealing
- **SIMD**: Vectorized operations

### Scalability Design

#### Horizontal Scaling
- **Sharding**: Content-based sharding
- **Load Distribution**: Consistent hashing
- **Auto-scaling**: Dynamic node addition

#### Vertical Scaling
- **Resource Management**: CPU/memory limits
- **Priority Queues**: QoS support
- **Batch Processing**: Bulk operations

## Deployment Architecture

### Container Architecture
```
Docker/Kubernetes
├── P2P Node Container
│   ├── Runtime: Rust binary
│   ├── Storage: Volume mount
│   └── Network: Host/Bridge
├── Sidecar Containers
│   ├── Monitoring: Prometheus
│   └── Logging: Fluentd
└── Init Containers
    └── Configuration: Setup
```

### Cloud Native
- **Kubernetes**: Orchestration
- **Helm Charts**: Package management
- **Service Mesh**: Istio/Linkerd
- **Observability**: OpenTelemetry

### Edge Deployment
- **Embedded Systems**: ARM support
- **IoT Devices**: Lightweight mode
- **Mobile**: iOS/Android via Tauri

## Monitoring & Observability

### Metrics
- **Network**: Latency, throughput, connections
- **Storage**: Cache hit rate, disk usage
- **Application**: Request rate, error rate

### Logging
- **Structured Logging**: JSON format
- **Log Levels**: TRACE, DEBUG, INFO, WARN, ERROR
- **Correlation IDs**: Request tracing

### Tracing
- **OpenTelemetry**: Distributed tracing
- **Span Context**: Request lifecycle
- **Performance Analysis**: Bottleneck identification

Last Updated: 2025-08-06