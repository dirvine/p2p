#!/bin/bash
# Build and package P2P binaries for distribution

set -e

echo "🔨 Building P2P Foundation Distribution Package"
echo "=============================================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Create distribution directory
DIST_DIR="p2p-distribution"
rm -rf $DIST_DIR
mkdir -p $DIST_DIR/{bin,scripts,docs}

echo "📦 Building binaries..."

# Build the CLI tool (called saorsa in p2p-cli)
echo -n "  • Building saorsa CLI..."
if cargo build --release --bin saorsa 2>/dev/null; then
    cp target/release/saorsa $DIST_DIR/bin/
    echo -e " ${GREEN}✓${NC}"
else
    echo -e " ${RED}✗${NC}"
fi

# Build the test suite
echo -n "  • Building test suite..."
if cargo build --release --bin saorsa-test-suite 2>/dev/null; then
    cp target/release/saorsa-test-suite $DIST_DIR/bin/
    echo -e " ${GREEN}✓${NC}"
else
    echo -e " ${RED}✗${NC}"
fi

# Build the chat example (use simple version for now)
echo -n "  • Building chat example..."
if [ -f "p2p-chat-simple" ]; then
    cp p2p-chat-simple $DIST_DIR/bin/p2p-chat
    echo -e " ${GREEN}✓${NC}"
elif rustc simple_p2p_chat.rs -o $DIST_DIR/bin/p2p-chat 2>/dev/null; then
    echo -e " ${GREEN}✓${NC}"
else
    echo -e " ${RED}✗${NC}"
fi

# Build Saorsa desktop app
echo -n "  • Building Saorsa desktop app..."
if cd apps/saorsa && cargo tauri build 2>/dev/null; then
    # Find the built binary (location varies by platform)
    if [ -f "src-tauri/target/release/saorsa" ]; then
        cp src-tauri/target/release/saorsa ../../$DIST_DIR/bin/saorsa-desktop
    elif [ -f "src-tauri/target/release/Saorsa" ]; then
        cp src-tauri/target/release/Saorsa ../../$DIST_DIR/bin/saorsa-desktop
    fi
    cd ../../
    echo -e " ${GREEN}✓${NC}"
else
    cd ../../
    echo -e " ${RED}✗${NC}"
fi

echo ""
echo "📝 Creating helper scripts..."

# Create start script
cat > $DIST_DIR/start-node.sh << 'EOF'
#!/bin/bash
echo "🐜 P2P Foundation Node Launcher"
echo "==============================="
echo ""
echo "Choose an option:"
echo "1) Start P2P chat"
echo "2) Run full test suite"
echo "3) Start Saorsa desktop app"
echo "4) Check network status"
echo ""
read -p "Enter choice (1-4): " choice

case $choice in
    1)
        ./bin/p2p-chat
        ;;
    2)
        ./bin/saorsa-test-suite all --local-nodes 3
        ;;
    3)
        ./bin/saorsa-desktop
        ;;
    4)
        ./bin/saorsa --status
        ;;
    *)
        echo "Invalid choice"
        ;;
esac
EOF

# Create test connectivity script
cat > $DIST_DIR/test-network.sh << 'EOF'
#!/bin/bash
echo "🔍 Testing P2P Network Connectivity"
echo "==================================="
echo ""
./bin/saorsa --status
echo ""
echo "Press any key to continue..."
read -n 1
EOF

# Create multi-node test script
cat > $DIST_DIR/run-local-network.sh << 'EOF'
#!/bin/bash
echo "🌐 Starting Local P2P Test Network"
echo "=================================="
echo ""
echo "This will start 5 local nodes for testing"
echo ""
./bin/saorsa-test-suite all --local-nodes 5 --verbose
EOF

# Make scripts executable
chmod +x $DIST_DIR/*.sh

# Copy documentation
echo "📚 Adding documentation..."
cp E2E_TEST_GUIDE.md $DIST_DIR/docs/
cat > $DIST_DIR/README.txt << 'EOF'
P2P Foundation - Distributed Network Tools
==========================================

This package contains tools for testing and using the P2P Foundation network.

QUICK START:
-----------
1. Run ./start-node.sh for an interactive menu
2. Run ./test-network.sh to test connectivity
3. Run ./run-local-network.sh to start a local test network

BINARIES:
---------
• p2p-chat         - P2P chat application with three-word address support
• saorsa           - CLI tool for network testing and connectivity
• saorsa-desktop   - Desktop application with GUI
• saorsa-test-suite - Comprehensive test suite

TESTING YOUR CONNECTION:
------------------------
./bin/saorsa --status                   # Quick status check
./bin/saorsa --test-connectivity        # Full connectivity test
./bin/saorsa --bootstrap-nodes          # List available bootstrap nodes

CHAT WITH FRIENDS:
------------------
./bin/p2p-chat                          # Start chat with auto-discovery
./bin/p2p-chat --bootstrap-words ocean.swift.mountain  # Connect to friend

RUNNING TESTS:
--------------
./bin/saorsa-test-suite --help          # See all test options
./bin/saorsa-test-suite all --local-nodes 5  # Run all tests with 5 nodes
./bin/saorsa-test-suite chat           # Run only chat tests
./bin/saorsa-test-suite stress --max-nodes 100  # Stress test

THREE-WORD ADDRESSES:
---------------------
The network uses human-friendly three-word addresses like "ocean.swift.mountain"
instead of complex technical addresses. Your address will be shown when you
start a node.

For more details, see docs/E2E_TEST_GUIDE.md

TROUBLESHOOTING:
----------------
If you have connection issues:
1. Check your firewall settings
2. Ensure ports 9000-9010 are available
3. Try using --bootstrap-words with a known address

Report issues at: https://github.com/dirvine/p2p/issues
EOF

# Create simple test binary
cat > $DIST_DIR/simple-test.c << 'EOF'
#include <stdio.h>
int main() {
    printf("🐜 P2P Foundation Network Tester\n");
    printf("================================\n\n");
    printf("To use the real tools, run:\n");
    printf("  ./bin/ant-connect --status\n");
    printf("  ./bin/saorsa\n\n");
    printf("Your three-word address would be: ocean.swift.mountain\n");
    return 0;
}
EOF

# Try to compile simple test
if command -v gcc &> /dev/null; then
    gcc $DIST_DIR/simple-test.c -o $DIST_DIR/bin/simple-test 2>/dev/null || true
fi
rm $DIST_DIR/simple-test.c

echo ""
echo "📊 Package Summary:"
echo "==================="
ls -la $DIST_DIR/bin/ 2>/dev/null | grep -v "^total" | grep -v "^d" | awk '{print "  • " $9 " (" $5 " bytes)"}'

# Create archive
echo ""
echo "📦 Creating archive..."
ARCHIVE_NAME="p2p-distribution-$(date +%Y%m%d).tar.gz"
tar -czf $ARCHIVE_NAME $DIST_DIR

echo ""
echo -e "${GREEN}✅ Success!${NC} Distribution package created:"
echo "   📁 Directory: $DIST_DIR/"
echo "   📦 Archive: $ARCHIVE_NAME"
echo ""
echo "To share with friends:"
echo "1. Send them the $ARCHIVE_NAME file"
echo "2. They extract it: tar -xzf $ARCHIVE_NAME"
echo "3. They run: cd $DIST_DIR && ./start-node.sh"
echo ""
echo "Your three-word address will be displayed when you start a node."
echo "Share it with friends so they can connect to you!"