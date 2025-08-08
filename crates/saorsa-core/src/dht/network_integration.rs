//! Network Integration Layer for DHT v2
//!
//! Bridges DHT operations with p2p-core transport infrastructure, providing
//! efficient protocol handling, connection management, and network optimization.

use crate::dht::{
    core_engine::{DhtKey, NodeId, NodeInfo, NodeCapacity, DhtCoreEngine, ConsistencyLevel},
    witness::{WitnessReceipt, OperationId},
};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, mpsc, Mutex};
use tokio::time::{sleep, timeout};
use lru::LruCache;
use bytes::Bytes;

/// Maximum number of active connections
const MAX_CONNECTIONS: usize = 100;

/// Connection idle timeout
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Keep-alive interval
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

/// Message batch window
const BATCH_WINDOW: Duration = Duration::from_millis(10);

/// Maximum batch size
const MAX_BATCH_SIZE: usize = 65536; // 64KB

/// Retry configuration
const MAX_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(100);

/// DHT protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DhtMessage {
    // Data Operations
    Store {
        key: DhtKey,
        value: Vec<u8>,
        ttl: Duration,
        witnesses: Vec<NodeId>,
    },
    Retrieve {
        key: DhtKey,
        consistency: ConsistencyLevel,
    },
    
    // Node Discovery
    FindNode {
        target: DhtKey,
        count: usize,
    },
    FindValue {
        key: DhtKey,
    },
    
    // Network Management
    Ping {
        timestamp: u64,
        sender_info: NodeInfo,
    },
    Join {
        node_info: NodeInfo,
        capacity: NodeCapacity,
    },
    Leave {
        node_id: NodeId,
        handoff_data: Vec<(DhtKey, NodeId)>,
    },
    
    // Replication
    Replicate {
        key: DhtKey,
        value: Vec<u8>,
        version: u64,
    },
    RepairRequest {
        key: DhtKey,
        missing_shards: Vec<u32>,
    },
}

/// DHT protocol responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DhtResponse {
    // Data Responses
    StoreAck {
        receipt: WitnessReceipt,
        replicas: Vec<NodeId>,
    },
    RetrieveReply {
        value: Option<Vec<u8>>,
        witnesses: Vec<WitnessReceipt>,
    },
    
    // Discovery Responses
    FindNodeReply {
        nodes: Vec<NodeInfo>,
        distances: Vec<u32>,
    },
    FindValueReply {
        value: Option<Vec<u8>>,
        nodes: Vec<NodeInfo>,
    },
    
    // Management Responses
    Pong {
        timestamp: u64,
        node_info: NodeInfo,
    },
    JoinAck {
        routing_info: RoutingInfo,
        neighbors: Vec<NodeInfo>,
    },
    LeaveAck {
        confirmed: bool,
    },
    
    // Error Responses
    Error {
        code: ErrorCode,
        message: String,
        retry_after: Option<Duration>,
    },
}

/// Error codes for DHT operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorCode {
    Timeout,
    ConnectionFailed,
    InvalidMessage,
    NodeNotFound,
    Overloaded,
    InternalError,
}

/// Routing information for new nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingInfo {
    pub bootstrap_nodes: Vec<NodeInfo>,
    pub network_size: usize,
    pub protocol_version: u32,
}

/// Transport trait abstraction
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn connect(&self, address: &str) -> Result<Box<dyn Connection>>;
    async fn listen(&self, address: &str) -> Result<Box<dyn ConnectionListener>>;
}

/// Connection trait abstraction
#[async_trait::async_trait]
pub trait Connection: Send + Sync {
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn receive(&mut self) -> Result<Vec<u8>>;
    async fn close(&mut self) -> Result<()>;
    fn is_alive(&self) -> bool;
    fn remote_address(&self) -> String;
}

/// Connection listener trait
#[async_trait::async_trait]
pub trait ConnectionListener: Send + Sync {
    async fn accept(&mut self) -> Result<Box<dyn Connection>>;
}

/// Connection wrapper with metadata
struct ManagedConnection {
    connection: Box<dyn Connection>,
    node_id: NodeId,
    last_used: Instant,
    message_count: u64,
}

impl ManagedConnection {
    fn new(connection: Box<dyn Connection>, node_id: NodeId) -> Self {
        Self {
            connection,
            node_id,
            last_used: Instant::now(),
            message_count: 0,
        }
    }
    
    fn touch(&mut self) {
        self.last_used = Instant::now();
        self.message_count += 1;
    }
    
    fn is_idle(&self) -> bool {
        self.last_used.elapsed() > IDLE_TIMEOUT
    }
}

