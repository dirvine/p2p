#!/bin/bash
# Quick check script to verify terminal apps can be built

cd "$(dirname "$0")"

echo "Checking terminal chat app..."
cargo check -p saorsa-terminal-chat 2>&1 | head -20

echo ""
echo "Checking network tester app..."  
cargo check -p saorsa-network-tester 2>&1 | head -20

echo ""
echo "To build the apps, run: ./build-terminal-apps.sh"