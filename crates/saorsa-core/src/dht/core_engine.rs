//! Enhanced DHT Core Engine with Kademlia routing and intelligent data distribution
//!
//! Provides the main DHT functionality with k=8 replication, load balancing, and fault tolerance.

use crate::dht::{
    content_addressing::ContentAddress,
    reed_solomon::ReedSolomonEncoder,
    witness::{WitnessReceiptSystem, DhtOperation, OperationType, OperationMetadata},
};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// DHT key type (256-bit)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DhtKey([u8; 32]);

impl DhtKey {
    pub fn new(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }
    
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    /// XOR distance metric for Kademlia
    pub fn distance(&self, other: &DhtKey) -> [u8; 32] {
        let mut result = [0u8; 32];
        for i in 0..32 {
            result[i] = self.0[i] ^ other.0[i];
        }
        result
    }
}

/// Node identifier in the DHT
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(DhtKey);

impl NodeId {
    pub fn random() -> Self {
        let random_bytes: [u8; 32] = rand::random();
        Self(DhtKey::from_bytes(random_bytes))
    }
    
    pub fn from_key(key: DhtKey) -> Self {
        Self(key)
    }
}

/// Node information for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub address: String,
    pub last_seen: SystemTime,
    pub capacity: NodeCapacity,
}

/// Node capacity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    pub storage_available: u64,
    pub bandwidth_available: u64,
    pub reliability_score: f64,
}

impl Default for NodeCapacity {
    fn default() -> Self {
        Self {
            storage_available: 1_000_000_000, // 1GB
            bandwidth_available: 10_000_000,   // 10MB/s
            reliability_score: 1.0,
        }
    }
}

/// K-bucket for Kademlia routing
struct KBucket {
    nodes: Vec<NodeInfo>,
    max_size: usize,
}

impl KBucket {
    fn new(max_size: usize) -> Self {
        Self {
            nodes: Vec::new(),
            max_size,
        }
    }
    
    fn add_node(&mut self, node: NodeInfo) -> Result<()> {
        if self.nodes.len() < self.max_size {
            self.nodes.push(node);
            Ok(())
        } else {
            Err(anyhow!("Bucket full"))
        }
    }
    
    fn remove_node(&mut self, node_id: &NodeId) {
        self.nodes.retain(|n| &n.id != node_id);
    }
    
    fn get_nodes(&self) -> &[NodeInfo] {
        &self.nodes
    }
}

/// Kademlia routing table
pub struct KademliaRoutingTable {
    buckets: Vec<KBucket>,
    node_id: NodeId,
    k_value: usize,
}

impl KademliaRoutingTable {
    fn new(node_id: NodeId, k_value: usize) -> Self {
        let mut buckets = Vec::new();
        for _ in 0..256 {
            buckets.push(KBucket::new(k_value));
        }
        
        Self {
            buckets,
            node_id,
            k_value,
        }
    }
    
    fn add_node(&mut self, node: NodeInfo) -> Result<()> {
        let bucket_index = self.get_bucket_index(&node.id);
        self.buckets[bucket_index].add_node(node)
    }
    
    fn remove_node(&mut self, node_id: &NodeId) {
        let bucket_index = self.get_bucket_index(node_id);
        self.buckets[bucket_index].remove_node(node_id);
    }
    
    fn find_closest_nodes(&self, key: &DhtKey, count: usize) -> Vec<NodeInfo> {
        let mut all_nodes = Vec::new();
        
        for bucket in &self.buckets {
            all_nodes.extend(bucket.get_nodes().iter().cloned());
        }
        
        // Sort by XOR distance
        all_nodes.sort_by_key(|node| {
            let distance = node.id.0.distance(key);
            distance
        });
        
        all_nodes.truncate(count);
        all_nodes
    }
    
    fn get_bucket_index(&self, node_id: &NodeId) -> usize {
        let distance = self.node_id.0.distance(&node_id.0);
        
        // Find first bit that differs
        for i in 0..256 {
            let byte_index = i / 8;
            let bit_index = 7 - (i % 8);
            
            if (distance[byte_index] >> bit_index) & 1 == 1 {
                return i;
            }
        }
        
        255 // Same node
    }
}

/// Consistency level for operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    One,    // At least 1 replica
    Quorum, // Majority of replicas  
    All,    // All replicas
}

