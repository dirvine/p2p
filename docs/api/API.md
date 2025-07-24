# Saorsa Core - API Documentation

> 🕊️ **Saorsa**: A next-generation P2P foundation library with quantum-resistant cryptography, human-friendly addressing, and AI integration.

[![Crates.io](https://img.shields.io/crates/v/saorsa-core)](https://crates.io/crates/saorsa-core)
[![Version](https://img.shields.io/badge/version-0.2.7-blue)](https://github.com/dirvine/p2p)
[![License](https://img.shields.io/badge/license-AGPL--3.0%20OR%20Commercial-green)](./LICENSING.md)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org)

## 🌟 Features & Capabilities

### Core Networking
- **🔗 P2P Networking**: Self-organizing peer-to-peer network with automatic NAT traversal
- **📡 DHT (Distributed Hash Table)**: Kademlia-based with S/Kademlia security extensions
- **🚀 Multi-Transport**: QUIC and TCP support with automatic transport selection
- **🌐 IPv6-First**: Native IPv6 with comprehensive tunneling protocols (6to4, Teredo, DS-Lite, etc.)
- **🎯 Three-Word Addresses**: Human-readable network addressing (e.g., "forest.lightning.compass")

### Quantum-Resistant Security
- **🔐 Post-Quantum Cryptography**: ML-KEM key encapsulation and ML-DSA signatures
- **🔀 Hybrid Cryptography**: Combines classical (Ed25519) and post-quantum algorithms
- **⚖️ Threshold Cryptography**: FROST multi-party signatures and distributed key generation
- **🛡️ Future-Proof**: Upgradeable cryptographic protocols for quantum safety

### Advanced Features
- **🤖 MCP Integration**: Native Model Context Protocol support for AI applications
- **👥 Collaboration**: Real-time chat, discussions, projects with threshold approval workflows
- **📁 Secure Storage**: Encrypted DHT storage with version control and access controls
- **🏢 Organizations**: Hierarchical structure (Organizations → Departments → Teams → Projects)

### Production Ready
- **⚡ Performance**: Optimized for low latency and high throughput
- **🔍 Observability**: Comprehensive metrics, tracing, and health monitoring
- **📈 Scalability**: Handles thousands of concurrent connections
- **🛠️ Developer Experience**: Rich CLI tools, extensive documentation, and examples

---

## 📖 Table of Contents

1. [Quick Start](#-quick-start)
2. [Core Architecture](#-core-architecture)
3. [Network & Transport](#-network--transport-api)
4. [Three-Word Addresses](#-three-word-addresses)
5. [DHT & Storage](#-dht--storage)
6. [Quantum Cryptography](#-quantum-cryptography)
7. [Threshold Signatures](#-threshold-signatures)
8. [IPv6 Tunneling](#-ipv6-tunneling)
9. [MCP Integration](#-mcp-integration)
10. [Identity & Organizations](#-identity--organizations)
11. [Collaboration Systems](#-collaboration-systems)
12. [Configuration](#-configuration)
13. [Error Handling](#-error-handling)
14. [Examples](#-examples)

---

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
saorsa-core = "0.2.7"
tokio = { version = "1.0", features = ["full"] }
```

### Basic P2P Node

```rust
use saorsa_core::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new P2P node
    let mut node = P2PNode::builder()
        .listen_on("/ip6/::1/udp/0/quic")
        .with_dht(DHTConfig::default())
        .with_mcp_server()
        .build()
        .await?;

    // Start the node
    node.start().await?;
    
    // Get the three-word address
    let encoder = WordEncoder::new();
    let addresses = node.listen_addrs().await?;
    let words = encoder.encode_multiaddr(&addresses[0])?;
    
    println!("🕊️ Node running at: {}", words);
    println!("📡 Peer ID: {}", node.peer_id());
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    node.shutdown().await?;
    
    Ok(())
}
```

---

## 🏗️ Core Architecture

### P2PNode - Main Entry Point

The `P2PNode` is the primary interface for the P2P foundation library.

```rust
pub struct P2PNode {
    // Internal fields are private
}

impl P2PNode {
    /// Create a new node with default configuration
    pub async fn new(config: NodeConfig) -> Result<Self>;
    
    /// Get a builder for configuring the node
    pub fn builder() -> NodeBuilder;
    
    /// Start the P2P node and all services
    pub async fn start(&mut self) -> Result<()>;
    
    /// Gracefully shutdown the node
    pub async fn shutdown(&mut self) -> Result<()>;
    
    /// Get the node's peer ID
    pub fn peer_id(&self) -> &PeerId;
    
    /// Get current listening addresses
    pub async fn listen_addrs(&self) -> Result<Vec<Multiaddr>>;
    
    /// Get node statistics
    pub async fn stats(&self) -> NodeStats;
    
    /// Register event handler
    pub fn on_event<F>(&mut self, handler: F) 
    where 
        F: Fn(P2PEvent) + Send + Sync + 'static;
}
```

### NodeBuilder - Configuration Builder

```rust
pub struct NodeBuilder {
    // Internal configuration
}

impl NodeBuilder {
    /// Add a listening address
    pub fn listen_on(self, addr: &str) -> Self;
    
    /// Enable MCP server
    pub fn with_mcp_server(self) -> Self;
    
    /// Configure DHT settings
    pub fn with_dht(self, config: DHTConfig) -> Self;
    
    /// Configure security settings
    pub fn with_security(self, config: SecurityConfig) -> Self;
    
    /// Configure IPv6 tunneling
    pub fn with_tunneling(self, config: TunnelConfig) -> Self;
    
    /// Set node identity
    pub fn with_identity(self, identity: EnhancedIdentity) -> Self;
    
    /// Build the configured node
    pub async fn build(self) -> Result<P2PNode>;
}
```

### Core Events

```rust
#[derive(Debug, Clone)]
pub enum P2PEvent {
    /// New peer connected
    PeerConnected {
        peer_id: PeerId,
        multiaddr: Multiaddr,
        protocols: Vec<String>,
    },
    
    /// Peer disconnected
    PeerDisconnected {
        peer_id: PeerId,
        reason: DisconnectReason,
    },
    
    /// DHT record updated
    DHTRecord {
        key: Key,
        operation: DHTOperation,
        peer_id: PeerId,
    },
    
    /// MCP message received
    MCPMessage {
        message: MCPMessage,
        client_id: String,
    },
    
    /// Security event
    SecurityEvent {
        event_type: SecurityEventType,
        peer_id: Option<PeerId>,
        details: SecurityDetails,
    },
    
    /// Tunnel state change
    TunnelStateChange {
        tunnel_id: String,
        state: TunnelState,
        protocol: TunnelProtocol,
    },
}
```

---

## 📡 Network & Transport API

### Transport Layer

The transport layer provides flexible networking with automatic protocol selection.

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start listening on the given address
    async fn listen(&self, addr: SocketAddr) -> Result<Vec<Multiaddr>>;
    
    /// Connect to a remote peer
    async fn connect(&self, addr: &Multiaddr) -> Result<Box<dyn Connection>>;
    
    /// Get transport type
    fn transport_type(&self) -> TransportType;
    
    /// Check if transport supports the address
    fn supports_address(&self, addr: &Multiaddr) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    QUIC,
    TCP,
}

pub enum TransportSelection {
    Auto,
    Prefer(TransportType),
    Force(TransportType),
}
```

### QUIC Transport

```rust
pub struct QuicTransport {
    // Implementation details
}

impl QuicTransport {
    /// Create new QUIC transport
    pub fn new(config: QuicConfig) -> Result<Self>;
    
    /// Get QUIC-specific stats
    pub async fn quic_stats(&self) -> QuicStats;
}

#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// Keep-alive interval
    pub keep_alive_interval: Option<Duration>,
    
    /// Maximum idle timeout
    pub max_idle_timeout: Duration,
    
    /// Maximum concurrent streams
    pub max_concurrent_streams: u64,
    
    /// Enable 0-RTT connections
    pub enable_0rtt: bool,
    
    /// Congestion control algorithm
    pub congestion_control: CongestionControl,
}

#[derive(Debug, Clone)]
pub enum CongestionControl {
    NewReno,
    Cubic,
    BBR,
}
```

### TCP Transport

```rust
pub struct TcpTransport {
    // Implementation details
}

impl TcpTransport {
    /// Create new TCP transport
    pub fn new(config: TcpConfig) -> Result<Self>;
    
    /// Get TCP-specific stats
    pub async fn tcp_stats(&self) -> TcpStats;
}

#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// SO_REUSEADDR socket option
    pub reuse_addr: bool,
    
    /// TCP_NODELAY socket option  
    pub nodelay: bool,
    
    /// Keep-alive settings
    pub keep_alive: Option<TcpKeepalive>,
    
    /// Connection timeout
    pub connect_timeout: Duration,
}
```

### Connection Interface

```rust
#[async_trait]
pub trait Connection: Send + Sync {
    /// Send data to the peer
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    
    /// Receive data from the peer
    async fn receive(&mut self) -> Result<Vec<u8>>;
    
    /// Get connection information
    async fn info(&self) -> ConnectionInfo;
    
    /// Close the connection
    async fn close(&mut self) -> Result<()>;
    
    /// Check if connection is still alive
    fn is_alive(&self) -> bool;
    
    /// Get local address
    fn local_addr(&self) -> SocketAddr;
    
    /// Get remote address
    fn remote_addr(&self) -> SocketAddr;
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub transport: TransportType,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub established_at: SystemTime,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub rtt: Option<Duration>,
    pub security: SecurityInfo,
}
```

---

## 🎯 Three-Word Addresses

Convert complex network addresses into human-readable three-word phrases like "forest.lightning.compass".

### WordEncoder - Core API

```rust
pub struct WordEncoder {
    // Internal dictionary and encoding logic
}

impl WordEncoder {
    /// Create a new word encoder with default dictionary
    pub fn new() -> Self;
    
    /// Create with custom dictionary
    pub fn with_dictionary(dict: WordDictionary) -> Self;
    
    /// Encode a multiaddr to three words
    pub fn encode_multiaddr(&self, multiaddr: &Multiaddr) -> Result<ThreeWordAddress>;
    
    /// Decode three words back to multiaddr
    pub fn decode_to_multiaddr(&self, words: &ThreeWordAddress) -> Result<Multiaddr>;
    
    /// Encode raw address data
    pub fn encode_bytes(&self, data: &[u8]) -> Result<ThreeWordAddress>;
    
    /// Decode to raw bytes
    pub fn decode_bytes(&self, words: &ThreeWordAddress) -> Result<Vec<u8>>;
    
    /// Validate word combination
    pub fn validate(&self, words: &ThreeWordAddress) -> Result<()>;
}
```

### ThreeWordAddress - Address Type

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreeWordAddress {
    pub first: String,
    pub second: String,
    pub third: String,
    pub suffix: Option<u32>,
}

impl ThreeWordAddress {
    /// Create from individual words
    pub fn new(first: String, second: String, third: String) -> Self;
    
    /// Create with suffix for disambiguation
    pub fn with_suffix(first: String, second: String, third: String, suffix: u32) -> Self;
    
    /// Parse from string format
    pub fn from_string(input: &str) -> Result<Self>;
    
    /// Convert to string format
    pub fn to_string(&self) -> String;
    
    /// Get as tuple
    pub fn as_tuple(&self) -> (&str, &str, &str, Option<u32>);
    
    /// Calculate checksum for validation
    pub fn checksum(&self) -> u32;
}

impl std::fmt::Display for ThreeWordAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(suffix) = self.suffix {
            write!(f, "{}.{}.{}.{}", self.first, self.second, self.third, suffix)
        } else {
            write!(f, "{}.{}.{}", self.first, self.second, self.third)
        }
    }
}

impl std::str::FromStr for ThreeWordAddress {
    type Err = P2PError;
    
    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s)
    }
}
```

### WordDictionary - Custom Vocabularies

```rust
pub struct WordDictionary {
    // Word lists and validation
}

impl WordDictionary {
    /// Create with default English dictionary
    pub fn default() -> Self;
    
    /// Create empty dictionary
    pub fn new() -> Self;
    
    /// Add words from list
    pub fn add_words(&mut self, words: Vec<String>) -> Result<()>;
    
    /// Load from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self>;
    
    /// Get total word count
    pub fn word_count(&self) -> usize;
    
    /// Check if word exists
    pub fn contains_word(&self, word: &str) -> bool;
    
    /// Get word by index
    pub fn get_word(&self, index: usize) -> Option<&str>;
    
    /// Get index of word
    pub fn get_index(&self, word: &str) -> Option<usize>;
    
    /// Export dictionary
    pub fn export_words(&self) -> Vec<String>;
}
```

### Bootstrap Manager

```rust
pub struct BootstrapManager {
    // Bootstrap node management
}

impl BootstrapManager {
    /// Create new bootstrap manager
    pub fn new(encoder: WordEncoder) -> Self;
    
    /// Register bootstrap node with three-word address
    pub async fn register_bootstrap(&mut self, words: ThreeWordAddress, multiaddr: Multiaddr) -> Result<()>;
    
    /// Resolve three-word address to multiaddrs
    pub async fn resolve(&self, words: &ThreeWordAddress) -> Result<Vec<Multiaddr>>;
    
    /// Get all registered bootstrap nodes
    pub async fn list_bootstrap_nodes(&self) -> Vec<(ThreeWordAddress, Vec<Multiaddr>)>;
    
    /// Remove bootstrap node
    pub async fn remove_bootstrap(&mut self, words: &ThreeWordAddress) -> Result<()>;
    
    /// Update bootstrap node address
    pub async fn update_bootstrap(&mut self, words: &ThreeWordAddress, multiaddr: Multiaddr) -> Result<()>;
}
```

### Usage Examples

```rust
use saorsa_core::bootstrap::*;

// Basic encoding
let encoder = WordEncoder::new();
let multiaddr = "/ip6/2001:db8::1/udp/9000/quic".parse()?;
let words = encoder.encode_multiaddr(&multiaddr)?;
println!("Address: {}", words); // "forest.lightning.compass"

// Decoding back
let decoded = encoder.decode_to_multiaddr(&words)?;
assert_eq!(multiaddr, decoded);

// Custom dictionary
let mut dict = WordDictionary::new();
dict.add_words(vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()])?;
let custom_encoder = WordEncoder::with_dictionary(dict);

// Bootstrap management
let mut bootstrap = BootstrapManager::new(encoder);
bootstrap.register_bootstrap(words.clone(), multiaddr).await?;
let resolved = bootstrap.resolve(&words).await?;
```

---

## 🗂️ DHT & Storage

### DHT (Distributed Hash Table)

The DHT provides a distributed key-value store with security extensions.

```rust
pub struct DHT {
    // Internal routing table and storage
}

impl DHT {
    /// Create new DHT instance
    pub fn new(local_id: Key, config: DHTConfig) -> Self;
    
    /// Start DHT services
    pub async fn start(&mut self) -> Result<()>;
    
    /// Store a value in the DHT
    pub async fn put(&self, key: Key, value: Vec<u8>) -> Result<()>;
    
    /// Store with expiration
    pub async fn put_with_ttl(&self, key: Key, value: Vec<u8>, ttl: Duration) -> Result<()>;
    
    /// Retrieve a value from the DHT
    pub async fn get(&self, key: &Key) -> Option<Record>;
    
    /// Find nodes closest to a key
    pub async fn find_node(&self, key: &Key) -> Vec<DHTNode>;
    
    /// Add bootstrap node
    pub async fn add_bootstrap_node(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<()>;
    
    /// Get DHT statistics
    pub async fn stats(&self) -> DHTStats;
    
    /// Get routing table information
    pub fn routing_table_info(&self) -> RoutingTableInfo;
    
    /// Subscribe to DHT events
    pub fn subscribe_events(&self) -> Receiver<DHTEvent>;
}
```

### Key Management

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key([u8; 32]);

impl Key {
    /// Create key from data
    pub fn new(data: &[u8]) -> Self;
    
    /// Generate random key
    pub fn random() -> Self;
    
    /// Create from hex string
    pub fn from_hex(hex: &str) -> Result<Self>;
    
    /// Convert to hex string
    pub fn to_hex(&self) -> String;
    
    /// Calculate XOR distance
    pub fn distance(&self, other: &Key) -> Key;
    
    /// Get key bytes
    pub fn as_bytes(&self) -> &[u8; 32];
    
    /// Check if key is in range
    pub fn in_range(&self, start: &Key, end: &Key) -> bool;
}
```

### Records and Storage

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// The key for this record
    pub key: Key,
    
    /// The stored value
    pub value: Vec<u8>,
    
    /// Publisher peer ID
    pub publisher: PeerId,
    
    /// When the record was created
    pub created_at: SystemTime,
    
    /// When the record expires
    pub expires_at: SystemTime,
    
    /// Optional signature for integrity
    pub signature: Option<Vec<u8>>,
    
    /// Record metadata
    pub metadata: HashMap<String, String>,
}

impl Record {
    /// Create new record
    pub fn new(key: Key, value: Vec<u8>, publisher: PeerId) -> Self;
    
    /// Create with TTL
    pub fn with_ttl(key: Key, value: Vec<u8>, publisher: PeerId, ttl: Duration) -> Self;
    
    /// Sign the record
    pub fn sign(&mut self, private_key: &[u8]) -> Result<()>;
    
    /// Verify record signature
    pub fn verify(&self, public_key: &[u8]) -> Result<()>;
    
    /// Check if record is expired
    pub fn is_expired(&self) -> bool;
    
    /// Get age of record
    pub fn age(&self) -> Duration;
}
```

### Encrypted Storage Manager

```rust
pub struct StorageManager {
    // DHT interface and encryption
}

impl StorageManager {
    /// Create new storage manager
    pub fn new(dht: DHT, identity: &EnhancedIdentity) -> Result<Self>;
    
    /// Store encrypted data
    pub async fn store_encrypted<T: Serialize>(
        &mut self, 
        key: &str, 
        data: &T, 
        ttl: Duration,
        metadata: Option<Value>
    ) -> Result<()>;
    
    /// Retrieve and decrypt data
    pub async fn get_encrypted<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T>;
    
    /// Store public (unencrypted) data
    pub async fn store_public<T: Serialize>(
        &mut self, 
        key: &str, 
        data: &T, 
        ttl: Duration
    ) -> Result<()>;
    
    /// Get public data
    pub async fn get_public<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T>;
    
    /// Delete data
    pub async fn delete(&mut self, key: &str) -> Result<()>;
    
    /// List keys matching pattern
    pub async fn list_keys(&self, pattern: &str) -> Result<Vec<String>>;
    
    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats;
}
```

### File Chunking

```rust
pub struct FileChunker {
    chunk_size: usize,
}

impl FileChunker {
    /// Create new file chunker
    pub fn new(chunk_size: usize) -> Self;
    
    /// Store large file as chunks
    pub async fn store_file(
        &self,
        storage: &mut StorageManager,
        file_id: &str,
        data: &[u8],
        metadata: FileMetadata,
    ) -> Result<()>;
    
    /// Retrieve file from chunks
    pub async fn get_file(&self, storage: &StorageManager, file_id: &str) -> Result<Vec<u8>>;
    
    /// Delete file chunks
    pub async fn delete_file(&self, storage: &mut StorageManager, file_id: &str) -> Result<()>;
    
    /// Get file metadata
    pub async fn get_metadata(&self, storage: &StorageManager, file_id: &str) -> Result<FileMetadata>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub hash: Vec<u8>,
    pub total_chunks: u64,
    pub created_at: SystemTime,
    pub created_by: String,
}
```

### Storage Keys

```rust
pub mod keys {
    /// User profile data
    pub fn profile(user_id: &str) -> String;
    
    /// Chat channel data
    pub fn chat_channel(channel_id: &str) -> String;
    
    /// Chat message
    pub fn chat_message(channel_id: &str, message_id: &str) -> String;
    
    /// Project data
    pub fn project(project_id: &str) -> String;
    
    /// Document metadata
    pub fn document_meta(document_id: &str) -> String;
    
    /// Document content chunk
    pub fn document_chunk(document_id: &str, chunk_id: u64) -> String;
    
    /// Organization data
    pub fn organization(org_id: &str) -> String;
    
    /// Threshold group
    pub fn threshold_group(group_id: &str) -> String;
    
    /// Call session
    pub fn call_session(session_id: &str) -> String;
}

pub mod ttl {
    use std::time::Duration;
    
    /// Profile data (long-lived)
    pub const PROFILE: Duration = Duration::from_secs(30 * 24 * 60 * 60); // 30 days
    
    /// Chat messages (medium-lived)
    pub const CHAT_MESSAGE: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days
    
    /// Session data (short-lived)
    pub const SESSION: Duration = Duration::from_secs(60 * 60); // 1 hour
    
    /// Temporary data (very short)
    pub const TEMPORARY: Duration = Duration::from_secs(5 * 60); // 5 minutes
}
```

---

## 🔐 Quantum Cryptography

### Overview

The quantum cryptography module provides future-proof security through post-quantum algorithms and hybrid schemes.

```rust
pub struct CryptoCapabilities {
    /// Supports ML-KEM key encapsulation
    pub supports_ml_kem: bool,
    
    /// Supports ML-DSA signatures
    pub supports_ml_dsa: bool,
    
    /// Supports FROST threshold signatures
    pub supports_frost: bool,
    
    /// Supports hybrid classical/post-quantum
    pub supports_hybrid: bool,
    
    /// Can participate in threshold protocols
    pub threshold_capable: bool,
}

impl CryptoCapabilities {
    /// Get default capabilities
    pub fn default() -> Self;
    
    /// Get minimal capabilities (classical only)
    pub fn minimal() -> Self;
    
    /// Get full capabilities (all algorithms)
    pub fn full() -> Self;
    
    /// Check compatibility with remote capabilities
    pub fn is_compatible_with(&self, other: &CryptoCapabilities) -> bool;
}
```

### Algorithm Negotiation

```rust
/// Negotiate cryptographic algorithms between peers
pub fn negotiate_algorithms(
    local_caps: &CryptoCapabilities,
    remote_caps: &CryptoCapabilities,
) -> Result<NegotiatedAlgorithms>;

#[derive(Debug, Clone)]
pub struct NegotiatedAlgorithms {
    /// Key encapsulation algorithm
    pub kem: KemAlgorithm,
    
    /// Digital signature algorithm
    pub signature: SignatureAlgorithm,
    
    /// Whether to use hybrid mode
    pub hybrid_mode: bool,
    
    /// Security level achieved
    pub security_level: SecurityLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KemAlgorithm {
    X25519,           // Classical ECDH
    MlKem512,         // Post-quantum
    MlKem768,         // Post-quantum (higher security)
    MlKem1024,        // Post-quantum (highest security)
    Hybrid(Box<KemAlgorithm>, Box<KemAlgorithm>), // Combined
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignatureAlgorithm {
    Ed25519,          // Classical
    MlDsa44,          // Post-quantum
    MlDsa65,          // Post-quantum (higher security)
    MlDsa87,          // Post-quantum (highest security)
    Hybrid(Box<SignatureAlgorithm>, Box<SignatureAlgorithm>), // Combined
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    Level1,   // 128-bit classical security
    Level3,   // 192-bit classical security
    Level5,   // 256-bit classical security
}
```

### Key Management

```rust
#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public: PublicKeySet,
    pub private: PrivateKeySet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeySet {
    /// ML-DSA public key for signatures
    pub ml_dsa: Option<MlDsaPublicKey>,
    
    /// ML-KEM public key for encryption
    pub ml_kem: Option<MlKemPublicKey>,
    
    /// Ed25519 public key for classical operations
    pub ed25519: Option<Ed25519PublicKey>,
    
    /// FROST group public key for threshold operations
    pub frost: Option<FrostGroupPublicKey>,
}

#[derive(Debug, Clone)]
pub struct PrivateKeySet {
    /// ML-DSA private key
    pub ml_dsa: Option<MlDsaPrivateKey>,
    
    /// ML-KEM private key
    pub ml_kem: Option<MlKemPrivateKey>,
    
    /// Ed25519 private key
    pub ed25519: Option<Ed25519PrivateKey>,
    
    /// FROST participant share
    pub frost: Option<FrostParticipantKey>,
}

/// Generate a new keypair with specified capabilities
pub async fn generate_keypair(capabilities: &CryptoCapabilities) -> Result<KeyPair>;
```

### Signature Schemes

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignatureScheme {
    /// Classical signature only
    Classical(Vec<u8>),
    
    /// Post-quantum signature only
    PostQuantum(Vec<u8>),
    
    /// Both classical and post-quantum signatures
    Dual {
        classical: Vec<u8>,
        post_quantum: Vec<u8>,
    },
    
    /// Threshold signature
    Threshold(ThresholdSignature),
}

impl SignatureScheme {
    /// Sign a message using the specified scheme
    pub fn sign(
        message: &[u8],
        private_keys: &PrivateKeySet,
        algorithm: SignatureAlgorithm,
    ) -> Result<Self>;
    
    /// Verify a signature
    pub fn verify(
        &self,
        message: &[u8],
        public_keys: &PublicKeySet,
    ) -> Result<()>;
    
    /// Get signature size in bytes
    pub fn size(&self) -> usize;
    
    /// Check if signature provides post-quantum security
    pub fn is_post_quantum_secure(&self) -> bool;
}
```

### ML-KEM (Key Encapsulation)

```rust
pub mod ml_kem {
    use super::*;
    
    /// Generate ML-KEM keypair
    pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)>;
    
    /// Encapsulate a shared secret
    pub fn encapsulate(public_key: &[u8]) -> Result<(Vec<u8>, SharedSecret)>;
    
    /// Decapsulate to recover shared secret
    pub fn decapsulate(private_key: &[u8], ciphertext: &[u8]) -> Result<SharedSecret>;
    
    /// Shared secret type
    #[derive(Debug, Clone)]
    pub struct SharedSecret([u8; 32]);
    
    impl SharedSecret {
        pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
        pub fn derive_key(&self, info: &[u8]) -> Result<[u8; 32]>;
    }
    
    /// ML-KEM state for protocol operations
    pub struct MlKemState {
        pub role: KeyExchangeRole,
        pub public_key: Option<Vec<u8>>,
        pub private_key: Option<Vec<u8>>,
        pub shared_secret: Option<SharedSecret>,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum KeyExchangeRole {
        Initiator,
        Responder,
    }
    
    impl MlKemState {
        pub fn new(role: KeyExchangeRole) -> Self;
        pub fn generate_keypair(&mut self) -> Result<Vec<u8>>;
        pub fn set_remote_public_key(&mut self, public_key: Vec<u8>);
        pub fn complete_as_initiator(&mut self, ciphertext: &[u8]) -> Result<()>;
        pub fn complete_as_responder(&mut self) -> Result<(Vec<u8>, SharedSecret)>;
    }
}
```

### ML-DSA (Digital Signatures)

```rust
pub mod ml_dsa {
    use super::*;
    
    /// Generate ML-DSA keypair
    pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)>;
    
    /// Sign a message
    pub fn sign(private_key: &[u8], message: &[u8]) -> Result<Vec<u8>>;
    
    /// Verify a signature
    pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<()>;
    
    /// ML-DSA state for signing operations
    pub struct MlDsaState {
        pub keypair: Option<(MlDsaPublicKey, MlDsaPrivateKey)>,
        pub signatures: Vec<CachedSignature>,
    }
    
    impl MlDsaState {
        pub fn new() -> Self;
        pub fn generate_keypair(&mut self) -> Result<MlDsaPublicKey>;
        pub fn sign_message(&mut self, message: &[u8]) -> Result<MlDsaSignature>;
        pub fn verify_signature(
            &self,
            public_key: &MlDsaPublicKey,
            message: &[u8],
            signature: &MlDsaSignature,
        ) -> Result<()>;
        pub fn get_cached_signature(&self, message: &[u8]) -> Option<&MlDsaSignature>;
    }
    
    /// Batch signature verification for performance
    pub struct BatchVerifier {
        // Implementation details
    }
    
    impl BatchVerifier {
        pub fn new() -> Self;
        pub fn add(&mut self, public_key: &MlDsaPublicKey, message: &[u8], signature: &MlDsaSignature);
        pub fn verify_all(&self) -> Result<Vec<bool>>;
        pub fn clear(&mut self);
    }
}
```

### Hybrid Cryptography

```rust
pub mod hybrid {
    use super::*;
    
    /// Hybrid key exchange combining classical and post-quantum
    pub struct HybridKeyExchange {
        pub ml_kem_state: ml_kem::MlKemState,
        pub x25519_private: Option<[u8; 32]>,
        pub x25519_public: Option<[u8; 32]>,
        pub x25519_shared: Option<[u8; 32]>,
        pub hybrid_secret: Option<[u8; 32]>,
    }
    
    impl HybridKeyExchange {
        pub fn new(role: ml_kem::KeyExchangeRole) -> Self;
        pub fn generate_x25519_keypair(&mut self) -> Result<[u8; 32]>;
        pub fn set_remote_x25519_public(&mut self, remote_public: [u8; 32]) -> Result<()>;
        pub fn derive_hybrid_secret(&mut self) -> Result<[u8; 32]>;
    }
    
    /// Hybrid signer combining classical and post-quantum signatures
    pub struct HybridSigner {
        pub ml_dsa_state: ml_dsa::MlDsaState,
        pub ed25519_keypair: Option<ed25519_dalek::Keypair>,
    }
    
    impl HybridSigner {
        pub fn new() -> Self;
        pub fn generate_keypair(&mut self) -> Result<(PublicKeySet, PrivateKeySet)>;
        pub fn sign_hybrid(&mut self, message: &[u8]) -> Result<HybridSignature>;
        pub fn verify_hybrid(
            public_keys: &PublicKeySet,
            message: &[u8],
            signature: &HybridSignature,
        ) -> Result<()>;
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HybridSignature {
        pub classical: Ed25519Signature,
        pub post_quantum: MlDsaSignature,
    }
    
    /// Migration utilities for upgrading from classical to hybrid
    pub mod migration {
        pub fn upgrade_ed25519_identity(
            ed25519_public: &[u8],
            ed25519_private: &[u8],
        ) -> Result<(PublicKeySet, PrivateKeySet)>;
        
        pub fn create_compatible_signature(
            signer: &mut HybridSigner,
            message: &[u8],
            use_hybrid: bool,
        ) -> Result<SignatureScheme>;
    }
}
```

---

## ⚖️ Threshold Signatures

### FROST (Flexible Round-Optimized Schnorr Threshold)

Threshold signatures allow t-of-n participants to collectively sign messages.

```rust
pub struct ThresholdGroup {
    /// Unique group identifier
    pub group_id: GroupId,
    
    /// Minimum signatures required (t)
    pub threshold: u16,
    
    /// Total number of participants (n)
    pub participants: u16,
    
    /// Group public key for verification
    pub frost_group_key: FrostGroupPublicKey,
    
    /// Currently active participants
    pub active_participants: Vec<ParticipantInfo>,
    
    /// Group version for updates
    pub version: u64,
    
    /// Additional metadata
    pub metadata: GroupMetadata,
}

impl ThresholdGroup {
    /// Create new threshold group
    pub async fn create(
        threshold: u16,
        participants: u16,
        creator_id: ParticipantId,
    ) -> Result<(Self, HashMap<ParticipantId, ParticipantShare>)>;
    
    /// Add new participant
    pub async fn add_participant(&mut self, participant: ParticipantInfo) -> Result<()>;
    
    /// Remove participant
    pub async fn remove_participant(&mut self, participant_id: &ParticipantId) -> Result<()>;
    
    /// Update threshold
    pub async fn update_threshold(&mut self, new_threshold: u16) -> Result<()>;
    
    /// Get signing participants for threshold
    pub fn get_signing_participants(&self, count: usize) -> Vec<ParticipantId>;
    
    /// Verify group integrity
    pub fn verify_integrity(&self) -> Result<()>;
}
```

### Threshold Group Manager

```rust
pub struct ThresholdGroupManager {
    local_identity: QuantumPeerIdentity,
    groups: HashMap<GroupId, ThresholdGroup>,
    shares: HashMap<GroupId, ParticipantShare>,
    active_sessions: HashMap<[u8; 32], FrostSession>,
}

impl ThresholdGroupManager {
    /// Create new manager
    pub fn new(local_identity: QuantumPeerIdentity) -> Self;
    
    /// Create a new threshold group
    pub async fn create_group(&mut self, config: GroupConfig) -> Result<ThresholdGroup>;
    
    /// Join an existing group
    pub async fn join_group(
        &mut self,
        group_id: GroupId,
        invitation: GroupInvitation,
    ) -> Result<()>;
    
    /// Leave a group
    pub async fn leave_group(&mut self, group_id: &GroupId) -> Result<()>;
    
    /// Propose a group operation
    pub async fn propose_operation(
        &mut self,
        group_id: &GroupId,
        operation: GroupOperation,
    ) -> Result<OperationId>;
    
    /// Vote on a proposal
    pub async fn vote_on_proposal(
        &mut self,
        operation_id: &OperationId,
        vote: ProposalVote,
    ) -> Result<()>;
    
    /// Initiate threshold signing
    pub async fn initiate_signing(
        &mut self,
        group_id: &GroupId,
        message: &[u8],
    ) -> Result<[u8; 32]>; // Returns session ID
    
    /// Participate in threshold signing
    pub async fn participate_in_signing(
        &mut self,
        session_id: &[u8; 32],
    ) -> Result<SigningContribution>;
    
    /// Complete threshold signing
    pub async fn complete_signing(
        &mut self,
        session_id: &[u8; 32],
    ) -> Result<ThresholdSignature>;
    
    /// Verify threshold signature
    pub fn verify_threshold_signature(
        &self,
        group_id: &GroupId,
        message: &[u8],
        signature: &ThresholdSignature,
    ) -> Result<()>;
    
    /// Get groups where we are a member
    pub fn get_member_groups(&self) -> Vec<&ThresholdGroup>;
    
    /// Get group information
    pub fn get_group(&self, group_id: &GroupId) -> Option<&ThresholdGroup>;
}
```

### FROST Session Management

```rust
pub struct FrostSession {
    pub session_id: [u8; 32],
    pub message: Vec<u8>,
    pub threshold: u16,
    pub commitments: HashMap<ParticipantId, SigningCommitments>,
    pub shares: HashMap<ParticipantId, SigningShare>,
    pub group_public_key: FrostGroupPublicKey,
    pub state: SessionState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    CollectingCommitments,
    CollectingShares,
    ReadyToAggregate,
    Completed,
    Failed(String),
}

impl FrostSession {
    pub fn new(message: Vec<u8>, threshold: u16, group_public_key: FrostGroupPublicKey) -> Self;
    pub fn add_commitments(&mut self, participant_id: ParticipantId, commitments: SigningCommitments) -> Result<()>;
    pub fn add_share(&mut self, participant_id: ParticipantId, share: SigningShare) -> Result<()>;
    pub fn aggregate(&mut self) -> Result<FrostSignature>;
    pub fn is_complete(&self) -> bool;
    pub fn get_progress(&self) -> SessionProgress;
}

pub struct FrostCoordinator {
    pub sessions: HashMap<[u8; 32], FrostSession>,
    pub groups: HashMap<GroupId, GroupInfo>,
}

impl FrostCoordinator {
    pub fn new() -> Self;
    pub fn register_group(&mut self, group_id: GroupId, group_public_key: FrostGroupPublicKey, threshold: u16, participants: Vec<ParticipantId>);
    pub fn initiate_signing(&mut self, group_id: &GroupId, message: Vec<u8>) -> Result<[u8; 32]>;
    pub fn process_commitment(&mut self, session_id: &[u8; 32], participant_id: ParticipantId, commitments: SigningCommitments) -> Result<()>;
    pub fn process_share(&mut self, session_id: &[u8; 32], participant_id: ParticipantId, share: SigningShare) -> Result<()>;
    pub fn complete_signing(&mut self, session_id: &[u8; 32]) -> Result<FrostSignature>;
    pub fn get_session_status(&self, session_id: &[u8; 32]) -> Option<SessionProgress>;
}
```

### Key Generation

```rust
/// Generate FROST key shares for a group
pub async fn generate_key_shares(
    threshold: u16,
    participants: u16,
) -> Result<KeyGenerationResult>;

pub struct KeyGenerationResult {
    /// Group public key for verification
    pub group_public_key: FrostGroupPublicKey,
    
    /// Individual participant shares
    pub shares: HashMap<ParticipantId, ParticipantShare>,
    
    /// Public commitments for verification
    pub commitments: Vec<Vec<u8>>,
}

#[derive(Clone)]
pub struct ParticipantShare {
    pub participant_id: ParticipantId,
    pub signing_share: Vec<u8>,
    pub verifying_share: Vec<u8>,
}
```

### Group Operations

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupOperation {
    /// Add new participant
    AddParticipant {
        participant: ParticipantInfo,
        permissions: ParticipantPermissions,
    },
    
    /// Remove participant
    RemoveParticipant {
        participant_id: ParticipantId,
        reason: String,
    },
    
    /// Change threshold
    ChangeThreshold {
        new_threshold: u16,
    },
    
    /// Update group metadata
    UpdateMetadata {
        metadata: GroupMetadata,
    },
    
    /// Dissolve group
    DissolveGroup {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalVote {
    Approve,
    Reject { reason: String },
    Abstain,
}

#[derive(Debug, Clone)]
pub struct ParticipantInfo {
    pub participant_id: ParticipantId,
    pub public_key: PublicKeySet,
    pub role: ParticipantRole,
    pub joined_at: SystemTime,
    pub permissions: ParticipantPermissions,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParticipantRole {
    Leader {
        permissions: LeaderPermissions,
    },
    Member {
        permissions: MemberPermissions,
    },
    Observer,
}
```

---

## 🌐 IPv6 Tunneling

### Tunnel Protocols

The tunneling system provides comprehensive IPv6 connectivity over IPv4 networks.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TunnelProtocol {
    /// 6to4 automatic tunneling (RFC 3056)
    SixToFour,
    
    /// Teredo NAT traversal (RFC 4380)
    Teredo,
    
    /// 6in4 static tunneling (RFC 4213)
    SixInFour,
    
    /// DS-Lite (RFC 6333)
    DsLite,
    
    /// ISATAP for enterprise networks (RFC 5214)
    Isatap,
    
    /// MAP-E (RFC 7597)
    MapE,
    
    /// MAP-T (RFC 7599)
    MapT,
}

impl TunnelProtocol {
    /// Get protocol description
    pub fn description(&self) -> &'static str;
    
    /// Check if protocol supports NAT traversal
    pub fn supports_nat_traversal(&self) -> bool;
    
    /// Get typical use case
    pub fn use_case(&self) -> &'static str;
    
    /// Get RFC reference
    pub fn rfc(&self) -> &'static str;
}
```

### Tunnel Interface

```rust
#[async_trait]
pub trait Tunnel: Send + Sync {
    /// Establish the tunnel
    async fn establish(&mut self, config: TunnelConfig) -> Result<()>;
    
    /// Send IPv6 packet through tunnel
    async fn send_packet(&mut self, packet: &[u8]) -> Result<()>;
    
    /// Receive IPv6 packet from tunnel
    async fn receive_packet(&mut self) -> Result<Vec<u8>>;
    
    /// Get tunnel metrics
    async fn get_metrics(&self) -> TunnelMetrics;
    
    /// Update tunnel configuration
    async fn update_config(&mut self, config: TunnelConfig) -> Result<()>;
    
    /// Get tunnel state
    fn get_state(&self) -> TunnelState;
    
    /// Shutdown tunnel
    async fn shutdown(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed(String),
}
```

### Tunnel Configuration

```rust
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Tunnel protocol to use
    pub protocol: TunnelProtocol,
    
    /// Local IPv4 endpoint
    pub local_endpoint: SocketAddr,
    
    /// Remote IPv4 endpoint
    pub remote_endpoint: Option<SocketAddr>,
    
    /// IPv6 prefix for tunnel
    pub ipv6_prefix: Option<Ipv6Network>,
    
    /// MTU for tunnel interface
    pub mtu: u16,
    
    /// Enable keepalive
    pub keepalive: Option<Duration>,
    
    /// Tunnel-specific options
    pub options: TunnelOptions,
}

#[derive(Debug, Clone, Default)]
pub struct TunnelOptions {
    /// Custom options per protocol
    pub custom: HashMap<String, String>,
}

impl TunnelConfig {
    /// Create config for automatic 6to4
    pub fn sixto4_auto() -> Self;
    
    /// Create config for Teredo with NAT
    pub fn teredo_nat() -> Self;
    
    /// Create config for enterprise ISATAP
    pub fn isatap_enterprise(router: Ipv4Addr) -> Self;
    
    /// Create config for static 6in4
    pub fn sixinfour_static(remote: SocketAddr, prefix: Ipv6Network) -> Self;
}
```

### Tunnel Manager

```rust
pub struct TunnelManager {
    // Active tunnels
    tunnels: HashMap<String, Box<dyn Tunnel>>,
    
    // Configuration
    config: TunnelManagerConfig,
    
    // Metrics
    metrics: TunnelManagerMetrics,
}

impl TunnelManager {
    /// Create new tunnel manager
    pub fn new(config: TunnelManagerConfig) -> Self;
    
    /// Start tunnel manager
    pub async fn start(&mut self) -> Result<()>;
    
    /// Create and establish tunnel
    pub async fn create_tunnel(
        &mut self,
        name: String,
        config: TunnelConfig,
    ) -> Result<String>; // Returns tunnel ID
    
    /// Get tunnel by ID
    pub fn get_tunnel(&self, tunnel_id: &str) -> Option<&dyn Tunnel>;
    
    /// Get mutable tunnel by ID
    pub fn get_tunnel_mut(&mut self, tunnel_id: &str) -> Option<&mut dyn Tunnel>;
    
    /// List all tunnels
    pub fn list_tunnels(&self) -> Vec<(String, TunnelState, TunnelProtocol)>;
    
    /// Remove tunnel
    pub async fn remove_tunnel(&mut self, tunnel_id: &str) -> Result<()>;
    
    /// Get best tunnel for destination
    pub fn get_best_tunnel(&self, destination: Ipv6Addr) -> Option<&str>;
    
    /// Route packet through appropriate tunnel
    pub async fn route_packet(&mut self, packet: &[u8], destination: Ipv6Addr) -> Result<()>;
    
    /// Get aggregated metrics
    pub fn get_metrics(&self) -> &TunnelManagerMetrics;
    
    /// Subscribe to tunnel events
    pub fn subscribe_events(&self) -> Receiver<TunnelEvent>;
}
```

### Specific Tunnel Implementations

```rust
/// 6to4 Tunnel (RFC 3056)
pub struct SixToFourTunnel {
    // Implementation details
}

impl SixToFourTunnel {
    pub fn new() -> Self;
    
    /// Calculate 6to4 IPv6 address from IPv4
    pub fn calculate_6to4_address(ipv4: Ipv4Addr) -> Ipv6Addr;
    
    /// Extract IPv4 from 6to4 address
    pub fn extract_ipv4(ipv6: Ipv6Addr) -> Option<Ipv4Addr>;
}

/// Teredo Tunnel (RFC 4380)
pub struct TeredoTunnel {
    // Implementation details
}

impl TeredoTunnel {
    pub fn new() -> Self;
    
    /// Perform Teredo qualification
    pub async fn qualify(&mut self, server: SocketAddr) -> Result<TeredoQualification>;
    
    /// Get Teredo IPv6 address
    pub fn get_teredo_address(&self) -> Option<Ipv6Addr>;
}

#[derive(Debug, Clone)]
pub struct TeredoQualification {
    pub cone_nat: bool,
    pub external_address: SocketAddr,
    pub server_address: SocketAddr,
}

/// ISATAP Tunnel (RFC 5214)
pub struct IsatapTunnel {
    // Implementation details
}

impl IsatapTunnel {
    pub fn new(router: Ipv4Addr) -> Self;
    
    /// Generate ISATAP identifier
    pub fn generate_identifier(ipv4: Ipv4Addr) -> [u8; 8];
    
    /// Perform router discovery
    pub async fn discover_router(&mut self) -> Result<Ipv4Addr>;
}
```

### Tunnel Metrics

```rust
#[derive(Debug, Clone, Default)]
pub struct TunnelMetrics {
    /// Bytes sent through tunnel
    pub bytes_sent: u64,
    
    /// Bytes received from tunnel
    pub bytes_received: u64,
    
    /// Packets sent
    pub packets_sent: u64,
    
    /// Packets received
    pub packets_received: u64,
    
    /// Connection uptime
    pub uptime: Duration,
    
    /// Average round-trip time
    pub avg_rtt: Option<Duration>,
    
    /// Packet loss rate (0.0 to 1.0)
    pub packet_loss: f64,
    
    /// Current bandwidth utilization
    pub bandwidth_utilization: f64,
    
    /// Error count
    pub errors: u64,
    
    /// Last error
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TunnelManagerMetrics {
    /// Number of active tunnels
    pub active_tunnels: usize,
    
    /// Total bytes across all tunnels
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    
    /// Per-protocol statistics
    pub protocol_stats: HashMap<TunnelProtocol, TunnelProtocolStats>,
    
    /// Connection success rate
    pub connection_success_rate: f64,
}

#[derive(Debug, Clone, Default)]
pub struct TunnelProtocolStats {
    pub tunnel_count: usize,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_setup_time: Duration,
}
```

### Tunnel Events

```rust
#[derive(Debug, Clone)]
pub enum TunnelEvent {
    /// Tunnel established successfully
    TunnelEstablished {
        tunnel_id: String,
        protocol: TunnelProtocol,
        local_addr: SocketAddr,
        remote_addr: Option<SocketAddr>,
    },
    
    /// Tunnel disconnected
    TunnelDisconnected {
        tunnel_id: String,
        reason: DisconnectReason,
    },
    
    /// Tunnel failed to establish
    TunnelFailed {
        tunnel_id: String,
        protocol: TunnelProtocol,
        error: String,
    },
    
    /// Tunnel metrics updated
    MetricsUpdated {
        tunnel_id: String,
        metrics: TunnelMetrics,
    },
    
    /// Packet routing decision
    PacketRouted {
        destination: Ipv6Addr,
        tunnel_id: String,
        protocol: TunnelProtocol,
    },
}

#[derive(Debug, Clone)]
pub enum DisconnectReason {
    UserRequested,
    NetworkError(String),
    Timeout,
    ProtocolError(String),
    Shutdown,
}
```

---

## 🤖 MCP Integration

### MCP Server

The Model Context Protocol (MCP) integration enables AI applications to interact with the P2P network.

```rust
pub struct MCPServer {
    // Server state and configuration
}

impl MCPServer {
    /// Create new MCP server
    pub fn new(config: MCPServerConfig) -> Self;
    
    /// Start the MCP server
    pub async fn start(&mut self) -> Result<()>;
    
    /// Stop the MCP server
    pub async fn stop(&mut self) -> Result<()>;
    
    /// Register a tool
    pub async fn register_tool(&mut self, tool: Tool) -> Result<()>;
    
    /// Unregister a tool
    pub async fn unregister_tool(&mut self, name: &str) -> Result<()>;
    
    /// Handle incoming MCP message
    pub async fn handle_message(&self, message: MCPMessage) -> Result<MCPMessage>;
    
    /// Broadcast service discovery
    pub async fn broadcast_service(&self, service: MCPService) -> Result<()>;
    
    /// Get server statistics
    pub fn get_stats(&self) -> MCPServerStats;
    
    /// Get list of registered tools
    pub fn list_tools(&self) -> Vec<&Tool>;
    
    /// Set capabilities
    pub fn set_capabilities(&mut self, capabilities: MCPCapabilities);
}
```

### MCP Configuration

```rust
#[derive(Debug, Clone)]
pub struct MCPServerConfig {
    /// Server listening address
    pub listen_addr: SocketAddr,
    
    /// Server name for identification
    pub server_name: String,
    
    /// Server version
    pub version: String,
    
    /// Supported MCP protocol version
    pub protocol_version: String,
    
    /// Maximum message size
    pub max_message_size: usize,
    
    /// Request timeout
    pub request_timeout: Duration,
    
    /// Enable service discovery
    pub enable_discovery: bool,
    
    /// Authentication settings
    pub auth: Option<MCPAuthConfig>,
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:3000".parse().unwrap(),
            server_name: "p2p-foundation".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: "2024-11-05".to_string(),
            max_message_size: 1024 * 1024, // 1MB
            request_timeout: Duration::from_secs(30),
            enable_discovery: true,
            auth: None,
        }
    }
}
```

### MCP Messages

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum MCPMessage {
    /// Initialize MCP session
    #[serde(rename = "initialize")]
    Initialize {
        protocol_version: String,
        capabilities: MCPCapabilities,
        client_info: MCPClientInfo,
    },
    
    /// List available tools
    #[serde(rename = "tools/list")]
    ListTools {
        cursor: Option<String>,
    },
    
    /// Call a specific tool
    #[serde(rename = "tools/call")]
    CallTool {
        name: String,
        arguments: Value,
    },
    
    /// List available resources
    #[serde(rename = "resources/list")]
    ListResources {
        cursor: Option<String>,
    },
    
    /// Read a specific resource
    #[serde(rename = "resources/read")]
    ReadResource {
        uri: String,
    },
    
    /// Subscribe to resource changes
    #[serde(rename = "resources/subscribe")]
    SubscribeResource {
        uri: String,
    },
    
    /// List available prompts
    #[serde(rename = "prompts/list")]
    ListPrompts {
        cursor: Option<String>,
    },
    
    /// Get a specific prompt
    #[serde(rename = "prompts/get")]
    GetPrompt {
        name: String,
        arguments: Option<Value>,
    },
    
    /// Ping for connection health
    #[serde(rename = "ping")]
    Ping,
    
    /// Notification message
    #[serde(rename = "notifications/message")]
    NotificationMessage {
        level: NotificationLevel,
        logger: Option<String>,
        data: Value,
    },
    
    /// Progress notification
    #[serde(rename = "notifications/progress")]
    NotificationProgress {
        progress_token: String,
        progress: f64,
        total: Option<f64>,
    },
    
    /// Response messages
    #[serde(rename = "response")]
    Response {
        id: String,
        result: Option<Value>,
        error: Option<MCPError>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPCapabilities {
    /// Experimental capabilities
    pub experimental: Option<Value>,
    
    /// Sampling capabilities
    pub sampling: Option<Value>,
    
    /// Logging capabilities
    pub logging: Option<MCPLoggingCapabilities>,
    
    /// Tool capabilities
    pub tools: Option<MCPToolCapabilities>,
    
    /// Resource capabilities
    pub resources: Option<MCPResourceCapabilities>,
    
    /// Prompt capabilities
    pub prompts: Option<MCPPromptCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPClientInfo {
    pub name: String,
    pub version: String,
}
```

### Tools System

```rust
pub struct Tool {
    /// Tool name (unique identifier)
    pub name: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Input schema (JSON Schema)
    pub input_schema: Value,
    
    /// Tool handler function
    handler: Box<dyn ToolHandler>,
}

impl Tool {
    /// Create new tool
    pub fn new<H>(name: String, description: String, handler: H) -> Self
    where
        H: ToolHandler + 'static;
    
    /// Call the tool with arguments
    pub async fn call(&self, args: Value, context: MCPCallContext) -> Result<Value>;
    
    /// Get tool schema
    pub fn schema(&self) -> ToolSchema;
    
    /// Validate arguments against schema
    pub fn validate_args(&self, args: &Value) -> Result<()>;
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with given arguments
    async fn execute(&self, args: Value, context: MCPCallContext) -> Result<Value>;
    
    /// Get tool metadata
    fn metadata(&self) -> ToolMetadata;
}

#[derive(Debug, Clone)]
pub struct MCPCallContext {
    /// Client making the request
    pub client_id: String,
    
    /// Request timestamp
    pub timestamp: SystemTime,
    
    /// Session information
    pub session: MCPSession,
    
    /// Additional context data
    pub context: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
    pub author: Option<String>,
}
```

### Built-in Tools

```rust
/// P2P network information tool
pub struct NetworkInfoTool;

impl ToolHandler for NetworkInfoTool {
    async fn execute(&self, _args: Value, context: MCPCallContext) -> Result<Value> {
        // Return network statistics, peer information, etc.
    }
}

/// DHT operations tool
pub struct DHTTool;

impl ToolHandler for DHTTool {
    async fn execute(&self, args: Value, context: MCPCallContext) -> Result<Value> {
        // Perform DHT get/put operations
    }
}

/// Chat messaging tool
pub struct ChatTool;

impl ToolHandler for ChatTool {
    async fn execute(&self, args: Value, context: MCPCallContext) -> Result<Value> {
        // Send messages, create channels, etc.
    }
}

/// Project management tool
pub struct ProjectTool;

impl ToolHandler for ProjectTool {
    async fn execute(&self, args: Value, context: MCPCallContext) -> Result<Value> {
        // Create projects, upload documents, manage workflows
    }
}

/// Three-word address tool
pub struct ThreeWordTool;

impl ToolHandler for ThreeWordTool {
    async fn execute(&self, args: Value, context: MCPCallContext) -> Result<Value> {
        // Encode/decode three-word addresses
    }
}

/// Identity management tool
pub struct IdentityTool;

impl ToolHandler for IdentityTool {
    async fn execute(&self, args: Value, context: MCPCallContext) -> Result<Value> {
        // Manage identities, organizations, threshold groups
    }
}
```

### Service Discovery

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPService {
    /// Service identifier
    pub id: String,
    
    /// Service name
    pub name: String,
    
    /// Service description
    pub description: String,
    
    /// Service endpoint
    pub endpoint: String,
    
    /// Supported capabilities
    pub capabilities: MCPCapabilities,
    
    /// Service metadata
    pub metadata: HashMap<String, Value>,
    
    /// Service version
    pub version: String,
}

impl MCPService {
    /// Create new service description
    pub fn new(id: String, name: String, endpoint: String) -> Self;
    
    /// Add capability
    pub fn with_capability(mut self, capability: String) -> Self;
    
    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: Value) -> Self;
    
    /// Check if service supports capability
    pub fn supports_capability(&self, capability: &str) -> bool;
}

/// Service discovery manager
pub struct MCPServiceDiscovery {
    // Service registry
}

impl MCPServiceDiscovery {
    pub fn new() -> Self;
    pub async fn discover_services(&self) -> Result<Vec<MCPService>>;
    pub async fn register_service(&mut self, service: MCPService) -> Result<()>;
    pub async fn unregister_service(&mut self, service_id: &str) -> Result<()>;
    pub fn find_services_by_capability(&self, capability: &str) -> Vec<&MCPService>;
}
```

---

## 👤 Identity & Organizations

### Enhanced Identity System

```rust
pub struct EnhancedIdentity {
    /// Base user identity
    pub base_identity: UserIdentity,
    
    /// Quantum-resistant cryptographic identity
    pub quantum_identity: QuantumPeerIdentity,
    
    /// Threshold group memberships
    pub threshold_groups: Vec<GroupMembership>,
    
    /// Organization memberships
    pub organizations: Vec<OrganizationMembership>,
    
    /// Device registry for multi-device support
    pub devices: DeviceRegistry,
}

impl EnhancedIdentity {
    /// Create new enhanced identity
    pub async fn new(user_info: UserInfo) -> Result<Self>;
    
    /// Load identity from storage
    pub async fn load(user_id: &str, storage: &StorageManager) -> Result<Self>;
    
    /// Save identity to storage
    pub async fn save(&self, storage: &mut StorageManager) -> Result<()>;
    
    /// Add device to identity
    pub async fn add_device(&mut self, device: DeviceInfo) -> Result<()>;
    
    /// Remove device from identity
    pub async fn remove_device(&mut self, device_id: &str) -> Result<()>;
    
    /// Join organization
    pub async fn join_organization(&mut self, org_id: OrganizationId, role: OrganizationRole) -> Result<()>;
    
    /// Leave organization
    pub async fn leave_organization(&mut self, org_id: &OrganizationId) -> Result<()>;
    
    /// Get permissions for organization
    pub fn get_org_permissions(&self, org_id: &OrganizationId) -> Vec<Permission>;
    
    /// Check if has permission
    pub fn has_permission(&self, permission: &Permission, context: &PermissionContext) -> bool;
    
    /// Generate identity proof
    pub fn generate_proof(&self) -> Result<IdentityProof>;
    
    /// Verify identity proof
    pub fn verify_proof(proof: &IdentityProof) -> Result<()>;
}
```

### Base Identity Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// Unique user identifier
    pub user_id: String,
    
    /// Display name
    pub display_name: String,
    
    /// Email address (optional)
    pub email: Option<String>,
    
    /// Profile picture (optional)
    pub avatar: Option<String>,
    
    /// Creation timestamp
    pub created_at: SystemTime,
    
    /// Last updated timestamp
    pub updated_at: SystemTime,
    
    /// User preferences
    pub preferences: UserPreferences,
    
    /// Profile visibility settings
    pub visibility: ProfileVisibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumPeerIdentity {
    /// Peer ID derived from public key
    pub peer_id: PeerId,
    
    /// Quantum-resistant key set
    pub public_keys: PublicKeySet,
    
    /// Three-word address
    pub three_word_address: Option<ThreeWordAddress>,
    
    /// Network addresses
    pub network_addresses: Vec<Multiaddr>,
    
    /// Cryptographic capabilities
    pub capabilities: CryptoCapabilities,
    
    /// Identity signature
    pub signature: SignatureScheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// UI theme preference
    pub theme: String,
    
    /// Language preference
    pub language: String,
    
    /// Notification settings
    pub notifications: NotificationSettings,
    
    /// Privacy settings
    pub privacy: PrivacySettings,
    
    /// Custom preferences
    pub custom: HashMap<String, Value>,
}
```

### Organizations

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    /// Unique organization identifier
    pub id: OrganizationId,
    
    /// Organization name
    pub name: String,
    
    /// Organization description
    pub description: String,
    
    /// Organization type
    pub org_type: OrganizationType,
    
    /// Owner user ID
    pub owner: String,
    
    /// Organization settings
    pub settings: OrganizationSettings,
    
    /// Departments within organization
    pub departments: Vec<Department>,
    
    /// Organization metadata
    pub metadata: OrganizationMetadata,
    
    /// Creation info
    pub created_at: SystemTime,
    pub created_by: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrganizationType {
    /// Individual/personal organization
    Personal,
    
    /// Small team or startup
    Team,
    
    /// Business or company
    Business,
    
    /// Non-profit organization
    NonProfit,
    
    /// Educational institution
    Educational,
    
    /// Government entity
    Government,
    
    /// Open source project
    OpenSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub id: DepartmentId,
    pub name: String,
    pub description: String,
    pub parent_id: Option<DepartmentId>,
    pub manager: Option<String>,
    pub teams: Vec<Team>,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub description: String,
    pub department_id: DepartmentId,
    pub lead: Option<String>,
    pub members: Vec<TeamMember>,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub user_id: String,
    pub role: TeamRole,
    pub joined_at: SystemTime,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TeamRole {
    Lead,
    Member,
    Contributor,
    Observer,
}
```

### Organization Roles & Permissions

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrganizationRole {
    /// Full control over organization
    Owner,
    
    /// Administrative privileges
    Admin {
        permissions: AdminPermissions,
    },
    
    /// Department or team management
    Manager {
        scope: ManagerScope,
        permissions: ManagerPermissions,
    },
    
    /// Regular member
    Member {
        permissions: MemberPermissions,
    },
    
    /// Limited access guest
    Guest {
        expires_at: Option<SystemTime>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    // Organization-level permissions
    ManageOrganization,
    ViewOrganization,
    
    // Department permissions
    CreateDepartment,
    ManageDepartment(DepartmentId),
    ViewDepartment(DepartmentId),
    
    // Team permissions
    CreateTeam,
    ManageTeam(TeamId),
    ViewTeam(TeamId),
    
    // Project permissions
    CreateProject,
    ManageProject(ProjectId),
    ViewProject(ProjectId),
    
    // Document permissions
    CreateDocument,
    EditDocument(DocumentId),
    ViewDocument(DocumentId),
    ApproveDocument(DocumentId),
    
    // Chat permissions
    CreateChannel,
    ManageChannel(String),
    SendMessage(String),
    
    // System permissions
    ManageUsers,
    ViewAnalytics,
    ManageIntegrations,
}

#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub organization_id: Option<OrganizationId>,
    pub department_id: Option<DepartmentId>,
    pub team_id: Option<TeamId>,
    pub project_id: Option<ProjectId>,
    pub user_id: String,
}
```

### Identity Manager

```rust
pub struct IdentityManager {
    storage: StorageManager,
    quantum_crypto: QuantumCryptoManager,
    organizations: HashMap<OrganizationId, Organization>,
}

impl IdentityManager {
    /// Create new identity manager
    pub fn new(storage: StorageManager) -> Self;
    
    /// Create new user identity
    pub async fn create_identity(&mut self, user_info: UserInfo) -> Result<EnhancedIdentity>;
    
    /// Load identity by user ID
    pub async fn load_identity(&self, user_id: &str) -> Result<EnhancedIdentity>;
    
    /// Update identity
    pub async fn update_identity(&mut self, identity: &EnhancedIdentity) -> Result<()>;
    
    /// Create organization
    pub async fn create_organization(&mut self, creator_id: &str, org_info: OrganizationInfo) -> Result<Organization>;
    
    /// Get organization
    pub async fn get_organization(&self, org_id: &OrganizationId) -> Result<Organization>;
    
    /// Add user to organization
    pub async fn add_organization_member(&mut self, org_id: &OrganizationId, user_id: &str, role: OrganizationRole) -> Result<()>;
    
    /// Remove user from organization
    pub async fn remove_organization_member(&mut self, org_id: &OrganizationId, user_id: &str) -> Result<()>;
    
    /// Create department
    pub async fn create_department(&mut self, org_id: &OrganizationId, dept_info: DepartmentInfo) -> Result<Department>;
    
    /// Create team
    pub async fn create_team(&mut self, dept_id: &DepartmentId, team_info: TeamInfo) -> Result<Team>;
    
    /// Check user permissions
    pub async fn check_permission(&self, user_id: &str, permission: &Permission, context: &PermissionContext) -> Result<bool>;
    
    /// Get user's organizations
    pub async fn get_user_organizations(&self, user_id: &str) -> Result<Vec<OrganizationMembership>>;
}
```

### Device Management

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistry {
    /// Registered devices
    pub devices: HashMap<String, DeviceInfo>,
    
    /// Primary device ID
    pub primary_device: Option<String>,
    
    /// Device sync settings
    pub sync_settings: DeviceSyncSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Unique device identifier
    pub device_id: String,
    
    /// Device name/description
    pub name: String,
    
    /// Device type
    pub device_type: DeviceType,
    
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
    
    /// Last seen timestamp
    pub last_seen: SystemTime,
    
    /// Device public key
    pub public_key: Vec<u8>,
    
    /// Registration timestamp
    pub registered_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Server,
    IoT,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Can store private keys
    pub secure_storage: bool,
    
    /// Has biometric authentication
    pub biometric_auth: bool,
    
    /// Supports push notifications
    pub push_notifications: bool,
    
    /// Can run in background
    pub background_processing: bool,
    
    /// Network capabilities
    pub network_capabilities: NetworkCapabilities,
}
```

---

## 💬 Collaboration Systems

### Chat System

```rust
pub struct ChatManager {
    storage: StorageManager,
    identity: EnhancedIdentity,
    channels: HashMap<String, Channel>,
    active_calls: HashMap<String, Call>,
}

impl ChatManager {
    /// Create new chat manager
    pub fn new(storage: StorageManager, identity: EnhancedIdentity) -> Self;
    
    /// Create a new channel
    pub async fn create_channel(&mut self, name: String, channel_type: ChannelType) -> Result<Channel>;
    
    /// Join a channel
    pub async fn join_channel(&mut self, channel_id: &str) -> Result<()>;
    
    /// Leave a channel
    pub async fn leave_channel(&mut self, channel_id: &str) -> Result<()>;
    
    /// Send message to channel
    pub async fn send_message(&mut self, channel_id: &str, content: String, message_type: MessageType) -> Result<Message>;
    
    /// Get channel messages
    pub async fn get_messages(&self, channel_id: &str, limit: Option<usize>) -> Result<Vec<Message>>;
    
    /// Start voice/video call
    pub async fn start_call(&mut self, channel_id: &str, call_type: CallType) -> Result<Call>;
    
    /// Join call
    pub async fn join_call(&mut self, call_id: &str) -> Result<()>;
    
    /// End call
    pub async fn end_call(&mut self, call_id: &str) -> Result<()>;
    
    /// Get channel list
    pub fn list_channels(&self) -> Vec<&Channel>;
    
    /// Search messages
    pub async fn search_messages(&self, query: &str, channel_id: Option<&str>) -> Result<Vec<Message>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub channel_type: ChannelType,
    pub members: Vec<ChannelMember>,
    pub settings: ChannelSettings,
    pub created_at: SystemTime,
    pub created_by: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelType {
    /// Direct message between two users
    DirectMessage,
    
    /// Group chat
    Group,
    
    /// Public channel
    Public,
    
    /// Organization channel
    Organization(OrganizationId),
    
    /// Project channel
    Project(ProjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub sender: String,
    pub content: String,
    pub message_type: MessageType,
    pub attachments: Vec<Attachment>,
    pub reactions: Vec<Reaction>,
    pub thread_id: Option<String>,
    pub reply_to: Option<String>,
    pub created_at: SystemTime,
    pub edited_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    Image,
    Video,
    Audio,
    File,
    System,
    Call,
}
```

### Voice/Video Calls

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub id: String,
    pub channel_id: String,
    pub call_type: CallType,
    pub participants: Vec<CallParticipant>,
    pub state: CallState,
    pub started_at: SystemTime,
    pub ended_at: Option<SystemTime>,
    pub settings: CallSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallType {
    Voice,
    Video,
    ScreenShare,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallState {
    Starting,
    Active,
    Paused,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallParticipant {
    pub user_id: String,
    pub joined_at: SystemTime,
    pub left_at: Option<SystemTime>,
    pub muted: bool,
    pub video_enabled: bool,
    pub screen_sharing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSettings {
    pub max_participants: Option<usize>,
    pub require_permission: bool,
    pub recording_enabled: bool,
    pub quality: CallQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallQuality {
    Low,
    Medium,
    High,
    Auto,
}
```

### Discussion System

```rust
pub struct DiscussManager {
    storage: StorageManager,
    identity: EnhancedIdentity,
}

impl DiscussManager {
    /// Create new discussion manager
    pub fn new(storage: StorageManager, identity: EnhancedIdentity) -> Self;
    
    /// Create discussion topic
    pub async fn create_topic(&mut self, title: String, content: String, category: Option<String>) -> Result<Topic>;
    
    /// Reply to topic
    pub async fn reply_to_topic(&mut self, topic_id: &str, content: String) -> Result<Reply>;
    
    /// Vote on topic
    pub async fn vote_on_topic(&mut self, topic_id: &str, vote_type: VoteType) -> Result<()>;
    
    /// Create poll
    pub async fn create_poll(&mut self, question: String, options: Vec<String>, settings: PollSettings) -> Result<Poll>;
    
    /// Vote in poll
    pub async fn vote_in_poll(&mut self, poll_id: &str, option_id: usize) -> Result<()>;
    
    /// Award badge
    pub async fn award_badge(&mut self, user_id: &str, badge_type: BadgeType, reason: String) -> Result<Badge>;
    
    /// Get topic with replies
    pub async fn get_topic(&self, topic_id: &str) -> Result<(Topic, Vec<Reply>)>;
    
    /// List topics
    pub async fn list_topics(&self, category: Option<&str>, limit: Option<usize>) -> Result<Vec<Topic>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub votes: i32,
    pub reply_count: u32,
    pub view_count: u32,
    pub created_at: SystemTime,
    pub last_activity: SystemTime,
    pub pinned: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub id: String,
    pub topic_id: String,
    pub content: String,
    pub author: String,
    pub parent_id: Option<String>,
    pub votes: i32,
    pub created_at: SystemTime,
    pub edited_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poll {
    pub id: String,
    pub question: String,
    pub options: Vec<PollOption>,
    pub settings: PollSettings,
    pub created_by: String,
    pub created_at: SystemTime,
    pub ends_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollOption {
    pub id: usize,
    pub text: String,
    pub votes: u32,
}
```

---

## ⚙️ Configuration

### Node Configuration

```rust
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Network configuration
    pub network: NetworkConfig,
    
    /// DHT configuration
    pub dht: DHTConfig,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// MCP server configuration
    pub mcp: Option<MCPServerConfig>,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Tunneling configuration
    pub tunneling: Option<TunnelConfig>,
    
    /// Identity configuration
    pub identity: IdentityConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            dht: DHTConfig::default(),
            security: SecurityConfig::default(),
            mcp: Some(MCPServerConfig::default()),
            storage: StorageConfig::default(),
            tunneling: None,
            identity: IdentityConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}
```

### Network Configuration

```rust
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Listening addresses
    pub listen_addrs: Vec<String>,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<(PeerId, Vec<Multiaddr>)>,
    
    /// Transport selection
    pub transport_selection: TransportSelection,
    
    /// Maximum connections
    pub max_connections: Option<usize>,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    
    /// Enable IPv6
    pub enable_ipv6: bool,
    
    /// Enable UPnP
    pub enable_upnp: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addrs: vec![
                "/ip6/::1/udp/0/quic".to_string(),
                "/ip4/127.0.0.1/udp/0/quic".to_string(),
            ],
            bootstrap_nodes: vec![],
            transport_selection: TransportSelection::Auto,
            max_connections: Some(1000),
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(60),
            enable_ipv6: true,
            enable_upnp: false,
        }
    }
}
```

### DHT Configuration

```rust
#[derive(Debug, Clone)]
pub struct DHTConfig {
    /// Replication factor
    pub replication_factor: usize,
    
    /// Record TTL
    pub record_ttl: Duration,
    
    /// Query timeout
    pub query_timeout: Duration,
    
    /// Maximum number of records to store
    pub max_records: Option<usize>,
    
    /// Enable S/Kademlia security extensions
    pub enable_security_extensions: bool,
    
    /// Provider record TTL
    pub provider_record_ttl: Duration,
    
    /// Publication interval
    pub publication_interval: Duration,
    
    /// Enable periodic republishing
    pub enable_republishing: bool,
}

impl Default for DHTConfig {
    fn default() -> Self {
        Self {
            replication_factor: 20,
            record_ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
            query_timeout: Duration::from_secs(10),
            max_records: Some(10000),
            enable_security_extensions: true,
            provider_record_ttl: Duration::from_secs(2 * 60 * 60), // 2 hours
            publication_interval: Duration::from_secs(60 * 60), // 1 hour
            enable_republishing: true,
        }
    }
}
```

### Security Configuration

```rust
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable quantum-resistant cryptography
    pub enable_quantum_crypto: bool,
    
    /// Force hybrid mode (classical + post-quantum)
    pub force_hybrid_mode: bool,
    
    /// Minimum security level
    pub min_security_level: SecurityLevel,
    
    /// Enable threshold signatures
    pub enable_threshold_signatures: bool,
    
    /// Default trust level for new peers
    pub default_trust_level: TrustLevel,
    
    /// Enable identity verification
    pub require_identity_verification: bool,
    
    /// Certificate validation settings
    pub certificate_validation: CertificateValidation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrustLevel {
    None,
    Basic,
    Full,
}

#[derive(Debug, Clone)]
pub struct CertificateValidation {
    pub verify_signatures: bool,
    pub check_expiration: bool,
    pub require_chain_validation: bool,
}
```

### Feature Flags

```rust
/// Feature flags that can be enabled at compile time
pub mod features {
    /// DHT support enabled
    pub const DHT: bool = cfg!(feature = "dht");
    
    /// MCP support enabled
    pub const MCP: bool = cfg!(feature = "mcp");
    
    /// IPv6 tunneling enabled
    pub const TUNNELING: bool = cfg!(feature = "tunneling");
    
    /// Three-word addresses enabled
    pub const THREE_WORD_ADDRESSES: bool = cfg!(feature = "three-word-addresses");
    
    /// Quantum-resistant cryptography enabled
    pub const QUANTUM_RESISTANT: bool = cfg!(feature = "quantum-resistant");
    
    /// Threshold cryptography enabled
    pub const THRESHOLD: bool = cfg!(feature = "threshold");
    
    /// CLI tools enabled
    pub const CLI: bool = cfg!(feature = "cli");
    
    /// Commercial license features
    pub const COMMERCIAL: bool = cfg!(feature = "commercial");
}

/// Runtime feature detection
pub struct RuntimeFeatures {
    // Features detected at runtime
}

impl RuntimeFeatures {
    pub fn detect() -> Self;
    pub fn has_quic_support(&self) -> bool;
    pub fn has_ipv6_support(&self) -> bool;
    pub fn has_upnp_support(&self) -> bool;
    pub fn quantum_crypto_available(&self) -> bool;
}
```

---

## ⚠️ Error Handling

### Main Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    /// Network-related errors
    #[error("Network error: {0}")]
    Network(String),
    
    /// DHT operation errors
    #[error("DHT error: {0}")]
    DHT(String),
    
    /// Transport layer errors
    #[error("Transport error: {0}")]
    Transport(String),
    
    /// Security and cryptography errors
    #[error("Security error: {0}")]
    Security(String),
    
    /// MCP protocol errors
    #[error("MCP error: {0}")]
    MCP(String),
    
    /// Identity management errors
    #[error("Identity error: {0}")]
    Identity(String),
    
    /// Storage system errors
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Three-word address errors
    #[error("Three-word address error: {0}")]
    ThreeWordAddress(String),
    
    /// Tunneling errors
    #[error("Tunneling error: {0}")]
    Tunneling(String),
    
    /// Threshold cryptography errors
    #[error("Threshold error: {0}")]
    Threshold(String),
    
    /// Quantum cryptography errors
    #[error("Quantum crypto error: {0}")]
    QuantumCrypto(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Other errors
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for P2P operations
pub type Result<T> = std::result::Result<T, P2PError>;
```

### Specific Error Types

```rust
/// Storage-specific errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    #[error("DHT operation failed: {0}")]
    DHTOperationFailed(String),
    
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
}

/// Quantum cryptography errors
#[derive(Debug, thiserror::Error)]
pub enum QuantumCryptoError {
    #[error("Invalid key: {0}")]
    InvalidKeyError(String),
    
    #[error("ML-KEM error: {0}")]
    MlKemError(String),
    
    #[error("ML-DSA error: {0}")]
    MlDsaError(String),
    
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    
    #[error("Algorithm not supported: {0}")]
    UnsupportedAlgorithm(String),
}

/// Threshold cryptography errors
#[derive(Debug, thiserror::Error)]
pub enum ThresholdError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Invalid share: {0}")]
    InvalidShare(String),
    
    #[error("Insufficient shares")]
    InsufficientShares,
    
    #[error("Group operation failed: {0}")]
    GroupOperationFailed(String),
    
    #[error("Aggregation failed: {0}")]
    AggregationFailed(String),
}

/// Project management errors
#[derive(Debug, thiserror::Error)]
pub enum ProjectsError {
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
    
    #[error("Document not found: {0}")]
    DocumentNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[error("Workflow error: {0}")]
    WorkflowError(String),
}
```

### Error Context and Recovery

```rust
/// Error context for better debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub peer_id: Option<PeerId>,
    pub timestamp: SystemTime,
    pub additional_info: HashMap<String, String>,
}

impl P2PError {
    /// Add context to error
    pub fn with_context(self, context: ErrorContext) -> P2PErrorWithContext;
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool;
    
    /// Get error category
    pub fn category(&self) -> ErrorCategory;
}

#[derive(Debug)]
pub struct P2PErrorWithContext {
    pub error: P2PError,
    pub context: ErrorContext,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    Network,
    Storage,
    Cryptography,
    Configuration,
    UserInput,
    System,
}
```

---

## 🎯 Examples

### Example 1: Basic P2P Node

```rust
use saorsa_core::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();
    
    // Create node configuration
    let config = NodeConfig {
        network: NetworkConfig {
            listen_addrs: vec![
                "/ip6/::1/udp/0/quic".to_string(),
                "/ip4/127.0.0.1/tcp/0".to_string(),
            ],
            ..Default::default()
        },
        dht: DHTConfig {
            replication_factor: 20,
            ..Default::default()
        },
        mcp: Some(MCPServerConfig::default()),
        ..Default::default()
    };
    
    // Create and start node
    let mut node = P2PNode::new(config).await?;
    node.start().await?;
    
    // Get three-word address
    let encoder = WordEncoder::new();
    let addresses = node.listen_addrs().await?;
    if let Some(addr) = addresses.first() {
        let words = encoder.encode_multiaddr(addr)?;
        println!("🐜 Connect to: {}", words);
    }
    
    // Set up event handling
    node.on_event(|event| {
        match event {
            P2PEvent::PeerConnected { peer_id, multiaddr, .. } => {
                println!("📡 Peer connected: {} at {}", peer_id, multiaddr);
            }
            P2PEvent::DHTRecord { key, operation, .. } => {
                println!("🗂️ DHT {}: {}", operation, key.to_hex());
            }
            _ => {}
        }
    });
    
    // Keep running
    println!("✅ Node running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    
    // Graceful shutdown
    node.shutdown().await?;
    println!("👋 Node stopped");
    
    Ok(())
}
```

### Example 2: Chat Application

```rust
use saorsa_core::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create user identity
    let user_info = UserInfo {
        display_name: "Alice".to_string(),
        email: Some("alice@example.com".to_string()),
        avatar: None,
    };
    
    let identity = EnhancedIdentity::new(user_info).await?;
    
    // Create node with identity
    let mut node = P2PNode::builder()
        .listen_on("/ip6/::1/udp/0/quic")
        .with_identity(identity.clone())
        .with_dht(DHTConfig::default())
        .build()
        .await?;
    
    node.start().await?;
    
    // Create storage and chat manager
    let dht = node.dht().await?;
    let storage = StorageManager::new(dht, &identity)?;
    let mut chat = ChatManager::new(storage, identity);
    
    // Create a channel
    let channel = chat.create_channel(
        "general".to_string(),
        ChannelType::Public,
    ).await?;
    
    println!("📢 Created channel: {}", channel.name);
    
    // Send a message
    let message = chat.send_message(
        &channel.id,
        "Hello, P2P world!".to_string(),
        MessageType::Text,
    ).await?;
    
    println!("💬 Sent message: {}", message.content);
    
    // Get messages
    let messages = chat.get_messages(&channel.id, Some(10)).await?;
    for msg in messages {
        println!("📨 {}: {}", msg.sender, msg.content);
    }
    
    node.shutdown().await?;
    Ok(())
}
```

### Example 3: Threshold Signatures

```rust
use saorsa_core::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Generate identities for participants
    let mut participants = Vec::new();
    for i in 0..5 {
        let user_info = UserInfo {
            display_name: format!("User{}", i),
            email: None,
            avatar: None,
        };
        let identity = EnhancedIdentity::new(user_info).await?;
        participants.push(identity);
    }
    
    // Create threshold group (3-of-5)
    let threshold = 3;
    let group_config = GroupConfig {
        threshold,
        participants: participants.len() as u16,
        name: "Example Group".to_string(),
        description: "Demo threshold group".to_string(),
    };
    
    let mut group_manager = ThresholdGroupManager::new(
        participants[0].quantum_identity.clone()
    );
    
    let group = group_manager.create_group(group_config).await?;
    println!("👥 Created threshold group: {}", group.group_id);
    
    // Simulate signing a message
    let message = b"Important document to sign";
    
    // Initiate signing
    let session_id = group_manager.initiate_signing(&group.group_id, message).await?;
    println!("✍️ Started signing session: {:?}", session_id);
    
    // Each participant contributes (simulated)
    for i in 0..threshold {
        let contribution = group_manager.participate_in_signing(&session_id).await?;
        println!("📝 Participant {} contributed", i);
    }
    
    // Complete signing
    let signature = group_manager.complete_signing(&session_id).await?;
    println!("✅ Threshold signature created");
    
    // Verify signature
    group_manager.verify_threshold_signature(&group.group_id, message, &signature)?;
    println!("🔍 Signature verified successfully");
    
    Ok(())
}
```

### Example 4: IPv6 Tunneling

```rust
use saorsa_core::tunneling::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create tunnel manager
    let config = TunnelManagerConfig::default();
    let mut tunnel_manager = TunnelManager::new(config);
    tunnel_manager.start().await?;
    
    // Create 6to4 tunnel for automatic IPv6
    let tunnel_config = TunnelConfig::sixto4_auto();
    let tunnel_id = tunnel_manager.create_tunnel(
        "auto-6to4".to_string(),
        tunnel_config,
    ).await?;
    
    println!("🌐 Created 6to4 tunnel: {}", tunnel_id);
    
    // Create Teredo tunnel for NAT traversal
    let teredo_config = TunnelConfig::teredo_nat();
    let teredo_id = tunnel_manager.create_tunnel(
        "teredo-nat".to_string(),
        teredo_config,
    ).await?;
    
    println!("🔗 Created Teredo tunnel: {}", teredo_id);
    
    // List all tunnels
    let tunnels = tunnel_manager.list_tunnels();
    for (id, state, protocol) in tunnels {
        println!("📡 Tunnel {}: {:?} ({:?})", id, state, protocol);
    }
    
    // Subscribe to tunnel events
    let mut events = tunnel_manager.subscribe_events();
    
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            match event {
                TunnelEvent::TunnelEstablished { tunnel_id, protocol, .. } => {
                    println!("✅ Tunnel {} established ({:?})", tunnel_id, protocol);
                }
                TunnelEvent::TunnelFailed { tunnel_id, error, .. } => {
                    println!("❌ Tunnel {} failed: {}", tunnel_id, error);
                }
                _ => {}
            }
        }
    });
    
    // Simulate packet routing
    let destination = "2001:db8::1".parse()?;
    let packet = vec![0x60, 0x00, 0x00, 0x00]; // IPv6 header start
    
    tunnel_manager.route_packet(&packet, destination).await?;
    println!("📦 Packet routed to {}", destination);
    
    // Keep running
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    
    Ok(())
}
```

### Example 5: Project Management

```rust
use saorsa_core::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup identity and storage
    let identity = EnhancedIdentity::new(UserInfo {
        display_name: "Project Manager".to_string(),
        email: Some("pm@example.com".to_string()),
        avatar: None,
    }).await?;
    
    let node = P2PNode::builder()
        .with_identity(identity.clone())
        .build()
        .await?;
    
    let dht = node.dht().await?;
    let storage = StorageManager::new(dht, &identity)?;
    let mut projects = ProjectsManager::new(storage, identity.clone());
    
    // Create organization
    let org_id = OrganizationId::new();
    
    // Create a project
    let project = projects.create_project(
        "Awesome Project".to_string(),
        "Building the next generation P2P app".to_string(),
        org_id,
        None, // No department
        None, // No team
        GroupId::new(), // Owner group
    ).await?;
    
    println!("📁 Created project: {}", project.name);
    
    // Upload a document
    let content = b"# Project Specification\n\nThis is our amazing project...";
    let document = projects.upload_document(
        project.id.clone(),
        project.root_folder.clone(),
        "specification.md".to_string(),
        "Project specification document".to_string(),
        content,
        DocumentType::Text { format: "markdown".to_string() },
    ).await?;
    
    println!("📄 Uploaded document: {}", document.name);
    
    // Create a new version
    let updated_content = b"# Project Specification v2\n\nUpdated with new requirements...";
    let new_version = projects.create_document_version(
        &document.id,
        updated_content,
        "Added new requirements".to_string(),
    ).await?;
    
    println!("📝 Created version {}: {}", new_version.version_number, new_version.comment);
    
    // Download the document
    let downloaded = projects.download_document(&document.id).await?;
    println!("⬇️ Downloaded {} bytes", downloaded.len());
    
    // Approve the document (if approval workflow enabled)
    projects.approve_document(&document.id, Some("Looks good!".to_string())).await?;
    println!("✅ Document approved");
    
    Ok(())
}
```

### Example 6: MCP Tool Integration

```rust
use saorsa_core::mcp::*;
use serde_json::json;

// Custom MCP tool for network operations
struct NetworkTool {
    node: Arc<P2PNode>,
}

#[async_trait]
impl ToolHandler for NetworkTool {
    async fn execute(&self, args: Value, _context: MCPCallContext) -> Result<Value> {
        let command = args["command"].as_str().unwrap_or("status");
        
        match command {
            "status" => {
                let stats = self.node.stats().await;
                Ok(json!({
                    "peer_count": stats.peer_count,
                    "connections": stats.active_connections,
                    "uptime": stats.uptime.as_secs(),
                    "bytes_sent": stats.bytes_sent,
                    "bytes_received": stats.bytes_received
                }))
            }
            "peers" => {
                let peers = self.node.list_peers().await;
                Ok(json!({
                    "peers": peers.iter().map(|p| {
                        json!({
                            "peer_id": p.peer_id,
                            "addresses": p.addresses,
                            "protocols": p.protocols
                        })
                    }).collect::<Vec<_>>()
                }))
            }
            _ => Err(P2PError::MCP("Unknown command".to_string()))
        }
    }
    
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            category: "Network".to_string(),
            tags: vec!["p2p".to_string(), "network".to_string()],
            version: "1.0.0".to_string(),
            author: Some("P2P Foundation".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create P2P node
    let node = Arc::new(P2PNode::builder()
        .with_mcp_server()
        .build()
        .await?);
    
    // Create and start MCP server
    let config = MCPServerConfig::default();
    let mut mcp_server = MCPServer::new(config);
    
    // Register network tool
    let network_tool = Tool::new(
        "network".to_string(),
        "P2P network operations and status".to_string(),
        NetworkTool { node: node.clone() },
    );
    
    mcp_server.register_tool(network_tool).await?;
    
    // Register built-in tools
    mcp_server.register_tool(DHTTool::new()).await?;
    mcp_server.register_tool(ThreeWordTool::new()).await?;
    mcp_server.register_tool(ChatTool::new()).await?;
    
    // Start services
    node.start().await?;
    mcp_server.start().await?;
    
    println!("🤖 MCP server running with P2P tools");
    println!("🔧 Available tools:");
    for tool in mcp_server.list_tools() {
        println!("  - {}: {}", tool.name, tool.description);
    }
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    
    mcp_server.stop().await?;
    node.shutdown().await?;
    
    Ok(())
}
```

---

## 🎉 Real-World Example: Saorsa App

The [Saorsa app](https://github.com/dirvine/p2p/tree/main/apps/desktop-tauri) demonstrates the P2P Foundation library in action:

### Features Implemented
- **Three-Word Addressing**: Connect using human-friendly addresses
- **Quantum-Resistant Security**: ML-KEM and ML-DSA cryptography
- **Real-time Chat**: Secure P2P messaging
- **Project Collaboration**: Document sharing and workflows
- **MCP Integration**: AI assistant capabilities
- **IPv6 Tunneling**: Automatic connectivity
- **Threshold Groups**: Multi-signature workflows

### Architecture
```
Saorsa Desktop App (Tauri)
├── Frontend (JavaScript/HTML/CSS)
│   ├── Chat Interface
│   ├── Project Management
│   ├── Settings UI
│   └── AI Assistant
└── Backend (Rust + ant-core)
    ├── P2P Node
    ├── Identity Manager
    ├── Storage Layer
    ├── MCP Server
    └── UI Bridge
```

### Getting Started with Saorsa
```bash
# Clone repository
git clone https://github.com/dirvine/p2p.git
cd p2p/apps/desktop-tauri

# Install dependencies
npm install

# Build and run
npm run tauri dev
```

The Saorsa app showcases how the P2P Foundation enables building sophisticated collaborative applications with minimal complexity, providing secure, decentralized communication and file sharing out of the box.

---

## 📜 License

This library is dual-licensed:

- **AGPL-3.0-or-later**: Free for open source projects
- **Commercial License**: Available for proprietary applications

See [LICENSING.md](./LICENSING.md) for details.

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](./CONTRIBUTING.md) for details.

## 📞 Support

- **Documentation**: [https://docs.p2p-foundation.org](https://docs.p2p-foundation.org)
- **Issues**: [GitHub Issues](https://github.com/dirvine/p2p/issues)
- **Discussions**: [GitHub Discussions](https://github.com/dirvine/p2p/discussions)
- **Email**: support@p2p-foundation.org

---

*🕊️ Built with Saorsa - Connecting the world, one peer at a time.*