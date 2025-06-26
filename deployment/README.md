# P2P Foundation Bootstrap Node Deployment Guide

This guide will help you set up a production-ready bootstrap node on Digital Ocean (or any VPS) to provide reliable network entry points for the P2P Foundation network.

## 🚀 Quick Start

### Prerequisites

- Digital Ocean droplet (or any VPS) with:
  - Ubuntu 24.04 LTS (recommended)
  - At least 2GB RAM, 1 vCPU
  - IPv6 enabled (preferred)
  - SSH access as root

### One-Command Setup

```bash
# Download and run the setup script
curl -sSL https://raw.githubusercontent.com/dirvine/p2p/main/deployment/bootstrap-node-setup.sh | sudo bash
```

Or manually:

```bash
# Clone the repository
git clone https://github.com/dirvine/p2p.git
cd p2p

# Run the setup script
sudo ./deployment/bootstrap-node-setup.sh
```

## 📋 What the Setup Script Does

### 1. System Preparation
- Updates system packages
- Installs essential tools (curl, git, build-essential, etc.)
- Configures firewall (UFW) with P2P-specific rules
- Enables IPv6 if available

### 2. Rust and P2P Foundation Installation
- Installs latest stable Rust
- Creates system user `p2p` for security
- Clones and builds the P2P Foundation project
- Creates dedicated directories for logs and configuration

### 3. Network Configuration
- Detects IPv6/IPv4 capabilities
- Configures optimal listen addresses
- Sets up tunneling protocols for maximum compatibility
- Opens required firewall ports (9000/udp for QUIC, 8080/tcp for health)

### 4. Service Setup
- Creates systemd service for automatic startup
- Configures comprehensive logging with rotation
- Sets up health check endpoint
- Implements monitoring and auto-restart capabilities

### 5. Security Hardening
- Runs bootstrap node as non-root user
- Implements resource limits and security restrictions
- Configures network access controls
- Sets up audit logging

## 🔧 Configuration

### Main Configuration File
Location: `/etc/p2p-foundation/bootstrap-node.toml`

Key settings you can modify:

```toml
[node]
# Network addresses - automatically detected during setup
listen_addresses = ["/ip6/YOUR_IPV6/udp/9000/quic"]
max_connections = 1000
connection_idle_timeout = "300s"

[logging]
level = "info"  # trace, debug, info, warn, error
file = "/var/log/p2p-foundation/bootstrap-node.log"

[dht]
# DHT configuration for network routing
replication_factor = 20
alpha = 3
random_walk_interval = "300s"

[health_check]
enabled = true
bind_address = "0.0.0.0:8080"
path = "/health"
```

### Service Management

```bash
# Check status
sudo systemctl status p2p-bootstrap

# Start/stop/restart
sudo systemctl start p2p-bootstrap
sudo systemctl stop p2p-bootstrap
sudo systemctl restart p2p-bootstrap

# View logs
sudo journalctl -u p2p-bootstrap -f

# Check service health
curl http://localhost:8080/health
```

## 🌐 Network Information

After setup, your bootstrap node will be accessible via:

### IPv6 (Preferred)
```
/ip6/YOUR_IPV6_ADDRESS/udp/9000/quic
```

### IPv4 (Fallback)
```
/ip4/YOUR_IPV4_ADDRESS/udp/9000/quic
```

### Three-Word Addresses
The bootstrap discovery system includes these well-known addresses:

- `foundation.main.bootstrap` → Your primary bootstrap node
- `foundation.backup.lighthouse` → Secondary bootstrap node
- `global.fast.eagle` → IPv6 primary node
- `reliable.sturdy.anchor` → IPv4 fallback node

## 📊 Monitoring and Health Checks

### Health Check Endpoint
```bash
# Basic health check
curl http://YOUR_SERVER:8080/health

# Expected response:
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "connected_peers": 42,
  "timestamp": "2024-06-26T13:00:00Z",
  "version": "0.1.0",
  "node_type": "bootstrap"
}
```

### Metrics (Prometheus Format)
```bash
curl http://YOUR_SERVER:9090/metrics
```

### Log Monitoring
```bash
# Real-time logs
sudo tail -f /var/log/p2p-foundation/bootstrap-node.log

# Search for errors
sudo grep -i error /var/log/p2p-foundation/bootstrap-node.log

# Monitor system resources
sudo /opt/p2p-foundation/monitor.sh
```

### Automated Monitoring
The setup includes a cron job that runs every 5 minutes to:
- Check node health
- Monitor system resources
- Restart the service if unhealthy
- Log status information

## 🔐 Security Considerations

### Firewall Configuration
The script automatically configures UFW with these rules:
```bash
# Allow SSH (port 22)
# Allow P2P QUIC (port 9000/udp)
# Allow P2P TCP fallback (port 9000/tcp)
# Allow health checks (port 8080/tcp)
# Deny all other incoming traffic
```

### Security Features
- **Non-root execution**: Service runs as `p2p` user
- **Resource limits**: Memory and CPU limits prevent abuse
- **Network restrictions**: Limited network access for security
- **Audit logging**: Comprehensive logging of security events
- **Automatic updates**: System packages stay current

### Hardening Checklist
- [ ] Change default SSH port if desired
- [ ] Set up SSH key authentication (disable password auth)
- [ ] Configure fail2ban for additional protection
- [ ] Set up automated backups if storing important data
- [ ] Monitor logs regularly for suspicious activity

## 🚀 Production Deployment

### Recommended Digital Ocean Configuration

