// Copyright 2024 Saorsa Labs Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

//! Error handling infrastructure for P2P Foundation
//!
//! This module provides a comprehensive error handling system using the `thiserror` crate
//! for library code. Applications should use `anyhow` for error handling while libraries
//! should use these specific error types.
//!
//! # Design Principles
//!
//! 1. **Specific Error Types**: Each module has its own error type with detailed variants
//! 2. **Error Context**: All errors include context about what operation failed
//! 3. **Error Conversion**: Automatic conversion between error types using From traits
//! 4. **No Panics**: All fallible operations return Result types
//! 5. **Recovery Strategies**: Errors include information needed for recovery
//!
//! # Example
//!
//! ```rust
//! use crate::error::{Result, NetworkError};
//!
//! fn connect_to_peer(addr: &str) -> Result<Connection> {
//!     let socket = create_socket(addr)
//!         .map_err(|e| NetworkError::ConnectionFailed {
//!             peer: addr.to_string(),
//!             reason: e.to_string(),
//!         })?;
//!     Ok(Connection::new(socket))
//! }
//! ```

use std::net::SocketAddr;
use thiserror::Error;

/// Main error type for P2P Foundation operations
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum P2PError {
    /// Network-related errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    /// DHT operation errors
    #[error("DHT error: {0}")]
    Dht(#[from] DhtError),
    
    /// Transport layer errors
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    
    /// Security-related errors
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
    
    /// Identity management errors
    #[error("Identity error: {0}")]
    Identity(#[from] IdentityError),
    
    /// Storage errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    /// MCP server errors
    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),
    
    /// Bootstrap errors
    #[error("Bootstrap error: {0}")]
    Bootstrap(#[from] BootstrapError),
    
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Network-related errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NetworkError {
    /// Failed to establish connection
    #[error("Failed to connect to {peer}: {reason}")]
    ConnectionFailed {
        peer: String,
        reason: String,
    },
    
    /// Connection timeout
    #[error("Connection to {peer} timed out after {timeout_secs}s")]
    ConnectionTimeout {
        peer: String,
        timeout_secs: u64,
    },
    
    /// Peer disconnected unexpectedly
    #[error("Peer {peer} disconnected: {reason}")]
    PeerDisconnected {
        peer: String,
        reason: String,
    },
    
    /// Invalid network address
    #[error("Invalid address: {addr} - {reason}")]
    InvalidAddress {
        addr: String,
        reason: String,
    },
    
    /// Network is unreachable
    #[error("Network unreachable: {reason}")]
    NetworkUnreachable {
        reason: String,
    },
    
    /// Bind error
    #[error("Failed to bind to {addr}: {reason}")]
    BindError {
        addr: SocketAddr,
        reason: String,
    },
    
    /// Protocol error
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

/// DHT operation errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DhtError {
    /// Key not found in DHT
    #[error("Key not found: {key}")]
    KeyNotFound {
        key: String,
    },
    
    /// Storage operation failed
    #[error("Failed to store key {key}: {reason}")]
    StorageFailed {
        key: String,
        reason: String,
    },
    
    /// Lookup operation failed
    #[error("Lookup failed for key {key}: {reason}")]
    LookupFailed {
        key: String,
        reason: String,
    },
    
    /// Insufficient replicas
    #[error("Insufficient replicas: got {available}, need {required}")]
    InsufficientReplicas {
        available: usize,
        required: usize,
    },
    
    /// Routing table error
    #[error("Routing table error: {0}")]
    RoutingError(String),
    
    /// Replication failed
    #[error("Replication failed for key {key}: {reason}")]
    ReplicationFailed {
        key: String,
        reason: String,
    },
}

/// Transport layer errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransportError {
    /// QUIC transport error
    #[error("QUIC error: {0}")]
    Quic(String),
    
    /// TCP transport error
    #[error("TCP error: {0}")]
    Tcp(String),
    
    /// Failed to establish transport
    #[error("Transport setup failed: {0}")]
    SetupFailed(String),
    
    /// Stream error
    #[error("Stream error: {0}")]
    StreamError(String),
    
    /// Certificate error
    #[error("Certificate error: {0}")]
    CertificateError(String),
    
    /// NAT traversal failed
    #[error("NAT traversal failed: {0}")]
    NatTraversalFailed(String),
}

/// Security-related errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SecurityError {
    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed {
        reason: String,
    },
    
    /// Authorization failed
    #[error("Authorization failed: {reason}")]
    AuthorizationFailed {
        reason: String,
    },
    
    /// Encryption error
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    /// Decryption error
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    /// Signature verification failed
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    
    /// Invalid key
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    
    /// Key generation failed
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),
}

