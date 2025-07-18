# Design Document

## Overview

This design outlines the systematic removal of Flutter references from the P2P Foundation codebase and documentation, replacing them with a unified Tauri-based cross-platform strategy. The refactoring will update documentation, code structure, and build processes to reflect the current architectural decisions.

## Architecture

### Current State Analysis

The project currently contains mixed references to both Flutter and Tauri:
- Documentation mentions Flutter FFI bindings for mobile development
- Cargo workspace includes `crates/p2p-ffi` for Flutter integration
- README and specifications reference hybrid Tauri + Flutter architecture
- Steering documents mention Flutter as a mobile solution

### Target State

After refactoring, the project will have:
- Pure Tauri-based cross-platform architecture
- Unified development workflow using Tauri for all platforms
- Clean documentation reflecting current technology choices
- Streamlined codebase without deprecated Flutter components

## Components and Interfaces

### Documentation Updates

#### README.md Changes
- Remove "Hybrid Architecture: Tauri Desktop + Flutter Mobile/Web" references
- Update mobile platform support to indicate Tauri mobile capabilities
- Remove Flutter FFI binding mentions
- Update quick start examples to focus on Tauri development
- Revise architecture diagrams to show pure Tauri stack

#### Technical Specification Updates
- Remove Flutter from system architecture diagrams
- Update cross-platform support section to focus on Tauri
- Remove Flutter FFI API examples
- Update development workflow to be Tauri-centric

#### Steering Documents Updates
- Update `tech.md` to remove Flutter references
- Revise platform support to emphasize Tauri's capabilities
- Update build commands to remove Flutter-specific steps
- Modify structure documentation to reflect current codebase

### Code Structure Changes

#### Workspace Cleanup
- Remove or deprecate `crates/p2p-ffi` from workspace members
- Clean up Flutter-specific dependencies from workspace Cargo.toml
- Update workspace package metadata to reflect Tauri focus
- Remove Flutter build profiles and configurations

#### Source Code Updates
- Remove Flutter FFI exports from `crates/p2p-core/src/lib.rs`
- Update module documentation to remove Flutter examples
- Clean up comments referencing Flutter integration
- Remove Flutter-specific conditional compilation flags

### Build System Updates

#### Script Modifications
- Remove Flutter build commands from build scripts
- Update distribution scripts to focus on Tauri packaging
- Clean up CI/CD references to Flutter toolchain
- Modify development setup instructions

## Data Models

### Documentation Structure
```
docs/
├── README.md (updated - no Flutter)
├── SPECIFICATION.md (updated - Tauri focus)
└── platform-support.md (new - Tauri capabilities)
```

### Workspace Structure
```
Cargo.toml (updated workspace members)
├── crates/p2p-core/ (cleaned exports)
├── crates/p2p-cli/ (unchanged)
├── apps/saorsa/ (Tauri focus)
└── [removed: crates/p2p-ffi/]
```

## Error Handling

### Migration Risks
- **Broken Links**: Documentation updates may create broken internal references
- **Build Failures**: Removing workspace members may cause build issues
- **Developer Confusion**: Rapid changes may confuse contributors

### Mitigation Strategies
- **Incremental Updates**: Update documentation in logical groups
- **Build Validation**: Test builds after each major change
- **Clear Communication**: Document the architectural decision rationale

## Testing Strategy

### Validation Approach
1. **Documentation Review**: Ensure all Flutter references are removed
2. **Build Testing**: Verify workspace builds successfully after changes
3. **Link Checking**: Validate all internal documentation links work
4. **Example Verification**: Ensure code examples compile and run

### Test Cases
- Build workspace with `cargo build --all`
- Run documentation link checker
- Verify Tauri app builds successfully
- Test that removed FFI crate doesn't break dependencies

## Implementation Phases

### Phase 1: Documentation Updates
- Update README.md and main documentation files
- Revise steering documents
- Update technical specifications

### Phase 2: Code Structure Cleanup
- Remove or deprecate p2p-ffi crate
- Clean up lib.rs exports and comments
- Update workspace configuration

### Phase 3: Build System Updates
- Update build scripts and CI/CD
- Clean up development setup instructions
- Test complete build pipeline

### Phase 4: Validation and Testing
- Comprehensive testing of all changes
- Documentation review and link validation
- Final build verification