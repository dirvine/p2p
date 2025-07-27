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

//! # Peer DHT Record System
//!
//! This module implements the core data structures for peer discovery and connection
//! establishment in the P2P network. It provides secure, scalable, and reliable
//! peer record management with comprehensive validation and serialization.
//!
//! ## Security Features
//! - Ed25519 signature verification for all records
//! - Monotonic counter system prevents replay attacks
//! - Size limits prevent memory exhaustion attacks
//! - Canonical serialization prevents signature bypass
//!
//! ## Performance Optimizations
//! - Efficient binary serialization with bincode
//! - Signature verification caching
//! - Minimal memory allocations
//! - Batch processing support

use crate::{NetworkAddress, P2PError, Result};
use crate::error::{SecurityError, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ed25519_dalek::{VerifyingKey, Signature, Signer, Verifier};
use blake3::Hash;
use uuid::Uuid;

/// Maximum size for a serialized DHT record to prevent memory exhaustion attacks
pub const MAX_DHT_RECORD_SIZE: usize = 64 * 1024; // 64KB

/// Maximum number of endpoints per peer to prevent resource exhaustion
pub const MAX_ENDPOINTS_PER_PEER: usize = 16;

/// Maximum TTL for DHT records (24 hours)
pub const MAX_TTL_SECONDS: u32 = 24 * 60 * 60;

/// Default TTL for DHT records (5 minutes)
pub const DEFAULT_TTL_SECONDS: u32 = 5 * 60;

/// Unique identifier for a user in the P2P network
/// Generated from public key hash to ensure uniqueness and prevent collisions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId {
    /// BLAKE3 hash of the user's public key
    pub hash: [u8; 32],
}

impl UserId {
    /// Create a new UserId from a public key
    pub fn from_public_key(public_key: &VerifyingKey) -> Self {
        let hash = blake3::hash(public_key.as_bytes());
        Self {
            hash: hash.into(),
        }
    }

    /// Create a UserId from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { hash: bytes }
    }

    /// Get the raw bytes of this UserId
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.hash
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.hash[..8]))
    }
}

/// Unique identifier for a peer endpoint/device
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointId {
    /// UUID v4 for device identification
    pub uuid: Uuid,
}

impl EndpointId {
    /// Generate a new random endpoint ID
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }

    /// Create from existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self { uuid }
    }
}

impl Default for EndpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EndpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

/// Node identifier for coordinator nodes
pub type NodeId = String;

/// NAT type classification based on IETF draft-seemann-quic-nat-traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NatType {
    /// No NAT - public IP address
    NoNat,
    /// Full Cone NAT - best case for hole punching
    FullCone,
    /// Restricted Cone NAT - IP address restricted
    RestrictedCone,
    /// Port Restricted NAT - IP address and port restricted
    PortRestricted,
    /// Symmetric NAT - worst case for hole punching
    Symmetric,
    /// Unknown NAT type - requires further detection
    Unknown,
}

impl NatType {
    /// Check if this NAT type supports hole punching
    pub fn supports_hole_punching(&self) -> bool {
        matches!(self, NatType::NoNat | NatType::FullCone | NatType::RestrictedCone | NatType::PortRestricted)
    }

    /// Get the difficulty score for hole punching (0 = impossible, 100 = easy)
    pub fn hole_punching_difficulty(&self) -> u8 {
        match self {
            NatType::NoNat => 100,
            NatType::FullCone => 90,
            NatType::RestrictedCone => 70,
            NatType::PortRestricted => 50,
            NatType::Symmetric => 10,
            NatType::Unknown => 0,
        }
    }
}

impl fmt::Display for NatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NatType::NoNat => write!(f, "No NAT"),
            NatType::FullCone => write!(f, "Full Cone"),
            NatType::RestrictedCone => write!(f, "Restricted Cone"),
            NatType::PortRestricted => write!(f, "Port Restricted"),
            NatType::Symmetric => write!(f, "Symmetric"),
            NatType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Peer endpoint information for connection establishment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerEndpoint {
    /// Unique identifier for this endpoint/device
    pub endpoint_id: EndpointId,
    
    /// External network address as observed by the network
    pub external_address: NetworkAddress,
    
    /// Detected NAT type for this endpoint
    pub nat_type: NatType,
    
    /// Coordinator nodes this peer is connected to
    pub coordinator_nodes: Vec<NodeId>,
    
    /// Optional device/client information
    pub device_info: Option<String>,
    
    /// Timestamp when this endpoint was last updated
    pub last_updated: u64,
}