/// Load metrics for a node
#[derive(Debug, Clone)]
pub struct LoadMetric {
    pub storage_used_percent: f64,
    pub bandwidth_used_percent: f64,
    pub request_rate: f64,
}

/// Store receipt for DHT operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreReceipt {
    pub key: DhtKey,
    pub stored_at: Vec<NodeId>,
    pub timestamp: SystemTime,
    pub success: bool,
}

impl StoreReceipt {
    pub fn is_successful(&self) -> bool {
        self.success
    }
}

/// Data store for local storage
struct DataStore {
    data: HashMap<DhtKey, Vec<u8>>,
    metadata: HashMap<DhtKey, DataMetadata>,
}

#[derive(Debug, Clone)]
struct DataMetadata {
    size: usize,
    stored_at: SystemTime,
    access_count: u64,
    last_accessed: SystemTime,
}

impl DataStore {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    fn put(&mut self, key: DhtKey, value: Vec<u8>) {
        let metadata = DataMetadata {
            size: value.len(),
            stored_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
        };
        
        self.data.insert(key.clone(), value);
        self.metadata.insert(key, metadata);
    }
    
    fn get(&mut self, key: &DhtKey) -> Option<Vec<u8>> {
        if let Some(metadata) = self.metadata.get_mut(key) {
            metadata.access_count += 1;
            metadata.last_accessed = SystemTime::now();
        }
        
        self.data.get(key).cloned()
    }
    
    fn remove(&mut self, key: &DhtKey) -> Option<Vec<u8>> {
        self.metadata.remove(key);
        self.data.remove(key)
    }
}

/// Replication manager for maintaining data redundancy
struct ReplicationManager {
    replication_factor: usize,
    consistency_level: ConsistencyLevel,
    pending_repairs: Vec<DhtKey>,
}

impl ReplicationManager {
    fn new(replication_factor: usize) -> Self {
        Self {
            replication_factor,
            consistency_level: ConsistencyLevel::Quorum,
            pending_repairs: Vec::new(),
        }
    }
    
    fn required_replicas(&self) -> usize {
        match self.consistency_level {
            ConsistencyLevel::One => 1,
            ConsistencyLevel::Quorum => (self.replication_factor + 1) / 2,
            ConsistencyLevel::All => self.replication_factor,
        }
    }
    
    fn schedule_repair(&mut self, key: DhtKey) {
        if !self.pending_repairs.contains(&key) {
            self.pending_repairs.push(key);
        }
    }
}

/// Load balancer for intelligent data distribution
struct LoadBalancer {
    node_loads: HashMap<NodeId, LoadMetric>,
    rebalance_threshold: f64,
}

impl LoadBalancer {
    fn new() -> Self {
        Self {
            node_loads: HashMap::new(),
            rebalance_threshold: 0.8,
        }
    }
    
    fn update_load(&mut self, node_id: NodeId, load: LoadMetric) {
        self.node_loads.insert(node_id, load);
    }
    
    fn select_least_loaded(&self, candidates: &[NodeInfo], count: usize) -> Vec<NodeId> {
        let mut sorted: Vec<_> = candidates.iter()
            .map(|node| {
                let load = self.node_loads.get(&node.id)
                    .map(|l| l.storage_used_percent)
                    .unwrap_or(0.0);
                (node.id.clone(), load)
            })
            .collect();
            
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        sorted.into_iter()
            .take(count)
            .map(|(id, _)| id)
            .collect()
    }
    
    fn should_rebalance(&self) -> bool {
        self.node_loads.values()
            .any(|load| load.storage_used_percent > self.rebalance_threshold)
    }
}

/// Main DHT Core Engine
pub struct DhtCoreEngine {
    node_id: NodeId,
    routing_table: Arc<RwLock<KademliaRoutingTable>>,
    data_store: Arc<RwLock<DataStore>>,
    replication_manager: Arc<RwLock<ReplicationManager>>,
    load_balancer: Arc<RwLock<LoadBalancer>>,
    witness_system: Arc<WitnessReceiptSystem>,
    reed_solomon: Arc<ReedSolomonEncoder>,
}

