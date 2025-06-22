//! Distributed Hash Table (DHT) Implementation
//!
//! This module provides a Kademlia-based DHT for distributed peer routing and data storage.
//! It implements the core Kademlia algorithm with proper distance metrics, k-buckets,
//! and network operations for a fully decentralized P2P system.

use crate::{PeerId, Multiaddr, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// DHT configuration parameters
#[derive(Debug, Clone)]
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

/// DHT node information
#[derive(Debug, Clone)]
pub struct DHTNode {
    /// Node peer ID
    pub peer_id: PeerId,
    /// Node addresses
    pub addresses: Vec<Multiaddr>,
    /// Last seen timestamp
    pub last_seen: Instant,
    /// Node distance from local node
    pub distance: Key,
    /// Connection status
    pub is_connected: bool,
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
    /// Configuration
    config: DHTConfig,
}

/// DHT storage for local records
#[derive(Debug)]
pub struct DHTStorage {
    /// Stored records
    records: RwLock<HashMap<Key, Record>>,
    /// Configuration
    config: DHTConfig,
}

/// Main DHT implementation
#[derive(Debug)]
pub struct DHT {
    /// Local node ID
    local_id: Key,
    /// Routing table
    routing_table: RoutingTable,
    /// Local storage
    storage: DHTStorage,
    /// Configuration
    config: DHTConfig,
}

/// DHT query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DHTQuery {
    /// Find nodes close to a key
    FindNode { key: Key, requester: PeerId },
    /// Find value for a key
    FindValue { key: Key, requester: PeerId },
    /// Store a record
    Store { record: Record, requester: PeerId },
    /// Ping to check node availability
    Ping { requester: PeerId },
}

/// DHT response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DHTResponse {
    /// Response to FindNode query
    Nodes { nodes: Vec<SerializableDHTNode> },
    /// Response to FindValue query
    Value { record: Record },
    /// Response to Store query
    Stored { success: bool },
    /// Response to Ping query
    Pong { responder: PeerId },
    /// Error response
    Error { message: String },
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
            let mut existing = self.nodes.remove(pos).unwrap();
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
        }
    }
    
    /// Add a bootstrap node to the DHT
    pub async fn add_bootstrap_node(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<()> {
        let node = DHTNode::new(peer_id, addresses, &self.local_id);
        self.routing_table.add_node(node).await
    }
    
    /// Store a record in the DHT
    pub async fn put(&self, key: Key, value: Vec<u8>) -> Result<()> {
        let record = Record::new(key.clone(), value, self.local_id.to_hex());
        
        // Store locally first
        self.storage.store(record.clone()).await?;
        
        // Find nodes closest to the key for replication
        let closest_nodes = self.routing_table
            .closest_nodes(&key, self.config.replication_factor)
            .await;
        
        info!("Storing record with key {} on {} nodes", key.to_hex(), closest_nodes.len());
        
        // TODO: Send STORE messages to closest nodes
        // This would be implemented with the transport layer
        
        Ok(())
    }
    
    /// Retrieve a record from the DHT
    pub async fn get(&self, key: &Key) -> Option<Record> {
        // Check local storage first
        if let Some(record) = self.storage.get(key).await {
            if !record.is_expired() {
                return Some(record);
            }
        }
        
        // TODO: Perform iterative lookup to find the record
        // This would query nodes closest to the key
        
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
        
        // TODO: Refresh buckets, republish records, etc.
        
        Ok(())
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

// Add hex dependency for key display
// This would need to be added to Cargo.toml dependencies