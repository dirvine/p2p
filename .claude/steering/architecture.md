# P2P Foundation - Technical Architecture

## System Architecture

### Layered Architecture

The P2P Foundation follows a strict layered architecture where each layer only depends on layers below it:

```
┌─────────────────────────────────────┐
│       Application Layer             │
│   - Saorsa GUI (Tauri)             │
│   - Terminal Chat                   │
│   - Network Tester                  │
│   - CLI Tools                       │
├─────────────────────────────────────┤
│       Service Layer                 │
│   - MCP Server & Tools              │
│   - Node Management API             │
│   - Health Monitoring               │
│   - Metrics Collection              │
├─────────────────────────────────────┤
│       Protocol Layer                │
│   - DHT (Kademlia)                 │
│   - Gossipsub                      │
│   - Trust Network                  │
│   - Consensus                      │
├─────────────────────────────────────┤
│       Transport Layer               │
│   - ant-quic v0.6.1                │
│   - NAT Traversal (built-in)       │
│   - Post-Quantum Crypto            │
│   - Connection Pooling             │
├─────────────────────────────────────┤
│       Foundation Layer              │
│   - Identity Management            │
│   - Cryptography (PQC)             │
│   - Storage Backend                │
│   - Configuration                  │
└─────────────────────────────────────┘
```

## Core Components

### 1. Identity System (`p2p-identity`)

```rust
pub struct NodeIdentity {
    pub keypair: Keypair,           // Ed25519 or ML-DSA
    pub peer_id: PeerId,            // Derived from public key
    pub three_word_address: String,  // Human-readable address
    pub did: String,                // Decentralized identifier
}
```

**Responsibilities:**
- Key generation and management
- Post-quantum cryptographic operations
- Three-word address generation
- DID (Decentralized Identifier) support
- Passkey/WebAuthn integration

**Key Files:**
- `crates/p2p-identity/src/identity.rs`
- `crates/p2p-identity/src/three_word.rs`
- `crates/p2p-identity/src/pqc.rs`

### 2. Transport System (`p2p-transport`)

```rust
pub struct P2PNetworkNode {
    pub node: Arc<QuicP2PNode>,     // ant-quic node
    pub local_addr: SocketAddr,     // Local binding
    pub peers: Arc<RwLock<Vec<PeerId>>>, // Connected peers
}
```

**Responsibilities:**
- QUIC connection management via ant-quic
- NAT traversal and hole punching
- Connection pooling and multiplexing
- Protocol negotiation
- Stream management

**Key Files:**
- `crates/p2p-core/src/transport/ant_quic_adapter.rs`
- `crates/p2p-core/src/transport.rs` (deprecated traits)

### 3. DHT System (`p2p-dht`)

```rust
pub struct DHT {
    routing_table: KademliaRoutingTable,
    storage: Arc<dyn Storage>,
    replication_factor: u8,  // K=8
    git_addressing: GitContentAddressing,
}
```

**Responsibilities:**
- Kademlia routing with K=8 replication
- Content addressing with BLAKE3
- Git-like version control semantics
- Data replication and repair
- Adaptive caching with Q-Learning

**Key Operations:**
- `put(key, value)` - Store with replication
- `get(key)` - Retrieve with routing
- `find_node(id)` - Locate peers
- `find_value(key)` - Content discovery

### 4. Gossipsub Protocol (`p2p-gossip`)

```rust
pub struct Gossipsub {
    mesh: HashMap<Topic, HashSet<PeerId>>,
    fanout: HashMap<Topic, HashSet<PeerId>>,
    seen_cache: LruCache<MessageId, ()>,
    config: GossipsubConfig,
}
```

**Responsibilities:**
- Topic-based pub/sub messaging
- Mesh network maintenance
- Message deduplication
- Adaptive fanout optimization
- Flood control

### 5. MCP Integration (`p2p-core/src/mcp`)

```rust
pub struct MCPServer {
    tools: HashMap<String, Box<dyn MCPTool>>,
    registry: ServiceRegistry,
    auth: MCPAuthProvider,
}
```

**Responsibilities:**
- Model Context Protocol server
- Tool registration and discovery
- Request routing and load balancing
- Authentication and authorization
- AI agent coordination

### 6. Adaptive Learning (`p2p-learning`)

```rust
pub struct AdaptiveSystem {
    q_learning_cache: QLearningCache,
    thompson_routing: ThompsonSampling,
    som_topology: SelfOrganizingMap,
    lstm_predictor: ChurnPredictor,
}
```

**Components:**
- **Q-Learning**: Cache optimization
- **Thompson Sampling**: Route selection
- **Self-Organizing Maps**: Network topology
- **LSTM**: Churn prediction

## Communication Patterns

### 1. Request-Response Pattern

```
Client → Request → P2PNode → DHT Lookup → Target Node
                                    ↓
Client ← Response ← P2PNode ← Response ← Target Node
```

Used for:
- DHT operations (get/put)
- Direct peer messaging
- RPC calls

### 2. Publish-Subscribe Pattern

```
Publisher → Topic → Gossipsub → Mesh Network → Subscribers
                         ↓
                    Fanout Peers
```

Used for:
- Broadcast messaging
- Event notifications
- Network announcements

### 3. Stream Pattern

```
Peer A ←→ QUIC Stream ←→ Peer B
        Bidirectional
        Multiplexed
        Ordered/Unordered
```

Used for:
- File transfers
- Real-time communication
- Continuous data flows

## Data Models

### 1. Network Message

```rust
#[derive(Serialize, Deserialize)]
pub struct NetworkMessage {
    pub id: MessageId,
    pub sender: PeerId,
    pub protocol: ProtocolId,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub signature: Signature,
}
```

