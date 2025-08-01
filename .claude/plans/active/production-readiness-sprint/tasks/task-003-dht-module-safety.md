# Task 003: DHT Module Error Handling

## Overview
Implement comprehensive error handling in the DHT module, ensuring that missing keys, storage failures, and routing errors are handled gracefully without panics.

## Acceptance Criteria
- [ ] Zero unwrap/expect in DHT operations
- [ ] Graceful handling of missing keys
- [ ] Proper error propagation with context
- [ ] Partial failure recovery implemented
- [ ] All DHT tests pass with new error handling

## Technical Details

### 1. Files to Update
- `dht/mod.rs` - Core DHT types
- `dht/skademlia.rs` - Kademlia implementation
- `dht/storage.rs` - Storage backend
- `dht/routing.rs` - Routing table management
- `dht/replication.rs` - Data replication

### 2. Key Error Scenarios

#### Key Not Found
```rust
// Before
let value = self.storage.get(&key).unwrap();

// After
let value = self.storage
    .get(&key)
    .map_err(|e| DhtError::Storage(e))?
    .ok_or_else(|| DhtError::KeyNotFound(key.clone()))?;
```

#### Routing Failures
```rust
// Before
let closest_peers = self.routing_table.find_closest(&key).unwrap();

// After
let closest_peers = self.routing_table
    .find_closest(&key)
    .ok_or_else(|| DhtError::EmptyRoutingTable)?;

// Handle case where we have some peers but not K peers
if closest_peers.len() < K_VALUE {
    log::warn!("Only found {} peers, expected {}", closest_peers.len(), K_VALUE);
}
```

### 3. Storage Error Handling
- Implement retry logic for transient storage failures
- Add corruption detection and recovery
- Handle disk space errors gracefully
- Implement write-ahead logging for crash recovery

### 4. Replication Strategy
- Continue operation with reduced replication factor
- Track unhealthy replicas
- Implement healing process for under-replicated data
- Add metrics for replication health

## Testing Requirements
- Test DHT operations with failing storage backend
- Simulate network partitions
- Test with corrupted routing table
- Verify data availability with node failures

## Dependencies
- Previous: Task 001 (Error Framework)
- Related: Task 002 (Network errors may propagate here)
- Blocks: Task 008 (Identity DHT storage)

## Time Estimate
- Implementation: 10 hours
- Testing: 4 hours
- Integration: 2 hours
- Total: 16 hours

## Definition of Done
- [ ] No unwrap/expect in DHT module
- [ ] All error conditions have tests
- [ ] Partial failures don't cascade
- [ ] Performance metrics show no regression
- [ ] Documentation updated with failure modes