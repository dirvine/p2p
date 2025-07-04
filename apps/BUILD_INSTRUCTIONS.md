# Building Saorsa Terminal Apps

## Quick Start

From the project root directory:
```bash
cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester
```

Or from the apps directory:
```bash
cd apps
make
```

## Prerequisites

- Rust toolchain (install from https://rustup.rs)
- macOS 10.12 or later
- Cargo in your PATH

## Build Commands

### Using Cargo (from project root)

```bash
# Build both apps in release mode
cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester

# Build just the chat app
cargo build --release -p saorsa-terminal-chat

# Build just the network tester
cargo build --release -p saorsa-network-tester

# Build in debug mode (faster compile, larger binary)
cargo build -p saorsa-terminal-chat -p saorsa-network-tester
```

### Using Make (from apps directory)

```bash
cd apps

# Build everything
make

# Build specific app
make chat
make tester

# Build and run
make run-chat
make run-tester

# Create macOS apps
make mac-apps
```

### Using Build Script

```bash
cd apps
./build_terminal_apps.sh
```

## Output

Built binaries will be in:
- `target/release/saorsa-terminal-chat`
- `target/release/saorsa-network-tester`

These are statically linked and can be copied anywhere.

## Creating macOS Apps

After building, create `.app` bundles:
```bash
cd apps
./create_macos_apps.sh
```

This creates:
- `Saorsa Terminal Chat.app`
- `Saorsa Network Tester.app`

## Optimization

The release builds use:
- `opt-level = "z"` - Optimize for small binary size
- `lto = true` - Link-time optimization
- `strip = true` - Strip debug symbols
- `codegen-units = 1` - Better optimization

Typical binary sizes:
- Chat app: ~2-3 MB
- Network tester: ~2-3 MB

## Troubleshooting

### Cargo not found
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH
source $HOME/.cargo/env
```

### Build fails
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release -p saorsa-terminal-chat
```

### Permission denied
```bash
# Make scripts executable
chmod +x build_terminal_apps.sh create_macos_apps.sh
```

## Distribution

The release binaries are self-contained and can be:
1. Copied directly to any macOS system
2. Packaged in a `.zip` file
3. Distributed as `.app` bundles
4. Signed and notarized (for App Store)

No runtime dependencies required!