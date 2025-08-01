# Task 005: Transport Module Error Handling

## Overview
Implement robust error handling in the transport layer (QUIC/TCP) to ensure connection failures, timeouts, and protocol errors are handled gracefully. This module is critical for network reliability.

## Acceptance Criteria
- [ ] Zero panics in QUIC and TCP transports
- [ ] Timeout handling on all operations
- [ ] Graceful fallback from QUIC to TCP
- [ ] Connection pooling errors handled
- [ ] All transport tests pass

## Technical Details

### 1. Files to Update
- `transport/mod.rs` - Transport traits and types
- `transport/quic.rs` - QUIC implementation
- `transport/tcp.rs` - TCP fallback
- `transport/connection_pool.rs` - Connection pooling
- `transport/quic_tests.rs` - Update tests

### 2. QUIC Error Handling

#### Connection Establishment
```rust
// Before
let connection = endpoint.connect(addr, "localhost").unwrap().await.unwrap();

// After
let connection = timeout(
    self.config.connection_timeout,
    endpoint.connect(addr, "localhost")?
)
.await
.map_err(|_| TransportError::ConnectionTimeout)?
.map_err(|e| TransportError::QuicConnection(e.to_string()))?;
```

#### Stream Management
```rust
// Before
let (send, recv) = connection.open_bi().await.unwrap();

// After
let (send, recv) = connection
    .open_bi()
    .await
    .map_err(|e| match e {
        quinn::ConnectionError::LocallyClosed => TransportError::ConnectionClosed,
        quinn::ConnectionError::TimedOut => TransportError::StreamTimeout,
        e => TransportError::StreamCreation(e.to_string()),
    })?;
```

### 3. TCP Fallback Logic
```rust
pub async fn connect_with_fallback(&self, addr: SocketAddr) -> Result<Connection> {
    // Try QUIC first
    match self.connect_quic(addr).await {
        Ok(conn) => {
            log::debug!("Connected via QUIC to {}", addr);
            Ok(Connection::Quic(conn))
        }
        Err(quic_err) => {
            log::warn!("QUIC connection failed: {}, falling back to TCP", quic_err);
            
            // Fallback to TCP
            self.connect_tcp(addr)
                .await
                .map(Connection::Tcp)
                .map_err(|tcp_err| TransportError::BothProtocolsFailed {
                    quic: quic_err.to_string(),
                    tcp: tcp_err.to_string(),
                })
        }
    }
}
```

### 4. Connection Pool Safety
- Handle pool exhaustion gracefully
- Implement connection health checks
- Clean up failed connections
- Add metrics for pool utilization

### 5. Message Framing
```rust
// Before
let len = buf.get_u32() as usize;
let data = buf.split_to(len);

// After
let len = buf.get_u32() as usize;
if len > self.config.max_message_size {
    return Err(TransportError::MessageTooLarge { 
        size: len, 
        max: self.config.max_message_size 
    });
}
if buf.remaining() < len {
    return Err(TransportError::IncompleteMessage);
}
let data = buf.split_to(len);
```

## Testing Requirements
- Simulate network failures during connection
- Test timeout scenarios
- Verify QUIC to TCP fallback
- Connection pool stress testing
- Message framing edge cases

## Dependencies
- Previous: Task 001 (Error Framework)
- Related: Task 002 (Network module integration)

## Time Estimate
- Implementation: 10 hours
- Testing: 4 hours
- Integration: 2 hours
- Total: 16 hours

## Definition of Done
- [ ] No unwrap/expect in transport code
- [ ] All async operations have timeouts
- [ ] Fallback mechanism tested
- [ ] Connection pool resilient to failures
- [ ] Performance benchmarks pass