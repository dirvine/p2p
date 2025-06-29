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

/// DHT Network Integration Manager
pub mod dht_network_manager;

/// Transport layer (QUIC, TCP)
pub mod transport;

/// IPv6/IPv4 tunneling protocols
pub mod tunneling;

/// Model Context Protocol server
pub mod mcp;

/// Security and cryptography
pub mod security;

/// User identity and privacy system
pub mod identity;

/// DHT-based storage for multi-device sync
pub mod storage;

/// Chat system (Slack-like)
pub mod chat;

/// Discuss system (Discourse-like)
pub mod discuss;

/// Projects system with hierarchical organization
pub mod projects;

/// Threshold cryptography for group operations
pub mod threshold;

/// Quantum-resistant cryptography
pub mod quantum_crypto;

/// Utility functions and types
pub mod utils;

/// Production hardening features
pub mod production;

/// Bootstrap cache for decentralized peer discovery
pub mod bootstrap;

/// Error types
pub mod error;

// Re-export main types
pub use network::{P2PNode, NodeConfig, NodeBuilder, P2PEvent};
pub use dht::{Key, Record};
pub use dht_network_manager::{DhtNetworkManager, DhtNetworkConfig, DhtNetworkOperation, DhtNetworkResult, DhtNetworkEvent, DhtPeerInfo, BootstrapNode};
pub use mcp::{MCPServer, Tool, MCPService};
pub use production::{ProductionConfig, ResourceManager, ResourceMetrics};
pub use bootstrap::{BootstrapManager, BootstrapCache, ContactEntry, CacheConfig};
pub use error::{P2PError, Result};

// Enhanced identity exports
#[cfg(feature = "quantum-resistant")]
pub use identity::enhanced::{
    EnhancedIdentity, EnhancedIdentityManager, Organization, 
    Department, Team, Permission,
};

// Storage exports
pub use storage::{StorageManager, FileChunker}; // SyncManager temporarily disabled

// Chat exports
pub use chat::{
    Channel, ChannelId, Message, MessageId, Thread, 
    ChatManager, ChannelType, Call,
};

// Discuss exports
pub use discuss::{
    Category, CategoryId, Topic, TopicId, Reply, ReplyId,
    DiscussManager, Poll, Badge, UserStats,
};

// Projects exports
pub use projects::{
    Project, ProjectId, Document, DocumentId, Folder,
    ProjectsManager, WorkflowState, ProjectAnalytics,
};

// Threshold exports
pub use threshold::{
    ThresholdGroup, ThresholdSignature,
    ThresholdGroupManager, ParticipantInfo, GroupMetadata,
};

// Quantum crypto exports for types used by threshold
pub use quantum_crypto::types::{GroupId, ParticipantId};

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