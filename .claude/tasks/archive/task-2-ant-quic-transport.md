# Task 2: ant-quic Transport Layer Integration

## Task Status
- **Status**: 🟡 In Progress
- **Priority**: Critical
- **Started**: 2025-07-28
- **Assigned**: Claude (Orchestrator)
- **Estimated**: 4 days

## Context Loaded
- **Specification**: P2P Foundation Specification v4
- **Design**: P2P Foundation Design Document
- **Tech Stack**: Rust, ant-quic, Tokio, Ed25519
- **Standards**: TDD, >80% test coverage, property-based testing

## Acceptance Criteria
- [ ] Complete ant-quic integration with raw key auth
- [ ] Implement coordinator role configuration
- [ ] Add NAT type detection
- [ ] Create connection pool management
- [ ] Implement retry logic with backoff
- [ ] Add connection quality monitoring

## Tests Required
- Unit test: Direct connection establishment
- Unit test: Coordinator-assisted connection
- Integration test: Various NAT type combinations
- Property test: Connection symmetry
- Stress test: 1000 concurrent connections

## Implementation Structure
```rust
// Key structures to implement
pub struct QuicTransport {
    config: QuicConfig,
    identity: NodeIdentity,
    connections: Arc<RwLock<ConnectionPool>>,
    nat_type: NatType,
    coordinator: Option<CoordinatorInfo>,
}

pub enum NatType {
    Open,
    FullCone,
    RestrictedCone,
    PortRestricted,
    Symmetric,
}

pub struct ConnectionPool {
    active: HashMap<NodeId, Connection>,
    max_connections: usize,
    idle_timeout: Duration,
}
```

## TDD Tests to Write First

### 1. Direct Connection Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_connection_establishment() {
        // Test basic connection between two nodes
        let node1 = create_test_node(9001).await;
        let node2 = create_test_node(9002).await;
        
        let conn = node1.connect(&node2.identity).await.unwrap();
        assert!(conn.is_connected());
        
        // Test bidirectional communication
        let msg = b"hello";
        conn.send(msg).await.unwrap();
        let received = node2.recv().await.unwrap();
        assert_eq!(received, msg);
    }

    #[tokio::test]
    async fn test_raw_key_authentication() {
        // Test that connections use Ed25519 keys directly
        let node1 = create_test_node(9003).await;
        let node2 = create_test_node(9004).await;
        
        // Connection should verify peer's public key
        let conn = node1.connect(&node2.identity).await.unwrap();
        assert_eq!(conn.peer_key(), node2.identity.verification_key());
        
        // Test rejection of invalid keys
        let fake_identity = NodeIdentity::generate().unwrap();
        let err = node1.connect(&fake_identity).await.unwrap_err();
        assert!(matches!(err, TransportError::AuthenticationFailed));
    }
}
```

### 2. NAT Detection Tests
```rust
#[test]
fn test_nat_type_detection() {
    // Test detection logic for different NAT types
    let open_responses = vec![
        StunResponse { mapped_addr: "1.2.3.4:5000", changed_addr: None },
        StunResponse { mapped_addr: "1.2.3.4:5000", changed_addr: Some("1.2.3.4:5000") },
    ];
    assert_eq!(detect_nat_type(&open_responses), NatType::Open);
    
    let symmetric_responses = vec![
        StunResponse { mapped_addr: "1.2.3.4:5000", changed_addr: None },
        StunResponse { mapped_addr: "1.2.3.4:5001", changed_addr: None },
    ];
    assert_eq!(detect_nat_type(&symmetric_responses), NatType::Symmetric);
}

#[tokio::test]
async fn test_nat_traversal_strategies() {
    // Test different traversal strategies based on NAT combinations
    let strategies = get_traversal_strategies(NatType::FullCone, NatType::Symmetric);
    assert!(strategies.contains(&TraversalMethod::Coordinator));
    assert!(!strategies.contains(&TraversalMethod::Direct));
}
```

### 3. Coordinator-Assisted Connection Tests
```rust
#[tokio::test]
async fn test_coordinator_assisted_connection() {
    // Set up coordinator node
    let coordinator = create_coordinator_node(9010).await;
    
    // Create two nodes behind NAT
    let node1 = create_nat_node(9011, NatType::Symmetric).await;
    let node2 = create_nat_node(9012, NatType::Symmetric).await;
    
    // Register with coordinator
    node1.register_with_coordinator(&coordinator).await.unwrap();
    node2.register_with_coordinator(&coordinator).await.unwrap();
    
    // Establish connection through coordinator
    let conn = node1.connect_via_coordinator(&node2.identity, &coordinator)
        .await.unwrap();
    assert!(conn.is_connected());
}

