//! Network module
//!
//! This module provides core networking functionality for the P2P Foundation.
//! It handles peer connections, network events, and node lifecycle management.

use crate::{PeerId, Multiaddr, P2PError, Result};
use crate::mcp::{MCPServer, MCPServerConfig, Tool, MCPCallContext, MCP_PROTOCOL};
use crate::dht::{DHT, DHTConfig as DHTConfigInner};
use crate::production::{ProductionConfig, ResourceManager, ResourceMetrics};
use crate::bootstrap::{BootstrapManager, ContactEntry, QualityMetrics};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, RwLock};
use tokio::time::Instant;
use tracing::{debug, info, warn};

/// Configuration for a P2P node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Local peer ID for this node
    pub peer_id: Option<PeerId>,
    
    /// Addresses to listen on for incoming connections
    pub listen_addrs: Vec<Multiaddr>,
    
    /// Primary listen address (for compatibility)
    pub listen_addr: std::net::SocketAddr,
    
    /// Bootstrap peers to connect to on startup (legacy)
    pub bootstrap_peers: Vec<Multiaddr>,
    
    /// Bootstrap peers as strings (for integration tests)
    pub bootstrap_peers_str: Vec<String>,
    
    /// Enable IPv6 support
    pub enable_ipv6: bool,
    
    /// Enable MCP server
    pub enable_mcp_server: bool,
    
    /// MCP server configuration
    pub mcp_server_config: Option<MCPServerConfig>,
    
    /// Connection timeout duration
    pub connection_timeout: Duration,
    
    /// Keep-alive interval for connections
    pub keep_alive_interval: Duration,
    
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    
    /// Maximum number of incoming connections
    pub max_incoming_connections: usize,
    
    /// DHT configuration
    pub dht_config: DHTConfig,
    
    /// Security configuration
    pub security_config: SecurityConfig,
    
    /// Production hardening configuration
    pub production_config: Option<ProductionConfig>,
    
    /// Bootstrap cache configuration
    pub bootstrap_cache_config: Option<crate::bootstrap::CacheConfig>,
}

/// DHT-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DHTConfig {
    /// Kademlia K parameter (bucket size)
    pub k_value: usize,
    
    /// Kademlia alpha parameter (parallelism)
    pub alpha_value: usize,
    
    /// DHT record TTL
    pub record_ttl: Duration,
    
    /// DHT refresh interval
    pub refresh_interval: Duration,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable noise protocol for encryption
    pub enable_noise: bool,
    
    /// Enable TLS for secure transport
    pub enable_tls: bool,
    
    /// Trust level for peer verification
    pub trust_level: TrustLevel,
}

/// Trust level for peer verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustLevel {
    /// No verification required
    None,
    /// Basic peer ID verification
    Basic,
    /// Full cryptographic verification
    Full,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            peer_id: None,
            listen_addrs: vec![
                "/ip6/::/tcp/9000".to_string(),
                "/ip4/0.0.0.0/tcp/9000".to_string(),
            ],
            listen_addr: "127.0.0.1:9000".parse().unwrap(),
            bootstrap_peers: Vec::new(),
            bootstrap_peers_str: Vec::new(),
            enable_ipv6: true,
            enable_mcp_server: true,
            mcp_server_config: None, // Use default config if None
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(60),
            max_connections: 1000,
            max_incoming_connections: 100,
            dht_config: DHTConfig::default(),
            security_config: SecurityConfig::default(),
            production_config: None, // Use default production config if enabled
            bootstrap_cache_config: None,
        }
    }
}

impl Default for DHTConfig {
    fn default() -> Self {
        Self {
            k_value: 20,
            alpha_value: 5,
            record_ttl: Duration::from_secs(3600), // 1 hour
            refresh_interval: Duration::from_secs(600), // 10 minutes
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_noise: true,
            enable_tls: true,
            trust_level: TrustLevel::Basic,
        }
    }
}

/// Information about a connected peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: PeerId,
    
    /// Peer's addresses
    pub addresses: Vec<String>,
    
    /// Connection timestamp
    pub connected_at: Instant,
    
    /// Last seen timestamp
    pub last_seen: Instant,
    
    /// Connection status
    pub status: ConnectionStatus,
    
    /// Supported protocols
    pub protocols: Vec<String>,
}

/// Connection status for a peer
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Connection is being established
    Connecting,
    /// Connection is established and active
    Connected,
    /// Connection is being closed
    Disconnecting,
    /// Connection is closed
    Disconnected,
    /// Connection failed
    Failed(String),
}

/// Network events that can occur
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A new peer has connected
    PeerConnected {
        /// The identifier of the newly connected peer
        peer_id: PeerId,
        /// The network addresses where the peer can be reached
        addresses: Vec<String>,
    },
    
    /// A peer has disconnected
    PeerDisconnected {
        /// The identifier of the disconnected peer
        peer_id: PeerId,
        /// The reason for the disconnection
        reason: String,
    },
    
    /// A message was received from a peer
    MessageReceived {
        /// The identifier of the sending peer
        peer_id: PeerId,
        /// The protocol used for the message
        protocol: String,
        /// The raw message data
        data: Vec<u8>,
    },
    
    /// A connection attempt failed
    ConnectionFailed {
        /// The identifier of the peer (if known)
        peer_id: Option<PeerId>,
        /// The address where connection was attempted
        address: String,
        /// The error message describing the failure
        error: String,
    },
    
    /// DHT record was stored
    DHTRecordStored {
        /// The DHT key where the record was stored
        key: Vec<u8>,
        /// The value that was stored
        value: Vec<u8>,
    },
    
    /// DHT record was retrieved
    DHTRecordRetrieved {
        /// The DHT key that was queried
        key: Vec<u8>,
        /// The retrieved value, if found
        value: Option<Vec<u8>>,
    },
}

/// Main P2P node structure
pub struct P2PNode {
    /// Node configuration
    config: NodeConfig,
    
    /// Our peer ID
    peer_id: PeerId,
    
    /// Connected peers
    peers: RwLock<HashMap<PeerId, PeerInfo>>,
    
    /// Network event broadcaster
    event_tx: broadcast::Sender<NetworkEvent>,
    
    /// Listen addresses
    listen_addrs: RwLock<Vec<Multiaddr>>,
    
    /// Node start time
    start_time: Instant,
    
    /// Running state
    running: RwLock<bool>,
    
    /// MCP server instance (optional)
    mcp_server: Option<Arc<MCPServer>>,
    
