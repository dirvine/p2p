#!/bin/bash

echo "🚀 Starting P2P Chat Demo"
echo "=========================="
echo

# Kill any existing instances
pkill -f "target/debug/examples/chat" 2>/dev/null || true

# Clean start
sleep 1

echo "📡 Starting Bootstrap Node (Instance 1)..."
echo "   Peer ID: Will be shown below"
echo "   Listen Address: /ip6/::/tcp/9000"
echo

# Start first instance and capture output
cargo run --example chat -- --listen-address "/ip6/::/tcp/9000" > node1.log 2>&1 &
NODE1_PID=$!

# Wait for it to start
sleep 2

# Show what happened
echo "Node 1 Output:"
cat node1.log | head -10
echo

echo "📡 Starting Second Node (Instance 2)..."
echo "   Will bootstrap from Node 1"
echo

# Start second instance, bootstrapping from first
cargo run --example chat -- --listen-address "/ip6/::/udp/9001/quic" --bootstrap "/ip6/::1/tcp/9000" > node2.log 2>&1 &
NODE2_PID=$!

# Wait for connection
sleep 3

echo "Node 2 Output:"
cat node2.log | head -10
echo

echo "🔗 Both nodes are now running!"
echo "   Node 1 PID: $NODE1_PID"  
echo "   Node 2 PID: $NODE2_PID"
echo
echo "📱 Flutter App Connection Info:"
echo "   Use these addresses in your Flutter app:"
echo "   Node 1: /ip6/::1/tcp/9000"
echo "   Node 2: /ip6/::1/udp/9001/quic"
echo
echo "💬 The nodes are chatting in the background!"
echo "   Check node1.log and node2.log for activity"
echo

# Keep running for demo
echo "⏰ Demo will run for 30 seconds..."
sleep 30

echo "🛑 Stopping demo nodes..."
kill $NODE1_PID $NODE2_PID 2>/dev/null || true
echo "Demo completed!"