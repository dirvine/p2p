# P2P Foundation - Release Notes v0.1.8

## 🎉 Major Milestone: First Public Release

We're excited to announce the first public release of the P2P Foundation! This release includes the **Ant Core** library published to crates.io and the **Saorsa** desktop application.

## 📦 What's New

### 🐜 Ant Core Library (Published to crates.io)
[![Crates.io](https://img.shields.io/crates/v/ant-core)](https://crates.io/crates/ant-core)

The foundational P2P networking library is now available for developers:

```bash
cargo add ant-core
```

**Key Features:**
- **QUIC Transport**: Modern, efficient networking with built-in encryption
- **Distributed Hash Table (DHT)**: Kademlia-based distributed storage
- **Privacy-First Identity**: Encrypted profiles with friend-based access control
- **Three-Word Addresses**: Human-readable network addressing (`forest.lightning.compass`)
- **MCP Integration**: Model Context Protocol for AI agent communication

### 🕊️ Saorsa Desktop Application
**The flagship P2P messaging application built with Tauri**

**Available for macOS:**
- Native .app bundle: `assets/desktop/macos/Saorsa.app`
- DMG installer: `assets/desktop/macos/Saorsa_0.1.8_aarch64.dmg` (4.9MB)

**Features:**
- ✅ Decentralized messaging (no central servers)
- ✅ Privacy-first encrypted profiles
- ✅ Friend-based contact system
- ✅ Three-word address sharing
- ✅ Modern chat interface with emoji support
- ✅ Profile management with granular privacy controls
- ✅ Contact discovery via DHT
- ✅ Real-time P2P communication

## 🏗️ Architecture Highlights

### Privacy-First Design
- **Default Encryption**: All profile data encrypted by default
- **Friend Network**: Share decryption keys only with trusted contacts
- **Granular Control**: Choose what information friends can see
- **Bloom Filter Discovery**: Find friends without revealing contacts
- **IPv6 Identity Binding**: Anti-spoofing cryptographic proofs

### Technical Innovation
- **QUIC-First Transport**: Low latency, reliable connections
- **Kademlia DHT**: Distributed storage for profiles and discovery
- **Ed25519 Cryptography**: Strong authentication and signing
- **AES-256-GCM Encryption**: Industry-standard profile encryption
- **Challenge-Response Auth**: Prevent replay attacks

## 🚀 Installation & Usage

### For Developers (Ant Core)
```rust
use ant_core::{
    network::{P2PNode, NodeConfig},
    identity::manager::{IdentityManager, IdentityManagerConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NodeConfig::default();
    let node = P2PNode::new(config).await?;
    
    let identity_manager = IdentityManager::new(IdentityManagerConfig::default());
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

### For End Users (Saorsa)
1. Download: `assets/desktop/macos/Saorsa_0.1.8_aarch64.dmg`
2. Install: Drag to Applications folder
3. Launch: Open from Applications or Spotlight
4. Create profile and start connecting with friends!

## 📊 Technical Specifications

### Performance
- **Connection establishment**: < 100ms (LAN), < 1s (Internet)
- **Memory usage**: < 100MB baseline per node
- **DHT operations**: < 200ms lookup, < 1s store/retrieve
- **Profile encryption**: AES-256-GCM with unique nonces

### Security
- **Transport encryption**: End-to-end via QUIC/TLS 1.3
- **Peer authentication**: Ed25519 cryptographic identities
- **Profile privacy**: Encrypted data with friend-based sharing
- **Access control**: Fine-grained capability-based permissions
- **Anti-spoofing**: IPv6 identity binding with cryptographic proofs

### Platform Support
- ✅ **macOS**: Native Apple Silicon and Intel support
- 🔄 **Windows**: Coming in v0.2.0
- 🔄 **Linux**: Coming in v0.2.0
- 🔄 **Mobile**: iOS/Android via Flutter (roadmap)

## 🧪 Testing & Validation

This release has been thoroughly tested with:
- **220+ Unit Tests**: Comprehensive test coverage
- **Multi-Node Testing**: Verified P2P communication between multiple instances
- **Identity System Testing**: Encrypted profile creation and sharing
- **DHT Integration Testing**: Distributed storage and retrieval
- **Contact Discovery Testing**: Friend finding and connection
- **Security Testing**: Encryption, authentication, and access control

## 📱 Project Structure

```
p2p-foundation/
├── crates/
│   ├── p2p-core/          # Ant Core library (published)
│   ├── p2p-cli/           # CLI tools
│   └── p2p-ffi/           # FFI bindings for mobile
├── apps/
│   └── desktop-tauri/     # Saorsa desktop app
├── assets/                # App binaries and assets
│   ├── desktop/macos/     # macOS app bundle and DMG
│   └── icons/             # Application icons
├── examples/              # Code examples
└── test-app/              # Multi-node testing
```

## 🔗 Links & Resources

- **📚 Documentation**: [docs.rs/ant-core](https://docs.rs/ant-core)
- **📦 Crates.io**: [crates.io/crates/ant-core](https://crates.io/crates/ant-core)
- **🐛 Issues**: [GitHub Issues](https://github.com/dirvine/p2p/issues)
- **💻 Repository**: [GitHub Repository](https://github.com/dirvine/p2p)

## 🎯 Roadmap

### 🔄 Next Release (v0.2.0) - Planned Q1 2025
- [ ] Windows desktop application
- [ ] Linux desktop application  
- [ ] Enhanced NAT traversal techniques
- [ ] Advanced bootstrap strategies
- [ ] Performance optimizations
- [ ] Voice/video calling capabilities

### 📋 Future Releases
- [ ] Mobile applications (iOS/Android via Flutter)
- [ ] File sharing and synchronization
- [ ] Plugin system for extensibility
- [ ] Advanced security features
- [ ] Geographic routing optimization

## 🤝 Contributing

We welcome contributions! Please see our [contributing guidelines](CONTRIBUTING.md) and follow the [development guidelines](CLAUDE.md).

### Getting Started
1. Fork the repository
2. Clone your fork
3. Follow the build instructions in the README
4. Make your changes following our code standards
5. Submit a pull request

## 🙏 Acknowledgments

Special thanks to the open source community and these excellent projects:
- **Quinn**: QUIC implementation in Rust
- **Tauri**: Desktop app framework  
- **Tokio**: Async runtime for Rust
- **libp2p**: P2P networking primitives

## 📄 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

---

**🌐✨ Building the decentralized future, one node at a time.**

*For installation help, see [assets/INSTALLATION.md](assets/INSTALLATION.md)*