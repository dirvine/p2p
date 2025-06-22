# P2P Foundation

A next-generation peer-to-peer networking foundation built in Rust, featuring QUIC transport, IPv6-first architecture, comprehensive tunneling support, and integrated AI capabilities through Model Context Protocol (MCP) servers at each node.

## Features

- 🚀 **Modern Transport**: QUIC protocol with 0-RTT connections
- 🌐 **IPv6-First**: Native IPv6 with automatic IPv4 tunneling
- 🔍 **Kademlia DHT**: Distributed hash table for peer discovery
- 🤖 **AI-Native**: Built-in MCP server at each node
- 🔒 **Secure by Default**: End-to-end encryption
- 📦 **Minimal Dependencies**: Small footprint, pure Rust
- 🛠️ **Developer Friendly**: Simple API with sensible defaults

## Quick Start

```rust
use p2p::{P2PNode, NodeConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a P2P node
    let node = P2PNode::builder()
        .listen_on("/ip6/::/tcp/9000")
        .with_mcp_server()
        .build()
        .await?;
    
    // Register an MCP service
    node.register_mcp_service("my-service", vec![
        Tool::new("echo").with_handler(|params| async {
            Ok(params)
        }),
    ]).await?;
    
    // Connect to peers
    node.connect("/ip6/2001:db8::1/tcp/9000").await?;
    
    // Run the node
    node.run().await?;
    
    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
p2p-foundation = "0.1.0"
```

## Architecture

```
┌─────────────────────────────────────┐
│        MCP Server Layer             │
├─────────────────────────────────────┤
│     Application Protocol Layer      │
├─────────────────────────────────────┤
│    Kademlia DHT (Routing Layer)     │
├─────────────────────────────────────┤
│      libp2p Core (Network)          │
├─────────────────────────────────────┤
│    QUIC/TCP Transport Layer         │
├─────────────────────────────────────┤
│    IPv6/IPv4 Tunneling Layer       │
└─────────────────────────────────────┘
```

## Supported Tunneling Protocols

- **DS-Lite** (Dual-Stack Lite) - ISP-provided
- **Teredo** - NAT traversal capable
- **6to4** - Simple but requires public IPv4
- **ISATAP** - Enterprise networks
- **MAP-E/MAP-T** - Modern ISPs
- **464XLAT** - Mobile networks
- **6rd** - IPv6 Rapid Deployment

## Documentation

- [Technical Specification](SPECIFICATION.md)
- [Product Requirements](PRD.md)
- [AI Development Guidelines](CLAUDE.md)
- [API Documentation](https://docs.rs/p2p-foundation)

## Examples

See the [`examples/`](examples/) directory for:
- Basic P2P node
- DHT storage
- MCP service
- Multi-node network
- NAT traversal

## Development

### Prerequisites

- Rust 1.75 or later
- IPv6 connectivity (native or tunneled)

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/p2p.git
cd p2p

# Build the project
cargo build --release

# Run tests
cargo test --all-features

# Run benchmarks
cargo bench
```

### Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test '*' --features integration

# Specific module
cargo test dht::

# With logging
RUST_LOG=debug cargo test
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## Performance

- Connection establishment: < 100ms (LAN), < 1s (Internet)
- Throughput: > 100 Mbps per connection
- Memory usage: < 100MB baseline
- Concurrent connections: 1000+

## Security

- End-to-end encryption via QUIC/TLS 1.3
- Ed25519 peer authentication
- Capability-based access control
- Rate limiting and DoS protection

## Roadmap

- [x] Core P2P networking
- [x] Kademlia DHT
- [x] QUIC transport
- [x] Basic MCP integration
- [ ] Complete tunneling support
- [ ] Production hardening
- [ ] Language bindings
- [ ] Mobile optimization

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

Built on top of excellent open source projects:
- [libp2p](https://libp2p.io/) - P2P networking stack
- [Quinn](https://github.com/quinn-rs/quinn) - QUIC implementation
- [Tokio](https://tokio.rs/) - Async runtime

## Support

- 📧 Email: support@p2pfoundation.dev
- 💬 Discord: [Join our server](https://discord.gg/p2pfoundation)
- 🐛 Issues: [GitHub Issues](https://github.com/yourusername/p2p/issues)

---

*Building the decentralized future, one node at a time.*