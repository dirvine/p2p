//! Error types module
//!
//! This module contains error types used throughout the P2P Foundation.

/// Main error type for P2P Foundation
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    /// Network-related error (connections, peers, protocols)
    #[error("Network error: {0}")]
    Network(String),
    
    /// DHT operation error (lookups, storage, routing)
    #[error("DHT error: {0}")]
    DHT(String),
    
    /// Transport layer error (QUIC, TCP, tunneling)
    #[error("Transport error: {0}")]
    Transport(String),
    
    /// Security-related error (authentication, encryption, validation)
    #[error("Security error: {0}")]
    Security(String),
    
    /// MCP server error (tool calls, message routing)
    #[error("MCP error: {0}")]
    MCP(String),
    
    /// Bootstrap cache error (peer discovery, cache management)
    #[error("Bootstrap error: {0}")]
    Bootstrap(String),
    
    /// Configuration error (invalid settings, missing parameters)
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Cryptography error (key generation, encryption, signatures)
    #[error("Cryptography error: {0}")]
    Cryptography(String),
    
    /// Invalid state error (operations in wrong state)
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    /// Invalid input error (bad parameters, malformed data)
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    /// IO error (file operations, network IO)
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    /// Serialization error (JSON, protocol encoding/decoding)
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Generic error (catch-all for other error types)
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

/// Result type alias for P2P Foundation operations
pub type Result<T> = std::result::Result<T, P2PError>;