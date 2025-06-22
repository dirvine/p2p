# P2P Foundation Technical Specification

## Overview

This specification defines a fully peer-to-peer (P2P) foundation built in Rust that provides a decentralized networking infrastructure with the following core capabilities:

- **QUIC-based connections** between devices for modern, multiplexed transport
- **IPv6-first architecture** with comprehensive IPv4 tunneling support
- **Kademlia DHT** for distributed routing and peer discovery
- **MCP server integration** at each node for AI agent communication
- **Minimal footprint** with AI-friendly modular design

## Architecture

### System Components

```
┌─────────────────────────────────────┐
│        MCP Server Layer             │  - Model Context Protocol servers
│        (AI Agent Interface)         │  - Local and remote model access
├─────────────────────────────────────┤
│     Application Protocol Layer      │  - Request/Response patterns
│        (Pub-Sub, RPC, etc.)        │  - GossipSub for messaging
├─────────────────────────────────────┤
│    Kademlia DHT (Routing Layer)     │  - Peer discovery & routing
│        (K=20, α=5)                  │  - S/Kademlia security
├─────────────────────────────────────┤
│      libp2p Core (Network)          │  - Connection management
│                                     │  - Protocol negotiation
├─────────────────────────────────────┤
│    QUIC/TCP Transport Layer         │  - Stream multiplexing
│        (Quinn-based)                │  - 0-RTT connections
├─────────────────────────────────────┤
│    IPv6/IPv4 Tunneling Layer       │  - Universal connectivity
│    (All protocols supported)        │  - Auto-selection
└─────────────────────────────────────┘
```

### Core Design Principles

1. **Serverless Architecture**: No central points of failure or control
2. **IPv6-First**: All internal communications use IPv6 endpoints
3. **AI-Native**: MCP server at each node for seamless AI integration
4. **Minimal Dependencies**: Small footprint for embedded and edge devices
5. **Modular Design**: Each layer can be used independently

## Technical Requirements

### Network Layer

#### QUIC Transport
- **Implementation**: Quinn (pure Rust QUIC implementation)
- **Features**:
  - 0-RTT connection establishment where possible
  - Multiplexed streams (100 bidirectional, 1000 unidirectional)
  - Built-in congestion control and loss recovery
  - TLS 1.3 encryption by default

#### IPv6 Primary Transport
- All nodes MUST have IPv6 addresses (native or tunneled)
- Dual-stack sockets for IPv4 compatibility
- Automatic IPv6 address configuration via SLAAC/DHCPv6

#### IPv4 Tunneling Support
Complete implementation of ALL available tunneling protocols:

1. **DS-Lite** (Dual-Stack Lite)
   - Priority: HIGH (ISP-provided, most reliable)
   - RFC 6333 compliant
   - Automatic AFTR discovery via DHCPv6

2. **Teredo**
   - Priority: HIGH (NAT traversal capable)
   - RFC 4380 compliant
   - Built-in NAT type detection
   - Fallback for symmetric NATs

3. **6to4**
   - Priority: MEDIUM
   - RFC 3056 compliant
   - Requires public IPv4 address

4. **ISATAP**
   - Priority: MEDIUM (enterprise networks)
   - RFC 5214 compliant
   - Automatic router discovery

5. **6over4**
   - Priority: LOW
   - RFC 2529 compliant
   - Multicast-capable networks only

6. **MAP-E/MAP-T**
   - Priority: HIGH (modern ISPs)
   - RFC 7597/7599 compliant
   - Stateless with port sharing

7. **464XLAT**
   - Priority: HIGH (mobile networks)
   - RFC 6877 compliant
   - CLAT implementation

8. **6rd** (IPv6 Rapid Deployment)
   - Priority: MEDIUM
   - RFC 5969 compliant

### P2P Layer

#### Kademlia DHT
- **Parameters**:
  - k = 20 (bucket size)
  - α = 5 (parallelism factor)
  - Replication factor = 20
- **Security**: S/Kademlia with disjoint paths
- **Storage**: In-memory with optional persistence
- **Record expiry**: 1 hour default, configurable

#### NAT Traversal
- **Primary**: UDP hole punching via libp2p
- **Fallback**: Relay nodes (Circuit Relay v2)
- **Discovery**: mDNS for local networks
- **UPnP/NAT-PMP**: Automatic port mapping where available

