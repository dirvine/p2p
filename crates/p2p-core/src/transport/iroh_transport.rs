
//! Iroh Transport Implementation
//!
//! This module provides an Iroh-based transport implementation that handles
//! automatic NAT traversal, dual-stack networking (IPv4/IPv6), and relay
//! fallback for P2P connections.

use crate::{PeerId, Multiaddr, P2PError, Result};
use crate::transport::{Transport, Connection, TransportType, TransportOptions, ConnectionInfo, ConnectionQuality};
use async_trait::async_trait;
use iroh_net::{Endpoint, NodeAddr, NodeId, RelayMode, RelayUrl};
use iroh_net::key::SecretKey;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, warn, error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Iroh transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConfig {
    /// Relay mode configuration
    pub relay_mode: RelayMode,
    
    /// Use STUN only (no relay) for testing
    pub stun_only: bool,
    
    /// Prefer IPv6 when available
    pub prefer_ipv6: bool,
    
    /// Custom relay URL (optional)
    pub custom_relay_url: Option<String>,
    
    /// Connection timeout for NAT traversal
    pub nat_traversal_timeout: Duration,
    
    /// Maximum concurrent connections
    pub max_connections: usize,
    
    /// Enable hole punching
    pub enable_hole_punching: bool,
}

/// Connection type used for the connection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionType {
    /// Direct IPv4 connection
    DirectIPv4,
    /// Direct IPv6 connection
    DirectIPv6,
    /// Connection via relay
    Relay,
    /// Connection using hole punching
    HolePunching,
}

/// Iroh transport implementation
pub struct IrohTransport {
    /// Iroh endpoint for connections
    endpoint: Endpoint,
    
    /// Our Iroh NodeId (derived from our P2P PeerId)
    node_id: NodeId,
    
    /// Secret key for Iroh
    secret_key: SecretKey,
    
    /// Active connections
    connections: Arc<RwLock<HashMap<PeerId, Arc<Mutex<IrohConnection>>>>>,
    
    /// Configuration
    config: IrohConfig,
    
    /// Local addresses cache
    local_addresses: Arc<RwLock<Vec<Multiaddr>>>,
}

/// Iroh connection wrapper
pub struct IrohConnection {
    /// Underlying Quinn connection
    connection: quinn::Connection,
    
    /// Remote peer ID
    remote_peer_id: PeerId,
    
    /// Connection type used
    connection_type: ConnectionType,
    
    /// Connection establishment time
    established_at: Instant,
    
    /// Last activity timestamp
    last_activity: Arc<Mutex<Instant>>,
    
    /// Whether hole punching was used
    used_hole_punching: bool,
    
    /// Connection quality metrics
    quality_cache: Arc<Mutex<Option<ConnectionQuality>>>,
    
    /// Bytes sent/received counters
    bytes_sent: Arc<Mutex<u64>>,
    bytes_received: Arc<Mutex<u64>>,
}

impl IrohTransport {
    /// Create a new Iroh transport
    pub async fn new(peer_id: &PeerId, config: IrohConfig) -> Result<Self> {
        // Derive Iroh key from our peer ID for consistency
        let secret_key = Self::derive_iroh_key(peer_id)?;
        let node_id = NodeId::from(secret_key.public());
        
        info!("Creating Iroh transport for peer {} with NodeId {}", peer_id, node_id);
        
        // Configure Iroh endpoint builder
        let mut endpoint_builder = Endpoint::builder()
            .secret_key(secret_key.clone())
            .alpns(vec![b"p2p-foundation/1".to_vec()]);
        
        // Configure relay mode
        endpoint_builder = match &config.relay_mode {
            RelayMode::Default => {
                debug!("Using default relay configuration");
                endpoint_builder.relay_mode(RelayMode::Default)
            }
            RelayMode::Disabled => {
                debug!("Relay disabled - direct connections only");
                endpoint_builder.relay_mode(RelayMode::Disabled)
            }
            RelayMode::Custom(url) => {
                debug!("Using custom relay: {}", url);
                endpoint_builder.relay_mode(RelayMode::Custom(url.clone()))
            }
        };
        
        // Bind to any available port
        let endpoint = endpoint_builder
            .bind(0)
            .await
            .map_err(|e| P2PError::Transport(format!("Failed to bind Iroh endpoint: {}", e)))?;
        
        info!("Iroh endpoint bound successfully");
        
        // Wait for endpoint to be ready and get initial addresses
        let local_addrs = Self::get_endpoint_addresses(&endpoint).await?;
        
        Ok(Self {
            endpoint,
            node_id,
            secret_key,
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
            local_addresses: Arc::new(RwLock::new(local_addrs)),
        })
    }
    
