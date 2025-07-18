# Project Structure

## Workspace Organization

This is a Cargo workspace with multiple interconnected components organized into logical groups.

## Top-Level Structure

```
├── crates/           # Core Rust libraries
├── apps/            # Applications and executables
├── src/             # Legacy source (being migrated to crates/)
├── tests/           # Integration and end-to-end tests
├── examples/        # Code examples and demos
├── docs/            # Documentation and specifications
├── benches/         # Performance benchmarks
└── scripts/         # Build and utility scripts
```

## Core Libraries (`crates/`)

### `crates/p2p-core/`
- **Purpose**: Main P2P networking library (published as `saorsa-core`)
- **Key modules**: network, dht, transport, identity, security, mcp
- **Architecture**: Modular design with feature flags
- **Dependencies**: Core networking and crypto dependencies

### `crates/p2p-ffi/` (Deprecated)
- **Status**: Deprecated - no longer used with Tauri-based mobile approach
- **Legacy Purpose**: Previously provided FFI bindings for Flutter integration
- **Migration**: Functionality moved to direct Rust integration via Tauri

### `crates/p2p-cli/`
- **Purpose**: Command-line utilities and tools
- **Binaries**: Network management and debugging tools

### `crates/ant-test-suite/`
- **Purpose**: Comprehensive test suite with data integrity verification
- **Features**: Multi-node testing, stress tests, integration validation
- **Structure**: Organized by subsystem (network, identity, crypto, storage)

## Applications (`apps/`)

### `apps/saorsa/`
- **Type**: Tauri cross-platform application (desktop, mobile, web)
- **Structure**: 
  - `src/` - HTML/JS frontend
  - `src-tauri/` - Rust backend
- **Purpose**: Main P2P chat and communication app

### `apps/saorsa-terminal-chat/`
- **Type**: Terminal-based chat application
- **Purpose**: CLI chat client for testing and development

### `apps/saorsa-network-tester/`
- **Type**: Network testing utility
- **Purpose**: DHT operations, connectivity testing

## Source Organization Patterns

### Module Structure
```rust
// Standard module organization
pub mod network;     // Core networking
pub mod dht;         // Distributed hash table
pub mod transport;   // QUIC/TCP transport
pub mod identity;    // User identity system
pub mod security;    // Cryptography and security
pub mod mcp;         // Model Context Protocol
```

### Feature Organization
- **Core features**: Always compiled
- **Optional features**: Behind feature flags
- **Platform-specific**: Conditional compilation

## Testing Structure (`tests/`)

### Integration Tests
- **File naming**: `*_integration_tests.rs`
- **Purpose**: Cross-component testing
- **Scope**: Real P2P operations with multiple nodes

### End-to-End Tests
- **Directory**: `tests/e2e_full_network/`
- **Structure**: Infrastructure, scenarios, stress tests
- **Purpose**: Full system validation

### Specialized Tests
- **Security**: `security_*_tests.rs`
- **Performance**: `stress_tests.rs`, `realistic_workload_tests.rs`
- **Protocol**: `mcp_*_tests.rs`, `dht_*_tests.rs`

## Documentation (`docs/`)

- **Specifications**: Technical design documents
- **Architecture**: System design and patterns
- **Security**: Cryptographic analysis and threat models
- **Licensing**: Compliance and commercial guidance

## Build Scripts and Automation

### Root Level Scripts
- `BUILD_NOW.sh` - Quick terminal apps build
- `build-terminal-apps.sh` - Comprehensive build
- `create_final_distribution.sh` - Release packaging

### App-Specific Scripts
- `apps/create_macos_apps.sh` - macOS app bundle creation
- `apps/BUILD_AND_PACKAGE.sh` - Cross-platform packaging

## Naming Conventions

### Crates
- `p2p-*` prefix for core libraries
- `saorsa-*` prefix for applications
- `ant-*` prefix for specialized tools

### Files
- `snake_case` for Rust files
- `kebab-case` for configuration files
- `SCREAMING_SNAKE_CASE` for documentation

### Modules
- Descriptive names reflecting functionality
- Avoid deep nesting (max 3 levels)
- Group related functionality together

## Dependencies Management

### Workspace Dependencies
- Shared versions in `Cargo.toml` `[workspace.dependencies]`
- Consistent versioning across all crates
- Feature flags for optional functionality

### External Dependencies
- Prefer established, well-maintained crates
- Pin versions for security-critical dependencies
- Document rationale for major dependencies