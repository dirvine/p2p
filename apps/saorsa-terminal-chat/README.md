# Saorsa Terminal Chat

A real P2P chat application using the Saorsa Core library with QUIC transport, DHT, and quantum-resistant cryptography.

## Features

- **Real P2P Networking**: Uses QUIC transport with automatic IPv6/IPv4 tunneling
- **Distributed Hash Table (DHT)**: For decentralized peer discovery and data storage
- **Quantum-Resistant Crypto**: ML-KEM/ML-DSA cryptographic foundation
- **MCP Integration**: AI-native capabilities through Model Context Protocol
- **Simple Terminal UI**: Easy-to-use command-line interface

## Building

From the project root:

```bash
cargo build --release -p saorsa-terminal-chat
```

Or use the build script:

```bash
./build-terminal-apps.sh
```

## Usage

### Creating a Chat Room

1. Run the application:
   ```bash
   ./target/release/saorsa-terminal-chat
   ```

2. Choose option 1 to create a new chat room

3. Share your peer ID with friends who want to join

### Joining a Chat Room

1. Run the application:
   ```bash
   ./target/release/saorsa-terminal-chat
   ```

2. Choose option 2 to join an existing chat room

3. Enter your friend's multiaddress (e.g., `/ip4/127.0.0.1/tcp/9000`)

## Chat Commands

- `/help` - Show available commands
- `/peers` - List connected peers
- `/info` - Show network information
- `/quit` - Exit the chat

## Network Architecture

The chat uses the full Saorsa P2P stack:

- **Transport Layer**: QUIC preferred, TCP fallback
- **Discovery**: DHT-based peer discovery
- **Security**: Quantum-resistant encryption for all messages
- **Protocol**: Custom `/chat/1.0.0` protocol for messages

## Technical Details

- Built with Rust and Tokio async runtime
- Uses broadcast channels for event handling
- Supports concurrent message handling
- Production-hardened with connection pooling and rate limiting