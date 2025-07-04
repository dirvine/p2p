#!/bin/bash
# Complete build and package script for Saorsa terminal apps

echo "🚀 Saorsa Apps Build & Package Script"
echo "===================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Change to project root
cd "$(dirname "$0")/.." || exit 1
echo "Working directory: $(pwd)"
echo ""

# Step 1: Build the apps
echo "Step 1: Building Rust applications..."
echo "-------------------------------------"
if command -v cargo &> /dev/null; then
    echo "Building saorsa-terminal-chat..."
    cargo build --release -p saorsa-terminal-chat || {
        echo -e "${RED}❌ Failed to build saorsa-terminal-chat${NC}"
        echo "Try running: cargo build --release -p saorsa-terminal-chat"
        exit 1
    }
    
    echo ""
    echo "Building saorsa-network-tester..."
    cargo build --release -p saorsa-network-tester || {
        echo -e "${RED}❌ Failed to build saorsa-network-tester${NC}"
        echo "Try running: cargo build --release -p saorsa-network-tester"
        exit 1
    }
    
    echo -e "${GREEN}✅ Build complete!${NC}"
    echo ""
    
    # Show binary sizes
    echo "Binary sizes:"
    ls -lh target/release/saorsa-terminal-chat target/release/saorsa-network-tester 2>/dev/null
    echo ""
else
    echo -e "${YELLOW}⚠️  Cargo not found!${NC}"
    echo "Please install Rust from https://rustup.rs"
    echo "Then run this script again."
    exit 1
fi

# Step 2: Create macOS app bundles
echo "Step 2: Creating macOS app bundles..."
echo "------------------------------------"
cd apps || exit 1

if [ -f "create_macos_apps.sh" ]; then
    ./create_macos_apps.sh || {
        echo -e "${RED}❌ Failed to create app bundles${NC}"
        exit 1
    }
else
    echo -e "${RED}❌ create_macos_apps.sh not found${NC}"
    exit 1
fi

echo ""

# Step 3: Package for distribution
echo "Step 3: Creating distribution package..."
echo "---------------------------------------"
if [ -f "package_for_distribution.sh" ]; then
    ./package_for_distribution.sh || {
        echo -e "${RED}❌ Failed to create distribution package${NC}"
        exit 1
    }
else
    echo -e "${RED}❌ package_for_distribution.sh not found${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🎉 Build and packaging complete!${NC}"
echo ""
echo "What's been created:"
echo "1. Rust binaries in: target/release/"
echo "2. macOS app bundles in: apps/"
echo "   - Saorsa Terminal Chat.app"
echo "   - Saorsa Network Tester.app"
echo "3. Distribution package: apps/Saorsa-Apps-*.zip"
echo ""
echo "To test the apps:"
echo "1. Double-click 'Saorsa Terminal Chat.app' or 'Saorsa Network Tester.app'"
echo "2. If you see a security warning, right-click → Open"
echo ""
echo "To share with friends:"
echo "Send them the Saorsa-Apps-*.zip file!"