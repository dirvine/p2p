# Saorsa Terminal Applications

## Overview

Two standalone terminal applications for the P2P Foundation network:

1. **saorsa-terminal-chat** - Real P2P chat with automatic IPv6/IPv4 detection
2. **saorsa-network-tester** - Network testing tool with port conflict handling

## Features

### Saorsa Terminal Chat
- **Real P2P connections** - No fake users or simulated responses
- **IPv6 Direct Support** - Uses IPv6 directly when available, no unnecessary tunneling
- **Smart Fallback** - Only mentions tunneling when IPv6 isn't available
- **Port Conflict Handling** - Automatically finds available ports
- **Three-word Addresses** - Human-friendly addressing system
- **Multiple Peers** - Support for group chats

### Saorsa Network Tester
- **Port Conflict Handling** - Treats busy ports as normal, not errors
- **Dynamic Port Allocation** - Always finds an available port
- **Real Network Testing** - No simulations
- **IPv6 Detection** - Shows when direct IPv6 is available
- **Tunnel Testing** - Tests IPv4→IPv6 tunnel connectivity back to your listener
- **Comprehensive Tests** - Network capabilities, ports, connectivity, tunnels

## Building

### Prerequisites
- Rust toolchain (cargo)
- macOS 10.12 or later

### Build Commands
```bash
# From the project root
cargo build --release -p saorsa-terminal-chat
cargo build --release -p saorsa-network-tester

# Or use the build script
cd apps
./build_terminal_apps.sh
```

### Creating macOS Apps
```bash
cd apps
./create_macos_apps.sh
```

This creates `.app` bundles that can be double-clicked.

## Usage

### Terminal Chat

**Host a chat room:**
```bash
./saorsa-terminal-chat
# Choose option 1
# Note the port number shown
# Share port with friends
```

**Join a chat room:**
```bash
./saorsa-terminal-chat
# Choose option 2
# Enter friend's port number
```

### Network Tester
```bash
./saorsa-network-tester
# Choose option 1 for quick test
# Choose option 2 for port scan
# Choose option 3 for tunnel testing
```

The tunnel test (option 3) will:
1. Start an IPv6 listener
2. Test IPv4-mapped IPv6 addresses (::ffff:x.x.x.x)
3. Check Teredo tunnel capability
4. Check 6to4 tunnel capability
5. Test if IPv4 clients can connect to your IPv6 server

## Architecture

Both apps are written in Rust using:
- **tokio** - Async runtime
- **colored** - Terminal colors
- **Native networking** - Direct TCP/UDP sockets

### Static Compilation
The apps are compiled with:
- `opt-level = "z"` - Optimize for size
- `lto = true` - Link-time optimization
- `strip = true` - Strip debug symbols
- `codegen-units = 1` - Single codegen unit

This produces small, fast binaries that work standalone.

## Distribution

The compiled binaries are completely self-contained:
- No runtime dependencies
- No need for Rust installation
- Work on any compatible macOS system

Just share the binary or `.app` bundle!

## Technical Details

### Network Detection Logic
```rust
if has_ipv6 {
    // Use IPv6 directly - no tunnel needed
    TcpListener::bind("[::]:port")
} else {
    // Fall back to IPv4
    TcpListener::bind("0.0.0.0:port")
    // Only now mention tunneling would be used in production
}
```

### Tunnel Testing
The network tester can verify IPv4→IPv6 tunnel connectivity:

1. **IPv4-mapped IPv6** (::ffff:127.0.0.1)
   - Tests if IPv4 clients can reach IPv6 servers
   - Works on dual-stack systems
   - No configuration needed

2. **Teredo** (2001:0::/32)
   - UDP-based IPv6 tunneling
   - Works through NAT
   - Built into Windows

3. **6to4** (2002::/16)
   - Uses public IPv4 in IPv6 prefix
   - Automatic tunneling
   - Requires public IPv4

4. **Local Testing**
   - Verifies dual-stack support
   - Tests actual connectivity
   - Shows what works on your system

### Port Allocation
```rust
// Try preferred ports
for port in 9000..9020 {
    if let Ok(listener) = TcpListener::bind(port) {
        return Ok(port);
    }
}
// Fall back to OS allocation
TcpListener::bind("0.0.0.0:0")
```

## Files

```
apps/
├── saorsa-terminal-chat/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── saorsa-network-tester/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── build_terminal_apps.sh
├── create_macos_apps.sh
└── TERMINAL_APPS_README.md
```