#\!/bin/bash
set -euo pipefail

# DigitalOcean Bootstrap Node Deployment Script for Communitas
DROPLET_NAME="communitas-bootstrap"
DROPLET_SIZE="s-2vcpu-4gb"
DROPLET_REGION="fra1" 
DROPLET_IMAGE="ubuntu-22-04-x64"

echo "Starting Communitas Bootstrap Node deployment..."

# Create droplet
echo "Creating DigitalOcean droplet..."
doctl compute droplet create "${DROPLET_NAME}" \
    --size "${DROPLET_SIZE}" \
    --image "${DROPLET_IMAGE}" \
    --region "${DROPLET_REGION}" \
    --enable-monitoring \
    --enable-ipv6 \
    --tag-names "communitas,bootstrap,production" \
    --wait

# Get droplet IP
DROPLET_IP=$(doctl compute droplet list --format Name,PublicIPv4 | grep "${DROPLET_NAME}" | awk '{print $2}')
echo "Droplet created with IP: ${DROPLET_IP}"

# Wait for SSH to be available
echo "Waiting for SSH to be available..."
while \! ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no root@"${DROPLET_IP}" exit 2>/dev/null; do
    sleep 5
done

echo "Deployment completed successfully\!"
echo "Bootstrap node will be available at: http://${DROPLET_IP}:8888"
echo "Server IP: ${DROPLET_IP}"
echo "SSH access: ssh root@${DROPLET_IP}"
EOF < /dev/null