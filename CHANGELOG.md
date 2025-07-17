# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Banner image to README.md for enhanced visual appeal
- Comprehensive secure user authentication documentation (SECURE_USER_AUTH.md)
- Four-word networking address system (upgraded from three-word)

### Changed
- Migrated from three-word to four-word networking addresses throughout codebase
- Updated Rust edition to 2024 in core modules
- Refactored bootstrap discovery with improved connection handling
- Enhanced DHT node discovery and connection mechanisms

### Fixed
- Bootstrap node connection reliability improvements
- Network address encoding consistency

## Previous Changes

### [0.1.0] - Previous Release
- Initial P2P Foundation implementation with quantum-resistant cryptography
- QUIC transport layer with automatic fallback
- DHT implementation with Git-like versioning
- MCP integration for AI-native capabilities
EOF < /dev/null