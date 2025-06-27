# Saorsa Installation Guide

## 🕊️ Saorsa Desktop App

Saorsa is a privacy-first P2P messaging application built with the Ant Core networking library.

### ✨ Features
- **Decentralized Messaging**: No central servers required
- **Privacy First**: Encrypted profiles with friend-based access control  
- **Human-Readable Addresses**: Share `forest.lightning.compass` instead of complex network addresses
- **Cross-Platform**: Native desktop performance with modern web UI
- **AI Integration**: Built-in MCP (Model Context Protocol) support

## 🍎 macOS Installation

### Quick Install
1. Download: [`Saorsa_0.1.8_aarch64.dmg`](desktop/macos/Saorsa_0.1.8_aarch64.dmg)
2. Double-click the DMG file to mount
3. Drag `Saorsa.app` to your Applications folder
4. Launch from Applications or Spotlight search

### System Requirements
- **macOS**: 11.0 (Big Sur) or later
- **Architecture**: Apple Silicon (M1/M2/M3) recommended
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 100MB available space
- **Network**: IPv6 capable connection preferred

### Security Notes
- The app is code-signed but may show a security warning on first launch
- If blocked: Right-click → Open, or System Settings → Privacy & Security → Allow
- No admin privileges required for installation

## 🔧 Build from Source

For developers or users who prefer building from source:

```bash
# Prerequisites
- Rust 1.75+
- Node.js 18+
- Xcode Command Line Tools (macOS)

# Clone and build
git clone https://github.com/dirvine/p2p.git
cd p2p/apps/desktop-tauri
npm install
cargo tauri build

# Find built app at:
# target/release/bundle/macos/Saorsa.app
```

## 🚀 Getting Started

### First Launch
1. **Create Identity**: Choose a display name and three-word address
2. **Generate Profile**: App creates encrypted profile automatically
3. **Find Friends**: Use three-word addresses or QR codes to connect
4. **Start Messaging**: Send encrypted messages via P2P network

### Basic Usage
```
🏠 Home Tab        - Recent conversations and activity
👤 Profile Tab     - Manage your identity and privacy settings  
📋 Contacts Tab    - Find and manage friends
⚙️  Settings Tab   - Configure network and app preferences
```

### Three-Word Addresses
- **Your Address**: Displayed prominently in profile (e.g., `forest.lightning.compass`)
- **Connect to Friends**: Enter their three-word address to send friend requests
- **QR Sharing**: Generate QR codes for easy address sharing

### Privacy Controls
- **Default Private**: All profile information encrypted by default
- **Friend Sharing**: Choose what information friends can see
- **Access Levels**: Granular control over profile visibility
- **No Central Servers**: Your data stays on your device and trusted friends

## 🌐 Network Configuration

### Automatic Setup
- App automatically detects network capabilities
- Configures IPv6 tunneling as needed
- Discovers and connects to bootstrap peers

### Manual Configuration
If you experience connection issues:

1. **Check Firewall**: Allow Saorsa through firewall
2. **IPv6 Support**: Ensure IPv6 is enabled on your network
3. **Port Ranges**: App uses dynamic ports (9000-9999 range)
4. **Bootstrap Peers**: App includes default bootstrap nodes

### Advanced Settings
```
Network Settings → Advanced:
- Custom bootstrap peers
- IPv4/IPv6 preferences  
- Connection timeouts
- Debug logging
```

## 🐛 Troubleshooting

### Common Issues

**App won't start**
- Check system requirements
- Verify app is in Applications folder
- Try launching from Terminal for error messages

**Can't connect to network**
- Check internet connection
- Verify firewall settings
- Try different network (cellular hotspot, etc.)

**Friends not appearing**
- Verify three-word addresses are correct
- Check both users are online
- Wait for DHT propagation (up to 30 seconds)

**Performance issues**
- Close other resource-intensive apps
- Check available RAM and CPU
- Monitor network usage in Activity Monitor

### Debug Information
- **Log Location**: `~/Library/Application Support/com.saorsa.app/logs/`
- **Config Location**: `~/Library/Application Support/com.saorsa.app/config/`
- **Enable Debug Logging**: Settings → Advanced → Debug Mode

### Getting Help
- 🐛 **Bug Reports**: [GitHub Issues](https://github.com/dirvine/p2p/issues)
- 💬 **Community**: [Discord](#) (coming soon)
- 📧 **Contact**: [Email](#) (coming soon)

## 🔄 Updates

### Automatic Updates
- App checks for updates on startup
- Notifications for new versions
- Secure update delivery via P2P network

### Manual Updates
1. Download latest DMG from releases page
2. Follow same installation process
3. App data and friends list preserved

---

## 🎯 What's Next?

Saorsa is actively developed with planned features:
- **Mobile Apps**: iOS and Android versions
- **File Sharing**: Secure P2P file transfer
- **Voice/Video**: Encrypted calls via P2P
- **Groups**: Private group messaging
- **Plugins**: Extensible functionality

Join our community to stay updated on development!