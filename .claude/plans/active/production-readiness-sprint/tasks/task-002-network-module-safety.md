# Task 002: Network Module Error Handling

## Overview
Replace all unwrap/expect/panic instances in the network module with proper error handling. This is a critical module that handles all peer connections and must never panic in production.

## Acceptance Criteria
- [ ] Zero unwrap() calls in crates/p2p-core/src/network/
- [ ] Zero expect() calls in production code paths
- [ ] All functions return Result types where failures are possible
- [ ] Comprehensive error context for debugging
- [ ] All existing tests still pass

## Technical Details

### 1. Module Analysis
Files to update:
- `network/mod.rs` - Core network types and traits
- `network/connection.rs` - Connection management
- `network/peer.rs` - Peer handling
- `network/discovery.rs` - Peer discovery
- `transport/quic.rs` - QUIC transport layer
- `transport/tcp.rs` - TCP fallback

### 2. Common Patterns to Apply

#### Connection Handling
```rust
// Before
let conn = self.connections.get(&peer_id).unwrap();

// After
let conn = self.connections
    .get(&peer_id)
    .ok_or_else(|| NetworkError::PeerNotConnected(peer_id.clone()))?;
```

#### Async Operations
```rust
// Before
let result = connection.send(data).await.unwrap();

// After
let result = tokio::time::timeout(
    self.config.send_timeout,
    connection.send(data)
)
.await
.map_err(|_| NetworkError::Timeout { 
    operation: "send",
    duration: self.config.send_timeout 
})?
.context("Failed to send data to peer")?;
```

### 3. Specific Areas of Focus
- Connection pool management
- Socket binding and listening
- Peer discovery broadcasts
- Message serialization/deserialization
- Transport negotiation

### 4. Error Recovery Strategies
- Automatic reconnection with exponential backoff
- Graceful degradation when peers disconnect
- Circuit breaker pattern for failing peers
- Connection pool cleanup on errors

## Testing Requirements
- Unit tests for each error condition
- Integration tests simulating network failures
- Stress tests with connection churn
- Verify no panics under adverse conditions

## Dependencies
- Previous: Task 001 (Error Framework)
- Blocks: Task 010 (Integration Tests)

## Time Estimate
- Implementation: 8 hours
- Testing: 4 hours
- Review and refinement: 2 hours
- Total: 14 hours

## Definition of Done
- [ ] All unwrap/expect removed from network module
- [ ] Error handling patterns consistently applied
- [ ] Tests cover all error paths
- [ ] No performance regression in benchmarks
- [ ] Code review completed