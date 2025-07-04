#!/bin/bash
# Rebuild script after fixing network tester issues

echo "🔧 Rebuilding Saorsa Apps with Fixes"
echo "===================================="
echo ""
echo "Fixed issues:"
echo "✅ Added main loop with quit option"
echo "✅ Fixed tunnel test hanging with timeouts"
echo "✅ Improved app bundle launcher robustness"
echo ""

# Build the apps
echo "Building apps..."
cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester || {
    echo "Build failed!"
    exit 1
}

# Create app bundles
echo ""
echo "Creating app bundles..."
cd apps && ./create_macos_apps.sh

echo ""
echo "✅ Apps rebuilt successfully!"
echo ""
echo "Test the apps by double-clicking:"
echo "• Saorsa Network Tester.app"
echo "• Saorsa Terminal Chat.app"