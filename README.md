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
- End-to-end encryption by default
- Encrypted user profiles stored in DHT
- Friend-based access control with granular permissions
- Anti-spoofing with cryptographic verification

### 🌐 **Universal Connectivity**
- IPv6-first with comprehensive IPv4 tunneling
- Automatic NAT traversal
- Works across any network topology
- Bootstrap system for peer discovery

### 🤖 **AI-Native Design**
- MCP (Model Context Protocol) server integration
- Built for AI agent communication
- Tool system for extensible functionality

### 🎯 **Developer Experience**
- Human-readable three-word addresses (`forest.lightning.compass`)
- Comprehensive Rust APIs with async/await
- Cross-platform FFI bindings for mobile development
- Extensive documentation and examples

## 🏛️ Architecture

```
┌─────────────────────────────────────┐
│          Saorsa Desktop App         │  ← Tauri-based UI
├─────────────────────────────────────┤
│        Identity Management          │  ← Encrypted profiles, friends
├─────────────────────────────────────┤
│     MCP Server Layer (AI Tools)     │  ← Tool discovery, remote execution
├─────────────────────────────────────┤
│   Kademlia DHT (Secure Storage)     │  ← Distributed data storage
├─────────────────────────────────────┤
│      QUIC Transport Layer           │  ← Modern, secure transport
├─────────────────────────────────────┤
│ IPv6/IPv4 Tunneling (Auto-Select)   │  ← Universal connectivity
└─────────────────────────────────────┘
```

### Core Components

- **Network**: P2P node management and configuration
- **DHT**: Distributed hash table for decentralized storage
- **Transport**: QUIC and TCP transport implementations
- **Identity**: Privacy-first user identity and profile management
- **MCP**: Model Context Protocol server for AI integration
- **Bootstrap**: Peer discovery and network bootstrapping

## 🔐 Security & Privacy

The P2P Foundation implements comprehensive **defense-in-depth** security:

- **Transport encryption**: End-to-end via QUIC/TLS 1.3
- **Peer authentication**: Ed25519 cryptographic identities
- **Privacy-first profiles**: Encrypted data with friend-based sharing
- **Access control**: Fine-grained capability-based permissions
- **Rate limiting**: Per-peer request throttling and DoS protection
- **Audit logging**: Comprehensive security event tracking

### Privacy Model

1. **Default Privacy**: All profile data encrypted by default
2. **Friend Network**: Share decryption keys only with trusted contacts
3. **Granular Control**: Choose what information friends can see
4. **Bloom Filter Discovery**: Find friends without revealing contacts
5. **IPv6 Identity Binding**: Anti-spoofing cryptographic proofs

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
- [ ] Enhanced NAT traversal techniques
- [ ] Mobile app development (Flutter)
- [ ] Advanced bootstrap strategies
- [ ] Performance optimizations

### 📋 Planned
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

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

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