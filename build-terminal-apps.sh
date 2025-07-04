#!/bin/bash
# Build script for Saorsa terminal applications

# Source cargo environment if available
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Add cargo to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "🚀 Building Saorsa terminal applications..."
echo "========================================="

# Change to project directory
cd "$(dirname "$0")"

# Build terminal chat
echo ""
echo "📦 Building saorsa-terminal-chat..."
cargo build --release -p saorsa-terminal-chat
if [ $? -eq 0 ]; then
    echo "✅ saorsa-terminal-chat built successfully!"
else
    echo "❌ Failed to build saorsa-terminal-chat"
    exit 1
fi

# Build network tester
echo ""
echo "📦 Building saorsa-network-tester..."
cargo build --release -p saorsa-network-tester
if [ $? -eq 0 ]; then
    echo "✅ saorsa-network-tester built successfully!"
else
    echo "❌ Failed to build saorsa-network-tester"
    exit 1
fi

echo ""
echo "🎉 All applications built successfully!"
echo ""
echo "Binaries are located at:"
echo "  • target/release/saorsa-terminal-chat"
echo "  • target/release/saorsa-network-tester"
echo ""
echo "To create macOS app bundles, run:"
echo "  ./create-macos-bundles.sh"