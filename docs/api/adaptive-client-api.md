# Adaptive P2P Client API Reference

## Overview

The Adaptive P2P Client API provides a high-level interface for interacting with the network. It abstracts away the complexity of routing, replication, and learning systems while providing powerful features for distributed applications.

## Table of Contents

- [Client Initialization](#client-initialization)
- [Storage Operations](#storage-operations)
- [Retrieval Operations](#retrieval-operations)
- [Messaging](#messaging)
- [Network Information](#network-information)
- [Configuration](#configuration)
- [Error Handling](#error-handling)
- [Examples](#examples)

## Client Initialization

### Creating a Client

```rust
use saorsa_core::adaptive::{Client, ClientConfig, ClientProfile};

// Default configuration
let client = Client::connect(ClientConfig::default()).await?;

// Custom configuration
let config = ClientConfig {
    profile: ClientProfile::Full,
    bootstrap_nodes: vec![
        "bootstrap1.network:8000".to_string(),
        "bootstrap2.network:8000".to_string(),
    ],
    storage_path: Some("/path/to/storage".into()),
    ..Default::default()
};
let client = Client::connect(config).await?;
```

### Client Profiles

```rust
pub enum ClientProfile {
    /// Full node with all capabilities
    Full,
    
    /// Light node without storage
    Light,
    
    /// Compute-optimized node
    Compute,
    
    /// Mobile-optimized node
    Mobile,
}
```

## Storage Operations

### Store Data

```rust
/// Store data in the network with automatic chunking and replication
async fn store(&self, data: Vec<u8>) -> Result<ContentHash>

// Example
let data = b"Hello, P2P World!".to_vec();
let hash = client.store(data).await?;
println!("Stored with hash: {:?}", hash);
```

### Store with Metadata

```rust
/// Store data with custom metadata
async fn store_with_metadata(
    &self,
    data: Vec<u8>,
    metadata: ContentMetadata
) -> Result<ContentHash>

// Example
let metadata = ContentMetadata {
    content_type: ContentType::DataRetrieval,
    importance: 0.8,
    ttl: Some(Duration::from_secs(3600)),
    tags: vec!["document".to_string(), "public".to_string()],
};
let hash = client.store_with_metadata(data, metadata).await?;
```

## Retrieval Operations

### Retrieve Data

```rust
/// Retrieve data by content hash
async fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>>

// Example
let data = client.retrieve(&hash).await?;
println!("Retrieved {} bytes", data.len());
```

### Retrieve with Options

```rust
/// Retrieve data with custom options
async fn retrieve_with_options(
    &self,
    hash: &ContentHash,
    options: RetrievalOptions
) -> Result<Vec<u8>>

// Example
let options = RetrievalOptions {
    timeout: Duration::from_secs(30),
    min_replicas: 3,
    verify_integrity: true,
};
let data = client.retrieve_with_options(&hash, options).await?;
```

## Messaging

### Publish Messages

```rust
/// Publish a message to a topic
async fn publish(&self, topic: &str, message: Vec<u8>) -> Result<()>

// Example
client.publish("chat.general", b"Hello everyone!".to_vec()).await?;
```

### Subscribe to Topics

```rust
/// Subscribe to a topic and receive messages
async fn subscribe(&self, topic: &str) -> Result<MessageStream>

// Example
let mut stream = client.subscribe("chat.general").await?;
while let Some(message) = stream.next().await {
    println!("Received: {:?}", String::from_utf8_lossy(&message));
}
```

### Message Stream

```rust
pub type MessageStream = Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>;
```

## Network Information

### Get Network Statistics

```rust
/// Get current network statistics
async fn get_network_stats(&self) -> Result<NetworkStats>

// Example
let stats = client.get_network_stats().await?;
println!("Connected peers: {}", stats.connected_peers);
println!("Network size estimate: {}", stats.network_size);
println!("Average latency: {}ms", stats.avg_latency_ms);
```

### Network Stats Structure

```rust
pub struct NetworkStats {
    pub connected_peers: usize,
    pub network_size: usize,
    pub avg_latency_ms: f64,
    pub total_storage: u64,
    pub available_storage: u64,
    pub routing_success_rate: f64,
    pub cache_hit_rate: f64,
}
```

### Get Node Information

```rust
/// Get information about connected nodes
async fn get_connected_nodes(&self) -> Result<Vec<NodeInfo>>

// Example
let nodes = client.get_connected_nodes().await?;
for node in nodes {
    println!("Node {}: trust={:.2}", node.id, node.trust_score);
}
```

## Configuration

### Client Configuration

```rust
pub struct ClientConfig {
    /// Client profile determining resource allocation
    pub profile: ClientProfile,
    
    /// Bootstrap nodes for initial connection
    pub bootstrap_nodes: Vec<String>,
    
    /// Local storage path
    pub storage_path: Option<PathBuf>,
    
    /// Maximum storage to contribute (bytes)
    pub max_storage: u64,
    
    /// Maximum bandwidth to use (bytes/sec)
    pub max_bandwidth: u64,
    
    /// Enable compute sharing
    pub enable_compute: bool,
    
    /// Custom configuration values
    pub custom: HashMap<String, String>,
}
```

### Default Configuration

```rust
impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            profile: ClientProfile::Full,
            bootstrap_nodes: vec![
                "bootstrap1.p2p.network:8000".to_string(),
                "bootstrap2.p2p.network:8000".to_string(),
            ],
            storage_path: None,
            max_storage: 50 * 1024 * 1024 * 1024, // 50GB
            max_bandwidth: 10 * 1024 * 1024,      // 10MB/s
            enable_compute: false,
            custom: HashMap::new(),
        }
    }
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Retrieval error: {0}")]
    RetrievalError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] std::io::Error),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Other error: {0}")]
    Other(String),
}
```

### Error Handling Example

```rust
match client.retrieve(&hash).await {
    Ok(data) => {
        println!("Retrieved {} bytes", data.len());
    }
    Err(ClientError::Timeout) => {
        println!("Request timed out, retrying...");
        // Retry logic
    }
    Err(ClientError::RetrievalError(msg)) => {
        println!("Retrieval failed: {}", msg);
    }
    Err(e) => {
        println!("Unexpected error: {}", e);
    }
}
```

## Examples

### Complete Example: File Storage

```rust
use saorsa_core::adaptive::*;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to network
    let client = Client::connect(ClientConfig::default()).await?;
    
    // Read file
    let file_data = fs::read("document.pdf")?;
    
    // Store file
    let hash = client.store(file_data).await?;
    println!("File stored with hash: {:?}", hash);
    
    // Save hash for later retrieval
    fs::write("document.hash", format!("{:?}", hash))?;
    
    Ok(())
}
```

### Complete Example: Chat Application

```rust
use saorsa_core::adaptive::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect(ClientConfig::default()).await?;
    
    // Subscribe to chat topic
    let mut stream = client.subscribe("chat.lobby").await?;
    
    // Spawn message receiver
    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            if let Ok(text) = String::from_utf8(msg) {
                println!("< {}", text);
            }
        }
    });
    
    // Send messages from stdin
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = reader.lines();
    
    while let Some(line) = lines.next_line().await? {
        client.publish("chat.lobby", line.into_bytes()).await?;
    }
    
    Ok(())
}
```

### Complete Example: Network Monitor

```rust
use saorsa_core::adaptive::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect(ClientConfig::default()).await?;
    
    loop {
        let stats = client.get_network_stats().await?;
        
        println!("\n=== Network Statistics ===");
        println!("Connected Peers: {}", stats.connected_peers);
        println!("Network Size: ~{}", stats.network_size);
        println!("Avg Latency: {:.1}ms", stats.avg_latency_ms);
        println!("Storage: {}/{} GB", 
            stats.available_storage / (1024*1024*1024),
            stats.total_storage / (1024*1024*1024)
        );
        println!("Routing Success: {:.1}%", stats.routing_success_rate * 100.0);
        println!("Cache Hit Rate: {:.1}%", stats.cache_hit_rate * 100.0);
        
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
```

## Advanced Features

### Compute Jobs (Future)

```rust
/// Submit a compute job to the network
async fn submit_compute_job(&self, job: ComputeJob) -> Result<JobId>

/// Get compute job results
async fn get_job_result(&self, job_id: &JobId) -> Result<JobResult>
```

### Direct Node Communication

```rust
/// Send a direct message to a specific node
async fn send_direct(
    &self, 
    node_id: &NodeId, 
    message: Vec<u8>
) -> Result<()>
```

## Best Practices

1. **Error Handling**: Always handle network errors gracefully with retries
2. **Resource Management**: Use appropriate client profiles for your use case
3. **Content Types**: Tag content appropriately for optimal routing
4. **Cleanup**: Always call `shutdown()` when done

```rust
// Ensure cleanup on exit
let client = Client::connect(config).await?;
let result = do_work(&client).await;
client.shutdown().await?;
result?;
```

## Performance Tips

1. **Batch Operations**: Store related data together
2. **Parallel Requests**: Use `tokio::join!` for concurrent operations
3. **Caching**: The network automatically caches popular content
4. **Local First**: Check local cache before network retrieval

## Security Considerations

1. **Encryption**: All data is encrypted in transit
2. **Authentication**: Nodes are authenticated via Ed25519
3. **Integrity**: Content addressing ensures data integrity
4. **Privacy**: Use encryption for sensitive data before storage

---

For more examples and advanced usage, see the [examples directory](../../crates/p2p-core/examples/).