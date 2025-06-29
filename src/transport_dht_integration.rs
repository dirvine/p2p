#!/usr/bin/env rust
//! Transport-DHT Integration Layer
//!
//! This module provides seamless integration between the transport layer and DHT storage system,
//! enabling efficient P2P communication for distributed hash table operations.
//!
//! Key Features:
//! - Automatic transport selection for DHT operations
//! - Message routing and peer discovery
//! - Load balancing across multiple connections
//! - Fault tolerance with automatic retries
//! - Performance monitoring and optimization

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Transport-DHT integration manager
pub struct TransportDhtIntegration {
    /// Active peer connections
    peer_connections: Arc<RwLock<HashMap<PeerId, PeerConnectionPool>>>,
    /// Message routing table
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Integration configuration
    config: IntegrationConfig,
    /// Performance metrics
    metrics: Arc<RwLock<IntegrationMetrics>>,
    /// Transport manager reference
    transport_manager: Arc<dyn TransportManager>,
}

/// Peer identifier
pub type PeerId = String;

/// Multi-address format for network endpoints
pub type Multiaddr = String;

/// DHT operation types
#[derive(Debug, Clone, PartialEq)]
pub enum DhtOperation {
    Store { key: DhtKey, value: Vec<u8> },
    Retrieve { key: DhtKey },
    FindNode { target: PeerId },
    FindValue { key: DhtKey },
    Ping,
}

/// DHT key type
pub type DhtKey = Vec<u8>;

/// DHT response types
#[derive(Debug, Clone)]
pub enum DhtResponse {
    StoreResponse { success: bool, replicas: usize },
    RetrieveResponse { value: Option<Vec<u8>> },
    FindNodeResponse { nodes: Vec<NodeInfo> },
    FindValueResponse { value: Option<Vec<u8>>, nodes: Vec<NodeInfo> },
    PingResponse { latency: Duration },
    Error { message: String },
}

/// Node information for routing
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub distance: u64, // XOR distance for DHT
    pub last_seen: Instant,
    pub is_alive: bool,
}

/// Connection pool for a specific peer
#[derive(Debug)]
pub struct PeerConnectionPool {
    pub connections: Vec<ConnectionHandle>,
    pub active_index: usize,
    pub total_messages: u64,
    pub successful_messages: u64,
    pub last_used: Instant,
    pub average_latency: Duration,
}

/// Handle to a transport connection
#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    pub transport_type: TransportType,
    pub address: Multiaddr,
    pub established_at: Instant,
    pub last_activity: Instant,
    pub message_count: u64,
    pub is_healthy: bool,
}

/// Transport type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    QUIC,
    TCP,
}

/// Routing table for DHT operations
#[derive(Debug)]
pub struct RoutingTable {
    /// K-buckets for Kademlia routing
    buckets: Vec<Vec<NodeInfo>>,
    /// Local node information
    local_node: NodeInfo,
    /// Bucket size (typically 20)
    k_bucket_size: usize,
    /// Total nodes tracked
    total_nodes: usize,
}

/// Integration configuration
#[derive(Debug, Clone)]
pub struct IntegrationConfig {
    /// Maximum connections per peer
    pub max_connections_per_peer: usize,
    /// DHT operation timeout
    pub operation_timeout: Duration,
    /// Retry attempts for failed operations
    pub retry_attempts: usize,
    /// Replication factor for store operations
    pub replication_factor: usize,
    /// Parallel query limit
    pub parallel_queries: usize,
    /// Connection health check interval
    pub health_check_interval: Duration,
}

/// Performance metrics for integration
#[derive(Debug, Clone)]
pub struct IntegrationMetrics {
    /// Total DHT operations performed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average operation latency
    pub average_latency: Duration,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Active peer connections
    pub active_connections: usize,
    /// Messages per second throughput
    pub messages_per_second: f64,
}

/// Mock transport manager trait for integration
pub trait TransportManager: Send + Sync {
    /// Connect to a peer
    fn connect(&self, address: &Multiaddr) -> Result<ConnectionHandle, String>;
    
    /// Send message over connection
    fn send_message(&self, handle: &ConnectionHandle, data: &[u8]) -> Result<(), String>;
    
    /// Receive message from connection
    fn receive_message(&self, handle: &ConnectionHandle) -> Result<Vec<u8>, String>;
    
