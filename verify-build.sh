#!/bin/bash
# Comprehensive build verification script

set -e

echo "🔍 Build Verification Script for Saorsa Terminal Apps"
echo "===================================================="
echo ""

# Check environment
echo "1. Checking environment..."
echo "   Current directory: $(pwd)"
echo "   HOME: $HOME"
echo ""

# Check if cargo exists
echo "2. Checking for cargo..."
if [ -f "$HOME/.cargo/bin/cargo" ]; then
    echo "   ✓ Found cargo at: $HOME/.cargo/bin/cargo"
    export PATH="$HOME/.cargo/bin:$PATH"
else
    echo "   ✗ Cargo not found at expected location"
    echo "   Please install Rust from https://rustup.rs/"
    exit 1
fi

# Change to project directory
cd "$(dirname "$0")"

echo ""
echo "3. Project structure check..."
if [ -f "Cargo.toml" ]; then
    echo "   ✓ Found workspace Cargo.toml"
else
    echo "   ✗ Missing workspace Cargo.toml"
    exit 1
fi

if [ -d "apps/saorsa-terminal-chat" ]; then
    echo "   ✓ Found saorsa-terminal-chat"
else
    echo "   ✗ Missing saorsa-terminal-chat directory"
fi

if [ -d "apps/saorsa-network-tester" ]; then
    echo "   ✓ Found saorsa-network-tester"
else
    echo "   ✗ Missing saorsa-network-tester directory"
fi

echo ""
echo "4. Checking workspace members..."
grep -A20 "members = \[" Cargo.toml | head -20

echo ""
echo "5. Attempting to build saorsa-terminal-chat..."
echo "   Command: cargo build --release -p saorsa-terminal-chat"
echo "   Output:"
echo "   ========================================"
cargo build --release -p saorsa-terminal-chat 2>&1 || echo "   Build failed with exit code: $?"

echo ""
echo "6. Attempting to build saorsa-network-tester..."
echo "   Command: cargo build --release -p saorsa-network-tester"
echo "   Output:"
echo "   ========================================"
cargo build --release -p saorsa-network-tester 2>&1 || echo "   Build failed with exit code: $?"

echo ""
echo "7. Checking for compilation errors in source files..."
echo ""
echo "   Checking saorsa-terminal-chat/src/main.rs syntax..."
cargo check --message-format=short -p saorsa-terminal-chat 2>&1 | grep -E "error|warning" | head -20 || echo "   No syntax errors found"

echo ""
echo "   Checking saorsa-network-tester/src/main.rs syntax..."
cargo check --message-format=short -p saorsa-network-tester 2>&1 | grep -E "error|warning" | head -20 || echo "   No syntax errors found"

echo ""
echo "Build verification complete!"
echo ""
echo "If builds failed, the errors above will help identify the issues."