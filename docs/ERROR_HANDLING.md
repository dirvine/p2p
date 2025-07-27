# Error Handling Guidelines for P2P Foundation

## Overview

This document provides comprehensive guidelines for error handling in the P2P Foundation codebase. We use a hybrid approach with `thiserror` for library code and `anyhow` for application code.

## Core Principles

### 1. Zero `unwrap()` Policy

**NEVER** use `unwrap()` or `expect()` in production code. All fallible operations must return `Result<T, E>`.

```rust
// ❌ BAD
let peer_id = PeerId::from_str(&input).unwrap();

// ✅ GOOD
let peer_id = PeerId::from_str(&input)
    .map_err(|e| NetworkError::InvalidAddress {
        addr: input.to_string(),
        reason: e.to_string(),
    })?;
```

### 2. Error Type Hierarchy

Use specific error types for each module:

```rust
use crate::error::{Result, NetworkError, P2PError};

// Module-specific errors convert to P2PError automatically
fn network_operation() -> Result<()> {
    connect_to_peer("192.168.1.1:8080")
        .map_err(|e| NetworkError::ConnectionFailed {
            peer: "192.168.1.1:8080".to_string(),
            reason: e.to_string(),
        })?;
    Ok(())
}
```

### 3. Error Context

Always provide context for errors:

```rust
use crate::error::ErrorContext;

fn complex_operation() -> Result<Data> {
    load_config()
        .context("failed to load configuration")?;
    
    connect_to_network()
        .with_context(|| format!("failed to connect to network at {}", timestamp()))?;
    
    Ok(data)
}
```

## Error Types Reference

### Network Errors

```rust
use crate::error::NetworkError;

// Connection failures
NetworkError::ConnectionFailed { peer, reason }
NetworkError::ConnectionTimeout { peer, timeout_secs }
NetworkError::PeerDisconnected { peer, reason }

// Address issues
NetworkError::InvalidAddress { addr, reason }
NetworkError::BindError { addr, reason }

// Network state
NetworkError::NetworkUnreachable { reason }
NetworkError::ProtocolError(message)
```

### DHT Errors

```rust
use crate::error::DhtError;

// Lookup failures
DhtError::KeyNotFound { key }
DhtError::LookupFailed { key, reason }

// Storage issues
DhtError::StorageFailed { key, reason }
DhtError::InsufficientReplicas { available, required }

// Routing problems
DhtError::RoutingError(message)
DhtError::ReplicationFailed { key, reason }
```

### Security Errors

```rust
use crate::error::SecurityError;

// Authentication/Authorization
SecurityError::AuthenticationFailed { reason }
SecurityError::AuthorizationFailed { reason }

// Cryptography
SecurityError::EncryptionFailed(message)
SecurityError::DecryptionFailed(message)
SecurityError::SignatureVerificationFailed

// Key management
SecurityError::InvalidKey(message)
SecurityError::KeyGenerationFailed(message)
```

## Recovery Strategies

### 1. Network Errors

```rust
async fn connect_with_retry(addr: &str) -> Result<Connection> {
    let mut attempts = 0;
    let max_attempts = 3;
    
    loop {
        match connect_to_peer(addr).await {
            Ok(conn) => return Ok(conn),
            Err(NetworkError::ConnectionTimeout { .. }) if attempts < max_attempts => {
                attempts += 1;
                tokio::time::sleep(Duration::from_secs(1 << attempts)).await;
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
}
```

### 2. DHT Errors

```rust
async fn get_with_fallback(key: &str) -> Result<Vec<u8>> {
    match dht.get(key).await {
        Ok(data) => Ok(data),
        Err(DhtError::KeyNotFound { .. }) => {
            // Try bootstrap nodes
            for bootstrap in &bootstrap_nodes {
                if let Ok(data) = bootstrap.get(key).await {
                    return Ok(data);
                }
            }
            Err(DhtError::KeyNotFound { key: key.to_string() }.into())
        }
        Err(e) => Err(e.into()),
    }
}
```

### 3. Storage Errors

```rust
fn save_with_fallback(data: &[u8]) -> Result<()> {
    match primary_storage.save(data) {
        Ok(()) => Ok(()),
        Err(StorageError::InsufficientSpace { .. }) => {
            // Try secondary storage
            secondary_storage.save(data)
                .map_err(|e| e.into())
        }
        Err(e) => Err(e.into()),
    }
}
```

