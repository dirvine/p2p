# Project Conventions

## Code Style

### Rust Conventions

#### Naming
```rust
// Modules: snake_case
mod network_layer;

// Types: PascalCase
struct NetworkManager;
trait StorageBackend;
enum MessageType;

// Functions/Methods: snake_case
fn process_message() {}
impl NetworkManager {
    fn new() -> Self {}
    fn connect_to_peer(&self) {}
}

// Constants: SCREAMING_SNAKE_CASE
const MAX_CONNECTIONS: usize = 1000;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// Variables: snake_case
let peer_count = 42;
let mut connection_pool = Vec::new();
```

#### Error Handling
```rust
// Library errors with thiserror
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Connection to {peer} failed: {reason}")]
    ConnectionFailed { peer: String, reason: String },
    
    #[error("Invalid message format")]
    InvalidMessage(#[from] serde_json::Error),
}

// Application errors with anyhow
use anyhow::{Result, Context};

pub async fn connect(addr: &str) -> Result<Connection> {
    TcpStream::connect(addr)
        .await
        .context("Failed to establish TCP connection")?
}

// Never use unwrap() in production code
// BAD
let value = some_option.unwrap();

// GOOD
let value = some_option.ok_or_else(|| {
    anyhow!("Expected value to be present")
})?;
```

#### Async Patterns
```rust
// Always use tokio for async runtime
#[tokio::main]
async fn main() -> Result<()> {
    // Main application logic
}

// Async trait methods
use async_trait::async_trait;

#[async_trait]
trait AsyncStorage {
    async fn store(&self, key: &str, value: Vec<u8>) -> Result<()>;
}

// Concurrent operations
use futures::future::join_all;

let futures = nodes.iter().map(|node| node.ping());
let results = join_all(futures).await;
```

#### Documentation
```rust
//! Module-level documentation
//! 
//! This module handles peer-to-peer networking operations.

/// Manages network connections and peer discovery.
/// 
/// # Examples
/// 
/// ```
/// use p2p_core::NetworkManager;
/// 
/// let manager = NetworkManager::new(config)?;
/// manager.start().await?;
/// ```
pub struct NetworkManager {
    /// Current peer connections
    peers: Vec<Peer>,
}

impl NetworkManager {
    /// Creates a new network manager with the given configuration.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Network configuration parameters
    /// 
    /// # Errors
    /// 
    /// Returns error if configuration is invalid
    pub fn new(config: Config) -> Result<Self> {
        // Implementation
    }
}
```

### Testing Conventions

#### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        // Arrange
        let msg = Message::new("test");
        
        // Act
        let serialized = msg.serialize().unwrap();
        let deserialized = Message::deserialize(&serialized).unwrap();
        
        // Assert
        assert_eq!(msg, deserialized);
    }

    #[tokio::test]
    async fn test_async_operation() {
        // Async test implementation
    }
}
```

#### Integration Tests
```rust
// tests/network_integration.rs
use p2p_core::test_utils::{start_test_network, TestNode};

#[tokio::test]
async fn test_peer_discovery() {
    // Start test network
    let network = start_test_network(3).await;
    
    // Wait for discovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Verify all peers discovered each other
    for node in &network.nodes {
        assert_eq!(node.peer_count().await, 2);
    }
}
```

## Project Structure

### File Organization
```
crates/p2p-core/src/
├── lib.rs              # Public API exports
├── adaptive/           # Feature modules
│   ├── mod.rs         # Module exports
│   ├── identity.rs    # Single responsibility
│   └── transport.rs   # Focused modules
├── network/           # Core networking
├── utils/             # Shared utilities
└── tests/             # Test utilities
```

### Module Guidelines
- One primary type per file
- Related helper types in same file
- Public API in mod.rs
- Internal modules may be private

## Git Workflow

### Branch Naming
- `feature/description` - New features
- `fix/issue-description` - Bug fixes
- `docs/what-changed` - Documentation
- `refactor/what-changed` - Code refactoring
- `test/what-added` - Test additions

### Commit Messages
```
type(scope): subject

Longer description explaining the what and why.

Fixes #123
BREAKING CHANGE: Description of breaking change
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Test additions/changes
- `build`: Build system changes
- `ci`: CI/CD changes
- `chore`: Maintenance tasks

### Pull Request Process
1. Create feature branch from `main`
2. Make changes with atomic commits
3. Add/update tests
4. Update documentation
5. Run full test suite
6. Create PR with description
7. Address review feedback
8. Squash merge when approved

## Dependency Management

### Adding Dependencies
```toml
# Workspace dependencies (preferred)
[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }

