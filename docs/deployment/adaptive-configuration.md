# Adaptive P2P Network Configuration Reference

This document provides a comprehensive reference for all configuration options available in the Adaptive P2P Network.

## Configuration Methods

Configuration can be provided through multiple methods (in order of precedence):

1. **Command-line arguments** (highest priority)
2. **Environment variables**
3. **Configuration file**
4. **Default values** (lowest priority)

## Configuration File

The default configuration file location is:
- Linux/macOS: `~/.p2p/config.toml`
- Windows: `%APPDATA%\p2p\config.toml`

### Complete Configuration Example

```toml
# Adaptive P2P Network Configuration

[client]
# Client profile: Full, Light, Compute, Mobile
profile = "Full"

# Node identifier (auto-generated if not specified)
# node_id = "optional-custom-node-id"

[network]
# Bootstrap nodes for initial connection
bootstrap_nodes = [
    "bootstrap1.p2p.network:8000",
    "bootstrap2.p2p.network:8000",
    "bootstrap3.p2p.network:8000"
]

# Local listening port (0 = random)
listen_port = 8000

# Maximum number of connections
max_connections = 1000

# Connection timeout in seconds
connection_timeout = 30

# Keep-alive interval in seconds
keep_alive_interval = 60

[storage]
# Storage directory path
path = "~/.p2p/storage"

# Maximum storage to contribute (bytes)
max_size = 107374182400  # 100GB

# Cache size (bytes)
cache_size = 1073741824  # 1GB

# Enable compression
compression = true

# Replication factor
replication_factor = 5

[bandwidth]
# Maximum upload bandwidth (bytes/sec)
max_upload = 10485760    # 10MB/s

# Maximum download bandwidth (bytes/sec)
max_download = 20971520  # 20MB/s

# Bandwidth allocation for different operations (percentage)
allocation = { storage = 40, retrieval = 30, gossip = 20, other = 10 }

[security]
# Enable rate limiting
enable_rate_limiting = true

# Maximum requests per minute per node
max_requests_per_minute = 1000

# Enable blacklist
enable_blacklist = true

# Blacklist duration in seconds
blacklist_duration = 3600  # 1 hour

# Eclipse attack detection threshold
eclipse_threshold = 0.6

[trust]
# Initial trust score for new nodes
initial_trust = 0.5

# Trust decay factor
decay_factor = 0.99

# Minimum trust score for interactions
min_trust = 0.1

# Trust computation interval in seconds
computation_interval = 300  # 5 minutes

[routing]
# Kademlia K parameter
k = 20

# Kademlia alpha parameter
alpha = 3

# Routing table refresh interval in seconds
refresh_interval = 900  # 15 minutes

# Enable hyperbolic routing
enable_hyperbolic = true

# Hyperbolic coordinate update interval
coordinate_update_interval = 600  # 10 minutes

[gossip]
# Gossip mesh degree
mesh_degree = 8

# Gossip mesh degree low
mesh_degree_low = 6

# Gossip mesh degree high
mesh_degree_high = 12

# Gossip heartbeat interval in seconds
heartbeat_interval = 1

# Message cache duration in seconds
message_cache_duration = 120

[learning]
# Enable Thompson Sampling router
enable_thompson_sampling = true

# Enable Q-learning cache
enable_qlearning_cache = true

# Enable LSTM churn predictor
enable_churn_predictor = true

# Learning rate
learning_rate = 0.1

# Exploration rate (epsilon)
exploration_rate = 0.1

[monitoring]
# Enable Prometheus metrics
enable_metrics = true

# Metrics port
metrics_port = 9090

# Enable debug logging
enable_debug = false

# Log level: trace, debug, info, warn, error
log_level = "info"

# Log file path (empty = stdout only)
log_file = ""

[advanced]
# Thread pool size (0 = CPU count)
thread_pool_size = 0

# Enable experimental features
enable_experimental = false

# Custom parameters (key-value pairs)
[advanced.custom]
# example_param = "value"
```

## Environment Variables

All configuration options can be set via environment variables using the prefix `P2P_`:

```bash
# Network settings
export P2P_BOOTSTRAP_NODES="node1:8000,node2:8000"
export P2P_LISTEN_PORT="8000"
export P2P_MAX_CONNECTIONS="1000"

# Storage settings
export P2P_STORAGE_PATH="/path/to/storage"
export P2P_MAX_STORAGE="107374182400"  # 100GB
export P2P_CACHE_SIZE="1073741824"     # 1GB

# Security settings
export P2P_ENABLE_RATE_LIMITING="true"
export P2P_MAX_REQUESTS_PER_MINUTE="1000"

# Logging
export RUST_LOG="info"
export P2P_LOG_FILE="/var/log/p2p.log"
```

