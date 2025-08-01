# Task 4: Implement Identity Encryption

## Overview
Implement proper encryption for identity storage to replace plaintext storage.

## Context
- **Phase**: Security Hardening (Week 2-3)
- **Priority**: CRITICAL
- **Impact**: Identity data stored in plaintext
- **Files**: `crates/p2p-core/src/identity_manager.rs`

## Requirements
1. Add ChaCha20Poly1305 encryption
2. Implement secure key derivation
3. Add key rotation support
4. Create migration for existing data

## Technical Specification
```rust
// Current code has TODOs:
let encrypted_identity = identity_data; // TODO: Encrypt
let encrypted_keys = key_data; // TODO: Encrypt

// Need to implement:
- Use ChaCha20Poly1305 for encryption
- Argon2id for key derivation
- Secure key storage with OS keyring
- Automatic key rotation
```

## Implementation Details
- Use existing `encrypted_key_storage.rs` patterns
- Leverage `key_derivation.rs` for master keys
- Add versioning for future migrations
- Implement zero-copy encryption where possible

## Acceptance Criteria
- [ ] Identity data encrypted at rest
- [ ] Key derivation implemented with Argon2id
- [ ] Key rotation mechanism in place
- [ ] Migration tool for existing data
- [ ] Performance impact < 10ms
- [ ] Security tests pass

## Dependencies
- Task 1: Error Handling Framework (for proper error types)

## Testing
- Encryption/decryption round trips
- Key rotation scenarios
- Migration testing
- Performance benchmarks
- Security audit tests