### 2. DHT Record

```rust
#[derive(Serialize, Deserialize)]
pub struct DHTRecord {
    pub key: Key,
    pub value: Vec<u8>,
    pub publisher: PeerId,
    pub timestamp: u64,
    pub expiry: Option<u64>,
    pub version: u64,  // Git-like versioning
}
```

### 3. Peer Info

```rust
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub protocols: Vec<ProtocolId>,
    pub metadata: HashMap<String, String>,
    pub reputation: f64,
    pub last_seen: Instant,
}
```

## Security Architecture

### 1. Cryptographic Layer

```
Application Data
      ↓
Post-Quantum Encryption (ML-KEM)
      ↓
Digital Signatures (ML-DSA)
      ↓
TLS 1.3 Transport Security
      ↓
Network Transport
```

### 2. Authentication Flow

```
1. Peer connects with three-word address
2. DHT lookup resolves to PeerId
3. TLS handshake with certificate verification
4. Post-quantum key exchange (ML-KEM)
5. Mutual authentication with ML-DSA signatures
6. Session establishment with forward secrecy
```

### 3. Authorization Model

- **Capability-based**: Tokens grant specific permissions
- **Decentralized**: No central authority
- **Cryptographic proofs**: All actions verifiable
- **Revocable**: Permissions can be withdrawn

## Scalability Design

### 1. Horizontal Scaling

- **Sharding**: DHT space partitioned by key prefix
- **Load balancing**: Automatic request distribution
- **Connection pooling**: Reuse existing connections
- **Parallel processing**: Concurrent request handling

### 2. Caching Strategy

```
L1 Cache: In-memory LRU (Hot data)
    ↓
L2 Cache: Local disk storage (Warm data)
    ↓
L3 Cache: Nearby peers (Distributed cache)
    ↓
DHT Network: Full data store
```

### 3. Network Optimization

- **Adaptive routing**: ML-based path selection
- **Connection reuse**: Multiplexing over QUIC
- **Compression**: zstd for large payloads
- **Batching**: Combine multiple small requests

## Error Handling

### 1. Error Hierarchy

```rust
#[derive(Error, Debug)]
pub enum P2PError {
    #[error("Network error: {0}")]
    Network(NetworkError),
    
    #[error("DHT error: {0}")]
    DHT(DHTError),
    
    #[error("Transport error: {0}")]
    Transport(TransportError),
    
    #[error("Identity error: {0}")]
    Identity(IdentityError),
}
```

### 2. Recovery Strategies

- **Automatic retry**: With exponential backoff
- **Fallback mechanisms**: Alternative transports
- **Circuit breakers**: Prevent cascade failures
- **Graceful degradation**: Partial functionality

### 3. Monitoring & Alerting

```rust
pub struct HealthCheck {
    pub network_status: NetworkHealth,
    pub dht_status: DHTHealth,
    pub peer_count: usize,
    pub bandwidth_usage: BandwidthMetrics,
    pub error_rate: f64,
}
```

## Performance Optimization

### 1. Zero-Copy Operations

- Use `Bytes` for network buffers
- Avoid unnecessary cloning
- Reference counting with `Arc`
- Memory-mapped files for large data

### 2. Async/Await Patterns

```rust
// Concurrent operations
let (result1, result2, result3) = tokio::join!(
    dht.get(key1),
    dht.get(key2),
    dht.get(key3)
);

// Selective waiting
tokio::select! {
    res = operation1() => handle_result1(res),
    res = operation2() => handle_result2(res),
    _ = timeout(Duration::from_secs(30)) => handle_timeout(),
}
```

### 3. Resource Pooling

- Connection pools per peer
- Thread pools for CPU-intensive tasks
- Buffer pools for network I/O
- Database connection pools

## Integration Points

### 1. External Systems

- **IPFS**: Content addressing compatibility
- **libp2p**: Protocol interoperability
- **Matrix**: Federated messaging bridge
- **ActivityPub**: Social network federation

### 2. Storage Backends

- **RocksDB**: Default local storage
- **Sled**: Embedded database option
- **PostgreSQL**: Centralized deployments
- **S3**: Cloud storage integration

### 3. Monitoring Systems

- **Prometheus**: Metrics export
- **Grafana**: Visualization
- **OpenTelemetry**: Distributed tracing
- **ELK Stack**: Log aggregation

## Deployment Architecture

### 1. Container Architecture

```dockerfile
FROM rust:1.75 as builder
# Multi-stage build for minimal image

FROM debian:bookworm-slim
# Runtime with minimal dependencies
```

### 2. Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: p2p-node
spec:
  replicas: 3
  serviceName: p2p-network
  # Persistent storage for DHT data
```

### 3. Edge Deployment

- Lightweight builds for IoT devices
- WebAssembly for browser deployment
- Mobile SDKs for iOS/Android
- Embedded systems support

## Migration Strategy

### From Legacy to Native ant-quic

1. **Phase 1**: Deprecate Transport/Connection traits ✅
2. **Phase 2**: Native ant-quic integration ✅
3. **Phase 3**: Remove legacy abstractions ✅
4. **Phase 4**: Optimize for ant-quic features (ongoing)

### Database Migration

- Version-tagged schemas
- Backward-compatible changes
- Migration scripts in `migrations/`
- Rollback capabilities

## Future Architecture

### Planned Enhancements

1. **Onion Routing**: Enhanced privacy layer
2. **Blockchain Integration**: Consensus mechanism
3. **WebRTC Support**: Browser-native P2P
4. **Hardware Security Modules**: Key protection
5. **Distributed Computing**: Task distribution