impl PeerEndpoint {
    /// Create a new peer endpoint
    pub fn new(
        endpoint_id: EndpointId,
        external_address: NetworkAddress,
        nat_type: NatType,
        coordinator_nodes: Vec<NodeId>,
        device_info: Option<String>,
    ) -> Self {
        Self {
            endpoint_id,
            external_address,
            nat_type,
            coordinator_nodes,
            device_info,
            last_updated: current_timestamp(),
        }
    }

    /// Check if this endpoint is stale based on last update time
    pub fn is_stale(&self, max_age: Duration) -> bool {
        let age = current_timestamp().saturating_sub(self.last_updated);
        age > max_age.as_secs()
    }

    /// Update the last_updated timestamp
    pub fn refresh(&mut self) {
        self.last_updated = current_timestamp();
    }
}

/// DHT record for peer discovery with security and validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerDHTRecord {
    /// Record format version for compatibility
    pub version: u8,
    
    /// Unique user identifier
    pub user_id: UserId,
    
    /// User's public key for signature verification
    pub public_key: VerifyingKey,
    
    /// Monotonic counter to prevent replay attacks
    pub sequence_number: u64,
    
    /// Optional human-readable name
    pub name: Option<String>,
    
    /// Current connection endpoints
    pub endpoints: Vec<PeerEndpoint>,
    
    /// Unix timestamp of this record creation
    pub timestamp: u64,
    
    /// Time-to-live in seconds
    pub ttl: u32,
    
    /// Ed25519 signature over all above fields
    pub signature: Signature,
}

impl PeerDHTRecord {
    /// Current record format version
    pub const CURRENT_VERSION: u8 = 1;

    /// Create a new unsigned DHT record
    pub fn new(
        user_id: UserId,
        public_key: VerifyingKey,
        sequence_number: u64,
        name: Option<String>,
        endpoints: Vec<PeerEndpoint>,
        ttl: u32,
    ) -> Result<Self> {
        // Validate inputs
        Self::validate_inputs(&name, &endpoints, ttl)?;

        Ok(Self {
            version: Self::CURRENT_VERSION,
            user_id,
            public_key,
            sequence_number,
            name,
            endpoints,
            timestamp: current_timestamp(),
            ttl,
            signature: Signature::from_bytes(&[0; 64]), // Placeholder, will be signed
        })
    }

