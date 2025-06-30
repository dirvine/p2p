# Changelog

All notable changes to Saorsa will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **DHT-Based Identity Management**: Complete overhaul of identity system to use distributed hash table storage
  - Network-wide identity persistence and verification
  - Real DHT integration replacing local-only storage placeholders
  - Cross-node identity discovery and lookup functionality
  
- **Three-Word Address Resolution**: Full DHT-based three-word address system
  - Register and resolve human-readable network addresses
  - Network-wide address verification and conflict resolution
  - Cross-node address lookup functionality

- **Comprehensive Multi-Node Testing**: Extensive test suite for DHT identity functionality
  - Cross-node identity lookup tests
  - Three-word address resolution tests
  - Network identity discovery tests
  - Concurrent identity operations testing
  - Identity persistence across network changes

- **New Tauri Commands**:
  - `resolve_three_word_address_command`: Resolve three-word addresses to user IDs
  - `lookup_user_by_id`: Look up user profiles by ID across the network
  - `search_network_users`: Search for users across the distributed network

### Changed
- **Identity Storage**: Migrated from local storage to DHT-based network storage
  - Identity creation now publishes to DHT network
  - Profile updates propagate across all network nodes
  - Identity verification now happens network-wide

- **Network Identity Management**: Enhanced with proper P2P network integration
  - Real cryptographic identity creation with network verification
  - Improved identity persistence and availability
  - Better error handling and fallback mechanisms

### Technical Details
- Added `publish_identity_to_dht()` helper function for network identity storage
- Added `register_three_word_address()` for DHT-based address registration
- Added `lookup_user_identity()` for cross-node identity retrieval
- Added `resolve_three_word_address()` for network address resolution
- Enhanced error handling with proper DHT error propagation
- Integrated with existing mobile lifecycle management

### Testing
- Added comprehensive multi-node test framework (`identity_dht_integration_tests.rs`)
- Added Saorsa-specific identity integration tests (`identity_integration_tests.rs`)
- Test coverage for all new DHT identity functionality
- Performance and concurrent operation testing

## [0.2.7] - Previous Release
- Tauri v2.6.2 compatibility updates
- Mobile support infrastructure
- Basic identity management foundation