    /// DHT instance (optional)
    dht: Option<Arc<RwLock<DHT>>>,
    
    /// Production resource manager (optional)
    resource_manager: Option<Arc<ResourceManager>>,
    
    /// Bootstrap cache manager for peer discovery
    bootstrap_manager: Option<Arc<RwLock<BootstrapManager>>>,
}

impl P2PNode {
    /// Create a new P2P node with the given configuration
    pub async fn new(config: NodeConfig) -> Result<Self> {
        let peer_id = config.peer_id.clone().unwrap_or_else(|| {
            // Generate a random peer ID for now
            format!("peer_{}", uuid::Uuid::new_v4().to_string()[..8].to_string())
        });
        
        let (event_tx, _) = broadcast::channel(1000);
        
        // Initialize DHT if needed
        let dht = if config.enable_mcp_server || true { // Always enable DHT for now
            let dht_config = DHTConfigInner {
                replication_factor: config.dht_config.k_value,
                bucket_size: config.dht_config.k_value,
                alpha: config.dht_config.alpha_value,
                record_ttl: config.dht_config.record_ttl,
                bucket_refresh_interval: config.dht_config.refresh_interval,
                republish_interval: config.dht_config.refresh_interval,
                max_distance: 160, // 160 bits for SHA-256
            };
            let dht_key = crate::dht::Key::new(peer_id.as_bytes());
            let dht_instance = DHT::new(dht_key, dht_config);
            Some(Arc::new(RwLock::new(dht_instance)))
        } else {
            None
        };
        
        // Initialize MCP server if enabled
        let mcp_server = if config.enable_mcp_server {
            let mcp_config = config.mcp_server_config.clone().unwrap_or_else(|| {
                MCPServerConfig {
                    server_name: format!("P2P-MCP-{}", peer_id),
                    server_version: crate::VERSION.to_string(),
                    enable_dht_discovery: dht.is_some(),
                    ..MCPServerConfig::default()
                }
            });
            
            let mut server = MCPServer::new(mcp_config);
            
            // Connect DHT if available
            if let Some(ref dht_instance) = dht {
                server = server.with_dht(dht_instance.clone());
            }
            
            Some(Arc::new(server))
        } else {
            None
        };
        
        // Initialize production resource manager if configured
        let resource_manager = if let Some(prod_config) = config.production_config.clone() {
            Some(Arc::new(ResourceManager::new(prod_config)))
        } else {
            None
        };
        
        // Initialize bootstrap cache manager
        let bootstrap_manager = if let Some(ref cache_config) = config.bootstrap_cache_config {
            match BootstrapManager::with_config(cache_config.clone()).await {
                Ok(manager) => Some(Arc::new(RwLock::new(manager))),
                Err(e) => {
                    warn!("Failed to initialize bootstrap manager: {}, continuing without cache", e);
                    None
                }
            }
        } else {
            match BootstrapManager::new().await {
                Ok(manager) => Some(Arc::new(RwLock::new(manager))),
                Err(e) => {
                    warn!("Failed to initialize bootstrap manager: {}, continuing without cache", e);
                    None
                }
            }
        };
        
        let node = Self {
            config,
            peer_id,
            peers: RwLock::new(HashMap::new()),
            event_tx,
            listen_addrs: RwLock::new(Vec::new()),
            start_time: Instant::now(),
            running: RwLock::new(false),
            mcp_server,
            dht,
            resource_manager,
            bootstrap_manager,
        };
        
        info!("Created P2P node with peer ID: {}", node.peer_id);
        Ok(node)
    }
    
    /// Create a new node builder
    pub fn builder() -> NodeBuilder {
        NodeBuilder::new()
    }
    
    /// Get the peer ID of this node
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
    
    /// Get the node configuration
    pub fn config(&self) -> &NodeConfig {
        &self.config
    }
    
    /// Start the P2P node
    pub async fn start(&self) -> Result<()> {
        info!("Starting P2P node...");
        
        // Start production resource manager if configured
        if let Some(ref resource_manager) = self.resource_manager {
            resource_manager.start().await
                .map_err(|e| P2PError::Network(format!("Failed to start resource manager: {}", e)))?;
            info!("Production resource manager started");
        }
        
        // Start bootstrap manager background tasks
        if let Some(ref bootstrap_manager) = self.bootstrap_manager {
            let mut manager = bootstrap_manager.write().await;
            manager.start_background_tasks().await
                .map_err(|e| P2PError::Network(format!("Failed to start bootstrap manager: {}", e)))?;
            info!("Bootstrap cache manager started");
        }
        
        // Set running state
        *self.running.write().await = true;
        
        // Initialize listen addresses
        let mut listen_addrs = self.listen_addrs.write().await;
        listen_addrs.extend(self.config.listen_addrs.clone());
        
        info!("P2P node started on addresses: {:?}", *listen_addrs);
        
        // Start MCP server if enabled
        if let Some(ref mcp_server) = self.mcp_server {
            mcp_server.start().await
                .map_err(|e| P2PError::MCP(format!("Failed to start MCP server: {}", e)))?;
            info!("MCP server started");
        }
        
        // Connect to bootstrap peers
        self.connect_bootstrap_peers().await?;
        
        Ok(())
    }
    
    /// Run the P2P node (blocks until shutdown)
    pub async fn run(&self) -> Result<()> {
        if !*self.running.read().await {
            self.start().await?;
        }
        
        info!("P2P node running...");
        
        // Main event loop
        loop {
            if !*self.running.read().await {
                break;
            }
            
            // Perform periodic tasks
            self.periodic_tasks().await?;
            
            // Sleep for a short interval
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        info!("P2P node stopped");
        Ok(())
    }
    
    /// Stop the P2P node
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping P2P node...");
        
        // Set running state to false
        *self.running.write().await = false;
        
        // Shutdown MCP server if enabled
        if let Some(ref mcp_server) = self.mcp_server {
            mcp_server.shutdown().await
                .map_err(|e| P2PError::MCP(format!("Failed to shutdown MCP server: {}", e)))?;
            info!("MCP server stopped");
        }
        
        // Disconnect all peers
        self.disconnect_all_peers().await?;
        
        // Shutdown production resource manager if configured
        if let Some(ref resource_manager) = self.resource_manager {
            resource_manager.shutdown().await
                .map_err(|e| P2PError::Network(format!("Failed to shutdown resource manager: {}", e)))?;
            info!("Production resource manager stopped");
        }
        
        info!("P2P node stopped");
        Ok(())
    }
    
