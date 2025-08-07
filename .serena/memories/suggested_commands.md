# Essential Development Commands

## Build Commands

### Core Library
```bash
# Build entire workspace
cargo build --release

# Build for development (faster)
cargo build

# Clean build artifacts
cargo clean
```

### Desktop Application (Saorsa)
```bash
cd apps/saorsa

# Development mode with hot reload
cargo tauri dev

# Build for testing
cargo tauri build

# Build for specific platforms
cargo tauri build --target x86_64-apple-darwin  # macOS
cargo tauri build --target x86_64-pc-windows-msvc  # Windows
cargo tauri build --target x86_64-unknown-linux-gnu  # Linux
```

## Testing Commands

### Core Tests
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test modules
cargo test network_tests
cargo test identity_tests
cargo test crypto_tests
cargo test storage_tests

# Run integration tests
cargo test --test dht_network_integration_test
cargo test --test mcp_service_discovery_tests

# Run comprehensive test suite
cd crates/ant-test-suite
cargo test
```

### Performance Testing
```bash
# Run benchmarks
cargo bench --all-features

# Test with different node counts
NODES=50 cargo test network_tests

# Enable debug logging
RUST_LOG=debug cargo test -- --nocapture
```

## Quality Assurance

### Code Formatting
```bash
# Format all code
cargo fmt

# Check formatting without changing
cargo fmt --check
```

### Linting
```bash
# Run clippy with all features
cargo clippy --all-features -- -D warnings

# Run clippy on workspace
cargo clippy --workspace -- -D warnings
```

### Documentation
```bash
# Generate documentation
cargo doc --no-deps --open

# Test documentation examples
cargo test --doc

# Check for missing documentation
RUSTFLAGS="-D missing_docs" cargo check --lib
```

## Debugging and Development

### Environment Variables
```bash
# Enable backtraces
export RUST_BACKTRACE=1

# Enable debug logging
export RUST_LOG=debug

# Include current directory in Python path
export PYTHONPATH=.
```

### Multi-Node Testing
```bash
# Terminal 1: Bootstrap node
cargo run --bin saorsa -- --port 9001 --bootstrap-file bootstrap.json

# Terminal 2: Additional node
cargo run --bin saorsa -- --port 9002 --bootstrap /ip6/::1/tcp/9001

# Terminal 3: Desktop app
cd apps/saorsa && cargo tauri dev
```

## Utility Commands (macOS)

### File Operations
```bash
# List files (basic)
ls -la

# Find files by pattern
find . -name "*.rs" -type f

# Search content (use ripgrep if available)
rg "pattern" --type rust
grep -r "pattern" src/
```

### Process Management
```bash
# Show running processes
ps aux | grep cargo

# Kill process by name
pkill -f "cargo tauri"

# Monitor system resources
top -o cpu
```

### Git Operations
```bash
# Standard git workflow
git status
git add .
git commit -m "message"
git push

# View commit history
git log --oneline

# Create and switch branch  
git checkout -b feature-name
```

## Quick Diagnostics
```bash
# Check Rust version
rustc --version

# Check Cargo version
cargo --version

# Check Node.js version (for Tauri)
node --version

# Verify workspace structure
cargo metadata --format-version 1 | jq '.workspace_members'
```