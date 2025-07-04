#!/bin/bash
# Build script after fixing compilation errors

echo "🔨 Building Fixed Saorsa Apps"
echo "============================"
echo ""

# Build both apps
echo "Building apps with fixes..."
cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Build successful!"
    echo ""
    
    # Create app bundles
    echo "Creating app bundles..."
    cd apps
    
    # Use the fixed version if it exists, otherwise use the original
    if [ -f "create_macos_apps_fixed.sh" ]; then
        chmod +x create_macos_apps_fixed.sh
        ./create_macos_apps_fixed.sh
    else
        ./create_macos_apps.sh
    fi
    
    echo ""
    echo "🎉 Apps are ready!"
    echo ""
    echo "Double-click to test:"
    echo "• Saorsa Terminal Chat.app"
    echo "• Saorsa Network Tester.app"
else
    echo ""
    echo "❌ Build failed. Please check the errors above."
fi