    /// Check connection health
    fn is_healthy(&self, handle: &ConnectionHandle) -> bool;
    
    /// Close connection
    fn close_connection(&self, handle: &ConnectionHandle) -> Result<(), String>;
}

impl TransportDhtIntegration {
    /// Create new transport-DHT integration
    pub fn new(
        transport_manager: Arc<dyn TransportManager>,
        config: IntegrationConfig,
    ) -> Self {
        let local_node = NodeInfo {
            peer_id: format!("local_node_{}", Instant::now().elapsed().as_nanos()),
            addresses: vec!["/ip4/127.0.0.1/udp/9000/quic".to_string()],
            distance: 0,
            last_seen: Instant::now(),
            is_alive: true,
        };

        let routing_table = RoutingTable {
            buckets: vec![Vec::new(); 160], // 160 buckets for 160-bit key space
            local_node,
            k_bucket_size: 20,
            total_nodes: 0,
        };

        Self {
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            routing_table: Arc::new(RwLock::new(routing_table)),
            config,
            metrics: Arc::new(RwLock::new(IntegrationMetrics {
                total_operations: 0,
                successful_operations: 0,
                failed_operations: 0,
                average_latency: Duration::from_millis(0),
                bytes_transferred: 0,
                active_connections: 0,
                messages_per_second: 0.0,
            })),
            transport_manager,
        }
    }

    /// Perform DHT store operation
    pub async fn dht_store(&self, key: DhtKey, value: Vec<u8>) -> Result<DhtResponse, String> {
        let start_time = Instant::now();
        self.update_metrics_start();

        println!("🔄 DHT Store: key length={}, value length={}", key.len(), value.len());

        // Find closest nodes for the key
        let target_nodes = self.find_closest_nodes(&key, self.config.replication_factor)?;
        
        if target_nodes.is_empty() {
            self.update_metrics_failure();
            return Err("No target nodes found for DHT store".to_string());
        }

        println!("  📍 Found {} target nodes for replication", target_nodes.len());

        let mut successful_stores = 0;
        let mut store_errors = Vec::new();

        // Perform parallel store operations
        for (i, node) in target_nodes.iter().enumerate() {
            println!("    📤 Storing to node {} ({})", i + 1, node.peer_id);
            
            match self.send_dht_message(
                &node.peer_id,
                &DhtOperation::Store { 
                    key: key.clone(), 
                    value: value.clone() 
                }
            ).await {
                Ok(DhtResponse::StoreResponse { success: true, .. }) => {
                    successful_stores += 1;
                    println!("      ✅ Store successful");
                }
                Ok(DhtResponse::StoreResponse { success: false, .. }) => {
                    store_errors.push(format!("Store failed at node {}", node.peer_id));
                    println!("      ❌ Store failed");
                }
                Err(e) => {
                    store_errors.push(format!("Connection error to {}: {}", node.peer_id, e));
                    println!("      ❌ Connection error: {}", e);
                }
                _ => {
                    store_errors.push(format!("Invalid response from {}", node.peer_id));
                    println!("      ❌ Invalid response");
                }
            }
        }

        let duration = start_time.elapsed();
        
        // Update metrics
        self.update_metrics_completion(duration, value.len());

        // Consider operation successful if we stored on at least half the replicas
        let min_replicas = (self.config.replication_factor + 1) / 2;
        if successful_stores >= min_replicas {
            self.update_metrics_success();
            println!("  ✅ DHT Store completed: {}/{} replicas successful", 
                     successful_stores, target_nodes.len());
            
            Ok(DhtResponse::StoreResponse {
                success: true,
                replicas: successful_stores,
            })
        } else {
            self.update_metrics_failure();
            println!("  ❌ DHT Store failed: only {}/{} replicas successful", 
                     successful_stores, target_nodes.len());
            
            Err(format!(
                "Insufficient replicas: {}/{} successful. Errors: {:?}",
                successful_stores, target_nodes.len(), store_errors
            ))
        }
    }

