# Ant Core

**Ant Network - Core P2P networking library with DHT, QUIC transport, three-word addresses, and MCP integration**

[![Crates.io](https://img.shields.io/crates/v/ant-core)](https://crates.io/crates/ant-core)
[![Documentation](https://docs.rs/ant-core/badge.svg)](https://docs.rs/ant-core)
[![License](https://img.shields.io/crates/l/ant-core)](https://github.com/dirvine/p2p/blob/main/LICENSE)

## Overview

Ant Core is a modern P2P networking library built in Rust that provides:

- **QUIC Transport**: Modern, efficient networking with built-in encryption
- **Distributed Hash Table (DHT)**: Kademlia-based distributed storage
- **Three-Word Addresses**: Human-readable network addressing system
- **MCP Integration**: Model Context Protocol for AI agent communication
- **Privacy-First Identity**: Encrypted profiles with friend-based access control
- **IPv6 Native**: Full IPv6 support with IPv4 tunneling fallback

## Features

### 🚀 **High Performance**
- QUIC protocol for low-latency, reliable communication
- Efficient DHT implementation with configurable parameters
- Minimal memory footprint suitable for edge deployment

### 🔒 **Security & Privacy**
- End-to-end encryption by default
- Privacy-first user identity system
- Friend-based access control for profile sharing
- Anti-spoofing with cryptographic verification

### 🌐 **Network Agnostic**
- Works across any network topology
- Automatic NAT traversal
- IPv6-first with comprehensive IPv4 tunneling
- Bootstrap system for peer discovery

### 🤖 **AI Integration**
- MCP (Model Context Protocol) server integration
- Built for AI agent communication
- Tool system for extensible functionality

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
ant-core = "0.1.8"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

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
    
    // Store data in DHT
    let key = ant_core::dht::Key::new(b"my-key");
    let value = b"my-value".to_vec();
    node.dht_put(key.clone(), value).await?;
    
    // Retrieve data from DHT
    if let Some(retrieved) = node.dht_get(key).await? {
        println!("Retrieved: {:?}", String::from_utf8_lossy(&retrieved));
    }
    
    Ok(())
}
```

### Identity System

```rust
use ant_core::identity::manager::{IdentityManager, IdentityManagerConfig};

let manager = IdentityManager::new(IdentityManagerConfig::default());

// Create encrypted identity
let identity = manager.create_identity(
    "Alice".to_string(),
    "alice.secure.network".to_string(),
    None,
    None,
).await?;

// Export for backup
let export_data = manager.export_identity().await?;

// Import on another device
let imported = manager.import_identity(&export_data).await?;
```

### DHT Operations

```rust
use ant_core::dht::Key;

// Store data
let key = Key::new(b"user:alice:profile");
let data = b"encrypted_profile_data".to_vec();
node.dht_put(key.clone(), data).await?;

// Retrieve data
if let Some(data) = node.dht_get(key).await? {
    println!("Found profile data");
}
```

## Architecture

Ant Core is designed with a modular architecture:

```
┌─────────────────┐
│   Application   │
├─────────────────┤
│  Identity Mgmt  │
├─────────────────┤
│   MCP Server    │
├─────────────────┤
│      DHT        │
├─────────────────┤
│   Transport     │
│  (QUIC/TCP)     │
├─────────────────┤
│   Bootstrap     │
└─────────────────┘
```

### Core Components

- **Network**: P2P node management and configuration
- **DHT**: Distributed hash table for decentralized storage
- **Transport**: QUIC and TCP transport implementations
- **Identity**: Privacy-first user identity and profile management
- **MCP**: Model Context Protocol server for AI integration
- **Bootstrap**: Peer discovery and network bootstrapping

## Configuration

```rust
use ant_core::network::{NodeConfig, SecurityConfig, TrustLevel};
use std::time::Duration;

let config = NodeConfig {
    listen_addr: "0.0.0.0:9000".parse()?,
    enable_ipv6: true,
    enable_mcp_server: true,
    connection_timeout: Duration::from_secs(30),
    max_connections: 100,
    security_config: SecurityConfig {
        enable_noise: true,
        enable_tls: true,
        trust_level: TrustLevel::Basic,
    },
    ..Default::default()
};
```

## Examples

Check out the [examples](https://github.com/dirvine/p2p/tree/main/examples) directory for:

- Basic P2P node setup
- DHT storage and retrieval
- Identity management
- MCP service integration
- Three-word address system

## Applications

Ant Core powers:

- **Saorsa**: Desktop P2P application built with Tauri
- **Flutter Integration**: Mobile and web applications via FFI
- **CLI Tools**: Command-line utilities for network management

## Roadmap

- [ ] Enhanced NAT traversal techniques
- [ ] Additional tunneling protocols
- [ ] Improved bootstrap strategies
- [ ] Advanced security features
- [ ] Performance optimizations

## Contributing

We welcome contributions! Please see our [contributing guidelines](https://github.com/dirvine/p2p/blob/main/CONTRIBUTING.md) for details.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Links

- [Documentation](https://docs.rs/ant-core)
- [Repository](https://github.com/dirvine/p2p)
- [Crates.io](https://crates.io/crates/ant-core)
- [Issues](https://github.com/dirvine/p2p/issues)