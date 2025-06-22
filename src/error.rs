//! Error types module
//!
//! This module contains error types used throughout the P2P Foundation.

/// Main error type for P2P Foundation
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("DHT error: {0}")]
    DHT(String),
    
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Security error: {0}")]
    Security(String),
    
    #[error("MCP error: {0}")]
    MCP(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias for P2P Foundation operations
pub type Result<T> = std::result::Result<T, P2PError>;