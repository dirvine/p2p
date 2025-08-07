# Technology Stack & Standards

## Languages & Runtimes

### Primary Language
- **Rust**: Edition 2024
- **Minimum Rust Version**: 1.75+
- **Async Runtime**: Tokio 1.35 with full features
- **Target Platforms**: Linux, macOS, Windows, iOS, Android, WebAssembly

### Secondary Languages
- **TypeScript/JavaScript**: For Tauri frontend (apps/saorsa/src)
- **HTML/CSS**: UI styling with modern CSS features
- **Shell Scripts**: Build automation and deployment

## Core Dependencies

### Networking Stack
- **ant-quic** (0.6.1): Advanced QUIC implementation with NAT traversal
  - Post-quantum cryptography (ML-KEM-768, ML-DSA-65)
  - ICE-like NAT traversal protocol (IETF draft-seemann-quic-nat-traversal-01)
  - Connection migration support
  - Direct peer-to-peer connectivity
- **four-word-networking** (2.3.1): Human-readable address system
  - Four-word addresses like "forest.lightning.compass.river"
  - DHT-integrated for address resolution
  - Voice-friendly for communication
- **rustls** (0.23): TLS implementation in pure Rust

### Cryptography
- **ed25519-dalek** (2.1): EdDSA signatures
- **ml-kem** (0.2): FIPS 203 post-quantum KEM (Kyber)
- **ml-dsa** (0.1.0-pre.2): FIPS 204 post-quantum signatures (Dilithium)
- **frost-ed25519** (2.0.0-rc.0): Threshold signatures
- **vsss-rs** (3.0): Verifiable secret sharing
- **shamir** (2.0): Secret sharing scheme
- **aes-gcm** (0.10): Authenticated encryption
- **sha2** (0.10): SHA-256/512 hashing
- **blake3**: Fast content addressing
- **hkdf** (0.12): Key derivation

### Data & Serialization
- **serde** (1.0): Serialization framework with derive macros
- **serde_json** (1.0): JSON support
- **serde-big-array** (0.5): Large array serialization
- **bincode**: Binary serialization for network messages
- **bytes** (1.0): Byte buffer management
- **base64** (0.22): Base64 encoding
- **hex** (0.4): Hexadecimal encoding

### Utilities
- **anyhow** (1.0): Flexible error handling for applications
- **thiserror** (1.0): Error derive macros for libraries
- **tracing** (0.1): Structured logging and diagnostics
- **log** (0.4): Legacy logging facade
- **chrono** (0.4): Date and time handling
- **uuid** (1.0): UUID generation with v4 support
- **toml** (0.8): Configuration file parsing
- **shellexpand** (3.1): Path expansion

### Testing
- **tokio-test**: Async test utilities
- **proptest**: Property-based testing
- **criterion**: Benchmarking framework
- **mockall**: Mock object generation

## Code Standards

### Rust Configuration

#### Workspace Lints (Cargo.toml)
```toml
[workspace.lints.rust]
unsafe_code = "warn"
missing_docs = "warn"
anonymous_parameters = "deny"
non_ascii_idents = "deny"
trivial_numeric_casts = "deny"

[workspace.lints.clippy]
# Critical for production
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
panic_in_result_fn = "deny"
unimplemented = "deny"
todo = "warn"
unreachable = "deny"

# Security
arithmetic_overflow = "deny"
cast_possible_truncation = "warn"
indexing_slicing = "warn"
```

### Error Handling Standards
- **NEVER use `unwrap()` or `expect()` in production code**
- Use `Result<T, E>` for all fallible operations
- Use `anyhow::Result` in applications
- Define custom error types with `thiserror` in libraries
- Always propagate errors with `?` operator
- Provide context with `.context()` from anyhow

### Async Programming
- Always use Tokio runtime
- Prefer `async`/`await` over manual futures
- Use `tokio::select!` for concurrent operations
- Implement graceful shutdown with cancellation tokens
- Avoid blocking operations in async context

### Documentation Requirements
- All public APIs must have doc comments
- Include examples in module-level documentation
- Use `cargo doc --no-deps` to verify documentation
- Document invariants and safety requirements
- Include usage examples for complex APIs

