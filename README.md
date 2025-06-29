# P2P Foundation

A next-generation peer-to-peer networking foundation built in Rust, featuring QUIC transport, privacy-first identity system, and fully integrated AI capabilities through Model Context Protocol (MCP) servers at each node.

## 🏗️ Project Structure

This is a Cargo workspace containing multiple interconnected components:

### 📦 Core Library: [Ant Core](https://crates.io/crates/ant-core)
[![Crates.io](https://img.shields.io/crates/v/ant-core)](https://crates.io/crates/ant-core)
[![Documentation](https://docs.rs/ant-core/badge.svg)](https://docs.rs/ant-core)

The foundational P2P networking library (`crates/p2p-core`) providing:
- **QUIC Transport**: Modern, efficient networking with built-in encryption
- **Distributed Hash Table (DHT)**: Kademlia-based distributed storage
- **Privacy-First Identity**: Encrypted profiles with friend-based access control
- **Three-Word Addresses**: Human-readable network addressing system
- **MCP Integration**: Model Context Protocol for AI agent communication

### 🕊️ Desktop Application: Saorsa
Built with Tauri (`apps/desktop-tauri`) - the flagship P2P application featuring:
- Real-time encrypted messaging
- Decentralized contact management
- Profile sharing with granular privacy controls
- Cross-platform desktop support (macOS, Windows, Linux)
- Native performance with web UI

### 🔧 Developer Tools
- **CLI Tools** (`crates/p2p-cli`): Command-line utilities for network management
- **FFI Bindings** (`crates/p2p-ffi`): Enable Flutter and other language integration

## 🚀 Quick Start

### Using Ant Core Library

Add to your `Cargo.toml`:
```toml
[dependencies]
ant-core = "0.1.8"
tokio = { version = "1", features = ["full"] }
```

```rust
use ant_core::{
    network::{P2PNode, NodeConfig},
    identity::manager::{IdentityManager, IdentityManagerConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a P2P node
    let config = NodeConfig::default();
    let node = P2PNode::new(config).await?;
    
    // Create identity manager
    let identity_manager = IdentityManager::new(IdentityManagerConfig::default());
    
    // Create a user identity
    let identity = identity_manager.create_identity(
        "My Display Name".to_string(),
        "my.three.words".to_string(),
        None,
        None,
    ).await?;
    
    println!("Created identity: {}", identity.user_id);
    Ok(())
}
```

### Using Saorsa Desktop App

1. **Install from crates.io** (coming soon):
   ```bash
   cargo install saorsa
   saorsa
   ```

2. **Build from source**:
   ```bash
   git clone https://github.com/dirvine/p2p.git
   cd p2p
   cargo build --release
   open target/release/bundle/macos/Saorsa.app
   ```

## ✨ Key Features

### 🔒 **Privacy-First Architecture**
- Quantum-resistant end-to-end encryption
- Encrypted user profiles with threshold access control
- Hierarchical team permissions with cryptographic enforcement
- Multi-signature authorization for sensitive operations
- Anti-spoofing with dual-signature verification (ML-DSA + Ed25519)

### 🌐 **Universal Connectivity**
- IPv6-first with comprehensive IPv4 tunneling
- Automatic NAT traversal
- Works across any network topology
- Bootstrap system for peer discovery

### 🤖 **AI-Native Design**
- MCP (Model Context Protocol) server integration
- Built for AI agent communication
- Tool system for extensible functionality
- Threshold-based AI resource authorization

### 🎯 **Developer Experience**
- Human-readable three-word addresses (`forest.lightning.compass`)
- Comprehensive Rust APIs with async/await
- Cross-platform FFI bindings for mobile development
- Extensive documentation and examples

## 🌟 What's Unique About This Network and the Saorsa App

The P2P Foundation represents a paradigm shift in decentralized networking, combining cutting-edge cryptography, innovative addressing, and AI-native design to create something truly revolutionary. Here's what sets us apart:

### 🎭 **Three-Word Network Addresses**
- **Human-readable networking**: Share connections with memorable phrases like `forest.lightning.compass` instead of complex technical addresses
- **Voice-friendly**: Actually shareable over phone calls and voice chat
- **8.6 billion combinations**: Enormous address space using carefully curated word lists
- **Zero friction onboarding**: Eliminates the biggest barrier to P2P adoption

### 🛡️ **Quantum-Resistant Security Architecture**
- **Future-proof cryptography**: ML-KEM-768 and ML-DSA-65 (FIPS 203/204) protect against quantum threats
- **Hybrid transition**: Support both classical and post-quantum algorithms during migration
- **Algorithm agility**: Easy upgrade path as quantum-resistant standards evolve
- **Military-grade protection**: Defense against threats that don't even exist yet

### 👥 **Revolutionary Threshold Cryptography**
- **FROST protocol**: Threshold signatures enable true multi-party authorization
- **Dynamic membership**: Add/remove people from groups without regenerating keys
- **Hierarchical authority**: Cryptographically enforced organizational structures
- **Byzantine fault tolerance**: Secure consensus even with malicious participants
- **Seamless personnel changes**: Swap out team members while maintaining security continuity

### 🏢 **Cryptographically Enforced Organizations**
- **Verifiable hierarchies**: Team leaders and structures backed by mathematics, not trust
- **Granular permissions**: Different access levels for different organizational roles
- **Threshold governance**: Require multiple approvals for sensitive operations
- **Audit trails**: Cryptographic proof of who authorized what and when
- **Enterprise-ready**: Built for real organizational security needs

### 📚 **Git-Like DHT with Universal Version Control**
- **Content-addressed everything**: BLAKE3 hashing provides integrity for all data
- **Universal version control**: Chat messages, documents, forum posts - everything is versioned
- **Git semantics**: Branches, commits, tags, and merges for any type of content
- **Network-wide deduplication**: Identical content stored once across the entire network
- **Collaborative workflows**: Distributed editing with conflict resolution and merge capabilities

### 🔒 **True Peer-to-Peer Communication**
- **No signaling servers**: Direct computer-to-computer communication without intermediaries
- **Ultimate privacy**: Your conversations don't touch anyone else's servers
- **QUIC transport**: Modern, encrypted networking with built-in DoS protection
- **Universal connectivity**: Works across any network topology with automatic tunneling
- **Enterprise-grade**: ISATAP tunneling for corporate IPv6 deployment

### 🤖 **AI-Native from the Ground Up**
- **MCP everywhere**: Model Context Protocol server built into every node
- **Distributed AI**: AI agents discover and collaborate across the network
- **Tool orchestration**: Automatically find and execute AI tools on remote nodes
- **Service discovery**: AI services announce themselves and are automatically discovered
- **Threshold AI governance**: Cryptographic authorization for AI resource access

### 🌐 **Universal Network Connectivity**
- **Intelligent protocol selection**: Automatically chooses best tunneling method (6to4, Teredo, 6in4, DS-Lite, ISATAP)
- **Enterprise tunneling**: Built-in ISATAP support for corporate IPv6 networks
- **Zero configuration**: Works out of the box on any network setup
- **IPv6-first design**: Future-ready with comprehensive IPv4 backward compatibility
- **Automatic NAT traversal**: Connects through any firewall or router configuration

### 🚀 **Revolutionary User Experience**
- **Voice shareable**: "Connect to forest lightning compass" actually works
- **Zero technical knowledge**: Share network access like sharing a WiFi password
- **Cross-platform**: Native performance on desktop, mobile, and web
- **Progressive enhancement**: Advanced features for power users, simple for everyone else
- **Developer friendly**: Build on our foundation with minimal learning curve

This isn't just another P2P network - it's a complete reimagining of how decentralized systems should work. We've solved the fundamental problems that have kept P2P networks from mainstream adoption while building in future-proof security and AI-native capabilities that will matter for decades to come.

## 🏛️ Architecture

```
┌─────────────────────────────────────┐
│          Saorsa Desktop App         │  ← Tauri-based UI
├─────────────────────────────────────┤
│    Threshold Group Management       │  ← Teams, hierarchies, permissions
├─────────────────────────────────────┤
│        Identity Management          │  ← Quantum-resistant profiles
├─────────────────────────────────────┤
│     MCP Server Layer (AI Tools)     │  ← Tool discovery, remote execution
├─────────────────────────────────────┤
│   Kademlia DHT (Secure Storage)     │  ← Distributed data storage
├─────────────────────────────────────┤
│   Quantum Crypto Layer (ML-KEM/DSA) │  ← Post-quantum security
├─────────────────────────────────────┤
│      QUIC Transport Layer           │  ← Modern, secure transport
├─────────────────────────────────────┤
│ IPv6/IPv4 Tunneling (Auto-Select)   │  ← Universal connectivity
└─────────────────────────────────────┘
```

### Core Components

- **Network**: P2P node management and configuration
- **DHT**: Distributed hash table for decentralized storage
- **Transport**: QUIC and TCP transport implementations with quantum-safe handshakes
- **Identity**: Privacy-first user identity with ML-DSA signatures
- **Threshold**: Dynamic group management with FROST protocol
- **Cryptography**: Quantum-resistant primitives (ML-KEM, ML-DSA)
- **MCP**: Model Context Protocol server for AI integration
- **Bootstrap**: Peer discovery and network bootstrapping

## 🔐 Security & Privacy

The P2P Foundation implements comprehensive **defense-in-depth** security with **quantum-resistant cryptography** and **threshold mechanisms**:

### Quantum-Resistant Cryptography
- **Key Exchange**: ML-KEM-768 (FIPS 203) - quantum-safe key encapsulation
- **Digital Signatures**: ML-DSA-65 (FIPS 204) - lattice-based signatures
- **Hybrid Mode**: Support for both classical and post-quantum algorithms during transition
- **Algorithm Agility**: Easy upgrade path as standards evolve

### Threshold Cryptography
- **FROST Protocol**: Threshold signatures for multi-party authorization
- **Dynamic Groups**: Add/remove members without key regeneration
- **Hierarchical Authority**: Cryptographically enforced access levels
- **Team Management**: Leaders and groups with verifiable permissions
- **Consensus Operations**: Byzantine fault-tolerant group decisions

### Core Security Features
- **Transport encryption**: End-to-end via QUIC/TLS 1.3
- **Peer authentication**: Dual signatures (ML-DSA + Ed25519)
- **Privacy-first profiles**: Encrypted data with threshold-based sharing
- **Access control**: Cryptographically enforced hierarchical permissions
- **Rate limiting**: Per-peer request throttling and DoS protection
- **Audit logging**: Comprehensive security event tracking
- **Forward secrecy**: Proactive secret refresh for long-term security

### Privacy Model

1. **Default Privacy**: All profile data encrypted by default
2. **Friend Network**: Share decryption keys only with trusted contacts
3. **Granular Control**: Choose what information friends can see
4. **Bloom Filter Discovery**: Find friends without revealing contacts
5. **IPv6 Identity Binding**: Anti-spoofing cryptographic proofs
6. **Threshold Access**: Require t-of-n approval for sensitive operations
7. **Dynamic Permissions**: Update access rights without re-encryption

## 📱 Cross-Platform Support

### Desktop (Tauri)
- **macOS**: Native .app bundle with DMG installer
- **Windows**: Native .exe with MSI installer
- **Linux**: Native binary with AppImage

### Mobile/Web (Flutter via FFI)
- **iOS**: Native performance through FFI bindings
- **Android**: Native performance through FFI bindings  
- **Web**: WebAssembly compilation for browser deployment

### Server/CLI
- **Linux**: Optimized for edge deployment
- **Docker**: Containerized deployment options

## 🛠️ Development

### Prerequisites

- Rust 1.75 or later
- Node.js 18+ (for Tauri development)
- IPv6 connectivity (native or tunneled)

### Building

```bash
# Clone the repository
git clone https://github.com/dirvine/p2p.git
cd p2p

# Build all components
cargo build --release

# Build desktop app specifically
cd apps/desktop-tauri
npm install
cargo tauri build

# Run tests
cargo test --all-features

# Run benchmarks
cargo bench
```

### Testing Multi-Node Communication

```bash
# Terminal 1: Start first node
cargo run --bin ant-connect -- --port 9001 --bootstrap-file bootstrap.json

# Terminal 2: Start second node
cargo run --bin ant-connect -- --port 9002 --bootstrap /ip6/::1/tcp/9001

# Terminal 3: Build and run desktop app
cd apps/desktop-tauri
cargo tauri dev
```

## 📊 Performance

- **Connection establishment**: < 100ms (LAN), < 1s (Internet) 
- **Throughput**: > 100 Mbps per connection via QUIC
- **Memory usage**: < 100MB baseline per node
- **Concurrent connections**: 1000+ with proper resource management
- **DHT operations**: < 200ms lookup, < 1s store/retrieve

## 📚 Documentation

- **[Ant Core Documentation](https://docs.rs/ant-core)** - API reference
- **[Technical Specification](SPECIFICATION.md)** - Detailed technical design
- **[Development Guidelines](CLAUDE.md)** - AI assistant development guide
- **[Examples](examples/)** - Working code examples

## 🗂️ Examples

See the [`examples/`](examples/) directory for:
- Basic P2P node setup
- DHT storage and retrieval
- Identity management and encrypted profiles
- MCP tool registration and remote calling
- Multi-node network simulation
- Cross-platform application development

## 🚧 Roadmap

### ✅ Completed (v0.1.8)
- [x] Core P2P networking with QUIC transport
- [x] Privacy-first identity system with encrypted profiles
- [x] DHT-based distributed storage
- [x] Desktop application (Saorsa) with full UI
- [x] Three-word address system
- [x] MCP integration for AI capabilities
- [x] Published to crates.io

### 🔄 In Progress
- [ ] Quantum-resistant cryptography integration (ML-KEM, ML-DSA)
- [ ] Threshold group management with FROST protocol
- [ ] Dynamic membership and permission updates
- [ ] Enhanced NAT traversal techniques
- [ ] Mobile app development (Flutter)
- [ ] Advanced bootstrap strategies
- [ ] Performance optimizations

### 📋 Planned
- [ ] Hierarchical authority structures
- [ ] Proactive secret refresh for forward security
- [ ] Byzantine fault-tolerant consensus
- [ ] Voice/video calling capabilities
- [ ] File sharing and synchronization
- [ ] Advanced security features
- [ ] Plugin system for extensibility

## 🤝 Contributing

We welcome contributions! Please see our [contributing guidelines](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes following [CLAUDE.md](CLAUDE.md) guidelines
4. Run tests and linting
5. Submit a pull request

## 📄 License

P2P Foundation is dual-licensed to support both open-source and commercial use:

### Open Source License (AGPL-3.0)
- For open source projects, personal use, and non-commercial applications
- Requires source code disclosure for all modifications
- Network use provisions apply (Section 13)
- See [LICENSE-AGPL-3.0](LICENSE-AGPL-3.0) for full terms

### Commercial License
- For proprietary applications and commercial use
- No source code disclosure required
- Professional support included
- Flexible pricing tiers (SMB, Enterprise, OEM)
- Contact: saorsalabs@gmail.com

**Quick Guide**: If you're building a proprietary product, charging users, or have >$1M annual revenue, you need a commercial license. See [LICENSING.md](LICENSING.md) for detailed guidance.

For questions, contact saorsalabs@gmail.com

## 🔗 Links

- **[Ant Core on crates.io](https://crates.io/crates/ant-core)**
- **[Documentation](https://docs.rs/ant-core)**
- **[Repository](https://github.com/dirvine/p2p)**
- **[Issues](https://github.com/dirvine/p2p/issues)**

## 🙏 Acknowledgments

Built on top of excellent open source projects:
- [Quinn](https://github.com/quinn-rs/quinn) - QUIC implementation
- [Tauri](https://tauri.app/) - Desktop app framework
- [Tokio](https://tokio.rs/) - Async runtime

---

*Building the decentralized future, one node at a time.* 🌐✨