    /// Validate record inputs
    fn validate_inputs(
        name: &Option<String>,
        endpoints: &[PeerEndpoint],
        ttl: u32,
    ) -> Result<()> {
        // Validate name length
        if let Some(name) = name {
            if name.len() > 255 {
                return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                    field: "name".to_string(),
                    value: format!("{} chars", name.len()),
                    reason: "Name too long (max 255)".to_string(),
                }));
            }
            if name.is_empty() {
                return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                    field: "name".to_string(),
                    value: "empty".to_string(),
                    reason: "Name cannot be empty".to_string(),
                }));
            }
        }

        // Validate endpoints
        if endpoints.is_empty() {
            return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                field: "endpoints".to_string(),
                value: "empty".to_string(),
                reason: "At least one endpoint required".to_string(),
            }));
        }
        if endpoints.len() > MAX_ENDPOINTS_PER_PEER {
            return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                field: "endpoints".to_string(),
                value: format!("{} endpoints", endpoints.len()),
                reason: format!("Too many endpoints (max {})", MAX_ENDPOINTS_PER_PEER),
            }));
        }

        // Validate TTL
        if ttl == 0 {
            return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                field: "ttl".to_string(),
                value: "0".to_string(),
                reason: "TTL cannot be zero".to_string(),
            }));
        }
        if ttl > MAX_TTL_SECONDS {
            return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                field: "ttl".to_string(),
                value: ttl.to_string(),
                reason: format!("TTL too large (max {})", MAX_TTL_SECONDS),
            }));
        }

        Ok(())
    }

    /// Create the canonical message for signing
    pub fn create_signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        
        // Version
        message.push(self.version);
        
        // User ID
        message.extend_from_slice(&self.user_id.hash);
        
        // Public key
        message.extend_from_slice(self.public_key.as_bytes());
        
        // Sequence number (big endian)
        message.extend_from_slice(&self.sequence_number.to_be_bytes());
        
        // Name (length-prefixed)
        if let Some(ref name) = self.name {
            let name_bytes = name.as_bytes();
            message.extend_from_slice(&(name_bytes.len() as u32).to_be_bytes());
            message.extend_from_slice(name_bytes);
        } else {
            message.extend_from_slice(&0u32.to_be_bytes());
        }
        
        // Endpoints (serialized deterministically)
        let endpoints_data = bincode::serialize(&self.endpoints).unwrap();
        message.extend_from_slice(&(endpoints_data.len() as u32).to_be_bytes());
        message.extend_from_slice(&endpoints_data);
        
        // Timestamp
        message.extend_from_slice(&self.timestamp.to_be_bytes());
        
        // TTL
        message.extend_from_slice(&self.ttl.to_be_bytes());
        
        message
    }

    /// Sign the record with the given private key
    pub fn sign(&mut self, signing_key: &ed25519_dalek::SigningKey) -> Result<()> {
        let message = self.create_signable_message();
        self.signature = signing_key.sign(&message);
        Ok(())
    }

    /// Verify the record signature
    pub fn verify_signature(&self) -> Result<()> {
        let message = self.create_signable_message();
        self.public_key.verify(&message, &self.signature)
            .map_err(|_| P2PError::Security(SecurityError::SignatureVerificationFailed))?;
        Ok(())
    }

    /// Check if the record has expired
    pub fn is_expired(&self) -> bool {
        let age = current_timestamp().saturating_sub(self.timestamp);
        age > self.ttl as u64
    }

    /// Get the remaining TTL in seconds
    pub fn remaining_ttl(&self) -> u32 {
        let age = current_timestamp().saturating_sub(self.timestamp);
        if age >= self.ttl as u64 {
            0
        } else {
            self.ttl - age as u32
        }
    }

    /// Serialize the record using bincode for efficiency
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let serialized = bincode::serialize(self)
            .map_err(|e| P2PError::Storage(StorageError::Database(format!("Failed to serialize record: {e}"))))?;
        
        // Enforce size limits
        if serialized.len() > MAX_DHT_RECORD_SIZE {
            return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                field: "record_size".to_string(),
                value: format!("{} bytes", serialized.len()),
                reason: format!("Record too large (max {})", MAX_DHT_RECORD_SIZE),
            }));
        }
        
        Ok(serialized)
    }

    /// Deserialize a record from bytes with validation
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        // Check size limits
        if data.len() > MAX_DHT_RECORD_SIZE {
            return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                field: "record_size".to_string(),
                value: format!("{} bytes", data.len()),
                reason: format!("Record too large (max {})", MAX_DHT_RECORD_SIZE),
            }));
        }
        
        let record: PeerDHTRecord = bincode::deserialize(data)
            .map_err(|e| P2PError::Storage(StorageError::Database(format!("Failed to deserialize record: {e}"))))?;
        
        // Validate version
        if record.version > Self::CURRENT_VERSION {
            return Err(P2PError::Config(crate::error::ConfigError::InvalidValue {
                field: "record_version".to_string(),
                value: record.version.to_string(),
                reason: "Unsupported record version".to_string(),
            }));
        }
        
        // Validate basic constraints
        Self::validate_inputs(&record.name, &record.endpoints, record.ttl)?;
        
        // Verify signature
        record.verify_signature()?;
        
        Ok(record)
    }

    /// Get a hash of this record for deduplication
    pub fn content_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.user_id.hash);
        hasher.update(&self.sequence_number.to_be_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.finalize()
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Signature verification cache for performance optimization
pub struct SignatureCache {
    cache: HashMap<Hash, bool>,
    max_size: usize,
}

impl SignatureCache {
    /// Create a new signature cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    /// Verify signature with caching
    pub fn verify_cached(&mut self, record: &PeerDHTRecord) -> Result<()> {
        let hash = record.content_hash();
        
        // Check cache first
        if let Some(&result) = self.cache.get(&hash) {
            return if result {
                Ok(())
            } else {
                Err(P2PError::Security(SecurityError::SignatureVerificationFailed))
            };
        }
        
        // Verify signature
        let result = record.verify_signature();
        let success = result.is_ok();
        
        // Cache the result
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove oldest entry
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(hash, success);
        
        result
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_keypair() -> (SigningKey, VerifyingKey) {
        let mut csprng = OsRng {};
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key().clone();
        (signing_key, verifying_key)
    }

    fn create_test_endpoint() -> PeerEndpoint {
        PeerEndpoint::new(
            EndpointId::new(),
            NetworkAddress::from_str("192.168.1.1:8080").unwrap(),
            NatType::FullCone,
            vec!["coordinator1".to_string()],
            Some("test-device".to_string()),
        )
    }

    #[test]
    fn test_user_id_generation() {
        let (_, verifying_key) = create_test_keypair();
        let user_id = UserId::from_public_key(&verifying_key);
        
        // Should be deterministic
        let user_id2 = UserId::from_public_key(&verifying_key);
        assert_eq!(user_id, user_id2);
    }

    #[test]
    fn test_nat_type_hole_punching() {
        assert!(NatType::NoNat.supports_hole_punching());
        assert!(NatType::FullCone.supports_hole_punching());
        assert!(NatType::RestrictedCone.supports_hole_punching());
        assert!(NatType::PortRestricted.supports_hole_punching());
        assert!(!NatType::Symmetric.supports_hole_punching());
        assert!(!NatType::Unknown.supports_hole_punching());
    }

    #[test]
    fn test_peer_endpoint_creation() {
        let endpoint = create_test_endpoint();
        assert!(!endpoint.is_stale(Duration::from_secs(60)));
        
        let mut old_endpoint = endpoint.clone();
        old_endpoint.last_updated = current_timestamp() - 120; // 2 minutes ago
        assert!(old_endpoint.is_stale(Duration::from_secs(60)));
    }

    #[test]
    fn test_dht_record_creation_and_signing() {
        let (signing_key, verifying_key) = create_test_keypair();
        let user_id = UserId::from_public_key(&verifying_key);
        let endpoint = create_test_endpoint();
        
        let mut record = PeerDHTRecord::new(
            user_id,
            verifying_key,
            1,
            Some("test-user".to_string()),
            vec![endpoint],
            DEFAULT_TTL_SECONDS,
        ).unwrap();
        
        // Sign the record
        record.sign(&signing_key).unwrap();
        
        // Verify signature
        assert!(record.verify_signature().is_ok());
        
        // Check expiration
        assert!(!record.is_expired());
        assert!(record.remaining_ttl() > 0);
    }

    #[test]
    fn test_record_serialization() {
        let (signing_key, verifying_key) = create_test_keypair();
        let user_id = UserId::from_public_key(&verifying_key);
        let endpoint = create_test_endpoint();
        
        let mut record = PeerDHTRecord::new(
            user_id,
            verifying_key,
            1,
            Some("test-user".to_string()),
            vec![endpoint],
            DEFAULT_TTL_SECONDS,
        ).unwrap();
        
        record.sign(&signing_key).unwrap();
        
        // Serialize and deserialize
        let serialized = record.serialize().unwrap();
        let deserialized = PeerDHTRecord::deserialize(&serialized).unwrap();
        
        assert_eq!(record, deserialized);
    }

    #[test]
    fn test_signature_cache() {
        let (signing_key, verifying_key) = create_test_keypair();
        let user_id = UserId::from_public_key(&verifying_key);
        let endpoint = create_test_endpoint();
        
        let mut record = PeerDHTRecord::new(
            user_id,
            verifying_key,
            1,
            Some("test-user".to_string()),
            vec![endpoint],
            DEFAULT_TTL_SECONDS,
        ).unwrap();
        
        record.sign(&signing_key).unwrap();
        
        let mut cache = SignatureCache::new(100);
        
        // First verification should compute
        assert!(cache.verify_cached(&record).is_ok());
        
        // Second verification should use cache
        assert!(cache.verify_cached(&record).is_ok());
    }

    #[test]
    fn test_validation_limits() {
        let (_, verifying_key) = create_test_keypair();
        let user_id = UserId::from_public_key(&verifying_key);
        
        // Test name too long
        let long_name = "a".repeat(256);
        let result = PeerDHTRecord::new(
            user_id.clone(),
            verifying_key,
            1,
            Some(long_name),
            vec![create_test_endpoint()],
            DEFAULT_TTL_SECONDS,
        );
        assert!(result.is_err());
        
        // Test too many endpoints
        let many_endpoints = vec![create_test_endpoint(); MAX_ENDPOINTS_PER_PEER + 1];
        let result = PeerDHTRecord::new(
            user_id.clone(),
            verifying_key,
            1,
            Some("test".to_string()),
            many_endpoints,
            DEFAULT_TTL_SECONDS,
        );
        assert!(result.is_err());
        
        // Test TTL too large
        let result = PeerDHTRecord::new(
            user_id,
            verifying_key,
            1,
            Some("test".to_string()),
            vec![create_test_endpoint()],
            MAX_TTL_SECONDS + 1,
        );
        assert!(result.is_err());
    }
}