# Technology Stack & Standards

**Last Updated**: 2025-08-03

## Languages & Runtimes

### Primary Language: Rust
- **Version**: 1.75+ (2024 edition) 
- **Async Runtime**: Tokio 1.35 with full features
- **Target Platforms**: x86_64, aarch64 (Linux, macOS, Windows)
- **Key Features**: async/await, const generics, GATs
- **Safety**: Zero-panic architecture enforced via clippy rules

### Frontend Technologies (Saorsa App)
- **Framework**: Tauri 2.x
- **UI**: HTML5/CSS3/JavaScript (vanilla)
- **Bundler**: Tauri's built-in bundling
- **Target Platforms**: Desktop (all OS), Mobile (iOS/Android), Web

## Core Dependencies

### Networking & Transport
- **QUIC Protocol**: Quinn 0.11 (pure Rust QUIC implementation)
  - Direct quinn usage (removed ant-quic abstraction)
  - Simplified transport architecture
  - NAT traversal built-in
- **TLS**: Rustls 0.23 (modern TLS 1.3)
  - 🚨 Critical: Empty certificate generation issue
- **Transport Strategy**: QUIC-only (consolidated in Task 3)
- **Async I/O**: Tokio with full feature set

### Cryptography
- **Signatures**: Ed25519-dalek 2.1 (v2 migration complete)
- **Key Exchange**: X25519-dalek 2.0
- **Hashing**: Blake3 1.5, SHA2 0.10
- **Encryption**: AES-GCM 0.10, ChaCha20Poly1305 0.10
- **Key Derivation**: Argon2 0.5, HKDF 0.12
- **Post-Quantum**: ML-KEM 0.2, ML-DSA 0.1.0-pre.2 (ready but not activated)
- **Threshold**: FROST-ed25519 2.0.0-rc.0, Shamir 2.0, VSSS-rs 3.0

### Data Structures & Storage
- **Serialization**: Serde 1.0, Bincode 1.3
- **Database**: None (pure in-memory with persistence via custom WAL)
- **Caching**: LRU 0.12 (with custom eviction strategies)
- **Compression**: Flate2 1.0
- **Concurrency**: Parking_lot 0.12 (faster mutexes/RwLocks)

### Monitoring & Observability
- **Metrics**: Prometheus 0.13
- **Logging**: Tracing 0.1, Tracing-subscriber 0.3
- **Structured Logging**: JSON output support

### Development Tools
- **CLI Parsing**: Clap 4.4 (optional feature)
- **HTTP Server**: Warp 0.3 (optional for MCP)
- **Testing**: Tokio-test, Proptest 1.4, Criterion 0.4
- **Mocking**: Built-in mock traits
- **Configuration**: Config 0.13 (layered configuration management)
  - Environment > File > Defaults precedence
  - TOML/JSON file support
  - SAORSA_* environment variable prefix
  - Development and production profiles
- **Property Testing**: Proptest 1.4 (property-based testing)
- **Error Handling**: Thiserror 1.0 (comprehensive framework - Task 1 complete)
  - Domain-specific error types for all modules
  - Zero-cost abstractions with Cow<'static, str>
  - Structured error logging with JSON support
  - Recovery patterns with Recoverable trait
  - Anyhow 1.0 integration for applications
- **Regular Expressions**: Regex 1.10 (configuration validation)
- **Temporary Files**: Tempfile 3.8 (for testing)
- **Performance**: SmallVec 1.11 (stack-allocated collections)
- **Security**: Zeroize 1.7 (secure memory wiping)

## Code Standards

### Rust Configuration
```toml
# Workspace-wide compiler settings
[workspace.lints.rust]
unsafe_code = "warn"
missing_docs = "warn"

[workspace.lints.clippy]
# Production critical
unwrap_used = "deny"  # Strict enforcement for production
expect_used = "deny"  # No expects in production paths
panic = "deny"
unimplemented = "deny"
todo = "warn"
unreachable = "deny"

# Performance
inefficient_to_string = "deny"
large_enum_variant = "warn"
needless_collect = "deny"

# Correctness
enum_glob_use = "deny"
mem_forget = "deny"

# Production readiness
missing_errors_doc = "warn"
missing_panics_doc = "warn"
```

### Code Formatting
- **Formatter**: rustfmt with default settings
- **Line Width**: 100 characters (default)
- **Imports**: Grouped and sorted automatically
- **Style**: Rust 2024 edition idioms

