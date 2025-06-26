#!/bin/bash

echo "🌟 Three-Word P2P Chat Demo"
echo "============================"
echo

# Kill any existing instances
pkill -f "target/debug/examples/chat" 2>/dev/null || true
sleep 1

echo "🚀 Starting Node 1 (Bootstrap node)..."
echo "   Will generate a three-word address for easy sharing"

# Start first node and capture its three-word address
cargo run --example chat -- --listen-address "/ip6/::/tcp/9000" > node1_three_word.log 2>&1 &
NODE1_PID=$!

echo "   Node 1 PID: $NODE1_PID"
echo "   Waiting for startup..."
sleep 3

echo
echo "📋 Node 1 Details:"
echo "=================="
cat node1_three_word.log | grep -E "(Listening|Share-friendly|Tell friends)"
echo

echo "🔗 Now testing three-word bootstrap..."
echo "======================================"
echo "   Traditional way: --bootstrap '/ip6/::1/tcp/9000'"
echo "   Three-word way:  --bootstrap-words 'global.fast.eagle'"
echo

echo "🧪 Testing three-word bootstrap parsing:"
cargo run --example chat -- --bootstrap-words "global.fast.eagle" --bootstrap-words "invalid.words.test" --listen-address "/ip6/::/udp/9001/quic" 2>&1 | head -20

echo
echo "🛑 Stopping demo..."
kill $NODE1_PID 2>/dev/null || true
echo "Demo completed!"

echo
echo "🎉 Key Benefits Demonstrated:"
echo "   ✅ Human-friendly addresses: quick.strong.sword vs /ip6/::/tcp/9000"
echo "   ✅ Easy sharing: 'Connect to quick.strong.sword'"
echo "   ✅ Validation: Catches invalid three-word combinations"
echo "   ✅ Voice-friendly: Can be spoken over phone/voice chat"
echo "   ✅ Deterministic: Same address always gives same words"