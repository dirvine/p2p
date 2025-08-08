# Saorsa - P2P Desktop Messaging App

Saorsa is a revolutionary peer-to-peer desktop messaging application with built-in privacy features and AI-powered invisible cryptocurrency management.

## Installation

```bash
cargo install saorsa
```

## Features

- 🌐 **Decentralized P2P Network**: No central servers, your data stays yours
- 🔐 **End-to-End Encryption**: All messages are encrypted by default
- 🏠 **Three-Word Addresses**: Easy-to-remember human-friendly addresses
- 🤖 **AI Integration**: Built-in AI assistance powered by MCP
- 💰 **Invisible Crypto**: Manage cryptocurrency without complexity
- 🖥️ **Native Desktop App**: Built with Tauri for optimal performance
- 🌍 **IPv6 First**: Modern networking with automatic tunneling

## Running

After installation, simply run:

```bash
saorsa
```

The app will:
1. Extract frontend assets to `~/.saorsa/frontend/` on first run
2. Launch the native desktop application
3. Connect to the P2P network automatically

## System Requirements

- **Operating Systems**: macOS, Windows, Linux
- **Memory**: 512MB RAM minimum
- **Storage**: 100MB free space
- **Network**: Internet connection (IPv4 or IPv6)

## First Run

On first launch, Saorsa will:
1. Generate a unique cryptographic identity
2. Create your three-word address
3. Connect to bootstrap nodes
4. Set up local storage

## Configuration

Configuration files are stored in:
- macOS/Linux: `~/.saorsa/`
- Windows: `%APPDATA%\saorsa\`

## Building from Source

If you prefer to build from source:

```bash
git clone https://github.com/dirvine/p2p.git
cd p2p/apps/desktop-tauri/src-tauri
cargo build --release
```

## Troubleshooting

### App doesn't start
- Check logs in `~/.saorsa/logs/`
- Ensure port 9000 is available
- Try running with debug: `RUST_LOG=debug saorsa`

### Connection issues
- Verify internet connectivity
- Check firewall settings
- Try different bootstrap nodes

## License

Licensed under either MIT or Apache-2.0 at your option.

## Contributing

Contributions welcome! Please see the main repository at https://github.com/dirvine/p2p