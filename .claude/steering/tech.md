# Technology Stack & Standards

## Languages & Runtimes

### Primary Language
- **Rust**: 2024 Edition
  - Minimum version: 1.75+
  - Key features used: async/await, const generics, GATs
  - Target platforms: Linux, macOS, Windows, iOS, Android, WASM

### Secondary Languages
- **TypeScript/JavaScript**: Frontend development
- **Python**: Testing scripts and tools
- **Shell**: Build and deployment scripts

## Core Dependencies

### Async Runtime
- **tokio**: v1.35+ with full features
  - Multi-threaded runtime
  - Async I/O, timers, sync primitives
  - Used throughout for all async operations

### Networking
- **quinn**: QUIC protocol implementation
- **ant-quic**: v0.4.4 - Custom QUIC extensions
- **libp2p**: P2P networking primitives
- **tokio-tungstenite**: WebSocket support

### Cryptography
- **ed25519-dalek**: v2.0 - Digital signatures
- **x25519-dalek**: v2.0 - Key exchange
- **sha2**: SHA-256 hashing
- **blake3**: Fast cryptographic hashing
- **ml-kem**: Quantum-resistant key encapsulation
- **ml-dsa**: Quantum-resistant signatures

### Serialization
- **serde**: v1.0 with derive
- **serde_json**: JSON support
- **bincode**: Binary serialization
- **prost**: Protocol buffers

### Storage
- **sqlx**: v0.7 - Async SQL (SQLite)
- **rocksdb**: Key-value storage (planned)

### Application Framework
- **tauri**: v2.0 - Cross-platform apps
  - Desktop: Native webview
  - Mobile: iOS/Android support
  - Web: WASM compilation

## Code Standards

### Rust Configuration

#### Workspace Cargo.toml
```toml
[workspace.package]
version = "0.2.6"
edition = "2024"
authors = ["..."]
license = "AGPL-3.0-or-later"

[workspace.dependencies]
# Shared dependencies with unified versions
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
# ... etc
```

#### Linting (clippy)
```rust
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]
```

#### Formatting
- `rustfmt` with default configuration
- Line width: 100 characters
- Imports: Grouped and sorted

### Error Handling
- **anyhow**: Application-level errors
- **thiserror**: Library-level errors
- Pattern: Custom error types for each module

```rust
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}
```

### Testing Strategy

#### Unit Tests
- Located in same file as code
- Minimum 80% code coverage target
- Use `#[tokio::test]` for async tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_feature() {
        // Arrange, Act, Assert pattern
    }
}
```

#### Integration Tests
- Separate `tests/` directory
- Multi-node test scenarios
- Performance benchmarks with Criterion

#### Test Utilities
- **mockall**: Mocking framework
- **proptest**: Property-based testing
- **criterion**: Benchmarking

### Documentation Standards

#### Code Documentation
```rust
/// Brief description of the struct.
///
/// Longer explanation with details about usage,
/// invariants, and examples.
///
/// # Examples
///
/// ```
/// use crate::MyStruct;
/// 
/// let instance = MyStruct::new();
/// assert!(instance.is_valid());
/// ```
pub struct MyStruct {
    /// Field documentation
    field: Type,
}
```

#### Module Documentation
```rust
//! Module-level documentation explaining purpose
//! and providing overview of contents.
//!
//! # Organization
//! 
//! - `submodule1` - Description
//! - `submodule2` - Description
```

### Performance Standards

#### Benchmarks
- All critical paths must have benchmarks
- Performance regression tests in CI
- Target metrics documented

```rust
#[bench]
fn bench_critical_operation(b: &mut Bencher) {
    b.iter(|| {
        // Operation to benchmark
    });
}
```

#### Optimization Guidelines
- Profile before optimizing
- Document performance-critical sections
- Prefer zero-copy operations
- Use `Arc` for shared immutable data

### Security Standards

#### Secure Coding
- No `unsafe` without justification
- All inputs validated
- Secrets never logged
- Crypto from audited libraries only

#### Dependency Management
- Regular security audits with `cargo audit`
- Minimal dependency principle
- Version pinning for security-critical deps

### Build & Deployment

#### CI/CD Pipeline
- **GitHub Actions** for all workflows
- Tests run on every PR
- Security scanning automated
- Release builds signed

#### Build Optimization
```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
```

### Monitoring & Observability

#### Logging
- **tracing**: Structured logging
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Context propagation with spans

```rust
#[instrument(skip(large_data))]
async fn process_data(id: u64, large_data: &[u8]) -> Result<()> {
    info!("Processing data");
    // ...
}
```

#### Metrics
- **prometheus**: Metrics collection
- Exposed on configurable port
- Key metrics: latency, throughput, errors

### Development Tools

#### Required Tools
- `rustup` - Rust toolchain manager
- `cargo` - Build system
- `rustfmt` - Code formatter
- `clippy` - Linter
- `cargo-edit` - Dependency management
- `cargo-watch` - File watcher

#### Recommended Tools
- `cargo-expand` - Macro expansion
- `cargo-flamegraph` - Performance profiling
- `cargo-bloat` - Binary size analysis
- `tokio-console` - Async runtime debugging

### Version Control

#### Git Workflow
- Feature branches from `main`
- Conventional commits
- Squash merge for features
- Linear history preferred

#### Commit Standards
```
type(scope): subject

Body explaining what and why

Footer with breaking changes or issues closed
```

Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore

### API Versioning

- Semantic versioning (SemVer)
- Breaking changes documented
- Deprecation warnings for 2 versions
- Migration guides provided