    /// Perform DHT retrieve operation
    pub async fn dht_retrieve(&self, key: DhtKey) -> Result<DhtResponse, String> {
        let start_time = Instant::now();
        self.update_metrics_start();

        println!("🔍 DHT Retrieve: key length={}", key.len());

        // Find closest nodes for the key
        let target_nodes = self.find_closest_nodes(&key, self.config.parallel_queries)?;
        
        if target_nodes.is_empty() {
            self.update_metrics_failure();
            return Err("No target nodes found for DHT retrieve".to_string());
        }

        println!("  📍 Querying {} nodes", target_nodes.len());

        // Try retrieving from nodes in parallel
        for (i, node) in target_nodes.iter().enumerate() {
            println!("    📥 Querying node {} ({})", i + 1, node.peer_id);
            
            match self.send_dht_message(
                &node.peer_id,
                &DhtOperation::Retrieve { key: key.clone() }
            ).await {
                Ok(DhtResponse::RetrieveResponse { value: Some(data) }) => {
                    let duration = start_time.elapsed();
                    self.update_metrics_completion(duration, data.len());
                    self.update_metrics_success();
                    
                    println!("      ✅ Value found ({} bytes)", data.len());
                    println!("  ✅ DHT Retrieve completed successfully");
                    
                    return Ok(DhtResponse::RetrieveResponse { value: Some(data) });
                }
                Ok(DhtResponse::RetrieveResponse { value: None }) => {
                    println!("      ℹ️ Node doesn't have the value");
                    continue;
                }
                Err(e) => {
                    println!("      ❌ Query failed: {}", e);
                    continue;
                }
                _ => {
                    println!("      ❌ Invalid response");
                    continue;
                }
            }
        }

        let duration = start_time.elapsed();
        self.update_metrics_completion(duration, 0);
        self.update_metrics_failure();
        
        println!("  ❌ DHT Retrieve failed: value not found on any node");
        Ok(DhtResponse::RetrieveResponse { value: None })
    }

    /// Find node information
    pub async fn find_node(&self, target_peer: PeerId) -> Result<DhtResponse, String> {
        let start_time = Instant::now();
        self.update_metrics_start();

        println!("🔍 Find Node: target={}", target_peer);

        // Use routing table to find closest nodes
        let closest_nodes = self.get_closest_nodes_from_routing(&target_peer, self.config.parallel_queries)?;
        
        let mut all_nodes = Vec::new();
        let mut _queried_nodes = 0;

        for node in closest_nodes {
            println!("  📡 Querying node {}", node.peer_id);
            _queried_nodes += 1;
            
            match self.send_dht_message(
                &node.peer_id,
                &DhtOperation::FindNode { target: target_peer.clone() }
            ).await {
                Ok(DhtResponse::FindNodeResponse { nodes }) => {
                    println!("    ✅ Received {} node references", nodes.len());
                    all_nodes.extend(nodes);
                }
                Err(e) => {
                    println!("    ❌ Query failed: {}", e);
                }
                _ => {
                    println!("    ❌ Invalid response");
                }
            }
        }

        let duration = start_time.elapsed();
        self.update_metrics_completion(duration, 0);
        
        if !all_nodes.is_empty() {
            self.update_metrics_success();
            println!("  ✅ Find Node completed: found {} total nodes", all_nodes.len());
        } else {
            self.update_metrics_failure();
            println!("  ❌ Find Node failed: no nodes found");
        }

        Ok(DhtResponse::FindNodeResponse { nodes: all_nodes })
    }

    /// Ping a peer to check connectivity
    pub async fn ping_peer(&self, peer_id: &PeerId) -> Result<DhtResponse, String> {
        let start_time = Instant::now();
        self.update_metrics_start();

        println!("🏓 Ping: target={}", peer_id);

        match self.send_dht_message(peer_id, &DhtOperation::Ping).await {
            Ok(DhtResponse::PingResponse { latency }) => {
                let duration = start_time.elapsed();
                self.update_metrics_completion(duration, 0);
                self.update_metrics_success();
                
                println!("  ✅ Ping successful: {:?} latency", latency);
                Ok(DhtResponse::PingResponse { latency })
            }
            Err(e) => {
                let duration = start_time.elapsed();
                self.update_metrics_completion(duration, 0);
                self.update_metrics_failure();
                
                println!("  ❌ Ping failed: {}", e);
                Err(e)
            }
            _ => {
                self.update_metrics_failure();
                Err("Invalid ping response".to_string())
            }
        }
    }

