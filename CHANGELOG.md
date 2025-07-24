# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Banner image to README.md for enhanced visual appeal
- Comprehensive secure user authentication documentation (SECURE_USER_AUTH.md)
- Four-word networking address system (upgraded from three-word)
- Kiro project steering documentation (.kiro/steering/)
- Deprecation notice for p2p-ffi crate with migration guidance
- Comprehensive documentation structure in docs/ directory
- Documentation index (docs/README.md) for easier navigation
- Adaptive network architecture research documentation (docs/network/)
- Research focus areas section highlighting experimental nature

### Changed
- Migrated from three-word to four-word networking addresses throughout codebase
- Updated Rust edition to 2024 in core modules
- Refactored bootstrap discovery with improved connection handling
- Enhanced DHT node discovery and connection mechanisms
- Focused project architecture on Tauri as the sole cross-platform framework
- Simplified build system by removing Flutter-specific configurations
- Updated documentation to emphasize Tauri for desktop, mobile, and web development
- Streamlined CI/CD workflows to remove Flutter build steps
- Reorganized all documentation into categorized subdirectories
- Repositioned project as experimental research rather than production-ready
- Updated language throughout to reflect research and development focus

### Removed
- Flutter references from all documentation and code comments
- Flutter-specific build commands and configurations
- ant-connect Flutter app from active development
- Flutter SDK requirements from development prerequisites
- Flutter FFI bindings from core library exports
- Redundant terminal application summary files
- Duplicate CLAUDE.md from docs directory
- Production-ready and revolutionary language from documentation

### Deprecated
- p2p-ffi crate (marked for removal in next major release)

### Fixed
- Bootstrap node connection reliability improvements
- Network address encoding consistency
- Documentation consistency across all markdown files

## Previous Changes

### [0.1.0] - Previous Release
- Initial P2P Foundation implementation with quantum-resistant cryptography
- QUIC transport layer with automatic fallback
- DHT implementation with Git-like versioning
- MCP integration for AI-native capabilities
EOF < /dev/null