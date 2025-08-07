# P2P Foundation Coding Conventions

**Last Updated**: 2025-08-03

## Naming Conventions

### Files and Modules
- **Snake_case** for all file names: `identity_manager.rs`, `crypto_verify.rs`
- **Module names** match file names without extension
- **Test files** in same module with `#[cfg(test)]` block
- **Integration tests** in `tests/` directory with descriptive names

### Code Naming
```rust
// Types and traits: PascalCase
pub struct NodeIdentity { }
pub trait NetworkBehavior { }

// Functions and methods: snake_case
pub fn create_identity() -> Result<Identity>
pub async fn connect_to_peer() -> Result<()>

// Constants: SCREAMING_SNAKE_CASE
pub const MAX_CONNECTIONS: usize = 1000;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// Type parameters: Single uppercase letter or descriptive PascalCase
fn process<T: Serialize>(data: T) -> Result<()>
fn convert<Input, Output>(value: Input) -> Output
```

### Module Organization
```rust
// Standard module layout
use std::collections::HashMap;  // std imports first
use std::sync::Arc;

use tokio::sync::Mutex;        // external crates
use serde::{Serialize, Deserialize};

use crate::error::P2PError;    // internal imports
use crate::network::PeerId;

// Public items before private
pub struct PublicType { }
struct PrivateType { }

// Logical grouping with comment headers
// === Core Types ===
// === Implementation ===
// === Helper Functions ===
```

## Code Organization

### Project Structure
```
crates/p2p-core/src/
├── adaptive/           # Adaptive network subsystems
│   ├── mod.rs         # Module exports and common types
│   ├── routing.rs     # Specific subsystem
│   └── trust.rs       # Related functionality
├── identity/          # Identity management
├── network/           # Core networking
└── lib.rs            # Crate root with exports
```

### Module Guidelines
1. **Single Responsibility**: Each module handles one concern
2. **Clear Exports**: Re-export commonly used types in `mod.rs`
3. **Documentation**: Module-level docs explaining purpose
4. **Test Locality**: Unit tests in same file

### Import Order
```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates (alphabetical)
use anyhow::Result;
use tokio::sync::RwLock;

// 3. Crate imports
use crate::error::P2PError;

// 4. Module imports
use super::common::utils;
```

## Git Workflow

### Branch Naming
- **Feature branches**: `feat/description-of-feature`
- **Bug fixes**: `fix/issue-description`
- **Documentation**: `docs/what-is-documented`
- **Refactoring**: `refactor/what-is-refactored`

### Commit Messages
```
type(scope): subject

Body explaining what and why (not how)

Fixes #123
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code restructuring
- `perf`: Performance improvements
- `test`: Adding tests
- `chore`: Maintenance tasks

### Pull Request Process
1. Create feature branch from `main`
2. Make changes following conventions
3. Run tests and linting locally
4. Create PR with descriptive title
5. Address review feedback
6. Squash merge when approved

## Code Style

### Rust Idioms
```rust
// Use ? for error propagation
let data = read_file(path)?;

// Prefer match over unwrap
match result {
    Ok(value) => process(value),
    Err(e) => log::error!("Failed: {}", e),
}

// Use if let for single patterns
if let Some(peer) = peers.get(&id) {
    peer.send(message)?;
}

// Iterator chains over loops
let active_peers: Vec<_> = peers
    .iter()
    .filter(|p| p.is_active())
    .collect();
```

### Error Handling

#### Zero-Panic Policy (NOT ACHIEVED)
**Production code must NEVER panic**. Current violations:

**Production Sprint Progress (4/15 Tasks - 26.7%)**

**Task 1: Error Framework** ✅ COMPLETE
- Comprehensive error framework implemented (`src/error.rs`)
- 880 lines of error handling with thiserror
- Domain-specific error types for all modules
- Zero-cost abstractions with Cow<'static, str>

**Task 2: High-Risk Unwrap Removal** 🔄 PARTIAL (16.7%)
- Network: ✅ Zero unwraps (41 removed)
- Identity: ✅ Zero unwraps (54 removed)
- Transport: ✅ Already clean
- **Progress**: 95/568 unwraps removed (16.7%)
- **Remaining**: 473 unwraps (CRITICAL BLOCKER)

**Task 3: Transport Debt** ✅ COMPLETE
- Native ant-quic integration completed
- Removed legacy Transport/Connection abstractions

**Critical Violations**:
- 🚨 473 unwrap() calls can panic in production
- 🚨 Multiple expect() calls present
- 🚨 Some panic!() in non-test code
- 🚨 Clippy rules configured but not enforced

```rust
// ❌ NEVER DO THIS in production
let value = some_result.unwrap();  // Will panic on error
let data = option.expect("should exist");  // Will panic on None

