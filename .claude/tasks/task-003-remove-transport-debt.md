# Task 3: Remove Transport Layer Technical Debt

## Overview
Remove the incomplete ant-quic transport implementation and ensure robust fallback mechanisms.

## Context
- **Phase**: Critical Error Handling (Week 1-2)
- **Priority**: CRITICAL
- **Impact**: Core networking functionality incomplete
- **Files**: `crates/p2p-core/src/transport/ant_quic.rs`

## Requirements
1. Remove ant-quic transport code
2. Update transport abstraction
3. Ensure fallback mechanisms work
4. Add transport selection tests

## Technical Specification
- Remove `ant_quic.rs` and related imports
- Update `transport/mod.rs` to exclude ant-quic
- Verify quinn transport is properly configured
- Add automatic fallback from QUIC to TCP
- Implement transport health monitoring

## Code Changes
```rust
// Remove from transport/mod.rs
// mod ant_quic;
// pub use ant_quic::*;

// Update transport selection logic
// Ensure quinn is default QUIC implementation
```

## Acceptance Criteria
- [ ] ant_quic.rs removed from codebase
- [ ] All ant_quic imports removed
- [ ] Transport tests pass without ant_quic
- [ ] Fallback mechanism documented
- [ ] Transport selection logic tested
- [ ] Performance not degraded

## Dependencies
- None - can be done in parallel with error handling

## Testing
- Transport selection tests
- Fallback scenario tests
- Performance benchmarks
- Connection reliability tests