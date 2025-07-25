# Adaptive P2P Network Troubleshooting Guide

This guide helps diagnose and resolve common issues with the Adaptive P2P Network.

## Table of Contents

- [Connection Issues](#connection-issues)
- [Performance Problems](#performance-problems)
- [Storage Issues](#storage-issues)
- [Security Warnings](#security-warnings)
- [Data Retrieval Failures](#data-retrieval-failures)
- [High Resource Usage](#high-resource-usage)
- [Debugging Tools](#debugging-tools)
- [Common Error Messages](#common-error-messages)

## Connection Issues

### Cannot Connect to Network

**Symptoms:**
- `Error: Connection failed: No bootstrap nodes available`
- `Error: Connection timeout`
- Client hangs during initialization

**Diagnosis:**
```bash
# Test network connectivity
ping bootstrap1.p2p.network

# Test port accessibility
telnet bootstrap1.p2p.network 8000

# Check firewall rules
sudo iptables -L -n | grep 8000
```

**Solutions:**

1. **Check Internet Connection**
   ```bash
   # Test general connectivity
   curl -I https://google.com
   ```

2. **Verify Bootstrap Nodes**
   ```rust
   let config = ClientConfig {
       bootstrap_nodes: vec![
           "bootstrap1.p2p.network:8000".to_string(),
           "bootstrap2.p2p.network:8000".to_string(),
           "bootstrap3.p2p.network:8000".to_string(),
       ],
       ..Default::default()
   };
   ```

3. **Configure Firewall**
   ```bash
   # Allow P2P port
   sudo ufw allow 8000/tcp
   
   # For custom port
   sudo ufw allow 12345/tcp
   ```

4. **Use Alternative Transport**
   ```toml
   [network]
   # Try different transport protocols
   preferred_transport = "tcp"  # or "quic", "websocket"
   ```

### Frequent Disconnections

**Symptoms:**
- `Peer disconnected unexpectedly`
- Connection count fluctuates rapidly
- Unstable network performance

**Solutions:**

1. **Increase Keep-Alive Interval**
   ```toml
   [network]
   keep_alive_interval = 30  # seconds
   connection_timeout = 60
   ```

2. **Check Network Stability**
   ```bash
   # Monitor packet loss
   ping -c 100 bootstrap1.p2p.network | grep loss
   
   # Check bandwidth
   speedtest-cli
   ```

3. **Adjust Connection Limits**
   ```toml
   [network]
   max_connections = 500  # Reduce if on limited connection
   ```

## Performance Problems

### Slow Data Retrieval

**Symptoms:**
- Retrieval takes >10 seconds
- Timeouts during retrieve operations
- Poor cache hit rates

**Diagnosis:**
```rust
// Check network statistics
let stats = client.get_network_stats().await?;
println!("Routing success rate: {:.1}%", stats.routing_success_rate * 100.0);
println!("Cache hit rate: {:.1}%", stats.cache_hit_rate * 100.0);
println!("Average latency: {:.1}ms", stats.avg_latency_ms);
```

**Solutions:**

1. **Increase Replication Factor**
   ```toml
   [storage]
   replication_factor = 10  # Default is 5
   ```

2. **Optimize Cache Settings**
   ```toml
   [storage]
   cache_size = 5368709120  # 5GB
   
   [learning]
   enable_qlearning_cache = true
   ```

3. **Use Retrieval Options**
   ```rust
   let options = RetrievalOptions {
       timeout: Duration::from_secs(60),
       min_replicas: 2,  # Accept partial results
       verify_integrity: false,  # Skip verification for speed
   };
   let data = client.retrieve_with_options(&hash, options).await?;
   ```

### High Latency

**Symptoms:**
- Operations take longer than expected
- Network feels sluggish
- Poor responsiveness

**Solutions:**

1. **Enable Hyperbolic Routing**
   ```toml
   [routing]
   enable_hyperbolic = true
   coordinate_update_interval = 300  # 5 minutes
   ```

2. **Optimize Routing Parameters**
   ```toml
   [routing]
   k = 20      # Routing table size
   alpha = 5   # Parallel queries
   ```

3. **Choose Closer Bootstrap Nodes**
   ```bash
   # Measure latency to bootstrap nodes
   for node in bootstrap1 bootstrap2 bootstrap3; do
       ping -c 5 $node.p2p.network | grep avg
   done
   ```

## Storage Issues

### Storage Full Errors

**Symptoms:**
- `Error: Storage error: No space left`
- `Failed to store: Disk full`
- Node stops accepting new data

**Solutions:**

1. **Check Available Space**
   ```bash
   df -h ~/.p2p/storage
   ```

2. **Adjust Storage Limits**
   ```toml
   [storage]
   max_size = 53687091200  # 50GB
   # Enable automatic cleanup
   auto_cleanup = true
   cleanup_threshold = 0.9  # Clean when 90% full
   ```

3. **Clear Cache**
   ```bash
   # Remove cache files
   rm -rf ~/.p2p/storage/cache/*
   ```

### Permission Denied

**Symptoms:**
- `Error: Permission denied`
- Cannot create storage directory
- Cannot write to storage

**Solutions:**

1. **Fix Permissions**
   ```bash
   # Create directory with correct permissions
   mkdir -p ~/.p2p/storage
   chmod 755 ~/.p2p/storage
   
   # Fix ownership
   chown -R $USER:$USER ~/.p2p/storage
   ```

2. **Use Different Storage Path**
   ```toml
   [storage]
   path = "/home/user/p2p-data"  # Use absolute path
   ```

## Security Warnings

### Rate Limit Exceeded

**Symptoms:**
- `Warning: Rate limit exceeded`
- Requests being rejected
- Temporary bans

**Solutions:**

1. **Adjust Rate Limits**
   ```toml
   [security]
   max_requests_per_minute = 2000  # Increase limit
   ```

2. **Implement Request Batching**
   ```rust
   // Batch multiple operations
   let futures = vec![
       client.store(data1),
       client.store(data2),
       client.store(data3),
   ];
   let results = futures::future::join_all(futures).await;
   ```

### Blacklisted Node

**Symptoms:**
- `Error: Node blacklisted`
- Cannot connect to certain nodes
- Reduced network connectivity

**Solutions:**

1. **Check Blacklist Status**
   ```rust
   // In security module
   let is_blacklisted = security_manager
       .is_blacklisted(&node_id)
       .await;
   ```

2. **Wait for Expiration**
   ```toml
   [security]
   blacklist_duration = 3600  # 1 hour
   ```

### Eclipse Attack Warning

**Symptoms:**
- `Warning: Possible eclipse attack detected`
- Routing table lacks diversity
- Connection to limited set of nodes

**Solutions:**

1. **Force Routing Table Refresh**
   ```rust
   // Trigger manual refresh
   client.refresh_routing_table().await?;
   ```

2. **Increase Bootstrap Diversity**
   ```toml
   [network]
   bootstrap_nodes = [
       # Add more geographically diverse nodes
       "us-east.p2p.network:8000",
       "eu-west.p2p.network:8000",
       "asia-pac.p2p.network:8000",
   ]
   ```

## Data Retrieval Failures

### Hash Not Found

**Symptoms:**
- `Error: Content not found`
- `No nodes have requested data`
- Retrieval returns empty

**Diagnosis:**
```rust
// Check if content exists in network
let exists = client.content_exists(&hash).await?;
println!("Content exists: {}", exists);
```

**Solutions:**

1. **Verify Hash Correctness**
   ```rust
   // Ensure hash is valid
   println!("Looking for hash: {:?}", hash);
   ```

2. **Increase Search Timeout**
   ```rust
   let options = RetrievalOptions {
       timeout: Duration::from_secs(120),  // 2 minutes
       ..Default::default()
   };
   ```

3. **Check Replication Status**
   ```rust
   // Get replica information
   let replicas = client.get_replica_info(&hash).await?;
   println!("Found {} replicas", replicas.len());
   ```

### Integrity Check Failed

**Symptoms:**
- `Error: Integrity verification failed`
- Retrieved data hash mismatch
- Corrupted data warnings

**Solutions:**

1. **Retry with Different Nodes**
   ```rust
   for attempt in 0..3 {
       match client.retrieve(&hash).await {
           Ok(data) => return Ok(data),
           Err(e) => {
               println!("Attempt {} failed: {}", attempt + 1, e);
               tokio::time::sleep(Duration::from_secs(2)).await;
           }
       }
   }
   ```

2. **Disable Integrity Check (Temporary)**
   ```rust
   let options = RetrievalOptions {
       verify_integrity: false,  // Use with caution
       ..Default::default()
   };
   ```

## High Resource Usage

### Excessive Memory Usage

**Symptoms:**
- Process using >4GB RAM
- System becoming unresponsive
- Out of memory errors

**Solutions:**

1. **Use Light Profile**
   ```rust
   let config = ClientConfig {
       profile: ClientProfile::Light,
       ..Default::default()
   };
   ```

2. **Limit Cache Size**
   ```toml
   [storage]
   cache_size = 536870912  # 512MB
   
   [network]
   max_connections = 200  # Reduce connections
   ```

3. **Monitor Memory Usage**
   ```rust
   // Add memory monitoring
   let memory = client.get_memory_usage().await?;
   println!("Memory usage: {}MB", memory / 1024 / 1024);
   ```

### High CPU Usage

**Symptoms:**
- CPU at 100%
- System sluggish
- Fan noise

**Solutions:**

1. **Disable Intensive Features**
   ```toml
   [learning]
   enable_churn_predictor = false  # CPU intensive
   enable_thompson_sampling = false
   
   [advanced]
   thread_pool_size = 4  # Limit threads
   ```

2. **Reduce Gossip Activity**
   ```toml
   [gossip]
   mesh_degree = 6  # Reduce from 8
   heartbeat_interval = 5  # Increase from 1
   ```

## Debugging Tools

### Enable Debug Logging

```toml
[monitoring]
log_level = "debug"
log_file = "p2p-debug.log"

# Specific component debugging
[advanced.custom]
log_routing = "true"
log_storage = "true"
log_gossip = "true"
log_security = "true"
```

### Network Diagnostics

```rust
// Comprehensive network diagnostic
async fn diagnose_network(client: &Client) -> Result<()> {
    println!("=== Network Diagnostics ===");
    
    // Check connectivity
    let stats = client.get_network_stats().await?;
    println!("Connected peers: {}", stats.connected_peers);
    
    // Check routing
    println!("Routing success rate: {:.1}%", 
        stats.routing_success_rate * 100.0);
    
    // Check storage
    println!("Storage available: {} GB", 
        stats.available_storage / (1024*1024*1024));
    
    // Check latency
    println!("Average latency: {:.1}ms", stats.avg_latency_ms);
    
    // Security status
    let security = client.get_security_metrics().await?;
    println!("Rate limit violations: {}", 
        security.rate_limit_violations);
    println!("Blacklisted nodes: {}", 
        security.blacklisted_nodes);
    
    Ok(())
}
```

### Performance Profiling

```bash
# CPU profiling
cargo build --release --features profiling
perf record --call-graph=dwarf ./target/release/p2p-node
perf report

# Memory profiling
valgrind --tool=massif ./target/release/p2p-node
ms_print massif.out.*
```

## Common Error Messages

### `Error: Connection refused`
- Bootstrap node is down
- Firewall blocking connection
- Wrong port specified

### `Error: Too many open files`
- System file descriptor limit reached
- Solution: `ulimit -n 65536`

### `Error: Address already in use`
- Port already occupied
- Previous instance still running
- Solution: Use different port or kill process

### `Error: Invalid configuration`
- Syntax error in config file
- Invalid parameter values
- Solution: Validate config with `p2p-cli config validate`

### `Error: Incompatible protocol version`
- Client version mismatch
- Network upgrade in progress
- Solution: Update to latest version

## Getting Further Help

1. **Check Logs**
   ```bash
   tail -f ~/.p2p/logs/node.log
   grep ERROR ~/.p2p/logs/node.log
   ```

2. **Community Support**
   - GitHub Issues: [Report bugs](https://github.com/dirvine/p2p/issues)
   - Discord: Join the community chat
   - Forums: Technical discussions

3. **Diagnostic Report**
   ```bash
   # Generate diagnostic report
   p2p-cli diagnose --output report.json
   ```

---

For performance optimization, see the [Performance Tuning Guide](performance.md).