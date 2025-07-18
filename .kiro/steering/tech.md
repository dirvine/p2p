# Technology Stack

## Build System

- **Primary**: Cargo workspace with Rust 2024 edition
- **Version**: 0.2.6 across all workspace members
- **Architecture**: Unified Tauri Cross-Platform + Shared Rust Core

## Core Technologies

### Languages & Frameworks
- **Rust**: Primary language (edition 2024, MSRV 1.75+)
- **Tauri**: Cross-platform application framework (v2) for desktop, mobile, and web
- **JavaScript/HTML**: Frontend for Tauri apps

### Networking & Transport
- **QUIC**: Primary transport via `ant-quic` and Quinn
- **Four-Word Networking**: Human-readable addresses via `four-word-networking` crate
- **Kademlia DHT**: Distributed hash table for peer discovery and storage
- **NAT Traversal**: Built-in hole punching and relay infrastructure

### Cryptography
- **Ed25519/X25519**: Current key pairs for signatures and encryption
- **Post-Quantum**: ML-KEM-768 (FIPS 203) and ML-DSA-65 (FIPS 204) foundation
- **Threshold Crypto**: FROST protocol via `frost-ed25519`
- **Hashing**: BLAKE3 for content addressing, SHA-2 for legacy compatibility
- **Encryption**: AES-256-GCM with Argon2id key derivation

### Key Dependencies
- **Async Runtime**: Tokio with full features
- **Serialization**: Serde with JSON support
- **Error Handling**: `anyhow` and `thiserror`
- **Logging**: `tracing` ecosystem
- **Crypto**: `ed25519-dalek`, `aes-gcm`, `hkdf`

## Common Commands

### Building
```bash
# Build all workspace members
cargo build --release

# Build specific apps
cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester

# Build desktop app
cd apps/saorsa && cargo tauri build

# Quick terminal apps build
./BUILD_NOW.sh
```

### Testing
```bash
# Run all tests
cargo test --all-features

# Run comprehensive test suite
cd crates/ant-test-suite && cargo test

# Run specific test categories
cargo test network_tests
cargo test integration_tests
cargo test stress_tests
```

### Development
```bash
# Run desktop app in dev mode
cd apps/saorsa && cargo tauri dev

# Run terminal chat
cargo run --bin saorsa-terminal-chat

# Run network tester
cargo run --bin saorsa-network-tester
```

### Distribution
```bash
# Create macOS app bundles
cd apps && ./create_macos_apps.sh

# Build release distribution
./create_final_distribution.sh
```

## Build Profiles

- **Release**: LTO thin, opt-level 3, panic abort
- **Dev**: Fast compilation with opt-level 2 for dependencies
- **Test**: Unoptimized for faster test compilation

## Platform Support

- **Desktop**: macOS, Windows, Linux (via Tauri)
- **Mobile**: iOS, Android (via Tauri mobile capabilities)
- **Web**: WebAssembly compilation and Tauri web targets
- **Server**: Linux optimized for edge deployment