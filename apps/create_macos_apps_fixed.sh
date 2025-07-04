#!/bin/bash
# Create macOS app bundles with improved launcher

echo "🍎 Creating macOS App Bundles (Fixed Version)"
echo "==========================================="
echo ""

# Check if binaries exist
if [ ! -f "../target/release/saorsa-terminal-chat" ] || [ ! -f "../target/release/saorsa-network-tester" ]; then
    echo "⚠️  Binaries not found. Building them first..."
    cd .. && cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester || {
        echo "❌ Build failed. Please run: cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester"
        exit 1
    }
    cd apps
fi

# Colors
GREEN='\033[0;32m'
NC='\033[0m'

# Function to create app bundle
create_app_bundle() {
    local APP_NAME="$1"
    local BINARY_NAME="$2"
    local BUNDLE_ID="$3"
    local ICON_EMOJI="$4"
    local BUNDLE_NAME="$APP_NAME.app"
    
    echo "Creating $BUNDLE_NAME..."
    
    # Clean up old bundle
    rm -rf "$BUNDLE_NAME"
    
    # Create directory structure
    mkdir -p "$BUNDLE_NAME/Contents/MacOS"
    mkdir -p "$BUNDLE_NAME/Contents/Resources"
    
    # Copy binary
    cp ../target/release/$BINARY_NAME "$BUNDLE_NAME/Contents/MacOS/" || {
        echo "❌ Failed to copy $BINARY_NAME"
        return 1
    }
    
    # Make binary executable
    chmod +x "$BUNDLE_NAME/Contents/MacOS/$BINARY_NAME"
    
    # Create launcher script - SIMPLER VERSION
    cat > "$BUNDLE_NAME/Contents/MacOS/launcher" << EOF
#!/bin/bash
# Simple launcher for $APP_NAME

# Get the directory containing this script
SCRIPT_DIR="\$(cd "\$(dirname "\$0")" && pwd)"

# Use osascript to launch Terminal with our app
osascript -e "
tell application \"Terminal\"
    activate
    set newWindow to do script \"clear && echo 'Starting $APP_NAME...' && echo '' && '\$SCRIPT_DIR/$BINARY_NAME'; echo ''; echo 'Press any key to close...'; read -n 1\"
    set custom title of newWindow to \"$APP_NAME\"
end tell
"
EOF
    
    chmod +x "$BUNDLE_NAME/Contents/MacOS/launcher"
    
    # Create Info.plist
    cat > "$BUNDLE_NAME/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>launcher</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSRequiresNativeExecution</key>
    <true/>
</dict>
</plist>
EOF

    # Create a simple icon placeholder
    echo "$ICON_EMOJI" > "$BUNDLE_NAME/Contents/Resources/icon.txt"
    
    # Try to extract icons from saorsa_iconset.zip if available
    if [ -f "/Users/davidirvine/Downloads/saorsa_iconset.zip" ]; then
        TEMP_ICON_DIR=$(mktemp -d)
        unzip -q "/Users/davidirvine/Downloads/saorsa_iconset.zip" -d "$TEMP_ICON_DIR" 2>/dev/null || true
        
        # Find appropriate PNG file
        if [[ "$BINARY_NAME" == *"chat"* ]]; then
            ICON_PNG=$(find "$TEMP_ICON_DIR" -name "*.png" 2>/dev/null | grep -i "chat\|message\|talk" | head -1)
        else
            ICON_PNG=$(find "$TEMP_ICON_DIR" -name "*.png" 2>/dev/null | grep -i "network\|test\|diagnostic" | head -1)
        fi
        
        # If no specific icon found, use any PNG
        if [ -z "$ICON_PNG" ]; then
            ICON_PNG=$(find "$TEMP_ICON_DIR" -name "*.png" 2>/dev/null | head -1)
        fi
        
        # Copy PNG if found
        if [ -n "$ICON_PNG" ] && [ -f "$ICON_PNG" ]; then
            cp "$ICON_PNG" "$BUNDLE_NAME/Contents/Resources/AppIcon.png"
            echo "  📸 Added PNG icon from saorsa_iconset.zip"
        fi
        
        rm -rf "$TEMP_ICON_DIR"
    fi
    
    # Set bundle bit (makes it appear as an app)
    SetFile -a B "$BUNDLE_NAME" 2>/dev/null || true
    
    # Also create a direct launcher for testing
    cat > "$BUNDLE_NAME/Contents/MacOS/run-direct.sh" << EOF
#!/bin/bash
# Direct runner for testing
cd "\$(dirname "\$0")"
./$BINARY_NAME
EOF
    chmod +x "$BUNDLE_NAME/Contents/MacOS/run-direct.sh"
    
    echo -e "${GREEN}✅ Created $BUNDLE_NAME${NC}"
    echo "   Binary: $BUNDLE_NAME/Contents/MacOS/$BINARY_NAME"
    echo "   Launcher: $BUNDLE_NAME/Contents/MacOS/launcher"
    echo "   Direct test: $BUNDLE_NAME/Contents/MacOS/run-direct.sh"
}

# Create app bundles
create_app_bundle "Saorsa Terminal Chat" "saorsa-terminal-chat" "com.saorsa.terminal-chat" "💬"
create_app_bundle "Saorsa Network Tester" "saorsa-network-tester" "com.saorsa.network-tester" "🔍"

echo ""
echo "🎉 App bundles created successfully!"
echo ""
echo "If double-clicking doesn't work, try:"
echo "1. Right-click → Open (for security bypass)"
echo "2. Run directly in Terminal:"
echo "   ./Saorsa\\ Network\\ Tester.app/Contents/MacOS/run-direct.sh"
echo "   ./Saorsa\\ Terminal\\ Chat.app/Contents/MacOS/run-direct.sh"
echo ""
echo "To debug launcher issues:"
echo "   ./Saorsa\\ Network\\ Tester.app/Contents/MacOS/launcher"