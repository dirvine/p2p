# Adaptive GossipSub Protocol Implementation

Date: July 26, 2025

## Overview

The P2P Foundation includes a comprehensive Adaptive GossipSub implementation that provides scalable pub/sub messaging with trust-based peer selection, adaptive parameters, and priority-based message handling.

## Implementation Status

### ✅ Core Features Already Implemented

The existing implementation in `crates/p2p-core/src/adaptive/gossip.rs` includes:

1. **Core GossipSub Protocol**
   - Topic subscription/unsubscription
   - Message publishing and propagation
   - Mesh-based peer management
   - Control messages (GRAFT, PRUNE, IHAVE, IWANT)
   - Message deduplication and caching

2. **Adaptive Features**
   - Dynamic mesh size based on churn rate
   - Topic priority levels (Critical, High, Normal, Low)
   - Trust-weighted peer scoring
   - Adaptive parameters per topic
   - Churn detection and tracking

3. **Trust Integration**
   - Uses EigenTrust++ scores for peer selection
   - Application-specific scores from trust system
   - Behavior penalties for protocol violations
   - Graylisting of low-scoring peers

4. **Performance Optimizations**
   - Message ID caching
   - Efficient heartbeat mechanism
   - Old message cleanup
   - Fanout management for non-subscribed topics

## Key Components

### Core Types

```rust
/// Main GossipSub implementation
pub struct AdaptiveGossipSub {
    local_id: NodeId,
    mesh: Arc<RwLock<HashMap<Topic, HashSet<NodeId>>>>,
    fanout: Arc<RwLock<HashMap<Topic, HashSet<NodeId>>>>,
    seen_messages: Arc<RwLock<HashMap<MessageId, Instant>>>,
    message_cache: Arc<RwLock<HashMap<MessageId, GossipMessage>>>,
    peer_scores: Arc<RwLock<HashMap<NodeId, PeerScore>>>,
    topics: Arc<RwLock<HashMap<Topic, TopicParams>>>,
    topic_priorities: Arc<RwLock<HashMap<Topic, TopicPriority>>>,
    heartbeat_interval: Duration,
    trust_provider: Arc<dyn TrustProvider>,
    churn_detector: Arc<RwLock<ChurnDetector>>,
    stats: Arc<RwLock<GossipStats>>,
}

/// Peer scoring
pub struct PeerScore {
    pub time_in_mesh: Duration,
    pub first_message_deliveries: u64,
    pub mesh_message_deliveries: u64,
    pub invalid_messages: u64,
    pub behavior_penalty: f64,
    pub app_specific_score: f64,
}

/// Topic configuration
pub struct TopicParams {
    pub d: usize,                    // Target degree: 8
    pub d_low: usize,                // Lower bound: 6
    pub d_high: usize,               // Upper bound: 12
    pub d_out: usize,                // Outbound degree: 2
    pub graylist_threshold: f64,     // Score threshold: -1.0
    pub gossip_factor: f64,          // IHave percentage: 0.25
    pub priority: TopicPriority,
}
```

### Control Messages

```rust
pub enum ControlMessage {
    Graft { topic: Topic },
    Prune { topic: Topic, backoff: Duration },
    IHave { topic: Topic, message_ids: Vec<MessageId> },
    IWant { message_ids: Vec<MessageId> },
}
```

### Adaptive Mesh Size Calculation

The mesh size adapts based on:

1. **Base Size**: 8 peers (configurable)
2. **Churn Factor**: Increases with network instability
3. **Priority Factor**: 
   - Critical: 2.0x
   - High: 1.5x
   - Normal: 1.0x
   - Low: 0.8x

```rust
mesh_size = base_size × (1 + churn_rate × 0.1) × priority_factor
```

### Peer Scoring

Composite score calculation:
```rust
score = time_score           // Time in mesh (max 10 points)
      + delivery_score       // First message deliveries
      + mesh_score          // Mesh message deliveries
      + invalid_penalty     // -10 per invalid message
      + behavior_penalty    // Decaying penalty
      + app_specific_score  // From trust system
```

## API Usage

### Creating GossipSub Instance

```rust
use p2p_core::adaptive::{AdaptiveGossipSub, TopicPriority};

// Create with trust provider
let gossip = AdaptiveGossipSub::new(local_id, trust_provider);

// Start heartbeat task
tokio::spawn(async move {
    loop {
        gossip.heartbeat().await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
});
```