/// Connection pool for efficient connection management
pub struct ConnectionPool {
    active_connections: Arc<RwLock<HashMap<NodeId, ManagedConnection>>>,
    connection_cache: Arc<RwLock<LruCache<NodeId, ManagedConnection>>>,
    max_connections: usize,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            connection_cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(max_connections * 2).unwrap()
            ))),
            max_connections,
        }
    }
    
    pub async fn get_connection(
        &self,
        node_id: NodeId,
        transport: &dyn Transport,
        address: &str,
    ) -> Result<Box<dyn Connection>> {
        // Check active connections
        {
            let mut active = self.active_connections.write().await;
            if active.contains_key(&node_id) {
                if let Some(conn) = active.get_mut(&node_id) {
                    if conn.connection.is_alive() {
                        conn.touch();
                        // For now, just return error - this is a mock implementation
                        return Err(anyhow!("Connection already exists"));
                    } else {
                        active.remove(&node_id);
                    }
                }
            }
        }
        
        // Check cache
        {
            let mut cache = self.connection_cache.write().await;
            if let Some(mut conn) = cache.pop(&node_id) {
                if conn.connection.is_alive() {
                    conn.touch();
                    return Ok(conn.connection);
                }
            }
        }
        
        // Create new connection
        let connection = transport.connect(address).await?;
        let managed = ManagedConnection::new(connection, node_id.clone());
        
        // Store in active connections
        let mut active = self.active_connections.write().await;
        if active.len() >= self.max_connections {
            // Evict oldest idle connection
            let to_remove = active.iter()
                .filter(|(_, c)| c.is_idle())
                .min_by_key(|(_, c)| c.last_used)
                .map(|(id, _)| id.clone());
            
            if let Some(id) = to_remove {
                active.remove(&id);
            } else {
                return Err(anyhow!("Connection pool full"));
            }
        }
        
        active.insert(node_id.clone(), managed);
        // Create a new connection for this request
        transport.connect(address).await
    }
    
    pub async fn release_connection(&self, node_id: NodeId) {
        let mut active = self.active_connections.write().await;
        if let Some(conn) = active.remove(&node_id) {
            if conn.connection.is_alive() && !conn.is_idle() {
                let mut cache = self.connection_cache.write().await;
                cache.put(node_id, conn);
            }
        }
    }
    
    pub async fn cleanup_idle_connections(&self) {
        let mut active = self.active_connections.write().await;
        active.retain(|_, conn| !conn.is_idle() && conn.connection.is_alive());
        
        let mut cache = self.connection_cache.write().await;
        cache.clear();
    }
}

/// Message router with batching and priority queuing
pub struct MessageRouter {
    high_priority: Arc<Mutex<VecDeque<(NodeId, DhtMessage)>>>,
    normal_priority: Arc<Mutex<VecDeque<(NodeId, DhtMessage)>>>,
    batch_buffer: Arc<Mutex<Vec<(NodeId, DhtMessage)>>>,
    last_batch_time: Arc<Mutex<Instant>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            high_priority: Arc::new(Mutex::new(VecDeque::new())),
            normal_priority: Arc::new(Mutex::new(VecDeque::new())),
            batch_buffer: Arc::new(Mutex::new(Vec::new())),
            last_batch_time: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    pub async fn queue_message(&self, target: NodeId, message: DhtMessage, high_priority: bool) {
        if high_priority {
            let mut queue = self.high_priority.lock().await;
            queue.push_back((target, message));
        } else {
            let mut queue = self.normal_priority.lock().await;
            queue.push_back((target, message));
        }
    }
    
    pub async fn get_next_batch(&self) -> Vec<(NodeId, DhtMessage)> {
        let mut batch = Vec::new();
        let mut size = 0;
        
        // Process high priority first
        {
            let mut high = self.high_priority.lock().await;
            while let Some((target, msg)) = high.pop_front() {
                let msg_size = bincode::serialized_size(&msg).unwrap_or(0) as usize;
                if size + msg_size > MAX_BATCH_SIZE && !batch.is_empty() {
                    high.push_front((target, msg));
                    break;
                }
                size += msg_size;
                batch.push((target, msg));
            }
        }
        
        // Then normal priority
        if size < MAX_BATCH_SIZE {
            let mut normal = self.normal_priority.lock().await;
            while let Some((target, msg)) = normal.pop_front() {
                let msg_size = bincode::serialized_size(&msg).unwrap_or(0) as usize;
                if size + msg_size > MAX_BATCH_SIZE {
                    normal.push_front((target, msg));
                    break;
                }
                size += msg_size;
                batch.push((target, msg));
            }
        }
        
        batch
    }
}

