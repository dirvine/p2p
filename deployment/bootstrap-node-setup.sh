#!/bin/bash
# P2P Foundation Bootstrap Node Setup Script for Digital Ocean
# This script sets up a persistent, production-ready bootstrap node

set -euo pipefail

# Configuration
PROJECT_NAME="p2p-foundation"
SERVICE_USER="p2p"
INSTALL_DIR="/opt/p2p-foundation"
LOG_DIR="/var/log/p2p-foundation"
CONFIG_DIR="/etc/p2p-foundation"
RUST_VERSION="stable"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   error "This script must be run as root"
fi

log "🚀 Setting up P2P Foundation Bootstrap Node on Digital Ocean"

# Update system
log "📦 Updating system packages..."
apt update
apt upgrade -y

# Install essential packages
log "🔧 Installing essential packages..."
apt install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ufw \
    htop \
    iotop \
    tcpdump \
    net-tools \
    iputils-ping \
    traceroute \
    jq \
    systemd \
    logrotate

# Install Rust
log "🦀 Installing Rust..."
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUST_VERSION
    source ~/.cargo/env
    # Make Rust available system-wide
    ln -sf ~/.cargo/bin/rustc /usr/local/bin/rustc
    ln -sf ~/.cargo/bin/cargo /usr/local/bin/cargo
    ln -sf ~/.cargo/bin/rustup /usr/local/bin/rustup
else
    log "Rust already installed"
fi

# Create system user for the service
log "👤 Creating service user..."
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd --system --home-dir "$INSTALL_DIR" --shell /bin/bash "$SERVICE_USER"
else
    log "User $SERVICE_USER already exists"
fi

# Create directories
log "📁 Creating directories..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$INSTALL_DIR/src"

# Set permissions
chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$LOG_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$CONFIG_DIR"

# Clone the P2P Foundation repository
log "📥 Cloning P2P Foundation repository..."
cd "$INSTALL_DIR"
if [ ! -d ".git" ]; then
    sudo -u "$SERVICE_USER" git clone https://github.com/dirvine/p2p.git .
else
    log "Repository already exists, pulling latest changes..."
    sudo -u "$SERVICE_USER" git pull origin main
fi

# Build the project
log "🔨 Building P2P Foundation..."
sudo -u "$SERVICE_USER" bash -c "source ~/.cargo/env && cargo build --release --bin bootstrap-node"

# Configure IPv6
log "🌐 Configuring IPv6..."
# Enable IPv6 if disabled
sysctl -w net.ipv6.conf.all.disable_ipv6=0
sysctl -w net.ipv6.conf.default.disable_ipv6=0
echo "net.ipv6.conf.all.disable_ipv6=0" >> /etc/sysctl.conf
echo "net.ipv6.conf.default.disable_ipv6=0" >> /etc/sysctl.conf

# Get IPv6 address
IPV6_ADDR=$(ip -6 addr show scope global | grep -oP '(?<=inet6\s)[0-9a-f:]+' | head -1)
IPV4_ADDR=$(curl -4 -s ifconfig.me || curl -4 -s ipv4.icanhazip.com)

if [ -z "$IPV6_ADDR" ]; then
    warn "No global IPv6 address found. The node will use IPv4 with tunneling."
    LISTEN_ADDR="/ip4/$IPV4_ADDR/udp/9000/quic"
else
    log "Found IPv6 address: $IPV6_ADDR"
    LISTEN_ADDR="/ip6/$IPV6_ADDR/udp/9000/quic"
fi

log "Bootstrap node will listen on: $LISTEN_ADDR"

# Configure firewall
log "🔥 Configuring firewall..."
ufw --force reset
ufw default deny incoming
ufw default allow outgoing
ufw allow ssh
ufw allow 9000/udp comment "P2P Foundation QUIC"
ufw allow 9000/tcp comment "P2P Foundation TCP fallback"
ufw allow 8080/tcp comment "P2P Foundation HTTP health check"
ufw --force enable

# Create bootstrap node configuration
log "⚙️ Creating bootstrap node configuration..."
cat > "$CONFIG_DIR/bootstrap-node.toml" << EOF
# P2P Foundation Bootstrap Node Configuration

[node]
# Node identity and networking
listen_addresses = ["$LISTEN_ADDR"]
announce_addresses = ["$LISTEN_ADDR"]

# Bootstrap mode - this node helps others discover the network
bootstrap_mode = true
enable_mdns = true
enable_kad_bootstrap = true

# Resource limits
max_connections = 1000
connection_idle_timeout = "300s"
max_concurrent_streams = 100

