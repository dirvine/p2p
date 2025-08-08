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


//! Distributed Hash Table (DHT) Implementation
//!
//! This module provides a Kademlia-based DHT for distributed peer routing and data storage.
//! It implements the core Kademlia algorithm with proper distance metrics, k-buckets,
//! and network operations for a fully decentralized P2P system.
//!
//! The implementation includes S/Kademlia security extensions for enhanced protection
//! against various attacks on the DHT infrastructure.

use crate::{PeerId, Multiaddr};
use crate::error::{P2PError as P2PError, P2pResult as Result};
use crate::validation::{Validate, ValidationContext, validate_peer_id, validate_dht_key, validate_dht_value};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::time::{Duration, Instant, SystemTime};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info};
use futures;

// S/Kademlia security extensions
pub mod skademlia;

// IPv6-based node identity system
pub mod ipv6_identity;

// Enhanced storage with K=8 replication
pub mod enhanced_storage;

/// DHT configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DHTConfig {
    /// Replication parameter (k) - number of nodes to store each record
    pub replication_factor: usize,
    /// Maximum nodes per k-bucket
    pub bucket_size: usize,
    /// Concurrency parameter for parallel lookups
    pub alpha: usize,
    /// Record expiration time
    pub record_ttl: Duration,
    /// Refresh interval for buckets
    pub bucket_refresh_interval: Duration,
    /// Republish interval for stored records
    pub republish_interval: Duration,
    /// Maximum distance for considering nodes "close"
    pub max_distance: u8,
}

/// DHT key type with proper Kademlia distance calculation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key {
    /// 256-bit key hash
    hash: [u8; 32],
}

impl Validate for Key {
    fn validate(&self, ctx: &ValidationContext) -> Result<()> {
        // Validate key hash
        validate_dht_key(&self.hash, ctx)?;
        Ok(())
    }
}

/// DHT record containing key-value data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// Record key
    pub key: Key,
    /// Record value
    pub value: Vec<u8>,
    /// Publisher peer ID
    pub publisher: PeerId,
    /// Record creation time
    pub created_at: SystemTime,
    /// Record expiration time
    pub expires_at: SystemTime,
    /// Signature for verification (optional)
    pub signature: Option<Vec<u8>>,
}

impl Validate for Record {
    fn validate(&self, ctx: &ValidationContext) -> Result<()> {
        // Validate key
        self.key.validate(ctx)?;
        
        // Validate value size
        validate_dht_value(&self.value, ctx)?;
        
        // Validate publisher
        validate_peer_id(&self.publisher)?;
        
        // Validate timestamps
        let now = SystemTime::now();
        if self.created_at > now {
            return Err(P2PError::validation("Record creation time is in the future"));
        }
        
        if self.expires_at < self.created_at {
            return Err(P2PError::validation("Record expiration time is before creation time"));
        }
        
        // Validate signature if present
        if let Some(sig) = &self.signature {
            if sig.is_empty() || sig.len() > 512 {
                return Err(P2PError::validation("Invalid signature size"));
            }
        }
        
        Ok(())
    }
}

/// DHT node information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DHTNode {
    /// Node peer ID
    pub peer_id: PeerId,
    /// Node addresses
    pub addresses: Vec<Multiaddr>,
    /// Last seen timestamp (seconds since epoch)
    #[serde(with = "instant_as_secs")]
    pub last_seen: Instant,
    /// Node distance from local node
    pub distance: Key,
    /// Connection status
    pub is_connected: bool,
}

/// Serde helper for Instant serialization
mod instant_as_secs {
    use serde::{Deserializer, Serializer, Deserialize, Serialize};
    use std::time::Instant;
    
    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to approximate seconds since creation
        instant.elapsed().as_secs().serialize(serializer)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        // Return current instant minus the stored duration (approximate)
        Ok(Instant::now() - std::time::Duration::from_secs(secs))
    }
}

/// Serializable DHT node for network transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableDHTNode {
    /// Node peer ID
    pub peer_id: PeerId,
    /// Node addresses
    pub addresses: Vec<Multiaddr>,
    /// Last seen timestamp as seconds since epoch
    pub last_seen_secs: u64,
    /// Node distance from local node
    pub distance: Key,
    /// Connection status
    pub is_connected: bool,
}

/// Kademlia routing table bucket
#[derive(Debug)]
struct KBucket {
    /// Nodes in this bucket (up to k nodes)
    nodes: VecDeque<DHTNode>,
    /// Bucket capacity
    capacity: usize,
    /// Last refresh time
    last_refresh: Instant,
}

/// Kademlia routing table
#[derive(Debug)]
pub struct RoutingTable {
    /// Local node ID
    local_id: Key,
    /// K-buckets indexed by distance
    buckets: Vec<RwLock<KBucket>>,
    /// Configuration (reserved for future use)
    #[allow(dead_code)]
    config: DHTConfig,
}

/// DHT storage for local records
#[derive(Debug)]
pub struct DHTStorage {
    /// Stored records
    records: RwLock<HashMap<Key, Record>>,
    /// Configuration (reserved for future use)
    #[allow(dead_code)]
    config: DHTConfig,
}

/// Main DHT implementation with S/Kademlia security extensions
#[derive(Debug)]
pub struct DHT {
    /// Local node ID
    local_id: Key,
    /// Routing table
    routing_table: RoutingTable,
    /// Local storage
    storage: DHTStorage,
    /// Configuration (reserved for future use)
    #[allow(dead_code)]
    config: DHTConfig,
    /// S/Kademlia security extensions
    pub skademlia: Option<skademlia::SKademlia>,
    /// IPv6-based identity manager
    pub ipv6_identity_manager: Option<ipv6_identity::IPv6DHTIdentityManager>,
}

/// DHT query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DHTQuery {
    /// Find nodes close to a key
    FindNode { 
        /// The key to find nodes near
        key: Key, 
        /// ID of the requesting peer
        requester: PeerId 
    },
    /// Find value for a key
    FindValue { 
        /// The key to find value for
        key: Key, 
        /// ID of the requesting peer
        requester: PeerId 
    },
    /// Store a record
    Store { 
        /// The record to store
        record: Record, 
        /// ID of the requesting peer
        requester: PeerId 
    },
    /// Ping to check node availability
    Ping { 
        /// ID of the requesting peer
        requester: PeerId 
    },
}

/// DHT response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DHTResponse {
    /// Response to FindNode query
    Nodes { 
        /// List of nodes near the requested key
        nodes: Vec<SerializableDHTNode> 
    },
    /// Response to FindValue query
    Value { 
        /// The found record
        record: Record 
    },
    /// Response to Store query
    Stored { 
        /// Whether storage was successful
        success: bool 
    },
    /// Response to Ping query
    Pong { 
        /// ID of the responding peer
        responder: PeerId 
    },
    /// Error response
    Error { 
        /// Error message describing what went wrong
        message: String 
    },
}

/// DHT lookup state for iterative queries
#[derive(Debug)]
pub struct LookupState {
    /// Target key
    pub target: Key,
    /// Nodes queried so far
    pub queried: HashMap<PeerId, Instant>,
    /// Nodes to query next
    pub to_query: VecDeque<DHTNode>,
    /// Closest nodes found
    pub closest: Vec<DHTNode>,
    /// Lookup start time
    pub started_at: Instant,
    /// Maximum nodes to query in parallel
    pub alpha: usize,
}

impl Default for DHTConfig {
    fn default() -> Self {
        Self {
            replication_factor: 20,     // k = 20 (standard Kademlia)
            bucket_size: 20,            // k = 20 nodes per bucket
            alpha: 3,                   // α = 3 concurrent lookups
            record_ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
            bucket_refresh_interval: Duration::from_secs(60 * 60), // 1 hour
            republish_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_distance: 160,          // 160-bit distance space
        }
    }
}

impl Key {
    /// Create a new key from raw data
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();
        Self { hash }
    }
    
    /// Create a key from existing hash
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self { hash }
    }
    
    /// Create a random key
    pub fn random() -> Self {
        use rand::RngCore;
        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        Self { hash }
    }
    
    /// Get key as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }
    
    /// Get key as hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.hash)
    }
    
    /// Calculate XOR distance between two keys (Kademlia distance metric)
    pub fn distance(&self, other: &Key) -> Key {
        let mut result = [0u8; 32];
        for i in 0..32 {
            result[i] = self.hash[i] ^ other.hash[i];
        }
        Key { hash: result }
    }
    
    /// Get the bit length of the distance (number of leading zeros)
    pub fn leading_zeros(&self) -> u32 {
        for (i, &byte) in self.hash.iter().enumerate() {
            if byte != 0 {
                return (i * 8) as u32 + byte.leading_zeros();
            }
        }
        256 // All bits are zero
    }
    
    /// Get bucket index for this key relative to local node
    pub fn bucket_index(&self, local_id: &Key) -> usize {
        let distance = self.distance(local_id);
        let leading_zeros = distance.leading_zeros();
        if leading_zeros >= 255 {
            255 // Maximum bucket index
        } else {
            (255 - leading_zeros) as usize
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.hash[..8]))
    }
}

