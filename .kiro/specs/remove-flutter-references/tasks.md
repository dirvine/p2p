# Implementation Plan

- [x] 1. Update steering documentation to remove Flutter references
  - Update `.kiro/steering/tech.md` to remove Flutter mentions and focus on Tauri
  - Update `.kiro/steering/structure.md` to reflect current codebase without FFI components
  - _Requirements: 1.4, 3.2_

- [x] 2. Update main project documentation files
  - [x] 2.1 Update README.md to remove Flutter references
    - Remove "Hybrid Architecture: Tauri Desktop + Flutter Mobile/Web" from Cargo.toml comments
    - Update mobile platform support section to focus on Tauri capabilities
    - Remove Flutter FFI binding mentions from core library description
    - Update quick start examples to be Tauri-focused
    - _Requirements: 1.1, 1.2, 3.1_

  - [x] 2.2 Update technical specification document
    - Remove Flutter from system architecture diagrams in docs/SPECIFICATION.md
    - Update cross-platform support section to emphasize Tauri
    - Remove Flutter FFI API examples and replace with Tauri integration examples
    - Update development workflow documentation to be Tauri-centric
    - _Requirements: 1.3, 3.2, 3.3_

- [x] 3. Clean up workspace configuration
  - [x] 3.1 Update Cargo.toml workspace members
    - Remove `crates/p2p-ffi` from workspace members list
    - Update workspace package description to remove Flutter references
    - Clean up any Flutter-specific dependencies from workspace dependencies
    - _Requirements: 2.1, 2.3_

  - [x] 3.2 Handle p2p-ffi crate
    - Either remove `crates/p2p-ffi` directory or add deprecation notice
    - Update any internal documentation referencing the FFI crate
    - _Requirements: 2.2_

- [x] 4. Update core library code and documentation
  - [x] 4.1 Clean up lib.rs exports and comments
    - Remove Flutter FFI exports from `crates/p2p-core/src/lib.rs`
    - Update module-level documentation to remove Flutter integration examples
    - Clean up any comments referencing Flutter development
    - _Requirements: 4.1, 4.2, 4.3_

  - [x] 4.2 Update code examples and inline documentation
    - Replace Flutter usage patterns in example code with Tauri examples
    - Update any conditional compilation flags that were Flutter-specific
    - Review and update API documentation to focus on Tauri integration
    - _Requirements: 4.4_

- [x] 5. Update build system and development workflow
  - [x] 5.1 Update build scripts and instructions
    - Remove Flutter build commands from build scripts like `BUILD_NOW.sh`
    - Update `apps/BUILD_INSTRUCTIONS.md` to remove Flutter prerequisites
    - Clean up any Flutter-related build profiles or configurations
    - _Requirements: 5.2, 5.3_

  - [x] 5.2 Update development setup documentation
    - Remove Flutter SDK from prerequisites in development documentation
    - Update development workflow to focus on Tauri development
    - Remove Flutter installation steps from setup instructions
    - _Requirements: 5.1, 5.4_

- [x] 6. Validation and testing
  - [x] 6.1 Test workspace builds successfully
    - Run `cargo build --all` to ensure workspace builds without errors
    - Test that Tauri app builds successfully with `cd apps/saorsa && cargo tauri build`
    - Verify no broken dependencies from removed FFI crate
    - _Requirements: All requirements validation_

  - [x] 6.2 Validate documentation consistency
    - Review all updated documentation for consistency and accuracy
    - Check that all internal links still work after documentation updates
    - Ensure code examples compile and run correctly
    - _Requirements: All requirements validation_