[logging]
level = "info"
file = "$LOG_DIR/bootstrap-node.log"
max_size = "100MB"
max_files = 5

[metrics]
enabled = true
bind_address = "127.0.0.1:9090"

[health_check]
enabled = true
bind_address = "0.0.0.0:8080"
path = "/health"

[security]
# Enhanced security for public bootstrap node
enable_noise = true
enable_yamux = true
connection_timeout = "30s"
handshake_timeout = "10s"

[dht]
# Kademlia DHT configuration
replication_factor = 20
alpha = 3
beta = 3
random_walk_interval = "300s"
bootstrap_interval = "60s"

[tunneling]
# Enable all tunneling protocols for maximum compatibility
enable_6to4 = true
enable_teredo = true
enable_6in4 = true
enable_dslite = true
enable_isatap = true
enable_map_e = true
enable_map_t = true
auto_detect = true
EOF

chown "$SERVICE_USER:$SERVICE_USER" "$CONFIG_DIR/bootstrap-node.toml"

# Create the bootstrap node systemd service
log "🔧 Creating systemd service..."
cat > /etc/systemd/system/p2p-bootstrap.service << EOF
[Unit]
Description=P2P Foundation Bootstrap Node
Documentation=https://github.com/dirvine/p2p
After=network-online.target
Wants=network-online.target

[Service]
Type=exec
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$INSTALL_DIR
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
ExecStart=$INSTALL_DIR/target/release/bootstrap-node --config $CONFIG_DIR/bootstrap-node.toml
ExecReload=/bin/kill -USR1 \$MAINPID
Restart=always
RestartSec=10
StartLimitInterval=60
StartLimitBurst=3

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$LOG_DIR $CONFIG_DIR

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Network
IPAddressDeny=any
IPAddressAllow=localhost
IPAddressAllow=10.0.0.0/8
IPAddressAllow=172.16.0.0/12
IPAddressAllow=192.168.0.0/16
IPAddressAllow=::/0

[Install]
WantedBy=multi-user.target
EOF

# Create health check script
log "🏥 Creating health check script..."
cat > "$INSTALL_DIR/health-check.sh" << 'EOF'
#!/bin/bash
# P2P Bootstrap Node Health Check

set -euo pipefail

HEALTH_URL="http://localhost:8080/health"
MAX_RETRIES=3
RETRY_DELAY=5

check_health() {
    local attempt=1
    while [ $attempt -le $MAX_RETRIES ]; do
        if curl -f -s "$HEALTH_URL" > /dev/null 2>&1; then
            echo "✅ Bootstrap node is healthy"
            return 0
        fi
        
        echo "❌ Health check failed (attempt $attempt/$MAX_RETRIES)"
        if [ $attempt -lt $MAX_RETRIES ]; then
            sleep $RETRY_DELAY
        fi
        ((attempt++))
    done
    
    echo "🚨 Bootstrap node is unhealthy after $MAX_RETRIES attempts"
    return 1
}

# Check if process is running
if ! pgrep -f "bootstrap-node" > /dev/null; then
    echo "🚨 Bootstrap node process not found"
    exit 1
fi

# Check health endpoint
check_health

# Check log for recent errors
if tail -100 /var/log/p2p-foundation/bootstrap-node.log | grep -i "error\|panic\|fatal" > /dev/null; then
    echo "⚠️  Recent errors found in logs"
    exit 1
fi

echo "✅ All health checks passed"
EOF

chmod +x "$INSTALL_DIR/health-check.sh"
chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/health-check.sh"

# Set up log rotation
log "📝 Setting up log rotation..."
cat > /etc/logrotate.d/p2p-foundation << EOF
$LOG_DIR/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 $SERVICE_USER $SERVICE_USER
    postrotate
        systemctl reload p2p-bootstrap.service || true
    endscript
}
EOF

# Create monitoring script
log "📊 Creating monitoring script..."
cat > "$INSTALL_DIR/monitor.sh" << 'EOF'
#!/bin/bash
# P2P Bootstrap Node Monitoring

set -euo pipefail

LOG_FILE="/var/log/p2p-foundation/monitor.log"
HEALTH_CHECK="/opt/p2p-foundation/health-check.sh"

log_message() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Check if health check script exists and is executable
if [ ! -x "$HEALTH_CHECK" ]; then
    log_message "ERROR: Health check script not found or not executable"
    exit 1
fi

# Run health check
if ! "$HEALTH_CHECK"; then
    log_message "ALERT: Bootstrap node health check failed, attempting restart"
    systemctl restart p2p-bootstrap.service
    sleep 30
    
    # Check again after restart
    if ! "$HEALTH_CHECK"; then
        log_message "CRITICAL: Bootstrap node still unhealthy after restart"
        exit 1
    else
        log_message "INFO: Bootstrap node recovered after restart"
    fi
