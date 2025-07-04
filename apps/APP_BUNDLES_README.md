# macOS App Bundles for Saorsa Terminal Apps

## Overview

We've created a complete macOS app bundle solution for the Saorsa terminal applications, providing a double-click experience that automatically opens Terminal.

## Features

### 🎯 One-Click Launch
- Double-click the `.app` file
- Terminal opens automatically
- App starts running immediately
- Custom window title shows app name

### 📦 Self-Contained
- Binary embedded in app bundle
- No installation required
- No dependencies
- Portable - can run from anywhere

### 🎨 Professional Appearance
- Proper macOS app bundle structure
- Info.plist with all metadata
- Ready for code signing
- Can be dragged to Applications folder

## App Bundle Structure

```
Saorsa Terminal Chat.app/
├── Contents/
│   ├── Info.plist              # App metadata
│   ├── MacOS/
│   │   ├── launcher            # AppleScript launcher
│   │   └── saorsa-terminal-chat # Actual binary
│   └── Resources/
│       └── icon.txt            # Emoji icon (placeholder)
```

## How It Works

1. **User double-clicks** the .app bundle
2. **Launcher script** runs via Info.plist
3. **AppleScript** opens Terminal with a custom script
4. **Terminal** runs the actual binary
5. **Clean exit** - "Press any key to close"

## Building & Packaging

### Quick Build
```bash
cd apps
make mac-apps
```

### Full Distribution Package
```bash
cd apps
make package
```

This creates:
- `Saorsa-Apps-YYYYMMDD/` folder with both apps
- `Saorsa-Apps-YYYYMMDD.zip` ready to share
- Install/Uninstall helper scripts
- README for users

### Manual Steps
```bash
# Build binaries
cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester

# Create app bundles
./create_macos_apps.sh

# Package for distribution
./package_for_distribution.sh
```

## Security & Signing

### First Run Security Warning
macOS will show "unidentified developer" warning. Users can:
1. Right-click → Open
2. Or System Preferences → Security & Privacy → Open Anyway

### Code Signing (Optional)
```bash
# Sign the apps (requires Apple Developer certificate)
codesign --force --deep --sign "Developer ID Application: Your Name" "Saorsa Terminal Chat.app"
codesign --force --deep --sign "Developer ID Application: Your Name" "Saorsa Network Tester.app"

# Notarize for distribution (requires Apple Developer account)
xcrun altool --notarize-app --primary-bundle-id "com.saorsa.terminal-chat" ...
```

## Distribution

### For Friends
1. Run `make package`
2. Send them `Saorsa-Apps-YYYYMMDD.zip`
3. They unzip and double-click!

### For App Store
Would need:
- Developer certificate
- Code signing
- Notarization
- Sandboxing adjustments

## User Experience

### What Users See
1. **Double-click** app icon
2. **Terminal opens** with app name in title
3. **App runs** with colored output
4. **Clean exit** - "Press any key to close"

### What Users Don't See
- No command line needed
- No path configuration
- No terminal commands
- No installation process

## Customization

### Icons
Currently using emoji placeholders. For real icons:
1. Create 1024x1024 PNG artwork
2. Use Icon Set Creator (Mac App Store)
3. Generate .icns file
4. Replace in Resources folder

### Terminal Appearance
The launcher sets:
- Window title to app name
- Basic terminal theme
- Could customize colors/font if needed

## Files Created

```
apps/
├── create_macos_apps.sh        # Creates .app bundles
├── create_app_icons.sh         # Icon creation helper
├── package_for_distribution.sh # Creates distribution ZIP
├── Makefile                    # Updated with mac-apps target
└── APP_BUNDLES_README.md       # This file
```

## Benefits

1. **Professional** - Looks like a real macOS app
2. **User-Friendly** - Just double-click
3. **Shareable** - Self-contained bundle
4. **Maintainable** - Clean structure
5. **Extensible** - Ready for signing/notarization

The apps are now ready for distribution with a fantastic user experience! 🎉