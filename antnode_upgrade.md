# Autonomi Node Network Stack Upgrade Plan

## Executive Summary

This document outlines a comprehensive plan to replace Autonomi's current libp2p-based networking with our P2P Foundation's QUIC-first networking stack while preserving all data storage capabilities and network semantics.

### Key Benefits
- **🚀 Performance**: QUIC transport with 0-RTT, multiplexing, and built-in encryption
- **🌐 Modern Networking**: IPv6-first design with comprehensive IPv4 fallback
- **🔧 Simplified Stack**: Fewer dependencies, more maintainable codebase
- **🤖 AI-Native**: Built-in MCP server integration for AI capabilities
- **📈 Scalability**: Optimized for high-performance P2P applications

## Current Autonomi Architecture Analysis

### Network Layer Components
- **ant-networking**: Core networking using libp2p + Kademlia DHT
- **ant-protocol**: Message definitions and network addressing
- **ant-node**: Node management with rewards and event handling
- **ant-bootstrap**: Peer discovery and network bootstrapping

### Key Data Structures

#### Storage Types
```rust
// Core data unit - content addressable
struct Chunk {
    network_address: ChunkAddress,  // Derived from content hash
    value: Bytes,                   // Max 4MB data
}

// Multiple address types for different data
enum NetworkAddress {
    PeerId(PeerId),
    ChunkAddress(ChunkAddress),
    GraphEntryAddress(GraphEntryAddress),
    ScratchpadAddress(ScratchpadAddress),
    PointerAddress(PointerAddress),
    RecordKey(RecordKey),
}
```

#### Communication Patterns
```rust
// Request/Response pattern
enum Request {
    Cmd(CmdRequest),    // Write/mutation operations
    Query(QueryRequest), // Read-only operations
}

enum Response {
    Cmd(CmdResponse),
    Query(QueryResponse),
}
```

#### Node Structure
```rust
struct RunningNode {
    network: Network,                    // libp2p networking
    node_events_channel: EventChannel,   // Event handling
    root_dir_path: PathBuf,             // Local storage
    rewards_address: EthereumAddress,    // Node rewards
}
```

## Integration Plan

### Phase 1: Data Type Compatibility Layer

#### 1.1 Implement Autonomi Data Structures
Create compatibility types in our P2P Foundation:

```rust
// p2p-foundation/src/autonomi/mod.rs
pub mod storage;
pub mod protocol;
pub mod node;

// Chunk implementation with our networking
impl Chunk {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        if data.len() > MAX_CHUNK_SIZE_4MB {
            return Err(ChunkError::TooLarge);
        }
        
        let address = ChunkAddress::from_content(&data);
        Ok(Self { address, data })
    }
    
    pub fn store_in_dht(&self, node: &P2PNode) -> Result<()> {
        let key = self.address.to_dht_key();
        node.dht_put(key, self.data.clone()).await
    }
}
```

#### 1.2 Protocol Bridge Module
Map Autonomi's messaging to our transport:

```rust
// p2p-foundation/src/autonomi/protocol.rs
pub struct AutonomiBridge {
    p2p_node: Arc<P2PNode>,
    message_router: MessageRouter,
}

impl AutonomiBridge {
    pub async fn handle_request(&self, req: Request) -> Result<Response> {
        match req {
            Request::Cmd(cmd) => self.handle_command(cmd).await,
            Request::Query(query) => self.handle_query(query).await,
        }
    }
    
    async fn handle_command(&self, cmd: CmdRequest) -> Result<CmdResponse> {
        // Map to our DHT operations
        // Use our QUIC transport for peer communication
    }
}
```

### Phase 2: Network Stack Replacement

#### 2.1 Replace libp2p with P2P Foundation
Create drop-in replacement for Autonomi's Network:

```rust
// p2p-foundation/src/autonomi/network.rs
pub struct AutonomiNetwork {
    p2p_node: Arc<P2PNode>,
    peer_manager: PeerManager,
    kademlia_dht: KademliaDHT,
}

impl AutonomiNetwork {
    // Maintain API compatibility
    pub async fn send_request(&self, peer: PeerId, req: Request) -> Result<Response> {
        let serialized = serialize_request(req)?;
        let response_data = self.p2p_node.send_message(&peer, "autonomi/1.0", serialized).await?;
        deserialize_response(response_data)
    }
    
    pub async fn get_closest_peers(&self, target: &NetworkAddress) -> Vec<PeerId> {
        let key = target.to_dht_key();
        self.p2p_node.dht_find_peers(&key).await
    }
}
```

#### 2.2 Node Integration
Adapt RunningNode to use our P2PNode:

