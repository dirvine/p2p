// Copyright 2024 MaidSafe Limited
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