else
    log_message "INFO: Bootstrap node health check passed"
fi

# Log system resources
CPU_USAGE=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1)
MEM_USAGE=$(free | grep Mem | awk '{printf("%.1f", $3/$2 * 100.0)}')
DISK_USAGE=$(df -h / | awk 'NR==2{print $5}' | cut -d'%' -f1)

log_message "INFO: System resources - CPU: ${CPU_USAGE}%, Memory: ${MEM_USAGE}%, Disk: ${DISK_USAGE}%"

# Log connection count
CONN_COUNT=$(ss -tun | wc -l)
log_message "INFO: Active connections: $CONN_COUNT"
EOF

chmod +x "$INSTALL_DIR/monitor.sh"
chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/monitor.sh"

# Set up cron job for monitoring
log "⏰ Setting up monitoring cron job..."
cat > /etc/cron.d/p2p-foundation-monitor << EOF
# P2P Foundation Bootstrap Node Monitoring
# Check every 5 minutes
*/5 * * * * $SERVICE_USER $INSTALL_DIR/monitor.sh > /dev/null 2>&1
EOF

# Create startup information script
log "📋 Creating startup information script..."
cat > "$INSTALL_DIR/bootstrap-info.sh" << EOF
#!/bin/bash
# P2P Foundation Bootstrap Node Information

set -euo pipefail

echo "🌐 P2P Foundation Bootstrap Node Information"
echo "=========================================="
echo
echo "📍 Network Addresses:"
echo "   IPv4: $IPV4_ADDR"
if [ -n "$IPV6_ADDR" ]; then
    echo "   IPv6: $IPV6_ADDR"
fi
echo "   Listen: $LISTEN_ADDR"
echo
echo "🔤 Three-Word Addresses:"
# We'll generate these after the service starts
echo "   Primary: (will be generated after first startup)"
echo "   Backup:  (will be generated after first startup)"
echo
echo "🔗 Connection Examples:"
echo "   CLI: cargo run --example chat -- --bootstrap '$LISTEN_ADDR'"
echo "   Three-word: cargo run --example chat -- --bootstrap-words 'bootstrap.node.address'"
echo
echo "🏥 Health Check:"
echo "   URL: http://localhost:8080/health"
echo "   Script: $INSTALL_DIR/health-check.sh"
echo
echo "📊 Monitoring:"
echo "   Logs: tail -f $LOG_DIR/bootstrap-node.log"
echo "   Monitor: $INSTALL_DIR/monitor.sh"
echo
echo "🔧 Service Management:"
echo "   Status: systemctl status p2p-bootstrap"
echo "   Start:  systemctl start p2p-bootstrap"
echo "   Stop:   systemctl stop p2p-bootstrap"
echo "   Logs:   journalctl -u p2p-bootstrap -f"
echo
EOF

chmod +x "$INSTALL_DIR/bootstrap-info.sh"
chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/bootstrap-info.sh"

# Enable and start the service
log "🚀 Enabling and starting the bootstrap service..."
systemctl daemon-reload
systemctl enable p2p-bootstrap.service

# Create the bootstrap node binary if it doesn't exist
if [ ! -f "$INSTALL_DIR/target/release/bootstrap-node" ]; then
    log "🔨 Building bootstrap node binary..."
    cd "$INSTALL_DIR"
    sudo -u "$SERVICE_USER" bash -c "source ~/.cargo/env && cargo build --release --bin bootstrap-node"
fi

# Start the service
systemctl start p2p-bootstrap.service

# Wait a moment for startup
sleep 5

# Check service status
if systemctl is-active --quiet p2p-bootstrap.service; then
    log "✅ Bootstrap service started successfully!"
else
    error "❌ Bootstrap service failed to start. Check logs: journalctl -u p2p-bootstrap -n 50"
fi

# Display final information
log "🎉 P2P Foundation Bootstrap Node setup complete!"
echo
echo "=========================================="
"$INSTALL_DIR/bootstrap-info.sh"
echo "=========================================="
echo
echo "📝 Next steps:"
echo "   1. Wait 2-3 minutes for the node to fully initialize"
echo "   2. Check the health endpoint: curl http://localhost:8080/health"
echo "   3. View logs: journalctl -u p2p-bootstrap -f"
echo "   4. Test connectivity from another machine"
echo
echo "🌟 Your bootstrap node is now helping build the decentralized future!"
EOF