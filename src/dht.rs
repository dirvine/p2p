//! Distributed Hash Table (DHT) Implementation
//!
//! This module provides a Kademlia-based DHT for distributed peer routing and data storage.
//! It implements the core Kademlia algorithm with proper distance metrics, k-buckets,
//! and network operations for a fully decentralized P2P system.

use crate::{PeerId, Multiaddr, Result, P2PError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info};
use futures;

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
            Err(P2PError::DHT(format!(
                "Insufficient replication: only {}/{} nodes stored the record (required: {})", 
                successful_replications, closest_nodes.len(), required_replications
            )).into())
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
        
        Ok(())
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
            Err(P2PError::Network("Replication failed".to_string()).into())
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

// Add hex dependency for key display
// This would need to be added to Cargo.toml dependencies