# P2P Foundation - E2E Testing Quick Start

## For You (Running Multiple Nodes)

### Option 1: Using the Test Suite

```bash
# Terminal 1 - Run a full local network test
./bin/saorsa-test-suite test all --verbose

# This will:
# - Start P2P nodes locally
# - Run all test scenarios
# - Verify data integrity across nodes
```

### Option 2: Manual Node Setup

```bash
# Terminal 1 - Bootstrap node
./p2p-quick-dist/bin/p2p-chat

# Note your three-word address (e.g., "ocean.swift.mountain")

# Terminal 2 - Second node
./p2p-quick-dist/bin/p2p-chat --bootstrap-words ocean.swift.mountain

# Terminal 3 - Third node  
./p2p-quick-dist/bin/p2p-chat --bootstrap-words ocean.swift.mountain
```

## For Your Friend

### Step 1: Send the Package

Send your friend the file: `p2p-quick-dist-[timestamp].tar.gz`

### Step 2: Friend Extracts

```bash
tar -xzf p2p-quick-dist-*.tar.gz
cd p2p-quick-dist
```

### Step 3: Friend Connects

```bash
# Easy way - using the script
./connect-to-friend.sh
# Enter your three-word address when prompted

# Or manually
./bin/p2p-chat --bootstrap-words YOUR.THREE.WORDS
```

## Running Extensive CLI Tests

### Full Test Suite
```bash
# Run all tests with detailed output
./bin/saorsa-test-suite test all --verbose

# Run specific test categories
./bin/saorsa-test-suite test network --verbose
./bin/saorsa-test-suite test identity --verbose
./bin/saorsa-test-suite test chat --verbose
./bin/saorsa-test-suite test storage --verbose
./bin/saorsa-test-suite test crypto --verbose

# Run with data verification
./bin/saorsa-test-suite test all --verify-all --cross-node
```

### Stress Testing
```bash
# Run stress tests
./bin/saorsa-test-suite stress --verbose

# Multiple iterations
./bin/saorsa-test-suite test all --iterations 10
```

### Monitoring Tests
```bash
# Monitor live test execution
./bin/saorsa-test-suite monitor

# Generate test report
./bin/saorsa-test-suite report

# Audit data consistency
./bin/saorsa-test-suite audit
```

## Example Test Session

```bash
# Start your bootstrap node
$ ./bin/p2p-chat
🐜 P2P Foundation - Simple Chat
================================

🎯 Your three-word address: ocean.swift.mountain
   Share this with friends so they can connect!

💬 Chat commands:
   /help   - Show help
   /peers  - List connected peers
   /quit   - Exit chat

> 

# Your friend connects
$ ./bin/p2p-chat --bootstrap-words ocean.swift.mountain
🐜 P2P Foundation - Simple Chat
================================

🔗 Connecting to: ocean.swift.mountain
   (In a real implementation, this would connect to the P2P network)

💬 Chat commands:
   /help   - Show help
   /peers  - List connected peers  
   /quit   - Exit chat

> Hello from your friend!
[You]: Hello from your friend!
(Message would be sent to all connected peers)
```

## Tips

1. **Port Issues**: If you get "address in use" errors, the test suite automatically finds available ports

2. **Three-Word Addresses**: These are human-friendly addresses like "ocean.swift.mountain" instead of complex technical addresses

3. **Multiple Nodes**: You can run many nodes on one computer by using different ports (the test suite handles this automatically)

4. **Verbose Mode**: Always use `--verbose` flag to see detailed information about what's happening

5. **Help**: Run `./bin/saorsa-test-suite --help` to see all available options

## What The Tests Do

- **Connectivity**: Tests basic P2P connections and message passing
- **Identity**: Tests the identity system and three-word address resolution  
- **Chat**: Tests multi-party chat functionality
- **DHT**: Tests distributed hash table operations
- **Stress**: Tests system under heavy load
- **Stability**: Long-running tests for reliability

Happy testing! 🐜🔗