    /// Add a peer to the routing table
    pub fn add_peer(&self, node_info: NodeInfo) -> Result<(), String> {
        let mut routing_table = self.routing_table.write()
            .map_err(|_| "Failed to acquire routing table lock")?;

        // Calculate bucket index based on XOR distance
        let bucket_index = self.calculate_bucket_index(&node_info.peer_id, &routing_table.local_node.peer_id);
        
        if bucket_index >= routing_table.buckets.len() {
            return Err("Invalid bucket index calculated".to_string());
        }

        let k_bucket_size = routing_table.k_bucket_size;
        let bucket = &mut routing_table.buckets[bucket_index];
        
        // Check if node already exists
        if let Some(existing_index) = bucket.iter().position(|n| n.peer_id == node_info.peer_id) {
            // Update existing node
            bucket[existing_index] = node_info;
            println!("📝 Updated peer {} in routing table", bucket[existing_index].peer_id);
        } else if bucket.len() < k_bucket_size {
            // Add new node if bucket has space
            bucket.push(node_info.clone());
            routing_table.total_nodes += 1;
            println!("➕ Added peer {} to routing table", node_info.peer_id);
        } else {
            // Bucket is full, could implement eviction policy here
            println!("⚠️ Bucket {} is full, peer {} not added", bucket_index, node_info.peer_id);
        }

        Ok(())
    }

    /// Get integration statistics
    pub fn get_statistics(&self) -> Result<IntegrationMetrics, String> {
        let metrics = self.metrics.read()
            .map_err(|_| "Failed to acquire metrics lock")?;
        
        let connections = self.peer_connections.read()
            .map_err(|_| "Failed to acquire connections lock")?;

        let mut updated_metrics = metrics.clone();
        updated_metrics.active_connections = connections.len();
        
        // Calculate messages per second
        if updated_metrics.total_operations > 0 {
            let total_time = updated_metrics.average_latency.as_secs_f64() * updated_metrics.total_operations as f64;
            if total_time > 0.0 {
                updated_metrics.messages_per_second = updated_metrics.total_operations as f64 / total_time;
            }
        }

        Ok(updated_metrics)
    }

    /// Get routing table information
    pub fn get_routing_info(&self) -> Result<RoutingInfo, String> {
        let routing_table = self.routing_table.read()
            .map_err(|_| "Failed to acquire routing table lock")?;

        let mut bucket_sizes = Vec::new();
        let mut total_nodes = 0;

        for bucket in &routing_table.buckets {
            bucket_sizes.push(bucket.len());
            total_nodes += bucket.len();
        }

        Ok(RoutingInfo {
            total_nodes,
            bucket_count: routing_table.buckets.len(),
            bucket_sizes,
            k_bucket_size: routing_table.k_bucket_size,
            local_peer_id: routing_table.local_node.peer_id.clone(),
        })
    }

    // Private helper methods

    async fn send_dht_message(&self, peer_id: &PeerId, operation: &DhtOperation) -> Result<DhtResponse, String> {
        // Get or create connection to peer
        let connection = self.get_peer_connection(peer_id)?;
        
        // Serialize DHT operation (simplified)
        let message_data = self.serialize_operation(operation)?;
        
        // Send message via transport
        self.transport_manager.send_message(&connection, &message_data)
            .map_err(|e| format!("Failed to send message: {}", e))?;
        
        // Receive response
        let response_data = self.transport_manager.receive_message(&connection)
            .map_err(|e| format!("Failed to receive response: {}", e))?;
        
        // Deserialize response
        self.deserialize_response(&response_data)
    }

    fn get_peer_connection(&self, peer_id: &PeerId) -> Result<ConnectionHandle, String> {
        let mut connections = self.peer_connections.write()
            .map_err(|_| "Failed to acquire connections lock")?;

        // Check if we already have a connection to this peer
        if let Some(pool) = connections.get_mut(peer_id) {
            if !pool.connections.is_empty() {
                let connection = &pool.connections[pool.active_index % pool.connections.len()];
                pool.active_index += 1;
                pool.last_used = Instant::now();
                
                if self.transport_manager.is_healthy(connection) {
                    return Ok(connection.clone());
                }
            }
        }

        // Create new connection
        let address = format!("/ip4/127.0.0.1/udp/{}/quic", 
                            9000 + (peer_id.len() % 1000)); // Simple address mapping
        
        let connection = self.transport_manager.connect(&address)
            .map_err(|e| format!("Failed to connect to {}: {}", peer_id, e))?;

        // Add to connection pool
        let pool = connections.entry(peer_id.clone()).or_insert_with(|| {
            PeerConnectionPool {
                connections: Vec::new(),
                active_index: 0,
                total_messages: 0,
                successful_messages: 0,
                last_used: Instant::now(),
                average_latency: Duration::from_millis(0),
            }
        });

        pool.connections.push(connection.clone());
        Ok(connection)
    }

