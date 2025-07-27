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

//! QUIC Transport Implementation with NAT Traversal
//!
//! This module provides QUIC-based transport using ant-quic with NAT traversal capabilities.
//! QUIC provides better performance, 0-RTT connections, built-in encryption, and robust NAT handling.

use super::{Transport, Connection, TransportType, TransportOptions, ConnectionInfo, ConnectionQuality};
use crate::{P2PError, Result, NetworkAddress};
use crate::identity::NodeIdentity;
use async_trait::async_trait;
use ant_quic::{
    quic_node::{QuicNodeConfig, QuicP2PNode},
    nat_traversal_api::{EndpointRole, PeerId},
    auth::AuthConfig,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, error};

/// QUIC transport implementation with NAT traversal
pub struct QuicTransport {
    /// Local identity for authentication
    identity: Option<Arc<NodeIdentity>>,
    /// QUIC P2P node with NAT traversal
    node: Arc<Mutex<Option<Arc<QuicP2PNode>>>>,
    /// Node configuration
    config: QuicNodeConfig,
    /// Bootstrap nodes for NAT traversal
    bootstrap_nodes: Vec<SocketAddr>,
    /// Whether 0-RTT is enabled
    enable_0rtt: bool,
}

/// QUIC connection implementation
pub struct QuicConnection {
    /// Remote peer ID
    peer_id: PeerId,
    /// QUIC P2P node reference
    node: Arc<QuicP2PNode>,
    /// Local address
    local_addr: NetworkAddress,
    /// Remote address
    remote_addr: NetworkAddress,
    /// Connection info
    info: ConnectionInfo,
    /// Active streams for multiplexing
    active_streams: Arc<Mutex<HashMap<u64, bool>>>,
    /// Stream counter for multiplexing
    stream_counter: Arc<Mutex<u64>>,
}

impl QuicTransport {
    /// Create a new QUIC transport with NAT traversal
    pub fn new(enable_0rtt: bool) -> Result<Self> {
        Self::new_with_identity(None, enable_0rtt)
    }
    
    /// Create a new QUIC transport with identity for raw key authentication
    pub fn new_with_identity(identity: Option<Arc<NodeIdentity>>, enable_0rtt: bool) -> Result<Self> {
        // TODO: Implement raw key authentication when ant-quic API supports it
        // For now, we store the identity and will use it for signing/verification
        // at the application layer
        let auth_config = AuthConfig::default();
        
        let config = QuicNodeConfig {
            role: EndpointRole::Client,
            bootstrap_nodes: vec![],
            enable_coordinator: false,
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),
            stats_interval: Duration::from_secs(60),
            auth_config,
            bind_addr: None, // Let the system choose
        };
        
