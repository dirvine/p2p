# Communitas CLI - Bootstrap Node Documentation

## Overview

The Communitas CLI has been enhanced to function as a full-featured P2P bootstrap node with:
- **DHT Storage**: Distributed hash table with 8x replication
- **Geographic Routing**: Region-aware peer selection and routing
- **MCP Server**: Remote management via Model Context Protocol
- **Persistent Storage**: Disk-based storage for DHT data
- **Multi-region Support**: Optimized for global deployment

## Quick Start

### Build the CLI
```bash
cd apps/communitas-cli
cargo build --release
```

### Run as Bootstrap Node
```bash
./target/release/communitas bootstrap \
  --port 9001 \
  --mcp-port 9090 \
  --region EU \
  --storage-mb 10240 \
  --api-token $(openssl rand -hex 32)
```

## Command Reference

### Bootstrap Mode

Start a full bootstrap node with all features:

```bash
communitas bootstrap [OPTIONS]
```

Options:
- `--port <PORT>`: P2P listening port (default: 9001)
- `--mcp-port <PORT>`: MCP server port (default: 9090)
- `--region <REGION>`: Geographic region (NA, EU, AP, SA, AF, OC)
- `--bootstrap <ADDR>`: Bootstrap nodes to connect to (can specify multiple)
- `--storage-mb <SIZE>`: Storage capacity in MB (default: 10240)
- `--api-token <TOKEN>`: API token for MCP access (or use env: COMMUNITAS_API_TOKEN)
- `--persistent <BOOL>`: Enable persistent storage (default: true)

### DHT Commands

Manage distributed hash table storage:

```bash
# Store a value
communitas dht put <KEY> <VALUE> [--ttl <SECONDS>] [--encrypt]

# Retrieve a value
communitas dht get <KEY> [--output <FILE>] [--decrypt]

# Delete a value
communitas dht delete <KEY>

# List stored keys
communitas dht list [--prefix <PREFIX>] [--limit <N>] [--detailed]

# Show statistics
communitas dht stats [--detailed] [--format json|table]

# Find closest nodes
communitas dht find-node <KEY> [--count <N>]

# Replicate data
communitas dht replicate <KEY|all> [--factor <N>]

# Verify integrity
communitas dht verify <KEY|all> [--repair]

# Export data
communitas dht export <OUTPUT> [--format json|binary] [--metadata]

# Import data
communitas dht import <INPUT> [--overwrite] [--validate]

# Manage buckets
communitas dht buckets [--show] [--refresh] [--compact]

# Configure DHT
communitas dht config [--replication <N>] [--capacity <MB>] [--show]
```

### Geographic Routing Commands

Manage geographic-aware routing:

```bash
# Show status
communitas geo status [--detailed]

# List peers by region
communitas geo peers [--region <REGION>] [--detailed]

# Show regional statistics
communitas geo stats [--format json|table]

# Configure geographic routing
communitas geo config [--region <REGION>] [--cross-region <BOOL>] [--show]

# Test connectivity
communitas geo test <REGION> [--count <N>]

# Optimize routing
communitas geo optimize [--dry-run]

# Show latency map
communitas geo latency [--matrix]
```

### Health & Monitoring

```bash
# Health check
communitas health [--target <ADDR>] [--dht] [--geo]

# Export data
communitas export --export-type <dht|config|peers> <OUTPUT>

# Import data
communitas import --import-type <dht|config|peers> <INPUT>
```

## MCP API Reference

The MCP server provides JSON-RPC style API for remote management:

### Endpoints

#### Node Status
```json
{
  "method": "node/status"
}
```

#### DHT Operations
```json
{
  "method": "dht/put",
  "key": "example-key",
  "value": "example-value",
  "ttl": 86400
}

{
  "method": "dht/get",
  "key": "example-key"
}

{
  "method": "dht/stats"
}
```

#### Geographic Operations
```json
{
  "method": "geo/status"
}

{
  "method": "geo/peers",
  "region": "EU"
}
```

### Authentication