impl Record {
    /// Create a new record
    pub fn new(key: Key, value: Vec<u8>, publisher: PeerId) -> Self {
        let now = SystemTime::now();
        let ttl = Duration::from_secs(24 * 60 * 60); // 24 hours default
        
        Self {
            key,
            value,
            publisher,
            created_at: now,
            expires_at: now + ttl,
            signature: None,
        }
    }
    
    /// Create a record with custom TTL
    pub fn with_ttl(key: Key, value: Vec<u8>, publisher: PeerId, ttl: Duration) -> Self {
        let now = SystemTime::now();
        
        Self {
            key,
            value,
            publisher,
            created_at: now,
            expires_at: now + ttl,
            signature: None,
        }
    }
    
    /// Check if record has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }
    
    /// Get record age
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::ZERO)
    }
    
    /// Sign the record (placeholder for future cryptographic verification)
    pub fn sign(&mut self, _private_key: &[u8]) -> Result<()> {
        // Placeholder implementation
        // In real implementation, this would create a cryptographic signature
        self.signature = Some(vec![0u8; 64]); // Dummy signature
        Ok(())
    }
    
    /// Verify record signature (placeholder)
    pub fn verify(&self, _public_key: &[u8]) -> bool {
        // Placeholder implementation
        // In real implementation, this would verify the cryptographic signature
        self.signature.is_some()
    }
}

impl DHTNode {
    /// Create a new DHT node
    pub fn new(peer_id: PeerId, addresses: Vec<Multiaddr>, local_id: &Key) -> Self {
        let node_key = Key::new(peer_id.as_bytes());
        let distance = node_key.distance(local_id);
        
        Self {
            peer_id,
            addresses,
            last_seen: Instant::now(),
            distance,
            is_connected: false,
        }
    }
    
    /// Create a new DHT node with explicit key (for testing)
    pub fn new_with_key(peer_id: PeerId, addresses: Vec<Multiaddr>, key: Key) -> Self {
        Self {
            peer_id,
            addresses,
            last_seen: Instant::now(),
            distance: key,
            is_connected: false,
        }
    }
    
    /// Update last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }
    
    /// Check if node is stale
    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed() > timeout
    }
    
    /// Get node key
    pub fn key(&self) -> Key {
        Key::new(self.peer_id.as_bytes())
    }
    
    /// Convert to serializable form
    pub fn to_serializable(&self) -> SerializableDHTNode {
        SerializableDHTNode {
            peer_id: self.peer_id.clone(),
            addresses: self.addresses.clone(),
            last_seen_secs: self.last_seen.elapsed().as_secs(),
            distance: self.distance.clone(),
            is_connected: self.is_connected,
        }
    }
}

impl SerializableDHTNode {
    /// Convert from serializable form to DHTNode
    pub fn to_dht_node(&self) -> DHTNode {
        DHTNode {
            peer_id: self.peer_id.clone(),
            addresses: self.addresses.clone(),
            last_seen: Instant::now() - Duration::from_secs(self.last_seen_secs),
            distance: self.distance.clone(),
            is_connected: self.is_connected,
        }
    }
}

impl KBucket {
    /// Create a new k-bucket
    fn new(capacity: usize) -> Self {
        Self {
            nodes: VecDeque::new(),
            capacity,
            last_refresh: Instant::now(),
        }
    }
    
    /// Add a node to the bucket
    fn add_node(&mut self, node: DHTNode) -> bool {
        // Check if node already exists
        if let Some(pos) = self.nodes.iter().position(|n| n.peer_id == node.peer_id) {
            // Move to front (most recently seen)
            let mut existing = self.nodes.remove(pos)
                .expect("Node position should be valid after successful find");
            existing.touch();
            existing.is_connected = node.is_connected;
            self.nodes.push_front(existing);
            return true;
        }
        
        if self.nodes.len() < self.capacity {
            // Add new node to front
            self.nodes.push_front(node);
            true
        } else {
            // Bucket is full - could implement replacement strategy here
            false
        }
    }
    
    /// Remove a node from the bucket
    fn remove_node(&mut self, peer_id: &PeerId) -> bool {
        if let Some(pos) = self.nodes.iter().position(|n| n.peer_id == *peer_id) {
            self.nodes.remove(pos);
            true
        } else {
            false
        }
    }
    
    /// Get nodes closest to a target
    fn closest_nodes(&self, target: &Key, count: usize) -> Vec<DHTNode> {
        let mut nodes: Vec<_> = self.nodes.iter().cloned().collect();
        nodes.sort_by_key(|node| node.key().distance(target).as_bytes().to_vec());
        nodes.into_iter().take(count).collect()
    }
    
    /// Check if bucket needs refresh
    fn needs_refresh(&self, interval: Duration) -> bool {
        self.last_refresh.elapsed() > interval
    }
}

impl RoutingTable {
    /// Create a new routing table
    pub fn new(local_id: Key, config: DHTConfig) -> Self {
        let mut buckets = Vec::new();
        for _ in 0..256 {
            buckets.push(RwLock::new(KBucket::new(config.bucket_size)));
        }
        
        Self {
            local_id,
            buckets,
            config,
        }
    }
    
    /// Add a node to the routing table
    pub async fn add_node(&self, node: DHTNode) -> Result<()> {
        let bucket_index = node.key().bucket_index(&self.local_id);
        let mut bucket = self.buckets[bucket_index].write().await;
        
        if bucket.add_node(node.clone()) {
            debug!("Added node {} to bucket {}", node.peer_id, bucket_index);
        } else {
            debug!("Bucket {} full, could not add node {}", bucket_index, node.peer_id);
        }
        
        Ok(())
    }
    
    /// Remove a node from the routing table
    pub async fn remove_node(&self, peer_id: &PeerId) -> Result<()> {
        let node_key = Key::new(peer_id.as_bytes());
        let bucket_index = node_key.bucket_index(&self.local_id);
        let mut bucket = self.buckets[bucket_index].write().await;
        
        if bucket.remove_node(peer_id) {
            debug!("Removed node {} from bucket {}", peer_id, bucket_index);
        }
        
        Ok(())
    }
    
    /// Find nodes closest to a target key
    pub async fn closest_nodes(&self, target: &Key, count: usize) -> Vec<DHTNode> {
        let mut all_nodes = Vec::new();
        
        // Check buckets in order of distance from target
        let target_bucket = target.bucket_index(&self.local_id);
        
        // Start with the target bucket and expand outward
        let mut checked = vec![false; 256];
        let mut to_check = VecDeque::new();
        to_check.push_back(target_bucket);
        
        while let Some(bucket_idx) = to_check.pop_front() {
            if checked[bucket_idx] {
                continue;
            }
            checked[bucket_idx] = true;
            
            let bucket = self.buckets[bucket_idx].read().await;
            all_nodes.extend(bucket.closest_nodes(target, bucket.nodes.len()));
            
            // Add adjacent buckets to check
            if bucket_idx > 0 && !checked[bucket_idx - 1] {
                to_check.push_back(bucket_idx - 1);
            }
            if bucket_idx < 255 && !checked[bucket_idx + 1] {
                to_check.push_back(bucket_idx + 1);
            }
            
            // Stop if we have enough nodes
            if all_nodes.len() >= count * 2 {
                break;
            }
        }
        
        // Sort by distance and return closest
        all_nodes.sort_by_key(|node| node.key().distance(target).as_bytes().to_vec());
        all_nodes.into_iter().take(count).collect()
    }
    
    /// Get routing table statistics
    pub async fn stats(&self) -> (usize, usize) {
        let mut total_nodes = 0;
        let mut active_buckets = 0;
        
        for bucket in &self.buckets {
            let bucket_guard = bucket.read().await;
            let node_count = bucket_guard.nodes.len();
            total_nodes += node_count;
            if node_count > 0 {
                active_buckets += 1;
            }
        }
        
        (total_nodes, active_buckets)
    }
}

