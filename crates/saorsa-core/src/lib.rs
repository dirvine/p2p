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

//! # Saorsa Core
//!
//! A next-generation peer-to-peer networking foundation built in Rust.
//!
//! ## Features
//!
//! - QUIC-based transport with NAT traversal
//! - IPv4-first with simple addressing
//! - Kademlia DHT for distributed routing
//! - Built-in MCP server for AI capabilities
//! - Four-word human-readable addresses
//!
//! ## Example
//!
//! ```rust,no_run
//! use saorsa_core::{P2PNode, NodeConfig, NetworkAddress};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let addr = "127.0.0.1:9000".parse::<NetworkAddress>()?;
//!     let node = P2PNode::builder()
//!         .listen_on(addr)
//!         .with_mcp_server()
//!         .build()
//!         .await?;
//!     
//!     node.run().await?;
//!     Ok(())
//! }
//! ```

#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
#![warn(rust_2018_idioms)]

/// Network address types
pub mod address;

/// Network core functionality
pub mod network;

/// Distributed Hash Table implementation
pub mod dht;

/// DHT Network Integration Manager
pub mod dht_network_manager;

/// Transport layer (QUIC, TCP)
pub mod transport;

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

/// Validation framework for input sanitization and rate limiting
pub mod validation;

/// Production hardening features
pub mod production;

/// Bootstrap cache for decentralized peer discovery
pub mod bootstrap;

/// Error types
pub mod error;

/// Peer record system for DHT-based peer discovery
pub mod peer_record;

/// Enhanced cryptographic signature verification system
pub mod crypto_verify;

/// Monotonic counter system for replay attack prevention
pub mod monotonic_counter;

/// Secure memory management for cryptographic operations
pub mod secure_memory;

/// Hierarchical key derivation system
pub mod key_derivation;

/// Encrypted key storage with Argon2id and AES-256-GCM
pub mod encrypted_key_storage;

/// Persistent state management with crash recovery
pub mod persistent_state;

/// Identity management system with Ed25519/X25519 key pairs
pub mod identity_manager;

/// Adaptive P2P network implementation
pub mod adaptive;

/// Configuration management system
pub mod config;

/// Health check system for monitoring and metrics
pub mod health;

// Re-export main types
pub use address::{AddressBook, NetworkAddress};
pub use bootstrap::{BootstrapCache, BootstrapManager, CacheConfig, ContactEntry};
pub use crypto_verify::{
    BatchVerificationRequest, BatchVerificationResult, EnhancedSignatureVerification,
    EnhancedSignatureVerifier, VerificationStats,
};
pub use dht::{Key, Record};
pub use dht_network_manager::{
    BootstrapNode, DhtNetworkConfig, DhtNetworkEvent, DhtNetworkManager, DhtNetworkOperation,
    DhtNetworkResult, DhtPeerInfo,
};
pub use encrypted_key_storage::{
    Argon2Config, DerivationPriority as KeyDerivationPriority, EncryptedKeyStorageManager,
    KeyMetadata, PasswordValidation, SecurityLevel, StorageStats,
};
pub use error::{P2PError, P2pResult as Result};
pub use health::{
    ComponentChecker, ComponentHealth, HealthEndpoints, HealthManager, HealthResponse,
    HealthServer, HealthStatus, PrometheusExporter,
};
pub use identity_manager::{
    Identity, IdentityCreationParams, IdentityKeyPair, IdentityManager, IdentityState,
    IdentityStats, IdentitySyncPackage, IdentityUpdate, IdentityVerification,
    RevocationCertificate, RevocationReason,
};
pub use key_derivation::{
    BatchDerivationRequest, BatchDerivationResult, DerivationPath, DerivationPriority,
    DerivationStats, DerivedKey, HierarchicalKeyDerivation, MasterSeed,
};
pub use mcp::{MCPServer, MCPService, Tool};
pub use monotonic_counter::{
    BatchUpdateRequest, BatchUpdateResult, CounterStats, MonotonicCounterSystem, PeerCounter,
    SequenceValidationResult,
};
pub use network::{NodeBuilder, NodeConfig, P2PEvent, P2PNode};
pub use peer_record::{EndpointId, NatType, PeerDHTRecord, PeerEndpoint, SignatureCache, UserId};
pub use persistent_state::{
    FlushStrategy, IntegrityReport, PersistentStateManager, RecoveryMode, RecoveryStats,
    StateChangeEvent, StateConfig, TransactionType, WalEntry,
};
pub use production::{ProductionConfig, ResourceManager, ResourceMetrics};
pub use secure_memory::{
    PoolStats, SecureMemory, SecureMemoryPool, SecureString, SecureVec, allocate_secure,
    secure_string_with_capacity, secure_vec_with_capacity,
};
pub use validation::{
    RateLimitConfig, RateLimiter, Sanitize, Validate, ValidationContext, ValidationError,
    sanitize_string, validate_dht_key, validate_dht_value, validate_file_path,
    validate_message_size, validate_network_address, validate_peer_id,
};

// Enhanced identity exports
#[cfg(feature = "quantum-resistant")]
pub use identity::enhanced::{
    Department, EnhancedIdentity, EnhancedIdentityManager, Organization, Permission, Team,
};

// Storage exports
pub use storage::{FileChunker, StorageManager}; // SyncManager temporarily disabled

// Chat exports
pub use chat::{Call, Channel, ChannelId, ChannelType, ChatManager, Message, MessageId, Thread};

// Discuss exports
pub use discuss::{
    Badge, Category, CategoryId, DiscussManager, Poll, Reply, ReplyId, Topic, TopicId, UserStats,
};

// Projects exports
pub use projects::{
    Document, DocumentId, Folder, Project, ProjectAnalytics, ProjectId, ProjectsManager,
    WorkflowState,
};

// Threshold exports
pub use threshold::{
    GroupMetadata, ParticipantInfo, ThresholdGroup, ThresholdGroupManager, ThresholdSignature,
};

// Quantum crypto exports for types used by threshold
pub use quantum_crypto::types::{GroupId, ParticipantId};

// Network address types
/// Peer identifier used throughout Saorsa
///
/// Currently implemented as a String for simplicity, but can be enhanced
/// with cryptographic verification in future versions.
pub type PeerId = String;

/// Network address used for peer-to-peer communication
///
/// Supports both traditional IP:port format and human-readable four-word format.
pub type Multiaddr = NetworkAddress;

/// Saorsa Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
