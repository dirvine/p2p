# Adaptive P2P Network - Quick Start Guide

Get up and running with the Adaptive P2P Network in minutes!

## Prerequisites

- Rust 1.75+ (install from [rustup.rs](https://rustup.rs))
- 2GB RAM minimum (4GB recommended)
- 10GB free disk space
- Open network connection

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/dirvine/p2p
cd p2p

# Build in release mode
cargo build --release

# Run tests to verify installation
cargo test
```

### Using Cargo

```bash
# Install the client library
cargo add saorsa-core

# For CLI tools
cargo install p2p-cli
```

## Quick Start Examples

### 1. Basic Node

Create a new Rust project and add to `Cargo.toml`:

```toml
[dependencies]
saorsa-core = "0.2"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

Create `src/main.rs`:

```rust
use saorsa_core::adaptive::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize client with default settings
    let client = Client::connect(ClientConfig::default()).await?;
    println!("Connected to P2P network!");
    
    // Store some data
    let data = b"Hello, Adaptive P2P Network!";
    let hash = client.store(data.to_vec()).await?;
    println!("Stored data with hash: {:?}", hash);
    
    // Retrieve the data
    let retrieved = client.retrieve(&hash).await?;
    println!("Retrieved: {}", String::from_utf8_lossy(&retrieved));
    
    // Keep running until Ctrl+C
    println!("Press Ctrl+C to exit");
    tokio::signal::ctrl_c().await?;
    
    // Clean shutdown
    client.shutdown().await?;
    Ok(())
}
```

Run with:
```bash
cargo run
```

### 2. File Storage System

```rust
use saorsa_core::adaptive::*;
use std::fs;
use std::path::Path;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect(ClientConfig::default()).await?;
    
    // Store a file
    let file_path = "document.pdf";
    let file_data = fs::read(file_path)?;
    let hash = client.store(file_data).await?;
    
    // Save hash for later retrieval
    fs::write(format!("{}.hash", file_path), format!("{:?}", hash))?;
    println!("File stored! Hash saved to {}.hash", file_path);
    
    // Retrieve file
    let retrieved_data = client.retrieve(&hash).await?;
    fs::write("retrieved_document.pdf", retrieved_data)?;
    println!("File retrieved and saved!");
    
    client.shutdown().await?;
    Ok(())
}
```

### 3. Real-time Chat

```rust
use saorsa_core::adaptive::*;
use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Connecting to P2P chat...");
    let client = Client::connect(ClientConfig::default()).await?;
    
    // Subscribe to chat channel
    let mut stream = client.subscribe("chat.general").await?;
    
    // Handle incoming messages
    let incoming = tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            if let Ok(text) = String::from_utf8(msg) {
                println!("\n< {}", text);
                print!("> ");
            }
        }
    });
    
    // Handle outgoing messages
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    
    println!("Connected! Type messages and press Enter to send.");
    print!("> ");
    
    while let Some(line) = reader.next_line().await? {
        let msg = format!("Anonymous: {}", line);
        client.publish("chat.general", msg.into_bytes()).await?;
        print!("> ");
    }
    
    incoming.abort();
    client.shutdown().await?;
    Ok(())
}
```

## Configuration Options

### Environment Variables

```bash
# Set custom bootstrap nodes
export P2P_BOOTSTRAP_NODES="node1.example.com:8000,node2.example.com:8000"

# Set storage location
export P2P_STORAGE_PATH="/path/to/storage"

# Set maximum storage contribution (bytes)
export P2P_MAX_STORAGE="107374182400"  # 100GB

# Enable debug logging
export RUST_LOG=debug
```

### Configuration File

Create `~/.p2p/config.toml`:

```toml
# Client profile: Full, Light, Compute, Mobile
profile = "Full"

# Bootstrap nodes
bootstrap_nodes = [
    "bootstrap1.p2p.network:8000",
    "bootstrap2.p2p.network:8000"
]

# Storage settings
[storage]
path = "/home/user/.p2p/storage"
max_size = 107374182400  # 100GB in bytes

# Network settings
[network]
listen_port = 8000
max_connections = 1000
max_bandwidth = 10485760  # 10MB/s in bytes

# Security settings
[security]
enable_rate_limiting = true
max_requests_per_minute = 1000
```

## Running Different Node Types

### Full Node (Default)
```rust
let config = ClientConfig {
    profile: ClientProfile::Full,
    ..Default::default()
};
```

### Light Client (No Storage)
```rust
let config = ClientConfig {
    profile: ClientProfile::Light,
    max_storage: 0,
    ..Default::default()
};
```

### Mobile Client
```rust
let config = ClientConfig {
    profile: ClientProfile::Mobile,
    max_storage: 1024 * 1024 * 1024, // 1GB
    max_bandwidth: 1024 * 1024,       // 1MB/s
    ..Default::default()
};
```

### Compute Node
```rust
let config = ClientConfig {
    profile: ClientProfile::Compute,
    enable_compute: true,
    ..Default::default()
};
```

## Monitoring

### Basic Health Check

```rust
let stats = client.get_network_stats().await?;
println!("Connected peers: {}", stats.connected_peers);
println!("Network health: {:.1}%", stats.routing_success_rate * 100.0);
```

### Continuous Monitoring

```rust
use std::time::Duration;

loop {
    let stats = client.get_network_stats().await?;
    println!("{:#?}", stats);
    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

## Common Issues

### Connection Failed

```
Error: Connection failed: No bootstrap nodes available
```

**Solution**: Ensure bootstrap nodes are reachable and ports are open:
```bash
# Test connectivity
telnet bootstrap1.p2p.network 8000

# Check firewall
sudo ufw allow 8000/tcp
```

### Storage Error

```
Error: Storage error: Permission denied
```

**Solution**: Ensure the storage directory is writable:
```bash
mkdir -p ~/.p2p/storage
chmod 755 ~/.p2p/storage
```

### High Memory Usage

**Solution**: Use a lighter profile:
```rust
let config = ClientConfig {
    profile: ClientProfile::Light,
    ..Default::default()
};
```

## Next Steps

1. **Explore Examples**: Check out more examples in `/crates/p2p-core/examples/`
2. **Read API Docs**: See the full [API Reference](../api/adaptive-client-api.md)
3. **Join Community**: Participate in discussions and get help
4. **Build Apps**: Start building your own P2P applications!

## Getting Help

- **Documentation**: [Full Documentation](../README.md)
- **API Reference**: [Client API](../api/adaptive-client-api.md)
- **Troubleshooting**: [Troubleshooting Guide](../guides/troubleshooting.md)
- **GitHub Issues**: [Report Issues](https://github.com/dirvine/p2p/issues)

---

Happy P2P networking! 🚀