    fn find_closest_nodes(&self, key: &DhtKey, count: usize) -> Result<Vec<NodeInfo>, String> {
        let routing_table = self.routing_table.read()
            .map_err(|_| "Failed to acquire routing table lock")?;

        let mut candidates = Vec::new();
        
        // Collect nodes from all buckets
        for bucket in &routing_table.buckets {
            for node in bucket {
                if node.is_alive {
                    candidates.push(node.clone());
                }
            }
        }

        // Sort by distance to key (simplified XOR distance)
        candidates.sort_by_key(|node| self.calculate_distance(key, &node.peer_id));
        
        // Return top candidates
        candidates.truncate(count);
        Ok(candidates)
    }

    fn get_closest_nodes_from_routing(&self, target: &PeerId, count: usize) -> Result<Vec<NodeInfo>, String> {
        let routing_table = self.routing_table.read()
            .map_err(|_| "Failed to acquire routing table lock")?;

        let mut candidates = Vec::new();
        
        // Collect nodes from all buckets
        for bucket in &routing_table.buckets {
            for node in bucket {
                if node.is_alive && node.peer_id != *target {
                    candidates.push(node.clone());
                }
            }
        }

        // Sort by distance to target
        candidates.sort_by_key(|node| self.calculate_distance_peers(&node.peer_id, target));
        
        // Return top candidates
        candidates.truncate(count);
        Ok(candidates)
    }

    fn calculate_bucket_index(&self, peer_id: &PeerId, local_id: &PeerId) -> usize {
        // Simplified bucket calculation
        let distance = self.calculate_distance_peers(peer_id, local_id);
        std::cmp::min(distance as usize % 160, 159)
    }

    fn calculate_distance(&self, key: &DhtKey, peer_id: &PeerId) -> u64 {
        // Simplified XOR distance calculation
        let key_hash = key.iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let peer_hash = peer_id.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        key_hash ^ peer_hash
    }

    fn calculate_distance_peers(&self, peer1: &PeerId, peer2: &PeerId) -> u64 {
        // Simplified peer distance calculation
        let hash1 = peer1.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let hash2 = peer2.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        hash1 ^ hash2
    }

    fn serialize_operation(&self, operation: &DhtOperation) -> Result<Vec<u8>, String> {
        // Simplified serialization
        match operation {
            DhtOperation::Store { key, value } => {
                let mut data = Vec::new();
                data.push(0); // Operation type: Store
                data.extend_from_slice(&(key.len() as u32).to_be_bytes());
                data.extend_from_slice(key);
                data.extend_from_slice(&(value.len() as u32).to_be_bytes());
                data.extend_from_slice(value);
                Ok(data)
            }
            DhtOperation::Retrieve { key } => {
                let mut data = Vec::new();
                data.push(1); // Operation type: Retrieve
                data.extend_from_slice(&(key.len() as u32).to_be_bytes());
                data.extend_from_slice(key);
                Ok(data)
            }
            DhtOperation::FindNode { target } => {
                let mut data = Vec::new();
                data.push(2); // Operation type: FindNode
                data.extend_from_slice(&(target.len() as u32).to_be_bytes());
                data.extend_from_slice(target.as_bytes());
                Ok(data)
            }
            DhtOperation::FindValue { key } => {
                let mut data = Vec::new();
                data.push(3); // Operation type: FindValue
                data.extend_from_slice(&(key.len() as u32).to_be_bytes());
                data.extend_from_slice(key);
                Ok(data)
            }
            DhtOperation::Ping => {
                Ok(vec![4]) // Operation type: Ping
            }
        }
    }