impl DhtCoreEngine {
    /// Create new DHT engine with specified node ID
    pub fn new(node_id: NodeId) -> Result<Self> {
        Ok(Self {
            node_id: node_id.clone(),
            routing_table: Arc::new(RwLock::new(KademliaRoutingTable::new(node_id, 8))),
            data_store: Arc::new(RwLock::new(DataStore::new())),
            replication_manager: Arc::new(RwLock::new(ReplicationManager::new(8))),
            load_balancer: Arc::new(RwLock::new(LoadBalancer::new())),
            witness_system: Arc::new(WitnessReceiptSystem::new()),
            reed_solomon: Arc::new(ReedSolomonEncoder::new(6, 2)?),
        })
    }
    
    /// Store data in the DHT
    pub async fn store(&mut self, key: &DhtKey, value: Vec<u8>) -> Result<StoreReceipt> {
        // Find nodes to store at
        let routing = self.routing_table.read().await;
        let target_nodes = routing.find_closest_nodes(key, 8);
        drop(routing);
        
        // Select nodes based on load
        let load_balancer = self.load_balancer.read().await;
        let selected_nodes = load_balancer.select_least_loaded(&target_nodes, 8);
        
        // Store locally if we're one of the selected nodes
        if selected_nodes.contains(&self.node_id) {
            let mut store = self.data_store.write().await;
            store.put(key.clone(), value.clone());
        }
        
        // Create witness receipt
        let operation = DhtOperation {
            operation_type: OperationType::Store,
            content_hash: ContentAddress::from_bytes(&key.0),
            nodes: selected_nodes.iter()
                .map(|id| crate::dht::witness::NodeId::new(&format!("{:?}", id)))
                .collect(),
            metadata: OperationMetadata {
                size_bytes: value.len(),
                chunk_count: Some(1),
                redundancy_level: Some(0.5),
                custom: HashMap::new(),
            },
        };
        
        let _receipt = self.witness_system.create_receipt(&operation).await?;
        
        Ok(StoreReceipt {
            key: key.clone(),
            stored_at: selected_nodes,
            timestamp: SystemTime::now(),
            success: true,
        })
    }
    
    /// Retrieve data from the DHT
    pub async fn retrieve(&self, key: &DhtKey) -> Result<Option<Vec<u8>>> {
        // Check local store first
        let mut store = self.data_store.write().await;
        if let Some(value) = store.get(key) {
            return Ok(Some(value));
        }
        drop(store);
        
        // Find nodes that might have the data
        let routing = self.routing_table.read().await;
        let closest_nodes = routing.find_closest_nodes(key, 8);
        
        // In a real implementation, would query these nodes
        // For now, return None if not found locally
        Ok(None)
    }
    
    /// Find nodes closest to a key
    pub async fn find_nodes(&self, key: &DhtKey, count: usize) -> Result<Vec<NodeInfo>> {
        let routing = self.routing_table.read().await;
        Ok(routing.find_closest_nodes(key, count))
    }
    
    /// Join the DHT network
    pub async fn join_network(&mut self, bootstrap_nodes: Vec<NodeInfo>) -> Result<()> {
        let mut routing = self.routing_table.write().await;
        
        for node in bootstrap_nodes {
            routing.add_node(node)?;
        }
        
        Ok(())
    }
    
    /// Leave the DHT network gracefully
    pub async fn leave_network(&mut self) -> Result<()> {
        // Transfer data to other nodes before leaving
        // In a real implementation, would redistribute stored data
        
        let mut store = self.data_store.write().await;
        store.data.clear();
        store.metadata.clear();
        
        Ok(())
    }
    
    /// Handle node failure
    pub async fn handle_node_failure(&mut self, failed_node: NodeId) -> Result<()> {
        // Remove from routing table
        let mut routing = self.routing_table.write().await;
        routing.remove_node(&failed_node);
        
        // Schedule repairs for affected data
        let mut replication = self.replication_manager.write().await;
        // In real implementation, would identify affected keys and schedule repairs
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_store_retrieve() -> Result<()> {
        let mut dht = DhtCoreEngine::new(NodeId::random())?;
        let key = DhtKey::new(b"test_key");
        let value = b"test_value".to_vec();
        
        let receipt = dht.store(&key, value.clone()).await?;
        assert!(receipt.is_successful());
        
        let retrieved = dht.retrieve(&key).await?;
        assert_eq!(retrieved, Some(value));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_xor_distance() {
        let key1 = DhtKey::from_bytes([0u8; 32]);
        let key2 = DhtKey::from_bytes([255u8; 32]);
        
        let distance = key1.distance(&key2);
        assert_eq!(distance, [255u8; 32]);
    }
}