## Client Profiles

### Full Node
```toml
[client]
profile = "Full"

[storage]
max_size = 107374182400  # 100GB

[bandwidth]
max_upload = 10485760    # 10MB/s
max_download = 20971520  # 20MB/s
```

### Light Client
```toml
[client]
profile = "Light"

[storage]
max_size = 0  # No storage contribution

[network]
max_connections = 50  # Fewer connections
```

### Mobile Client
```toml
[client]
profile = "Mobile"

[storage]
max_size = 1073741824  # 1GB

[bandwidth]
max_upload = 1048576    # 1MB/s
max_download = 2097152  # 2MB/s

[network]
max_connections = 20
```

### Compute Node
```toml
[client]
profile = "Compute"

[advanced]
enable_compute = true
compute_threads = 8
gpu_enabled = true
```

## Security Configuration

### Strict Security
```toml
[security]
enable_rate_limiting = true
max_requests_per_minute = 100
enable_blacklist = true
blacklist_duration = 7200  # 2 hours
eclipse_threshold = 0.5    # More sensitive

[trust]
initial_trust = 0.3        # Lower initial trust
min_trust = 0.3           # Higher minimum
```

### Relaxed Security (Testing)
```toml
[security]
enable_rate_limiting = false
enable_blacklist = false

[trust]
initial_trust = 0.7
min_trust = 0.0
```

## Performance Tuning

### High Performance
```toml
[storage]
cache_size = 10737418240  # 10GB
compression = false       # Disable for speed

[network]
max_connections = 2000
connection_timeout = 10   # Faster timeouts

[routing]
k = 30                   # Larger routing table
alpha = 5                # More parallel queries

[advanced]
thread_pool_size = 16    # More threads
```

### Low Resource
```toml
[storage]
cache_size = 268435456   # 256MB
compression = true       # Save space

[network]
max_connections = 100
keep_alive_interval = 300  # Less frequent

[routing]
k = 10                   # Smaller routing table
```

## Monitoring Configuration

### Development
```toml
[monitoring]
enable_metrics = true
enable_debug = true
log_level = "debug"
log_file = "p2p-debug.log"
```

### Production
```toml
[monitoring]
enable_metrics = true
enable_debug = false
log_level = "info"
log_file = "/var/log/p2p/node.log"

# Rotate logs
[monitoring.rotation]
max_size = "100MB"
max_age = "7d"
max_backups = 5
```

## Network Specific Configurations

### Private Network
```toml
[network]
# Use only specified bootstrap nodes
bootstrap_nodes = [
    "private-node1.local:8000",
    "private-node2.local:8000"
]

# Disable public node discovery
enable_public_discovery = false

# Use custom network ID
network_id = "my-private-network"
```

### Test Network
```toml
[network]
bootstrap_nodes = [
    "testnet1.p2p.network:8000",
    "testnet2.p2p.network:8000"
]

[advanced]
enable_experimental = true
```

## Troubleshooting Configuration

### Debug Connection Issues
```toml
[monitoring]
log_level = "trace"

[network]
connection_timeout = 60  # Longer timeout
keep_alive_interval = 30  # More frequent

[advanced.custom]
log_connections = "true"
log_routing = "true"
```

### Debug Storage Issues
```toml
[monitoring]
log_level = "debug"

[storage]
# Use absolute path
path = "/home/user/p2p-storage"

[advanced.custom]
log_storage = "true"
verify_writes = "true"
```

## Configuration Validation

The system validates configuration on startup. Invalid configurations will result in clear error messages:

```
Error: Invalid configuration
  - storage.max_size: Must be at least 1GB
  - network.listen_port: Port 80 requires root privileges
  - bandwidth.max_upload: Cannot exceed max_download
```

## Best Practices

1. **Start with defaults**: The default configuration works well for most users
2. **Incremental changes**: Make one change at a time when tuning
3. **Monitor impact**: Use metrics to measure configuration changes
4. **Profile-based**: Use predefined profiles as starting points
5. **Document changes**: Keep notes on why configuration was changed

## Configuration Migration

When upgrading versions, configuration may need migration:

```bash
# Check configuration compatibility
p2p-cli config check

# Migrate configuration
p2p-cli config migrate

# Validate configuration
p2p-cli config validate
```

---

For specific use cases and optimization strategies, see the [Performance Tuning Guide](../guides/performance.md).