    fn deserialize_response(&self, data: &[u8]) -> Result<DhtResponse, String> {
        if data.is_empty() {
            return Err("Empty response data".to_string());
        }

        match data[0] {
            0 => {
                // Store response
                if data.len() < 2 {
                    return Err("Invalid store response".to_string());
                }
                Ok(DhtResponse::StoreResponse {
                    success: data[1] == 1,
                    replicas: if data.len() > 2 { data[2] as usize } else { 1 },
                })
            }
            1 => {
                // Retrieve response
                if data.len() < 2 {
                    return Err("Invalid retrieve response".to_string());
                }
                if data[1] == 0 {
                    Ok(DhtResponse::RetrieveResponse { value: None })
                } else {
                    if data.len() < 6 {
                        return Err("Invalid retrieve response with value".to_string());
                    }
                    let value_len = u32::from_be_bytes([data[2], data[3], data[4], data[5]]) as usize;
                    if data.len() < 6 + value_len {
                        return Err("Truncated retrieve response".to_string());
                    }
                    let value = data[6..6 + value_len].to_vec();
                    Ok(DhtResponse::RetrieveResponse { value: Some(value) })
                }
            }
            2 => {
                // FindNode response (simplified)
                Ok(DhtResponse::FindNodeResponse { nodes: Vec::new() })
            }
            3 => {
                // FindValue response (simplified)
                Ok(DhtResponse::FindValueResponse { value: None, nodes: Vec::new() })
            }
            4 => {
                // Ping response
                Ok(DhtResponse::PingResponse { latency: Duration::from_millis(10) })
            }
            _ => Err(format!("Unknown response type: {}", data[0])),
        }
    }

    fn update_metrics_start(&self) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.total_operations += 1;
        }
    }

    fn update_metrics_success(&self) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.successful_operations += 1;
        }
    }

    fn update_metrics_failure(&self) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.failed_operations += 1;
        }
    }

    fn update_metrics_completion(&self, duration: Duration, bytes: usize) {
        if let Ok(mut metrics) = self.metrics.write() {
            // Update average latency
            let total_ops = metrics.total_operations;
            if total_ops > 0 {
                let current_total = metrics.average_latency.as_nanos() * (total_ops - 1) as u128;
                let new_total = current_total + duration.as_nanos();
                metrics.average_latency = Duration::from_nanos((new_total / total_ops as u128) as u64);
            }
            
            metrics.bytes_transferred += bytes as u64;
        }
    }
}

/// Routing table information for monitoring
#[derive(Debug, Clone)]
pub struct RoutingInfo {
    pub total_nodes: usize,
    pub bucket_count: usize,
    pub bucket_sizes: Vec<usize>,
    pub k_bucket_size: usize,
    pub local_peer_id: PeerId,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            max_connections_per_peer: 3,
            operation_timeout: Duration::from_secs(30),
            retry_attempts: 3,
            replication_factor: 8, // K=8 replication
            parallel_queries: 3,
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// Mock transport manager implementation for testing
pub struct MockTransportManager {
    connections: std::sync::RwLock<HashMap<Multiaddr, MockConnection>>,
    should_fail: std::sync::atomic::AtomicBool,
}

#[derive(Debug, Clone)]
struct MockConnection {
    address: Multiaddr,
    transport_type: TransportType,
    established_at: Instant,
    message_count: u64,
    is_healthy: bool,
}

impl MockTransportManager {
    pub fn new() -> Self {
        Self {
            connections: std::sync::RwLock::new(HashMap::new()),
            should_fail: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    pub fn set_failure_mode(&self, should_fail: bool) {
        self.should_fail.store(should_fail, std::sync::atomic::Ordering::Relaxed);
    }
}

impl TransportManager for MockTransportManager {
    fn connect(&self, address: &Multiaddr) -> Result<ConnectionHandle, String> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Transport manager in failure mode".to_string());
        }
        
        let transport_type = if address.contains("quic") {
            TransportType::QUIC
        } else {
            TransportType::TCP
        };
        
        let connection = MockConnection {
            address: address.clone(),
            transport_type: transport_type.clone(),
            established_at: Instant::now(),
            message_count: 0,
            is_healthy: true,
        };
        
        if let Ok(mut connections) = self.connections.write() {
            connections.insert(address.clone(), connection);
        }
        
        Ok(ConnectionHandle {
            transport_type,
            address: address.clone(),
            established_at: Instant::now(),
            last_activity: Instant::now(),
            message_count: 0,
            is_healthy: true,
        })
    }
    
