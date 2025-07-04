#!/bin/bash
# Simple build script for Saorsa terminal apps

echo "Building Saorsa Terminal Apps..."
echo ""

# Build both apps
cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester

echo ""
echo "Build complete! Now run:"
echo "cd apps && ./create_macos_apps.sh"