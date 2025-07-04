#!/bin/bash
# Create final user-friendly distribution package

echo "📦 Creating Final P2P Distribution Package"
echo "========================================="
echo ""

# Create distribution directory
DIST_DIR="p2p-apps-final"
rm -rf $DIST_DIR
mkdir -p $DIST_DIR

# Copy the stable chat app
cp p2p-chat-stable "$DIST_DIR/P2P-Chat"

# Also copy the network tester
cp p2p-network-tester "$DIST_DIR/P2P-Network-Tester" 2>/dev/null || echo "Network tester not found, skipping"

# Make them executable
chmod +x $DIST_DIR/*

# Create a README
cat > $DIST_DIR/README.txt << 'EOF'
P2P Foundation - Click & Connect
================================

Just double-click the apps to start!

P2P-Chat:
---------
• Choose option 1 to host a chat
• Choose option 2 to join a chat
• Everything else is automatic!

The app will:
- Show your three-word address
- Display what type of tunnel it's using
- Handle all the technical stuff
- Let you chat with friends

That's it! No setup needed.
EOF

# Create start scripts for different platforms
# macOS/Linux script
cat > $DIST_DIR/start-chat.sh << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
./P2P-Chat
EOF
chmod +x $DIST_DIR/start-chat.sh

# Windows batch file
cat > $DIST_DIR/start-chat.bat << 'EOF'
@echo off
cd /d "%~dp0"
P2P-Chat.exe
pause
EOF

# Create a simple test script
cat > $DIST_DIR/test-chat.sh << 'EOF'
#!/bin/bash
echo "Testing P2P Chat..."
echo "1" | ./P2P-Chat
EOF
chmod +x $DIST_DIR/test-chat.sh

# If on macOS, create an app bundle
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🍎 Creating macOS app bundle..."
    
    APP_NAME="P2P Chat.app"
    mkdir -p "$DIST_DIR/$APP_NAME/Contents/MacOS"
    mkdir -p "$DIST_DIR/$APP_NAME/Contents/Resources"
    
    # Copy executable
    cp "$DIST_DIR/P2P-Chat" "$DIST_DIR/$APP_NAME/Contents/MacOS/P2P-Chat"
    
    # Create Info.plist
    cat > "$DIST_DIR/$APP_NAME/Contents/Info.plist" << 'EOF'
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
    <key>LSUIElement</key>
    <false/>
</dict>
</plist>
EOF

    # Create a launcher script that opens Terminal
    cat > "$DIST_DIR/$APP_NAME/Contents/MacOS/launcher.sh" << 'EOF'
#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
osascript -e "tell application \"Terminal\" to do script \"cd '$DIR' && ./P2P-Chat; exit\""
EOF
    chmod +x "$DIST_DIR/$APP_NAME/Contents/MacOS/launcher.sh"
fi

# Create the archive
ARCHIVE_NAME="p2p-apps-final-$(date +%Y%m%d-%H%M).zip"
cd $DIST_DIR
zip -r ../$ARCHIVE_NAME . >/dev/null 2>&1
cd ..

echo ""
echo "✅ Success! Created final distribution:"
echo "   📁 Folder: $DIST_DIR/"
echo "   📦 Archive: $ARCHIVE_NAME"
echo ""
echo "Contents:"
ls -la $DIST_DIR/
echo ""
echo "To test locally:"
echo "  cd $DIST_DIR && ./P2P-Chat"
echo ""
echo "To share with friends:"
echo "  1. Send them: $ARCHIVE_NAME"
echo "  2. They unzip and double-click P2P-Chat (or the .app on Mac)"
echo "  3. No terminal knowledge needed!"