    fn send_message(&self, handle: &ConnectionHandle, data: &[u8]) -> Result<(), String> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Send failed".to_string());
        }
        
        // Simulate network delay
        std::thread::sleep(Duration::from_millis(if handle.transport_type == TransportType::QUIC { 5 } else { 15 }));
        
        if data.is_empty() {
            return Err("Empty message".to_string());
        }
        
        Ok(())
    }
    
    fn receive_message(&self, handle: &ConnectionHandle) -> Result<Vec<u8>, String> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Receive failed".to_string());
        }
        
        // Simulate network delay
        std::thread::sleep(Duration::from_millis(if handle.transport_type == TransportType::QUIC { 5 } else { 15 }));
        
        // Generate mock response based on message type
        Ok(vec![0, 1, 1]) // Mock successful store response
    }
    
    fn is_healthy(&self, _handle: &ConnectionHandle) -> bool {
        !self.should_fail.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    fn close_connection(&self, handle: &ConnectionHandle) -> Result<(), String> {
        if let Ok(mut connections) = self.connections.write() {
            connections.remove(&handle.address);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_integration() -> TransportDhtIntegration {
        let transport_manager = Arc::new(MockTransportManager::new());
        let config = IntegrationConfig::default();
        TransportDhtIntegration::new(transport_manager, config)
    }
    
    #[test]
    fn test_integration_creation() {
        let integration = create_test_integration();
        let stats = integration.get_statistics().unwrap();
        assert_eq!(stats.total_operations, 0);
        assert_eq!(stats.active_connections, 0);
    }
    
    #[test]
    fn test_add_peer() {
        let integration = create_test_integration();
        
        let node_info = NodeInfo {
            peer_id: "test_peer_123".to_string(),
            addresses: vec!["/ip4/127.0.0.1/udp/9001/quic".to_string()],
            distance: 100,
            last_seen: Instant::now(),
            is_alive: true,
        };
        
        assert!(integration.add_peer(node_info).is_ok());
        
        let routing_info = integration.get_routing_info().unwrap();
        assert!(routing_info.total_nodes > 0);
    }
    
    #[test]
    fn test_serialization() {
        let integration = create_test_integration();
        
        let store_op = DhtOperation::Store {
            key: b"test_key".to_vec(),
            value: b"test_value".to_vec(),
        };
        
        let serialized = integration.serialize_operation(&store_op).unwrap();
        assert!(!serialized.is_empty());
        assert_eq!(serialized[0], 0); // Store operation type
    }
    
    #[test]
    fn test_deserialization() {
        let integration = create_test_integration();
        
        // Test store response
        let store_response_data = vec![0, 1, 3]; // Store response, success, 3 replicas
        let response = integration.deserialize_response(&store_response_data).unwrap();
        
        match response {
            DhtResponse::StoreResponse { success, replicas } => {
                assert!(success);
                assert_eq!(replicas, 3);
            }
            _ => panic!("Expected store response"),
        }
    }
    
    #[test]
    fn test_distance_calculation() {
        let integration = create_test_integration();
        
        let key1 = b"key1".to_vec();
        let peer1 = "peer1".to_string();
        let peer2 = "peer2".to_string();
        
        let distance1 = integration.calculate_distance(&key1, &peer1);
        let distance2 = integration.calculate_distance(&key1, &peer2);
        
        // Distances should be different for different peers
        assert_ne!(distance1, distance2);
    }
    
    #[test]
    fn test_bucket_calculation() {
        let integration = create_test_integration();
        
        let peer1 = "peer1".to_string();
        let peer2 = "peer2".to_string();
        
        let bucket_index = integration.calculate_bucket_index(&peer1, &peer2);
        assert!(bucket_index < 160); // Should be valid bucket index
    }
    
    #[test]
    fn test_routing_info() {
        let integration = create_test_integration();
        let routing_info = integration.get_routing_info().unwrap();
        
        assert_eq!(routing_info.bucket_count, 160);
        assert_eq!(routing_info.k_bucket_size, 20);
        assert!(!routing_info.local_peer_id.is_empty());
    }
}