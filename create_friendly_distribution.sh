#!/bin/bash
# Create user-friendly distribution package

echo "📦 Creating User-Friendly P2P Distribution"
echo "=========================================="
echo ""

# Create distribution directory
DIST_DIR="p2p-friendly-apps"
rm -rf $DIST_DIR
mkdir -p $DIST_DIR

# Copy the applications
cp p2p-chat-friendly $DIST_DIR/P2P-Chat
cp p2p-network-tester $DIST_DIR/P2P-Network-Tester

# Make them executable
chmod +x $DIST_DIR/*

# Create a simple README
cat > $DIST_DIR/README.txt << 'EOF'
P2P Foundation - Easy Setup Guide
=================================

You have two applications:

1. P2P-Chat
   - Double-click to start
   - Choose "Start new chat" or "Join friend's chat"
   - Everything else is automatic!

2. P2P-Network-Tester
   - Double-click to start
   - Choose your test type
   - Watch the results in real-time

That's it! No configuration needed.

For help: https://github.com/dirvine/p2p
EOF

# Create macOS app bundles if on Mac
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🍎 Creating macOS app bundles..."
    
    # Create P2P Chat app
    mkdir -p "$DIST_DIR/P2P Chat.app/Contents/MacOS"
    mkdir -p "$DIST_DIR/P2P Chat.app/Contents/Resources"
    
    cp "$DIST_DIR/P2P-Chat" "$DIST_DIR/P2P Chat.app/Contents/MacOS/"
    
    cat > "$DIST_DIR/P2P Chat.app/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>P2P-Chat</string>
    <key>CFBundleIdentifier</key>
    <string>com.p2pfoundation.chat</string>
    <key>CFBundleName</key>
    <string>P2P Chat</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
EOF

    # Create Network Tester app
    mkdir -p "$DIST_DIR/P2P Network Tester.app/Contents/MacOS"
    mkdir -p "$DIST_DIR/P2P Network Tester.app/Contents/Resources"
    
    cp "$DIST_DIR/P2P-Network-Tester" "$DIST_DIR/P2P Network Tester.app/Contents/MacOS/"
    
    cat > "$DIST_DIR/P2P Network Tester.app/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>P2P-Network-Tester</string>
    <key>CFBundleIdentifier</key>
    <string>com.p2pfoundation.tester</string>
    <key>CFBundleName</key>
    <string>P2P Network Tester</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
EOF
fi

# Create Windows batch files if needed
cat > "$DIST_DIR/P2P-Chat.bat" << 'EOF'
@echo off
start "P2P Chat" P2P-Chat.exe
EOF

cat > "$DIST_DIR/P2P-Network-Tester.bat" << 'EOF'
@echo off
start "P2P Network Tester" P2P-Network-Tester.exe
EOF

# Create the archive
ARCHIVE_NAME="p2p-friendly-apps-$(date +%Y%m%d).zip"
cd $DIST_DIR
zip -r ../$ARCHIVE_NAME . >/dev/null 2>&1
cd ..

echo ""
echo "✅ Success! Created user-friendly distribution:"
echo "   📁 Folder: $DIST_DIR/"
echo "   📦 Archive: $ARCHIVE_NAME"
echo ""
echo "To share with friends:"
echo "1. Send them: $ARCHIVE_NAME"
echo "2. They unzip and double-click the apps"
echo "3. No terminal or technical knowledge needed!"
echo ""
echo "The apps will:"
echo "• Auto-detect their network"
echo "• Set up any needed tunnels"
echo "• Show friendly progress messages"
echo "• Guide them through everything"