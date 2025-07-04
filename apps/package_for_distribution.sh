#!/bin/bash
# Package Saorsa Terminal Apps for distribution

echo "📦 Packaging Saorsa Apps for Distribution"
echo "========================================"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Create distribution directory
DIST_DIR="Saorsa-Apps-$(date +%Y%m%d)"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Check if apps exist
if [ ! -d "Saorsa Terminal Chat.app" ] || [ ! -d "Saorsa Network Tester.app" ]; then
    echo "⚠️  App bundles not found. Creating them first..."
    ./create_macos_apps.sh || {
        echo "❌ Failed to create app bundles"
        exit 1
    }
fi

echo "📁 Copying apps to distribution folder..."

# Copy the apps
cp -R "Saorsa Terminal Chat.app" "$DIST_DIR/"
cp -R "Saorsa Network Tester.app" "$DIST_DIR/"

# Create README
cat > "$DIST_DIR/README.txt" << 'EOF'
Saorsa Terminal Apps for macOS
===============================

This package contains two network testing applications:

1. Saorsa Terminal Chat
   - Real peer-to-peer chat
   - Automatic IPv6/IPv4 detection
   - No configuration needed

2. Saorsa Network Tester
   - Test network capabilities
   - Check IPv6 availability
   - Test tunnel connectivity
   - Find available ports

HOW TO USE:
-----------
1. Double-click either app to launch
2. If you see a security warning, right-click → Open
3. The app will open in Terminal automatically

FIRST TIME USE:
--------------
macOS may show: "Cannot be opened because it is from an unidentified developer"

Solution:
1. Right-click the app → Open
2. Click "Open" in the dialog
3. Or go to System Preferences → Security & Privacy → General
4. Click "Open Anyway" for the app

SHARING WITH FRIENDS:
--------------------
These apps are completely self-contained. Just share this folder!
No installation or dependencies required.

REQUIREMENTS:
------------
- macOS 10.12 or later
- Terminal.app (included with macOS)

For more information: https://github.com/dirvine/p2p
EOF

# Create a simple install script
cat > "$DIST_DIR/Install to Applications.command" << 'EOF'
#!/bin/bash
echo "Installing Saorsa Apps to Applications folder..."
echo ""

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Copy apps to Applications
cp -R "$SCRIPT_DIR/Saorsa Terminal Chat.app" /Applications/ 2>/dev/null && \
    echo "✅ Installed Saorsa Terminal Chat" || \
    echo "❌ Failed to install Saorsa Terminal Chat (try manually)"

cp -R "$SCRIPT_DIR/Saorsa Network Tester.app" /Applications/ 2>/dev/null && \
    echo "✅ Installed Saorsa Network Tester" || \
    echo "❌ Failed to install Saorsa Network Tester (try manually)"

echo ""
echo "You can find the apps in your Applications folder!"
echo "Press any key to close..."
read -n 1
EOF

chmod +x "$DIST_DIR/Install to Applications.command"

# Create uninstall script
cat > "$DIST_DIR/Uninstall.command" << 'EOF'
#!/bin/bash
echo "Uninstalling Saorsa Apps..."
echo ""

rm -rf "/Applications/Saorsa Terminal Chat.app" 2>/dev/null && \
    echo "✅ Removed Saorsa Terminal Chat" || \
    echo "ℹ️  Saorsa Terminal Chat not found"

rm -rf "/Applications/Saorsa Network Tester.app" 2>/dev/null && \
    echo "✅ Removed Saorsa Network Tester" || \
    echo "ℹ️  Saorsa Network Tester not found"

echo ""
echo "Uninstall complete!"
echo "Press any key to close..."
read -n 1
EOF

chmod +x "$DIST_DIR/Uninstall.command"

# Create ZIP archive
echo ""
echo "📦 Creating ZIP archive..."
ZIP_NAME="${DIST_DIR}.zip"
zip -r "$ZIP_NAME" "$DIST_DIR" -x "*.DS_Store" > /dev/null

# Calculate sizes
DIST_SIZE=$(du -sh "$DIST_DIR" | cut -f1)
ZIP_SIZE=$(ls -lh "$ZIP_NAME" | awk '{print $5}')

echo ""
echo -e "${GREEN}✅ Distribution package created!${NC}"
echo ""
echo "📊 Package Info:"
echo "  • Folder: $DIST_DIR/ ($DIST_SIZE)"
echo "  • Archive: $ZIP_NAME ($ZIP_SIZE)"
echo ""
echo "📤 Ready to share!"
echo "  Just send the ZIP file to your friends."
echo "  They can unzip and double-click the apps!"
echo ""
echo "Contents:"
ls -la "$DIST_DIR/" | grep -v "^total" | tail -n +2