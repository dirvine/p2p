//! # P2P Foundation
//! 
//! A next-generation peer-to-peer networking foundation built in Rust.
//! 
//! ## Features
//! 
//! - QUIC-based transport for modern networking
//! - IPv6-first with comprehensive tunneling support
//! - Kademlia DHT for distributed routing
//! - Built-in MCP server for AI capabilities
//! - Minimal dependencies and small footprint
//! 
//! ## Example
//! 
//! ```rust,no_run
//! use p2p_foundation::{P2PNode, NodeConfig};
//! 
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let node = P2PNode::builder()
//!         .listen_on("/ip6/::/tcp/9000")
//!         .with_mcp_server()
//!         .build()
//!         .await?;
//!     
//!     node.run().await?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

/// Network core functionality
pub mod network;

/// Distributed Hash Table implementation
pub mod dht;

/// Transport layer (QUIC, TCP)
pub mod transport;

/// IPv6/IPv4 tunneling protocols
pub mod tunneling;

/// Model Context Protocol server
pub mod mcp;

/// Security and cryptography
pub mod security;

/// Utility functions and types
pub mod utils;

/// Error types
pub mod error;

// Re-export main types
pub use network::{P2PNode, NodeConfig, NodeBuilder};
pub use dht::{Key, Record};
pub use mcp::{MCPServer, Tool, MCPService};
pub use error::{P2PError, Result};

// Re-export libp2p types that users need
pub use libp2p::{PeerId, Multiaddr};

/// P2P Foundation version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}