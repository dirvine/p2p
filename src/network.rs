//! Network module
//!
//! This module provides core networking functionality for the P2P Foundation.
//! It handles peer connections, network events, and node lifecycle management.

use crate::{PeerId, Multiaddr, P2PError, Result};
use crate::mcp::{MCPServer, MCPServerConfig, Tool, MCPCallContext, MCP_PROTOCOL};
use crate::dht::{DHT, DHTConfig as DHTConfigInner};
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
    
    /// Bootstrap peers to connect to on startup
    pub bootstrap_peers: Vec<Multiaddr>,
    
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            bootstrap_peers: Vec::new(),
            enable_ipv6: true,
            enable_mcp_server: true,
            mcp_server_config: None, // Use default config if None
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(60),
            max_connections: 1000,
            max_incoming_connections: 100,
            dht_config: DHTConfig::default(),
            security_config: SecurityConfig::default(),
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
    pub addresses: Vec<Multiaddr>,
    
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
        peer_id: PeerId,
        addresses: Vec<Multiaddr>,
    },
    
    /// A peer has disconnected
    PeerDisconnected {
        peer_id: PeerId,
        reason: String,
    },
    
    /// A message was received from a peer
    MessageReceived {
        peer_id: PeerId,
        protocol: String,
        data: Vec<u8>,
    },
    
    /// A connection attempt failed
    ConnectionFailed {
        peer_id: Option<PeerId>,
        address: Multiaddr,
        error: String,
    },
    
    /// DHT record was stored
    DHTRecordStored {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    
    /// DHT record was retrieved
    DHTRecordRetrieved {
        key: Vec<u8>,
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
    pub async fn connect_peer(&self, address: &Multiaddr) -> Result<PeerId> {
        info!("Connecting to peer at: {}", address);
        
        // This is a placeholder implementation
        // In a real implementation, this would:
        // 1. Establish a connection using the transport layer
        // 2. Perform handshake and authentication
        // 3. Add the peer to our peer list
        // 4. Emit a PeerConnected event
        
        let peer_id = format!("peer_from_{}", address);
        
        let peer_info = PeerInfo {
            peer_id: peer_id.clone(),
            addresses: vec![address.clone()],
            connected_at: Instant::now(),
            last_seen: Instant::now(),
            status: ConnectionStatus::Connected,
            protocols: vec!["p2p-foundation/1.0".to_string()],
        };
        
        self.peers.write().await.insert(peer_id.clone(), peer_info);
        
        // Emit event
        let _ = self.event_tx.send(NetworkEvent::PeerConnected {
            peer_id: peer_id.clone(),
            addresses: vec![address.clone()],
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
    
    /// Get DHT reference
    pub fn dht(&self) -> Option<&Arc<RwLock<DHT>>> {
        self.dht.as_ref()
    }
    
    /// Connect to bootstrap peers
    async fn connect_bootstrap_peers(&self) -> Result<()> {
        if self.config.bootstrap_peers.is_empty() {
            info!("No bootstrap peers configured");
            return Ok(());
        }
        
        info!("Connecting to {} bootstrap peers", self.config.bootstrap_peers.len());
        
        for addr in &self.config.bootstrap_peers {
            match self.connect_peer(addr).await {
                Ok(peer_id) => {
                    info!("Connected to bootstrap peer: {}", peer_id);
                }
                Err(e) => {
                    warn!("Failed to connect to bootstrap peer {}: {}", addr, e);
                }
            }
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
    
    /// Build the P2P node
    pub async fn build(self) -> Result<P2PNode> {
        P2PNode::new(self.config).await
    }
}