// ✅ DO THIS instead (actual patterns from codebase)
let value = some_result
    .map_err(|e| P2PError::Internal(format!("Operation failed: {}", e).into()))?;
    
let data = option
    .ok_or_else(|| P2PError::Validation("Missing required data".into()))?;

// Using the ErrorContext trait (from src/error.rs)
use crate::error::ErrorContext;
let config = Config::load()
    .context("Failed to load configuration")?;

// For performance-critical paths with Cow<'static, str>
let err = P2PError::Internal("Static error message".into()); // Zero allocation

// Structured error logging
use crate::error::ErrorReporting;
if let Err(e) = operation() {
    e.log(); // Automatically logs with appropriate level
    let log_entry = e.to_error_log(); // For custom monitoring
}
```

#### Panic Risk Assessment

**Modules with Zero Unwraps** ✅:
- Network: 41 unwraps removed
- Identity: 54 unwraps removed  
- Transport: Already clean

**High-Risk Modules** (473 unwraps) 🚨:
- DHT: Highest concentration
- Adaptive: Extensive unwrap usage
- Storage: Many unwraps
- Bootstrap: Critical path unwraps
- MCP: Service unwraps
- Projects: Application unwraps

**Impact**: ANY of these 473 unwraps can crash the entire system

#### Custom Error Types (Task 1 ✅ COMPLETE)
```rust
// IMPORTANT: Use thiserror for libraries, NOT anyhow
// This pattern was established in Task 1 (src/error.rs)

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Connection failed to {addr}: {reason}")]
    ConnectionFailed { 
        addr: SocketAddr, 
        reason: String 
    },
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error(transparent)]
    Transport(#[from] TransportError),
}

// Comprehensive error type for P2P operations
// Fully implemented with all subsystem errors
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
    
    // Zero-allocation for static messages
    #[error("Internal error: {0}")]
    Internal(Cow<'static, str>),
}

// Type alias for convenience
pub type P2pResult<T> = Result<T, P2PError>;
```

#### Error Context Pattern
```rust
// Use error context helpers for meaningful errors
use crate::error::ErrorContext;

pub async fn store_data(key: &[u8], value: &[u8]) -> P2pResult<()> {
    // Add context at each error point
    validate_key(key)
        .context("Invalid storage key")?;
        
    let encoded = encode_value(value)
        .with_context(|| format!("Failed to encode {} bytes", value.len()))?;
        
    storage.put(key, &encoded)
        .await
        .context("Storage operation failed")?;
        
    Ok(())
}

// For applications, anyhow::Result is acceptable
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()
        .context("Failed to load configuration")?;
        
    let node = P2PNode::new(config)
        .await
        .context("Failed to initialize P2P node")?;
        
    Ok(())
}
```

### Async Patterns
```rust
// Always use tokio::spawn for background tasks
tokio::spawn(async move {
    if let Err(e) = background_task().await {
        log::error!("Background task failed: {}", e);
    }
});

// Timeouts for network operations
use tokio::time::timeout;

timeout(Duration::from_secs(30), async {
    peer.connect().await
}).await??;

// Coordinator extension pattern for modularity
impl CoordinatorExtensions for NetworkCoordinator {
    async fn with_monitoring(mut self) -> Self {
        self.enable_monitoring().await;
        self
    }
}
```

## Documentation Requirements

### Public API Documentation
```rust
/// Brief one-line description.
///
/// More detailed explanation of what this does,
/// when to use it, and any important notes.
///
/// # Arguments
///
/// * `param1` - Description of first parameter
/// * `param2` - Description of second parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// * `NetworkError::ConnectionFailed` - When connection cannot be established
/// * `NetworkError::Timeout` - When operation times out
///
/// # Examples
///
/// ```rust
/// use p2p_core::connect;
///
/// let connection = connect("peer_address").await?;
/// ```
pub async fn connect(address: &str) -> Result<Connection, NetworkError> {
    // Implementation
}
```

### Module Documentation
```rust
//! # Module Name
//!
//! Brief description of module purpose.
//!
//! ## Overview
//!
//! Longer explanation of what this module provides
//! and how it fits into the larger system.
//!
//! ## Usage
//!
//! Basic usage examples and common patterns.
```

## Performance Standards

### Optimization Guidelines
1. **Profile First**: Never optimize without profiling
2. **Document Optimizations**: Explain why code is optimized
3. **Benchmarks**: Add criterion benchmarks for optimized code

### Common Patterns
```rust
// Use Cow for potentially owned strings (heavily used in error.rs)
use std::borrow::Cow;
fn process(data: Cow<str>) -> String {
    // Avoid allocation if not needed
}

