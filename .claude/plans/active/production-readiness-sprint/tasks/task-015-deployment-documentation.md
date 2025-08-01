# Task 015: Production Deployment Documentation

## Overview
Create comprehensive deployment documentation including installation guides, configuration references, operational procedures, and troubleshooting guides for production deployment.

## Acceptance Criteria
- [ ] Deployment guide complete
- [ ] Configuration reference documented
- [ ] Operational runbook created
- [ ] Troubleshooting guide written
- [ ] Rollback procedures tested

## Technical Details

### 1. Deployment Guide Structure

Create `docs/DEPLOYMENT_GUIDE.md`:

```markdown
# P2P Foundation Deployment Guide

## Table of Contents
1. [System Requirements](#system-requirements)
2. [Pre-deployment Checklist](#pre-deployment-checklist)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Starting the Node](#starting-the-node)
6. [Verification](#verification)
7. [Monitoring Setup](#monitoring-setup)
8. [Backup Procedures](#backup-procedures)
9. [Upgrade Process](#upgrade-process)
10. [Rollback Procedures](#rollback-procedures)

## System Requirements

### Hardware
- CPU: 2+ cores (4+ recommended)
- RAM: 4GB minimum (8GB recommended)
- Disk: 50GB SSD (100GB recommended)
- Network: 100Mbps symmetric minimum

### Software
- OS: Ubuntu 20.04 LTS or newer
- Rust: 1.75.0 or newer (if building from source)

### Network
- Ports required:
  - 30303: P2P communication (TCP/UDP)
  - 9090: Metrics endpoint (TCP)
  - 8080: Health checks (TCP)
- IPv6 support recommended

## Pre-deployment Checklist

- [ ] System requirements verified
- [ ] Firewall rules configured
- [ ] DNS entries created (if applicable)
- [ ] SSL certificates obtained (if applicable)
- [ ] Backup strategy defined
- [ ] Monitoring infrastructure ready
- [ ] Rollback plan documented

## Installation

### Binary Installation

```bash
# Download latest release
wget https://github.com/your-org/p2p-foundation/releases/latest/download/p2p-node-linux-amd64.tar.gz

# Verify checksum
sha256sum -c p2p-node-linux-amd64.tar.gz.sha256

# Extract
tar -xzf p2p-node-linux-amd64.tar.gz

# Install
sudo mv p2p-node /usr/local/bin/
sudo chmod +x /usr/local/bin/p2p-node

# Create service user
sudo useradd -r -s /bin/false p2p-node

# Create directories
sudo mkdir -p /etc/p2p-foundation
sudo mkdir -p /var/lib/p2p-foundation
sudo chown p2p-node:p2p-node /var/lib/p2p-foundation
```

### Systemd Service

Create `/etc/systemd/system/p2p-node.service`:

```ini
[Unit]
Description=P2P Foundation Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=p2p-node
Group=p2p-node
ExecStart=/usr/local/bin/p2p-node --config /etc/p2p-foundation/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=p2p-node

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/p2p-foundation

# Resource limits
LimitNOFILE=65536
MemoryLimit=8G
CPUQuota=400%

[Install]
WantedBy=multi-user.target
```

## Configuration

### Basic Configuration

Create `/etc/p2p-foundation/config.toml`:

```toml
# Node Configuration
[node]
name = "prod-node-01"
data_dir = "/var/lib/p2p-foundation"

# Network Configuration
[network]
listen_address = "0.0.0.0"
port = 30303
max_connections = 1000
connection_timeout_secs = 30

# DHT Configuration
[dht]
replication_factor = 3
storage_max_size_gb = 50
cleanup_interval_secs = 3600

# Identity Configuration
[identity]
key_file = "/var/lib/p2p-foundation/node.key"
three_words_cache = true

# Monitoring Configuration
[monitoring]
metrics_enabled = true
metrics_port = 9090
health_check_port = 8080

# Logging Configuration
[logging]
level = "info"
format = "json"
file = "/var/log/p2p-foundation/node.log"
max_size_mb = 100
max_backups = 10

# Security Configuration
[security]
tls_enabled = true
tls_cert_file = "/etc/p2p-foundation/cert.pem"
tls_key_file = "/etc/p2p-foundation/key.pem"
min_tls_version = "1.3"
```

### Environment-Specific Configs

Production overrides in `/etc/p2p-foundation/production.toml`:

```toml
[node]
environment = "production"

[network]
bootstrap_nodes = [
    "/ip6/2001:db8::1/tcp/30303/p2p/QmBootstrap1...",
    "/ip6/2001:db8::2/tcp/30303/p2p/QmBootstrap2..."
]

[monitoring]
telemetry_enabled = true
telemetry_endpoint = "https://telemetry.your-org.com"
```

## Starting the Node

```bash
# Enable service
sudo systemctl enable p2p-node