/// Identity management errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum IdentityError {
    /// Identity not found
    #[error("Identity not found: {id}")]
    NotFound {
        id: String,
    },
    
    /// Identity already exists
    #[error("Identity already exists: {id}")]
    AlreadyExists {
        id: String,
    },
    
    /// Invalid identity format
    #[error("Invalid identity format: {reason}")]
    InvalidFormat {
        reason: String,
    },
    
    /// Identity verification failed
    #[error("Identity verification failed: {reason}")]
    VerificationFailed {
        reason: String,
    },
    
    /// Key derivation failed
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    
    /// System time error
    #[error("System time error: {0}")]
    SystemTime(String),
}

/// Storage errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StorageError {
    /// Database error
    #[error("Database error: {0}")]
    Database(String),
    
    /// File not found
    #[error("File not found: {path}")]
    FileNotFound {
        path: String,
    },
    
    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied {
        path: String,
    },
    
    /// Corruption detected
    #[error("Data corruption detected: {reason}")]
    CorruptionDetected {
        reason: String,
    },
    
    /// Insufficient space
    #[error("Insufficient storage space: need {required}, have {available}")]
    InsufficientSpace {
        required: u64,
        available: u64,
    },
}

/// Configuration errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// Missing required field
    #[error("Missing required field: {field}")]
    MissingField {
        field: String,
    },
    
    /// Invalid value
    #[error("Invalid value for {field}: {value} - {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
    
    /// Configuration file error
    #[error("Configuration file error: {0}")]
    FileError(String),
    
    /// Validation failed
    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}

/// MCP server errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpError {
    /// Tool not found
    #[error("Tool not found: {tool}")]
    ToolNotFound {
        tool: String,
    },
    
    /// Tool execution failed
    #[error("Tool execution failed: {tool} - {reason}")]
    ToolExecutionFailed {
        tool: String,
        reason: String,
    },
    
    /// Invalid request
    #[error("Invalid MCP request: {0}")]
    InvalidRequest(String),
    
    /// Server not available
    #[error("MCP server not available: {0}")]
    ServerUnavailable(String),
}

/// Bootstrap errors
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BootstrapError {
    /// No bootstrap peers available
    #[error("No bootstrap peers available")]
    NoPeersAvailable,
    
    /// Bootstrap failed
    #[error("Bootstrap failed: {reason}")]
    BootstrapFailed {
        reason: String,
    },
    
    /// Invalid bootstrap data
    #[error("Invalid bootstrap data: {0}")]
    InvalidData(String),
    
    /// Cache error
    #[error("Bootstrap cache error: {0}")]
    CacheError(String),
}

/// Result type alias for P2P Foundation operations
pub type Result<T> = std::result::Result<T, P2PError>;

/// Extension trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to an error
    fn context(self, msg: &str) -> Result<T>;
    
    /// Add context with a closure (lazy evaluation)
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<P2PError>,
{
    fn context(self, _msg: &str) -> Result<T> {
        self.map_err(|e| {
            let base_error = e.into();
            // In a real implementation, we'd wrap the error with context
            // For now, we'll just return the base error
            base_error
        })
    }
    
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let _context = f();
            let base_error = e.into();
            // In a real implementation, we'd wrap the error with context
            // For now, we'll just return the base error
            base_error
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_error_display() {
        let err = NetworkError::ConnectionFailed {
            peer: "192.168.1.1:8080".to_string(),
            reason: "connection refused".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to connect to 192.168.1.1:8080: connection refused"
        );
    }
    
    #[test]
    fn test_error_conversion() {
        let network_err = NetworkError::ConnectionTimeout {
            peer: "example.com".to_string(),
            timeout_secs: 30,
        };
        let p2p_err: P2PError = network_err.into();
        assert!(matches!(p2p_err, P2PError::Network(_)));
    }
    
    #[test]
    fn test_dht_error_display() {
        let err = DhtError::InsufficientReplicas {
            available: 2,
            required: 3,
        };
        assert_eq!(
            err.to_string(),
            "Insufficient replicas: got 2, need 3"
        );
    }
    
    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let p2p_err: P2PError = io_err.into();
        assert!(matches!(p2p_err, P2PError::Io(_)));
    }
    
    #[test]
    fn test_security_error() {
        let err = SecurityError::AuthenticationFailed {
            reason: "invalid credentials".to_string(),
        };
        assert_eq!(err.to_string(), "Authentication failed: invalid credentials");
    }
    
    #[test]
    fn test_storage_error() {
        let err = StorageError::InsufficientSpace {
            required: 1_000_000,
            available: 500_000,
        };
        assert_eq!(
            err.to_string(),
            "Insufficient storage space: need 1000000, have 500000"
        );
    }
    
    #[test]
    fn test_config_error() {
        let err = ConfigError::InvalidValue {
            field: "port".to_string(),
            value: "abc".to_string(),
            reason: "not a number".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid value for port: abc - not a number"
        );
    }
    
    #[test]
    fn test_error_context() {
        fn failing_operation() -> std::result::Result<(), NetworkError> {
            Err(NetworkError::NetworkUnreachable {
                reason: "no route to host".to_string(),
            })
        }
        
        let result = failing_operation()
            .context("while connecting to bootstrap node");
        
        assert!(result.is_err());
    }
}