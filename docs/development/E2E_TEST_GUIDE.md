# P2P Foundation - E2E Testing & Binary Distribution Guide

## Quick Start - Binary for Your Friend

### 1. Build the Chat Binary

```bash
# Build the chat example in release mode
cargo build --release --example chat

# The binary will be at:
# ./target/release/examples/chat
```

### 2. Share Your Three-Word Address

First, start your node to get your three-word address:

```bash
# Start with auto-discovery (finds bootstrap nodes automatically)
./target/release/examples/chat

# Or start with a specific bootstrap
./target/release/examples/chat --bootstrap-words global.fast.eagle
```

Your node will display something like:
```
[System] 🎯 Your three-word address: ocean.swift.mountain
[System] 📍 Listening on: /ip6/::1/udp/9000/quic
```

### 3. Give Your Friend Simple Instructions

Send your friend:
1. The chat binary
2. Your three-word address (e.g., `ocean.swift.mountain`)

They just need to run:
```bash
./chat --bootstrap-words ocean.swift.mountain
```

That's it! They'll connect to your node and you can start chatting.

## Running Multiple Nodes on Your Computer

### Setup Multiple Test Nodes

```bash
# Terminal 1 - Bootstrap node
./target/release/examples/chat --listen-address "/ip6/::/udp/9001/quic"

# Terminal 2 - Node 2 connecting to bootstrap
./target/release/examples/chat --listen-address "/ip6/::/udp/9002/quic" \
  --bootstrap "/ip6/::1/udp/9001/quic"

# Terminal 3 - Node 3 using three-word address
./target/release/examples/chat --listen-address "/ip6/::/udp/9003/quic" \
  --bootstrap-words YOUR.THREE.WORDS

# Terminal 4 - Node 4 with auto-discovery
./target/release/examples/chat --listen-address "/ip6/::/udp/9004/quic"
```

## E2E Test Suite

### Build the Test Binary

```bash
# Build the comprehensive test suite
cargo build --release --bin saorsa-test-suite

# The binary will be at:
# ./target/release/saorsa-test-suite
```

### Run Different Test Scenarios

```bash
# Quick network status check
./target/release/saorsa-test-suite --status

# List available bootstrap nodes
./target/release/saorsa-test-suite --bootstrap-nodes

# Full connectivity test
./target/release/saorsa-test-suite --test-connectivity

# Run all e2e tests with 5 local nodes
./target/release/saorsa-test-suite all --local-nodes 5

# Run specific test suites
./target/release/saorsa-test-suite identity    # Identity tests
./target/release/saorsa-test-suite chat        # Chat tests
./target/release/saorsa-test-suite projects    # Project management tests
./target/release/saorsa-test-suite threshold   # Threshold signature tests
./target/release/saorsa-test-suite tunneling   # IPv6 tunneling tests
./target/release/saorsa-test-suite mcp         # MCP integration tests

# Run stress tests with 100 nodes
./target/release/saorsa-test-suite stress --max-nodes 100 --operations-per-node 1000
```

### Distributed Testing Across Multiple Machines

On your main machine (coordinator):
```bash
# Start the test coordinator
./target/release/saorsa-test-suite coordinator --bind-addr :: --port 9999
```

On other machines:
```bash
# Join the distributed test
./target/release/saorsa-test-suite remote \
  --coordinator "[YOUR_IPV6_ADDRESS]:9999" \
  --node-count 10 \
  --name "TestMachine1"
```

## CLI Binary (saorsa)

### Build and Install

```bash
# Build the CLI
cargo build --release --bin saorsa

# Or install it
cargo install --path crates/p2p-cli

# The binary will be at:
# ./target/release/saorsa
```

### Usage

```bash
# Quick network status
saorsa --status

# List bootstrap nodes
saorsa --bootstrap-nodes

# Full connectivity test
saorsa --test-connectivity
```

## Creating a Distribution Package

### For macOS/Linux:

```bash
#!/bin/bash
# create_p2p_package.sh

# Create distribution directory
mkdir -p p2p-distribution/{bin,docs}

# Copy binaries
cp target/release/examples/chat p2p-distribution/bin/p2p-chat
cp target/release/saorsa-test-suite p2p-distribution/bin/
cp target/release/saorsa p2p-distribution/bin/

# Create simple wrapper scripts
cat > p2p-distribution/start-chat.sh << 'EOF'
#!/bin/bash
echo "🐜 P2P Foundation Chat"
echo "====================="
echo ""
echo "Starting with auto-discovery..."
./bin/p2p-chat "$@"
EOF

cat > p2p-distribution/connect-to-friend.sh << 'EOF'
#!/bin/bash
echo "🔗 Connect to Friend's Node"
echo "=========================="
echo ""
read -p "Enter your friend's three-word address: " THREE_WORDS
./bin/p2p-chat --bootstrap-words "$THREE_WORDS"
EOF

chmod +x p2p-distribution/*.sh

# Create README
cat > p2p-distribution/README.txt << 'EOF'
P2P Foundation - Decentralized Chat
===================================

QUICK START:
1. Run ./start-chat.sh to start your node
2. Share your three-word address with friends
3. To connect to a friend, run ./connect-to-friend.sh

ADVANCED:
- Run specific tests: ./bin/saorsa-test-suite --help
- Network status: ./bin/saorsa --status

Your three-word address will be displayed when you start.
Share it with friends so they can connect to you!
EOF

# Create tarball
tar -czf p2p-distribution.tar.gz p2p-distribution/
echo "📦 Package created: p2p-distribution.tar.gz"
```

### For Windows:

Create a batch file `start-chat.bat`:
```batch
@echo off
echo P2P Foundation Chat
echo ===================
echo.
echo Starting with auto-discovery...
bin\p2p-chat.exe %*
pause
```

## Troubleshooting

### Common Issues:

1. **"No bootstrap nodes found"**
   - Solution: Use `--bootstrap-words global.fast.eagle` or another known bootstrap

2. **"Address already in use"**
   - Solution: Change port with `--listen-address "/ip6/::/udp/9005/quic"`

3. **IPv6 not available**
   - The system will automatically use IPv4 with tunneling

### Test Network Connectivity:

```bash
# Check if you can reach bootstrap nodes
./target/release/saorsa --test-connectivity

# This will show:
# - IPv6 support status
# - NAT type
# - Recommended tunnel type
# - Bootstrap connectivity
# - Response times
```

## Security Note

The chat example is for testing and demonstration. For production use:
- Messages are encrypted in transit
- Each node has a unique cryptographic identity
- Three-word addresses are derived from public keys
- All communication uses quantum-resistant cryptography

## Next Steps

1. Start your node and get your three-word address
2. Share the binary and your address with friends
3. Run the e2e test suite to verify everything works
4. Explore the other examples in the `examples/` directory

Happy decentralized chatting! 🐜🔗