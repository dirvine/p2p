# P2P Foundation API Reference

## Overview

The P2P Foundation provides a comprehensive adaptive networking framework combining multiple distributed systems technologies. This document provides a complete API reference for all public interfaces.

## Table of Contents

1. [Core Components](#core-components)
2. [Identity Management](#identity-management)
3. [Network Operations](#network-operations)
4. [Storage and Retrieval](#storage-and-retrieval)
5. [Routing Strategies](#routing-strategies)
6. [Trust and Reputation](#trust-and-reputation)
7. [Machine Learning](#machine-learning)
8. [Monitoring and Metrics](#monitoring-and-metrics)
9. [Error Handling](#error-handling)

## Core Components

### NetworkCoordinator

The central integration point for all adaptive network layers.

```rust
use saorsa_core::adaptive::{NetworkCoordinator, NetworkConfig};

// Create configuration
let config = NetworkConfig {
    bootstrap_nodes: vec!["localhost:8000".to_string()],
    storage_capacity: 100,        // GB
    max_connections: 1000,
    replication_factor: 5,
    ml_enabled: true,
    monitoring_interval: Duration::from_secs(30),
    security_level: 7,
};

// Initialize coordinator
let coordinator = NetworkCoordinator::new(identity, config).await?;

// Join network
coordinator.join_network().await?;
```

#### Methods

##### `store(data: Vec<u8>) -> Result<ContentHash>`
Store data in the distributed network with adaptive replication.

```rust
let data = b"Hello, P2P Network!".to_vec();
let hash = coordinator.store(data).await?;
println!("Stored with hash: {:?}", hash);
```

##### `retrieve(hash: &ContentHash) -> Result<Vec<u8>>`
Retrieve data using parallel strategies with ML optimization.

```rust
let data = coordinator.retrieve(&hash).await?;
println!("Retrieved {} bytes", data.len());
```

##### `publish(topic: &str, message: Vec<u8>) -> Result<()>`
Publish message to gossip network.

```rust
coordinator.publish("announcements", b"Node update".to_vec()).await?;
```

##### `get_network_stats() -> NetworkStats`
Get comprehensive network statistics.

```rust
let stats = coordinator.get_network_stats().await;
println!("Connected peers: {}", stats.connected_peers);
println!("Cache hit rate: {:.2}%", stats.cache_hit_rate * 100.0);
```

## Identity Management

### NodeIdentity

Cryptographic identity with four-word human-readable addresses.

```rust
use saorsa_core::adaptive::NodeIdentity;

// Generate new identity with proof-of-work
let identity = NodeIdentity::generate()?;
println!("Address: {}", identity.word_address());

// Generate from seed (deterministic)
let seed = [42u8; 32];
let identity = NodeIdentity::from_seed(&seed)?;

// Save/load identity
identity.save_to_file("identity.json").await?;
let loaded = NodeIdentity::load_from_file("identity.json").await?;
```

#### Methods

##### `sign(message: &[u8]) -> Signature`
Sign a message with the node's private key.

```rust
let signature = identity.sign(b"Hello");
```

##### `verify(message: &[u8], signature: &Signature) -> bool`
Verify a signature.

```rust
let valid = identity.verify(b"Hello", &signature);
```

## Network Operations

### Transport Layer

Multi-protocol transport with automatic fallback.

```rust
use saorsa_core::adaptive::{TransportManager, TransportProtocol};

let transport = TransportManager::new(identity)?;

// Connect to peer
transport.connect("localhost:8001").await?;

// Send message
transport.send(peer_id, message).await?;

// Receive messages
let mut receiver = transport.receive().await?;
while let Some(msg) = receiver.recv().await {
    handle_message(msg);
}
```

### DHT Operations

Kademlia-based distributed hash table with S/Kademlia security.

```rust
use saorsa_core::adaptive::AdaptiveDHT;

let dht = AdaptiveDHT::new(identity, transport, router)?;

// Store value
dht.put(key, value).await?;

// Retrieve value
let value = dht.get(key).await?;

// Find nodes near a key
let nodes = dht.find_nodes(key, 20).await?;
```

## Storage and Retrieval

### Content Store

Adaptive storage with ML-optimized caching.

```rust
use saorsa_core::adaptive::{ContentStore, StorageConfig};

let config = StorageConfig {
    max_size: 100 * 1024 * 1024 * 1024, // 100GB
    chunk_size: 1024 * 1024,             // 1MB chunks
    compression_enabled: true,
    encryption_enabled: true,
};

let storage = ContentStore::new(config)?;

// Store with automatic chunking
let hash = storage.store(large_data).await?;

// Retrieve with parallel chunk fetching
let data = storage.retrieve(&hash).await?;
```

### Replication Manager

Intelligent replication based on content heat and churn prediction.

```rust
use saorsa_core::adaptive::{ReplicationManager, ReplicationConfig};

let config = ReplicationConfig {
    min_replicas: 3,
    target_replicas: 5,
    max_replicas: 10,
    check_interval: Duration::from_secs(300),
};

let replication = ReplicationManager::new(config, trust, predictor, router);

// Manual replication
replication.replicate(&hash, data, ReplicationStrategy::HighAvailability).await?;

// Check replication health
let health = replication.check_health(&hash).await?;
```

## Routing Strategies

### Adaptive Router

Combines multiple routing strategies with ML selection.

```rust
use saorsa_core::adaptive::AdaptiveRouter;

let router = AdaptiveRouter::new(trust_engine, hyperbolic_space, som);

// Route to a node
let path = router.find_path(&target_node_id).await?;

// Route to content
let nodes = router.find_content_holders(&content_hash).await?;
```

### Hyperbolic Routing

Greedy routing in hyperbolic space.

```rust
use saorsa_core::adaptive::{HyperbolicSpace, HyperbolicCoordinate};

let space = HyperbolicSpace::new();

// Embed node in hyperbolic space
let coord = space.embed(&node_id).await?;

// Find path using hyperbolic geometry
let path = space.greedy_route(&source, &target).await?;
```

### Self-Organizing Map

Content and capability clustering.

```rust
use saorsa_core::adaptive::SelfOrganizingMap;

let som = SelfOrganizingMap::new(20, 20, 4);

// Train with node features
som.train(&features, 0.1, 2.0);

// Find best matching unit
let (x, y) = som.find_bmu(&query_features);
```

## Trust and Reputation

### EigenTrust Engine

Decentralized reputation system with Sybil resistance.

```rust
use saorsa_core::adaptive::EigenTrustEngine;

let trust = EigenTrustEngine::new(local_node_id)?;

// Update trust based on interaction
trust.update_trust(&from_node, &to_node, success);

// Get trust score
let score = trust.get_trust(&node_id);

// Get globally trusted nodes
let trusted_nodes = trust.get_trusted_nodes(10).await?;
```

## Machine Learning

### Multi-Armed Bandit

Adaptive route selection with Thompson sampling.

```rust
use saorsa_core::adaptive::{MultiArmedBandit, MABConfig};

let config = MABConfig {
    exploration_factor: 0.1,
    decay_rate: 0.01,
    min_samples: 10,
};

let mab = MultiArmedBandit::new(config);

// Add routing options
mab.add_arm(RouteId::from("kademlia"));
mab.add_arm(RouteId::from("hyperbolic"));
mab.add_arm(RouteId::from("trust_based"));

// Select best route
let route = mab.select_route(options).await?;

// Update with outcome
mab.update_performance(route.selected_route, success, latency).await;
```

### Q-Learning Cache

Intelligent caching decisions.

```rust
use saorsa_core::adaptive::{QLearningCacheManager, StateVector};

let q_cache = QLearningCacheManager::new(QLearningConfig::default());

// Make caching decision
let state = StateVector {
    cache_size: 0.7,
    hit_rate: 0.8,
    network_load: 0.4,
    available_space: 0.3,
    time_of_day: 0.5,
    content_popularity: 0.9,
};

let action = q_cache.select_action(&state).await;
```

### LSTM Churn Predictor

Predict node departures for proactive replication.

```rust
use saorsa_core::adaptive::LSTMChurnPredictor;

let predictor = LSTMChurnPredictor::new();

// Extract features
let features = predictor.extract_features(&node_id, &churn_detector, network_size).await;

// Get predictions
let prediction = predictor.predict(&features).await?;
println!("1-hour churn probability: {:.2}%", prediction.hour_1 * 100.0);

// Get replication recommendations
let recommendations = predictor.get_replication_recommendations(
    &nodes,
    &churn_detector,
    network_size,
    0.7, // threshold
).await;
```

## Monitoring and Metrics

### Monitoring System

Comprehensive metrics collection and alerting.

```rust
use saorsa_core::adaptive::{MonitoringSystem, MonitoringConfig};

let config = MonitoringConfig {
    collection_interval: Duration::from_secs(30),
    retention_period: Duration::from_secs(86400),
    alert_thresholds: AlertThresholds {
        high_churn_rate: 0.3,
        low_connectivity: 10,
        high_latency_ms: 500,
    },
};

let monitoring = MonitoringSystem::new(config);

// Start collection
monitoring.start_collection().await?;

// Get metrics
let metrics = monitoring.get_metrics().await;

// Set up alerts
monitoring.on_alert(|alert| {
    println!("Alert: {:?}", alert);
});
```

## Error Handling

All operations return `Result<T>` with specific error types:

```rust
use saorsa_core::error::{P2PError, Result};

match coordinator.store(data).await {
    Ok(hash) => println!("Success: {:?}", hash),
    Err(P2PError::Network(e)) => eprintln!("Network error: {}", e),
    Err(P2PError::Storage(e)) => eprintln!("Storage error: {}", e),
    Err(P2PError::Security(e)) => eprintln!("Security error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

### Common Error Types

- `P2PError::Network` - Network connectivity issues
- `P2PError::Storage` - Storage capacity or I/O errors
- `P2PError::Security` - Authentication/encryption failures
- `P2PError::NotFound` - Requested content not found
- `P2PError::Timeout` - Operation timed out
- `P2PError::InvalidInput` - Invalid parameters

## Examples

### Complete Example: Building a P2P Application

```rust
use saorsa_core::adaptive::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Generate or load identity
    let identity = match NodeIdentity::load_from_file("identity.json").await {
        Ok(id) => id,
        Err(_) => {
            let id = NodeIdentity::generate()?;
            id.save_to_file("identity.json").await?;
            id
        }
    };
    
    println!("Node address: {}", identity.word_address());
    
    // 2. Configure network
    let config = NetworkConfig {
        bootstrap_nodes: vec![
            "seed1.network.com:8000".to_string(),
            "seed2.network.com:8000".to_string(),
        ],
        storage_capacity: 50,
        max_connections: 500,
        replication_factor: 5,
        ml_enabled: true,
        monitoring_interval: Duration::from_secs(60),
        security_level: 8,
    };
    
    // 3. Create coordinator
    let coordinator = NetworkCoordinator::new(identity, config).await?;
    
    // 4. Join network
    coordinator.join_network().await?;
    
    // 5. Store some data
    let data = b"Important document content".to_vec();
    let hash = coordinator.store(data).await?;
    println!("Stored document with hash: {:?}", hash);
    
    // 6. Subscribe to gossip topic
    let mut messages = coordinator.subscribe("announcements").await?;
    
    // 7. Handle messages
    tokio::spawn(async move {
        while let Some(msg) = messages.recv().await {
            println!("Received announcement: {:?}", msg);
        }
    });
    
    // 8. Monitor network health
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        
        let stats = coordinator.get_network_stats().await;
        println!("Network stats: {:?}", stats);
        
        if stats.connected_peers < 5 {
            println!("Low connectivity detected!");
        }
    }
}
```

## Best Practices

1. **Identity Management**
   - Always save generated identities
   - Use hardware security modules for production
   - Rotate keys periodically

2. **Network Configuration**
   - Start with at least 3 bootstrap nodes
   - Set replication factor based on network size
   - Enable ML features for better performance

3. **Error Handling**
   - Always handle network timeouts
   - Implement exponential backoff for retries
   - Log errors for debugging

4. **Performance**
   - Use async operations for I/O
   - Batch operations when possible
   - Monitor resource usage

5. **Security**
   - Enable encryption for sensitive data
   - Validate all inputs
   - Use rate limiting

## Migration Guide

For migrating from earlier versions, see [MIGRATION.md](MIGRATION.md).

## Support

- GitHub Issues: https://github.com/yourusername/p2p-foundation/issues
- Documentation: https://docs.p2p-foundation.org
- Community Forum: https://forum.p2p-foundation.org
## Business Metrics API

### BusinessMetricsCollector

The `BusinessMetricsCollector` provides thread-safe collection of business-critical P2P network metrics.

```rust
use saorsa_core::health::business_metrics::BusinessMetricsCollector;

// Create collector
let collector = BusinessMetricsCollector::new();

// Record peer events
collector.record_peer_connected()?;
collector.record_peer_disconnected()?;

// Record data operations
collector.record_data_stored(1024, true, 50)?; // bytes, success, latency_ms
collector.record_data_retrieved(512, true, 30)?;

// Get current metrics
let metrics = collector.get_metrics()?;
println\!("Active peers: {}", metrics.active_peers);
println\!("Success rate: {}", metrics.dht_success_rate);

// Export to Prometheus format
let prometheus = collector.to_prometheus()?;
```

#### Methods

##### `record_peer_connected() -> Result<()>`
Records a new peer connection, incrementing the active peer count.

##### `record_peer_disconnected() -> Result<()>`
Records a peer disconnection, decrementing the active peer count.

##### `record_data_stored(bytes: u64, success: bool, latency_ms: u64) -> Result<()>`
Records a data storage operation with size, success status, and latency.
Updates storage success rate and average response times.

##### `record_data_retrieved(bytes: u64, success: bool, latency_ms: u64) -> Result<()>`
Records a data retrieval operation with size, success status, and latency.
Updates DHT success rate and average response times.

##### `get_metrics() -> Result<BusinessMetrics>`
Returns current snapshot of all business metrics including:
- `active_peers`: Number of connected peers
- `total_data_stored/retrieved`: Cumulative data transfer
- `dht_success_rate/storage_success_rate`: Success rates (0.0-1.0)
- `operations_per_second`: Recent operation rate
- `average_response_time_ms`: Average latency

##### `to_prometheus() -> Result<String>`
Exports metrics in Prometheus text format for monitoring integration.

### BusinessMetrics

Core business metrics structure containing:

```rust
pub struct BusinessMetrics {
    pub active_peers: u64,
    pub total_data_stored: u64, 
    pub total_data_retrieved: u64,
    pub operations_per_second: f64,
    pub dht_success_rate: f64,
    pub storage_success_rate: f64,
    pub network_growth_rate: f64,
    pub average_response_time_ms: f64,
    pub timestamp: u64,
}
```

## Enhanced Health System

The health system now includes business metrics integration:

### PrometheusExporter

##### `export_with_business_metrics(&self, business_metrics: &BusinessMetricsCollector) -> Result<String>`
Combines standard health metrics with business metrics for comprehensive monitoring.

```rust
use saorsa_core::health::{PrometheusExporter, business_metrics::BusinessMetricsCollector};

let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));
let business_collector = BusinessMetricsCollector::new();
let exporter = PrometheusExporter::new(health_manager);

// Export combined metrics
let metrics = exporter.export_with_business_metrics(&business_collector).await?;
```

## Error Codes Reference

| Error Code | Description | Recovery Action |
|------------|-------------|-----------------|
| `P2PError::Network(ConnectionFailed)` | Network connection failed | Check connectivity, retry |
| `P2PError::Storage(NotFound)` | Data not found in DHT | Verify hash, check replication |
| `P2PError::Identity(InvalidSignature)` | Cryptographic signature invalid | Verify keys, regenerate identity |
| `P2PError::Internal(message)` | Internal system error | Check logs, restart if needed |

## Rate Limits

- DHT operations: 1000/minute per peer
- Storage operations: 100MB/minute per peer  
- Health checks: 4/minute
- Metrics scraping: 6/minute

## Authentication

P2P operations use cryptographic identity verification:
- Ed25519 signatures for all operations
- ML-KEM/ML-DSA for quantum resistance
- Three-word addresses for human readability