        Ok(Self {
            identity,
            node: Arc::new(Mutex::new(None)),
            config,
            bootstrap_nodes: vec![],
            enable_0rtt,
        })
    }

    /// Create a new QUIC transport with bootstrap nodes
    pub fn new_with_bootstrap(bootstrap_nodes: Vec<SocketAddr>, enable_0rtt: bool) -> Result<Self> {
        Self::new_with_bootstrap_and_identity(None, bootstrap_nodes, enable_0rtt)
    }
    
    /// Create a new QUIC transport with bootstrap nodes and identity
    pub fn new_with_bootstrap_and_identity(
        identity: Option<Arc<NodeIdentity>>, 
        bootstrap_nodes: Vec<SocketAddr>, 
        enable_0rtt: bool
    ) -> Result<Self> {
        // TODO: Implement raw key authentication when ant-quic API supports it
        // For now, we store the identity and will use it for signing/verification
        // at the application layer
        let auth_config = AuthConfig::default();
        
        let config = QuicNodeConfig {
            role: EndpointRole::Client,
            bootstrap_nodes: bootstrap_nodes.clone(),
            enable_coordinator: false,
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),
            stats_interval: Duration::from_secs(60),
            auth_config,
            bind_addr: None,
        };
        
        Ok(Self {
            identity,
            node: Arc::new(Mutex::new(None)),
            config,
            bootstrap_nodes,
            enable_0rtt,
        })
    }
    
    /// Set whether to enable coordinator services (for public nodes)
    pub fn set_enable_coordinator(&mut self, enable: bool) {
        self.config.enable_coordinator = enable;
    }
    
    /// Initialize the QUIC P2P node with specific config
    async fn ensure_node_initialized_with_config(&self, config: QuicNodeConfig) -> Result<Arc<QuicP2PNode>> {
        let mut node_guard = self.node.lock().await;
        
        if let Some(node) = node_guard.as_ref() {
            Ok(Arc::clone(node))
        } else {
            // Create new node
            let node = Arc::new(QuicP2PNode::new(config).await
                .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to create QUIC node: {e}"))))?);
            
            // Connect to bootstrap nodes if configured
            for bootstrap_addr in &self.bootstrap_nodes {
                match node.connect_to_bootstrap(*bootstrap_addr).await {
                    Ok(peer_id) => {
                        info!("Connected to bootstrap node {} with peer ID {:?}", bootstrap_addr, peer_id);
                    }
                    Err(e) => {
                        error!("Failed to connect to bootstrap node {}: {}", bootstrap_addr, e);
                    }
                }
            }
            
            *node_guard = Some(Arc::clone(&node));
            Ok(node)
        }
    }
    
    /// Initialize the QUIC P2P node
    async fn ensure_node_initialized(&self) -> Result<Arc<QuicP2PNode>> {
        let mut node_guard = self.node.lock().await;
        
        if let Some(node) = node_guard.as_ref() {
            Ok(Arc::clone(node))
        } else {
            // Create new node
            let node = Arc::new(QuicP2PNode::new(self.config.clone()).await
                .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to create QUIC node: {e}"))))?);
            
            // Connect to bootstrap nodes if configured
            for bootstrap_addr in &self.bootstrap_nodes {
                match node.connect_to_bootstrap(*bootstrap_addr).await {
                    Ok(peer_id) => {
                        info!("Connected to bootstrap node {} with peer ID {:?}", bootstrap_addr, peer_id);
                    }
                    Err(e) => {
                        error!("Failed to connect to bootstrap node {}: {}", bootstrap_addr, e);
                    }
                }
            }
            
            *node_guard = Some(Arc::clone(&node));
            Ok(node)
        }
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn listen(&self, addr: NetworkAddress) -> Result<NetworkAddress> {
        debug!("QUIC listening on {}", addr);
        
        // Update config with bind address before initialization
        let mut config = self.config.clone();
        config.bind_addr = Some(addr.socket_addr());
        
        // Initialize the node with updated config
        let node = self.ensure_node_initialized_with_config(config).await?;
        
        // Get actual listen address from the quinn endpoint
        let quinn_endpoint = node.get_nat_endpoint()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get NAT endpoint: {e}"))))?
            .get_quinn_endpoint()
            .ok_or_else(|| P2PError::Transport(crate::error::TransportError::SetupFailed("Quinn endpoint not available".to_string())))?;
        
        let local_addr = quinn_endpoint.local_addr()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get local address: {e}"))))?;
        
        info!("QUIC transport listening on {} with peer ID {:?}", local_addr, node.peer_id());
        Ok(NetworkAddress::new(local_addr))
    }
    
    async fn accept(&self) -> Result<Box<dyn Connection>> {
        debug!("QUIC waiting to accept incoming connection");
        
        // Get the node
        let node = self.ensure_node_initialized().await?;
        
        // For now, we'll use the receive method to detect new connections
        // ant-quic doesn't have explicit accept, connections are established via NAT traversal
        // We'll need to handle this differently - perhaps by tracking new peer connections
        
        // Accept a connection from ant-quic
        let (remote_addr, peer_id) = node.accept().await
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to accept connection: {e}"))))?;
        
        // Get local address from the quinn endpoint
        let quinn_endpoint = node.get_nat_endpoint()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get NAT endpoint: {e}"))))?
            .get_quinn_endpoint()
            .ok_or_else(|| P2PError::Transport(crate::error::TransportError::SetupFailed("Quinn endpoint not available".to_string())))?;
        
        let local_addr = quinn_endpoint.local_addr()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get local address: {e}"))))?;
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: NetworkAddress::new(local_addr),
            remote_addr: NetworkAddress::new(remote_addr),
            is_encrypted: true,
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(),
            used_0rtt: false,
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let quic_connection = QuicConnection {
            peer_id,
            node: Arc::clone(&node),
            local_addr: NetworkAddress::new(local_addr),
            remote_addr: NetworkAddress::new(remote_addr),
            info: connection_info,
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            stream_counter: Arc::new(Mutex::new(0)),
        };
        
        info!("QUIC accepted incoming connection from {:?}", peer_id);
        Ok(Box::new(quic_connection))
    }
    
    async fn connect(&self, addr: NetworkAddress) -> Result<Box<dyn Connection>> {
        self.connect_with_options(addr, TransportOptions::default()).await
    }
    
    async fn connect_with_options(&self, addr: NetworkAddress, _options: TransportOptions) -> Result<Box<dyn Connection>> {
        debug!("QUIC connecting to {} with NAT traversal", addr);
        
        // Initialize node
        let node = self.ensure_node_initialized().await?;
        
        // For direct connections, we first try to connect as a bootstrap
        // If that fails, we might need coordinator assistance
        let peer_id = match node.connect_to_bootstrap(addr.socket_addr()).await {
            Ok(peer_id) => {
                info!("Direct connection established to {}", addr);
                peer_id
            }
            Err(e) => {
                // If direct connection fails and we have bootstrap nodes,
                // we could try NAT traversal through them
                return Err(P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to connect to {addr}: {e}"))));
            }
        };
        
        // Get local address from the quinn endpoint
        let quinn_endpoint = node.get_nat_endpoint()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get NAT endpoint: {e}"))))?
            .get_quinn_endpoint()
            .ok_or_else(|| P2PError::Transport(crate::error::TransportError::SetupFailed("Quinn endpoint not available".to_string())))?;
        
        let local_addr = quinn_endpoint.local_addr()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get local address: {e}"))))?;
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: NetworkAddress::new(local_addr),
            remote_addr: addr.clone(),
            is_encrypted: true,
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(),
            used_0rtt: self.enable_0rtt,
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let quic_connection = QuicConnection {
            peer_id,
            node: Arc::clone(&node),
            local_addr: NetworkAddress::new(local_addr),
            remote_addr: addr.clone(),
            info: connection_info,
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            stream_counter: Arc::new(Mutex::new(0)),
        };
        
        info!("QUIC connection established to {} with peer ID {:?}", addr.clone(), peer_id);
        Ok(Box::new(quic_connection))
    }
    
    fn supports_ipv6(&self) -> bool {
        false // Default to IPv4 as requested, IPv6 can be enabled later
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::QUIC
    }
    
    fn supports_address(&self, addr: &NetworkAddress) -> bool {
        // Check if address is IPv4 (default) or IPv6 if enabled
        match addr.socket_addr() {
            std::net::SocketAddr::V4(_) => true,
            std::net::SocketAddr::V6(_) => self.supports_ipv6(),
        }
    }
}

#[async_trait]
impl Connection for QuicConnection {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.node.send_to_peer(&self.peer_id, data).await
            .map_err(|e| P2PError::Transport(crate::error::TransportError::StreamError(format!("Failed to send data: {e}"))))?;
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<Vec<u8>> {
        // Receive from any peer, but filter for our peer
        loop {
            let (recv_peer_id, data) = self.node.receive().await
                .map_err(|e| P2PError::Transport(crate::error::TransportError::StreamError(format!("Failed to receive data: {e}"))))?;
            
            if recv_peer_id == self.peer_id {
                return Ok(data);
            }
            // Otherwise, continue waiting for data from our peer
        }
    }
    
    async fn close(&mut self) -> Result<()> {
        // ant-quic doesn't have explicit close for individual connections
        // Connections are managed at the node level
        info!("Closing QUIC connection to {:?}", self.peer_id);
        Ok(())
    }
    
    async fn info(&self) -> ConnectionInfo {
        self.info.clone()
    }
    
    async fn is_alive(&self) -> bool {
        // Check if peer is still in connected list
        // This is async in ant-quic, so we'll assume connected for now
        // In practice, you might want to periodically check with the node
        true
    }
    
    async fn measure_quality(&self) -> Result<ConnectionQuality> {
        // Try to get metrics from ant-quic
        match self.node.get_connection_metrics(&self.peer_id).await {
            Ok(metrics) => Ok(ConnectionQuality {
                latency: metrics.rtt.unwrap_or(Duration::from_millis(0)),
                throughput_mbps: 0.0, // ant-quic doesn't provide bandwidth estimate
                packet_loss: metrics.packet_loss * 100.0, // Convert to percentage
                jitter: Duration::from_millis(0), // ant-quic doesn't provide jitter
                connect_time: Duration::from_millis(0), // Not tracked
            }),
            Err(e) => Err(P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get metrics: {e}"))))
        }
    }
    
    fn local_addr(&self) -> NetworkAddress {
        self.local_addr.clone()
    }
    
    fn remote_addr(&self) -> NetworkAddress {
        self.remote_addr.clone()
    }
}

/// Connect to a peer using NAT traversal through a coordinator
impl QuicTransport {
    /// Connect to a peer via NAT traversal using a coordinator
    pub async fn connect_to_peer_via_coordinator(
        &self,
        peer_id: PeerId,
        coordinator_addr: SocketAddr,
    ) -> Result<Box<dyn Connection>> {
        debug!("Connecting to peer {:?} via coordinator {}", peer_id, coordinator_addr);
        
        // Initialize node
        let node = self.ensure_node_initialized().await?;
        
        // Connect via coordinator
        let remote_addr = node.connect_to_peer(peer_id, coordinator_addr).await
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to connect via coordinator: {e}"))))?;
        
        // Get local address from the quinn endpoint
        let quinn_endpoint = node.get_nat_endpoint()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get NAT endpoint: {e}"))))?
            .get_quinn_endpoint()
            .ok_or_else(|| P2PError::Transport(crate::error::TransportError::SetupFailed("Quinn endpoint not available".to_string())))?;
        
        let local_addr = quinn_endpoint.local_addr()
            .map_err(|e| P2PError::Transport(crate::error::TransportError::SetupFailed(format!("Failed to get local address: {e}"))))?;
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: NetworkAddress::new(local_addr),
            remote_addr: NetworkAddress::new(remote_addr),
            is_encrypted: true,
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(),
            used_0rtt: self.enable_0rtt,
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let quic_connection = QuicConnection {
            peer_id,
            node: Arc::clone(&node),
            local_addr: NetworkAddress::new(local_addr),
            remote_addr: NetworkAddress::new(remote_addr),
            info: connection_info,
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            stream_counter: Arc::new(Mutex::new(0)),
        };
        
        info!("QUIC connection established to peer {:?} at {} via coordinator", peer_id, remote_addr);
        Ok(Box::new(quic_connection))
    }
    
    /// Get the local peer ID
    pub async fn peer_id(&self) -> Result<PeerId> {
        let node = self.ensure_node_initialized().await?;
        Ok(node.peer_id())
    }
}

#[cfg(test)]
#[path = "quic_tests.rs"]
mod quic_tests;