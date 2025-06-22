//! Network module
//!
//! This module provides core networking functionality for the P2P Foundation.
//! It handles peer connections, network events, and node lifecycle management.

use crate::{PeerId, Multiaddr, P2PError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
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
}

impl P2PNode {
    /// Create a new P2P node with the given configuration
    pub async fn new(config: NodeConfig) -> Result<Self> {
        let peer_id = config.peer_id.clone().unwrap_or_else(|| {
            // Generate a random peer ID for now
            format!("peer_{}", uuid::Uuid::new_v4().to_string()[..8].to_string())
        });
        
        let (event_tx, _) = broadcast::channel(1000);
        
        let node = Self {
            config,
            peer_id,
            peers: RwLock::new(HashMap::new()),
            event_tx,
            listen_addrs: RwLock::new(Vec::new()),
            start_time: Instant::now(),
            running: RwLock::new(false),
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
    pub async fn send_message(&self, peer_id: &PeerId, protocol: &str, _data: Vec<u8>) -> Result<()> {
        debug!("Sending message to peer {} on protocol {}", peer_id, protocol);
        
        // Check if peer is connected
        if !self.peers.read().await.contains_key(peer_id) {
            return Err(P2PError::Network(format!("Peer {} not connected", peer_id)));
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