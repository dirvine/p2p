# Assets Directory

This directory contains all the assets for the P2P Foundation project, including application binaries, icons, screenshots, and platform-specific resources.

## 📁 Directory Structure

```
assets/
├── desktop/           # Desktop application assets
│   ├── macos/        # macOS-specific assets
│   ├── windows/      # Windows-specific assets
│   └── linux/        # Linux-specific assets
├── icons/            # Application icons and branding
├── screenshots/      # Application screenshots
└── README.md        # This file
```

## 🍎 macOS Assets (`desktop/macos/`)

### Saorsa.app
- **Native macOS application bundle**
- **Architecture**: Apple Silicon (ARM64)
- **Requirements**: macOS 11.0 or later
- **Installation**: Double-click to run, or drag to Applications folder

### Saorsa_0.1.8_aarch64.dmg
- **Disk image installer for macOS**
- **Size**: ~10MB
- **Architecture**: Apple Silicon (ARM64)
- **Installation**: 
  1. Double-click the DMG file
  2. Drag Saorsa.app to Applications folder
  3. Launch from Applications or Spotlight

### Features
- ✅ Native macOS performance
- ✅ Retina display support
- ✅ macOS notifications integration
- ✅ Keychain integration for secure storage
- ✅ Universal clipboard support

## 🖼️ Icons (`icons/`)

Application icons in multiple formats for cross-platform compatibility:

### PNG Icons
- `32x32.png` - Standard small icon
- `128x128.png` - Standard large icon  
- `128x128@2x.png` - Retina display icon
- `icon.png` - Master icon file

### Platform-Specific Icons
- `icon.icns` - macOS icon bundle
- `icon.ico` - Windows icon file

### Windows Store Icons
- `Square30x30Logo.png` - Windows tile (small)
- `Square44x44Logo.png` - Windows tile (medium)
- `Square71x71Logo.png` - Windows tile (medium)
- `Square89x89Logo.png` - Windows tile (large)
- `Square107x107Logo.png` - Windows tile (large)
- `Square142x142Logo.png` - Windows tile (extra large)
- `Square150x150Logo.png` - Windows tile (extra large)
- `Square284x284Logo.png` - Windows tile (huge)
- `Square310x310Logo.png` - Windows tile (huge)
- `StoreLogo.png` - Windows Store logo

## 📸 Screenshots (`screenshots/`)

*Coming soon* - Application screenshots will be added here showcasing:
- Main chat interface
- Profile management
- Contact discovery
- Privacy settings
- Cross-platform compatibility

## 🚀 Installation Instructions

### macOS
1. Download `Saorsa_0.1.8_aarch64.dmg`
2. Double-click to mount the disk image
3. Drag `Saorsa.app` to your Applications folder
4. Launch from Applications or Spotlight search

### Windows (Coming Soon)
1. Download `Saorsa_0.1.8_x64.msi`
2. Run the installer
3. Follow installation wizard
4. Launch from Start Menu

### Linux (Coming Soon)
1. Download `Saorsa_0.1.8_x86_64.AppImage`
2. Make executable: `chmod +x Saorsa_0.1.8_x86_64.AppImage`
3. Run: `./Saorsa_0.1.8_x86_64.AppImage`

## 🔧 Building from Source

If you prefer to build the desktop application yourself:

```bash
# Clone the repository
git clone https://github.com/dirvine/p2p.git
cd p2p

# Install dependencies
cd apps/desktop-tauri
npm install

# Build for your platform
cargo tauri build

# Find built assets in target/release/bundle/
```

## 📋 Version Information

- **Current Version**: 0.1.8
- **Release Date**: December 2024
- **Supported Platforms**: 
  - ✅ macOS (Apple Silicon & Intel)
  - 🔄 Windows (Coming Soon)
  - 🔄 Linux (Coming Soon)

## 🔐 Security & Verification

### Code Signing
- **macOS**: App is code-signed for security
- **Windows**: Authenticode signing (coming soon)
- **Linux**: GPG signatures (coming soon)

### Checksums
```bash
# Verify DMG integrity (example)
shasum -a 256 Saorsa_0.1.8_aarch64.dmg
# Expected: [checksum will be added upon release]
```

## 🐛 Troubleshooting

### macOS
- **"App can't be opened"**: Right-click → Open, or go to System Settings → Privacy & Security
- **Performance issues**: Ensure you have at least 8GB RAM available
- **Network issues**: Check firewall settings allow P2P connections

### General Issues
- Check the [Issues page](https://github.com/dirvine/p2p/issues) for known problems
- Join our community for support
- Review logs in the application data directory

## 📱 Mobile Apps (Roadmap)

Future mobile applications will be available:
- **iOS**: App Store distribution
- **Android**: Google Play Store and APK downloads
- **Flutter Web**: Browser-based version

## 🤝 Contributing Assets

To contribute assets (icons, screenshots, etc.):

1. Fork the repository
2. Add assets to appropriate directory
3. Update this README with descriptions
4. Submit a pull request

### Asset Guidelines
- **Icons**: SVG preferred, minimum 512x512 PNG
- **Screenshots**: 16:10 aspect ratio, high DPI
- **Formats**: Cross-platform compatible formats
- **Size**: Optimize for reasonable file sizes

---

For more information, see the [main project README](../README.md).