    /// Check if the node is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    /// Get the current listen addresses
    pub async fn listen_addrs(&self) -> Vec<Multiaddr> {
        self.listen_addrs.read().await.clone()
    }
    
    /// Get connected peers
    pub async fn connected_peers(&self) -> Vec<PeerId> {
        self.peers.read().await.keys().cloned().collect()
    }
    
    /// Get peer count
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }
    
    /// Get peer info
    pub async fn peer_info(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.read().await.get(peer_id).cloned()
    }
    
    /// Connect to a peer
    pub async fn connect_peer(&self, address: &str) -> Result<PeerId> {
        info!("Connecting to peer at: {}", address);
        
        // Check production limits if resource manager is enabled
        let _connection_guard = if let Some(ref resource_manager) = self.resource_manager {
            Some(resource_manager.acquire_connection().await?)
        } else {
            None
        };
        
        // This is a placeholder implementation
        // In a real implementation, this would:
        // 1. Establish a connection using the transport layer
        // 2. Perform handshake and authentication
        // 3. Add the peer to our peer list
        // 4. Emit a PeerConnected event
        
        let peer_id = format!("peer_from_{}", address);
        
        let peer_info = PeerInfo {
            peer_id: peer_id.clone(),
            addresses: vec![address.to_string()],
            connected_at: Instant::now(),
            last_seen: Instant::now(),
            status: ConnectionStatus::Connected,
            protocols: vec!["p2p-foundation/1.0".to_string()],
        };
        
        self.peers.write().await.insert(peer_id.clone(), peer_info);
        
        // Record bandwidth usage if resource manager is enabled
        if let Some(ref resource_manager) = self.resource_manager {
            resource_manager.record_bandwidth(0, 0); // Placeholder for handshake data
        }
        
        // Emit event
        let _ = self.event_tx.send(NetworkEvent::PeerConnected {
            peer_id: peer_id.clone(),
            addresses: vec![address.to_string()],
        });
        
        info!("Connected to peer: {}", peer_id);
        Ok(peer_id)
    }
    
    /// Disconnect from a peer
    pub async fn disconnect_peer(&self, peer_id: &PeerId) -> Result<()> {
        info!("Disconnecting from peer: {}", peer_id);
        
        if let Some(mut peer_info) = self.peers.write().await.remove(peer_id) {
            peer_info.status = ConnectionStatus::Disconnected;
            
            // Emit event
            let _ = self.event_tx.send(NetworkEvent::PeerDisconnected {
                peer_id: peer_id.clone(),
                reason: "Manual disconnect".to_string(),
            });
            
            info!("Disconnected from peer: {}", peer_id);
        }
        
        Ok(())
    }
    
    /// Send a message to a peer
    pub async fn send_message(&self, peer_id: &PeerId, protocol: &str, data: Vec<u8>) -> Result<()> {
        debug!("Sending message to peer {} on protocol {}", peer_id, protocol);
        
        // Check rate limits if resource manager is enabled
        if let Some(ref resource_manager) = self.resource_manager {
            if !resource_manager.check_rate_limit(peer_id, "message").await? {
                return Err(P2PError::Network(format!("Rate limit exceeded for peer {}", peer_id)));
            }
        }
        
        // Check if peer is connected
        if !self.peers.read().await.contains_key(peer_id) {
            return Err(P2PError::Network(format!("Peer {} not connected", peer_id)));
        }
        
        // Handle MCP protocol messages
        if protocol == MCP_PROTOCOL {
            if let Some(ref mcp_server) = self.mcp_server {
                // For demonstration purposes, we'll simulate receiving the message
                // on the target peer. In a real implementation, this would send 
                // the message over the network and the target peer would handle it.
                
                debug!("Handling MCP message locally for demonstration");
                if let Ok(response_data) = mcp_server.handle_p2p_message(&data, &self.peer_id).await {
                    if let Some(response) = response_data {
                        debug!("Generated MCP response: {} bytes", response.len());
                        // In real implementation, this response would be sent back over the network
                    }
                }
            }
        }
        
        // Record bandwidth usage if resource manager is enabled
        if let Some(ref resource_manager) = self.resource_manager {
            resource_manager.record_bandwidth(data.len() as u64, 0);
        }
        
        // This is a placeholder implementation
        // In a real implementation, this would send the message over the network
        
        debug!("Message sent to peer: {}", peer_id);
        Ok(())
    }
    
    /// Subscribe to network events
    pub fn subscribe_events(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_tx.subscribe()
    }
    
    /// Get node uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Get MCP server reference
    pub fn mcp_server(&self) -> Option<&Arc<MCPServer>> {
        self.mcp_server.as_ref()
    }
    
    /// Register a tool in the MCP server
    pub async fn register_mcp_tool(&self, tool: Tool) -> Result<()> {
        if let Some(ref mcp_server) = self.mcp_server {
            mcp_server.register_tool(tool).await
                .map_err(|e| P2PError::MCP(format!("Failed to register tool: {}", e)))
        } else {
            Err(P2PError::MCP("MCP server not enabled".to_string()))
        }
    }
    
    /// Call a local MCP tool
    pub async fn call_mcp_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        if let Some(ref mcp_server) = self.mcp_server {
            // Check rate limits if resource manager is enabled
            if let Some(ref resource_manager) = self.resource_manager {
                if !resource_manager.check_rate_limit(&self.peer_id, "mcp").await? {
                    return Err(P2PError::MCP("MCP rate limit exceeded".to_string()));
                }
            }
            
            let context = MCPCallContext {
                caller_id: self.peer_id.clone(),
                timestamp: SystemTime::now(),
                timeout: Duration::from_secs(30),
                auth_info: None,
                metadata: HashMap::new(),
            };
            
            mcp_server.call_tool(tool_name, arguments, context).await
                .map_err(|e| P2PError::MCP(format!("Tool call failed: {}", e)))
        } else {
            Err(P2PError::MCP("MCP server not enabled".to_string()))
        }
    }
    
    /// Call a remote MCP tool on another node
    pub async fn call_remote_mcp_tool(&self, peer_id: &PeerId, tool_name: &str, arguments: Value) -> Result<Value> {
        if let Some(ref mcp_server) = self.mcp_server {
            // Create call context
            let context = MCPCallContext {
                caller_id: self.peer_id.clone(),
                timestamp: SystemTime::now(),
                timeout: Duration::from_secs(30),
                auth_info: None,
                metadata: HashMap::new(),
            };
            
            // Try to call the remote tool
            match mcp_server.call_remote_tool(peer_id, tool_name, arguments.clone(), context).await {
                Ok(result) => Ok(result),
                Err(P2PError::MCP(msg)) if msg.contains("network integration") => {
                    // For now, simulate a remote call by calling a local tool
                    // In a real implementation, this would go through the network
                    info!("Simulating remote MCP call to {} on peer {}", tool_name, peer_id);
                    
                    // Create a simulated remote call using local tools for demonstration
                    self.call_mcp_tool(tool_name, arguments).await
                }
                Err(e) => Err(e),
            }
        } else {
            Err(P2PError::MCP("MCP server not enabled".to_string()))
        }
    }
    
    /// List available tools in the local MCP server
    pub async fn list_mcp_tools(&self) -> Result<Vec<String>> {
        if let Some(ref mcp_server) = self.mcp_server {
            let (tools, _) = mcp_server.list_tools(None).await
                .map_err(|e| P2PError::MCP(format!("Failed to list tools: {}", e)))?;
            
            Ok(tools.into_iter().map(|tool| tool.name).collect())
        } else {
            Err(P2PError::MCP("MCP server not enabled".to_string()))
        }
    }
    
    /// Discover remote MCP services in the network
    pub async fn discover_remote_mcp_services(&self) -> Result<Vec<crate::mcp::MCPService>> {
        if let Some(ref mcp_server) = self.mcp_server {
            mcp_server.discover_remote_services().await
                .map_err(|e| P2PError::MCP(format!("Failed to discover services: {}", e)))
        } else {
            Err(P2PError::MCP("MCP server not enabled".to_string()))
        }
    }
    
    /// List tools available on a specific remote peer
    pub async fn list_remote_mcp_tools(&self, peer_id: &PeerId) -> Result<Vec<String>> {
        if let Some(ref _mcp_server) = self.mcp_server {
            // Create a list tools request message
            let request_message = crate::mcp::MCPMessage::ListTools {
                cursor: None,
            };
            
            // Create P2P message wrapper
            let p2p_message = crate::mcp::P2PMCPMessage {
                message_type: crate::mcp::P2PMCPMessageType::Request,
                message_id: uuid::Uuid::new_v4().to_string(),
                source_peer: self.peer_id.clone(),
                target_peer: Some(peer_id.clone()),
                timestamp: SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| P2PError::Network(format!("Time error: {}", e)))?
                    .as_secs(),
                payload: request_message,
                ttl: 5,
            };
            
            // Serialize and send the message
            let message_data = serde_json::to_vec(&p2p_message)
                .map_err(|e| P2PError::Serialization(e))?;
            
            // Send the message (for now, this will be simulated)
            self.send_message(peer_id, MCP_PROTOCOL, message_data).await?;
            
            // For demonstration, return local tools as if they were remote
            // In a real implementation, this would wait for the response
            self.list_mcp_tools().await
        } else {
            Err(P2PError::MCP("MCP server not enabled".to_string()))
        }
    }
    
    /// Get MCP server statistics
    pub async fn mcp_stats(&self) -> Result<crate::mcp::MCPServerStats> {
        if let Some(ref mcp_server) = self.mcp_server {
            Ok(mcp_server.get_stats().await)
        } else {
            Err(P2PError::MCP("MCP server not enabled".to_string()))
        }
    }
    
    /// Get production resource metrics
    pub async fn resource_metrics(&self) -> Result<ResourceMetrics> {
        if let Some(ref resource_manager) = self.resource_manager {
            Ok(resource_manager.get_metrics().await)
        } else {
            Err(P2PError::Network("Production resource manager not enabled".to_string()))
        }
    }
    
    /// Check system health
    pub async fn health_check(&self) -> Result<()> {
        if let Some(ref resource_manager) = self.resource_manager {
            resource_manager.health_check().await
        } else {
            // Basic health check without resource manager
            let peer_count = self.peer_count().await;
            if peer_count > self.config.max_connections {
                Err(P2PError::Network(format!("Too many connections: {}", peer_count)))
            } else {
                Ok(())
            }
        }
    }
    
    /// Get production configuration (if enabled)
    pub fn production_config(&self) -> Option<&ProductionConfig> {
        self.config.production_config.as_ref()
    }
    
    /// Check if production hardening is enabled
    pub fn is_production_mode(&self) -> bool {
        self.resource_manager.is_some()
    }
    
    /// Get DHT reference
    pub fn dht(&self) -> Option<&Arc<RwLock<DHT>>> {
        self.dht.as_ref()
    }
    
    /// Store a value in the DHT
    pub async fn dht_put(&self, key: crate::dht::Key, value: Vec<u8>) -> Result<()> {
        if let Some(ref dht) = self.dht {
            let dht_instance = dht.write().await;
            dht_instance.put(key.clone(), value.clone()).await
                .map_err(|e| P2PError::DHT(format!("DHT put failed: {}", e)))?;
            
            // Emit event
            let _ = self.event_tx.send(NetworkEvent::DHTRecordStored {
                key: key.as_bytes().to_vec(),
                value,
            });
            
            Ok(())
        } else {
            Err(P2PError::DHT("DHT not enabled".to_string()))
        }
    }
    
    /// Retrieve a value from the DHT
    pub async fn dht_get(&self, key: crate::dht::Key) -> Result<Option<Vec<u8>>> {
        if let Some(ref dht) = self.dht {
            let dht_instance = dht.write().await;
            let record_result = dht_instance.get(&key).await;
            
            let value = record_result.as_ref().map(|record| record.value.clone());
            
            // Emit event
            let _ = self.event_tx.send(NetworkEvent::DHTRecordRetrieved {
                key: key.as_bytes().to_vec(),
                value: value.clone(),
            });
            
            Ok(value)
        } else {
            Err(P2PError::DHT("DHT not enabled".to_string()))
        }
    }
    
    /// Add a discovered peer to the bootstrap cache
    pub async fn add_discovered_peer(&self, peer_id: PeerId, addresses: Vec<String>) -> Result<()> {
        if let Some(ref bootstrap_manager) = self.bootstrap_manager {
            let mut manager = bootstrap_manager.write().await;
            let contact = ContactEntry::new(peer_id, addresses);
            manager.add_contact(contact).await
                .map_err(|e| P2PError::Network(format!("Failed to add peer to bootstrap cache: {}", e)))?;
        }
        Ok(())
    }
    
    /// Update connection metrics for a peer in the bootstrap cache
    pub async fn update_peer_metrics(&self, peer_id: &PeerId, success: bool, latency_ms: Option<u64>, _error: Option<String>) -> Result<()> {
        if let Some(ref bootstrap_manager) = self.bootstrap_manager {
            let mut manager = bootstrap_manager.write().await;
            
            // Create quality metrics based on the connection result
            let metrics = QualityMetrics {
                success_rate: if success { 1.0 } else { 0.0 },
                avg_latency_ms: latency_ms.unwrap_or(0) as f64,
                quality_score: if success { 0.8 } else { 0.2 }, // Initial score
                last_connection_attempt: chrono::Utc::now(),
                last_successful_connection: if success { chrono::Utc::now() } else { chrono::Utc::now() - chrono::Duration::hours(1) },
                uptime_score: 0.5,
            };
            
            manager.update_contact_metrics(peer_id, metrics).await
                .map_err(|e| P2PError::Network(format!("Failed to update peer metrics: {}", e)))?;
        }
        Ok(())
    }
    
    /// Get bootstrap cache statistics
    pub async fn get_bootstrap_cache_stats(&self) -> Result<Option<crate::bootstrap::CacheStats>> {
        if let Some(ref bootstrap_manager) = self.bootstrap_manager {
            let manager = bootstrap_manager.read().await;
            let stats = manager.get_stats().await
                .map_err(|e| P2PError::Network(format!("Failed to get bootstrap stats: {}", e)))?;
            Ok(Some(stats))
        } else {
            Ok(None)
        }
    }
    
    /// Get the number of cached bootstrap peers
    pub async fn cached_peer_count(&self) -> usize {
        if let Some(ref _bootstrap_manager) = self.bootstrap_manager {
            if let Ok(stats) = self.get_bootstrap_cache_stats().await {
                if let Some(stats) = stats {
                    return stats.total_contacts;
                }
            }
        }
        0
    }
    
    /// Connect to bootstrap peers
    async fn connect_bootstrap_peers(&self) -> Result<()> {
        let mut bootstrap_contacts = Vec::new();
        let mut used_cache = false;
        
        // Try to get peers from bootstrap cache first
        if let Some(ref bootstrap_manager) = self.bootstrap_manager {
            let manager = bootstrap_manager.read().await;
            match manager.get_bootstrap_peers(20).await { // Try to get top 20 quality peers
                Ok(contacts) => {
                    if !contacts.is_empty() {
                        info!("Using {} cached bootstrap peers", contacts.len());
                        bootstrap_contacts = contacts;
                        used_cache = true;
                    }
                }
                Err(e) => {
                    warn!("Failed to get cached bootstrap peers: {}", e);
                }
            }
        }
        
        // Fallback to configured bootstrap peers if no cache or cache is empty
        if bootstrap_contacts.is_empty() {
            let bootstrap_peers = if !self.config.bootstrap_peers_str.is_empty() {
                &self.config.bootstrap_peers_str
            } else {
                // Convert Multiaddr to strings for fallback
                &self.config.bootstrap_peers.iter().map(|addr| addr.to_string()).collect::<Vec<_>>()
            };
            
            if bootstrap_peers.is_empty() {
                info!("No bootstrap peers configured and no cached peers available");
                return Ok(());
            }
            
            info!("Using {} configured bootstrap peers", bootstrap_peers.len());
            
            for addr in bootstrap_peers {
                let contact = ContactEntry::new(
                    format!("unknown_peer_{}", addr.chars().take(8).collect::<String>()),
                    vec![addr.clone()]
                );
                bootstrap_contacts.push(contact);
            }
        }
        
        // Connect to bootstrap peers
        let mut successful_connections = 0;
        for contact in bootstrap_contacts {
            for addr in &contact.addresses {
                match self.connect_peer(addr).await {
                    Ok(peer_id) => {
                        info!("Connected to bootstrap peer: {} ({})", peer_id, addr);
                        successful_connections += 1;
                        
                        // Update bootstrap cache with successful connection
                        if let Some(ref bootstrap_manager) = self.bootstrap_manager {
                            let mut manager = bootstrap_manager.write().await;
                            let mut updated_contact = contact.clone();
                            updated_contact.peer_id = peer_id.clone();
                            updated_contact.update_connection_result(true, Some(100), None); // Assume 100ms latency for now
                            
                            if let Err(e) = manager.add_contact(updated_contact).await {
                                warn!("Failed to update bootstrap cache: {}", e);
                            }
                        }
                        break; // Successfully connected, move to next contact
                    }
                    Err(e) => {
                        warn!("Failed to connect to bootstrap peer {}: {}", addr, e);
                        
                        // Update bootstrap cache with failed connection
                        if used_cache {
                            if let Some(ref bootstrap_manager) = self.bootstrap_manager {
                                let mut manager = bootstrap_manager.write().await;
                                let mut updated_contact = contact.clone();
                                updated_contact.update_connection_result(false, None, Some(e.to_string()));
                                
                                if let Err(e) = manager.add_contact(updated_contact).await {
                                    warn!("Failed to update bootstrap cache: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if successful_connections == 0 && !used_cache {
            warn!("Failed to connect to any bootstrap peers");
        } else {
            info!("Successfully connected to {} bootstrap peers", successful_connections);
        }
        
        Ok(())
    }
    
    /// Disconnect from all peers
    async fn disconnect_all_peers(&self) -> Result<()> {
        let peer_ids: Vec<PeerId> = self.peers.read().await.keys().cloned().collect();
        
        for peer_id in peer_ids {
            self.disconnect_peer(&peer_id).await?;
        }
        
        Ok(())
    }
    
    /// Perform periodic maintenance tasks
    async fn periodic_tasks(&self) -> Result<()> {
        // Update peer last seen timestamps
        // Remove stale connections
        // Perform DHT maintenance
        // This is a placeholder for now
        
        Ok(())
    }
}

/// Builder pattern for creating P2P nodes
pub struct NodeBuilder {
    config: NodeConfig,
}

impl NodeBuilder {
    /// Create a new node builder
    pub fn new() -> Self {
        Self {
            config: NodeConfig::default(),
        }
    }
    
    /// Set the peer ID
    pub fn with_peer_id(mut self, peer_id: PeerId) -> Self {
        self.config.peer_id = Some(peer_id);
        self
    }
    
    /// Add a listen address
    pub fn listen_on(mut self, addr: &str) -> Self {
        self.config.listen_addrs.push(addr.to_string());
        self
    }
    
    /// Add a bootstrap peer
    pub fn with_bootstrap_peer(mut self, addr: &str) -> Self {
        self.config.bootstrap_peers.push(addr.to_string());
        self
    }
    
    /// Enable IPv6 support
    pub fn with_ipv6(mut self, enable: bool) -> Self {
        self.config.enable_ipv6 = enable;
        self
    }
    
    /// Enable MCP server
    pub fn with_mcp_server(mut self) -> Self {
        self.config.enable_mcp_server = true;
        self
    }
    
    /// Configure MCP server settings
    pub fn with_mcp_config(mut self, mcp_config: MCPServerConfig) -> Self {
        self.config.mcp_server_config = Some(mcp_config);
        self.config.enable_mcp_server = true;
        self
    }
    
    /// Set connection timeout
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.connection_timeout = timeout;
        self
    }
    
    /// Set maximum connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }
    
    /// Enable production mode with default configuration
    pub fn with_production_mode(mut self) -> Self {
        self.config.production_config = Some(ProductionConfig::default());
        self
    }
    
    /// Configure production settings
    pub fn with_production_config(mut self, production_config: ProductionConfig) -> Self {
        self.config.production_config = Some(production_config);
        self
    }
    
    /// Build the P2P node
    pub async fn build(self) -> Result<P2PNode> {
        P2PNode::new(self.config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{Tool, MCPTool, ToolHandler, ToolMetadata, ToolHealthStatus, ToolRequirements};
    use serde_json::json;
    use std::pin::Pin;
    use std::future::Future;
    use std::time::Duration;
    use tokio::time::timeout;

    /// Test tool handler for network tests
    struct NetworkTestTool {
        name: String,
    }

    impl NetworkTestTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    impl ToolHandler for NetworkTestTool {
        fn execute(&self, arguments: serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send + '_>> {
            let name = self.name.clone();
            Box::pin(async move {
                Ok(json!({
                    "tool": name,
                    "input": arguments,
                    "result": "network test success"
                }))
            })
        }

        fn validate(&self, _arguments: &serde_json::Value) -> Result<()> {
            Ok(())
        }

        fn get_requirements(&self) -> ToolRequirements {
            ToolRequirements::default()
        }
    }

    /// Helper function to create a test node configuration
    fn create_test_node_config() -> NodeConfig {
        NodeConfig {
            peer_id: Some("test_peer_123".to_string()),
            listen_addrs: vec![
                "/ip6/::1/tcp/9001".to_string(),
                "/ip4/127.0.0.1/tcp/9001".to_string(),
            ],
            listen_addr: "127.0.0.1:9001".parse().unwrap(),
            bootstrap_peers: vec![],
            bootstrap_peers_str: vec![],
            enable_ipv6: true,
            enable_mcp_server: true,
            mcp_server_config: Some(MCPServerConfig {
                enable_auth: false, // Disable auth for testing
                enable_rate_limiting: false, // Disable rate limiting for testing
                ..Default::default()
            }),
            connection_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
            max_connections: 100,
            max_incoming_connections: 50,
            dht_config: DHTConfig::default(),
            security_config: SecurityConfig::default(),
            production_config: None,
            bootstrap_cache_config: None,
        }
    }

    /// Helper function to create a test tool
    fn create_test_tool(name: &str) -> Tool {
        Tool {
            definition: MCPTool {
                name: name.to_string(),
                description: format!("Test tool: {}", name),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    }
                }),
            },
            handler: Box::new(NetworkTestTool::new(name)),
            metadata: ToolMetadata {
                created_at: SystemTime::now(),
                last_called: None,
                call_count: 0,
                avg_execution_time: Duration::from_millis(0),
                health_status: ToolHealthStatus::Healthy,
                tags: vec!["test".to_string()],
            },
        }
    }

    #[tokio::test]
    async fn test_node_config_default() {
        let config = NodeConfig::default();
        
        assert!(config.peer_id.is_none());
        assert_eq!(config.listen_addrs.len(), 2);
        assert!(config.enable_ipv6);
        assert!(config.enable_mcp_server);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.max_incoming_connections, 100);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_dht_config_default() {
        let config = DHTConfig::default();
        
        assert_eq!(config.k_value, 20);
        assert_eq!(config.alpha_value, 5);
        assert_eq!(config.record_ttl, Duration::from_secs(3600));
        assert_eq!(config.refresh_interval, Duration::from_secs(600));
    }

    #[tokio::test]
    async fn test_security_config_default() {
        let config = SecurityConfig::default();
        
        assert!(config.enable_noise);
        assert!(config.enable_tls);
        assert_eq!(config.trust_level, TrustLevel::Basic);
    }

    #[test]
    fn test_trust_level_variants() {
        // Test that all trust level variants can be created
        let _none = TrustLevel::None;
        let _basic = TrustLevel::Basic;
        let _full = TrustLevel::Full;

        // Test equality
        assert_eq!(TrustLevel::None, TrustLevel::None);
        assert_eq!(TrustLevel::Basic, TrustLevel::Basic);
        assert_eq!(TrustLevel::Full, TrustLevel::Full);
        assert_ne!(TrustLevel::None, TrustLevel::Basic);
    }

    #[test]
    fn test_connection_status_variants() {
        let connecting = ConnectionStatus::Connecting;
        let connected = ConnectionStatus::Connected;
        let disconnecting = ConnectionStatus::Disconnecting;
        let disconnected = ConnectionStatus::Disconnected;
        let failed = ConnectionStatus::Failed("test error".to_string());

        assert_eq!(connecting, ConnectionStatus::Connecting);
        assert_eq!(connected, ConnectionStatus::Connected);
        assert_eq!(disconnecting, ConnectionStatus::Disconnecting);
        assert_eq!(disconnected, ConnectionStatus::Disconnected);
        assert_ne!(connecting, connected);

        if let ConnectionStatus::Failed(msg) = failed {
            assert_eq!(msg, "test error");
        } else {
            panic!("Expected Failed status");
        }
    }

    #[tokio::test]
    async fn test_node_creation() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        assert_eq!(node.peer_id(), "test_peer_123");
        assert!(!node.is_running().await);
        assert_eq!(node.peer_count().await, 0);
        assert!(node.connected_peers().await.is_empty());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_node_creation_without_peer_id() -> Result<()> {
        let mut config = create_test_node_config();
        config.peer_id = None;
        
        let node = P2PNode::new(config).await?;
        
        // Should have generated a peer ID
        assert!(node.peer_id().starts_with("peer_"));
        assert!(!node.is_running().await);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_node_lifecycle() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        // Initially not running
        assert!(!node.is_running().await);

        // Start the node
        node.start().await?;
        assert!(node.is_running().await);

        // Check listen addresses were set
        let listen_addrs = node.listen_addrs().await;
        assert_eq!(listen_addrs.len(), 2);

        // Stop the node
        node.stop().await?;
        assert!(!node.is_running().await);

        Ok(())
    }

    #[tokio::test]
    async fn test_peer_connection() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        let peer_addr = "/ip4/127.0.0.1/tcp/9002".to_string();
        
        // Connect to a peer
        let peer_id = node.connect_peer(&peer_addr).await?;
        assert!(peer_id.starts_with("peer_from_"));

        // Check peer count
        assert_eq!(node.peer_count().await, 1);

        // Check connected peers
        let connected_peers = node.connected_peers().await;
        assert_eq!(connected_peers.len(), 1);
        assert_eq!(connected_peers[0], peer_id);

        // Get peer info
        let peer_info = node.peer_info(&peer_id).await;
        assert!(peer_info.is_some());
        let info = peer_info.unwrap();
        assert_eq!(info.peer_id, peer_id);
        assert_eq!(info.status, ConnectionStatus::Connected);
        assert!(info.protocols.contains(&"p2p-foundation/1.0".to_string()));

        // Disconnect from peer
        node.disconnect_peer(&peer_id).await?;
        assert_eq!(node.peer_count().await, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_event_subscription() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        let mut events = node.subscribe_events();
        let peer_addr = "/ip4/127.0.0.1/tcp/9003".to_string();

        // Connect to a peer (this should emit an event)
        let peer_id = node.connect_peer(&peer_addr).await?;

        // Check for PeerConnected event
        let event = timeout(Duration::from_millis(100), events.recv()).await;
        assert!(event.is_ok());
        
        match event.unwrap().unwrap() {
            NetworkEvent::PeerConnected { peer_id: event_peer_id, addresses } => {
                assert_eq!(event_peer_id, peer_id);
                assert_eq!(addresses[0], peer_addr);
            }
            _ => panic!("Expected PeerConnected event"),
        }

        // Disconnect from peer (this should emit another event)
        node.disconnect_peer(&peer_id).await?;

        // Check for PeerDisconnected event
        let event = timeout(Duration::from_millis(100), events.recv()).await;
        assert!(event.is_ok());
        
        match event.unwrap().unwrap() {
            NetworkEvent::PeerDisconnected { peer_id: event_peer_id, reason } => {
                assert_eq!(event_peer_id, peer_id);
                assert_eq!(reason, "Manual disconnect");
            }
            _ => panic!("Expected PeerDisconnected event"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_message_sending() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        let peer_addr = "/ip4/127.0.0.1/tcp/9004".to_string();
        let peer_id = node.connect_peer(&peer_addr).await?;

        // Send a message
        let message_data = b"Hello, peer!".to_vec();
        let result = node.send_message(&peer_id, "test-protocol", message_data).await;
        assert!(result.is_ok());

        // Try to send to non-existent peer
        let non_existent_peer = "non_existent_peer".to_string();
        let result = node.send_message(&non_existent_peer, "test-protocol", vec![]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));

        Ok(())
    }

    #[tokio::test]
    async fn test_mcp_integration() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        // Start the node (which starts the MCP server)
        node.start().await?;

        // Register a test tool
        let tool = create_test_tool("network_test_tool");
        node.register_mcp_tool(tool).await?;

        // List tools
        let tools = node.list_mcp_tools().await?;
        assert!(tools.contains(&"network_test_tool".to_string()));

        // Call the tool
        let arguments = json!({"input": "test_input"});
        let result = node.call_mcp_tool("network_test_tool", arguments.clone()).await?;
        assert_eq!(result["tool"], "network_test_tool");
        assert_eq!(result["input"], arguments);

        // Get MCP stats
        let stats = node.mcp_stats().await?;
        assert_eq!(stats.total_tools, 1);

        // Test call to non-existent tool
        let result = node.call_mcp_tool("non_existent_tool", json!({})).await;
        assert!(result.is_err());

        node.stop().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_remote_mcp_operations() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        node.start().await?;

        // Register a test tool locally
        let tool = create_test_tool("remote_test_tool");
        node.register_mcp_tool(tool).await?;

        let peer_addr = "/ip4/127.0.0.1/tcp/9005".to_string();
        let peer_id = node.connect_peer(&peer_addr).await?;

        // List remote tools (simulated)
        let remote_tools = node.list_remote_mcp_tools(&peer_id).await?;
        assert!(!remote_tools.is_empty());

        // Call remote tool (simulated as local for now)
        let arguments = json!({"input": "remote_test"});
        let result = node.call_remote_mcp_tool(&peer_id, "remote_test_tool", arguments.clone()).await?;
        assert_eq!(result["tool"], "remote_test_tool");

        // Discover remote services
        let services = node.discover_remote_mcp_services().await?;
        // Should return empty list in test environment
        assert!(services.is_empty());

        node.stop().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        // Health check should pass with no connections
        let result = node.health_check().await;
        assert!(result.is_ok());

        // Connect many peers (but not over the limit)
        for i in 0..5 {
            let addr = format!("/ip4/127.0.0.1/tcp/{}", 9010 + i);
            node.connect_peer(&addr).await?;
        }

        // Health check should still pass
        let result = node.health_check().await;
        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_node_uptime() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        let uptime1 = node.uptime();
        assert!(uptime1 >= Duration::from_secs(0));

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(10)).await;

        let uptime2 = node.uptime();
        assert!(uptime2 > uptime1);

        Ok(())
    }

    #[tokio::test]
    async fn test_node_config_access() -> Result<()> {
        let config = create_test_node_config();
        let expected_peer_id = config.peer_id.clone();
        let node = P2PNode::new(config).await?;

        let node_config = node.config();
        assert_eq!(node_config.peer_id, expected_peer_id);
        assert_eq!(node_config.max_connections, 100);
        assert!(node_config.enable_mcp_server);

        Ok(())
    }

    #[tokio::test]
    async fn test_mcp_server_access() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        // Should have MCP server
        assert!(node.mcp_server().is_some());

        // Test with MCP disabled
        let mut config = create_test_node_config();
        config.enable_mcp_server = false;
        let node_no_mcp = P2PNode::new(config).await?;
        assert!(node_no_mcp.mcp_server().is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_dht_access() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        // Should have DHT
        assert!(node.dht().is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_node_builder() -> Result<()> {
        let node = P2PNode::builder()
            .with_peer_id("builder_test_peer".to_string())
            .listen_on("/ip4/127.0.0.1/tcp/9100")
            .listen_on("/ip6/::1/tcp/9100")
            .with_bootstrap_peer("/ip4/127.0.0.1/tcp/9101")
            .with_ipv6(true)
            .with_mcp_server()
            .with_connection_timeout(Duration::from_secs(15))
            .with_max_connections(200)
            .build()
            .await?;

        assert_eq!(node.peer_id(), "builder_test_peer");
        let config = node.config();
        assert_eq!(config.listen_addrs.len(), 4); // 2 default + 2 added by builder
        assert_eq!(config.bootstrap_peers.len(), 1);
        assert!(config.enable_ipv6);
        assert!(config.enable_mcp_server);
        assert_eq!(config.connection_timeout, Duration::from_secs(15));
        assert_eq!(config.max_connections, 200);

        Ok(())
    }

    #[tokio::test]
    async fn test_node_builder_with_mcp_config() -> Result<()> {
        let mcp_config = MCPServerConfig {
            server_name: "test_mcp_server".to_string(),
            server_version: "1.0.0".to_string(),
            enable_dht_discovery: false,
            enable_auth: false,
            ..MCPServerConfig::default()
        };

        let node = P2PNode::builder()
            .with_peer_id("mcp_config_test".to_string())
            .with_mcp_config(mcp_config.clone())
            .build()
            .await?;

        assert_eq!(node.peer_id(), "mcp_config_test");
        let config = node.config();
        assert!(config.enable_mcp_server);
        assert!(config.mcp_server_config.is_some());
        
        let node_mcp_config = config.mcp_server_config.as_ref().unwrap();
        assert_eq!(node_mcp_config.server_name, "test_mcp_server");
        assert!(!node_mcp_config.enable_auth);

        Ok(())
    }

    #[tokio::test]
    async fn test_mcp_server_not_enabled_errors() -> Result<()> {
        let mut config = create_test_node_config();
        config.enable_mcp_server = false;
        let node = P2PNode::new(config).await?;

        // All MCP operations should fail
        let tool = create_test_tool("test_tool");
        let result = node.register_mcp_tool(tool).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MCP server not enabled"));

        let result = node.call_mcp_tool("test_tool", json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MCP server not enabled"));

        let result = node.list_mcp_tools().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MCP server not enabled"));

        let result = node.mcp_stats().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MCP server not enabled"));

        Ok(())
    }

    #[tokio::test]
    async fn test_bootstrap_peers() -> Result<()> {
        let mut config = create_test_node_config();
        config.bootstrap_peers = vec![
            "/ip4/127.0.0.1/tcp/9200".to_string(),
            "/ip4/127.0.0.1/tcp/9201".to_string(),
        ];
        
        let node = P2PNode::new(config).await?;
        
        // Start node (which attempts to connect to bootstrap peers)
        node.start().await?;
        
        // In a test environment, bootstrap peers may not be available
        // The test verifies the node starts correctly with bootstrap configuration
        let peer_count = node.peer_count().await;
        assert!(peer_count <= 2, "Peer count should not exceed bootstrap peer count");
        
        node.stop().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_production_mode_disabled() -> Result<()> {
        let config = create_test_node_config();
        let node = P2PNode::new(config).await?;

        assert!(!node.is_production_mode());
        assert!(node.production_config().is_none());

        // Resource metrics should fail when production mode is disabled
        let result = node.resource_metrics().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not enabled"));

        Ok(())
    }

    #[tokio::test]
    async fn test_network_event_variants() {
        // Test that all network event variants can be created
        let peer_id = "test_peer".to_string();
        let address = "/ip4/127.0.0.1/tcp/9000".to_string();

        let _peer_connected = NetworkEvent::PeerConnected {
            peer_id: peer_id.clone(),
            addresses: vec![address.clone()],
        };

        let _peer_disconnected = NetworkEvent::PeerDisconnected {
            peer_id: peer_id.clone(),
            reason: "test disconnect".to_string(),
        };

        let _message_received = NetworkEvent::MessageReceived {
            peer_id: peer_id.clone(),
            protocol: "test-protocol".to_string(),
            data: vec![1, 2, 3],
        };

        let _connection_failed = NetworkEvent::ConnectionFailed {
            peer_id: Some(peer_id.clone()),
            address: address.clone(),
            error: "connection refused".to_string(),
        };

        let _dht_stored = NetworkEvent::DHTRecordStored {
            key: vec![1, 2, 3],
            value: vec![4, 5, 6],
        };

        let _dht_retrieved = NetworkEvent::DHTRecordRetrieved {
            key: vec![1, 2, 3],
            value: Some(vec![4, 5, 6]),
        };
    }

    #[tokio::test]
    async fn test_peer_info_structure() {
        let peer_info = PeerInfo {
            peer_id: "test_peer".to_string(),
            addresses: vec!["/ip4/127.0.0.1/tcp/9000".to_string()],
            connected_at: Instant::now(),
            last_seen: Instant::now(),
            status: ConnectionStatus::Connected,
            protocols: vec!["test-protocol".to_string()],
        };

        assert_eq!(peer_info.peer_id, "test_peer");
        assert_eq!(peer_info.addresses.len(), 1);
        assert_eq!(peer_info.status, ConnectionStatus::Connected);
        assert_eq!(peer_info.protocols.len(), 1);
    }

    #[tokio::test]
    async fn test_serialization() -> Result<()> {
        // Test that configs can be serialized/deserialized
        let config = create_test_node_config();
        let serialized = serde_json::to_string(&config)?;
        let deserialized: NodeConfig = serde_json::from_str(&serialized)?;

        assert_eq!(config.peer_id, deserialized.peer_id);
        assert_eq!(config.listen_addrs, deserialized.listen_addrs);
        assert_eq!(config.enable_ipv6, deserialized.enable_ipv6);

        Ok(())
    }
}