Include API token in request:
```json
{
  "auth": "your-api-token-here",
  "method": "node/status"
}
```

## DigitalOcean Deployment

### Prerequisites

1. DigitalOcean account with API access
2. SSH key configured (included in deployment config)
3. Built CLI binary

### Deploy Bootstrap Nodes

```bash
# Deploy to multiple regions
./deploy-bootstrap-nodes.sh

# Or deploy manually to a specific droplet
doctl compute droplet create \
  communitas-bootstrap-eu-1 \
  --region ams3 \
  --size s-2vcpu-4gb \
  --image ubuntu-24-04-x64 \
  --ssh-keys YOUR_KEY_ID \
  --user-data-file user-data.sh
```

### Configuration Files

#### Bootstrap Configuration (bootstrap.toml)
```toml
[network]
port = 9001
bootstrap_mode = true
max_connections = 1000

[dht]
replication_factor = 8
storage_capacity_mb = 10240
persistent_storage = true
storage_path = "/opt/communitas/data/dht"

[geographic]
local_region = "Europe"
cross_region_optimization = true
latency_threshold_ms = 150

[mcp]
enabled = true
port = 9090
auth_required = true
```

### Monitoring

```bash
# Check node health
curl -k -H "Auth: YOUR_TOKEN" https://NODE_IP:9090/health

# Get DHT statistics
curl -k -H "Auth: YOUR_TOKEN" https://NODE_IP:9090/dht/stats

# Monitor via systemd
ssh root@NODE_IP "systemctl status communitas-bootstrap"
ssh root@NODE_IP "journalctl -u communitas-bootstrap -f"
```

## Geographic Regions

The system supports 6 geographic regions:

- **NA** (North America): NYC, SFO, TOR
- **EU** (Europe): AMS, FRA, LON
- **AP** (Asia Pacific): SGP, BLR, SYD
- **SA** (South America): SAO
- **AF** (Africa): JNB
- **OC** (Oceania): SYD

## Storage Architecture

### DHT Storage
- **Replication Factor**: 8 (configurable)
- **Storage Backend**: Disk-based with memory cache
- **Record TTL**: 24 hours default
- **Capacity**: 10GB default per node

### Data Persistence
- **Location**: `/opt/communitas/data/dht/`
- **Format**: Binary files with hash-based naming
- **Index**: In-memory with periodic snapshots
- **Backup**: Daily backups to DigitalOcean Spaces

## Security

### MCP Authentication
- Token-based authentication
- TLS encryption for MCP connections
- IP whitelisting support

### Network Security
- Firewall rules for P2P and MCP ports
- DDoS protection
- Rate limiting

## Performance

### Benchmarks
- **DHT Operations**: ~1ms local, ~50ms cross-region
- **Storage Capacity**: 10GB per node
- **Concurrent Connections**: 1000 per node
- **Replication Speed**: ~100MB/s local network

### Optimization Tips
1. Use SSD storage for better I/O performance
2. Increase memory for larger DHT caches
3. Deploy nodes in multiple regions for redundancy
4. Use load balancers for high-traffic scenarios

## Troubleshooting

### Common Issues

#### Port Already in Use
```bash
# Check what's using the port
lsof -i :9001

# Use a different port
communitas bootstrap --port 9002
```

#### Storage Full
```bash
# Check disk usage
df -h /opt/communitas/data

# Clean old records
communitas dht verify all --repair
```

#### Connection Issues
```bash
# Test connectivity
communitas geo test EU --count 5

# Check firewall
ufw status
```

## Development

### Running Tests
```bash
./test-bootstrap-cli.sh
```

### Building for Production
```bash
cargo build --release --features "network"
strip target/release/communitas
```

### Docker Deployment
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin communitas

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/communitas /usr/local/bin/
EXPOSE 9001 9090
CMD ["communitas", "bootstrap"]
```

## Support

For issues or questions:
- GitHub: https://github.com/saorsalabs/p2p
- Documentation: /docs/
- MCP API Docs: /docs/mcp-api.md