    /// Derive Iroh secret key from P2P peer ID
    fn derive_iroh_key(peer_id: &PeerId) -> Result<SecretKey> {
        // Use SHA-256 to derive a consistent key from peer ID
        let mut hasher = Sha256::new();
        hasher.update(b"iroh-key-derivation-v1:");
        hasher.update(peer_id.as_bytes());
        let hash = hasher.finalize();
        
        // Create secret key from hash
        let key_bytes: [u8; 32] = hash.into();
        SecretKey::from(key_bytes)
    }
    
    /// Get addresses from Iroh endpoint
    async fn get_endpoint_addresses(endpoint: &Endpoint) -> Result<Vec<Multiaddr>> {
        let mut addresses = Vec::new();
        
        // Get direct addresses
        if let Some(endpoints) = endpoint.local_endpoints().next().await {
            for endpoint_addr in endpoints.iter() {
                let multiaddr = Self::socket_addr_to_multiaddr(endpoint_addr)?;
                addresses.push(multiaddr);
            }
        }
        
        // Add relay address if available
        if let Some(relay_url) = endpoint.my_relay() {
            let relay_addr = format!("/p2p-circuit/relay/{}/p2p/{}", 
                                   relay_url, endpoint.node_id());
            addresses.push(relay_addr);
        }
        
        debug!("Iroh endpoint addresses: {:?}", addresses);
        Ok(addresses)
    }
    
    /// Convert SocketAddr to Multiaddr format
    fn socket_addr_to_multiaddr(addr: &SocketAddr) -> Result<Multiaddr> {
        match addr {
            SocketAddr::V4(v4) => Ok(format!("/ip4/{}/udp/{}/quic", v4.ip(), v4.port())),
            SocketAddr::V6(v6) => Ok(format!("/ip6/{}/udp/{}/quic", v6.ip(), v6.port())),
        }
    }
    
    /// Convert Multiaddr to SocketAddr
    fn multiaddr_to_socket_addr(addr: &Multiaddr) -> Result<SocketAddr> {
        // Parse multiaddr string like "/ip4/127.0.0.1/udp/9000/quic"
        let parts: Vec<&str> = addr.split('/').collect();
        
        if parts.len() < 5 {
            return Err(P2PError::Transport(format!("Invalid multiaddr format: {}", addr)));
        }
        
        let ip = parts[2];
        let port: u16 = parts[4].parse()
            .map_err(|e| P2PError::Transport(format!("Invalid port in multiaddr {}: {}", addr, e)))?;
        
        let socket_addr = format!("{}:{}", ip, port).parse()
            .map_err(|e| P2PError::Transport(format!("Failed to parse socket address: {}", e)))?;
        
        Ok(socket_addr)
    }
    
