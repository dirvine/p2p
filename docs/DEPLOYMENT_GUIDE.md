# P2P Foundation Deployment Guide

## Overview

This guide covers deployment of P2P Foundation nodes in various environments, from single-node development setups to large-scale production networks.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Development Deployment](#development-deployment)
4. [Production Deployment](#production-deployment)
5. [Docker Deployment](#docker-deployment)
6. [Kubernetes Deployment](#kubernetes-deployment)
7. [Configuration](#configuration)
8. [Monitoring](#monitoring)
9. [Security](#security)
10. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

**Minimum:**
- CPU: 2 cores
- RAM: 4GB
- Storage: 50GB SSD
- Network: 10 Mbps symmetric

**Recommended:**
- CPU: 4+ cores
- RAM: 16GB
- Storage: 500GB NVMe SSD
- Network: 100 Mbps symmetric

### Software Requirements

- Rust 1.70+ (for building from source)
- Docker 20.10+ (for container deployment)
- systemd (for service management)

### Network Requirements

- IPv6 support (IPv4 fallback available)
- Open ports:
  - TCP 8000: P2P communication
  - TCP 8001: HTTP API (optional)
  - TCP 9090: Metrics (optional)
  - UDP 8000: QUIC transport

## Quick Start

### Binary Installation

```bash
# Download latest release
curl -L https://github.com/yourusername/p2p-foundation/releases/latest/download/p2p-node -o p2p-node
chmod +x p2p-node

# Generate identity
./p2p-node identity generate

# Start node
./p2p-node start --bootstrap seed1.network.com:8000,seed2.network.com:8000
```

### Building from Source

```bash
# Clone repository
git clone https://github.com/yourusername/p2p-foundation.git
cd p2p-foundation

# Build release binary
cargo build --release

# Install
sudo cp target/release/p2p-node /usr/local/bin/
```

## Development Deployment

### Local Network Setup

```bash
# Start bootstrap node
p2p-node start \
  --port 8000 \
  --data-dir ./node0 \
  --log-level debug

# Start additional nodes
for i in {1..3}; do
  p2p-node start \
    --port $((8000 + i)) \
    --data-dir ./node$i \
    --bootstrap localhost:8000 \
    --log-level debug &
done
```

### Development Configuration

Create `config.toml`:

```toml
[node]
identity_file = "identity.json"
data_dir = "./data"
port = 8000

[network]
bootstrap_nodes = ["localhost:8000"]
max_connections = 100
enable_ipv6 = true

[storage]
capacity_gb = 10
cache_size_mb = 1024
compression = true
encryption = false

[development]
enable_debug_api = true
mock_latency_ms = 0
disable_proof_of_work = true
```

## Production Deployment

### System Preparation

```bash
# Create dedicated user
sudo useradd -r -s /bin/false p2p-node

# Create directories
sudo mkdir -p /etc/p2p-foundation /var/lib/p2p-foundation /var/log/p2p-foundation
sudo chown p2p-node:p2p-node /var/lib/p2p-foundation /var/log/p2p-foundation

# Set up systemd service
sudo cp deployment/p2p-node.service /etc/systemd/system/
sudo systemctl daemon-reload
```

### Systemd Service File

`/etc/systemd/system/p2p-node.service`:

```ini
[Unit]
Description=P2P Foundation Node
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
User=p2p-node
Group=p2p-node
ExecStart=/usr/local/bin/p2p-node start --config /etc/p2p-foundation/config.toml
Restart=always
RestartSec=10
Environment="RUST_LOG=info"

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/p2p-foundation /var/log/p2p-foundation

# Resource limits
LimitNOFILE=65536
MemoryLimit=4G
CPUQuota=200%

[Install]
WantedBy=multi-user.target
```

### Production Configuration

`/etc/p2p-foundation/config.toml`:

```toml
[node]
identity_file = "/var/lib/p2p-foundation/identity.json"
data_dir = "/var/lib/p2p-foundation/data"
port = 8000

[network]
bootstrap_nodes = [
  "seed1.network.com:8000",
  "seed2.network.com:8000",
  "seed3.network.com:8000"
]
max_connections = 1000
connection_timeout_secs = 30
enable_ipv6 = true

[storage]
capacity_gb = 500
cache_size_mb = 8192
compression = true
encryption = true
chunk_size_kb = 1024

[security]
enable_rate_limiting = true
max_requests_per_minute = 1000
blacklist_threshold = 10
enable_ddos_protection = true

[performance]
worker_threads = 8
io_threads = 4
max_concurrent_operations = 10000

[monitoring]
enable_metrics = true
metrics_port = 9090
enable_tracing = true
jaeger_endpoint = "http://localhost:14268/api/traces"

[logging]
level = "info"
file = "/var/log/p2p-foundation/node.log"
max_size_mb = 100
max_backups = 10
```

### Starting the Service

```bash
# Enable and start
sudo systemctl enable p2p-node
sudo systemctl start p2p-node

# Check status
sudo systemctl status p2p-node

# View logs
sudo journalctl -u p2p-node -f
```

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/p2p-node /usr/local/bin/

RUN useradd -r -s /bin/false p2p-node
USER p2p-node

EXPOSE 8000/tcp 8000/udp 8001/tcp 9090/tcp

VOLUME ["/data"]
ENTRYPOINT ["p2p-node"]
CMD ["start", "--data-dir", "/data"]
```

### Docker Compose

```yaml
version: '3.8'

services:
  bootstrap:
    image: p2p-foundation:latest
    ports:
      - "8000:8000"
      - "8000:8000/udp"
    volumes:
      - bootstrap-data:/data
    environment:
      - RUST_LOG=info
    command: start --bootstrap-mode --data-dir /data

  node1:
    image: p2p-foundation:latest
    depends_on:
      - bootstrap
    volumes:
      - node1-data:/data
    environment:
      - RUST_LOG=info
    command: start --bootstrap bootstrap:8000 --data-dir /data

  node2:
    image: p2p-foundation:latest
    depends_on:
      - bootstrap
    volumes:
      - node2-data:/data
    environment:
      - RUST_LOG=info
    command: start --bootstrap bootstrap:8000 --data-dir /data

  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus

volumes:
  bootstrap-data:
  node1-data:
  node2-data:
  prometheus-data:
```

## Kubernetes Deployment

### StatefulSet

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: p2p-node
spec:
  serviceName: p2p-network
  replicas: 3
  selector:
    matchLabels:
      app: p2p-node
  template:
    metadata:
      labels:
        app: p2p-node
    spec:
      containers:
      - name: p2p-node
        image: p2p-foundation:latest
        ports:
        - containerPort: 8000
          name: p2p-tcp
        - containerPort: 8000
          protocol: UDP
          name: p2p-udp
        - containerPort: 9090
          name: metrics
        env:
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: BOOTSTRAP_NODES
          value: "p2p-node-0.p2p-network:8000"
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            memory: "2Gi"
            cpu: "1"
          limits:
            memory: "4Gi"
            cpu: "2"
        livenessProbe:
          httpGet:
            path: /health
            port: 8001
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8001
          initialDelaySeconds: 5
          periodSeconds: 5
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: "fast-ssd"
      resources:
        requests:
          storage: 100Gi
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: p2p-network
spec:
  clusterIP: None
  selector:
    app: p2p-node
  ports:
  - port: 8000
    name: p2p-tcp
  - port: 8000
    protocol: UDP
    name: p2p-udp
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `P2P_CONFIG` | Config file path | `./config.toml` |
| `P2P_DATA_DIR` | Data directory | `./data` |
| `P2P_PORT` | Listen port | `8000` |
| `P2P_BOOTSTRAP` | Bootstrap nodes | - |

### Configuration Options

See [CONFIG_REFERENCE.md](CONFIG_REFERENCE.md) for complete configuration documentation.

## Monitoring

### Prometheus Metrics

Available at `http://localhost:9090/metrics`:

- `p2p_connected_peers` - Number of connected peers
- `p2p_storage_used_bytes` - Storage space used
- `p2p_messages_sent_total` - Total messages sent
- `p2p_messages_received_total` - Total messages received
- `p2p_routing_success_rate` - Routing success percentage
- `p2p_cache_hit_rate` - Cache hit percentage

### Grafana Dashboard

Import dashboard from `deployment/grafana-dashboard.json`.

### Health Checks

```bash
# Basic health check
curl http://localhost:8001/health

# Detailed status
curl http://localhost:8001/status

# Metrics
curl http://localhost:9090/metrics
```

## Security

### Firewall Configuration

```bash
# Allow P2P traffic
sudo ufw allow 8000/tcp
sudo ufw allow 8000/udp

# Allow metrics (internal only)
sudo ufw allow from 10.0.0.0/8 to any port 9090

# Allow API (internal only)
sudo ufw allow from 10.0.0.0/8 to any port 8001
```

### TLS Configuration

For production, enable TLS:

```toml
[security.tls]
enabled = true
cert_file = "/etc/p2p-foundation/cert.pem"
key_file = "/etc/p2p-foundation/key.pem"
ca_file = "/etc/p2p-foundation/ca.pem"
verify_peer = true
```

### Key Management

```bash
# Generate new identity with hardware security module
p2p-node identity generate --hsm /dev/ttyUSB0

# Backup identity
p2p-node identity export --output identity-backup.enc

# Restore identity
p2p-node identity import --input identity-backup.enc
```

## Troubleshooting

### Common Issues

#### Node Won't Start

```bash
# Check logs
journalctl -u p2p-node -n 100

# Verify identity file
p2p-node identity verify

# Test network connectivity
p2p-node network test --bootstrap seed1.network.com:8000
```

#### High Memory Usage

```bash
# Check cache size
p2p-node cache stats

# Clear cache
p2p-node cache clear --older-than 7d

# Reduce cache size in config
```

#### Poor Connectivity

```bash
# Check peer connections
p2p-node peers list

# Test NAT traversal
p2p-node network nat-test

# Enable UPnP
p2p-node network enable-upnp
```

### Debug Commands

```bash
# Enable debug logging
export RUST_LOG=debug

# Dump routing table
p2p-node debug routing-table

# Show trust scores
p2p-node debug trust-scores

# Performance profiling
p2p-node debug profile --duration 60s
```

### Recovery Procedures

#### Corrupted Database

```bash
# Stop node
sudo systemctl stop p2p-node

# Backup current data
sudo cp -r /var/lib/p2p-foundation /var/lib/p2p-foundation.backup

# Run recovery
p2p-node repair --data-dir /var/lib/p2p-foundation

# Restart
sudo systemctl start p2p-node
```

#### Identity Recovery

```bash
# From backup
p2p-node identity import --input identity-backup.enc

# From seed phrase (if enabled)
p2p-node identity recover --words "word1 word2 word3..."
```

## Performance Tuning

### Linux Kernel Parameters

`/etc/sysctl.d/99-p2p-node.conf`:

```
# Network optimizations
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 87380 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_congestion_control = bbr

# File descriptors
fs.file-max = 1000000

# Memory
vm.swappiness = 10
```

### Application Tuning

```toml
[performance]
# Use all CPU cores
worker_threads = 0  # 0 = auto-detect

# Increase batch sizes
batch_size = 1000

# Enable compression for large transfers
compression_threshold_bytes = 10240

# Connection pooling
connection_pool_size = 100
connection_idle_timeout_secs = 300
```

## Maintenance

### Regular Tasks

```bash
# Weekly: Clean old logs
find /var/log/p2p-foundation -name "*.log" -mtime +30 -delete

# Monthly: Optimize database
p2p-node maintenance optimize

# Quarterly: Update bootstrap nodes
p2p-node config update-bootstrap

# Yearly: Rotate identity keys
p2p-node identity rotate
```

### Backup Procedures

```bash
#!/bin/bash
# backup-p2p-node.sh

NODE_DIR="/var/lib/p2p-foundation"
BACKUP_DIR="/backup/p2p-foundation"
DATE=$(date +%Y%m%d-%H%M%S)

# Stop node
systemctl stop p2p-node

# Create backup
tar -czf "$BACKUP_DIR/p2p-node-$DATE.tar.gz" -C "$NODE_DIR" .

# Start node
systemctl start p2p-node

# Keep last 30 days
find "$BACKUP_DIR" -name "p2p-node-*.tar.gz" -mtime +30 -delete
```

## Support

- Documentation: https://docs.p2p-foundation.org
- Community Forum: https://forum.p2p-foundation.org
- GitHub Issues: https://github.com/yourusername/p2p-foundation/issues
- Commercial Support: support@p2p-foundation.org