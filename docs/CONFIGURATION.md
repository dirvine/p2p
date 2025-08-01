# Saorsa P2P Network Configuration Guide

## Overview

The Saorsa P2P network uses a flexible configuration system that supports multiple sources with clear precedence:

1. **Environment variables** (highest priority)
2. **Configuration files** (TOML format)
3. **Default values** (lowest priority)

## Configuration File Locations

The system looks for configuration files in the following order:
1. `saorsa.toml` (current directory)
2. `config.toml` (current directory)
3. `/etc/saorsa/config.toml` (system-wide)

You can also specify a custom configuration file path when starting the node.

## Environment Variables

All configuration options can be overridden using environment variables with the `SAORSA_` prefix:

| Environment Variable | Configuration Path | Description |
|---------------------|-------------------|-------------|
| `SAORSA_LISTEN_ADDRESS` | `network.listen_address` | Local listen address |
| `SAORSA_PUBLIC_ADDRESS` | `network.public_address` | Public address for external connections |
| `SAORSA_BOOTSTRAP_NODES` | `network.bootstrap_nodes` | Comma-separated list of bootstrap nodes |
| `SAORSA_MAX_CONNECTIONS` | `network.max_connections` | Maximum concurrent connections |
| `SAORSA_RATE_LIMIT` | `security.rate_limit` | Rate limit per IP |
| `SAORSA_ENCRYPTION_ENABLED` | `security.encryption_enabled` | Enable/disable encryption |
| `SAORSA_DATA_PATH` | `storage.path` | Data storage path |
| `SAORSA_MAX_STORAGE` | `storage.max_size` | Maximum storage size |
| `SAORSA_MCP_ENABLED` | `mcp.enabled` | Enable/disable MCP server |
| `SAORSA_MCP_PORT` | `mcp.port` | MCP server port |

## Configuration Sections

### Network Configuration

```toml
[network]
# Bootstrap nodes for initial network discovery
bootstrap_nodes = [
    "seed1.saorsa.network:9000",
    "seed2.saorsa.network:9000",
]

# Local listen address (0.0.0.0 for all interfaces)
listen_address = "0.0.0.0:9000"

# Public address (auto-detected if not set)
public_address = "203.0.113.1:9000"

# IPv6 support
ipv6_enabled = true

# Connection limits and timeouts
max_connections = 10000
connection_timeout = 30
keepalive_interval = 60
```

### Security Configuration

```toml
[security]
# Rate limiting
rate_limit = 1000              # requests per second per IP
connection_limit = 100         # max connections per IP

# Encryption settings
encryption_enabled = true
min_tls_version = "1.3"       # "1.2" or "1.3"

# Identity security
identity_security_level = "High"  # "Low", "Medium", or "High"
```

### Storage Configuration

```toml
[storage]
# Storage paths and limits
path = "./data"
max_size = "10GB"            # Supports: B, KB, MB, GB, TB
cache_size = 256             # MB
compression_enabled = true
```

### MCP (Model Context Protocol) Configuration

```toml
[mcp]
# MCP server settings
enabled = true
port = 9001
max_execution_time = 300     # seconds
monitoring_enabled = true
```

### DHT Configuration

```toml
[dht]
# Kademlia DHT parameters
replication_factor = 8       # K value
alpha = 3                    # Parallel queries
beta = 1                     # Routing optimization
record_ttl = 3600           # seconds
adaptive_routing = true
```

### Transport Configuration

```toml
[transport]
# Transport preferences
protocol = "quic"           # "quic", "tcp", or "webrtc"
quic_enabled = true
tcp_enabled = true
webrtc_enabled = false
buffer_size = 65536         # bytes
```

### Identity Configuration

```toml
[identity]
# Key management
derivation_path = "m/44'/0'/0'/0/0"
rotation_interval = 90      # days (0 = disabled)
backup_enabled = true
backup_interval = 24        # hours
```

## Configuration Profiles

### Development Profile

Use `config.development.toml` for local development:
- Localhost addresses
- Relaxed security limits
- Smaller storage requirements
- Faster timeouts

### Production Profile

Use `config.production.toml` for production deployments:
- Public addresses with proper bootstrap nodes
- Strict security settings
- Large storage allocations
- Optimized performance settings

## Validation

The configuration system performs automatic validation:
- Network addresses are checked for validity
- Storage sizes must use valid units
- Numeric values are range-checked
- Required fields are verified

## Examples

### Minimal Configuration

```toml
[network]
listen_address = "0.0.0.0:9000"
```

### Custom Bootstrap Configuration

```toml
[network]
bootstrap_nodes = [
    "10.0.0.1:9000",
    "10.0.0.2:9000",
    "/ip4/10.0.0.3/tcp/9000",
]
listen_address = "10.0.0.100:9000"
```

### High-Security Configuration

```toml
[security]
rate_limit = 100
connection_limit = 10
encryption_enabled = true
min_tls_version = "1.3"
identity_security_level = "High"

[transport]
protocol = "quic"
quic_enabled = true
tcp_enabled = false
```

## Programmatic Usage

```rust
use saorsa_core::config::Config;
use saorsa_core::network::NodeConfig;

// Load default configuration
let config = Config::load()?;

// Load from specific file
let config = Config::load_with_path("custom.toml")?;

// Create node configuration
let node_config = NodeConfig::from_config(&config)?;

// Use development profile
let dev_config = Config::development();

// Use production profile
let prod_config = Config::production();
```

## Migration from Hardcoded Values

If you're upgrading from a version with hardcoded addresses:

1. Create a configuration file based on the examples
2. Replace hardcoded addresses with configuration values
3. Set appropriate environment variables for deployment
4. Test with different profiles before production deployment

## Troubleshooting

### Common Issues

1. **Invalid address format**: Ensure addresses are in `host:port` format
2. **Permission denied**: Check file permissions for storage path
3. **Port already in use**: Change the listen address port
4. **Bootstrap connection failed**: Verify bootstrap nodes are accessible

### Debug Configuration

Enable debug logging to see configuration loading:

```bash
RUST_LOG=saorsa_core::config=debug saorsa
```

This will show:
- Which configuration file was loaded
- Environment variable overrides applied
- Validation results