# Start service
sudo systemctl start p2p-node

# Check status
sudo systemctl status p2p-node

# View logs
sudo journalctl -u p2p-node -f
```

## Verification

### Health Checks

```bash
# Liveness check
curl http://localhost:8080/health/live

# Readiness check
curl http://localhost:8080/health/ready

# Metrics
curl http://localhost:9090/metrics
```

### Network Connectivity

```bash
# Check peer connections
p2p-node peers list

# Verify DHT functionality
p2p-node dht stats

# Test identity resolution
p2p-node identity resolve "word1-word2-word3"
```

## Monitoring Setup

### Prometheus Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'p2p-node'
    static_configs:
      - targets: ['node1:9090', 'node2:9090']
    relabel_configs:
      - source_labels: [__address__]
        regex: '([^:]+):.*'
        target_label: instance
```

### Alerting Rules

Create `alerts.yml`:

```yaml
groups:
  - name: p2p_node
    rules:
      - alert: NodeDown
        expr: up{job="p2p-node"} == 0
        for: 5m
        annotations:
          summary: "P2P node {{ $labels.instance }} is down"
      
      - alert: HighConnectionCount
        expr: p2p_connections_active > 900
        for: 10m
        annotations:
          summary: "Node {{ $labels.instance }} near connection limit"
      
      - alert: DHTUnhealthy
        expr: p2p_dht_replication_factor < 3
        for: 15m
        annotations:
          summary: "DHT replication degraded on {{ $labels.instance }}"
```

## Backup Procedures

### Automated Backups

Create `/usr/local/bin/p2p-backup.sh`:

```bash
#!/bin/bash
set -euo pipefail

BACKUP_DIR="/backup/p2p-foundation"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="${BACKUP_DIR}/backup_${TIMESTAMP}"

# Create backup directory
mkdir -p "${BACKUP_PATH}"

# Stop writes (optional - implement in app)
curl -X POST http://localhost:8080/admin/read-only

# Backup data
rsync -av /var/lib/p2p-foundation/ "${BACKUP_PATH}/"

# Resume writes
curl -X POST http://localhost:8080/admin/read-write

# Compress
tar -czf "${BACKUP_PATH}.tar.gz" -C "${BACKUP_DIR}" "backup_${TIMESTAMP}"
rm -rf "${BACKUP_PATH}"

# Cleanup old backups (keep 7 days)
find "${BACKUP_DIR}" -name "backup_*.tar.gz" -mtime +7 -delete

echo "Backup completed: ${BACKUP_PATH}.tar.gz"
```

Add to crontab:
```bash
0 2 * * * /usr/local/bin/p2p-backup.sh
```

## Upgrade Process

### Rolling Upgrade

```bash
# 1. Download new version
wget https://github.com/your-org/p2p-foundation/releases/download/v1.1.0/p2p-node-linux-amd64.tar.gz

# 2. Verify compatibility
p2p-node --version
p2p-node upgrade-check v1.1.0

# 3. Create backup
/usr/local/bin/p2p-backup.sh

# 4. Install new binary
sudo systemctl stop p2p-node
sudo cp /usr/local/bin/p2p-node /usr/local/bin/p2p-node.backup
sudo tar -xzf p2p-node-linux-amd64.tar.gz -C /usr/local/bin/

# 5. Start with new version
sudo systemctl start p2p-node

# 6. Verify
curl http://localhost:8080/health/ready
```

## Rollback Procedures

### Quick Rollback

```bash
# 1. Stop current version
sudo systemctl stop p2p-node

# 2. Restore previous binary
sudo mv /usr/local/bin/p2p-node.backup /usr/local/bin/p2p-node

# 3. Start previous version
sudo systemctl start p2p-node

# 4. Verify
curl http://localhost:8080/health/ready
```

### Full Rollback

```bash
# 1. Stop node
sudo systemctl stop p2p-node

# 2. Restore from backup
LATEST_BACKUP=$(ls -t /backup/p2p-foundation/backup_*.tar.gz | head -1)
tar -xzf "${LATEST_BACKUP}" -C /tmp/
rsync -av --delete /tmp/backup_*/ /var/lib/p2p-foundation/

# 3. Restore binary
sudo mv /usr/local/bin/p2p-node.backup /usr/local/bin/p2p-node

# 4. Start node
sudo systemctl start p2p-node
```
```

### 2. Configuration Reference

Create `docs/CONFIGURATION_REFERENCE.md`:

```markdown
# Configuration Reference

## Complete Configuration Options