    /// Connect to a peer using Iroh's automatic NAT traversal
    pub async fn connect_to_peer(&self, peer_id: &PeerId, addrs: Vec<SocketAddr>) -> Result<Arc<Mutex<IrohConnection>>> {
        // Convert our PeerId to Iroh NodeId (for now, use a simple mapping)
        let target_node_id = Self::peer_id_to_node_id(peer_id)?;
        
        // Create NodeAddr with all known addresses
        let mut node_addr = NodeAddr::new(target_node_id);
        
        // Add direct addresses
        for addr in &addrs {
            node_addr = node_addr.with_direct_addresses(vec![*addr]);
        }
        
        // Add relay if configured and available
        if let Some(relay_url) = self.endpoint.my_relay() {
            node_addr = node_addr.with_relay_url(relay_url);
        }
        
        info!("Connecting to peer {} via Iroh with {} addresses", peer_id, addrs.len());
        debug!("Target addresses: {:?}", addrs);
        
        // Iroh handles NAT traversal automatically!
        let connection = tokio::time::timeout(
            self.config.nat_traversal_timeout,
            self.endpoint.connect(node_addr, b"p2p-foundation/1")
        )
        .await
        .map_err(|_| P2PError::Transport("Connection timeout during NAT traversal".to_string()))?
        .map_err(|e| P2PError::Transport(format!("Iroh connection failed: {}", e)))?;
        
        // Determine connection type
        let connection_type = self.determine_connection_type(&connection, &addrs).await;
        let used_hole_punching = matches!(connection_type, ConnectionType::HolePunching);
        
        info!("Successfully connected to peer {} using {:?}", peer_id, connection_type);
        
        // Create connection wrapper
        let iroh_conn = IrohConnection {
            connection,
            remote_peer_id: peer_id.clone(),
            connection_type,
            established_at: Instant::now(),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            used_hole_punching,
            quality_cache: Arc::new(Mutex::new(None)),
            bytes_sent: Arc::new(Mutex::new(0)),
            bytes_received: Arc::new(Mutex::new(0)),
        };
        
        let conn_arc = Arc::new(Mutex::new(iroh_conn));
        
        // Store connection
        self.connections.write().await.insert(peer_id.clone(), conn_arc.clone());
        
        Ok(conn_arc)
    }
    
    /// Determine what type of connection was established
    async fn determine_connection_type(&self, connection: &quinn::Connection, addrs: &[SocketAddr]) -> ConnectionType {
        let remote_addr = connection.remote_address();
        
        // Check if remote address matches any of the direct addresses provided
        for addr in addrs {
            if addr.ip() == remote_addr.ip() {
                return if addr.is_ipv4() {
                    ConnectionType::DirectIPv4
                } else {
                    ConnectionType::DirectIPv6
                };
            }
        }
        
        // If we're using a relay, the remote address won't match direct addresses
        if self.endpoint.my_relay().is_some() {
            // This is a simplified check - in reality, we'd need more sophisticated
            // detection to distinguish between relay and hole punching
            return ConnectionType::Relay;
        }
        
        // Default to hole punching if we can't determine the exact type
        ConnectionType::HolePunching
    }
    
    /// Convert P2P PeerId to Iroh NodeId (simplified mapping for now)
    fn peer_id_to_node_id(peer_id: &PeerId) -> Result<NodeId> {
        // For now, use a hash-based mapping
        // In a real implementation, this would be part of the identity system
        let mut hasher = Sha256::new();
        hasher.update(b"iroh-node-id-v1:");
        hasher.update(peer_id.as_bytes());
        let hash = hasher.finalize();
        
        let key_bytes: [u8; 32] = hash.into();
        let secret_key = SecretKey::from(key_bytes);
        Ok(NodeId::from(secret_key.public()))
    }
    
    /// Get our local addresses (including relay)
    pub async fn local_addresses(&self) -> Vec<Multiaddr> {
        // Update cached addresses
        if let Ok(addrs) = Self::get_endpoint_addresses(&self.endpoint).await {
            *self.local_addresses.write().await = addrs.clone();
            addrs
        } else {
            self.local_addresses.read().await.clone()
        }
    }
    
    /// Get connection to a peer
    pub async fn get_connection(&self, peer_id: &PeerId) -> Option<Arc<Mutex<IrohConnection>>> {
        self.connections.read().await.get(peer_id).cloned()
    }
    
    /// Remove connection for a peer
    pub async fn remove_connection(&self, peer_id: &PeerId) {
        self.connections.write().await.remove(peer_id);
    }
    
    /// Get statistics about all connections
    pub async fn get_connection_stats(&self) -> HashMap<PeerId, ConnectionType> {
        let mut stats = HashMap::new();
        let connections = self.connections.read().await;
        
        for (peer_id, conn) in connections.iter() {
            if let Ok(conn_guard) = conn.try_lock() {
                stats.insert(peer_id.clone(), conn_guard.connection_type.clone());
            }
        }
        
        stats
    }
}

