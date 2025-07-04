#!/bin/bash
# Create macOS app bundles for Saorsa terminal applications

# Function to create app bundle
create_app_bundle() {
    local app_name="$1"
    local binary_name="$2"
    local identifier="$3"
    local icon_emoji="$4"
    
    echo "📦 Creating $app_name.app..."
    
    # Create app bundle structure
    local app_path="$app_name.app"
    mkdir -p "$app_path/Contents/MacOS"
    mkdir -p "$app_path/Contents/Resources"
    
    # Copy binary
    if [ -f "target/release/$binary_name" ]; then
        cp "target/release/$binary_name" "$app_path/Contents/MacOS/$app_name"
        chmod +x "$app_path/Contents/MacOS/$app_name"
    else
        echo "❌ Error: Binary not found at target/release/$binary_name"
        echo "   Please run ./build-terminal-apps.sh first"
        return 1
    fi
    
    # Create Info.plist
    cat > "$app_path/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$app_name</string>
    <key>CFBundleIdentifier</key>
    <string>$identifier</string>
    <key>CFBundleName</key>
    <string>$app_name</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.2.6</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>LSUIElement</key>
    <false/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

    # Create a simple icon (placeholder - you can replace with actual .icns file)
    echo "$icon_emoji $app_name" > "$app_path/Contents/Resources/icon.txt"
    
    echo "✅ Created $app_name.app"
    
    # Create launch script
    local launch_script="${app_name// /-}.sh"
    cat > "$launch_script" << EOF
#!/bin/bash
# Launch $app_name
open "$app_path"
EOF
    chmod +x "$launch_script"
    echo "✅ Created launch script: $launch_script"
    
    return 0
}

# Change to project directory
cd "$(dirname "$0")"

echo "🎨 Creating macOS App Bundles for Saorsa Terminal Apps"
echo "===================================================="
echo ""

# Create bundles directory
mkdir -p bundles
cd bundles

# Create Saorsa Terminal Chat bundle
create_app_bundle "Saorsa Terminal Chat" "saorsa-terminal-chat" "com.saorsa.terminal-chat" "💬"

echo ""

# Create Saorsa Network Tester bundle
create_app_bundle "Saorsa Network Tester" "saorsa-network-tester" "com.saorsa.network-tester" "🔧"

echo ""
echo "🎉 App bundles created successfully!"
echo ""
echo "App bundles are located in the 'bundles' directory:"
echo "  • Saorsa Terminal Chat.app"
echo "  • Saorsa Network Tester.app"
echo ""
echo "To run the apps:"
echo "  • Double-click the .app bundles in Finder"
echo "  • Or use the launch scripts: ./saorsa-terminal-chat.sh or ./saorsa-network-tester.sh"
echo ""
echo "To distribute:"
echo "  • Compress the .app bundles as .zip files"
echo "  • Share with users who can then drag to Applications folder"