#!/bin/bash
# Create macOS app bundles for Saorsa terminal applications

echo "🍎 Creating macOS App Bundles"
echo "============================"
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
    
    # Create launcher script
    cat > "$BUNDLE_NAME/Contents/MacOS/launcher" << EOF
#!/bin/bash
# Launcher for $APP_NAME

# Get the directory of this script
DIR="\$(cd "\$(dirname "\$0")" && pwd)"

# Create a temporary script that will run our app
TEMP_SCRIPT=\$(mktemp /tmp/saorsa_launcher.XXXXXX.sh)
cat > "\$TEMP_SCRIPT" << 'SCRIPT'
#!/bin/bash
clear
echo "Starting $APP_NAME..."
echo ""

# Change to app directory to ensure proper working directory
cd "\$DIR" || exit 1

# Check if binary exists
if [ ! -f "./$BINARY_NAME" ]; then
    echo "Error: Application binary not found!"
    echo "Expected: \$DIR/$BINARY_NAME"
    echo ""
    echo "Press any key to close..."
    read -n 1
    exit 1
fi

# Run the application
"./$BINARY_NAME"
EXIT_CODE=\$?

echo ""
if [ \$EXIT_CODE -eq 0 ]; then
    echo "Application exited successfully."
else
    echo "Application exited with code: \$EXIT_CODE"
fi
echo ""
echo "Press any key to close this window..."
read -n 1
exit
SCRIPT

chmod +x "\$TEMP_SCRIPT"

# Open Terminal with our script
osascript << APPLESCRIPT
tell application "Terminal"
    activate
    do script "\$TEMP_SCRIPT"
    set current settings of first window to settings set "Basic"
    set title displays custom title of first window to true
    set custom title of first window to "$APP_NAME"
end tell
APPLESCRIPT
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
</dict>
</plist>
EOF

    # Create a simple icon (optional - creates a text file that macOS will use)
    echo "$ICON_EMOJI" > "$BUNDLE_NAME/Contents/Resources/icon.txt"
    
    # Try to extract and use icons from saorsa_iconset.zip if available
    if [ -f "/Users/davidirvine/Downloads/saorsa_iconset.zip" ]; then
        TEMP_ICON_DIR=$(mktemp -d)
        unzip -q "/Users/davidirvine/Downloads/saorsa_iconset.zip" -d "$TEMP_ICON_DIR" 2>/dev/null
        
        # Find appropriate PNG file
        if [[ "$BINARY_NAME" == *"chat"* ]]; then
            ICON_PNG=$(find "$TEMP_ICON_DIR" -name "*.png" | grep -i "chat\|message\|talk" | head -1)
        else
            ICON_PNG=$(find "$TEMP_ICON_DIR" -name "*.png" | grep -i "network\|test\|diagnostic" | head -1)
        fi
        
        # If no specific icon found, use any PNG
        if [ -z "$ICON_PNG" ]; then
            ICON_PNG=$(find "$TEMP_ICON_DIR" -name "*.png" | head -1)
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
    
    echo -e "${GREEN}✅ Created $BUNDLE_NAME${NC}"
}

# Create app bundles
create_app_bundle "Saorsa Terminal Chat" "saorsa-terminal-chat" "com.saorsa.terminal-chat" "💬"
create_app_bundle "Saorsa Network Tester" "saorsa-network-tester" "com.saorsa.network-tester" "🔍"

echo ""
echo "🎉 App bundles created successfully!"
echo ""
echo "You can now:"
echo "1. Double-click the .app files to run them"
echo "2. Drag them to your Applications folder"
echo "3. Share them with friends (they're self-contained!)"
echo ""
echo "Note: First time running, macOS might show a security warning."
echo "      Right-click → Open to bypass this, or go to"
echo "      System Preferences → Security & Privacy to allow."
echo ""
echo "Created apps:"
ls -la *.app 2>/dev/null | grep "^d" | awk '{print "  • " $9}' || echo "  Check the apps directory for .app bundles"