```toml
# Node Configuration
[node]
# Node identifier (required)
name = "string"

# Environment: development, staging, production
environment = "production"

# Data directory path
data_dir = "/path/to/data"

# Network Configuration
[network]
# IP address to listen on (0.0.0.0 for all)
listen_address = "0.0.0.0"

# Port number (1024-65535)
port = 30303

# Maximum concurrent connections
max_connections = 1000

# Connection timeout in seconds
connection_timeout_secs = 30

# Message size limit in bytes
max_message_size = 1048576

# Bootstrap nodes (array of multiaddrs)
bootstrap_nodes = [
    "/ip6/::1/tcp/30303/p2p/Qm..."
]

# DHT Configuration
[dht]
# Replication factor (minimum 3)
replication_factor = 3

# Maximum storage size in GB
storage_max_size_gb = 50

# Cleanup interval in seconds
cleanup_interval_secs = 3600

# Cache size in MB
cache_size_mb = 512

# Identity Configuration
[identity]
# Path to node identity key file
key_file = "/path/to/node.key"

# Enable three-words cache
three_words_cache = true

# Passkey timeout in seconds
passkey_timeout_secs = 300

# Monitoring Configuration
[monitoring]
# Enable metrics collection
metrics_enabled = true

# Metrics HTTP port
metrics_port = 9090

# Health check HTTP port  
health_check_port = 8080

# Enable OpenTelemetry
telemetry_enabled = false

# OpenTelemetry endpoint
telemetry_endpoint = "http://localhost:4317"

# Logging Configuration
[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: text, json
format = "json"

# Log file path (optional)
file = "/var/log/p2p-foundation/node.log"

# Maximum log file size in MB
max_size_mb = 100

# Maximum number of backup files
max_backups = 10

# Security Configuration
[security]
# Enable TLS
tls_enabled = true

# TLS certificate file
tls_cert_file = "/path/to/cert.pem"

# TLS key file
tls_key_file = "/path/to/key.pem"

# Minimum TLS version: "1.2" or "1.3"
min_tls_version = "1.3"

# Advanced Configuration
[advanced]
# Worker thread count (0 = CPU count)
worker_threads = 0

# Database connection pool size
db_pool_size = 32

# Enable experimental features
experimental_features = false
```

## Environment Variables

All configuration options can be overridden with environment variables:

```bash
# Format: P2P_SECTION_KEY
P2P_NODE_NAME=prod-01
P2P_NETWORK_PORT=30303
P2P_MONITORING_METRICS_ENABLED=true
```

## Configuration Precedence

1. Command line arguments
2. Environment variables
3. Configuration file
4. Default values
```

### 3. Operational Runbook

Create `docs/OPERATIONAL_RUNBOOK.md`:

```markdown
# Operational Runbook

## Daily Operations

### Health Monitoring
```bash
# Check all nodes
for node in node1 node2 node3; do
    echo "Checking $node"
    curl -s http://$node:8080/health/ready | jq .
done
```

### Log Monitoring
```bash
# Check for errors
journalctl -u p2p-node --since "1 hour ago" | grep ERROR

# Connection statistics
journalctl -u p2p-node --since "1 hour ago" | grep "connection" | wc -l
```

## Common Operations

### Adding New Node
1. Provision hardware
2. Install OS and dependencies
3. Copy configuration template
4. Generate new node identity
5. Update bootstrap nodes
6. Start node service
7. Verify connectivity

### Removing Node
1. Notify cluster of planned removal
2. Wait for data migration
3. Stop node service
4. Remove from monitoring
5. Archive node data

### Updating Configuration
1. Edit configuration file
2. Validate configuration
3. Reload service
4. Verify changes applied

## Emergency Procedures

### Node Failure
1. Check node status
2. Review logs for errors
3. Attempt restart
4. If persistent, replace node
5. Restore from backup if needed

### Network Partition
1. Identify affected nodes
2. Check network connectivity
3. Review firewall rules
4. Heal partition
5. Verify data consistency

### High Load
1. Check metrics
2. Identify bottleneck
3. Scale horizontally if needed
4. Optimize configuration
5. Monitor recovery

## Maintenance Windows

### Weekly Maintenance
- Log rotation
- Metrics cleanup
- Security updates
- Performance review

### Monthly Maintenance
- Full backup verification
- Certificate renewal check
- Capacity planning review
- Security audit

## Incident Response

### Severity Levels
- P1: Complete outage
- P2: Degraded performance
- P3: Minor issues
- P4: Informational

### Response Times
- P1: 15 minutes
- P2: 1 hour
- P3: 4 hours
- P4: Next business day

### Escalation Path
1. On-call engineer
2. Team lead
3. Engineering manager
4. CTO
```

### 4. Troubleshooting Guide

Create `docs/TROUBLESHOOTING_GUIDE.md`:

```markdown
# Troubleshooting Guide

## Common Issues

