#!/bin/bash
echo "Building saorsa-terminal-chat..."
cargo build --release -p saorsa-terminal-chat 2>&1 | tee chat_build.log | head -50

echo ""
echo "Building saorsa-network-tester..."
cargo build --release -p saorsa-network-tester 2>&1 | tee tester_build.log | head -50

echo ""
echo "Full logs saved to chat_build.log and tester_build.log"