#[async_trait]
impl Transport for IrohTransport {
    async fn listen(&self, _addr: SocketAddr) -> Result<Vec<Multiaddr>> {
        // Iroh handles listening automatically when the endpoint is created
        // Return our current local addresses
        Ok(self.local_addresses().await)
    }
    
    async fn accept(&self) -> Result<Box<dyn Connection>> {
        // For Iroh, incoming connections are handled through the endpoint
        // This is a placeholder - in practice, we'd need to set up an incoming connection handler
        Err(P2PError::Transport("Direct accept not implemented for Iroh transport - use connection events".to_string()))
    }
    
    async fn connect(&self, addr: &Multiaddr) -> Result<Box<dyn Connection>> {
        // Convert multiaddr to socket address
        let socket_addr = Self::multiaddr_to_socket_addr(addr)?;
        
        // Extract peer ID from address (this is a simplified approach)
        // In practice, peer ID would be provided separately or encoded in the address
        let peer_id = format!("peer_from_{}", addr.replace("/", "_").replace(":", "_"));
        
        // Connect using Iroh
        let connection = self.connect_to_peer(&peer_id, vec![socket_addr]).await?;
        
        Ok(Box::new(IrohConnectionAdapter { 
            connection,
            local_transport: Arc::new(self.clone()),
        }))
    }
    
    async fn connect_with_options(&self, addr: &Multiaddr, _options: TransportOptions) -> Result<Box<dyn Connection>> {
        // For now, ignore options and use default connection
        // TODO: Map TransportOptions to Iroh configuration
        self.connect(addr).await
    }
    
    fn supported_addresses(&self) -> Vec<String> {
        vec![
            "/ip4/0.0.0.0/udp/0/quic".to_string(),
            "/ip6/::/udp/0/quic".to_string(),
            "/p2p-circuit/relay".to_string(), // Relay support
        ]
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::QUIC // Iroh uses QUIC under the hood
    }
    
    fn supports_address(&self, addr: &Multiaddr) -> bool {
        // Support QUIC addresses and relay addresses
        addr.contains("/quic") || addr.contains("/p2p-circuit")
    }
}

/// This is needed because we can't directly implement Clone for IrohTransport
/// due to the Endpoint not implementing Clone
impl Clone for IrohTransport {
    fn clone(&self) -> Self {
        // This is a simplified clone - in practice, we'd need to handle this more carefully
        // For now, panic to indicate this needs proper implementation
        panic!("IrohTransport clone not implemented - use Arc<IrohTransport> instead")
    }
}

/// Adapter to make IrohConnection work with the Transport trait
pub struct IrohConnectionAdapter {
    connection: Arc<Mutex<IrohConnection>>,
    local_transport: Arc<IrohTransport>,
}

#[async_trait]
impl Connection for IrohConnectionAdapter {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        let mut conn = self.connection.lock().await;
        conn.send_data(data).await
    }
    
    async fn receive(&mut self) -> Result<Vec<u8>> {
        let mut conn = self.connection.lock().await;
        conn.receive_data().await
    }
    
    async fn info(&self) -> ConnectionInfo {
        let conn = self.connection.lock().await;
        conn.get_info().await
    }
    
    async fn close(&mut self) -> Result<()> {
        let mut conn = self.connection.lock().await;
        conn.close().await
    }
    
    async fn is_alive(&self) -> bool {
        let conn = self.connection.lock().await;
        conn.is_alive().await
    }
    
    async fn measure_quality(&self) -> Result<ConnectionQuality> {
        let conn = self.connection.lock().await;
        conn.measure_quality().await
    }
    
    fn local_addr(&self) -> Multiaddr {
        // This would need the connection to be accessible
        "/ip4/127.0.0.1/udp/0/quic".to_string() // Placeholder
    }
    
    fn remote_addr(&self) -> Multiaddr {
        // This would need the connection to be accessible  
        "/ip4/127.0.0.1/udp/0/quic".to_string() // Placeholder
    }
}