### Error Handling
```rust
// Comprehensive error types with thiserror (implemented in src/error.rs)
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    #[error("DHT error: {0}")]
    Dht(#[from] DhtError),
    
    #[error("Identity error: {0}")]
    Identity(#[from] IdentityError),
    
    #[error("Cryptography error: {0}")]
    Crypto(#[from] CryptoError),
    
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
    
    #[error("Bootstrap error: {0}")]
    Bootstrap(#[from] BootstrapError),
    
    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),
    
    #[error("Internal error: {0}")]
    Internal(Cow<'static, str>), // Zero-allocation for static messages
}

// Result type alias
pub type P2pResult<T> = Result<T, P2PError>;

// Error context trait for adding context without heap allocation
pub trait ErrorContext<T> {
    fn context(self, msg: &str) -> Result<T, P2PError>;
    fn with_context<F>(self, f: F) -> Result<T, P2PError>
    where F: FnOnce() -> String;
}

// Structured error logging with JSON support
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorLog {
    pub timestamp: i64,
    pub error_type: &'static str,
    pub message: Cow<'static, str>,
    pub context: SmallVec<[(&'static str, ErrorValue); 4]>,
    pub stack_trace: Option<Cow<'static, str>>,
}

// Recovery patterns for transient errors
pub trait Recoverable {
    fn is_transient(&self) -> bool;
    fn suggested_retry_after(&self) -> Option<Duration>;
    fn max_retries(&self) -> usize;
}
```

### Testing Requirements

#### Unit Tests
- Location: In-module `#[cfg(test)]` blocks
- Coverage: All public APIs must have tests
- Async: Use `#[tokio::test]` for async tests
- Naming: `test_` prefix for all test functions
- Error Cases: Must test error conditions explicitly

#### Integration Tests
- Location: `tests/` directory
- Framework: Tokio runtime with serial_test for isolation
- Coverage: Cross-component interactions
- Data Verification: All tests verify data integrity
- Error Flows: Test error propagation across modules

#### Property-Based Tests
- Framework: Proptest 1.4
- Coverage: API contracts, invariants, edge cases
- Strategies: Custom generators for domain types
- Minimum: 80% property coverage for public APIs
- Error Properties: Test error handling invariants

#### Benchmarks
- Framework: Criterion 0.4
- Location: `benches/` directory
- Metrics: Latency, throughput, memory usage
- Regular: Run on CI for regression detection
- Error Overhead: Measure error handling performance

##### Implemented Benchmark Suites
1. **Adaptive Network** (`adaptive_network_bench.rs`) - NetworkCoordinator performance
2. **EigenTrust** (`eigentrust_bench.rs`) - Trust computation scalability
3. **Eviction Strategies** (`eviction_bench.rs`) - Cache eviction algorithms
4. **GossipSub** (`gossipsub_bench.rs`) - Pub/sub message propagation
5. **Identity Encryption** (`identity_encryption_bench.rs`) - Crypto operations
6. **Multi-Armed Bandit** (`multi_armed_bandit_bench.rs`) - Route optimization
7. **Q-Learning Cache** (`q_learning_cache_bench.rs`) - ML cache decisions

### Documentation Standards

#### Code Documentation
```rust
//! Module-level documentation explaining purpose
//! 
//! ## Example
//! ```rust
//! // Example code here
//! ```

/// Function documentation with examples
/// 
/// # Arguments
/// * `param` - Description
/// 
/// # Returns
/// Description of return value
/// 
/// # Errors
/// When this function returns errors
pub fn example(param: Type) -> Result<ReturnType> {
    // Implementation
}
```

#### API Documentation
- All public items must have doc comments
- Include examples for non-trivial APIs
- Use `cargo doc --no-deps --open` to preview
- Published to docs.rs automatically

## Build System

### Cargo Workspace
- **Structure**: Monorepo with multiple crates
- **Resolver**: Version 2 for better dependency resolution
- **Shared Dependencies**: Defined at workspace level
- **Features**: Modular feature flags for optional components

### Release Profiles
```toml
[profile.release]
lto = "thin"              # Link-time optimization
codegen-units = 1         # Single codegen unit for best optimization
opt-level = 3             # Maximum optimization
panic = "abort"           # Smaller binaries