#### Droplet Specifications
- **Size**: s-2vcpu-2gb ($18/month) or larger
- **Region**: Choose based on your target user base
- **Image**: Ubuntu 24.04 LTS x64
- **Options**: 
  - ✅ IPv6 enabled
  - ✅ Monitoring
  - ✅ Private networking (if using multiple droplets)

#### Setup Steps
1. Create the droplet with IPv6 enabled
2. SSH into the server as root
3. Run the bootstrap setup script
4. Verify the service is running
5. Test connectivity from external machines
6. Update DNS records (if using custom domains)

### DNS Configuration (Optional)
If you want to use custom domains:

```bash
# A record for IPv4
bootstrap.yournetwork.org    IN A     YOUR_IPV4_ADDRESS

# AAAA record for IPv6  
bootstrap.yournetwork.org    IN AAAA  YOUR_IPV6_ADDRESS

# Create your own three-word mappings
# Update the hardcoded addresses in:
# - src/bootstrap/discovery.rs
# - apps/ant-connect/lib/main.dart
```

### Load Balancing Multiple Bootstrap Nodes
For high availability, deploy multiple bootstrap nodes:

```bash
# Primary bootstrap (US East)
foundation.main.bootstrap → bootstrap1.yournetwork.org

# Secondary bootstrap (EU)
foundation.backup.lighthouse → bootstrap2.yournetwork.org

# IPv6 specialist (Asia Pacific)
global.fast.eagle → bootstrap3.yournetwork.org
```

## 🛠️ Troubleshooting

### Common Issues

#### Bootstrap Node Won't Start
```bash
# Check logs for errors
sudo journalctl -u p2p-bootstrap -n 50

# Common issues:
# - Port already in use
# - IPv6 not available
# - Firewall blocking connections
```

#### No Peers Connecting
```bash
# Verify firewall allows connections
sudo ufw status

# Test connectivity from external machine
telnet YOUR_SERVER_IP 9000

# Check if the node is listening
sudo netstat -tulpn | grep 9000
```

#### High Memory Usage
```bash
# Check current usage
free -h

# Restart service if needed
sudo systemctl restart p2p-bootstrap

# Monitor over time
watch free -h
```

#### IPv6 Issues
```bash
# Check if IPv6 is enabled
cat /proc/sys/net/ipv6/conf/all/disable_ipv6

# Should return 0. If it returns 1:
sudo sysctl -w net.ipv6.conf.all.disable_ipv6=0
sudo sysctl -w net.ipv6.conf.default.disable_ipv6=0
```

### Performance Optimization

#### For High-Traffic Bootstrap Nodes
```bash
# Increase connection limits
echo 'net.core.somaxconn = 65535' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_max_syn_backlog = 65535' >> /etc/sysctl.conf
sudo sysctl -p

# Increase file descriptor limits
echo 'p2p soft nofile 65535' >> /etc/security/limits.conf
echo 'p2p hard nofile 65535' >> /etc/security/limits.conf
```

## 📈 Scaling and Maintenance

### Regular Maintenance Tasks

#### Weekly
- [ ] Check service status and logs
- [ ] Monitor system resources
- [ ] Review security logs
- [ ] Test health check endpoint

#### Monthly  
- [ ] Update system packages: `sudo apt update && sudo apt upgrade`
- [ ] Review and rotate logs if needed
- [ ] Check for P2P Foundation updates
- [ ] Backup configuration files

#### Quarterly
- [ ] Review security settings
- [ ] Analyze usage patterns and performance
- [ ] Plan for capacity increases if needed
- [ ] Test disaster recovery procedures

### Updating the P2P Foundation Code
```bash
# Stop the service
sudo systemctl stop p2p-bootstrap

# Update the code
cd /opt/p2p-foundation
sudo -u p2p git pull origin main
sudo -u p2p cargo build --release --bin bootstrap-node

# Restart the service
sudo systemctl start p2p-bootstrap

# Verify it's working
curl http://localhost:8080/health
```

## 🎯 Integration with Applications

### Chat Example
```bash
# Connect using three-word address
cargo run --example chat -- --bootstrap-words 'foundation.main.bootstrap'

# Connect using direct address  
cargo run --example chat -- --bootstrap '/ip4/YOUR_SERVER/udp/9000/quic'
```

### Flutter App
The Connect app includes Quick Connect buttons that automatically connect to well-known bootstrap nodes. Users can also manually enter three-word addresses.

### Custom Applications
```rust
use p2p_foundation::bootstrap::BootstrapDiscovery;

// Auto-discover bootstrap nodes
let discovery = BootstrapDiscovery::new();
let bootstraps = discovery.discover_bootstraps().await?;

// Or resolve specific three-word addresses
let addr = discovery.resolve_three_words("foundation.main.bootstrap")?;
```

## 📞 Support and Community

- **GitHub Issues**: [Report bugs and issues](https://github.com/dirvine/p2p/issues)
- **Documentation**: Check the main README and documentation
- **Health Check**: Use `/health` endpoint to verify node status

## 🏁 Conclusion

With this setup, you'll have a robust, production-ready bootstrap node that:

- ✅ Automatically handles IPv6/IPv4 connectivity
- ✅ Provides three-word address resolution
- ✅ Includes comprehensive monitoring and health checks
- ✅ Implements security best practices
- ✅ Supports automatic restart and recovery
- ✅ Integrates seamlessly with P2P Foundation applications

Your bootstrap node will help new users join the decentralized network easily using memorable three-word addresses like `foundation.main.bootstrap` instead of complex technical addresses.

**🌟 You're now contributing to the decentralized future!**