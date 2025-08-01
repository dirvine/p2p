# Task 2: Fix High-Risk unwrap() Calls

## Overview
Identify and fix unwrap() calls in critical paths including network operations, cryptographic operations, and storage operations.

## Context
- **Phase**: Critical Error Handling (Week 1-2)
- **Priority**: CRITICAL
- **Impact**: Direct cause of production crashes
- **Count**: ~568 unwrap() calls to review

## Requirements
1. Fix unwrap() in network paths (transport, connections)
2. Fix unwrap() in cryptographic operations
3. Fix unwrap() in storage operations
4. Add tests for error conditions

## Technical Specification
- Replace unwrap() with ? operator where possible
- Use expect() with descriptive messages for truly impossible cases
- Add proper error handling with context
- Ensure all errors are logged appropriately

## Focus Areas
- `crates/p2p-core/src/transport/`
- `crates/p2p-core/src/adaptive/`
- `crates/p2p-core/src/identity/`
- `crates/p2p-core/src/crypto_verify.rs`
- `crates/p2p-core/src/encrypted_key_storage.rs`

## Acceptance Criteria
- [ ] Zero unwrap() in transport layer
- [ ] Zero unwrap() in crypto operations
- [ ] Zero unwrap() in storage layer
- [ ] All replacements have proper error context
- [ ] Tests added for error paths
- [ ] Clippy rule added to prevent new unwrap()

## Dependencies
- Task 1: Error Handling Framework

## Testing
- Error injection tests
- Network failure scenarios
- Crypto operation failures
- Storage permission issues