```rust
// p2p-foundation/src/autonomi/node.rs
pub struct AutonomiNode {
    p2p_node: Arc<P2PNode>,
    autonomi_network: AutonomiNetwork,
    local_storage: LocalStorage,
    rewards_address: EthereumAddress,
    event_dispatcher: EventDispatcher,
}

impl AutonomiNode {
    pub async fn new(config: AutonomieNodeConfig) -> Result<Self> {
        // Create P2P node with QUIC transport
        let p2p_config = NodeConfig {
            listen_addrs: vec!["/ip6/::/udp/0/quic".to_string()],
            enable_mcp_server: true,
            ..Default::default()
        };
        
        let p2p_node = P2PNode::new(p2p_config).await?;
        let autonomi_network = AutonomiNetwork::new(Arc::clone(&p2p_node))?;
        
        Ok(Self {
            p2p_node,
            autonomi_network,
            local_storage: LocalStorage::new(config.root_dir)?,
            rewards_address: config.rewards_address,
            event_dispatcher: EventDispatcher::new(),
        })
    }
}
```

### Phase 3: Storage Layer Integration

#### 3.1 DHT Storage Implementation
Use our DHT for chunk storage:

```rust
// p2p-foundation/src/autonomi/storage.rs
pub struct AutonomiStorage {
    dht: Arc<DHT>,
    local_store: RecordStore,
    replication_manager: ReplicationManager,
}

impl AutonomiStorage {
    pub async fn store_chunk(&self, chunk: Chunk) -> Result<()> {
        // Store locally first
        self.local_store.put(chunk.address.clone(), chunk.data.clone()).await?;
        
        // Replicate to DHT
        let key = chunk.address.to_dht_key();
        self.dht.put(key, chunk.data).await?;
        
        // Trigger replication to close group
        self.replication_manager.replicate_to_close_group(&chunk.address).await?;
        
        Ok(())
    }
    
    pub async fn retrieve_chunk(&self, address: &ChunkAddress) -> Result<Option<Chunk>> {
        // Try local first
        if let Some(data) = self.local_store.get(address).await? {
            return Ok(Some(Chunk::new_with_address(address.clone(), data)));
        }
        
        // Query DHT
        let key = address.to_dht_key();
        if let Some(data) = self.dht.get(&key).await? {
            return Ok(Some(Chunk::new_with_address(address.clone(), data)));
        }
        
        Ok(None)
    }
}
```

#### 3.2 Network Commands Implementation
Map Autonomi's command system:

```rust
// p2p-foundation/src/autonomi/commands.rs
pub enum AutonomiCommand {
    StoreRecord { key: RecordKey, value: Vec<u8> },
    GetRecord { key: RecordKey },
    AddPeer { peer_id: PeerId, addresses: Vec<Multiaddr> },
    GetClosestPeers { target: NetworkAddress },
    TriggerReplication { address: NetworkAddress },
}

impl AutonomiNode {
    pub async fn execute_command(&self, cmd: AutonomiCommand) -> Result<CommandResponse> {
        match cmd {
            AutonomiCommand::StoreRecord { key, value } => {
                self.p2p_node.dht_put(key.into(), value).await?;
                Ok(CommandResponse::Success)
            }
            AutonomiCommand::GetRecord { key } => {
                let value = self.p2p_node.dht_get(&key.into()).await?;
                Ok(CommandResponse::Record(value))
            }
            // ... other commands
        }
    }
}
```

### Phase 4: Feature Parity & Advanced Integration

#### 4.1 Rewards System Integration
```rust
// p2p-foundation/src/autonomi/rewards.rs
pub struct RewardsManager {
    ethereum_address: EthereumAddress,
    metrics_collector: MetricsCollector,
    payment_processor: PaymentProcessor,
}

impl RewardsManager {
    pub async fn track_storage_operation(&self, operation: StorageOperation) {
        self.metrics_collector.record_operation(operation).await;
        // Integrate with existing Autonomi rewards logic
    }
}
```

#### 4.2 Metrics and Monitoring
```rust
// p2p-foundation/src/autonomi/metrics.rs
pub struct AutonomiMetrics {
    network_metrics: NetworkMetrics,
    storage_metrics: StorageMetrics,
    openmetrics_exporter: OpenMetricsExporter,
}

impl AutonomiMetrics {
    pub fn export_openmetrics(&self) -> String {
        // Export in OpenMetrics format for Autonomi compatibility
        let mut buffer = String::new();
        buffer.push_str(&self.network_metrics.to_openmetrics());
        buffer.push_str(&self.storage_metrics.to_openmetrics());
        buffer
    }
}
```

#### 4.3 Graph Storage and Pointers
```rust
// p2p-foundation/src/autonomi/graph.rs
pub struct GraphStorage {
    chunk_storage: AutonomiStorage,
    graph_resolver: GraphResolver,
}

impl GraphStorage {
    pub async fn store_graph_entry(&self, entry: GraphEntry) -> Result<GraphEntryAddress> {
        let serialized = serialize_graph_entry(&entry)?;
        let chunk = Chunk::new(serialized)?;
        self.chunk_storage.store_chunk(chunk).await?;
        Ok(GraphEntryAddress::from_chunk_address(&chunk.address))
    }
}
```

