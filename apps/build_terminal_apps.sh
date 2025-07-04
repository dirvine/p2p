#!/bin/bash
# Build script for Saorsa terminal applications

echo "🔨 Building Saorsa Terminal Applications"
echo "========================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠️  Cargo not found in PATH${NC}"
    echo "Please ensure Rust is installed and cargo is in your PATH"
    echo "Visit: https://rustup.rs for installation instructions"
    exit 1
fi

# Move to project root
cd "$(dirname "$0")/.." || exit 1

echo "Building in release mode with optimizations..."
echo ""

# Build saorsa-terminal-chat
echo -n "Building saorsa-terminal-chat..."
if cargo build --release -p saorsa-terminal-chat; then
    echo -e " ${GREEN}✓${NC}"
    CHAT_SIZE=$(ls -lh target/release/saorsa-terminal-chat 2>/dev/null | awk '{print $5}')
    echo "  Size: ${CHAT_SIZE}"
else
    echo -e " ${RED}✗${NC}"
    echo "Error building saorsa-terminal-chat"
    echo "Run with: cargo build --release -p saorsa-terminal-chat"
    exit 1
fi

# Build saorsa-network-tester
echo -n "Building saorsa-network-tester..."
if cargo build --release -p saorsa-network-tester; then
    echo -e " ${GREEN}✓${NC}"
    TESTER_SIZE=$(ls -lh target/release/saorsa-network-tester 2>/dev/null | awk '{print $5}')
    echo "  Size: ${TESTER_SIZE}"
else
    echo -e " ${RED}✗${NC}"
    echo "Error building saorsa-network-tester"
    echo "Run with: cargo build --release -p saorsa-network-tester"
    exit 1
fi

echo ""
echo -e "${GREEN}✅ Build successful!${NC}"
echo ""
echo "Binaries location:"
echo "  ./target/release/saorsa-terminal-chat"
echo "  ./target/release/saorsa-network-tester"
echo ""
echo "To run directly:"
echo "  ./target/release/saorsa-terminal-chat"
echo "  ./target/release/saorsa-network-tester"
echo ""
echo "To create macOS app bundles:"
echo "  cd apps && ./create_macos_apps.sh"