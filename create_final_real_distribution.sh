#!/bin/bash
# Create final distribution with real P2P apps

echo "📦 Creating Final Real P2P Distribution"
echo "======================================"
echo ""

# Create distribution directory
DIST_DIR="p2p-real-apps"
rm -rf $DIST_DIR
mkdir -p $DIST_DIR

# Copy the Python apps
cp p2p_chat_real.py "$DIST_DIR/P2P-Chat"
cp p2p_network_tester.py "$DIST_DIR/P2P-Network-Tester"

# Make them executable
chmod +x $DIST_DIR/*

# Create wrapper scripts for better UX
cat > "$DIST_DIR/start-chat.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
python3 P2P-Chat
EOF
chmod +x "$DIST_DIR/start-chat.sh"

cat > "$DIST_DIR/start-chat.bat" << 'EOF'
@echo off
cd /d "%~dp0"
python P2P-Chat
pause
EOF

cat > "$DIST_DIR/test-network.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
python3 P2P-Network-Tester
EOF
chmod +x "$DIST_DIR/test-network.sh"

cat > "$DIST_DIR/test-network.bat" << 'EOF'
@echo off
cd /d "%~dp0"
python P2P-Network-Tester
pause
EOF

# Create README
cat > "$DIST_DIR/README.txt" << 'EOF'
P2P Foundation - Real Chat & Network Testing
============================================

This package contains REAL P2P applications:
- No fake users or simulated connections
- Actual network testing with port handling
- Direct IPv6 detection and fallback to IPv4

QUICK START:
------------
1. P2P Chat:
   - Host: Run P2P-Chat, choose option 1
   - Join: Run P2P-Chat, choose option 2, enter host's port
   
2. Network Tester:
   - Run P2P-Network-Tester
   - Choose Quick Test or Port Scan
   - Handles busy ports gracefully

REQUIREMENTS:
-------------
- Python 3 (usually pre-installed on Mac/Linux)
- No other dependencies needed!

HOW IT WORKS:
-------------
The chat app:
- Detects if IPv6 is available (no tunnel needed)
- Falls back to IPv4 if necessary
- Shows what tunnel would be used in production
- Creates real TCP connections between peers
- No fake "river-quick-fox" user!

The network tester:
- Tests actual network capabilities
- Handles port conflicts gracefully
- Finds available ports automatically
- Shows real network status

USAGE:
------
Host a chat:
1. Run: ./P2P-Chat (or python3 P2P-Chat)
2. Choose option 1
3. Note the port number shown
4. Share port with friends

Join a chat:
1. Run: ./P2P-Chat
2. Choose option 2
3. Enter the port number from host
4. Start chatting!

Test network:
1. Run: ./P2P-Network-Tester
2. Choose Quick Test
3. See real network capabilities
EOF

# Create the archive
ARCHIVE_NAME="p2p-real-apps-$(date +%Y%m%d-%H%M).zip"
cd $DIST_DIR
zip -r ../$ARCHIVE_NAME . >/dev/null 2>&1
cd ..

echo "✅ Success! Created real P2P distribution:"
echo "   📁 Folder: $DIST_DIR/"
echo "   📦 Archive: $ARCHIVE_NAME"
echo ""
echo "Contents:"
ls -la $DIST_DIR/
echo ""
echo "These apps:"
echo "• Actually connect to each other"
echo "• Detect IPv6 vs IPv4 properly"
echo "• Handle port conflicts gracefully"
echo "• Show real tunnel requirements"
echo "• No fake users or responses!"