## Implementation Strategy

### Modular Approach
1. **Parallel Development**: Build compatibility layer alongside existing system
2. **Gradual Migration**: Replace components one at a time
3. **Fallback Support**: Keep libp2p as backup during transition
4. **Testing Integration**: Comprehensive testing at each phase

### Directory Structure
```
p2p-foundation/
├── src/
│   ├── autonomi/           # New Autonomi compatibility layer
│   │   ├── mod.rs
│   │   ├── storage.rs      # Chunk storage with our DHT
│   │   ├── protocol.rs     # Message protocol bridge
│   │   ├── network.rs      # Network API compatibility
│   │   ├── node.rs         # Node management
│   │   ├── commands.rs     # Command mapping
│   │   ├── rewards.rs      # Rewards integration
│   │   ├── metrics.rs      # Metrics compatibility
│   │   └── graph.rs        # Graph storage support
│   ├── network.rs          # Our existing P2P network
│   ├── transport.rs        # Our QUIC/TCP transport
│   └── dht.rs             # Our DHT implementation
└── examples/
    └── autonomi_node.rs    # Example Autonomi node
```

## Migration Path

### Step 1: Compatibility Layer (Weeks 1-2)
- [ ] Implement basic Autonomi data types
- [ ] Create protocol message bridge
- [ ] Basic chunk storage/retrieval

### Step 2: Network Replacement (Weeks 3-4)
- [ ] Replace libp2p with our QUIC transport
- [ ] Implement Kademlia DHT compatibility
- [ ] Peer management and routing

### Step 3: Storage Integration (Weeks 5-6)
- [ ] Full chunk storage system
- [ ] Replication and consensus
- [ ] Graph storage and pointers

### Step 4: Feature Completion (Weeks 7-8)
- [ ] Rewards system integration
- [ ] Metrics and monitoring
- [ ] Performance optimization
- [ ] Comprehensive testing

## Benefits Analysis

### Performance Improvements
| Feature | libp2p (Current) | P2P Foundation (Proposed) |
|---------|------------------|---------------------------|
| Transport | TCP/WebRTC | QUIC (0-RTT, multiplexing) |
| Encryption | Manual setup | Built-in TLS 1.3 |
| IPv6 Support | Limited | Native IPv6-first |
| Connection Setup | ~100ms | ~0ms (0-RTT) |
| Multiplexing | Limited | Full QUIC streams |

### Maintainability Benefits
- **Fewer Dependencies**: Reduce libp2p dependency tree
- **Simpler Debugging**: Single transport protocol
- **Better Testing**: Integrated test suite
- **Modern Codebase**: Clean, well-documented APIs

### AI Integration
- **MCP Server**: Built-in AI capabilities
- **Tool Integration**: Native AI tool support
- **Agent Communication**: Designed for AI agents

## Risk Assessment & Mitigation

### Technical Risks
| Risk | Impact | Probability | Mitigation |
|------|---------|-------------|------------|
| Performance regression | High | Low | Extensive benchmarking |
| Compatibility issues | High | Medium | Phased rollout with fallback |
| Network fragmentation | Medium | Low | Maintain protocol compatibility |
| Migration complexity | Medium | Medium | Modular approach |

### Mitigation Strategies
1. **Parallel Operation**: Run both systems during transition
2. **Extensive Testing**: Comprehensive test suite
3. **Gradual Rollout**: Phase-by-phase implementation
4. **Community Feedback**: Regular feedback integration
5. **Fallback Option**: Keep libp2p as backup

## Success Metrics

### Technical Metrics
- [ ] **Performance**: 50% reduction in connection setup time
- [ ] **Bandwidth**: 20% reduction in network overhead
- [ ] **Latency**: 30% improvement in message delivery
- [ ] **Reliability**: 99.9% message delivery success

### Integration Metrics
- [ ] **API Compatibility**: 100% existing API support
- [ ] **Data Integrity**: Zero data loss during migration
- [ ] **Feature Parity**: All current features preserved
- [ ] **Test Coverage**: 95% code coverage

## Timeline Summary

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| Phase 1 | 2 weeks | Data compatibility layer |
| Phase 2 | 2 weeks | Network stack replacement |
| Phase 3 | 2 weeks | Storage integration |
| Phase 4 | 2 weeks | Feature parity & testing |
| **Total** | **8 weeks** | **Production-ready Autonomi node** |

## Conclusion

This upgrade plan provides a comprehensive roadmap for replacing Autonomi's libp2p networking with our P2P Foundation's QUIC-first stack. The modular approach ensures minimal risk while delivering significant performance and maintainability improvements.

The result will be a more efficient, modern, and AI-native networking stack that preserves all of Autonomi's data storage capabilities while providing a foundation for future enhancements.

---

*This document serves as the technical specification for the Autonomi network stack upgrade. For implementation details and progress tracking, see the project repository.*