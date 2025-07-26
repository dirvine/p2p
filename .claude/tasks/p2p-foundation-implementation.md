# Task: P2P Foundation Complete Implementation

## Specification
See: `.claude/tasks/p2p-foundation-specification-v4.md`

## Design
See: `.claude/tasks/p2p-foundation-design.md`

## Steering Context
- **Overview**: Adaptive P2P network with multiple routing layers
- **Tech Stack**: Rust, ant-quic, four-word-networking, Tokio
- **Standards**: TDD, >80% test coverage, property-based testing

## Implementation Tasks

### Task 0: Documentation Cleanup and Consolidation
**Status**: 🔴 Not Started
**Priority**: Critical
**Estimated**: 2 days

**Acceptance Criteria**:
- [ ] Audit all existing documentation files
- [ ] Remove outdated design docs and specifications
- [ ] Consolidate network layer docs into canonical location
- [ ] Update README with current architecture
- [ ] Create migration guide from old to new design
- [ ] Update all code comments referencing old design

**Tests Required**:
- Documentation link checker passes
- All examples in docs compile and run
- No references to deprecated components

**Implementation Notes**:
- Start with `find . -name "*.md" | grep -E "(old|deprecated|legacy)"`
- Check for conflicting information between docs
- Ensure docs/network/ contains only current design

---

### Task 1: Core Identity System with Four-Word Addresses
**Status**: 🔴 Not Started
**Priority**: Critical
**Estimated**: 3 days

**Acceptance Criteria**:
- [ ] Implement `NodeIdentity` with Ed25519 keys
- [ ] Integrate four-word-networking crate
- [ ] Generate deterministic four-word addresses from peer IDs
- [ ] Implement proof-of-work for Sybil resistance
- [ ] Create identity persistence and loading
- [ ] Add identity CLI commands

**Tests Required**:
- Property test: Same seed produces same identity
- Property test: Different seeds produce different addresses
- Unit test: PoW validation
- Integration test: Identity save/load cycle
- Benchmark: Identity generation time

**Implementation Notes**:
```rust
// Key structure to implement
pub struct NodeIdentity {
    signing_key: SigningKey,
    verification_key: VerifyingKey,
    node_id: NodeId,  // SHA-256(verification_key)
    word_address: FourWordAddress,
    proof_of_work: ProofOfWork,
}
```

---

### Task 2: ant-quic Transport Layer Integration
**Status**: 🔴 Not Started
**Priority**: Critical
**Estimated**: 4 days

**Acceptance Criteria**:
- [ ] Complete ant-quic integration with raw key auth
- [ ] Implement coordinator role configuration
- [ ] Add NAT type detection
- [ ] Create connection pool management
- [ ] Implement retry logic with backoff
- [ ] Add connection quality monitoring

**Tests Required**:
- Unit test: Direct connection establishment
- Unit test: Coordinator-assisted connection
- Integration test: Various NAT type combinations
- Property test: Connection symmetry
- Stress test: 1000 concurrent connections

**Implementation Notes**:
- Build on existing `crates/p2p-core/src/transport/quic.rs`
- Add raw key authentication to replace certificates
- Implement adaptive timeout based on network conditions

---

### Task 3: Secure Kademlia DHT Implementation
**Status**: 🟡 In Progress
**Priority**: High
**Estimated**: 5 days
**Started**: 2025-07-26

**Acceptance Criteria**:
- [ ] Implement k-buckets with k=20
- [ ] Add trust-weighted routing decisions
- [ ] Create XOR-based distance metric
- [ ] Implement FIND_NODE, FIND_VALUE, STORE operations
- [ ] Add message signing and verification
- [ ] Implement adaptive replication factor

**Tests Required**:
- Unit test: Routing table operations
- Property test: XOR metric properties
- Integration test: Multi-node DHT operations
- Chaos test: Node failures during operations
- Benchmark: Lookup performance at scale

**Implementation Notes**:
```rust
pub struct SecureKademlia {
    routing_table: Arc<RwLock<KBuckets>>,
    node_id: NodeId,
    trust_scores: Arc<RwLock<HashMap<NodeId, f64>>>,
}
```

---

### Task 4: Hyperbolic Geometry Routing Layer
**Status**: 🔴 Not Started
**Priority**: High
**Estimated**: 4 days

**Acceptance Criteria**:
- [ ] Implement Poincaré disk coordinate system
- [ ] Create greedy routing algorithm
- [ ] Add coordinate adjustment protocol
- [ ] Implement success rate tracking
- [ ] Create fallback to Kademlia on failure
- [ ] Add visualization for debugging

**Tests Required**:
- Unit test: Distance calculations
- Property test: Greedy routing convergence
- Integration test: Coordinate stability
- Simulation test: Various network topologies
- Benchmark: Routing speed vs Kademlia

**Implementation Notes**:
- Use fixed-point arithmetic for coordinate precision
- Implement community detection for angular positioning
- Add hysteresis to prevent coordinate oscillation

---

### Task 5: Self-Organizing Map (SOM) Implementation
**Status**: 🔴 Not Started
**Priority**: Medium
**Estimated**: 4 days

**Acceptance Criteria**:
- [ ] Implement multi-dimensional feature space
- [ ] Create SOM grid with dynamic sizing
- [ ] Add neuron weight updates with learning rate
- [ ] Implement best matching unit (BMU) search
- [ ] Create neighborhood function
- [ ] Add node assignment to neurons

**Tests Required**:
- Unit test: Feature normalization
- Property test: BMU consistency
- Integration test: Clustering quality
- Benchmark: Update performance
- Visualization: Cluster formation over time

