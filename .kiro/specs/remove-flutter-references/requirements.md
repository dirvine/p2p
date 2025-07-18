# Requirements Document

## Introduction

This specification defines the removal of Flutter references from the P2P Foundation codebase and documentation. The project is consolidating on Tauri as the primary cross-platform application framework, eliminating Flutter-based mobile development in favor of a unified Tauri approach for desktop, mobile, and web platforms.

## Requirements

### Requirement 1

**User Story:** As a developer working on the P2P Foundation, I want all documentation to accurately reflect the current technology stack, so that I understand the correct architecture and don't get confused by outdated Flutter references.

#### Acceptance Criteria

1. WHEN reviewing project documentation THEN all references to Flutter mobile development SHALL be removed
2. WHEN reading the README.md THEN the mobile platform support SHALL indicate Tauri-based solutions instead of Flutter FFI
3. WHEN examining the technical specification THEN Flutter SHALL NOT be mentioned as a supported platform
4. WHEN looking at steering documentation THEN Flutter SHALL be replaced with Tauri-focused mobile strategies

### Requirement 2

**User Story:** As a developer examining the codebase structure, I want the Cargo workspace to reflect only actively used components, so that I don't waste time on deprecated Flutter FFI bindings.

#### Acceptance Criteria

1. WHEN examining Cargo.toml workspace members THEN Flutter FFI crate references SHALL be removed
2. WHEN looking at the crates directory THEN p2p-ffi crate SHALL be removed or clearly marked as deprecated
3. WHEN reviewing workspace dependencies THEN Flutter-specific dependencies SHALL be removed
4. WHEN examining build scripts THEN Flutter build commands SHALL be removed

### Requirement 3

**User Story:** As a developer understanding the project architecture, I want clear documentation about Tauri's cross-platform capabilities, so that I understand how mobile and web support will be achieved.

#### Acceptance Criteria

1. WHEN reading platform support documentation THEN Tauri's mobile capabilities SHALL be clearly explained
2. WHEN examining the architecture diagrams THEN Flutter layers SHALL be replaced with Tauri architecture
3. WHEN reviewing cross-platform strategy THEN the unified Tauri approach SHALL be documented
4. WHEN looking at development workflows THEN Tauri-based mobile development SHALL be described

### Requirement 4

**User Story:** As a project maintainer, I want all code comments and inline documentation to be consistent with the current architecture, so that future developers aren't misled by outdated references.

#### Acceptance Criteria

1. WHEN examining Rust source code THEN Flutter FFI comments SHALL be removed or updated
2. WHEN reviewing lib.rs exports THEN Flutter-specific exports SHALL be removed
3. WHEN looking at module documentation THEN Flutter integration examples SHALL be replaced with Tauri examples
4. WHEN examining example code THEN Flutter usage patterns SHALL be removed

### Requirement 5

**User Story:** As a developer setting up the development environment, I want build instructions that only include necessary tools, so that I don't install unused Flutter toolchain components.

#### Acceptance Criteria

1. WHEN following setup instructions THEN Flutter installation SHALL NOT be required
2. WHEN examining prerequisites THEN Flutter SDK SHALL be removed from requirements
3. WHEN reviewing build commands THEN Flutter build steps SHALL be removed
4. WHEN looking at CI/CD configuration THEN Flutter-related jobs SHALL be removed or updated