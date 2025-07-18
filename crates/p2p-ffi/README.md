# P2P Foundation FFI Bindings (DEPRECATED)

⚠️ **This crate is deprecated and no longer maintained.**

## Migration Notice

The P2P Foundation has moved to a unified **Tauri-based cross-platform architecture**. Flutter integration is no longer supported.

### Current Development

For current development, please use:

- **`apps/saorsa/`** - Tauri desktop application for cross-platform GUI
- **`crates/p2p-core/`** - Core P2P library for direct Rust integration
- **`apps/saorsa-terminal-chat/`** - Terminal-based chat application
- **`apps/saorsa-network-tester/`** - Network testing utilities

### Why This Change?

- **Unified Architecture**: Tauri provides a single framework for desktop, mobile, and web
- **Simplified Maintenance**: No need to maintain separate FFI bindings
- **Better Performance**: Direct Rust integration without FFI overhead
- **Modern Development**: Leverages web technologies with Rust backend

### Legacy Information

This crate previously provided C-compatible FFI bindings for Flutter/Dart integration, including:

- Cross-language API integration
- Async operation support via callbacks
- Memory-safe string and data handling
- Thread-safe mobile application support
- Platform-specific iOS/Android optimizations

**This functionality is now superseded by Tauri's native cross-platform capabilities.**