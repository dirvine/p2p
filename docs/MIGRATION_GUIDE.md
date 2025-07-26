# P2P Foundation Documentation Migration Guide

This guide explains the documentation reorganization completed on July 26, 2025.

## What Changed

### Moved to Archive
The following outdated documentation has been moved to `docs/archive/old-specs/`:
- `adaptive-p2p-overview.md` - Superseded by comprehensive network docs
- `DHT_STORAGE_SPECIFICATION.md` - Old DHT spec without adaptive features
- `dht_storage_detailed_impl_spec.md` - Detailed implementation of old design
- `git_like_content_addressed_dht_storage.md` - Old storage design
- `three-word-addresses.md` - We now use four-word addresses

### Updated Documentation
- `docs/architecture/SPECIFICATION.md` - Now reflects adaptive network architecture
- `README.md` - Updated to emphasize adaptive P2P network features

### Canonical Documentation
The authoritative documentation for the adaptive network is now:
- `/docs/network/overview.md` - High-level conceptual overview
- `/docs/network/specification.md` - Detailed technical specification
- `/docs/network/design.md` - Implementation design document

## Key Architectural Changes

### From Simple P2P to Adaptive Network
The project has evolved from a basic P2P network to a sophisticated adaptive system:

1. **Single DHT → Multi-Layer Routing**
   - Secure Kademlia remains the foundation
   - Added hyperbolic geometry routing
   - Added self-organizing maps
   - Multiple strategies work in parallel

2. **Static Parameters → Machine Learning**
   - Thompson Sampling for routing optimization
   - Q-Learning for cache management
   - LSTM networks for churn prediction

3. **Basic Security → Trust System**
   - EigenTrust++ for distributed reputation
   - Trust influences all routing decisions
   - Sybil resistance through proof-of-work

4. **Three-Word → Four-Word Addresses**
   - Using four-word-networking crate
   - Better collision resistance
   - Consistent with industry direction

## Implementation Status

The adaptive network is currently being implemented. See:
- `.claude/tasks/p2p-foundation-implementation.md` - Full implementation plan
- Task 0: Documentation cleanup (completed)
- Tasks 1-12: Core implementation (in progress)

## For Developers

### Code Structure (Planned)
```
crates/p2p-core/src/
├── identity/          # Four-word addresses, PoW
├── transport/         # ant-quic integration
├── layers/           # All network layers
│   ├── kademlia/     # Secure DHT
│   ├── hyperbolic/   # Geometry routing
│   ├── som/          # Self-organizing maps
│   ├── trust/        # EigenTrust++
│   └── gossip/       # Adaptive GossipSub
└── learning/         # ML systems
```

### Breaking Changes
- Node IDs now require proof-of-work
- Routing APIs will support multiple strategies
- Trust scores affect all operations
- Configuration includes layer-specific settings

## Questions?

For questions about the new architecture:
1. Read the network documentation in `/docs/network/`
2. Check the implementation plan
3. Review the updated specification

The adaptive network represents a significant advancement in P2P technology, combining proven distributed systems techniques with modern machine learning.