[profile.dev]
opt-level = 0             # Fast compilation
debug = true              # Full debug info
```

### CI/CD Pipeline
- **Linting**: `cargo clippy -- -D warnings`
- **Formatting**: `cargo fmt --check`
- **Testing**: `cargo test --all-features`
- **Security**: `cargo audit` for vulnerability scanning
- **Documentation**: `cargo doc --no-deps`

## Security Standards

### Cryptographic Guidelines
- No hardcoded secrets or keys
- Use secure random generators (`rand::rngs::OsRng`)
- Constant-time operations for crypto
- Zeroize sensitive data on drop
- Validate all external input
- Encryption Standards:
  - AES-256-GCM for symmetric encryption
  - Argon2id for key derivation (32MB memory, 2 iterations)
  - Ed25519 for signatures (v2.1)
  - X25519 for key exchange
  - 12-byte nonces for AES-GCM

### Memory Safety
- Minimize `unsafe` usage
- Document all safety invariants
- Use `mlock` for sensitive memory (via secure_memory module)
- Automatic zeroization with `zeroize` crate

### Network Security
- TLS 1.3 minimum for all connections
- Certificate validation for QUIC
- Rate limiting on all endpoints
- Input validation and sanitization

## Performance Standards

### Benchmarks Required For
- DHT operations (< 200ms lookup)
- Crypto operations (constant time)
- Network throughput (> 100 Mbps)
- Memory usage (< 100MB baseline)

### Optimization Guidelines
- Profile before optimizing
- Use `&str` over `String` when possible
- Prefer iterators over explicit loops
- Zero-copy operations where feasible
- Connection pooling for network ops

## Infrastructure

### Development Environment
- **Rust**: Latest stable via rustup
- **IDE**: VS Code with rust-analyzer recommended
- **Debugging**: LLDB or GDB with Rust support

### Deployment Targets
- **Binary Distribution**: Static binaries with musl
- **Container**: Minimal Alpine-based images
- **Package Managers**: Cargo for libraries, platform-specific for apps

### Monitoring & Operations
- **Metrics**: Prometheus-compatible endpoints
- **Logging**: Structured JSON logs
- **Health Checks**: HTTP endpoints for liveness/readiness
- **Debugging**: Tokio console support

## Production Standards

### Zero-Panic Policy
- **No unwrap() in production code**: All Results must be handled explicitly
- **No expect() in production code**: Use proper error propagation
- **No panic!() in production code**: Return errors instead
- **Comprehensive error handling**: Using custom error types with thiserror
- **Error context**: All errors include meaningful context for debugging

### Production Readiness Progress (4/15 Tasks Complete - 26.7%)
#### Sprint Status: IN PROGRESS ⚠️

**Foundation (Tasks 1-3)**:
- ✅ Task 1: Error handling framework (880 lines)
- ✅ Task 2: Fixed high-risk unwraps (95/568 eliminated)
- ✅ Task 3: Transport consolidation (quinn only)

**Security (Tasks 4-6)**:
- ⏳ Task 4: Identity encryption (PARTIAL - need persistence)
- ✅ Task 5: Configuration system (full validation)
- ⏳ Task 6: Input validation (TODO)

**Operations (Tasks 7-9)**:
- ⏳ Task 7: Health checks (TODO)
- ⏳ Task 8: TODO completion (142 remaining)
- ⏳ Task 9: Integration tests (TODO)

**Quality (Tasks 10-12)**:
- ⏳ Task 10: Final unwrap removal (473 remaining)
- ⏳ Task 11: Performance testing (TODO)
- ⏳ Task 12: Security audit (TODO)

**Finalization (Tasks 13-15)**:
- ⏳ Task 13: Monitoring setup (TODO)
- ⏳ Task 14: Documentation (TODO)
- ⏳ Task 15: Final validation (TODO)
### Production Readiness Reality (v0.2.6)

**Status**: NOT READY (45/100 score)
**Timeline**: 6-8 weeks required

#### What Works ✅
- 100% compilation success
- Error handling framework (880 lines)
- Configuration management system
- Transport simplified to pure QUIC
- 95/568 unwraps removed (16.7%)

#### Critical Blockers 🚨
1. **SECURITY EMERGENCY**:
   - Empty TLS certificates (NO ENCRYPTION!)
   - Vulnerable protobuf v2.28.0 (RUSTSEC-2024-0437)
   - Hardcoded test keys in production
   - Weak password validation

2. **PANIC RISKS**:
   - 473 unwrap() calls remaining
   - expect() usage throughout
   - panic!() in non-test code

3. **PERFORMANCE**:
   - O(n²) algorithms in DHT
   - Lock contention issues
   - No Arc<T> optimization

4. **QUALITY**:
   - Only 65-70% test coverage
   - 142 TODO/FIXME placeholders
   - Documentation gaps

## Version Control

### Git Workflow
- **Branching**: Feature branches from `main`
- **Commits**: Conventional commits (feat:, fix:, docs:, etc.)
- **PRs**: Required for all changes
- **Reviews**: At least one approval required

### Release Process
- **Versioning**: Semantic versioning (X.Y.Z)
- **Changelog**: Maintained in CHANGELOG.md
- **Tags**: Git tags for all releases
- **Publishing**: Automated via CI to crates.io