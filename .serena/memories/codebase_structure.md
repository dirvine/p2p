# P2P Foundation Codebase Structure

## Root Directory
```
p2p/
├── Cargo.toml              # Workspace configuration
├── Cargo.lock              # Dependency lock file
├── README.md               # Project documentation
├── LICENSE-AGPL            # Open source license
├── LICENSE-COMMERCIAL      # Commercial license
├── .github/                # GitHub Actions CI/CD
├── .claude/                # Claude AI development configuration
├── .serena/                # Serena MCP configuration
├── apps/                   # Application layer
├── crates/                 # Core Rust libraries
├── docs/                   # Documentation
├── scripts/                # Build and utility scripts
└── tests/                  # Integration tests
```

## Core Libraries (`crates/`)

### Foundation Layer
- **p2p-core**: Main P2P library (published as saorsa-core)
  - Network management
  - DHT implementation
  - Identity system
  - MCP integration
  - Configuration management

### Network & Transport
- **p2p-transport**: Transport layer abstractions
  - QUIC/TCP implementations
  - IPv6/IPv4 tunneling
  - Connection pooling
- **p2p-network**: High-level network operations
  - Peer discovery
  - Message routing
  - Connection management

### Storage & DHT
- **p2p-dht**: Distributed Hash Table
  - Kademlia routing
  - Content addressing
  - Replication strategies
- **p2p-storage**: Persistent storage
  - Encrypted data storage
  - Cache management
  - Data repair mechanisms

### Identity & Security
- **p2p-identity**: Identity management
  - ML-KEM/ML-DSA cryptography
  - Three-word addresses
  - Passkey authentication
  - FROST threshold crypto

### Learning & Optimization
- **p2p-learning**: Machine learning components
  - Q-Learning cache optimization
  - Thompson Sampling
  - Adaptive strategies
- **p2p-som**: Self-Organizing Maps
  - Network topology learning
  - Peer clustering
- **p2p-hyperbolic**: Hyperbolic routing
  - Efficient peer discovery
  - Geometric routing

### Communication
- **p2p-gossip**: Gossipsub protocol
  - Pub/sub messaging
  - Broadcast optimization
  - Topic management
- **p2p-trust**: Trust and reputation
  - EigenTrust implementation
  - Reputation scoring

### Additional Components
- **p2p-node**: Node implementation
  - Complete P2P node
  - Service orchestration
- **p2p-client**: Client library
  - High-level API
  - Application interface
- **p2p-cli**: Command-line tools
  - Administrative tools
  - Debugging utilities
- **p2p-ffi**: Foreign Function Interface
  - C bindings
  - Language interoperability
- **ant-test-suite**: Testing framework
  - Integration tests
  - Performance benchmarks
  - Network simulations

## Applications (`apps/`)

### Saorsa (Main App)
```
apps/saorsa/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── commands.rs    # Tauri commands
│   │   └── platform/      # Platform-specific code
│   └── Cargo.toml
├── src/                    # Frontend (React/TypeScript)
│   ├── components/         # React components
│   ├── hooks/             # Custom hooks
│   ├── services/          # API services
│   └── App.tsx
├── package.json           # Node dependencies
└── tauri.conf.json        # Tauri configuration
```

### Terminal Applications
- **saorsa-terminal-chat**: CLI chat application
  - Real-time messaging
  - Three-word addressing
  - Encrypted communication
- **saorsa-network-tester**: Network testing utility
  - Performance benchmarks
  - Stress testing
  - Network diagnostics

### Communitas (Experimental)
- Tauri v2 diagnostic chat application
- Identity system integration
- DHT integration experiments

## Documentation (`docs/`)

### Categories
- **architecture/**: System design documents
- **api/**: API reference documentation
- **deployment/**: Deployment guides
- **development/**: Development guides
- **examples/**: Code examples
- **guides/**: User guides
- **network/**: Network protocols
- **runbooks/**: Operational procedures
- **security/**: Security documentation

### Key Documents
- PROJECT_SUMMARY.md: High-level overview
- API_REFERENCE.md: Complete API documentation
- DEPLOYMENT_GUIDE.md: Production deployment
- TROUBLESHOOTING_GUIDE.md: Common issues
- MIGRATION_GUIDE.md: Version migration

## Scripts & Tools

### Build Scripts
- test-runner.sh: Comprehensive test runner
- build_terminal_apps.sh: Build terminal applications
- package_for_distribution.sh: Package for release

### Development Tools
- .github/workflows/: CI/CD pipelines
- .claude/: AI development configuration
- .serena/: MCP tool configuration

## Testing Structure

### Unit Tests
- Located within each crate's `src/` directory
- Use `#[cfg(test)]` modules
- Run with `cargo test`

### Integration Tests
- Located in `crates/p2p-integration-tests/`
- Test cross-crate functionality
- Network simulation tests

### Test Suite
- `crates/ant-test-suite/`: Comprehensive testing
- Performance benchmarks
- Stress tests
- Security audits

## Configuration Files

### Workspace Level
- Cargo.toml: Workspace members and dependencies
- .gitignore: Git ignore patterns
- rustfmt.toml: Code formatting rules
- clippy.toml: Linting configuration

### Application Level
- tauri.conf.json: Tauri app configuration
- package.json: Node.js dependencies
- tsconfig.json: TypeScript configuration

Last Updated: 2025-08-06