/// Peer manager for network peer management
pub struct PeerManager {
    known_peers: Arc<RwLock<HashMap<NodeId, PeerInfo>>>,
    bootstrap_nodes: Arc<RwLock<Vec<NodeInfo>>>,
}

#[derive(Debug, Clone)]
struct PeerInfo {
    node_info: NodeInfo,
    last_seen: SystemTime,
    reputation: f64,
    failure_count: u32,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn add_peer(&self, node_info: NodeInfo) {
        let mut peers = self.known_peers.write().await;
        peers.insert(
            node_info.id.clone(),
            PeerInfo {
                node_info,
                last_seen: SystemTime::now(),
                reputation: 1.0,
                failure_count: 0,
            },
        );
    }
    
    pub async fn update_peer_status(&self, node_id: &NodeId, success: bool) {
        let mut peers = self.known_peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.last_seen = SystemTime::now();
            if success {
                peer.reputation = (peer.reputation * 0.95 + 0.05).min(1.0);
                peer.failure_count = 0;
            } else {
                peer.reputation = (peer.reputation * 0.9).max(0.0);
                peer.failure_count += 1;
            }
        }
    }
    
    pub async fn get_peers(&self, count: usize) -> Vec<NodeInfo> {
        let peers = self.known_peers.read().await;
        let mut sorted: Vec<_> = peers.values()
            .filter(|p| p.reputation > 0.5 && p.failure_count < 5)
            .collect();
        
        sorted.sort_by(|a, b| b.reputation.partial_cmp(&a.reputation).unwrap());
        
        sorted.into_iter()
            .take(count)
            .map(|p| p.node_info.clone())
            .collect()
    }
}

/// DHT protocol handler
pub struct DhtProtocolHandler {
    dht_engine: Arc<RwLock<DhtCoreEngine>>,
    request_id_counter: Arc<Mutex<u64>>,
    pending_requests: Arc<RwLock<HashMap<u64, mpsc::Sender<DhtResponse>>>>,
}