impl IrohConnection {
    /// Send data over the connection
    pub async fn send_data(&mut self, data: &[u8]) -> Result<()> {
        // Open a new stream for each message (QUIC pattern)
        let mut send_stream = self.connection.open_uni().await
            .map_err(|e| P2PError::Network(format!("Failed to open stream: {}", e)))?;
        
        // Send data
        send_stream.write_all(data).await
            .map_err(|e| P2PError::Network(format!("Failed to send data: {}", e)))?;
        
        send_stream.finish().await
            .map_err(|e| P2PError::Network(format!("Failed to finish stream: {}", e)))?;
        
        // Update counters
        *self.bytes_sent.lock().await += data.len() as u64;
        *self.last_activity.lock().await = Instant::now();
        
        debug!("Sent {} bytes to peer {}", data.len(), self.remote_peer_id);
        Ok(())
    }
    
    /// Receive data from the connection
    pub async fn receive_data(&mut self) -> Result<Vec<u8>> {
        // Accept incoming stream
        let mut recv_stream = self.connection.accept_uni().await
            .map_err(|e| P2PError::Network(format!("Failed to accept stream: {}", e)))?;
        
        // Read all data from stream
        let mut buffer = Vec::new();
        recv_stream.read_to_end(&mut buffer).await
            .map_err(|e| P2PError::Network(format!("Failed to read data: {}", e)))?;
        
        // Update counters
        *self.bytes_received.lock().await += buffer.len() as u64;
        *self.last_activity.lock().await = Instant::now();
        
        debug!("Received {} bytes from peer {}", buffer.len(), self.remote_peer_id);
        Ok(buffer)
    }
    
    /// Get connection information
    pub async fn get_info(&self) -> ConnectionInfo {
        let local_addr = Self::socket_addr_to_multiaddr(&self.connection.local_ip_address())
            .unwrap_or_else(|_| "/ip4/0.0.0.0/udp/0/quic".to_string());
        let remote_addr = Self::socket_addr_to_multiaddr(&self.connection.remote_address())
            .unwrap_or_else(|_| "/ip4/0.0.0.0/udp/0/quic".to_string());
        
        ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr,
            remote_addr,
            is_encrypted: true, // QUIC is always encrypted
            cipher_suite: "QUIC-TLS-1.3".to_string(), // Simplified
            used_0rtt: false, // TODO: Get actual 0-RTT status from QUIC
            established_at: self.established_at,
            last_activity: *self.last_activity.lock().await,
        }
    }
    
    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        self.connection.close(0u32.into(), b"Connection closed");
        debug!("Closed connection to peer {}", self.remote_peer_id);
        Ok(())
    }
    
    /// Check if connection is alive
    pub async fn is_alive(&self) -> bool {
        // QUIC connections can be checked for liveness
        self.connection.close_reason().is_none()
    }
    
    /// Measure connection quality
    pub async fn measure_quality(&self) -> Result<ConnectionQuality> {
        // Check cache first
        if let Some(cached) = &*self.quality_cache.lock().await {
            if cached.connect_time.elapsed() < Duration::from_secs(30) {
                return Ok(cached.clone());
            }
        }
        
        let stats = self.connection.stats();
        
        // Create quality metrics from QUIC stats
        let quality = ConnectionQuality {
            latency: Duration::from_millis(stats.path.rtt.as_millis() as u64),
            throughput_mbps: 0.0, // TODO: Calculate from sent/received bytes over time
            packet_loss: stats.path.lost_packets as f64 / stats.path.sent_packets.max(1) as f64,
            jitter: Duration::from_millis(5), // TODO: Calculate actual jitter
            connect_time: self.established_at.elapsed(),
        };
        
        // Cache the result
        *self.quality_cache.lock().await = Some(quality.clone());
        
        Ok(quality)
    }
    
    /// Convert SocketAddr to Multiaddr (utility method)
    fn socket_addr_to_multiaddr(addr: &SocketAddr) -> Result<Multiaddr> {
        match addr {
            SocketAddr::V4(v4) => Ok(format!("/ip4/{}/udp/{}/quic", v4.ip(), v4.port())),
            SocketAddr::V6(v6) => Ok(format!("/ip6/{}/udp/{}/quic", v6.ip(), v6.port())),
        }
    }
}