### MCP Server Layer

#### Core Functionality
- JSON-RPC 2.0 over libp2p streams
- Tool registration and discovery
- Resource management and access control
- Automatic capability advertisement via DHT

#### Required Tools
1. **Network Tools**:
   - `query_network`: DHT lookups
   - `find_peers`: Peer discovery
   - `relay_message`: Message forwarding

2. **AI Tools**:
   - `run_inference`: Local model execution
   - `forward_to_peer`: Remote inference
   - `aggregate_results`: Distributed compute

3. **Management Tools**:
   - `get_status`: Node health
   - `list_connections`: Active peers
   - `configure`: Runtime configuration

## Data Structures

### Peer Identity
```rust
pub struct PeerIdentity {
    pub peer_id: PeerId,           // libp2p peer ID
    pub public_key: PublicKey,     // Ed25519 public key
    pub ipv6_endpoints: Vec<SocketAddrV6>,
    pub capabilities: Vec<String>,  // MCP capabilities
    pub last_seen: Instant,
}
```

### DHT Record
```rust
pub struct DHTRecord {
    pub key: Key,                  // 256-bit key
    pub value: Vec<u8>,           // Arbitrary data
    pub publisher: PeerId,        
    pub signature: Signature,      // Ed25519 signature
    pub ttl: Duration,
    pub version: u64,             // Lamport timestamp
}
```

### MCP Message
```rust
pub struct MCPMessage {
    pub id: Uuid,
    pub method: String,
    pub params: serde_json::Value,
    pub peer_id: PeerId,
    pub timestamp: SystemTime,
    pub signature: Option<Signature>,
}
```

## Protocol Specifications

### P2P Discovery Protocol
1. **Bootstrap**: Connect to known bootstrap nodes
2. **DHT Integration**: Store peer info in DHT
3. **Gossip**: Advertise via GossipSub topic
4. **mDNS**: Local network discovery
5. **Relay Discovery**: Find via relay nodes

### MCP Communication Protocol
1. **Service Registration**:
   - Store service descriptor in DHT
   - Key: `mcp:service:{service_name}:{peer_id}`
   - Value: JSON service capabilities

2. **Service Discovery**:
   - Query DHT for service keys
   - Filter by capabilities
   - Sort by XOR distance

3. **Request Routing**:
   - Direct connection if possible
   - Relay through closest peer
   - Automatic failover

### Security Model

#### Transport Security
- **QUIC**: TLS 1.3 mandatory
- **TCP**: Noise protocol (XX handshake)
- **Relay**: End-to-end encryption

#### Application Security
- **Authentication**: Ed25519 signatures
- **Authorization**: Capability-based access control
- **Rate Limiting**: Per-peer quotas
- **DOS Protection**: Proof-of-work for DHT writes

## Performance Requirements

### Latency
- **Local network**: < 10ms P2P RTT
- **Internet**: < 200ms P2P RTT (same region)
- **DHT lookup**: < 500ms (99th percentile)
- **MCP request**: < 1s end-to-end

### Throughput
- **QUIC streams**: > 100 Mbps per connection
- **Total bandwidth**: Configurable limits
- **Concurrent connections**: 1000+ peers
- **MCP requests**: 100+ concurrent

### Resource Usage
- **Memory**: < 100MB base footprint
- **CPU**: < 5% idle usage
- **Storage**: Optional, configurable
- **File descriptors**: < 10k

## Implementation Phases

### Phase 1: Core Networking (Weeks 1-4)
- Basic libp2p setup with Kad DHT
- QUIC transport via Quinn
- IPv6/IPv4 dual-stack
- Basic tunneling (Teredo, 6to4)

### Phase 2: Advanced Networking (Weeks 5-8)
- Complete tunneling support
- NAT traversal optimization
- Relay infrastructure
- Performance tuning

### Phase 3: MCP Integration (Weeks 9-12)
- MCP server implementation
- Service discovery
- Tool framework
- Security layer

### Phase 4: Production Hardening (Weeks 13-16)
- Comprehensive testing
- Security audit
- Performance optimization
- Documentation

## Testing Requirements

### Unit Tests
- Coverage > 80%
- All core algorithms
- Edge cases and error paths

