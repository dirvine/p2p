# Communitas - P2P Diagnostic Chat Application

A modern desktop chat application built with Tauri v2 and React, providing comprehensive diagnostics for the P2P Foundation network while delivering a polished user experience.

## Features

- 🗨️ **Group Chat** - Up to 20 participants with persistent 4-word identities
- 📊 **Network Diagnostics** - Real-time visualization of P2P network operations
- 🔒 **Quantum-Resistant Security** - ML-KEM/ML-DSA cryptography throughout
- 📁 **File Sharing** - Transfer files up to 5MB with progress tracking
- 🎥 **Voice/Video** - Real-time communication when both users are online
- 🧪 **Test Harness** - Built-in property testing and network simulation
- 🎨 **Modern UI** - Beautiful React interface with Material-UI components
- 🌓 **Theme Support** - Light and dark modes with smooth transitions

## Quick Start

```bash
# Clone and install dependencies
git clone https://github.com/p2pfoundation/communitas
cd communitas
npm install
cd src-tauri && cargo build

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Download

Pre-built binaries will be available for:
- macOS (Intel & Apple Silicon)
- Windows (x64)
- Linux (AppImage, deb, rpm)

## Architecture

Communitas combines the power of Rust backend with modern React frontend:

- **Frontend**: React + TypeScript with Material-UI components
- **Backend**: Tauri v2 with async Rust for native performance
- **Transport**: QUIC-only using ant-quic v0.5.0 with automatic NAT traversal
- **Storage**: Kademlia DHT with K=8 replication and 1-week message persistence
- **Identity**: Persistent Ed25519 keys mapped to human-readable 4-word addresses
- **IPC**: Type-safe communication between frontend and backend

## Documentation

- [Specification](SPECIFICATION.md) - Detailed feature requirements
- [Technical Design](DESIGN.md) - Architecture and implementation details
- [Implementation Plan](../../.claude/tasks/communitas-implementation-plan.md) - Development roadmap

## Development Status

Currently in active development. See the implementation plan for progress tracking.

## Contributing

Communitas is part of the P2P Foundation project. See the main project README for contribution guidelines.

## License

Dual-licensed under AGPL-3.0 (open source) and commercial license.