impl Default for IrohConfig {
    fn default() -> Self {
        Self {
            relay_mode: RelayMode::Default,
            stun_only: false,
            prefer_ipv6: true,
            custom_relay_url: None,
            nat_traversal_timeout: Duration::from_secs(30),
            max_connections: 1000,
            enable_hole_punching: true,
        }
    }
}

impl Default for ConnectionQuality {
    fn default() -> Self {
        Self {
            latency: Duration::from_millis(50),
            throughput_mbps: 100.0,
            packet_loss: 0.0,
            jitter: Duration::from_millis(5),
            connect_time: Duration::from_millis(100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_iroh_config_default() {
        let config = IrohConfig::default();
        assert!(matches!(config.relay_mode, RelayMode::Default));
        assert!(!config.stun_only);
        assert!(config.prefer_ipv6);
        assert!(config.enable_hole_punching);
        assert_eq!(config.nat_traversal_timeout, Duration::from_secs(30));
    }
    
    #[test]
    fn test_connection_type_variants() {
        let types = [
            ConnectionType::DirectIPv4,
            ConnectionType::DirectIPv6,
            ConnectionType::Relay,
            ConnectionType::HolePunching,
        ];
        
        for conn_type in types {
            // Test serialization/deserialization
            let serialized = serde_json::to_string(&conn_type).unwrap();
            let deserialized: ConnectionType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(conn_type, deserialized);
        }
    }
    
    #[tokio::test]
    async fn test_peer_id_to_node_id_consistency() {
        let peer_id = "test_peer_123".to_string();
        
        // Should always produce the same NodeId for the same peer ID
        let node_id1 = IrohTransport::peer_id_to_node_id(&peer_id).unwrap();
        let node_id2 = IrohTransport::peer_id_to_node_id(&peer_id).unwrap();
        
        assert_eq!(node_id1, node_id2);
    }
    
    #[tokio::test]
    async fn test_derive_iroh_key_consistency() {
        let peer_id = "test_peer_456".to_string();
        
        // Should always produce the same key for the same peer ID
        let key1 = IrohTransport::derive_iroh_key(&peer_id).unwrap();
        let key2 = IrohTransport::derive_iroh_key(&peer_id).unwrap();
        
        assert_eq!(key1.public(), key2.public());
    }
    
    #[test]
    fn test_socket_addr_conversion() {
        let ipv4_addr: SocketAddr = "192.168.1.1:8080".parse().unwrap();
        let ipv6_addr: SocketAddr = "[::1]:8080".parse().unwrap();
        
        let ipv4_multi = IrohTransport::socket_addr_to_multiaddr(&ipv4_addr).unwrap();
        let ipv6_multi = IrohTransport::socket_addr_to_multiaddr(&ipv6_addr).unwrap();
        
        assert_eq!(ipv4_multi, "/ip4/192.168.1.1/udp/8080/quic");
        assert_eq!(ipv6_multi, "/ip6/::1/udp/8080/quic");
        
        // Test round-trip conversion
        let back_to_socket_ipv4 = IrohTransport::multiaddr_to_socket_addr(&ipv4_multi).unwrap();
        let back_to_socket_ipv6 = IrohTransport::multiaddr_to_socket_addr(&ipv6_multi).unwrap();
        
        assert_eq!(back_to_socket_ipv4, ipv4_addr);
        assert_eq!(back_to_socket_ipv6, ipv6_addr);
    }
    
    #[test]
    fn test_multiaddr_parsing() {
        let valid_addrs = [
            "/ip4/127.0.0.1/udp/9000/quic",
            "/ip6/::1/udp/8080/quic",
        ];
        
        for addr in valid_addrs {
            let result = IrohTransport::multiaddr_to_socket_addr(addr);
            assert!(result.is_ok(), "Failed to parse {}: {:?}", addr, result);
        }
        
        let invalid_addrs = [
            "/ip4/127.0.0.1/tcp/9000", // Wrong protocol
            "/ip4/127.0.0.1", // Missing port
            "invalid", // Not a multiaddr
        ];
        
        for addr in invalid_addrs {
            let result = IrohTransport::multiaddr_to_socket_addr(addr);
            assert!(result.is_err(), "Should have failed to parse {}", addr);
        }
    }
}