### Topic Management

```rust
// Subscribe to topic
gossip.subscribe("network-announcements").await?;

// Set topic priority
gossip.set_topic_priority("critical-updates", TopicPriority::Critical).await;

// Unsubscribe
gossip.unsubscribe("old-topic").await?;
```

### Publishing Messages

```rust
let message = GossipMessage {
    topic: "announcements".to_string(),
    data: b"Hello network!".to_vec(),
    from: local_id,
    seqno: sequence_number,
    timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
};

gossip.publish("announcements", message).await?;
```

### Handling Control Messages

```rust
// Handle incoming control message
match control_msg {
    ControlMessage::Graft { topic } => {
        // Peer wants to join mesh
        gossip.handle_control_message(&from_peer, control_msg).await?;
    }
    ControlMessage::IHave { topic, message_ids } => {
        // Peer announcing messages
        gossip.handle_control_message(&from_peer, control_msg).await?;
    }
    // ... other messages
}
```

### Monitoring

```rust
// Get statistics
let stats = gossip.get_stats().await;
println!("Messages sent: {}", stats.messages_sent);
println!("Mesh size: {}", stats.mesh_size);
println!("Active topics: {}", stats.topic_count);

// Monitor churn
let churn_rate = gossip.churn_detector.read().await.get_rate();
if churn_rate > 0.5 {
    // High churn detected, reduce fanout
    gossip.reduce_fanout(0.5).await;
}
```

## Testing

The implementation includes comprehensive tests:

1. **Basic Operations** - Subscribe, unsubscribe, publish
2. **Peer Scoring** - Score calculation and penalties
3. **Message IDs** - Deterministic ID generation
4. **Adaptive Mesh** - Size calculation based on priority
5. **Churn Detection** - Rate calculation from events
6. **Control Messages** - GRAFT/PRUNE handling
7. **IHAVE/IWANT** - Message exchange flow

## Integration Points

### 1. With Trust System
- Pulls trust scores for peer selection
- Updates trust based on peer behavior
- Filters peers below trust threshold

### 2. With Network Layer
- Messages sent via transport layer
- Control messages use same infrastructure
- Bandwidth management integration

### 3. With Client API
- High-level publish/subscribe interface
- Stream-based message delivery
- Topic management

### 4. With Monitoring
- Exports metrics for dashboard
- Churn rate affects other components
- Performance tracking

## Performance Characteristics

1. **Message Propagation**: O(d) where d = mesh degree
2. **Heartbeat**: O(n×m) for n topics, m peers
3. **Message Deduplication**: O(1) hash lookup
4. **Peer Selection**: O(n log n) for scoring
5. **Memory**: O(m×p) for m messages, p peers

## Security Features

1. **Message Validation**
   - Signature verification
   - Timestamp checks
   - Size limits

2. **Peer Scoring**
   - Penalizes invalid messages
   - Rewards consistent delivery
   - Trust-based selection

3. **Attack Resistance**
   - Sybil resistance via trust system
   - Spam prevention via rate limiting
   - Eclipse resistance via peer diversity

## Task 7 Completion Summary

Task 7 (Adaptive GossipSub Protocol) is effectively complete as the implementation already exists and is comprehensive. The implementation includes:

1. ✅ Full GossipSub protocol with mesh management
2. ✅ Adaptive parameters based on network conditions
3. ✅ Trust-based peer scoring and selection
4. ✅ Topic priorities and dynamic mesh sizing
5. ✅ Churn detection and adaptation
6. ✅ Control message handling
7. ✅ Performance optimizations
8. ✅ Comprehensive test coverage

The Adaptive GossipSub provides efficient, scalable pub/sub messaging for the P2P network.

## Future Enhancement Opportunities

1. **Extended Validation**
   - Custom message validators per topic
   - Proof-of-work for critical messages
   - Content-based filtering

2. **Advanced Features**
   - Hierarchical topics
   - Selective message propagation
   - Gossip aggregation

3. **Performance Optimizations**
   - Batch control messages
   - Compressed message formats
   - Probabilistic message selection

4. **Monitoring & Debug**
   - Message flow visualization
   - Peer relationship graphs
   - Protocol analyzer tools

## Conclusion

The Adaptive GossipSub implementation provides a robust, scalable pub/sub system that adapts to network conditions while maintaining security through trust-based peer selection. It forms a critical component for network-wide communication in the P2P Foundation.