# Saorsa - Revolutionary P2P Desktop Messaging

**🕊️ The flagship P2P messaging application powered by Ant Core**

Saorsa (pronounced "SEER-sha", Irish for "freedom") is a revolutionary desktop messaging application that demonstrates the full potential of the P2P Foundation ecosystem. Built with Tauri, it provides native desktop performance with a modern web UI while delivering true peer-to-peer communication with zero-friction onboarding.

## ✨ Revolutionary Features

### 🤖 AI-Powered Cryptocurrency Management
- **Invisible Wallet Operations**: AI handles all cryptocurrency operations behind the scenes
- **Zero Crypto Knowledge Required**: Users never see wallets, private keys, or token balances
- **Automatic Economic Participation**: Earn ANT tokens by contributing storage, spend automatically for features
- **Seamless Fiat Integration**: Simple "Add Credits" button when needed - AI handles exchange setup

### 🔒 Privacy-First Architecture
- **Encrypted by Default**: All profile data encrypted with AES-256-GCM
- **Friend-Based Sharing**: Granular control over what friends can see
- **Local AI Processing**: All sensitive operations happen on your device
- **Zero Data Collection**: No tracking, analytics, or external data harvesting

### 🌐 True Peer-to-Peer Communication
- **No Central Servers**: Direct encrypted communication between devices
- **Three-Word Addresses**: Share `alice.secure.network` instead of complex addresses
- **Universal Connectivity**: Works on any network through intelligent tunneling
- **Cross-Device Sync**: Access your data from any device worldwide

## 🚀 Quick Start

### Installation from Binary
```bash
# Install the desktop application
cargo install saorsa

# Run the application
saorsa
```

### Building from Source
```bash
# Clone the repository
git clone https://github.com/dirvine/p2p.git
cd p2p/apps/desktop-tauri

# Install frontend dependencies
npm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

### Using as a Library
```toml
[dependencies]
saorsa = "0.1.8"
```

```rust
use saorsa::SaorsaApp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = SaorsaApp::new().await?;
    app.run().await?;
    Ok(())
}
```

## 🏗️ Architecture

Saorsa demonstrates the revolutionary three-layer invisible complexity architecture:

1. **User Experience Layer**: Traditional chat interface with no crypto terminology
2. **AI Management Layer**: Local AI models handle all cryptocurrency operations invisibly
3. **P2P Foundation Layer**: Ant Core library provides privacy-first networking

## 🎯 User Experience Innovation

### Zero-Friction Onboarding
1. **Download AI Model**: App downloads personalized AI model (1-10GB)
2. **Create Profile**: Choose display name and three-word address
3. **Start Chatting**: Full functionality available immediately
4. **Invisible Setup**: AI creates wallet and begins earning tokens automatically

### Invisible Economics
- Users earn "network credits" automatically by contributing storage
- Features unlock naturally through token accumulation
- Simple "Add Credits" purchasing when needed
- AI optimizes all economic decisions automatically

## 📱 Platform Support

### Current Status
- ✅ **macOS**: Native Apple Silicon and Intel support
- 🔄 **Windows**: Coming in v0.2.0
- 🔄 **Linux**: Coming in v0.2.0

### Future Platforms
- **iOS/Android**: Via Flutter integration
- **Web**: WebAssembly compilation

## 🔗 Related Projects

- **[Ant Core](https://crates.io/crates/ant-core)**: The foundational P2P networking library
- **[P2P Foundation](https://github.com/dirvine/p2p)**: Complete ecosystem repository

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/dirvine/p2p/blob/main/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/dirvine/p2p/blob/main/LICENSE-MIT))

at your option.

---

**🕊️ Saorsa - Freedom through decentralized communication**

*Building the future of private, peer-to-peer messaging with invisible complexity and maximum user empowerment.*