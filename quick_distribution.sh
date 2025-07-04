#!/bin/bash
# Quick distribution package builder

set -e

echo "🔨 Building P2P Foundation Quick Distribution"
echo "============================================"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Create distribution directory
DIST_DIR="p2p-quick-dist"
rm -rf $DIST_DIR
mkdir -p $DIST_DIR/{bin,scripts,docs}

echo "📦 Collecting binaries..."

# Copy existing binaries
if [ -f "target/release/saorsa-test-suite" ]; then
    cp target/release/saorsa-test-suite $DIST_DIR/bin/
    echo -e "  • Test suite ${GREEN}✓${NC}"
else
    echo -e "  • Test suite ${RED}✗${NC} (not found)"
fi

if [ -f "p2p-chat-simple" ]; then
    cp p2p-chat-simple $DIST_DIR/bin/p2p-chat
    echo -e "  • Chat binary ${GREEN}✓${NC}"
else
    echo -e "  • Chat binary ${RED}✗${NC} (not found)"
fi

echo ""
echo "📝 Creating helper scripts..."

# Create start script
cat > $DIST_DIR/start.sh << 'EOF'
#!/bin/bash
echo "🐜 P2P Foundation - Quick Start"
echo "==============================="
echo ""
echo "1) Start P2P chat"
echo "2) Run connectivity test"
echo "3) Run local test network (5 nodes)"
echo ""
read -p "Enter choice (1-3): " choice

case $choice in
    1)
        ./bin/p2p-chat
        ;;
    2)
        ./bin/saorsa-test-suite connectivity --verbose
        ;;
    3)
        ./bin/saorsa-test-suite all --local-nodes 5
        ;;
    *)
        echo "Invalid choice"
        ;;
esac
EOF

# Create connect script
cat > $DIST_DIR/connect-to-friend.sh << 'EOF'
#!/bin/bash
echo "🔗 Connect to Friend's P2P Node"
echo "==============================="
echo ""
echo "Enter your friend's three-word address"
echo "(Example: ocean.swift.mountain)"
echo ""
read -p "Three-word address: " THREE_WORDS

if [ -z "$THREE_WORDS" ]; then
    echo "No address entered, exiting..."
    exit 1
fi

echo ""
echo "🚀 Connecting to $THREE_WORDS..."
./bin/p2p-chat --bootstrap-words "$THREE_WORDS"
EOF

# Make scripts executable
chmod +x $DIST_DIR/*.sh

# Create README
cat > $DIST_DIR/README.txt << 'EOF'
P2P Foundation - Quick Distribution
===================================

This package contains essential P2P Foundation tools for testing.

QUICK START:
-----------
1. Run ./start.sh for interactive menu
2. Run ./connect-to-friend.sh to connect to a friend

MANUAL USAGE:
-------------
Chat:
  ./bin/p2p-chat                                    # Start with auto-discovery
  ./bin/p2p-chat --bootstrap-words friend.address   # Connect to specific node

Testing:
  ./bin/saorsa-test-suite --help                    # See all test options
  ./bin/saorsa-test-suite connectivity --verbose    # Test connectivity
  ./bin/saorsa-test-suite all --local-nodes 5       # Run full test suite

THREE-WORD ADDRESSES:
---------------------
When you start a chat node, you'll see your three-word address like:
"ocean.swift.mountain"

Share this address with friends so they can connect to you!

TROUBLESHOOTING:
----------------
- Ensure ports 9000-9010 are available
- Check firewall settings if connection fails
- Try different bootstrap nodes if auto-discovery fails

For more info: https://github.com/dirvine/p2p
EOF

# Copy documentation
if [ -f "E2E_TEST_GUIDE.md" ]; then
    cp E2E_TEST_GUIDE.md $DIST_DIR/docs/
fi

echo ""
echo "📊 Package contents:"
echo "==================="
ls -la $DIST_DIR/bin/ 2>/dev/null | grep -v "^total" | grep -v "^d" | awk '{print "  • " $9 " (" $5 " bytes)"}'

# Create archive
echo ""
echo "📦 Creating archive..."
ARCHIVE_NAME="p2p-quick-dist-$(date +%Y%m%d-%H%M%S).tar.gz"
tar -czf $ARCHIVE_NAME $DIST_DIR

echo ""
echo -e "${GREEN}✅ Success!${NC} Quick distribution created:"
echo "   📁 Directory: $DIST_DIR/"
echo "   📦 Archive: $ARCHIVE_NAME"
echo ""
echo "To share with your friend:"
echo "1. Send them: $ARCHIVE_NAME"
echo "2. They extract: tar -xzf $ARCHIVE_NAME"
echo "3. They run: cd $DIST_DIR && ./connect-to-friend.sh"
echo "4. Give them your three-word address when you start your node!"