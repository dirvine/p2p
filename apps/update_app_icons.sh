#!/bin/bash
# Update macOS app bundles with icons from saorsa_iconset.zip

echo "🎨 Updating App Icons"
echo "===================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if the iconset exists in Downloads
ICONSET_PATH="/Users/davidirvine/Downloads/saorsa_iconset.zip"
if [ ! -f "$ICONSET_PATH" ]; then
    echo -e "${YELLOW}⚠️  Icon set not found at: $ICONSET_PATH${NC}"
    exit 1
fi

# Create temporary directory for extraction
TEMP_DIR=$(mktemp -d)
echo "Extracting icons to temporary directory..."

# Extract the iconset
unzip -q "$ICONSET_PATH" -d "$TEMP_DIR" || {
    echo "❌ Failed to extract iconset"
    rm -rf "$TEMP_DIR"
    exit 1
}

echo "Contents of iconset:"
ls -la "$TEMP_DIR"

# Function to convert PNG to ICNS
create_icns_from_png() {
    local PNG_PATH="$1"
    local ICNS_PATH="$2"
    local APP_NAME="$3"
    
    if [ -f "$PNG_PATH" ]; then
        echo "Converting $APP_NAME icon to .icns format..."
        
        # Create iconset directory
        ICONSET_DIR="${TEMP_DIR}/${APP_NAME}.iconset"
        mkdir -p "$ICONSET_DIR"
        
        # Generate all required sizes using sips
        sips -z 16 16 "$PNG_PATH" --out "${ICONSET_DIR}/icon_16x16.png" >/dev/null 2>&1
        sips -z 32 32 "$PNG_PATH" --out "${ICONSET_DIR}/icon_16x16@2x.png" >/dev/null 2>&1
        sips -z 32 32 "$PNG_PATH" --out "${ICONSET_DIR}/icon_32x32.png" >/dev/null 2>&1
        sips -z 64 64 "$PNG_PATH" --out "${ICONSET_DIR}/icon_32x32@2x.png" >/dev/null 2>&1
        sips -z 128 128 "$PNG_PATH" --out "${ICONSET_DIR}/icon_128x128.png" >/dev/null 2>&1
        sips -z 256 256 "$PNG_PATH" --out "${ICONSET_DIR}/icon_128x128@2x.png" >/dev/null 2>&1
        sips -z 256 256 "$PNG_PATH" --out "${ICONSET_DIR}/icon_256x256.png" >/dev/null 2>&1
        sips -z 512 512 "$PNG_PATH" --out "${ICONSET_DIR}/icon_256x256@2x.png" >/dev/null 2>&1
        sips -z 512 512 "$PNG_PATH" --out "${ICONSET_DIR}/icon_512x512.png" >/dev/null 2>&1
        sips -z 1024 1024 "$PNG_PATH" --out "${ICONSET_DIR}/icon_512x512@2x.png" >/dev/null 2>&1
        
        # Convert to icns
        iconutil -c icns "$ICONSET_DIR" -o "$ICNS_PATH" 2>/dev/null
        
        if [ -f "$ICNS_PATH" ]; then
            echo -e "${GREEN}✅ Created .icns for $APP_NAME${NC}"
            return 0
        else
            echo "⚠️  Could not create .icns file, using PNG instead"
            return 1
        fi
    else
        echo "⚠️  PNG not found: $PNG_PATH"
        return 1
    fi
}

# Update Terminal Chat app
CHAT_APP="Saorsa Terminal Chat.app"
if [ -d "$CHAT_APP" ]; then
    echo ""
    echo "Updating $CHAT_APP..."
    
    # Look for chat-related icon
    CHAT_PNG=$(find "$TEMP_DIR" -name "*chat*.png" -o -name "*message*.png" -o -name "*talk*.png" | head -1)
    if [ -z "$CHAT_PNG" ]; then
        CHAT_PNG=$(find "$TEMP_DIR" -name "*.png" | grep -v "network\|test" | head -1)
    fi
    
    if [ -n "$CHAT_PNG" ]; then
        # Copy PNG
        cp "$CHAT_PNG" "$CHAT_APP/Contents/Resources/AppIcon.png"
        
        # Try to create ICNS
        create_icns_from_png "$CHAT_PNG" "$CHAT_APP/Contents/Resources/AppIcon.icns" "Chat"
    fi
fi

# Update Network Tester app
TESTER_APP="Saorsa Network Tester.app"
if [ -d "$TESTER_APP" ]; then
    echo ""
    echo "Updating $TESTER_APP..."
    
    # Look for network/test-related icon
    TESTER_PNG=$(find "$TEMP_DIR" -name "*network*.png" -o -name "*test*.png" -o -name "*diagnostic*.png" | head -1)
    if [ -z "$TESTER_PNG" ]; then
        TESTER_PNG=$(find "$TEMP_DIR" -name "*.png" | grep -v "chat\|message" | tail -1)
    fi
    
    if [ -n "$TESTER_PNG" ]; then
        # Copy PNG
        cp "$TESTER_PNG" "$TESTER_APP/Contents/Resources/AppIcon.png"
        
        # Try to create ICNS
        create_icns_from_png "$TESTER_PNG" "$TESTER_APP/Contents/Resources/AppIcon.icns" "Tester"
    fi
fi

# Clean up
rm -rf "$TEMP_DIR"

echo ""
echo "🎉 Icon update complete!"
echo ""
echo "Note: The apps now have proper icons!"
echo "If you want different icons, place PNG files named:"
echo "  - saorsa-chat-icon.png (for Terminal Chat)"
echo "  - saorsa-network-icon.png (for Network Tester)"
echo "in the apps directory and run this script again."