impl DHTStorage {
    /// Create new DHT storage
    pub fn new(config: DHTConfig) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// Store a record
    pub async fn store(&self, record: Record) -> Result<()> {
        let mut records = self.records.write().await;
        records.insert(record.key.clone(), record);
        Ok(())
    }
    
    /// Retrieve a record
    pub async fn get(&self, key: &Key) -> Option<Record> {
        let records = self.records.read().await;
        records.get(key).cloned()
    }
    
    /// Remove expired records
    pub async fn cleanup_expired(&self) -> usize {
        let mut records = self.records.write().await;
        let initial_count = records.len();
        records.retain(|_, record| !record.is_expired());
        initial_count - records.len()
    }
    
    /// Get all stored records (for republishing)
    pub async fn all_records(&self) -> Vec<Record> {
        let records = self.records.read().await;
        records.values().cloned().collect()
    }
    
    /// Get storage statistics
    pub async fn stats(&self) -> (usize, usize) {
        let records = self.records.read().await;
        let total = records.len();
        let expired = records.values().filter(|r| r.is_expired()).count();
        (total, expired)
    }
}

impl DHT {
    /// Create a new DHT instance
    pub fn new(local_id: Key, config: DHTConfig) -> Self {
        let routing_table = RoutingTable::new(local_id.clone(), config.clone());
        let storage = DHTStorage::new(config.clone());
        
        Self {
            local_id,
            routing_table,
            storage,
            config,
            skademlia: None,
            ipv6_identity_manager: None,
        }
    }
    
    /// Create a new DHT instance with S/Kademlia security extensions
    pub fn new_with_security(local_id: Key, config: DHTConfig, skademlia_config: skademlia::SKademliaConfig) -> Self {
        let routing_table = RoutingTable::new(local_id.clone(), config.clone());
        let storage = DHTStorage::new(config.clone());
        let skademlia = skademlia::SKademlia::new(skademlia_config);
        
        Self {
            local_id,
            routing_table,
            storage,
            config,
            skademlia: Some(skademlia),
            ipv6_identity_manager: None,
        }
    }

    /// Create a new DHT instance with full IPv6 security integration
    pub fn new_with_ipv6_security(
        local_id: Key, 
        config: DHTConfig, 
        skademlia_config: skademlia::SKademliaConfig,
        ipv6_config: ipv6_identity::IPv6DHTConfig
    ) -> Self {
        let routing_table = RoutingTable::new(local_id.clone(), config.clone());
        let storage = DHTStorage::new(config.clone());
        let skademlia = skademlia::SKademlia::new(skademlia_config);
        let ipv6_identity_manager = ipv6_identity::IPv6DHTIdentityManager::new(ipv6_config);
        
        Self {
            local_id,
            routing_table,
            storage,
            config,
            skademlia: Some(skademlia),
            ipv6_identity_manager: Some(ipv6_identity_manager),
        }
    }

    /// Initialize local IPv6 identity
    pub fn set_local_ipv6_identity(&mut self, identity: crate::security::IPv6NodeID) -> Result<()> {
        if let Some(ref mut manager) = self.ipv6_identity_manager {
            // Update local_id to match IPv6 identity
            self.local_id = ipv6_identity::IPv6DHTIdentityManager::generate_dht_key(&identity);
            manager.set_local_identity(identity)?;
            info!("Local IPv6 identity set and DHT key updated");
            Ok(())
        } else {
            Err(P2PError::Security(crate::error::SecurityError::AuthorizationFailed("IPv6 identity manager not enabled".into())))
        }
    }
    
