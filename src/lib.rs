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

/// Quantum-resistant cryptography
#[cfg(feature = "quantum-resistant")]
pub mod quantum_crypto;

/// Threshold cryptography and group management
#[cfg(feature = "threshold")]
pub mod threshold;

/// Utility functions and types
pub mod utils;

/// Production hardening features
pub mod production;

/// Bootstrap cache for decentralized peer discovery
pub mod bootstrap;

/// Error types
pub mod error;

/// License management and enforcement
pub mod licensing;

/// Storage layer for distributed data
pub mod storage;

/// Git-like content addressing system
pub mod git_content_addressing;

/// Git object implementations
pub mod git_objects;

/// Transport-DHT integration layer
pub mod transport_dht_integration;

/// Git-DHT storage integration layer
pub mod git_dht_storage;

/// Git application layer for high-level operations
pub mod git_application_layer;

// Re-export main types
pub use network::{P2PNode, NodeConfig, NodeBuilder, P2PEvent};
pub use dht::{Key, Record};
pub use mcp::{MCPServer, Tool, MCPService};
pub use production::{ProductionConfig, ResourceManager, ResourceMetrics};
pub use bootstrap::{BootstrapManager, BootstrapCache, ContactEntry, CacheConfig};
pub use error::{P2PError, Result};
pub use licensing::{LicenseChecker, LicenseType, LicenseStatus, Feature};

// Git-related exports
pub use git_content_addressing::{ContentHash, ObjectType, GitObject, GitContentError, GitResult};
pub use git_objects::{
    BlobObject, TreeObject, CommitObject, TagObject, Reference, ReferenceType,
    CommitAuthor, CommitType, BranchState, TreeEntry, EntryMode,
};
pub use git_dht_storage::{GitDhtStorage, DhtStorageProvider, GitCacheStats};
pub use git_application_layer::{GitApplicationLayer, ChatMessage, ForumPost, Document, DocumentFormat, RepositoryStats};

#[cfg(feature = "quantum-resistant")]
pub use quantum_crypto::{
    QuantumPeerIdentity, CryptoCapabilities, SignatureScheme,
    generate_keypair, negotiate_algorithms,
};

#[cfg(feature = "quantum-resistant")]
pub use quantum_crypto::types::{GroupId, ParticipantId};

#[cfg(feature = "threshold")]
pub use threshold::{
    ThresholdGroup, ThresholdGroupManager, GroupOperation,
    ParticipantInfo, ParticipantRole, ParticipantStatus,
};

// Placeholder types (will be replaced with actual libp2p types)
/// Peer identifier used throughout the P2P Foundation
/// 
/// Currently implemented as a String for simplicity, but will be replaced
/// with proper libp2p PeerId type in future versions.
pub type PeerId = String;

/// Multiaddress used for network addressing
/// 
/// Currently implemented as a String for simplicity, but will be replaced  
/// with proper libp2p Multiaddr type in future versions.
pub type Multiaddr = String;

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