### Integration Tests
- Multi-node networks (5-100 nodes)
- Cross-platform testing
- Tunneling protocol verification
- MCP communication flows

### Performance Tests
- Stress testing (1000+ connections)
- Bandwidth saturation
- DHT scaling
- MCP throughput

### Security Tests
- Fuzzing all inputs
- DOS resistance
- Cryptographic verification
- Access control validation

## Dependencies

### Core Dependencies
```toml
[dependencies]
# P2P Networking
libp2p = { version = "0.54", features = ["kad", "gossipsub", "quic", "relay", "dcutr", "identify", "noise", "yamux"] }
quinn = "0.11"

# Async Runtime
tokio = { version = "1.40", features = ["full"] }

# MCP Implementation  
jsonrpc-core = "18.0"
jsonrpc-derive = "18.0"

# Cryptography
ed25519-dalek = "2.1"
x25519-dalek = "2.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
anyhow = "1.0"
tracing = "0.1"
uuid = { version = "1.10", features = ["v4", "serde"] }
```

## Configuration

### Example Configuration File
```toml
[network]
peer_id = "auto" # or specify base58 peer id
listen_addresses = [
    "/ip6/::/tcp/9000",
    "/ip4/0.0.0.0/tcp/9000", 
    "/ip6/::/udp/9001/quic-v1",
    "/ip4/0.0.0.0/udp/9001/quic-v1"
]

[bootstrap]
nodes = [
    "/dnsaddr/bootstrap.p2p.io/p2p/QmBootstrap1",
    "/dnsaddr/bootstrap.p2p.io/p2p/QmBootstrap2"
]

[dht]
protocol = "/p2p/kad/1.0.0"
replication_factor = 20
record_ttl = 3600
provider_ttl = 86400

[tunneling]
enabled = true
preferred_protocols = ["native", "dslite", "teredo"]
fallback_cascade = true
probe_interval = 300

[mcp]
enabled = true
server_address = "0.0.0.0:8545"
max_concurrent_requests = 100
request_timeout = 30
available_tools = ["query_network", "run_inference", "forward_to_peer"]

[security]
enable_encryption = true
require_authentication = true
rate_limit_per_peer = 100
total_rate_limit = 10000

[resources]
max_connections = 1000
max_memory_mb = 500
bandwidth_limit_mbps = 100
connection_timeout = 30
```

## Monitoring and Observability

### Metrics (Prometheus format)
- `p2p_peers_connected`: Number of connected peers
- `p2p_dht_records_stored`: DHT entries count
- `p2p_bandwidth_bytes`: Bandwidth usage by direction
- `p2p_tunnel_protocol`: Active tunneling protocol
- `mcp_requests_total`: Total MCP requests
- `mcp_request_duration_seconds`: Request latency histogram

### Logging
- Structured logging with tracing
- Configurable log levels per module
- Correlation IDs for request tracking
- Privacy-preserving peer ID logging

### Health Checks
- `/health`: Basic liveness check
- `/ready`: Full readiness check
- `/metrics`: Prometheus endpoint
- `/debug/peers`: Peer connection details

## Compliance and Standards

### Network Standards
- RFC 9000 (QUIC)
- RFC 8445 (ICE for NAT traversal)
- All IPv6 transition mechanism RFCs

### Security Standards
- TLS 1.3 minimum
- Ed25519 for signatures
- X25519 for key exchange
- SHA-256 for hashing

### API Standards
- JSON-RPC 2.0 for MCP
- REST principles for management API
- OpenAPI 3.0 documentation

## Future Considerations

### Planned Enhancements
1. **Blockchain Integration**: Optional consensus layer
2. **Advanced Routing**: Geo-aware routing
3. **Storage Layer**: IPFS compatibility
4. **Mobile Optimization**: Battery-efficient protocols
5. **Hardware Acceleration**: QUIC crypto offload

### Extensibility Points
- Custom transport protocols
- Additional DHT algorithms
- Plugin system for MCP tools
- Alternative crypto primitives
- Custom serialization formats

## Success Criteria

1. **Network Formation**: 100 nodes form stable network in < 30s
2. **Message Delivery**: 99.9% delivery rate under normal conditions
3. **Scalability**: Linear performance up to 10k nodes
4. **Interoperability**: Works with existing libp2p networks
5. **Developer Experience**: < 10 lines to create basic node