    /// Add a bootstrap node to the DHT
    pub async fn add_bootstrap_node(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<()> {
        let node = DHTNode::new(peer_id, addresses, &self.local_id);
        self.routing_table.add_node(node).await
    }

    /// Add an IPv6-verified node to the DHT
    pub async fn add_ipv6_node(&mut self, peer_id: PeerId, addresses: Vec<Multiaddr>, ipv6_identity: crate::security::IPv6NodeID) -> Result<()> {
        if let Some(ref mut manager) = self.ipv6_identity_manager {
            // Validate the node join with IPv6 security checks
            let base_node = DHTNode::new(peer_id.clone(), addresses, &self.local_id);
            let security_event = manager.validate_node_join(&base_node, &ipv6_identity).await?;
            
            match security_event {
                ipv6_identity::IPv6SecurityEvent::NodeJoined { verification_confidence, .. } => {
                    // Enhance the node with IPv6 identity
                    let ipv6_node = manager.enhance_dht_node(base_node.clone(), ipv6_identity).await?;
                    
                    // Update the DHT key to match IPv6 identity
                    let mut enhanced_base_node = base_node;
                    enhanced_base_node.distance = ipv6_node.get_dht_key().distance(&self.local_id);
                    
                    // Add to routing table
                    self.routing_table.add_node(enhanced_base_node).await?;
                    
                    info!("Added IPv6-verified node {} with confidence {:.2}", peer_id, verification_confidence);
                    Ok(())
                }
                ipv6_identity::IPv6SecurityEvent::VerificationFailed { .. } => {
                    Err(P2PError::Security(crate::error::SecurityError::AuthenticationFailed))
                }
                ipv6_identity::IPv6SecurityEvent::DiversityViolation { subnet_type, .. } => {
                    Err(P2PError::Security(crate::error::SecurityError::AuthorizationFailed(format!("IP diversity violation: {subnet_type}").into())))
                }
                ipv6_identity::IPv6SecurityEvent::NodeBanned { reason, .. } => {
                    Err(P2PError::Security(crate::error::SecurityError::AuthorizationFailed(format!("Node banned: {reason}").into())))
                }
                _ => {
                    Err(P2PError::Security(crate::error::SecurityError::AuthenticationFailed))
                }
            }
        } else {
            // Fallback to regular node addition if IPv6 manager not enabled
            self.add_bootstrap_node(peer_id, addresses).await
        }
    }

    /// Remove node with IPv6 cleanup
    pub async fn remove_ipv6_node(&mut self, peer_id: &PeerId) -> Result<()> {
        // Remove from routing table
        self.routing_table.remove_node(peer_id).await?;
        
        // Clean up IPv6 tracking
        if let Some(ref mut manager) = self.ipv6_identity_manager {
            manager.remove_node(peer_id);
        }
        
        Ok(())
    }

    /// Check if node is banned due to IPv6 security violations
    pub fn is_node_banned(&self, peer_id: &PeerId) -> bool {
        if let Some(ref manager) = self.ipv6_identity_manager {
            manager.is_node_banned(peer_id)
        } else {
            false
        }
    }
    
    /// Store a record in the DHT with replication
    pub async fn put(&self, key: Key, value: Vec<u8>) -> Result<()> {
        let record = Record::new(key.clone(), value, self.local_id.to_hex());
        
        // Store locally first
        self.storage.store(record.clone()).await?;
        
        // Find nodes closest to the key for replication
        let closest_nodes = self.routing_table
            .closest_nodes(&key, self.config.replication_factor)
            .await;
        
        info!("Storing record with key {} on {} nodes", key.to_hex(), closest_nodes.len());
        
        // If no other nodes available, just store locally (single node scenario)
        if closest_nodes.is_empty() {
            info!("No other nodes available for replication, storing only locally");
            return Ok(());
        }
        
        // Replicate to closest nodes (simulated for now)
        let mut successful_replications = 0;
        for node in &closest_nodes {
            if self.replicate_record(&record, node).await.is_ok() {
                successful_replications += 1;
            }
        }
        
        info!("Successfully replicated record {} to {}/{} nodes", 
              key.to_hex(), successful_replications, closest_nodes.len());
        
        // Consider replication successful if we stored to at least 1 node or have reasonable coverage
        let required_replications = if closest_nodes.len() == 1 {
            1
        } else {
            std::cmp::max(1, closest_nodes.len() / 2)
        };
        
        if successful_replications >= required_replications {
            Ok(())
        } else {
            Err(P2PError::Dht(crate::error::DhtError::ReplicationFailed(
                format!(
                    "Insufficient replication for key {}: only {}/{} nodes stored the record (required: {})",
                    key, 
                    successful_replications, closest_nodes.len(), required_replications
                ).into()
            )))
        }
    }
    
    /// Retrieve a record from the DHT with consistency checks
    pub async fn get(&self, key: &Key) -> Option<Record> {
        // Check local storage first
        if let Some(record) = self.storage.get(key).await {
            if !record.is_expired() {
                return Some(record);
            }
        }
        
        // Perform iterative lookup to find the record
        if let Some(record) = self.iterative_find_value(key).await {
            // Store locally for future access (caching)
            if self.storage.store(record.clone()).await.is_ok() {
                debug!("Cached retrieved record with key {}", key.to_hex());
            }
            return Some(record);
        }
        
        None
    }
    
    /// Find nodes close to a key
    pub async fn find_node(&self, key: &Key) -> Vec<DHTNode> {
        self.routing_table.closest_nodes(key, self.config.replication_factor).await
    }
    
    /// Handle incoming DHT query
    pub async fn handle_query(&self, query: DHTQuery) -> DHTResponse {
        match query {
            DHTQuery::FindNode { key, requester: _ } => {
                let nodes = self.find_node(&key).await;
                let serializable_nodes = nodes.into_iter().map(|n| n.to_serializable()).collect();
                DHTResponse::Nodes { nodes: serializable_nodes }
            }
            DHTQuery::FindValue { key, requester: _ } => {
                if let Some(record) = self.storage.get(&key).await {
                    if !record.is_expired() {
                        return DHTResponse::Value { record };
                    }
                }
                let nodes = self.find_node(&key).await;
                let serializable_nodes = nodes.into_iter().map(|n| n.to_serializable()).collect();
                DHTResponse::Nodes { nodes: serializable_nodes }
            }
            DHTQuery::Store { record, requester: _ } => {
                match self.storage.store(record).await {
                    Ok(()) => DHTResponse::Stored { success: true },
                    Err(_) => DHTResponse::Stored { success: false },
                }
            }
            DHTQuery::Ping { requester: _ } => {
                DHTResponse::Pong { responder: self.local_id.to_hex() }
            }
        }
    }
    
    /// Get DHT statistics
    pub async fn stats(&self) -> DHTStats {
        let (total_nodes, active_buckets) = self.routing_table.stats().await;
        let (stored_records, expired_records) = self.storage.stats().await;
        
        DHTStats {
            local_id: self.local_id.clone(),
            total_nodes,
            active_buckets,
            stored_records,
            expired_records,
        }
    }
    
    /// Perform periodic maintenance
    pub async fn maintenance(&self) -> Result<()> {
        // Clean up expired records
        let expired_count = self.storage.cleanup_expired().await;
        if expired_count > 0 {
            debug!("Cleaned up {} expired records", expired_count);
        }
        
        // Republish records that are close to expiration
        self.republish_records().await?;
        
        // Refresh buckets that haven't been active
        self.refresh_buckets().await?;
        
        // Note: S/Kademlia cleanup would happen here in a mutable context
        
        Ok(())
    }
    
    /// Perform secure lookup using S/Kademlia disjoint paths with distance verification
    pub async fn secure_get(&mut self, key: &Key) -> Result<Option<Record>> {
        // Check local storage first
        if let Some(record) = self.storage.get(key).await {
            if !record.is_expired() {
                return Ok(Some(record));
            }
        }
        
        // Check if S/Kademlia is enabled and extract configuration
        let (enable_distance_verification, disjoint_path_count, min_reputation) = if let Some(ref skademlia) = self.skademlia {
            (skademlia.config.enable_distance_verification, 
             skademlia.config.disjoint_path_count,
             skademlia.config.min_routing_reputation)
        } else {
            // Fallback to regular get if S/Kademlia not enabled
            return Ok(self.get(key).await);
        };
        
        // Get initial nodes for disjoint path lookup
        let initial_nodes = self.routing_table
            .closest_nodes(key, disjoint_path_count * 3)
            .await;
        
        if initial_nodes.is_empty() {
            return Ok(None);
        }
        
        // Perform secure lookup with disjoint paths
        let secure_nodes = if let Some(ref mut skademlia) = self.skademlia {
            skademlia.secure_lookup(key.clone(), initial_nodes).await?
        } else {
            return Ok(None);
        };
        
        // Query the securely found nodes with distance verification
        for node in &secure_nodes {
            // Verify node distance before querying if enabled
            if enable_distance_verification {
                let witness_nodes = self.select_witness_nodes(&node.peer_id, 3).await;
                
                let consensus = if let Some(ref mut skademlia) = self.skademlia {
                    skademlia.verify_distance_consensus(&node.peer_id, key, witness_nodes).await?
                } else {
                    continue;
                };
                
                if consensus.confidence < min_reputation {
                    debug!("Skipping node {} due to low distance verification confidence", node.peer_id);
                    continue;
                }
            }
            
            let query = DHTQuery::FindValue { 
                key: key.clone(), 
                requester: self.local_id.to_hex() 
            };
            if let Ok(DHTResponse::Value { record }) = self.simulate_query(node, query).await {
                // Store locally for future access
                let _ = self.storage.store(record.clone()).await;
                return Ok(Some(record));
            }
        }
        
        Ok(None)
    }
    
    /// Store a record using S/Kademlia security-aware node selection
    pub async fn secure_put(&mut self, key: Key, value: Vec<u8>) -> Result<()> {
        let record = Record::new(key.clone(), value, self.local_id.to_hex());
        
        // Store locally first
        self.storage.store(record.clone()).await?;
        
        // Get secure nodes and perform replications
        let secure_nodes = if let Some(ref skademlia) = self.skademlia {
            // Get candidate nodes
            let candidate_nodes = self.routing_table
                .closest_nodes(&key, self.config.replication_factor * 2)
                .await;
            
            // Use S/Kademlia to select secure nodes based on reputation
            skademlia.select_secure_nodes(
                &candidate_nodes, 
                &key, 
                self.config.replication_factor
            )
        } else {
            // Fallback to regular closest nodes
            self.routing_table.closest_nodes(&key, self.config.replication_factor).await
        };
        
        info!("Storing record with key {} on {} secure nodes", key.to_hex(), secure_nodes.len());
        
        // Perform replications and collect results
        let mut replication_results = Vec::new();
        let mut successful_replications = 0;
        
        for node in &secure_nodes {
            let success = self.replicate_record(&record, node).await.is_ok();
            replication_results.push((node.peer_id.clone(), success));
            if success {
                successful_replications += 1;
            }
        }
        
        // Update reputations if S/Kademlia enabled
        if let Some(ref mut skademlia) = self.skademlia {
            for (peer_id, success) in replication_results {
                skademlia.reputation_manager.update_reputation(
                    &peer_id, 
                    success, 
                    Duration::from_millis(100)
                );
            }
        }
        
        if successful_replications > 0 {
            info!("Successfully replicated to {}/{} secure nodes", 
                 successful_replications, secure_nodes.len());
        }
        
        Ok(())
    }
    
    /// Update sibling lists for a key range
    pub async fn update_sibling_list(&mut self, key: Key) -> Result<()> {
        if let Some(ref mut skademlia) = self.skademlia {
            let nodes = self.routing_table.closest_nodes(&key, skademlia.config.sibling_list_size).await;
            skademlia.update_sibling_list(key, nodes);
        }
        Ok(())
    }
    
    /// Validate routing table consistency using S/Kademlia
    pub async fn validate_routing_consistency(&self) -> Result<skademlia::ConsistencyReport> {
        if let Some(ref skademlia) = self.skademlia {
            // Get sample of nodes for validation (using closest_nodes as a proxy)
            let sample_key = Key::random();
            let sample_nodes = self.routing_table.closest_nodes(&sample_key, 100).await;
            skademlia.validate_routing_consistency(&sample_nodes).await
        } else {
            Err(P2PError::Dht(crate::error::DhtError::RoutingError("S/Kademlia not enabled".to_string().into())))
        }
    }
    
    /// Create a distance verification challenge for a peer
    pub fn create_distance_challenge(&mut self, peer_id: &PeerId, key: &Key) -> Option<skademlia::DistanceChallenge> {
        self.skademlia.as_mut()
            .map(|skademlia| skademlia.create_distance_challenge(peer_id, key))
    }
    
    /// Verify a distance proof
    pub fn verify_distance_proof(&self, proof: &skademlia::DistanceProof) -> Result<bool> {
        if let Some(ref skademlia) = self.skademlia {
            skademlia.verify_distance_proof(proof)
        } else {
            Err(P2PError::Dht(crate::error::DhtError::RoutingError("S/Kademlia not enabled".to_string().into())))
        }
    }

    /// Verify distances of multiple nodes using enhanced consensus
    #[allow(dead_code)]
    async fn verify_node_distances(&self, nodes: &[DHTNode], _target_key: &Key, min_reputation: f64) -> Result<Vec<DHTNode>> {
        let mut verified_nodes = Vec::new();
        
        for node in nodes {
            let witness_nodes = self.select_witness_nodes(&node.peer_id, 3).await;
            
            // Only proceed if we have enough witness nodes
            if witness_nodes.len() >= 2 {
                // Simulate consensus verification (simplified for now)
                let consensus_confidence = 0.8; // Placeholder
                
                if consensus_confidence >= min_reputation {
                    verified_nodes.push(node.clone());
                } else {
                    debug!("Node {} failed distance verification with confidence {}", 
                           node.peer_id, consensus_confidence);
                }
            }
        }
        
        Ok(verified_nodes)
    }

    /// Select witness nodes for distance verification  
    async fn select_witness_nodes(&self, target_peer: &PeerId, count: usize) -> Vec<PeerId> {
        // Get nodes that are close to the target but not the target itself
        let target_key = Key::new(target_peer.as_bytes());
        let candidate_nodes = self.routing_table.closest_nodes(&target_key, count * 2).await;
        
        candidate_nodes.into_iter()
            .filter(|node| node.peer_id != *target_peer)
            .take(count)
            .map(|node| node.peer_id)
            .collect()
    }

    /// Create enhanced distance challenge with adaptive difficulty
    pub fn create_enhanced_distance_challenge(&mut self, peer_id: &PeerId, key: &Key, suspected_attack: bool) -> Option<skademlia::EnhancedDistanceChallenge> {
        self.skademlia.as_mut().map(|skademlia| skademlia.create_adaptive_distance_challenge(peer_id, key, suspected_attack))
    }

    /// Verify distance using multi-round challenge protocol  
    pub async fn verify_distance_multi_round(&mut self, challenge: &skademlia::EnhancedDistanceChallenge) -> Result<bool> {
        if let Some(ref mut skademlia) = self.skademlia {
            skademlia.verify_distance_multi_round(challenge).await
        } else {
            Err(P2PError::Dht(crate::error::DhtError::RoutingError("S/Kademlia not enabled".to_string().into())))
        }
    }
    
    /// Get security bucket for a key range
    pub fn get_security_bucket(&mut self, key: &Key) -> Option<&mut skademlia::SecurityBucket> {
        self.skademlia.as_mut()
            .map(|skademlia| skademlia.get_security_bucket(key))
    }
    
    /// Add trusted node to security bucket
    pub async fn add_trusted_node(&mut self, key: &Key, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<()> {
        if let Some(ref mut skademlia) = self.skademlia {
            let node = DHTNode::new(peer_id, addresses, &self.local_id);
            let security_bucket = skademlia.get_security_bucket(key);
            security_bucket.add_trusted_node(node);
        }
        Ok(())
    }

    /// Perform IPv6-enhanced secure get operation
    pub async fn ipv6_secure_get(&mut self, key: &Key) -> Result<Option<Record>> {
        // Check if requester would be banned
        if self.is_node_banned(&self.local_id.to_hex()) {
            return Err(P2PError::Security(crate::error::SecurityError::AuthorizationFailed(
                "Local node is banned".into()
            )));
        }

        // Check local storage first
        if let Some(record) = self.storage.get(key).await {
            if !record.is_expired() {
                return Ok(Some(record));
            }
        }
        
        // Get IPv6-verified nodes for secure lookup
        let verified_nodes = self.get_ipv6_verified_nodes_for_key(key).await?;
        
        if verified_nodes.is_empty() {
            // Fallback to regular secure_get if no IPv6 nodes available
            return self.secure_get(key).await;
        }

        // Perform S/Kademlia secure lookup with IPv6-verified nodes
        if let Some(ref mut skademlia) = self.skademlia {
            let secure_nodes = skademlia.secure_lookup(key.clone(), verified_nodes).await?;
            
            // Query nodes with both distance and IPv6 verification
            for node in &secure_nodes {
                // Additional IPv6 verification
                if let Some(ref manager) = self.ipv6_identity_manager {
                    if let Some(ipv6_node) = manager.get_verified_node(&node.peer_id) {
                        // Check if IPv6 identity needs refresh
                        if ipv6_node.needs_identity_refresh(manager.config.identity_refresh_interval) {
                            debug!("Skipping node {} due to stale IPv6 identity", node.peer_id);
                            continue;
                        }
                    } else {
                        debug!("Skipping node {} without verified IPv6 identity", node.peer_id);
                        continue;
                    }
                }
                
                let query = DHTQuery::FindValue { 
                    key: key.clone(), 
                    requester: self.local_id.to_hex() 
                };
                if let Ok(DHTResponse::Value { record }) = self.simulate_query(node, query).await {
                    // Update IPv6 reputation for successful response
                    if let Some(ref mut manager) = self.ipv6_identity_manager {
                        manager.update_ipv6_reputation(&node.peer_id, true);
                    }
                    
                    // Store locally for future access
                    let _ = self.storage.store(record.clone()).await;
                    return Ok(Some(record));
                }
            }
        }
        
        Ok(None)
    }

    /// Perform IPv6-enhanced secure put operation
    pub async fn ipv6_secure_put(&mut self, key: Key, value: Vec<u8>) -> Result<()> {
        // Check if local node would be banned
        if self.is_node_banned(&self.local_id.to_hex()) {
            return Err(P2PError::Security(crate::error::SecurityError::AuthorizationFailed(
                "Local node is banned".into()
            )));
        }

        let record = Record::new(key.clone(), value, self.local_id.to_hex());
        
        // Store locally first
        self.storage.store(record.clone()).await?;
        
        // Get IPv6-verified nodes for secure replication
        let verified_nodes = self.get_ipv6_verified_nodes_for_key(&key).await?;
        
        // Use S/Kademlia for secure node selection among verified nodes
        let secure_nodes = if let Some(ref skademlia) = self.skademlia {
            skademlia.select_secure_nodes(&verified_nodes, &key, self.config.replication_factor)
        } else {
            verified_nodes.into_iter().take(self.config.replication_factor).collect()
        };

        info!("Storing record with key {} on {} IPv6-verified secure nodes", key.to_hex(), secure_nodes.len());
        
        // Perform replications with IPv6 reputation tracking
        let mut successful_replications = 0;
        
        for node in &secure_nodes {
            let success = self.replicate_record(&record, node).await.is_ok();
            
            // Update IPv6 reputation based on replication success
            if let Some(ref mut manager) = self.ipv6_identity_manager {
                manager.update_ipv6_reputation(&node.peer_id, success);
            }
            
            if success {
                successful_replications += 1;
            }
        }
        
        if successful_replications == 0 && !secure_nodes.is_empty() {
            return Err(P2PError::Dht(crate::error::DhtError::ReplicationFailed(
                format!("Failed to replicate key {} to any IPv6-verified nodes", key).into()
            )));
        }
        
        info!("Successfully replicated to {}/{} IPv6-verified nodes", 
              successful_replications, secure_nodes.len());
        Ok(())
    }

    /// Get IPv6-verified nodes suitable for a key
    async fn get_ipv6_verified_nodes_for_key(&self, key: &Key) -> Result<Vec<DHTNode>> {
        let mut verified_nodes = Vec::new();
        
        // Get closest nodes from routing table
        let candidate_nodes = self.routing_table.closest_nodes(key, self.config.replication_factor * 2).await;
        
        if let Some(ref manager) = self.ipv6_identity_manager {
            for node in candidate_nodes {
                // Check if node has verified IPv6 identity
                if let Some(ipv6_node) = manager.get_verified_node(&node.peer_id) {
                    // Check if identity is still fresh
                    if !ipv6_node.needs_identity_refresh(manager.config.identity_refresh_interval) {
                        // Check if node is not banned
                        if !manager.is_node_banned(&node.peer_id) {
                            verified_nodes.push(node);
                        }
                    }
                }
            }
        } else {
            // If no IPv6 manager, return all candidates
            verified_nodes = candidate_nodes;
        }
        
        Ok(verified_nodes)
    }

    /// Get IPv6 diversity statistics
    pub fn get_ipv6_diversity_stats(&self) -> Option<crate::security::DiversityStats> {
        self.ipv6_identity_manager.as_ref()
            .map(|manager| manager.get_ipv6_diversity_stats())
    }

    /// Cleanup expired IPv6 identities and reputation data
    pub fn cleanup_ipv6_data(&mut self) {
        if let Some(ref mut manager) = self.ipv6_identity_manager {
            manager.cleanup_expired();
        }
    }

    /// Ban a node for IPv6 security violations
    pub fn ban_ipv6_node(&mut self, peer_id: &PeerId, reason: &str) {
        if let Some(ref mut manager) = self.ipv6_identity_manager {
            manager.ban_node(peer_id, reason);
        }
    }

    /// Get local IPv6 identity
    pub fn get_local_ipv6_identity(&self) -> Option<&crate::security::IPv6NodeID> {
        self.ipv6_identity_manager.as_ref()
            .and_then(|manager| manager.get_local_identity())
    }
    
    /// Replicate a record to a specific node
    async fn replicate_record(&self, record: &Record, node: &DHTNode) -> Result<()> {
        // In a real implementation, this would send a STORE message over the network
        // For now, we simulate successful replication to nodes in our routing table
        debug!("Replicating record {} to node {}", record.key.to_hex(), node.peer_id);
        
        // Simulate network delay and occasional failures
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Simulate 95% success rate for replication (high success rate for testing)
        if rand::random::<f64>() < 0.95 {
            Ok(())
        } else {
            Err(P2PError::Network(crate::error::NetworkError::ProtocolError("Replication failed".to_string().into())))
        }
    }
    
    /// Perform iterative lookup to find a value
    async fn iterative_find_value(&self, key: &Key) -> Option<Record> {
        debug!("Starting iterative lookup for key {}", key.to_hex());
        
        let mut lookup_state = LookupState::new(key.clone(), self.config.alpha);
        
        // Start with closest nodes from routing table
        let initial_nodes = self.routing_table.closest_nodes(key, self.config.alpha).await;
        lookup_state.add_nodes(initial_nodes);
        
        // Perform iterative queries
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10;
        
        while !lookup_state.is_complete() && iterations < MAX_ITERATIONS {
            let nodes_to_query = lookup_state.next_nodes();
            if nodes_to_query.is_empty() {
                break;
            }
            
            // Query nodes in parallel
            let mut queries = Vec::new();
            for node in &nodes_to_query {
                let query = DHTQuery::FindValue { 
                    key: key.clone(), 
                    requester: self.local_id.to_hex() 
                };
                queries.push(self.simulate_query(node, query));
            }
            
            // Process responses
            for query_result in futures::future::join_all(queries).await {
                match query_result {
                    Ok(DHTResponse::Value { record }) => {
                        debug!("Found value for key {} in iteration {}", key.to_hex(), iterations);
                        return Some(record);
                    }
                    Ok(DHTResponse::Nodes { nodes }) => {
                        let dht_nodes: Vec<DHTNode> = nodes.into_iter()
                            .map(|n| n.to_dht_node())
                            .collect();
                        lookup_state.add_nodes(dht_nodes);
                    }
                    _ => {
                        // Query failed or returned unexpected response
                        debug!("Query failed during iterative lookup");
                    }
                }
            }
            
            iterations += 1;
        }
        
        debug!("Iterative lookup for key {} completed after {} iterations, value not found", 
               key.to_hex(), iterations);
        None
    }
    
    /// Simulate a query to a remote node (placeholder for real network implementation)
    async fn simulate_query(&self, _node: &DHTNode, query: DHTQuery) -> Result<DHTResponse> {
        // Add some realistic delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Handle the query locally (simulating remote node response)
        Ok(self.handle_query(query).await)
    }
    
    /// Republish records that are close to expiration
    async fn republish_records(&self) -> Result<()> {
        let all_records = self.storage.all_records().await;
        let mut republished_count = 0;
        
        for record in all_records {
            // Republish if record has less than 1/4 of its TTL remaining
            let remaining_ttl = record.expires_at
                .duration_since(SystemTime::now())
                .unwrap_or(Duration::ZERO);
            
            if remaining_ttl < self.config.record_ttl / 4 {
                // Find nodes responsible for this key
                let closest_nodes = self.routing_table
                    .closest_nodes(&record.key, self.config.replication_factor)
                    .await;
                
                // Republish to closest nodes
                for node in &closest_nodes {
                    if self.replicate_record(&record, node).await.is_ok() {
                        republished_count += 1;
                    }
                }
            }
        }
        
        if republished_count > 0 {
            debug!("Republished {} records during maintenance", republished_count);
        }
        
        Ok(())
    }
    
    /// Refresh buckets that haven't been active recently
    async fn refresh_buckets(&self) -> Result<()> {
        let mut refreshed_count = 0;
        
        for bucket_index in 0..256 {
            let needs_refresh = {
                let bucket = self.routing_table.buckets[bucket_index].read().await;
                bucket.needs_refresh(self.config.bucket_refresh_interval)
            };
            
            if needs_refresh {
                // Generate a random key in this bucket's range and perform lookup
                let target_key = self.generate_key_for_bucket(bucket_index);
                let _nodes = self.iterative_find_node(&target_key).await;
                refreshed_count += 1;
                
                // Update bucket refresh time
                {
                    let mut bucket = self.routing_table.buckets[bucket_index].write().await;
                    bucket.last_refresh = Instant::now();
                }
            }
        }
        
        if refreshed_count > 0 {
            debug!("Refreshed {} buckets during maintenance", refreshed_count);
        }
        
        Ok(())
    }
    
    /// Generate a key that would fall into the specified bucket
    fn generate_key_for_bucket(&self, bucket_index: usize) -> Key {
        let mut key_bytes = self.local_id.as_bytes().to_vec();
        
        // Flip the bit at position (255 - bucket_index) to ensure distance
        if bucket_index < 256 {
            let byte_index = (255 - bucket_index) / 8;
            let bit_index = (255 - bucket_index) % 8;
            
            if byte_index < key_bytes.len() {
                key_bytes[byte_index] ^= 1 << bit_index;
            }
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&key_bytes);
        Key::from_hash(hash)
    }
    
    /// Perform iterative node lookup
    async fn iterative_find_node(&self, key: &Key) -> Vec<DHTNode> {
        debug!("Starting iterative node lookup for key {}", key.to_hex());
        
        let mut lookup_state = LookupState::new(key.clone(), self.config.alpha);
        
        // Start with closest nodes from routing table
        let initial_nodes = self.routing_table.closest_nodes(key, self.config.alpha).await;
        lookup_state.add_nodes(initial_nodes);
        
        // Perform iterative queries
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10;
        
        while !lookup_state.is_complete() && iterations < MAX_ITERATIONS {
            let nodes_to_query = lookup_state.next_nodes();
            if nodes_to_query.is_empty() {
                break;
            }
            
            // Query nodes in parallel
            let mut queries = Vec::new();
            for node in &nodes_to_query {
                let query = DHTQuery::FindNode { 
                    key: key.clone(), 
                    requester: self.local_id.to_hex() 
                };
                queries.push(self.simulate_query(node, query));
            }
            
            // Process responses
            for query_result in futures::future::join_all(queries).await {
                if let Ok(DHTResponse::Nodes { nodes }) = query_result {
                    let dht_nodes: Vec<DHTNode> = nodes.into_iter()
                        .map(|n| n.to_dht_node())
                        .collect();
                    lookup_state.add_nodes(dht_nodes);
                }
            }
            
            iterations += 1;
        }
        
        debug!("Iterative node lookup for key {} completed after {} iterations", 
               key.to_hex(), iterations);
        
        // Return the closest nodes found
        lookup_state.closest.into_iter()
            .take(self.config.replication_factor)
            .collect()
    }
    
    /// Check consistency of a record across multiple nodes
    pub async fn check_consistency(&self, key: &Key) -> Result<ConsistencyReport> {
        debug!("Checking consistency for key {}", key.to_hex());
        
        // Find nodes that should have this record
        let closest_nodes = self.routing_table
            .closest_nodes(key, self.config.replication_factor)
            .await;
        
        let mut records_found = Vec::new();
        let mut nodes_queried = 0;
        let mut nodes_responded = 0;
        
        // Query each node for the record
        for node in &closest_nodes {
            nodes_queried += 1;
            
            let query = DHTQuery::FindValue { 
                key: key.clone(), 
                requester: self.local_id.to_hex() 
            };
            
            match self.simulate_query(node, query).await {
                Ok(DHTResponse::Value { record }) => {
                    nodes_responded += 1;
                    records_found.push((node.peer_id.clone(), record));
                }
                Ok(DHTResponse::Nodes { .. }) => {
                    nodes_responded += 1;
                    // Node doesn't have the record
                }
                _ => {
                    // Node didn't respond or error occurred
                }
            }
        }
        
        // Analyze consistency
        let mut consistent = true;
        let mut canonical_record: Option<Record> = None;
        let mut conflicts = Vec::new();
        
        for (node_id, record) in &records_found {
            if let Some(ref canonical) = canonical_record {
                // Check if records match
                if record.value != canonical.value || 
                   record.created_at != canonical.created_at ||
                   record.publisher != canonical.publisher {
                    consistent = false;
                    conflicts.push((node_id.clone(), record.clone()));
                }
            } else {
                canonical_record = Some(record.clone());
            }
        }
        
        let report = ConsistencyReport {
            key: key.clone(),
            nodes_queried,
            nodes_responded,
            records_found: records_found.len(),
            consistent,
            canonical_record,
            conflicts,
            replication_factor: self.config.replication_factor,
        };
        
        debug!("Consistency check for key {}: {} nodes queried, {} responded, {} records found, consistent: {}", 
               key.to_hex(), report.nodes_queried, report.nodes_responded, 
               report.records_found, report.consistent);
        
        Ok(report)
    }
    
    /// Repair inconsistencies for a specific key
    pub async fn repair_record(&self, key: &Key) -> Result<RepairResult> {
        debug!("Starting repair for key {}", key.to_hex());
        
        let consistency_report = self.check_consistency(key).await?;
        
        if consistency_report.consistent {
            return Ok(RepairResult {
                key: key.clone(),
                repairs_needed: false,
                repairs_attempted: 0,
                repairs_successful: 0,
                final_state: "consistent".to_string(),
            });
        }
        
        // Determine the canonical version (use most recent)
        let canonical_record = if let Some(canonical) = consistency_report.canonical_record {
            canonical
        } else {
            return Ok(RepairResult {
                key: key.clone(),
                repairs_needed: false,
                repairs_attempted: 0,
                repairs_successful: 0,
                final_state: "no_records_found".to_string(),
            });
        };
        
        // Find the most recent version among conflicts
        let mut most_recent = canonical_record.clone();
        for (_, conflicted_record) in &consistency_report.conflicts {
            if conflicted_record.created_at > most_recent.created_at {
                most_recent = conflicted_record.clone();
            }
        }
        
        // Replicate the canonical version to all responsible nodes
        let closest_nodes = self.routing_table
            .closest_nodes(key, self.config.replication_factor)
            .await;
        
        let mut repairs_attempted = 0;
        let mut repairs_successful = 0;
        
        for node in &closest_nodes {
            repairs_attempted += 1;
            if self.replicate_record(&most_recent, node).await.is_ok() {
                repairs_successful += 1;
            }
        }
        
        let final_state = if repairs_successful >= (self.config.replication_factor / 2) {
            "repaired".to_string()
        } else {
            "repair_failed".to_string()
        };
        
        debug!("Repair for key {} completed: {}/{} repairs successful, final state: {}", 
               key.to_hex(), repairs_successful, repairs_attempted, final_state);
        
        Ok(RepairResult {
            key: key.clone(),
            repairs_needed: true,
            repairs_attempted,
            repairs_successful,
            final_state,
        })
    }
    
    // ===============================
    // INBOX SYSTEM IMPLEMENTATION
    // ===============================
    
    /// Create a new inbox for a user with infinite TTL
    pub async fn create_inbox(&self, inbox_id: &str, owner_peer_id: PeerId) -> Result<InboxInfo> {
        info!("Creating inbox {} for peer {}", inbox_id, owner_peer_id);
        
        let inbox_key = Key::from_inbox_id(inbox_id);
        
        // Create inbox metadata record with infinite TTL
        let inbox_metadata = InboxMetadata {
            inbox_id: inbox_id.to_string(),
            owner: owner_peer_id.clone(),
            created_at: SystemTime::now(),
            message_count: 0,
            max_messages: 1000, // Configurable limit
            is_public: true,
            access_keys: vec![owner_peer_id.clone()],
        };
        
        let metadata_value = serde_json::to_vec(&inbox_metadata)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        let metadata_record = Record {
            key: inbox_key.clone(),
            value: metadata_value,
            publisher: owner_peer_id.clone(),
            created_at: SystemTime::now(),
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(u64::MAX), // Infinite TTL
            signature: None,
        };
        
        // Store metadata with infinite TTL
        self.put_record_with_infinite_ttl(metadata_record).await?;
        
        // Create empty message index
        let index_key = Key::from_inbox_index(inbox_id);
        let empty_index = InboxMessageIndex {
            inbox_id: inbox_id.to_string(),
            messages: Vec::new(),
            last_updated: SystemTime::now(),
        };
        
        let index_value = serde_json::to_vec(&empty_index)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        let index_record = Record {
            key: index_key,
            value: index_value,
            publisher: owner_peer_id.clone(),
            created_at: SystemTime::now(),
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(u64::MAX), // Infinite TTL
            signature: None,
        };
        
        self.put_record_with_infinite_ttl(index_record).await?;
        
        let inbox_info = InboxInfo {
            inbox_id: inbox_id.to_string(),
            four_word_address: self.generate_four_word_address(inbox_id),
            owner: owner_peer_id,
            created_at: SystemTime::now(),
            message_count: 0,
            is_accessible: true,
        };
        
        info!("Successfully created inbox {} with four-word address: {}", 
              inbox_id, inbox_info.four_word_address);
        
        Ok(inbox_info)
    }
    
    /// Send a message to an inbox
    pub async fn send_message_to_inbox(&self, inbox_id: &str, message: InboxMessage) -> Result<()> {
        info!("Sending message to inbox {}", inbox_id);
        
        // Get current inbox metadata
        let inbox_key = Key::from_inbox_id(inbox_id);
        let metadata_record = self.get(&inbox_key).await
            .ok_or_else(|| P2PError::Dht(crate::error::DhtError::KeyNotFound(
                format!("inbox:{inbox_id}").into()
            )))?;
        
        let mut inbox_metadata: InboxMetadata = serde_json::from_slice(&metadata_record.value)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        // Check message limit
        if inbox_metadata.message_count >= inbox_metadata.max_messages {
            return Err(P2PError::Dht(crate::error::DhtError::StoreFailed(
                format!("Inbox {} is full", inbox_id).into()
            )));
        }
        
        // Create message record with infinite TTL
        let message_key = Key::from_inbox_message(inbox_id, &message.id);
        let message_value = serde_json::to_vec(&message)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        let message_record = Record {
            key: message_key.clone(),
            value: message_value,
            publisher: message.sender.clone(),
            created_at: message.timestamp,
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(u64::MAX), // Infinite TTL
            signature: None,
        };
        
        self.put_record_with_infinite_ttl(message_record).await?;
        
        // Update message index
        let index_key = Key::from_inbox_index(inbox_id);
        let index_record = self.get(&index_key).await
            .ok_or_else(|| P2PError::Dht(crate::error::DhtError::KeyNotFound(
                format!("inbox_index:{inbox_id}").into()
            )))?;
        
        let mut message_index: InboxMessageIndex = serde_json::from_slice(&index_record.value)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        message_index.messages.push(MessageRef {
            message_id: message.id.clone(),
            sender: message.sender.clone(),
            timestamp: message.timestamp,
            message_type: message.message_type.clone(),
        });
        message_index.last_updated = SystemTime::now();
        
        // Update metadata
        inbox_metadata.message_count += 1;
        
        // Store updated index and metadata
        let updated_index_value = serde_json::to_vec(&message_index)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        let updated_metadata_value = serde_json::to_vec(&inbox_metadata)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        let updated_index_record = Record {
            key: index_key,
            value: updated_index_value,
            publisher: message.sender.clone(),
            created_at: SystemTime::now(),
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(u64::MAX),
            signature: None,
        };
        
        let updated_metadata_record = Record {
            key: inbox_key,
            value: updated_metadata_value,
            publisher: message.sender.clone(),
            created_at: SystemTime::now(),
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(u64::MAX),
            signature: None,
        };
        
        self.put_record_with_infinite_ttl(updated_index_record).await?;
        self.put_record_with_infinite_ttl(updated_metadata_record).await?;
        
        info!("Successfully sent message {} to inbox {}", message.id, inbox_id);
        Ok(())
    }
    
    /// Get messages from an inbox
    pub async fn get_inbox_messages(&self, inbox_id: &str, limit: Option<usize>) -> Result<Vec<InboxMessage>> {
        info!("Retrieving messages from inbox {}", inbox_id);
        
        let index_key = Key::from_inbox_index(inbox_id);
        let index_record = self.get(&index_key).await
            .ok_or_else(|| P2PError::Dht(crate::error::DhtError::KeyNotFound(
                format!("inbox:{inbox_id}").into()
            )))?;
        
        let message_index: InboxMessageIndex = serde_json::from_slice(&index_record.value)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
        
        let mut messages = Vec::new();
        let message_refs: Vec<&MessageRef> = if let Some(limit) = limit {
            message_index.messages.iter().rev().take(limit).collect()
        } else {
            message_index.messages.iter().collect()
        };
        
        for message_ref in message_refs {
            let message_key = Key::from_inbox_message(inbox_id, &message_ref.message_id);
            if let Some(message_record) = self.get(&message_key).await {
                if let Ok(message) = serde_json::from_slice::<InboxMessage>(&message_record.value) {
                    messages.push(message);
                }
            }
        }
        
        // Sort messages by timestamp (newest first)
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        info!("Retrieved {} messages from inbox {}", messages.len(), inbox_id);
        Ok(messages)
    }
    
    /// Get inbox information
    pub async fn get_inbox_info(&self, inbox_id: &str) -> Result<Option<InboxInfo>> {
        let inbox_key = Key::from_inbox_id(inbox_id);
        let metadata_record = self.get(&inbox_key).await;
        
        if let Some(record) = metadata_record {
            let metadata: InboxMetadata = serde_json::from_slice(&record.value)
                .map_err(|e| P2PError::Serialization(e.to_string().into()))?;
            
            let inbox_info = InboxInfo {
                inbox_id: inbox_id.to_string(),
                four_word_address: self.generate_four_word_address(inbox_id),
                owner: metadata.owner,
                created_at: metadata.created_at,
                message_count: metadata.message_count,
                is_accessible: true,
            };
            
            Ok(Some(inbox_info))
        } else {
            Ok(None)
        }
    }
    
    /// Store a record with infinite TTL (never expires)
    async fn put_record_with_infinite_ttl(&self, record: Record) -> Result<()> {
        // Store locally first
        self.storage.store(record.clone()).await?;
        
        // Find closest nodes for replication
        let closest_nodes = self.routing_table
            .closest_nodes(&record.key, self.config.replication_factor)
            .await;
        
        // Replicate to closest nodes
        for node in &closest_nodes {
            if let Err(e) = self.replicate_record(&record, node).await {
                debug!("Failed to replicate infinite TTL record to node {}: {}", node.peer_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Generate a four-word address for an inbox
    fn generate_four_word_address(&self, inbox_id: &str) -> String {
        // TODO: Replace with real four_word_networking when available
        // For now, use a simple placeholder implementation
        let words = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta"];
        let mut hash = 0u32;
        
        for byte in inbox_id.as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u32);
        }
        
        let word1 = words[hash as usize % words.len()];
        let word2 = words[(hash >> 8) as usize % words.len()];
        let word3 = words[(hash >> 16) as usize % words.len()];
        
        format!("{word1}.{word2}.{word3}").into()
    }
    
    /// Add a node to the DHT routing table
    pub async fn add_node(&self, node: DHTNode) -> Result<()> {
        self.routing_table.add_node(node).await
    }
    
    /// Remove a node from the DHT routing table
    pub async fn remove_node(&self, peer_id: &PeerId) -> Result<()> {
        self.routing_table.remove_node(peer_id).await
    }
}

/// DHT statistics
#[derive(Debug, Clone)]
pub struct DHTStats {
    /// Local node ID
    pub local_id: Key,
    /// Total nodes in routing table
    pub total_nodes: usize,
    /// Number of active buckets
    pub active_buckets: usize,
    /// Number of stored records
    pub stored_records: usize,
    /// Number of expired records
    pub expired_records: usize,
}

/// Consistency check report
#[derive(Debug, Clone)]
pub struct ConsistencyReport {
    /// Key being checked
    pub key: Key,
    /// Number of nodes queried
    pub nodes_queried: usize,
    /// Number of nodes that responded
    pub nodes_responded: usize,
    /// Number of records found
    pub records_found: usize,
    /// Whether all records are consistent
    pub consistent: bool,
    /// The canonical record (if any)
    pub canonical_record: Option<Record>,
    /// Conflicting records found
    pub conflicts: Vec<(PeerId, Record)>,
    /// Expected replication factor
    pub replication_factor: usize,
}

/// Result of a repair operation
#[derive(Debug, Clone)]
pub struct RepairResult {
    /// Key that was repaired
    pub key: Key,
    /// Whether repairs were needed
    pub repairs_needed: bool,
    /// Number of repair attempts made
    pub repairs_attempted: usize,
    /// Number of successful repairs
    pub repairs_successful: usize,
    /// Final state description
    pub final_state: String,
}

impl LookupState {
    /// Create a new lookup state
    pub fn new(target: Key, alpha: usize) -> Self {
        Self {
            target,
            queried: HashMap::new(),
            to_query: VecDeque::new(),
            closest: Vec::new(),
            started_at: Instant::now(),
            alpha,
        }
    }
    
    /// Add nodes to query
    pub fn add_nodes(&mut self, nodes: Vec<DHTNode>) {
        for node in nodes {
            if !self.queried.contains_key(&node.peer_id) {
                self.to_query.push_back(node);
            }
        }
        
        // Sort by distance to target
        let target = &self.target;
        self.to_query.make_contiguous().sort_by_key(|node| {
            node.key().distance(target).as_bytes().to_vec()
        });
    }
    
    /// Get next nodes to query
    pub fn next_nodes(&mut self) -> Vec<DHTNode> {
        let mut nodes = Vec::new();
        for _ in 0..self.alpha {
            if let Some(node) = self.to_query.pop_front() {
                self.queried.insert(node.peer_id.clone(), Instant::now());
                nodes.push(node);
            } else {
                break;
            }
        }
        nodes
    }
    
    /// Check if lookup is complete
    pub fn is_complete(&self) -> bool {
        self.to_query.is_empty() || self.started_at.elapsed() > Duration::from_secs(30)
    }
}

// ===============================
// INBOX SYSTEM DATA STRUCTURES
// ===============================

/// Inbox metadata stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMetadata {
    pub inbox_id: String,
    pub owner: PeerId,
    pub created_at: SystemTime,
    pub message_count: usize,
    pub max_messages: usize,
    pub is_public: bool,
    pub access_keys: Vec<PeerId>,
}

/// Inbox message stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    pub id: String,
    pub sender: PeerId,
    pub recipient_inbox: String,
    pub content: String,
    pub message_type: String,
    pub timestamp: SystemTime,
    pub attachments: Vec<MessageAttachment>,
}

/// Message attachment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub hash: String,
}

/// Message index for efficient inbox querying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessageIndex {
    pub inbox_id: String,
    pub messages: Vec<MessageRef>,
    pub last_updated: SystemTime,
}