impl DhtProtocolHandler {
    pub fn new(dht_engine: Arc<RwLock<DhtCoreEngine>>) -> Self {
        Self {
            dht_engine,
            request_id_counter: Arc::new(Mutex::new(0)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn handle_message(&self, message: DhtMessage) -> Result<DhtResponse> {
        match message {
            DhtMessage::Store { key, value, .. } => {
                let value_len = value.len();
                let mut engine = self.dht_engine.write().await;
                let receipt = engine.store(&key, value).await?;
                
                Ok(DhtResponse::StoreAck {
                    receipt: WitnessReceipt {
                        operation_id: OperationId::new(),
                        operation_type: crate::dht::witness::OperationType::Store,
                        content_hash: crate::dht::content_addressing::ContentAddress::from_bytes(key.as_bytes()),
                        timestamp: chrono::Utc::now(),
                        participating_nodes: vec![crate::dht::witness::NodeId::new("local")],
                        operation_metadata: crate::dht::witness::OperationMetadata {
                            size_bytes: value_len,
                            chunk_count: Some(1),
                            redundancy_level: Some(1.0),
                            custom: std::collections::HashMap::new(),
                        },
                        signature: crate::dht::witness::MlKemSignature::placeholder(),
                        witness_proofs: vec![],
                    },
                    replicas: receipt.stored_at,
                })
            }
            
            DhtMessage::Retrieve { key, .. } => {
                let engine = self.dht_engine.read().await;
                let value = engine.retrieve(&key).await?;
                
                Ok(DhtResponse::RetrieveReply {
                    value,
                    witnesses: Vec::new(),
                })
            }
            
            DhtMessage::FindNode { target, count } => {
                let engine = self.dht_engine.read().await;
                let nodes = engine.find_nodes(&target, count).await?;
                
                Ok(DhtResponse::FindNodeReply {
                    nodes,
                    distances: Vec::new(),
                })
            }
            
            DhtMessage::Ping { timestamp, sender_info } => {
                Ok(DhtResponse::Pong {
                    timestamp,
                    node_info: sender_info,
                })
            }
            
            _ => Ok(DhtResponse::Error {
                code: ErrorCode::InvalidMessage,
                message: "Unsupported message type".to_string(),
                retry_after: None,
            })
        }
    }
}

/// Main network integration layer
pub struct NetworkIntegrationLayer {
    transport: Arc<dyn Transport>,
    connection_pool: Arc<ConnectionPool>,
    message_router: Arc<MessageRouter>,
    peer_manager: Arc<PeerManager>,
    protocol_handler: Arc<DhtProtocolHandler>,
}

impl NetworkIntegrationLayer {
    pub fn new(
        transport: Arc<dyn Transport>,
        dht_engine: Arc<RwLock<DhtCoreEngine>>,
    ) -> Self {
        Self {
            transport,
            connection_pool: Arc::new(ConnectionPool::new(MAX_CONNECTIONS)),
            message_router: Arc::new(MessageRouter::new()),
            peer_manager: Arc::new(PeerManager::new()),
            protocol_handler: Arc::new(DhtProtocolHandler::new(dht_engine)),
        }
    }
    
    /// Send a message to a target node
    pub async fn send_message(
        &self,
        target: NodeId,
        message: DhtMessage,
    ) -> Result<DhtResponse> {
        let mut retries = 0;
        let mut delay = INITIAL_RETRY_DELAY;
        
        loop {
            match self.try_send_message(&target, &message).await {
                Ok(response) => {
                    self.peer_manager.update_peer_status(&target, true).await;
                    return Ok(response);
                }
                Err(e) if retries < MAX_RETRIES => {
                    self.peer_manager.update_peer_status(&target, false).await;
                    sleep(delay).await;
                    delay *= 2;
                    retries += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    async fn try_send_message(
        &self,
        target: &NodeId,
        message: &DhtMessage,
    ) -> Result<DhtResponse> {
        // Get peer info
        let peers = self.peer_manager.known_peers.read().await;
        let peer_info = peers.get(target)
            .ok_or_else(|| anyhow!("Unknown peer"))?;
        let address = peer_info.node_info.address.clone();
        drop(peers);
        
        // Get connection
        let mut connection = self.connection_pool
            .get_connection(target.clone(), self.transport.as_ref(), &address)
            .await?;
        
        // Serialize and send message
        let data = bincode::serialize(message)?;
        connection.send(&data).await?;
        
        // Receive response with timeout
        let response_data = timeout(Duration::from_secs(5), connection.receive()).await??;
        let response: DhtResponse = bincode::deserialize(&response_data)?;
        
        // Release connection
        self.connection_pool.release_connection(target.clone()).await;
        
        Ok(response)
    }
    
    /// Broadcast a message to multiple nodes
    pub async fn broadcast_message(
        &self,
        targets: Vec<NodeId>,
        message: DhtMessage,
    ) -> Result<Vec<DhtResponse>> {
        let mut tasks = Vec::new();
        
        for target in targets {
            let self_clone = self.clone();
            let message_clone = message.clone();
            
            tasks.push(tokio::spawn(async move {
                self_clone.send_message(target, message_clone).await
            }));
        }
        
        let mut responses = Vec::new();
        for task in tasks {
            match task.await? {
                Ok(response) => responses.push(response),
                Err(_) => {} // Ignore individual failures in broadcast
            }
        }
        
        Ok(responses)
    }
    
    /// Listen for incoming messages
    pub async fn listen_for_messages(&self, address: &str) -> Result<()> {
        let mut listener = self.transport.listen(address).await?;
        
        loop {
            let connection = listener.accept().await?;
            let handler = self.protocol_handler.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(connection, handler).await {
                    eprintln!("Connection handling error: {}", e);
                }
            });
        }
    }
    
    async fn handle_connection(
        mut connection: Box<dyn Connection>,
        handler: Arc<DhtProtocolHandler>,
    ) -> Result<()> {
        loop {
            let data = connection.receive().await?;
            let message: DhtMessage = bincode::deserialize(&data)?;
            
            let response = handler.handle_message(message).await?;
            let response_data = bincode::serialize(&response)?;
            
            connection.send(&response_data).await?;
        }
    }
    
    /// Discover peers in the network
    pub async fn discover_peers(
        &self,
        bootstrap_nodes: Vec<NodeInfo>,
    ) -> Result<Vec<NodeInfo>> {
        for node in bootstrap_nodes {
            self.peer_manager.add_peer(node.clone()).await;
        }
        
        // Send FindNode messages to bootstrap nodes
        let target = DhtKey::new(b"discover");
        let message = DhtMessage::FindNode {
            target: target.clone(),
            count: 20,
        };
        
        let responses = self.broadcast_message(
            self.peer_manager.get_peers(5).await.into_iter()
                .map(|info| info.id)
                .collect(),
            message,
        ).await?;
        
        let mut discovered = Vec::new();
        for response in responses {
            if let DhtResponse::FindNodeReply { nodes, .. } = response {
                for node in nodes {
                    self.peer_manager.add_peer(node.clone()).await;
                    discovered.push(node);
                }
            }
        }
        
        Ok(discovered)
    }
    
    /// Maintain connections and perform housekeeping
    pub async fn maintain_connections(&self) -> Result<()> {
        loop {
            // Cleanup idle connections
            self.connection_pool.cleanup_idle_connections().await;
            
            // Send keepalive pings
            let peers = self.peer_manager.get_peers(10).await;
            for peer in peers {
                let message = DhtMessage::Ping {
                    timestamp: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    sender_info: peer.clone(),
                };
                
                let _ = self.send_message(peer.id, message).await;
            }
            
            sleep(KEEPALIVE_INTERVAL).await;
        }
    }
}

impl Clone for NetworkIntegrationLayer {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            connection_pool: self.connection_pool.clone(),
            message_router: self.message_router.clone(),
            peer_manager: self.peer_manager.clone(),
            protocol_handler: self.protocol_handler.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock transport for testing
    struct MockTransport;
    
    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn connect(&self, _address: &str) -> Result<Box<dyn Connection>> {
            Ok(Box::new(MockConnection::new()))
        }
        
        async fn listen(&self, _address: &str) -> Result<Box<dyn ConnectionListener>> {
            Ok(Box::new(MockListener))
        }
    }
    
    struct MockConnection {
        alive: bool,
    }
    
    impl MockConnection {
        fn new() -> Self {
            Self { alive: true }
        }
    }
    
    #[async_trait::async_trait]
    impl Connection for MockConnection {
        async fn send(&mut self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
        
        async fn receive(&mut self) -> Result<Vec<u8>> {
            let response = DhtResponse::Pong {
                timestamp: 0,
                node_info: NodeInfo {
                    id: NodeId::random(),
                    address: "mock".to_string(),
                    last_seen: SystemTime::now(),
                    capacity: NodeCapacity::default(),
                },
            };
            Ok(bincode::serialize(&response)?)
        }
        
        async fn close(&mut self) -> Result<()> {
            self.alive = false;
            Ok(())
        }
        
        fn is_alive(&self) -> bool {
            self.alive
        }
        
        fn remote_address(&self) -> String {
            "mock://test".to_string()
        }
    }
    
    struct MockListener;
    
    #[async_trait::async_trait]
    impl ConnectionListener for MockListener {
        async fn accept(&mut self) -> Result<Box<dyn Connection>> {
            Ok(Box::new(MockConnection::new()))
        }
    }
    
    #[tokio::test]
    async fn test_connection_pool() -> Result<()> {
        let pool = ConnectionPool::new(10);
        let transport = MockTransport;
        let node_id = NodeId::random();
        
        let conn1 = pool.get_connection(
            node_id.clone(),
            &transport,
            "mock://test",
        ).await?;
        
        assert!(conn1.is_alive());
        
        pool.release_connection(node_id.clone()).await;
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_message_router() -> Result<()> {
        let router = MessageRouter::new();
        let node_id = NodeId::random();
        
        let message = DhtMessage::Ping {
            timestamp: 0,
            sender_info: NodeInfo {
                id: node_id.clone(),
                address: "test".to_string(),
                last_seen: SystemTime::now(),
                capacity: NodeCapacity::default(),
            },
        };
        
        router.queue_message(node_id.clone(), message.clone(), true).await;
        router.queue_message(node_id.clone(), message.clone(), false).await;
        
        let batch = router.get_next_batch().await;
        assert_eq!(batch.len(), 2);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_peer_manager() -> Result<()> {
        let manager = PeerManager::new();
        
        let node_info = NodeInfo {
            id: NodeId::random(),
            address: "test".to_string(),
            last_seen: SystemTime::now(),
            capacity: NodeCapacity::default(),
        };
        
        manager.add_peer(node_info.clone()).await;
        manager.update_peer_status(&node_info.id, true).await;
        
        let peers = manager.get_peers(10).await;
        assert_eq!(peers.len(), 1);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_network_integration_ping() -> Result<()> {
        let transport = Arc::new(MockTransport);
        let dht_engine = Arc::new(RwLock::new(
            DhtCoreEngine::new(NodeId::random())?
        ));
        
        let network = NetworkIntegrationLayer::new(transport, dht_engine);
        
        let target = NodeId::random();
        let peer_info = NodeInfo {
            id: target.clone(),
            address: "mock://test".to_string(),
            last_seen: SystemTime::now(),
            capacity: NodeCapacity::default(),
        };
        
        network.peer_manager.add_peer(peer_info.clone()).await;
        
        let message = DhtMessage::Ping {
            timestamp: 0,
            sender_info: peer_info,
        };
        
        let response = network.send_message(target, message).await?;
        
        assert!(matches!(response, DhtResponse::Pong { .. }));
        
        Ok(())
    }
}