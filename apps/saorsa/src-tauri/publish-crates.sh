#!/bin/bash

# Script to publish Saorsa to crates.io with bundled frontend

echo "🚀 Preparing Saorsa for crates.io publication..."

# Change to the src-tauri directory
cd "$(dirname "$0")"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Build with frontend bundling enabled
echo "📦 Building with bundled frontend..."
BUNDLE_FRONTEND=1 cargo build --release --features bundle-frontend

# Test the binary works
echo "🧪 Testing binary..."
BUNDLE_FRONTEND=1 cargo test --features bundle-frontend

# Package for crates.io
echo "📦 Creating package..."
BUNDLE_FRONTEND=1 cargo package --features bundle-frontend

# Verify the package
echo "✅ Verifying package..."
BUNDLE_FRONTEND=1 cargo package --features bundle-frontend --list

echo ""
echo "📋 Package ready! To publish to crates.io, run:"
echo "   BUNDLE_FRONTEND=1 cargo publish --features bundle-frontend"
echo ""
echo "💡 Users will be able to install with:"
echo "   cargo install saorsa"
echo ""
echo "🎯 And run the app with:"
echo "   saorsa"