# Crate uses workspace version
[dependencies]
tokio = { workspace = true }
```

### Version Pinning
- Pin major versions for stability
- Use exact versions for security-critical deps
- Document why specific versions are required

### Security Considerations
- Run `cargo audit` before commits
- Keep dependencies minimal
- Review new dependencies carefully
- Prefer well-maintained crates

## Performance Guidelines

### Benchmarking
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_lookup(c: &mut Criterion) {
    let network = setup_test_network();
    
    c.bench_function("dht_lookup", |b| {
        b.iter(|| {
            network.lookup(black_box(&key))
        })
    });
}

criterion_group!(benches, benchmark_lookup);
criterion_main!(benches);
```

### Optimization Rules
1. Profile before optimizing
2. Document performance-critical sections
3. Prefer algorithmic improvements
4. Use benchmarks to verify improvements
5. Consider memory usage, not just speed

## Documentation Standards

### API Documentation
- All public items must have doc comments
- Include usage examples for complex APIs
- Document error conditions
- Explain performance characteristics
- Cross-reference related items

### Architecture Documentation
- Keep diagrams up-to-date
- Document design decisions
- Explain trade-offs
- Include sequence diagrams for complex flows

### README Files
- Project overview at root
- Module README for complex subsystems
- Installation instructions
- Usage examples
- Contributing guidelines

## Security Practices

### Code Security
```rust
// Validate all inputs
pub fn process_data(data: &[u8]) -> Result<()> {
    if data.len() > MAX_DATA_SIZE {
        return Err(anyhow!("Data too large"));
    }
    // Process validated data
}

// Never log sensitive data
trace!("Processing request from peer {}", peer_id);
// NOT: trace!("Private key: {:?}", private_key);

// Use constant-time comparisons for secrets
use subtle::ConstantTimeEq;
if hash.ct_eq(&expected_hash).into() {
    // Hashes match
}
```

### Cryptographic Guidelines
- Use established libraries (ed25519-dalek, x25519-dalek)
- Never implement custom crypto
- Generate randomness with `rand::thread_rng()`
- Zeroize sensitive data when done

## Monitoring & Logging

### Logging Levels
```rust
use tracing::{error, warn, info, debug, trace};

error!("Critical error: {}", err);      // System failures
warn!("Degraded performance");          // Important warnings
info!("Server started on {}", addr);    // Key events
debug!("Processing message: {}", id);   // Detailed flow
trace!("Entering function X");          // Verbose tracing
```

### Structured Logging
```rust
#[instrument(skip(large_data))]
async fn process_request(
    peer: &PeerId,
    request_type: &str,
    large_data: &[u8]
) -> Result<Response> {
    info!(
        peer_id = %peer,
        request_type = request_type,
        data_size = large_data.len(),
        "Processing request"
    );
    // Implementation
}
```

## Release Process

### Version Numbering
- Follow Semantic Versioning (SemVer)
- Major: Breaking API changes
- Minor: New features, backwards compatible
- Patch: Bug fixes, backwards compatible

### Release Checklist
- [ ] All tests passing
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] Security audit run
- [ ] Performance benchmarks run
- [ ] Migration guide (if breaking changes)

## Code Review Guidelines

### Review Focus
1. **Correctness** - Does it work as intended?
2. **Security** - Any vulnerabilities?
3. **Performance** - Any obvious bottlenecks?
4. **Maintainability** - Is it easy to understand?
5. **Testing** - Adequate test coverage?
6. **Documentation** - Well documented?

### Review Comments
- Be constructive and specific
- Suggest improvements, not just problems
- Distinguish between required and optional changes
- Appreciate good code when you see it

## Environment Setup

### Required Tools
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add rustfmt clippy

# Development tools
cargo install cargo-edit cargo-watch cargo-audit

# Optional but recommended
cargo install cargo-expand cargo-outdated tokio-console
```

### IDE Configuration
- Use rust-analyzer for IDE support
- Enable format on save
- Configure clippy warnings
- Set up debugging configuration

## Continuous Integration

### CI Pipeline Stages
1. **Format Check** - `cargo fmt --check`
2. **Lint** - `cargo clippy -- -D warnings`
3. **Build** - `cargo build --all-features`
4. **Test** - `cargo test --all-features`
5. **Security** - `cargo audit`
6. **Benchmarks** - `cargo bench`

### Platform Testing
- Linux (Ubuntu latest)
- macOS (latest)
- Windows (latest)
- Minimum Rust version (1.75)