## Testing Strategy

### Test Coverage Requirements
- **Minimum 80% code coverage** for all modules
- Unit tests for all business logic
- Integration tests for API endpoints
- End-to-end tests for critical paths
- Property-based tests for complex algorithms

### Test Organization
```
tests/
├── unit/           # Unit tests
├── integration/    # Integration tests
├── e2e/           # End-to-end tests
└── benchmarks/    # Performance benchmarks
```

### Test Execution
```bash
# Run all tests
cargo test --all-features

# Run with coverage
cargo tarpaulin --out Html

# Run benchmarks
cargo bench

# Run specific test suite
cargo test -p ant-test-suite
```

## Build Tools

### Rust Toolchain
- **Cargo**: Package manager and build tool
- **rustfmt**: Code formatting (config in rustfmt.toml)
- **clippy**: Linting with strict rules
- **cargo-audit**: Security vulnerability scanning
- **cargo-tarpaulin**: Code coverage reporting

### Frontend Build (Tauri Apps)
- **npm/yarn**: Package management
- **Vite**: Fast frontend bundling
- **Tauri CLI**: Cross-platform app building
- **TypeScript**: Type-safe JavaScript

### CI/CD Pipeline
- **GitHub Actions**: Automated testing and deployment
- **Docker**: Containerization for deployment
- **cargo-release**: Automated version management

## Infrastructure

### Deployment Targets
- **Native Binaries**: Direct OS installation
- **Docker Containers**: Containerized deployment
- **Kubernetes**: Orchestrated deployment
- **Edge Devices**: IoT and embedded systems
- **WebAssembly**: Browser-based deployment

### Monitoring & Observability
- **OpenTelemetry**: Distributed tracing
- **Prometheus**: Metrics collection
- **Grafana**: Metrics visualization
- **Custom dashboards**: Network health monitoring

## Security Standards

### Cryptographic Requirements
- **Post-quantum algorithms mandatory** for new deployments
- Hybrid classical/PQ during transition period
- Regular security audits with cargo-audit
- No hardcoded secrets or credentials
- Environment variables for sensitive configuration

### Network Security
- **TLS 1.3 minimum** for all connections
- Certificate pinning for known peers
- Rate limiting on all endpoints
- DDoS protection mechanisms
- Input validation on all external data

## Performance Standards

### Benchmarks
- DHT lookup: < 1ms average
- Connection establishment: < 100ms
- Message routing: < 10ms
- Throughput: > 100 Mbps per connection
- Memory usage: < 100MB base footprint

### Optimization Guidelines
- Profile before optimizing
- Use `&str` over `String` when possible
- Prefer iterators over explicit loops
- Zero-copy operations where feasible
- Connection pooling for resource efficiency

## Development Workflow

### Git Workflow
- **Main branch**: Production-ready code
- **Feature branches**: feature/description
- **Bugfix branches**: fix/issue-number
- **Conventional commits**: feat, fix, docs, test, refactor
- **PR required**: All changes via pull request

### Code Review Standards
- All code must be reviewed before merge
- Run tests locally before PR
- Update documentation with code changes
- Follow style guide consistently
- Address all review comments

### Release Process
1. Version bump in Cargo.toml
2. Update CHANGELOG.md
3. Run full test suite
4. Create git tag
5. Build release artifacts
6. Publish to crates.io (for libraries)
7. Create GitHub release

## Formatting & Style

### Rust Formatting (rustfmt.toml)
```toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

### Naming Conventions
- **Types**: `CamelCase`
- **Functions/Methods**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`
- **Crate names**: `kebab-case`

### File Organization
- One module per file for clarity
- Group related functionality
- Keep files under 500 lines
- Separate concerns into modules
- Use mod.rs for module organization

## Documentation Standards

### Code Documentation
- Document all public APIs
- Include usage examples
- Document error conditions
- Explain complex algorithms
- Note performance characteristics

### External Documentation
- README.md for each crate
- API documentation with rustdoc
- Architecture diagrams in docs/
- User guides for applications
- Migration guides for breaking changes