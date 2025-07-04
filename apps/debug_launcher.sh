#!/bin/bash
# Debug script to test app bundle issues

echo "🔍 Debugging App Bundle Launcher"
echo "================================"
echo ""

# Check if app bundles exist
for APP in "Saorsa Terminal Chat.app" "Saorsa Network Tester.app"; do
    echo "Checking $APP..."
    if [ -d "$APP" ]; then
        echo "✅ App bundle exists"
        
        # Check launcher
        LAUNCHER="$APP/Contents/MacOS/launcher"
        if [ -f "$LAUNCHER" ]; then
            echo "✅ Launcher exists"
            echo "   Permissions: $(ls -l "$LAUNCHER" | awk '{print $1}')"
        else
            echo "❌ Launcher missing!"
        fi
        
        # Check binary
        if [ "$APP" == "Saorsa Terminal Chat.app" ]; then
            BINARY="$APP/Contents/MacOS/saorsa-terminal-chat"
        else
            BINARY="$APP/Contents/MacOS/saorsa-network-tester"
        fi
        
        if [ -f "$BINARY" ]; then
            echo "✅ Binary exists"
            echo "   Size: $(ls -lh "$BINARY" | awk '{print $5}')"
            echo "   Permissions: $(ls -l "$BINARY" | awk '{print $1}')"
        else
            echo "❌ Binary missing!"
        fi
        
        # Check Info.plist
        if [ -f "$APP/Contents/Info.plist" ]; then
            echo "✅ Info.plist exists"
        else
            echo "❌ Info.plist missing!"
        fi
    else
        echo "❌ App bundle not found!"
    fi
    echo ""
done

# Test launching directly
echo "Testing direct binary execution..."
if [ -f "../target/release/saorsa-network-tester" ]; then
    echo "Binary found at: ../target/release/saorsa-network-tester"
    echo "You can test it directly with:"
    echo "  ../target/release/saorsa-network-tester"
else
    echo "❌ Binary not found in target/release!"
fi