#[tokio::test]
async fn test_coordinator_hole_punching() {
    // Test STUN-like hole punching coordination
    let coordinator = create_coordinator_node(9020).await;
    let node1 = create_nat_node(9021, NatType::RestrictedCone).await;
    let node2 = create_nat_node(9022, NatType::RestrictedCone).await;
    
    // Coordinator should facilitate simultaneous open
    let (conn1, conn2) = coordinator.coordinate_connection(&node1, &node2)
        .await.unwrap();
    
    assert!(conn1.is_connected());
    assert!(conn2.is_connected());
}
```

### 4. Connection Pool Tests
```rust
#[tokio::test]
async fn test_connection_pool_management() {
    let pool = ConnectionPool::new(100, Duration::from_secs(300));
    let node = create_test_node(9030).await;
    
    // Test connection reuse
    let peer_id = NodeId::from_bytes([1u8; 32]);
    let conn1 = pool.get_or_create(peer_id, || node.connect(peer_id)).await.unwrap();
    let conn2 = pool.get_or_create(peer_id, || node.connect(peer_id)).await.unwrap();
    assert_eq!(conn1.id(), conn2.id()); // Same connection reused
    
    // Test pool limits
    for i in 0..101 {
        let peer_id = NodeId::from_bytes([i as u8; 32]);
        pool.get_or_create(peer_id, || node.connect(peer_id)).await.unwrap();
    }
    assert_eq!(pool.active_connections(), 100); // Limited to max
}

#[tokio::test]
async fn test_connection_idle_timeout() {
    let pool = ConnectionPool::new(10, Duration::from_millis(100));
    let conn = pool.get_or_create(peer_id, connect_fn).await.unwrap();
    
    // Connection should be active
    assert!(pool.has_connection(&peer_id));
    
    // Wait for idle timeout
    tokio::time::sleep(Duration::from_millis(150)).await;
    pool.cleanup_idle().await;
    
    assert!(!pool.has_connection(&peer_id));
}
```

### 5. Retry Logic Tests
```rust
#[tokio::test]
async fn test_connection_retry_with_backoff() {
    let mut attempt = 0;
    let connect_fn = || async {
        attempt += 1;
        if attempt < 3 {
            Err(TransportError::ConnectionFailed)
        } else {
            Ok(create_mock_connection())
        }
    };
    
    let retry_config = RetryConfig {
        max_attempts: 5,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(10),
        multiplier: 2.0,
    };
    
    let start = Instant::now();
    let conn = retry_with_backoff(connect_fn, retry_config).await.unwrap();
    let elapsed = start.elapsed();
    
    // Should succeed on 3rd attempt after ~300ms (100ms + 200ms)
    assert!(elapsed >= Duration::from_millis(300));
    assert!(elapsed < Duration::from_millis(400));
}
```

### 6. Connection Quality Monitoring Tests
```rust
#[tokio::test]
async fn test_connection_quality_metrics() {
    let monitor = ConnectionMonitor::new();
    let conn = create_test_connection().await;
    
    // Simulate various quality conditions
    monitor.record_rtt(&conn, Duration::from_millis(50));
    monitor.record_packet_loss(&conn, 0.01); // 1% loss
    monitor.record_bandwidth(&conn, 10_000_000); // 10 Mbps
    
    let quality = monitor.get_quality(&conn);
    assert_eq!(quality.rating, QualityRating::Good);
    
    // Test quality degradation
    for _ in 0..10 {
        monitor.record_rtt(&conn, Duration::from_millis(500));
        monitor.record_packet_loss(&conn, 0.15); // 15% loss
    }
    
    let quality = monitor.get_quality(&conn);
    assert_eq!(quality.rating, QualityRating::Poor);
}
```

## Implementation Steps

### Step 1: Basic ant-quic Integration
1. Create `QuicTransport` wrapper around ant-quic
2. Implement raw Ed25519 key authentication
3. Basic connection establishment

### Step 2: NAT Detection
1. Implement STUN-like protocol for NAT detection
2. Create NAT type classification logic
3. Add detection caching

### Step 3: Coordinator Role
1. Define coordinator protocol messages
2. Implement registration and discovery
3. Add hole-punching coordination

### Step 4: Connection Pool
1. Create connection pool with limits
2. Implement connection reuse
3. Add idle timeout management

### Step 5: Retry Logic
1. Implement exponential backoff
2. Add jitter to prevent thundering herd
3. Create configurable retry policies

### Step 6: Quality Monitoring
1. Track RTT, packet loss, bandwidth
2. Create quality scoring algorithm
3. Add adaptive behavior based on quality

## Notes
- Use ant-quic's raw key feature to avoid certificate overhead
- Coordinator should be optional (direct connections when possible)
- Consider using libp2p's NAT detection logic as reference
- Ensure all connections are properly cleaned up
- Add comprehensive logging for debugging NAT issues