**Implementation Notes**:
```rust
pub struct NodeFeatures {
    content_vector: Vec<f64>,      // 128-bit semantic hash
    compute_capability: f64,        // 0-1000 benchmark
    network_latency: f64,          // Average RTT
    storage_available: f64,        // GB available
}
```

---

### Task 6: EigenTrust++ Trust System
**Status**: 🔴 Not Started
**Priority**: High
**Estimated**: 4 days

**Acceptance Criteria**:
- [ ] Implement local trust calculation
- [ ] Create global trust vector computation
- [ ] Add pre-trusted node support
- [ ] Implement trust decay over time
- [ ] Create trust inheritance for new nodes
- [ ] Add trust-based routing weights

**Tests Required**:
- Unit test: Trust normalization
- Property test: Trust monotonicity
- Integration test: Trust propagation
- Attack test: Sybil resistance
- Benchmark: Convergence speed

**Implementation Notes**:
- Use sparse matrix for efficiency
- Implement power iteration with early termination
- Add checkpointing for trust state

---

### Task 7: Adaptive GossipSub Protocol
**Status**: 🔴 Not Started
**Priority**: Medium
**Estimated**: 3 days

**Acceptance Criteria**:
- [ ] Implement topic-based mesh construction
- [ ] Add adaptive mesh degree based on churn
- [ ] Create peer scoring system
- [ ] Implement message validation
- [ ] Add gossip factor adjustment
- [ ] Create topic prioritization

**Tests Required**:
- Unit test: Mesh maintenance
- Integration test: Message propagation
- Chaos test: Network partitions
- Benchmark: Propagation latency
- Load test: High message volume

**Implementation Notes**:
```rust
pub enum GossipTopic {
    TrustUpdate,
    CoordinateAdjust,
    SomPosition,
    ContentAnnounce,
    ComputeOffer,
    ChurnPredict,
}
```

---

### Task 8: Multi-Armed Bandit Routing Optimization
**Status**: 🔴 Not Started
**Priority**: Medium
**Estimated**: 3 days

**Acceptance Criteria**:
- [ ] Implement Thompson Sampling algorithm
- [ ] Track success/failure per route per content type
- [ ] Create Beta distribution sampling
- [ ] Add exploration vs exploitation balance
- [ ] Implement statistics persistence
- [ ] Create routing decision API

**Tests Required**:
- Unit test: Beta distribution sampling
- Property test: Exploration guarantees
- Integration test: Learning convergence
- A/B test: Performance vs random selection
- Benchmark: Decision overhead

---

### Task 9: Q-Learning Cache Management
**Status**: 🔴 Not Started
**Priority**: Medium
**Estimated**: 3 days

**Acceptance Criteria**:
- [ ] Implement Q-table for state-action values
- [ ] Create state representation (cache util, frequency, etc)
- [ ] Define action space (cache, evict, replicate)
- [ ] Implement reward function
- [ ] Add ε-greedy exploration
- [ ] Create experience replay buffer

**Tests Required**:
- Unit test: State discretization
- Property test: Q-value convergence
- Integration test: Cache hit rate improvement
- Benchmark: Decision latency
- Simulation: Various workload patterns

---

### Task 10: LSTM Churn Prediction
**Status**: 🔴 Not Started
**Priority**: Low
**Estimated**: 4 days

**Acceptance Criteria**:
- [ ] Implement LSTM model architecture
- [ ] Create feature extraction pipeline
- [ ] Add online learning with experience replay
- [ ] Implement prediction for 1h, 6h, 24h horizons
- [ ] Create proactive replication triggers
- [ ] Add model persistence and loading

**Tests Required**:
- Unit test: Feature extraction
- Integration test: Prediction accuracy
- Backtesting: Historical churn data
- A/B test: Proactive vs reactive replication
- Benchmark: Inference time

---

### Task 11: Full System Integration
**Status**: 🔴 Not Started
**Priority**: Critical
**Estimated**: 5 days

**Acceptance Criteria**:
- [ ] Integrate all layers through NetworkCoordinator
- [ ] Implement unified message routing
- [ ] Create layer coordination protocols
- [ ] Add comprehensive metrics collection
- [ ] Implement graceful degradation
- [ ] Create system health monitoring

**Tests Required**:
- Integration test: All layers active
- Chaos test: Random layer failures
- Load test: 10,000 nodes simulation
- End-to-end test: Real network deployment
- Performance test: Baseline metrics

---

### Task 12: Testing, Benchmarking, and Documentation
**Status**: 🔴 Not Started
**Priority**: High
**Estimated**: 5 days

**Acceptance Criteria**:
- [ ] Achieve >80% test coverage
- [ ] Create comprehensive benchmarks
- [ ] Add property-based test suite
- [ ] Write API documentation
- [ ] Create deployment guide
- [ ] Add troubleshooting guide

**Tests Required**:
- Coverage report shows >80%
- All benchmarks establish baselines
- Property tests cover core invariants
- Documentation examples compile
- Integration tests cover all scenarios

---

## Progress Tracking
- Total Tasks: 13 (including Task 0)
- Completed: 0
- In Progress: 1 (Task 3)
- Blocked: 0

## Dependencies
- Task 1 blocks all others (need identity)
- Task 2 blocks network operations
- Tasks 3-7 can be developed in parallel
- Tasks 8-10 depend on layers 3-7
- Task 11 requires all previous tasks
- Task 12 can start with any completed task

## Next Task
Recommended: **Task 0** - Documentation Cleanup (critical for clean slate)

## Success Metrics
- All tests passing with >80% coverage
- Connection establishment <400ms (99th percentile)
- Lookup success rate >99.5%
- Network handles 50% hourly churn
- Documentation complete and accurate