// Arc for shared immutable data (needed for DHT optimization)
use std::sync::Arc;
let shared_config = Arc::new(config);

// Pre-allocate collections when size is known
let mut peers = Vec::with_capacity(expected_peers);

// SmallVec for stack-allocated small collections (used in ErrorLog)
use smallvec::SmallVec;
let mut context: SmallVec<[(&'static str, ErrorValue); 4]> = SmallVec::new();

// Atomics for lock-free counters and flags
use std::sync::atomic::{AtomicU64, Ordering};
static COUNTER: AtomicU64 = AtomicU64::new(0);

// Configuration management pattern (FULLY IMPLEMENTED)
// Complete system in config.rs with validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub listen_address: String,
    pub bootstrap_nodes: Vec<String>,
    pub max_connections: usize,
    // ... other fields with defaults
}

// Environment variable override pattern (SAORSA_* prefix)
// Precedence: Environment > File > Defaults
if let Ok(val) = env::var("SAORSA_LISTEN_ADDRESS") {
    config.network.listen_address = val;
}

// Config files provided:
// - config.example.toml (fully documented)
// - config.development.toml (optimized for local dev)
// - config.production.toml (security-hardened)
// - config.test.toml (isolated test environment)
```

## Transport Layer Conventions

### Transport Architecture Decision (Task 3: Completed ✅)
The P2P Foundation uses **ant-quic as the primary transport**:
- **Integrated**: ant-quic v0.6.1 with NAT traversal and post-quantum crypto
- **Native**: Direct use of ant-quic's QuicP2PNode without abstraction layers
- **Advanced**: Built-in NAT traversal using IETF draft-seemann-quic-nat-traversal-01
- **Secure**: Post-quantum cryptography (ML-KEM-768, ML-DSA-65) built-in

### Transport Patterns
```rust
// QUIC transport is the only option
pub enum TransportType {
    QUIC,  // Single variant enum for future extensibility
}

// Connection pooling for performance
pub struct ConnectionPool {
    active: HashMap<NodeId, Connection>,
    max_connections: usize,
    idle_timeout: Duration,
}

// Connection quality monitoring
pub struct ConnectionQuality {
    pub latency: Duration,
    pub throughput_mbps: f64,
    pub packet_loss: f64,
    pub jitter: Duration,
}
```

### Transport Best Practices
1. **Connection Reuse**: Always use connection pooling
2. **0-RTT**: Enable for repeat connections
3. **Multiplexing**: Use QUIC streams for concurrent operations
4. **Error Handling**: All transport errors map to TransportError
5. **Monitoring**: Track connection quality metrics

## Security Conventions

### Input Validation
```rust
// Always validate external input
pub fn set_peer_id(id: &str) -> Result<PeerId> {
    if id.is_empty() {
        return Err(P2PError::validation("Peer ID cannot be empty"));
    }
    if id.len() > MAX_PEER_ID_LENGTH {
        return Err(P2PError::validation("Peer ID too long"));
    }
    // Additional validation...
    Ok(PeerId::from(id))
}
```

### Secure Defaults
- TLS 1.3 minimum for all connections
- Encryption enabled by default
- Rate limiting on all public endpoints
- Timeout on all network operations

### Cryptographic Standards
- **Symmetric Encryption**: AES-256-GCM
- **Key Derivation**: Argon2id (32MB memory, 2 iterations)
- **Digital Signatures**: Ed25519 (v2.1)
- **Key Exchange**: X25519
- **Nonce Generation**: 12 bytes via OsRng

### Secret Handling
```rust
// Use zeroize for sensitive data
use zeroize::Zeroize;

#[derive(Zeroize)]
#[zeroize(drop)]
struct SecretKey {
    key: Vec<u8>,
}

// Never log secrets
log::debug!("Connecting with key: [REDACTED]");
```

### Security Status (CRITICAL ISSUES)

#### Working Security Features
- ✅ Four-word address system implemented
- ✅ Ed25519 cryptography (v2.1 migration complete)
- 🔄 Secure memory code exists (not fully integrated)
- 🔄 Monotonic counter implemented

#### Critical Security Vulnerabilities 🚨
1. **NO ENCRYPTION**: Empty TLS certificates in QUIC
2. **Vulnerable Dependency**: protobuf v2.28.0 (RUSTSEC-2024-0437)
3. **Hardcoded Test Keys**: In production code paths
4. **Weak Passwords**: Only 10 passwords validated
5. **No Input Validation**: Task 6 not started

#### Security Tasks Status
- Task 4 (Identity Encryption): 📋 Code exists, not integrated
- Task 6 (Input Validation): 📋 Not started
- Task 12 (Security Audit): 🔴 Found critical issues

## Testing Conventions

### Test Organization
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    // Unit tests for specific functions
    #[test]
    fn test_peer_id_validation() {
        assert!(PeerId::from_str("").is_err());
        assert!(PeerId::from_str("valid_id").is_ok());
    }
    
    // Async tests use tokio::test
    #[tokio::test]
    async fn test_connection() {
        let conn = connect("test_addr").await;
        assert!(conn.is_ok());
    }
    
    // Property-based tests with proptest
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_serialization_roundtrip(data: Vec<u8>) {
            let encoded = encode(&data);
            let decoded = decode(&encoded)?;
            prop_assert_eq!(data, decoded);
        }
        
        #[test]
        fn test_network_address_parsing(
            ip in any::<std::net::IpAddr>(),
            port in 1024u16..65535u16
        ) {
            let addr = format!("{}:{}", ip, port);
            let parsed = NetworkAddress::from_str(&addr)?;
            prop_assert_eq!(parsed.ip(), ip);
            prop_assert_eq!(parsed.port(), port);
        }
        
        #[test]
        fn test_error_context_preservation(
            msg in "[a-zA-Z0-9 ]{1,100}",
            context in "[a-zA-Z0-9 ]{1,50}"
        ) {
            let err = P2PError::internal(msg.clone());
            let with_context = err.context(&context);
            let error_string = with_context.to_string();
            prop_assert!(error_string.contains(&msg));
            prop_assert!(error_string.contains(&context));
        }
        
        // Actual property tests from codebase
        #[test]
        fn prop_node_identity_deterministic(seed in prop::array::uniform32(any::<u8>())) {
            // Same seed should always produce same identity
            let id1 = NodeIdentity::from_seed(&seed).unwrap();
            let id2 = NodeIdentity::from_seed(&seed).unwrap();
            
            prop_assert_eq!(id1.node_id(), id2.node_id());
            prop_assert_eq!(id1.word_address(), id2.word_address());
        }
    }
    
    // Error handling tests
    #[test]
    fn test_error_propagation() {
        let result: Result<(), P2PError> = Err(NetworkError::Timeout.into());
        assert!(matches!(result, Err(P2PError::Network(_))));
    }
}
```

### Test Coverage Reality
- **Current Coverage**: 🔴 65-70% (target: 80%+) - TOO LOW
- **Total Tests**: 719 tests (but gaps remain)
- **Network Module**: Basic tests only
- **Identity Module**: Unit tests present
- **Property Tests**: Some proptest usage
- **Benchmarks**: 📋 Files exist but Task 11 not started
- **Integration Tests**: Basic only, no failure scenarios
- **Critical Gaps**: 
  - No network failure testing
  - No concurrent operation tests
  - No adversarial testing
  - No performance baselines

### Test Naming
- `test_` prefix for all test functions
- Descriptive names: `test_connection_timeout_handling`
- Group related tests with modules

### Integration Tests
```rust
// tests/network_integration_test.rs
use p2p_core::test_utils::setup_test_network;

#[tokio::test]
async fn test_multi_node_communication() {
    let network = setup_test_network(5).await;
    // Test cross-node communication
}
```

### Benchmark Organization
```rust
// benches/routing_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_routing_performance(c: &mut Criterion) {
    c.bench_function("route_lookup", |b| {
        b.iter(|| {
            router.find_route(black_box(&destination))
        });
    });
}

criterion_group!(benches, bench_routing_performance);
criterion_main!(benches);
```

## Review Process

### Code Review Checklist
- [ ] Follows naming conventions
- [ ] Appropriate error handling (no unwrap/expect)
- [ ] Documented public APIs
- [ ] Tests for new functionality
- [ ] Error conditions tested
- [ ] No unwrap() or panic!() in production code
- [ ] Security considerations addressed
- [ ] Performance impact considered
- [ ] Configuration values not hardcoded
- [ ] Logging uses tracing (not println!)

### Review Standards
1. **Constructive Feedback**: Focus on code, not person
2. **Explain Why**: Provide reasoning for suggestions
3. **Suggest Alternatives**: Offer solutions, not just problems
4. **Praise Good Code**: Acknowledge well-written sections

### Production Readiness Checklist

**VERDICT: NOT READY** - Score: 45/100

- **Error Handling**: 🔴 473 unwraps (BLOCKER)
- **Panic Safety**: 🔴 Can crash in production
- **Security**: 🔴 NO ENCRYPTION + vulnerabilities
- **Performance**: 🔴 Will fail under load
- **Test Coverage**: 🔴 65-70% (need 80%+)
- **Documentation**: 🔴 142 TODOs, placeholders
- **Dependencies**: 🔴 Known vulnerabilities
- **Configuration**: ✅ Basic system works
- **Transport**: ✅ ant-quic integrated with NAT traversal

### Production Readiness Sprint Status

**Sprint Progress: 4/15 Tasks (26.7%) - NOT READY**

#### Completed Tasks ✅
1. **Error Handling Framework** - 880 lines, comprehensive
2. **Fix High-Risk Unwraps** - 95/568 removed (16.7%)
3. **Remove Transport Debt** - Native ant-quic integration

#### Critical Path (Must Fix First)
4. **Fix TLS Certificates** - Currently EMPTY
5. **Remove ALL Unwraps** - 473 remaining
6. **Fix Vulnerabilities** - protobuf v2.28.0

#### Remaining Tasks (9/15)
7. Identity Encryption (code exists)
8. Configuration Hardcoding
9. Input Validation
10. Health Checks
11. TODO Completion (142)
12. Integration Tests
13. Performance Testing
14. Monitoring Setup
15. Documentation

#### Timeline Estimate
- **Weeks 1-2**: Security fixes (TLS, vulnerabilities)
- **Weeks 3-4**: Panic removal (473 unwraps)
- **Weeks 5-6**: Testing and performance
- **Weeks 7-8**: Final validation

**Total: 6-8 weeks to production**

## Continuous Integration

### Required Checks
```bash
# Format check
cargo fmt --all -- --check

# Linting (with production rules)
cargo clippy --all-features -- -D warnings \
  -D clippy::unwrap_used \
  -D clippy::expect_used \
  -D clippy::panic \
  -D clippy::unimplemented \
  -D clippy::todo

# Tests
cargo test --all-features

# Benchmarks
cargo bench

# Documentation
cargo doc --no-deps

# Security audit
cargo audit

# Coverage report
cargo tarpaulin --out Html
```

### Performance Regression
- Benchmarks run on every PR
- Alert on >5% performance degradation
- Require justification for slowdowns

## Debugging Conventions

### Logging
```rust
// Use structured logging with tracing
use tracing::{info, debug, error, instrument};

#[instrument(skip(large_data))]
pub async fn process_data(id: u64, large_data: &[u8]) -> Result<()> {
    info!("Processing data for peer {}", id);
    debug!("Data size: {} bytes", large_data.len());
    
    if let Err(e) = validate_data(large_data) {
        error!("Data validation failed: {}", e);
        return Err(e);
    }
    
    Ok(())
}
```

### Debug Assertions
```rust
// Use debug_assert! for development checks
debug_assert!(!peers.is_empty(), "Peer list should not be empty");
debug_assert_eq!(computed_hash, expected_hash);
```

## Migration and Deprecation

### Deprecation Process
```rust
#[deprecated(since = "0.2.0", note = "Use `new_function` instead")]
pub fn old_function() {
    // Still works but warns users
}
```

### Breaking Changes
1. Document in CHANGELOG.md
2. Provide migration guide
3. Deprecate for 2 minor versions
4. Remove in next major version