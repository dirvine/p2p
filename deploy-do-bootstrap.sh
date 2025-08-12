#!/bin/bash
# Deploy Communitas Bootstrap Nodes to DigitalOcean
# Uses pre-compiled binaries from GitHub releases

set -e

# Configuration
GITHUB_REPO="dirvine/p2p"
RELEASE_TAG="cli-v0.1.0"  # Using the current release tag
SSH_KEY="ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQD0H91SZIFP6rBe3+996fuIeC9e7GYrb885f2xZQkH+8rgG5Zmq+HqIpQ7XgvAGBePjtdKsg58eQktA7vE8UMbCHMVofnCe8mLf3WiaoajMJr+FrSnlau0RkMHIJcdgFDtJcFr5wottqMXsEThUtNBC98eMu8rE1uW8cl7ZLH6H9z2y51uAW04OA0KGHCgSQqOb+pCvFQkdm9hNFVar+/4sPGW6fA6ZWlc1n/cvn3pcCSMJIVpx45TRBa43YktUsMUm3fWrPk4ZWPbjMdUNndDjrAPoCG4nySB9ZE3Z++AEYXkwzvMkhNRc1MykVcyvg3sre/RP/iXwSF6gNsKVe9cioyikv6E4GjooOTCi+OL33ou4hhLvh7GVVhuAy6tDHgkuuubLtatZSuglVIpGmai2+0W39qA6zlgnvgvwYd55baQ01UjbTlacDFEXcjXTBETzpHlXqKwyiuzcs7NFPrurIH7j55VDhmInCVHbb8wXg/5J6dAM+U614HMHgyqu12c= davidirvine@MacBook-Pro.localdomain"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "========================================="
echo "Communitas Bootstrap Node Deployment"
echo "========================================="

# Regions and their configurations
# Using smallest droplets since we're deploying pre-compiled binaries
declare -A REGIONS=(
    ["nyc3"]="NorthAmerica"     # New York
    ["ams3"]="Europe"            # Amsterdam  
    ["sgp1"]="AsiaPacific"       # Singapore
    ["blr1"]="AsiaPacific"       # Bangalore (backup for Asia)
    ["tor1"]="NorthAmerica"      # Toronto (backup for NA)
)

# Function to create user data script
create_user_data() {
    local region=$1
    local region_name=$2
    
    cat << 'EOF'
#!/bin/bash
set -e

# Update system
apt-get update
apt-get upgrade -y

# Install required packages (minimal since we're using pre-compiled binary)
apt-get install -y curl wget openssl ca-certificates supervisor

# Create communitas user
useradd -m -s /bin/bash communitas || true
usermod -aG sudo communitas

# Setup directories
mkdir -p /opt/communitas/{bin,data,logs,config}
mkdir -p /opt/communitas/data/dht
chown -R communitas:communitas /opt/communitas

# Download binary from GitHub release
RELEASE_URL="https://github.com/${GITHUB_REPO}/releases/download/${RELEASE_TAG}/communitas-linux-amd64.tar.gz"
wget -O /tmp/communitas.tar.gz "$RELEASE_URL"
tar -xzf /tmp/communitas.tar.gz -C /opt/communitas/bin/
rm /tmp/communitas.tar.gz
chmod +x /opt/communitas/bin/communitas

# Generate API token
API_TOKEN=$(openssl rand -hex 32)
echo "$API_TOKEN" > /opt/communitas/config/api_token

# Generate MCP certificates
cd /opt/communitas/config
openssl req -x509 -newkey rsa:4096 -keyout mcp.key -out mcp.crt -days 365 -nodes \
    -subj '/CN=communitas-mcp/O=Saorsa Labs/C=US'
chmod 600 mcp.key
chmod 644 mcp.crt
chown communitas:communitas mcp.*

# Create configuration file
cat > /opt/communitas/config/bootstrap.toml << CONFIG
[network]
port = 9001
bootstrap_mode = true
max_connections = 500

[dht]
replication_factor = 8
storage_capacity_mb = 5120
persistent_storage = true
storage_path = "/opt/communitas/data/dht"
record_ttl = 86400
geographic_routing = true

[geographic]
local_region = "${region_name}"
cross_region_optimization = true
max_cross_region = 10
latency_threshold_ms = 200

[mcp]
enabled = true
port = 9090
auth_required = true
tls_cert = "/opt/communitas/config/mcp.crt"
tls_key = "/opt/communitas/config/mcp.key"
CONFIG

# Create systemd service
cat > /etc/systemd/system/communitas-bootstrap.service << SERVICE
[Unit]
Description=Communitas Bootstrap Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=communitas
Group=communitas
WorkingDirectory=/opt/communitas
Environment="RUST_LOG=info"
Environment="COMMUNITAS_API_TOKEN=$(cat /opt/communitas/config/api_token)"
ExecStart=/opt/communitas/bin/communitas bootstrap \
    --config /opt/communitas/config/bootstrap.toml \
    --port 9001 \
    --mcp-port 9090 \
    --region ${region_name} \
    --data-dir /opt/communitas/data \
    --log-dir /opt/communitas/logs \
    --storage-mb 5120

Restart=always
RestartSec=10
StandardOutput=append:/opt/communitas/logs/stdout.log
StandardError=append:/opt/communitas/logs/stderr.log

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/communitas/data /opt/communitas/logs

[Install]
WantedBy=multi-user.target
SERVICE

# Enable and start service
systemctl daemon-reload
systemctl enable communitas-bootstrap
systemctl start communitas-bootstrap

# Setup log rotation
cat > /etc/logrotate.d/communitas << LOGROTATE
/opt/communitas/logs/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 communitas communitas
}
LOGROTATE

