# Task 001: Error Handling Framework Setup

## Overview
Establish the foundational error handling framework that will be used throughout the codebase. This includes creating custom error types, establishing patterns, and setting up tooling for tracking progress.

## Acceptance Criteria
- [ ] Custom error types created for all major modules
- [ ] Error handling patterns documented
- [ ] Automated scanning script to find all unwrap/expect/panic instances
- [ ] Progress tracking dashboard set up
- [ ] All new error types compile without warnings

## Technical Details

### 1. Create Core Error Types
Location: `crates/p2p-core/src/error.rs`

- Define `P2PError` as the root error type using `thiserror`
- Create domain-specific error types: `NetworkError`, `DhtError`, `IdentityError`, `CryptoError`, `StorageError`
- Implement proper error conversions and context propagation
- Add validation error variants for input sanitization

### 2. Error Scanning Tool
Create `scripts/find_panics.sh`:
```bash
#!/bin/bash
echo "=== Panic-inducing code scan ==="
echo "unwrap() calls:"
rg "\.unwrap\(\)" --type rust -g '!tests/' -g '!benches/' | wc -l
echo "expect() calls:"
rg "\.expect\(" --type rust -g '!tests/' -g '!benches/' | wc -l
echo "panic! macros:"
rg "panic!\(" --type rust -g '!tests/' -g '!benches/' | wc -l
```

### 3. Module-Specific Error Types
- NetworkError: Connection failures, timeouts, invalid addresses
- DhtError: Key not found, storage failures, routing errors
- IdentityError: Crypto failures, validation errors, passkey errors
- StorageError: IO errors, corruption, space issues

### 4. Documentation
Create `docs/ERROR_HANDLING.md` with:
- Standard patterns for error handling
- When to use Result vs Option
- Context adding guidelines
- Logging best practices

## Testing Requirements
- Unit tests for error type conversions
- Integration test verifying no panics in critical paths
- Compile-time verification of error handling

## Dependencies
- Previous: None (first task)
- Blocks: All subsequent error handling tasks

## Time Estimate
- Implementation: 4 hours
- Testing: 2 hours
- Documentation: 1 hour
- Total: 7 hours

## Definition of Done
- [ ] All error types implemented and documented
- [ ] Scanning script operational and added to CI
- [ ] Documentation complete with examples
- [ ] Zero compilation warnings
- [ ] PR reviewed and merged