### Node Won't Start

**Symptom**: Service fails to start or crashes immediately

**Diagnosis**:
```bash
# Check service status
sudo systemctl status p2p-node

# Check logs
sudo journalctl -u p2p-node -n 100

# Verify configuration
p2p-node --config /etc/p2p-foundation/config.toml validate
```

**Common Causes**:
1. **Port already in use**
   ```bash
   sudo lsof -i :30303
   # Kill conflicting process or change port
   ```

2. **Permission issues**
   ```bash
   # Fix ownership
   sudo chown -R p2p-node:p2p-node /var/lib/p2p-foundation
   ```

3. **Corrupted identity file**
   ```bash
   # Regenerate identity
   sudo -u p2p-node p2p-node identity generate
   ```

### No Peer Connections

**Symptom**: Node running but no peers connected

**Diagnosis**:
```bash
# Check connectivity
p2p-node peers list

# Test bootstrap nodes
for node in $(grep bootstrap /etc/p2p-foundation/config.toml); do
    nc -zv $node 30303
done
```

**Solutions**:
1. Check firewall rules
2. Verify bootstrap nodes are reachable
3. Check NAT configuration
4. Review network logs

### High Memory Usage

**Symptom**: Memory usage growing unbounded

**Diagnosis**:
```bash
# Check memory stats
ps aux | grep p2p-node
cat /proc/$(pgrep p2p-node)/status | grep -E "VmRSS|VmSize"

# Check cache sizes
curl http://localhost:9090/metrics | grep cache
```

**Solutions**:
1. Reduce cache sizes in config
2. Lower connection limits
3. Enable memory profiling
4. Check for memory leaks

### DHT Not Functioning

**Symptom**: DHT operations failing or timing out

**Diagnosis**:
```bash
# Check DHT stats
p2p-node dht stats

# Test DHT operations
p2p-node dht put test-key test-value
p2p-node dht get test-key
```

**Solutions**:
1. Verify sufficient peers (min 3)
2. Check storage space
3. Review DHT configuration
4. Restart DHT subsystem

## Performance Issues

### Slow Response Times

**Diagnosis**:
```bash
# Check CPU usage
top -p $(pgrep p2p-node)

# Review metrics
curl http://localhost:9090/metrics | grep duration
```

**Optimization**:
1. Increase worker threads
2. Optimize database queries
3. Enable caching
4. Scale horizontally

### Network Bottlenecks

**Diagnosis**:
```bash
# Check bandwidth usage
iftop -i eth0

# Review connection counts
ss -tan | grep :30303 | wc -l
```

**Solutions**:
1. Increase bandwidth
2. Implement rate limiting
3. Optimize message sizes
4. Use compression

## Debug Commands

### Enable Debug Logging
```bash
# Temporary debug mode
P2P_LOGGING_LEVEL=debug p2p-node

# Or update config
sed -i 's/level = "info"/level = "debug"/' /etc/p2p-foundation/config.toml
sudo systemctl restart p2p-node
```

### Dump Internal State
```bash
# Get full node state
p2p-node debug dump-state > state.json

# Get specific subsystem
p2p-node debug dump-dht > dht.json
p2p-node debug dump-network > network.json
```

### Profile Performance
```bash
# Enable profiling
p2p-node --profile cpu --profile-output cpu.prof

# Analyze profile
go tool pprof -http=:8080 cpu.prof
```

## Recovery Procedures

### Corrupt Database
```bash
# Stop node
sudo systemctl stop p2p-node

# Backup corrupt data
mv /var/lib/p2p-foundation/db /var/lib/p2p-foundation/db.corrupt

# Restore from backup or resync
# Option 1: Restore
tar -xzf /backup/latest.tar.gz -C /var/lib/p2p-foundation/

# Option 2: Resync
sudo systemctl start p2p-node
# Node will resync from network
```

### Identity Recovery
```bash
# If identity backup exists
cp /backup/node.key /var/lib/p2p-foundation/

# Otherwise, generate new identity
p2p-node identity generate --output /var/lib/p2p-foundation/node.key

# Update three-words mapping
p2p-node identity register
```
```

## Testing Requirements
- Deploy to test environment
- Verify all procedures work
- Test rollback scenarios
- Validate monitoring setup
- Review with operations team

## Dependencies
- Previous: All implementation tasks
- Final task of sprint

## Time Estimate
- Deployment guide: 4 hours
- Configuration reference: 2 hours
- Operational runbook: 3 hours
- Troubleshooting guide: 3 hours
- Testing procedures: 2 hours
- Total: 14 hours

## Definition of Done
- [ ] All documentation complete
- [ ] Procedures tested
- [ ] Reviewed by operations
- [ ] Published to docs site
- [ ] Team trained on procedures