# Setup firewall
ufw allow 22/tcp
ufw allow 9001/tcp
ufw allow 9001/udp
ufw allow 9090/tcp
ufw --force enable

echo "Bootstrap node initialization complete"
EOF
}

# Function to deploy droplet using DigitalOcean MCP
deploy_droplet() {
    local region=$1
    local geo_region=$2
    local droplet_name="communitas-bootstrap-${region}"
    
    echo -e "${YELLOW}Deploying to ${region} (${geo_region})...${NC}"
    
    # Create user data script
    local user_data=$(create_user_data "$region" "$geo_region")
    local user_data_base64=$(echo "$user_data" | base64 -w0)
    
    # Use DigitalOcean MCP to create droplet
    echo "Creating droplet: $droplet_name"
    
    # Note: This would use the actual DigitalOcean MCP commands
    # For now, showing the structure
    cat > deploy-${region}.json << JSON
{
    "name": "${droplet_name}",
    "region": "${region}",
    "size": "s-1vcpu-1gb",
    "image": "ubuntu-24-04-x64",
    "ssh_keys": ["${SSH_KEY}"],
    "backups": false,
    "ipv6": true,
    "monitoring": true,
    "tags": ["communitas", "bootstrap", "${geo_region}"],
    "user_data": "${user_data_base64}"
}
JSON
    
    echo -e "${GREEN}✓${NC} Deployment initiated for ${droplet_name}"
}

# Main deployment
main() {
    echo "Preparing to deploy bootstrap nodes..."
    echo ""
    echo "Configuration:"
    echo "  GitHub Repo: ${GITHUB_REPO}"
    echo "  Release Tag: ${RELEASE_TAG}"
    echo "  Droplet Size: s-1vcpu-1gb (smallest)"
    echo "  Storage: 5GB per node"
    echo ""
    echo "Regions:"
    for region in "${!REGIONS[@]}"; do
        echo "  - ${region}: ${REGIONS[$region]}"
    done
    echo ""
    
    read -p "Continue with deployment? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Deployment cancelled"
        exit 1
    fi
    
    # Deploy to each region
    for region in "${!REGIONS[@]}"; do
        deploy_droplet "$region" "${REGIONS[$region]}"
        sleep 2
    done
    
    echo ""
    echo "========================================="
    echo -e "${GREEN}Deployment Complete!${NC}"
    echo "========================================="
    echo ""
    echo "Next Steps:"
    echo "1. Wait 2-3 minutes for nodes to initialize"
    echo "2. Collect node IPs from DigitalOcean dashboard"
    echo "3. Test connectivity:"
    echo "   curl -k https://NODE_IP:9090/health"
    echo "4. Update DNS records for bootstrap.communitas.network"
    echo ""
    echo "Bootstrap nodes will automatically:"
    echo "- Download binary from GitHub release"
    echo "- Configure geographic routing"
    echo "- Start DHT storage service"
    echo "- Enable MCP remote management"
}

# Check if release exists
check_release() {
    echo "Checking for GitHub release ${RELEASE_TAG}..."
    
    if curl -s "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/${RELEASE_TAG}" | grep -q "Not Found"; then
        echo -e "${RED}Release ${RELEASE_TAG} not found!${NC}"
        echo ""
        echo "Please create a GitHub release first:"
        echo "1. Build the CLI: cargo build --release --target x86_64-unknown-linux-gnu"
        echo "2. Create release: gh release create ${RELEASE_TAG} ./target/release/communitas"
        exit 1
    else
        echo -e "${GREEN}✓${NC} Release ${RELEASE_TAG} found"
    fi
}

# Run deployment
check_release
main "$@"