/// Reference to a message in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRef {
    pub message_id: String,
    pub sender: PeerId,
    pub timestamp: SystemTime,
    pub message_type: String,
}

/// Inbox information returned to users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxInfo {
    pub inbox_id: String,
    pub four_word_address: String,
    pub owner: PeerId,
    pub created_at: SystemTime,
    pub message_count: usize,
    pub is_accessible: bool,
}

// ===============================
// KEY EXTENSIONS FOR INBOX SYSTEM
// ===============================

impl Key {
    /// Create a key for inbox metadata
    pub fn from_inbox_id(inbox_id: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"INBOX_METADATA:");
        hasher.update(inbox_id.as_bytes());
        let hash = hasher.finalize();
        Key { hash: hash.into() }
    }
    
    /// Create a key for inbox message index
    pub fn from_inbox_index(inbox_id: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"INBOX_INDEX:");
        hasher.update(inbox_id.as_bytes());
        let hash = hasher.finalize();
        Key { hash: hash.into() }
    }
    
    /// Create a key for a specific message in an inbox
    pub fn from_inbox_message(inbox_id: &str, message_id: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"INBOX_MESSAGE:");
        hasher.update(inbox_id.as_bytes());
        hasher.update(b":");
        hasher.update(message_id.as_bytes());
        let hash = hasher.finalize();
        Key { hash: hash.into() }
    }
}

// Add hex dependency for key display
// This would need to be added to Cargo.toml dependencies