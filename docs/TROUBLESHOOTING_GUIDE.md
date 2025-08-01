# P2P Foundation Troubleshooting Guide

## Overview

This guide helps diagnose and resolve common issues with P2P Foundation nodes. It covers symptoms, causes, and solutions for various problems you might encounter.

## Table of Contents

1. [Quick Diagnostics](#quick-diagnostics)
2. [Common Issues](#common-issues)
3. [Network Problems](#network-problems)
4. [Storage Issues](#storage-issues)
5. [Performance Problems](#performance-problems)
6. [Security Issues](#security-issues)
7. [Advanced Debugging](#advanced-debugging)
8. [Recovery Procedures](#recovery-procedures)
9. [Error Reference](#error-reference)
10. [Getting Help](#getting-help)

## Quick Diagnostics

### Health Check Script

```bash
#!/bin/bash
# p2p-health-check.sh

echo "P2P Node Health Check"
echo "===================="

# Check if node is running
if systemctl is-active --quiet p2p-node; then
    echo "✓ Node service is running"
else
    echo "✗ Node service is not running"
    exit 1
fi

# Check API responsiveness
if curl -s http://localhost:8001/health > /dev/null; then
    echo "✓ API is responsive"
else
    echo "✗ API is not responding"
fi

# Check peer connections
PEERS=$(curl -s http://localhost:8001/status | jq '.connected_peers')
if [ "$PEERS" -gt 0 ]; then
    echo "✓ Connected to $PEERS peers"
else
    echo "✗ No peer connections"
fi

# Check storage
STORAGE=$(df -h /var/lib/p2p-foundation | tail -1 | awk '{print $5}' | sed 's/%//')
if [ "$STORAGE" -lt 90 ]; then
    echo "✓ Storage usage: $STORAGE%"
else
    echo "✗ Storage critical: $STORAGE%"
fi

# Check memory
MEM=$(free | grep Mem | awk '{print int($3/$2 * 100)}')
if [ "$MEM" -lt 90 ]; then
    echo "✓ Memory usage: $MEM%"
else
    echo "✗ Memory critical: $MEM%"
fi
```

### Quick Status Commands

```bash
# Service status
systemctl status p2p-node

# Recent logs
journalctl -u p2p-node -n 100 --no-pager

# Network status
p2p-node status

# Peer list
p2p-node peers list

# Storage stats
p2p-node storage stats
```

## Common Issues

### Node Won't Start

**Symptoms:**
- Service fails to start
- Exit code 1 or 255
- No log output

**Possible Causes:**

1. **Missing Identity File**
   ```bash
   # Check if identity exists
   ls -la /var/lib/p2p-foundation/identity.json
   
   # Generate new identity if missing
   p2p-node identity generate --output /var/lib/p2p-foundation/identity.json
   ```

2. **Port Already in Use**
   ```bash
   # Check what's using the port
   sudo lsof -i :8000
   
   # Change port in config or kill conflicting process
   ```

3. **Permission Issues**
   ```bash
   # Fix ownership
   sudo chown -R p2p-node:p2p-node /var/lib/p2p-foundation
   
   # Fix permissions
   sudo chmod 750 /var/lib/p2p-foundation
   sudo chmod 640 /var/lib/p2p-foundation/identity.json
   ```

4. **Corrupted Configuration**
   ```bash
   # Validate config
   p2p-node config validate --file /etc/p2p-foundation/config.toml
   
   # Use default config
   p2p-node config generate > /etc/p2p-foundation/config.toml
   ```

### Node Keeps Crashing

**Symptoms:**
- Frequent restarts
- "Signal: 11 (SIGSEGV)" in logs
- OOM killer messages

**Solutions:**

1. **Memory Issues**
   ```bash
   # Check memory limits
   systemctl show p2p-node | grep MemoryLimit
   
   # Increase memory limit
   sudo systemctl edit p2p-node
   # Add: MemoryLimit=8G
   
   # Reduce cache size in config
   cache_size_mb = 512
   ```

2. **Database Corruption**
   ```bash
   # Stop node
   sudo systemctl stop p2p-node
   
   # Run integrity check
   p2p-node db check --data-dir /var/lib/p2p-foundation
   
   # Repair if needed
   p2p-node db repair --data-dir /var/lib/p2p-foundation
   ```

3. **Stack Overflow**
   ```bash
   # Increase stack size
   ulimit -s 16384
   
   # Or in systemd service
   LimitSTACK=16M
   ```

## Network Problems

### No Peer Connections

**Diagnostics:**
```bash
# Test network connectivity
p2p-node network test

# Check firewall
sudo iptables -L -n | grep 8000

# Test bootstrap nodes
for node in seed1.network.com seed2.network.com; do
    nc -zv $node 8000
done
```

**Solutions:**

1. **Firewall Blocking**
   ```bash
   # Open required ports
   sudo ufw allow 8000/tcp
   sudo ufw allow 8000/udp
   
   # For iptables
   sudo iptables -A INPUT -p tcp --dport 8000 -j ACCEPT
   sudo iptables -A INPUT -p udp --dport 8000 -j ACCEPT
   ```

2. **NAT Issues**
   ```bash
   # Enable UPnP
   p2p-node network enable-upnp
   
   # Or configure port forwarding manually
   # Router: Forward external:8000 -> internal:8000
   
   # Set external IP in config
   external_ip = "YOUR_PUBLIC_IP"
   ```

3. **DNS Resolution**
   ```bash
   # Test DNS
   nslookup seed1.network.com
   
   # Use IP addresses instead
   bootstrap_nodes = ["192.168.1.100:8000", "10.0.0.5:8000"]
   ```

### High Latency

**Diagnostics:**
```bash
# Ping test to peers
p2p-node network ping --all

# Check routing metrics
p2p-node debug routing-metrics

# Network interface stats
ip -s link show
```

**Solutions:**

1. **Optimize Routing**
   ```toml
   [routing]
   prefer_low_latency = true
   max_hops = 5
   parallel_queries = 3
   ```

2. **Reduce Connection Count**
   ```toml
   [network]
   max_connections = 50  # Reduce from default
   connection_quality_threshold = 100  # ms
   ```

3. **Enable Fast Protocols**
   ```toml
   [transport]
   prefer_quic = true
   enable_compression = false  # Reduce CPU overhead
   ```

### Connection Drops

**Common Causes:**
- Aggressive firewalls
- ISP throttling
- Resource exhaustion

**Solutions:**

1. **Increase Timeouts**
   ```toml
   [network]
   connection_timeout_secs = 60
   idle_timeout_secs = 300
   keepalive_interval_secs = 30
   ```

2. **Connection Pooling**
   ```toml
   [performance]
   connection_pool_size = 100
   connection_reuse = true
   ```

## Storage Issues

### Disk Full

**Immediate Actions:**
```bash
# Check disk usage
df -h /var/lib/p2p-foundation

# Find large files
du -h /var/lib/p2p-foundation | sort -rh | head -20

# Clean old data
p2p-node storage cleanup --older-than 30d
```

**Long-term Solutions:**

1. **Adjust Storage Limits**
   ```toml
   [storage]
   capacity_gb = 100  # Reduce capacity
   reserved_space_gb = 10  # Keep free space
   auto_cleanup = true
   cleanup_threshold = 0.9  # Start cleanup at 90%
   ```

2. **Move to Larger Disk**
   ```bash
   # Stop node
   sudo systemctl stop p2p-node
   
   # Copy data
   sudo rsync -av /var/lib/p2p-foundation/ /new/disk/p2p-foundation/
   
   # Update config
   data_dir = "/new/disk/p2p-foundation"
   
   # Start node
   sudo systemctl start p2p-node
   ```

### Slow Storage Operations

**Diagnostics:**
```bash
# I/O statistics
iostat -x 1

# Check for I/O errors
dmesg | grep -i "i/o error"

# Storage benchmark
p2p-node benchmark storage
```

**Solutions:**

1. **Optimize Database**
   ```bash
   # Defragment database
   p2p-node db optimize
   
   # Rebuild indices
   p2p-node db reindex
   ```

2. **Tune Filesystem**
   ```bash
   # For ext4
   tune2fs -o journal_data_writeback /dev/sda1
   
   # Mount options
   /dev/sda1 /var/lib/p2p-foundation ext4 noatime,nodiratime 0 2
   ```

## Performance Problems

### High CPU Usage

**Diagnostics:**
```bash
# Top processes
top -p $(pgrep p2p-node)

# CPU profiling
p2p-node debug profile --duration 60s --output cpu.prof

# Thread dump
p2p-node debug threads
```

**Solutions:**

1. **Reduce Workload**
   ```toml
   [performance]
   worker_threads = 4  # Limit threads
   max_concurrent_operations = 100
   rate_limit_operations_per_sec = 1000
   ```

2. **Disable Features**
   ```toml
   [features]
   enable_compression = false
   enable_encryption = false  # Only for testing!
   enable_ml_optimization = false
   ```

### Memory Leaks

**Detection:**
```bash
# Monitor memory growth
while true; do
    ps aux | grep p2p-node | grep -v grep
    sleep 60
done

# Memory profiling
p2p-node debug memory --output mem.prof
```

**Solutions:**

1. **Limit Caches**
   ```toml
   [caching]
   routing_cache_size = 10000
   content_cache_size_mb = 512
   peer_cache_size = 1000
   cache_ttl_secs = 3600
   ```

2. **Restart Periodically**
   ```bash
   # Add to crontab
   0 3 * * * systemctl restart p2p-node
   ```

## Security Issues

### Suspected Attack

**Symptoms:**
- Unusual traffic patterns
- Many failed authentications
- Resource exhaustion

**Response:**

1. **Enable Emergency Mode**
   ```bash
   # Strict security mode
   p2p-node security emergency-mode --enable
   
   # This enables:
   # - Aggressive rate limiting
   # - Connection whitelisting
   # - Enhanced logging
   ```

2. **Block Attackers**
   ```bash
   # View suspicious peers
   p2p-node security suspicious-peers
   
   # Block specific peer
   p2p-node security block-peer PEER_ID
   
   # Block IP range
   iptables -A INPUT -s 192.168.1.0/24 -j DROP
   ```

3. **Collect Evidence**
   ```bash
   # Enable debug logging
   p2p-node config set log_level=debug
   
   # Capture packets
   tcpdump -i any -w attack.pcap port 8000
   
   # Export security logs
   p2p-node security export-logs --output security-incident.log
   ```

### Identity Compromise

**If private key is compromised:**

1. **Immediate Actions**
   ```bash
   # Stop node
   sudo systemctl stop p2p-node
   
   # Generate new identity
   p2p-node identity generate --output new-identity.json
   
   # Revoke old identity (if supported by network)
   p2p-node identity revoke --identity old-identity.json
   ```

2. **Notify Network**
   ```bash
   # Broadcast revocation
   p2p-node security broadcast-revocation
   ```

## Advanced Debugging

### Enable Debug Mode

```toml
[debug]
enabled = true
verbose_logging = true
trace_messages = true
dump_packets = false
profile_operations = true
```

### Debug Commands

```bash
# Dump internal state
p2p-node debug state --output state.json

# Trace message flow
p2p-node debug trace MESSAGE_ID

# Inspect routing table
p2p-node debug routing-table --format json

# Check trust scores
p2p-node debug trust-matrix

# Memory analysis
p2p-node debug heap-dump

# Goroutine dump (for deadlocks)
p2p-node debug goroutines
```

### Performance Analysis

```bash
# CPU profiling
p2p-node debug profile cpu --duration 5m --output cpu.prof
go tool pprof -http=:8080 cpu.prof

# Memory profiling
p2p-node debug profile heap --output mem.prof
go tool pprof -http=:8080 mem.prof

# Trace execution
p2p-node debug trace --duration 10s --output trace.out
go tool trace trace.out
```

## Recovery Procedures

### Complete Node Recovery

```bash
#!/bin/bash
# recover-node.sh

# 1. Backup current state
tar -czf backup-$(date +%Y%m%d).tar.gz /var/lib/p2p-foundation

# 2. Stop node
systemctl stop p2p-node

# 3. Clean state
rm -rf /var/lib/p2p-foundation/data/*

# 4. Restore identity
cp /secure/backup/identity.json /var/lib/p2p-foundation/

# 5. Fix permissions
chown -R p2p-node:p2p-node /var/lib/p2p-foundation

# 6. Start fresh
systemctl start p2p-node

# 7. Monitor startup
journalctl -u p2p-node -f
```

### Network Partition Recovery

```bash
# Detect partition
p2p-node network analyze-partition

# Force reconnect to main network
p2p-node network rejoin --force --bootstrap seed1.network.com:8000

# Resync data
p2p-node sync --full
```

## Error Reference

### Common Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| `E001` | Identity not found | Generate or restore identity |
| `E002` | Port already in use | Change port or stop conflicting service |
| `E003` | No bootstrap nodes | Configure bootstrap nodes |
| `E004` | Storage full | Clean up or increase capacity |
| `E005` | Network unreachable | Check firewall and connectivity |
| `E006` | Invalid configuration | Fix config syntax |
| `E007` | Database corrupted | Run repair tool |
| `E008` | Out of memory | Increase memory limit |
| `E009` | Too many connections | Increase connection limit |
| `E010` | Authentication failed | Check identity and permissions |

### Log Patterns

```bash
# Find errors in logs
journalctl -u p2p-node | grep -E "ERROR|WARN|FATAL"

# Common patterns
"connection refused" - Peer is down or firewall blocking
"i/o timeout" - Network issues or peer overloaded
"no route to host" - Network configuration problem
"too many open files" - Increase ulimit
"out of memory" - Memory limit reached
"permission denied" - File permission issues
```

## Getting Help

### Collect Diagnostic Information

```bash
#!/bin/bash
# collect-diagnostics.sh

DIAG_DIR="diagnostics-$(date +%Y%m%d-%H%M%S)"
mkdir -p $DIAG_DIR

# System info
uname -a > $DIAG_DIR/system.txt
free -m >> $DIAG_DIR/system.txt
df -h >> $DIAG_DIR/system.txt

# Service status
systemctl status p2p-node > $DIAG_DIR/service-status.txt

# Recent logs
journalctl -u p2p-node -n 1000 > $DIAG_DIR/logs.txt

# Configuration (remove sensitive data)
grep -v "key\|secret\|password" /etc/p2p-foundation/config.toml > $DIAG_DIR/config.toml

# Network state
p2p-node status > $DIAG_DIR/node-status.txt
p2p-node peers list > $DIAG_DIR/peers.txt

# Create archive
tar -czf $DIAG_DIR.tar.gz $DIAG_DIR/
echo "Diagnostics collected in $DIAG_DIR.tar.gz"
```

### Support Channels

1. **Community Forum**
   - https://forum.p2p-foundation.org
   - Search existing issues first
   - Include diagnostic information

2. **GitHub Issues**
   - https://github.com/yourusername/p2p-foundation/issues
   - Use issue templates
   - Include version information

3. **Real-time Chat**
   - Discord: https://discord.gg/p2pfoundation
   - Matrix: #p2p-foundation:matrix.org

4. **Commercial Support**
   - Email: support@p2p-foundation.org
   - Include diagnostic archive
   - Provide node ID and issue timeline

### Information to Include

When reporting issues, always include:

1. Node version: `p2p-node --version`
2. Operating system and version
3. Configuration (sanitized)
4. Recent logs
5. Steps to reproduce
6. Expected vs actual behavior
7. Diagnostic archive

## Prevention

### Best Practices

1. **Regular Maintenance**
   ```bash
   # Weekly
   p2p-node maintenance check
   
   # Monthly
   p2p-node db optimize
   
   # Quarterly
   p2p-node security audit
   ```

2. **Monitoring**
   - Set up alerts for critical metrics
   - Monitor disk space trends
   - Track peer connection stability

3. **Backups**
   - Daily identity backup
   - Weekly configuration backup
   - Monthly full state backup

4. **Updates**
   - Subscribe to security announcements
   - Test updates in staging first
   - Keep dependencies updated