## Testing Error Handling

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_failure_handling() {
        let result = connect_to_invalid_address("invalid:address");
        assert!(matches!(
            result,
            Err(P2PError::Network(NetworkError::InvalidAddress { .. }))
        ));
    }
    
    #[test]
    fn test_error_context() {
        let result = failing_operation()
            .context("during startup");
        
        if let Err(e) = result {
            let error_chain = format!("{:?}", e);
            assert!(error_chain.contains("during startup"));
        }
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_network_error_recovery() {
    let mut network = TestNetwork::new();
    
    // Simulate network failure
    network.disconnect_peer("peer1").await;
    
    // Operation should handle disconnection gracefully
    let result = network.send_message("peer1", b"test").await;
    assert!(matches!(
        result,
        Err(P2PError::Network(NetworkError::PeerDisconnected { .. }))
    ));
    
    // Verify automatic reconnection
    tokio::time::sleep(Duration::from_secs(2)).await;
    let result = network.send_message("peer1", b"test").await;
    assert!(result.is_ok());
}
```

## Migration Guide

### Replacing `unwrap()`

1. **Simple unwrap replacement**:
```rust
// Before
let value = some_operation().unwrap();

// After
let value = some_operation()?;
```

2. **With custom error**:
```rust
// Before
let addr = peer_addr.parse().unwrap();

// After
let addr = peer_addr.parse()
    .map_err(|e| NetworkError::InvalidAddress {
        addr: peer_addr.to_string(),
        reason: e.to_string(),
    })?;
```

3. **In match statements**:
```rust
// Before
match some_result {
    Ok(val) => process(val),
    Err(_) => panic!("unexpected error"),
}

// After
match some_result {
    Ok(val) => process(val),
    Err(e) => return Err(e.into()),
}
```

### Clippy Integration

Add these lints to catch unwrap usage:

```toml
# In Cargo.toml or .cargo/config.toml
[lints.rust]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic_in_result_fn = "deny"
```

## Best Practices

### 1. Error Messages

- Be specific about what failed
- Include relevant context (addresses, keys, etc.)
- Don't expose sensitive information
- Use consistent terminology

### 2. Error Propagation

```rust
// Use ? operator for automatic conversion
fn high_level_operation() -> Result<()> {
    low_level_operation()?;
    another_operation()?;
    Ok(())
}

// Add context when propagating
fn operation_with_context() -> Result<()> {
    database_operation()
        .context("failed to update user record")?;
    Ok(())
}
```

### 3. Logging Errors

```rust
use tracing::{error, warn};

match operation().await {
    Ok(result) => process(result),
    Err(e) => {
        // Log at appropriate level
        match &e {
            P2PError::Network(NetworkError::PeerDisconnected { .. }) => {
                warn!("Peer disconnected: {}", e);
            }
            P2PError::Security(_) => {
                error!("Security error: {}", e);
            }
            _ => {
                error!("Operation failed: {}", e);
            }
        }
        return Err(e);
    }
}
```

### 4. Application Error Handling

In application code (binaries), use `anyhow`:

```rust
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config()
        .context("Failed to load configuration")?;
    
    let node = P2PNode::new(config)
        .await
        .context("Failed to create P2P node")?;
    
    node.run()
        .await
        .context("Node execution failed")?;
    
    Ok(())
}
```

## Monitoring and Metrics

Track errors for monitoring:

```rust
use prometheus::{Counter, register_counter_vec};

lazy_static! {
    static ref ERROR_COUNTER: Counter = register_counter_vec!(
        "p2p_errors_total",
        "Total number of errors by type",
        &["error_type", "module"]
    ).unwrap();
}

fn track_error(error: &P2PError) {
    let (error_type, module) = match error {
        P2PError::Network(_) => ("network", "network"),
        P2PError::Dht(_) => ("dht", "dht"),
        P2PError::Security(_) => ("security", "security"),
        // ... other cases
    };
    
    ERROR_COUNTER
        .with_label_values(&[error_type, module])
        .inc();
}
```

## Security Considerations

### 1. Error Information Disclosure

Never expose internal details in external APIs:

```rust
// ❌ BAD - Exposes internal paths
StorageError::FileNotFound { 
    path: "/home/user/.p2p/keys/private.key".to_string() 
}

// ✅ GOOD - Generic message
StorageError::FileNotFound { 
    path: "configuration file".to_string() 
}
```

### 2. Timing Attacks

Use constant-time operations for security-critical errors:

```rust
fn verify_signature(sig: &[u8], data: &[u8]) -> Result<()> {
    // Constant-time verification
    let is_valid = constant_time_verify(sig, data);
    
    if !is_valid {
        // Don't reveal why verification failed
        return Err(SecurityError::SignatureVerificationFailed.into());
    }
    
    Ok(())
}
```

## Checklist

Before marking error handling complete:

- [ ] All `unwrap()` and `expect()` removed
- [ ] All `panic!()` calls removed (except in tests)
- [ ] Error types defined for each module
- [ ] From trait implementations for error conversions
- [ ] Context added to error propagation
- [ ] Recovery strategies implemented
- [ ] Error tracking/metrics in place
- [ ] Security considerations addressed
- [ ] Tests cover error paths
- [ ] Documentation updated

## Further Reading

- [The Rust Book - Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Error Handling in Rust - A Deep Dive](https://nick.groenen.me/posts/rust-error-handling/)
- [thiserror Documentation](https://docs.rs/thiserror/latest/thiserror/)
- [anyhow Documentation](https://docs.rs/anyhow/latest/anyhow/)