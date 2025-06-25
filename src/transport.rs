//! Transport Layer
//!
//! This module provides transport protocol implementations for the P2P Foundation.
//! It supports QUIC and TCP transports with automatic selection, connection pooling,
//! and performance monitoring.

pub mod tcp;
pub mod quic;
pub mod tunneled;

use crate::{PeerId, Multiaddr, P2PError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Transport protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    /// QUIC transport protocol
    QUIC,
    /// TCP transport protocol  
    TCP,
}

/// Transport selection strategy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransportSelection {
    /// Automatically select best transport
    Auto,
    /// Prefer specific transport with fallback
    Prefer(TransportType),
    /// Force specific transport only
    Force(TransportType),
}

/// Connection quality metrics
#[derive(Debug, Clone)]
pub struct ConnectionQuality {
    /// Round-trip latency
    pub latency: Duration,
    /// Throughput in Mbps
    pub throughput_mbps: f64,
    /// Packet loss percentage
    pub packet_loss: f64,
    /// Jitter (latency variation)
    pub jitter: Duration,
    /// Connection establishment time
    pub connect_time: Duration,
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Transport type being used
    pub transport_type: TransportType,
    /// Local address
    pub local_addr: Multiaddr,
    /// Remote address
    pub remote_addr: Multiaddr,
    /// Whether connection is encrypted
    pub is_encrypted: bool,
    /// Cipher suite being used
    pub cipher_suite: String,
    /// Whether 0-RTT was used
    pub used_0rtt: bool,
    /// Connection establishment time
    pub established_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
}

/// Connection pool information
#[derive(Debug, Clone)]
pub struct ConnectionPoolInfo {
    /// Number of active connections
    pub active_connections: usize,
    /// Total connections ever created
    pub total_connections: usize,
    /// Bytes sent through pool
    pub bytes_sent: u64,
    /// Bytes received through pool
    pub bytes_received: u64,
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    /// Messages sent per connection
    pub messages_per_connection: HashMap<String, usize>,
    /// Bytes per connection
    pub bytes_per_connection: HashMap<String, u64>,
    /// Average latency per connection
    pub latency_per_connection: HashMap<String, Duration>,
}

/// Message received from transport
#[derive(Debug, Clone)]
pub struct TransportMessage {
    /// Sender peer ID
    pub sender: PeerId,
    /// Message data
    pub data: Vec<u8>,
    /// Protocol identifier
    pub protocol: String,
    /// Timestamp when received
    pub received_at: Instant,
}

/// Transport trait for protocol implementations
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start listening on the given address
    async fn listen(&self, addr: SocketAddr) -> Result<Vec<Multiaddr>>;
    
    /// Accept incoming connections (for server-side)
    async fn accept(&self) -> Result<Box<dyn Connection>>;
    
    /// Connect to a remote peer
    async fn connect(&self, addr: &Multiaddr) -> Result<Box<dyn Connection>>;
    
    /// Connect with specific transport options
    async fn connect_with_options(&self, addr: &Multiaddr, options: TransportOptions) -> Result<Box<dyn Connection>>;
    
    /// Get supported addresses for this transport
    fn supported_addresses(&self) -> Vec<String>;
    
    /// Get transport type
    fn transport_type(&self) -> TransportType;
    
    /// Check if address is supported
    fn supports_address(&self, addr: &Multiaddr) -> bool;
}

/// Connection trait for active connections
#[async_trait]
pub trait Connection: Send + Sync {
    /// Send data over the connection
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    
    /// Receive data from the connection
    async fn receive(&mut self) -> Result<Vec<u8>>;
    
    /// Get connection info
    async fn info(&self) -> ConnectionInfo;
    
    /// Close the connection
    async fn close(&mut self) -> Result<()>;
    
    /// Check if connection is alive
    async fn is_alive(&self) -> bool;
    
    /// Measure connection quality
    async fn measure_quality(&self) -> Result<ConnectionQuality>;
    
    /// Get local address
    fn local_addr(&self) -> Multiaddr;
    
    /// Get remote address
    fn remote_addr(&self) -> Multiaddr;
}

/// Transport configuration options
#[derive(Debug, Clone)]
pub struct TransportOptions {
    /// Enable 0-RTT for QUIC
    pub enable_0rtt: bool,
    /// Force encryption
    pub require_encryption: bool,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive: Duration,
    /// Maximum message size
    pub max_message_size: usize,
}

/// Transport manager coordinates different transport protocols
pub struct TransportManager {
    /// Available transports
    transports: HashMap<TransportType, Arc<dyn Transport>>,
    /// Active connections
    connections: Arc<RwLock<HashMap<PeerId, Arc<Mutex<ConnectionPool>>>>>,
    /// Transport selection strategy
    selection: TransportSelection,
    /// Configuration options
    options: TransportOptions,
}

/// Connection pool for a specific peer
struct ConnectionPool {
    /// Active connections
    connections: Vec<Arc<Mutex<Box<dyn Connection>>>>,
    /// Connection info cache (reserved for future use)
    _info_cache: HashMap<String, ConnectionInfo>,
    /// Pool statistics
    stats: ConnectionPoolStats,
    /// Pool configuration
    max_connections: usize,
    /// Round-robin index for load balancing
    round_robin_index: usize,
}

impl TransportManager {
    /// Create a new transport manager
    pub fn new(selection: TransportSelection, options: TransportOptions) -> Self {
        Self {
            transports: HashMap::new(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            selection,
            options,
        }
    }
    
    /// Register a transport implementation
    pub fn register_transport(&mut self, transport: Arc<dyn Transport>) {
        let transport_type = transport.transport_type();
        self.transports.insert(transport_type, transport);
        info!("Registered transport: {:?}", transport_type);
    }
    
    /// Connect to a peer using the best available transport
    pub async fn connect(&self, addr: &Multiaddr) -> Result<PeerId> {
        let transport_type = self.select_transport(addr).await?;
        let transport = self.transports.get(&transport_type)
            .ok_or_else(|| P2PError::Transport(format!("Transport {:?} not available", transport_type)))?;
        
        debug!("Connecting to {} using {:?}", addr, transport_type);
        
        let connection = transport.connect_with_options(addr, self.options.clone()).await?;
        let peer_id = format!("peer_from_{}", addr); // Placeholder peer ID extraction
        
        // Add to connection pool
        self.add_connection(peer_id.clone(), connection).await?;
        
        info!("Connected to peer {} via {:?}", peer_id, transport_type);
        Ok(peer_id)
    }
    
    /// Connect with specific transport
    pub async fn connect_with_transport(&self, addr: &Multiaddr, transport_type: TransportType) -> Result<PeerId> {
        let transport = self.transports.get(&transport_type)
            .ok_or_else(|| P2PError::Transport(format!("Transport {:?} not available", transport_type)))?;
        
        let connection = transport.connect_with_options(addr, self.options.clone()).await?;
        let peer_id = format!("peer_from_{}", addr);
        
        self.add_connection(peer_id.clone(), connection).await?;
        Ok(peer_id)
    }
    
    /// Send message to a peer
    pub async fn send_message(&self, peer_id: &PeerId, data: Vec<u8>) -> Result<()> {
        let connections = self.connections.read().await;
        let pool = connections.get(peer_id)
            .ok_or_else(|| P2PError::Network(format!("No connection to peer {}", peer_id)))?;
        
        let mut pool_guard = pool.lock().await;
        let connection = pool_guard.get_connection()?;
        
        let mut conn_guard = connection.lock().await;
        conn_guard.send(&data).await?;
        
        debug!("Sent {} bytes to peer {}", data.len(), peer_id);
        Ok(())
    }
    
    /// Get connection info for a peer
    pub async fn get_connection_info(&self, peer_id: &PeerId) -> Result<ConnectionInfo> {
        let connections = self.connections.read().await;
        let pool = connections.get(peer_id)
            .ok_or_else(|| P2PError::Network(format!("No connection to peer {}", peer_id)))?;
        
        let mut pool_guard = pool.lock().await;
        let connection = pool_guard.get_connection()?;
        let conn_guard = connection.lock().await;
        
        Ok(conn_guard.info().await)
    }
    
    /// Get connection pool info
    pub async fn get_connection_pool_info(&self, peer_id: &PeerId) -> Result<ConnectionPoolInfo> {
        let connections = self.connections.read().await;
        let pool = connections.get(peer_id)
            .ok_or_else(|| P2PError::Network(format!("No connection to peer {}", peer_id)))?;
        
        let pool_guard = pool.lock().await;
        Ok(ConnectionPoolInfo {
            active_connections: pool_guard.connections.len(),
            total_connections: pool_guard.stats.messages_per_connection.len(),
            bytes_sent: pool_guard.stats.bytes_per_connection.values().sum(),
            bytes_received: 0, // TODO: Track separately
        })
    }
    
    /// Get connection pool statistics
    pub async fn get_connection_pool_stats(&self, peer_id: &PeerId) -> Result<ConnectionPoolStats> {
        let connections = self.connections.read().await;
        let pool = connections.get(peer_id)
            .ok_or_else(|| P2PError::Network(format!("No connection to peer {}", peer_id)))?;
        
        let pool_guard = pool.lock().await;
        Ok(pool_guard.stats.clone())
    }
    
    /// Measure connection quality
    pub async fn measure_connection_quality(&self, peer_id: &PeerId) -> Result<ConnectionQuality> {
        let connections = self.connections.read().await;
        let pool = connections.get(peer_id)
            .ok_or_else(|| P2PError::Network(format!("No connection to peer {}", peer_id)))?;
        
        let mut pool_guard = pool.lock().await;
        let connection = pool_guard.get_connection()?;
        let conn_guard = connection.lock().await;
        
        conn_guard.measure_quality().await
    }
    
    /// Switch transport for a peer
    pub async fn switch_transport(&self, peer_id: &PeerId, _new_transport: TransportType) -> Result<()> {
        // This is a placeholder implementation
        // In reality, this would establish a new connection with the new transport
        // and gracefully migrate the existing connection
        
        warn!("Transport switching not yet fully implemented for peer {}", peer_id);
        Ok(())
    }
    
    /// Select best transport for an address
    async fn select_transport(&self, addr: &Multiaddr) -> Result<TransportType> {
        match &self.selection {
            TransportSelection::Force(transport_type) => {
                if self.transports.contains_key(transport_type) {
                    Ok(*transport_type)
                } else {
                    Err(P2PError::Transport(format!("Forced transport {:?} not available", transport_type)))
                }
            }
            TransportSelection::Prefer(preferred) => {
                if self.transports.contains_key(preferred) {
                    Ok(*preferred)
                } else {
                    // Fall back to any available transport
                    self.auto_select_transport(addr).await
                }
            }
            TransportSelection::Auto => {
                self.auto_select_transport(addr).await
            }
        }
    }
    
    /// Auto-select best transport based on address and conditions
    async fn auto_select_transport(&self, addr: &Multiaddr) -> Result<TransportType> {
        // Strongly prefer QUIC if available (better performance, 0-RTT, multiplexing)
        if self.transports.contains_key(&TransportType::QUIC) {
            if let Some(transport) = self.transports.get(&TransportType::QUIC) {
                if transport.supports_address(addr) {
                    debug!("Selected QUIC transport for {} (preferred for P2P)", addr);
                    return Ok(TransportType::QUIC);
                }
            }
        }
        
        // Fall back to TCP only as last resort
        if self.transports.contains_key(&TransportType::TCP) {
            if let Some(transport) = self.transports.get(&TransportType::TCP) {
                if transport.supports_address(addr) {
                    warn!("Falling back to TCP transport for {}. QUIC would provide better performance.", addr);
                    return Ok(TransportType::TCP);
                }
            }
        }
        
        Err(P2PError::Transport("No suitable transport available. Consider using QUIC-compatible addresses.".to_string()))
    }
    
    /// Add connection to pool
    async fn add_connection(&self, peer_id: PeerId, connection: Box<dyn Connection>) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        let pool = connections.entry(peer_id.clone()).or_insert_with(|| {
            Arc::new(Mutex::new(ConnectionPool::new(3))) // Default max 3 connections per peer
        });
        
        let mut pool_guard = pool.lock().await;
        pool_guard.add_connection(connection).await?;
        
        Ok(())
    }
}

impl ConnectionPool {
    /// Create a new connection pool
    fn new(max_connections: usize) -> Self {
        Self {
            connections: Vec::new(),
            _info_cache: HashMap::new(),
            stats: ConnectionPoolStats {
                messages_per_connection: HashMap::new(),
                bytes_per_connection: HashMap::new(),
                latency_per_connection: HashMap::new(),
            },
            max_connections,
            round_robin_index: 0,
        }
    }
    
    /// Add a connection to the pool
    async fn add_connection(&mut self, connection: Box<dyn Connection>) -> Result<()> {
        if self.connections.len() >= self.max_connections {
            // Remove oldest connection
            self.connections.remove(0);
        }
        
        let conn_id = format!("conn_{}", self.connections.len());
        self.stats.messages_per_connection.insert(conn_id.clone(), 0);
        self.stats.bytes_per_connection.insert(conn_id.clone(), 0);
        self.stats.latency_per_connection.insert(conn_id, Duration::from_millis(0));
        
        self.connections.push(Arc::new(Mutex::new(connection)));
        Ok(())
    }
    
    /// Get a connection using round-robin load balancing
    fn get_connection(&mut self) -> Result<Arc<Mutex<Box<dyn Connection>>>> {
        if self.connections.is_empty() {
            return Err(P2PError::Network("No connections available".to_string()));
        }
        
        let connection = self.connections[self.round_robin_index % self.connections.len()].clone();
        self.round_robin_index += 1;
        
        // Update stats
        let conn_id = format!("conn_{}", self.round_robin_index % self.connections.len());
        if let Some(count) = self.stats.messages_per_connection.get_mut(&conn_id) {
            *count += 1;
        }
        
        Ok(connection)
    }
}

impl fmt::Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportType::QUIC => write!(f, "quic"),
            TransportType::TCP => write!(f, "tcp"),
        }
    }
}

impl Default for TransportSelection {
    fn default() -> Self {
        // Default to preferring QUIC with TCP fallback
        TransportSelection::Prefer(TransportType::QUIC)
    }
}

impl Default for TransportOptions {
    fn default() -> Self {
        Self {
            enable_0rtt: true,
            require_encryption: true,
            connect_timeout: Duration::from_secs(30),
            keep_alive: Duration::from_secs(60),
            max_message_size: 64 * 1024 * 1024, // 64MB
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


/// Legacy transport types module for backward compatibility
pub mod transport_types {
    pub use super::TransportType;
}

// Re-export transport implementations
pub use tcp::TcpTransport;
pub use quic::QuicTransport;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::Duration;

    /// Mock transport implementation for testing
    struct MockTransport {
        transport_type: TransportType,
        should_fail: bool,
        supports_all: bool,
    }

    impl MockTransport {
        fn new(transport_type: TransportType) -> Self {
            Self {
                transport_type,
                should_fail: false,
                supports_all: true,
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn with_limited_support(mut self) -> Self {
            self.supports_all = false;
            self
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn listen(&self, _addr: SocketAddr) -> Result<Vec<Multiaddr>> {
            if self.should_fail {
                return Err(P2PError::Transport("Listen failed".to_string()));
            }
            Ok(vec!["/ip4/127.0.0.1/tcp/9000".to_string()])
        }

        async fn connect(&self, addr: &Multiaddr) -> Result<Box<dyn Connection>> {
            if self.should_fail {
                return Err(P2PError::Transport("Connection failed".to_string()));
            }
            Ok(Box::new(MockConnection::new(addr.clone())))
        }

        async fn connect_with_options(&self, addr: &Multiaddr, _options: TransportOptions) -> Result<Box<dyn Connection>> {
            self.connect(addr).await
        }

        fn supported_addresses(&self) -> Vec<String> {
            if self.supports_all {
                vec!["/ip4/0.0.0.0/tcp/0".to_string(), "/ip6/::/tcp/0".to_string()]
            } else {
                vec!["/ip4/0.0.0.0/tcp/0".to_string()]
            }
        }

        fn transport_type(&self) -> TransportType {
            self.transport_type
        }

        fn supports_address(&self, addr: &Multiaddr) -> bool {
            if !self.supports_all && addr.contains("ip6") {
                return false;
            }
            addr.contains("tcp") || addr.contains("quic")
        }
    }

    /// Mock connection implementation for testing
    struct MockConnection {
        remote_addr: Multiaddr,
        is_alive: bool,
        bytes_sent: AtomicUsize,
        bytes_received: AtomicUsize,
    }

    impl MockConnection {
        fn new(remote_addr: Multiaddr) -> Self {
            Self {
                remote_addr,
                is_alive: true,
                bytes_sent: AtomicUsize::new(0),
                bytes_received: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl Connection for MockConnection {
        async fn send(&mut self, data: &[u8]) -> Result<()> {
            if !self.is_alive {
                return Err(P2PError::Network("Connection closed".to_string()));
            }
            self.bytes_sent.fetch_add(data.len(), Ordering::Relaxed);
            Ok(())
        }

        async fn receive(&mut self) -> Result<Vec<u8>> {
            if !self.is_alive {
                return Err(P2PError::Network("Connection closed".to_string()));
            }
            let data = b"mock_response".to_vec();
            self.bytes_received.fetch_add(data.len(), Ordering::Relaxed);
            Ok(data)
        }

        async fn info(&self) -> ConnectionInfo {
            ConnectionInfo {
                transport_type: TransportType::QUIC,
                local_addr: "/ip4/127.0.0.1/tcp/9000".to_string(),
                remote_addr: self.remote_addr.clone(),
                is_encrypted: true,
                cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(),
                used_0rtt: false,
                established_at: Instant::now(),
                last_activity: Instant::now(),
            }
        }

        async fn close(&mut self) -> Result<()> {
            self.is_alive = false;
            Ok(())
        }

        async fn is_alive(&self) -> bool {
            self.is_alive
        }

        async fn measure_quality(&self) -> Result<ConnectionQuality> {
            Ok(ConnectionQuality {
                latency: Duration::from_millis(10),
                throughput_mbps: 1000.0,
                packet_loss: 0.1,
                jitter: Duration::from_millis(2),
                connect_time: Duration::from_millis(50),
            })
        }

        fn local_addr(&self) -> Multiaddr {
            "/ip4/127.0.0.1/tcp/9000".to_string()
        }

        fn remote_addr(&self) -> Multiaddr {
            self.remote_addr.clone()
        }
    }

    fn create_test_transport_manager() -> TransportManager {
        let options = TransportOptions::default();
        TransportManager::new(TransportSelection::Auto, options)
    }

    #[test]
    fn test_transport_type_display() {
        assert_eq!(format!("{}", TransportType::QUIC), "quic");
        assert_eq!(format!("{}", TransportType::TCP), "tcp");
    }

    #[test]
    fn test_transport_type_serialization() {
        let quic_type = TransportType::QUIC;
        let tcp_type = TransportType::TCP;

        assert_eq!(quic_type, TransportType::QUIC);
        assert_eq!(tcp_type, TransportType::TCP);
        assert_ne!(quic_type, tcp_type);
    }

    #[test]
    fn test_transport_selection_variants() {
        let auto = TransportSelection::Auto;
        let prefer_quic = TransportSelection::Prefer(TransportType::QUIC);
        let force_tcp = TransportSelection::Force(TransportType::TCP);

        assert!(matches!(auto, TransportSelection::Auto));
        assert!(matches!(prefer_quic, TransportSelection::Prefer(TransportType::QUIC)));
        assert!(matches!(force_tcp, TransportSelection::Force(TransportType::TCP)));
    }

    #[test]
    fn test_transport_selection_default() {
        let default = TransportSelection::default();
        assert!(matches!(default, TransportSelection::Prefer(TransportType::QUIC)));
    }

    #[test]
    fn test_transport_options_default() {
        let options = TransportOptions::default();
        
        assert!(options.enable_0rtt);
        assert!(options.require_encryption);
        assert_eq!(options.connect_timeout, Duration::from_secs(30));
        assert_eq!(options.keep_alive, Duration::from_secs(60));
        assert_eq!(options.max_message_size, 64 * 1024 * 1024);
    }

    #[test]
    fn test_connection_quality_default() {
        let quality = ConnectionQuality::default();
        
        assert_eq!(quality.latency, Duration::from_millis(50));
        assert_eq!(quality.throughput_mbps, 100.0);
        assert_eq!(quality.packet_loss, 0.0);
        assert_eq!(quality.jitter, Duration::from_millis(5));
        assert_eq!(quality.connect_time, Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_transport_manager_creation() {
        let manager = create_test_transport_manager();
        assert!(manager.transports.is_empty());
    }

    #[tokio::test]
    async fn test_transport_registration() {
        let mut manager = create_test_transport_manager();
        let quic_transport = Arc::new(MockTransport::new(TransportType::QUIC));
        let tcp_transport = Arc::new(MockTransport::new(TransportType::TCP));

        manager.register_transport(quic_transport.clone());
        manager.register_transport(tcp_transport.clone());

        assert_eq!(manager.transports.len(), 2);
        assert!(manager.transports.contains_key(&TransportType::QUIC));
        assert!(manager.transports.contains_key(&TransportType::TCP));
    }

    #[tokio::test]
    async fn test_connection_establishment() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::QUIC));
        manager.register_transport(transport);

        let peer_id = manager.connect(&"/ip4/127.0.0.1/tcp/9001".to_string()).await?;
        assert_eq!(peer_id, "peer_from_/ip4/127.0.0.1/tcp/9001");

        let connections = manager.connections.read().await;
        assert!(connections.contains_key(&peer_id));

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_with_specific_transport() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::TCP));
        manager.register_transport(transport);

        let peer_id = manager.connect_with_transport(
            &"/ip4/127.0.0.1/tcp/9002".to_string(),
            TransportType::TCP
        ).await?;

        assert_eq!(peer_id, "peer_from_/ip4/127.0.0.1/tcp/9002");
        Ok(())
    }

    #[tokio::test]
    async fn test_connection_failure_handling() {
        let mut manager = create_test_transport_manager();
        let failing_transport = Arc::new(MockTransport::new(TransportType::QUIC).with_failure());
        manager.register_transport(failing_transport);

        let result = manager.connect(&"/ip4/127.0.0.1/tcp/9003".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection failed"));
    }

    #[tokio::test]
    async fn test_message_sending() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::QUIC));
        manager.register_transport(transport);

        let peer_id = manager.connect(&"/ip4/127.0.0.1/tcp/9004".to_string()).await?;
        let message = b"Hello, transport!".to_vec();
        
        manager.send_message(&peer_id, message.clone()).await?;

        // Verify message was processed
        let pool_info = manager.get_connection_pool_info(&peer_id).await?;
        assert_eq!(pool_info.active_connections, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_message_sending_no_connection() {
        let manager = create_test_transport_manager();
        let result = manager.send_message(&"nonexistent_peer".to_string(), vec![1, 2, 3]).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No connection to peer"));
    }

    #[tokio::test]
    async fn test_connection_info_retrieval() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::QUIC));
        manager.register_transport(transport);

        let peer_id = manager.connect(&"/ip4/127.0.0.1/tcp/9005".to_string()).await?;
        let info = manager.get_connection_info(&peer_id).await?;

        assert_eq!(info.transport_type, TransportType::QUIC);
        assert_eq!(info.remote_addr, "/ip4/127.0.0.1/tcp/9005");
        assert!(info.is_encrypted);
        assert_eq!(info.cipher_suite, "TLS_AES_256_GCM_SHA384");

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_pool_info() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::QUIC));
        manager.register_transport(transport);

        let peer_id = manager.connect(&"/ip4/127.0.0.1/tcp/9006".to_string()).await?;
        let pool_info = manager.get_connection_pool_info(&peer_id).await?;

        assert_eq!(pool_info.active_connections, 1);
        assert_eq!(pool_info.total_connections, 1);
        assert_eq!(pool_info.bytes_sent, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_pool_stats() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::QUIC));
        manager.register_transport(transport);

        let peer_id = manager.connect(&"/ip4/127.0.0.1/tcp/9007".to_string()).await?;
        let stats = manager.get_connection_pool_stats(&peer_id).await?;

        assert_eq!(stats.messages_per_connection.len(), 1);
        assert_eq!(stats.bytes_per_connection.len(), 1);
        assert_eq!(stats.latency_per_connection.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_quality_measurement() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::QUIC));
        manager.register_transport(transport);

        let peer_id = manager.connect(&"/ip4/127.0.0.1/tcp/9008".to_string()).await?;
        let quality = manager.measure_connection_quality(&peer_id).await?;

        assert_eq!(quality.latency, Duration::from_millis(10));
        assert_eq!(quality.throughput_mbps, 1000.0);
        assert_eq!(quality.packet_loss, 0.1);
        assert_eq!(quality.jitter, Duration::from_millis(2));

        Ok(())
    }

    #[tokio::test]
    async fn test_transport_switching() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let transport = Arc::new(MockTransport::new(TransportType::QUIC));
        manager.register_transport(transport);

        let peer_id = manager.connect(&"/ip4/127.0.0.1/tcp/9009".to_string()).await?;
        
        // Transport switching is not fully implemented, but should not error
        let result = manager.switch_transport(&peer_id, TransportType::TCP).await;
        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_auto_transport_selection_prefer_quic() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let quic_transport = Arc::new(MockTransport::new(TransportType::QUIC));
        let tcp_transport = Arc::new(MockTransport::new(TransportType::TCP));
        
        manager.register_transport(quic_transport);
        manager.register_transport(tcp_transport);

        let addr = "/ip4/127.0.0.1/tcp/9010".to_string();
        let selected = manager.auto_select_transport(&addr).await?;
        
        // Should prefer QUIC when available
        assert_eq!(selected, TransportType::QUIC);

        Ok(())
    }

    #[tokio::test]
    async fn test_transport_selection_fallback_to_tcp() -> Result<()> {
        let mut manager = create_test_transport_manager();
        let tcp_transport = Arc::new(MockTransport::new(TransportType::TCP));
        
        manager.register_transport(tcp_transport);

        let addr = "/ip4/127.0.0.1/tcp/9011".to_string();
        let selected = manager.auto_select_transport(&addr).await?;
        
        // Should fall back to TCP when QUIC not available
        assert_eq!(selected, TransportType::TCP);

        Ok(())
    }

    #[tokio::test]
    async fn test_transport_selection_no_suitable_transport() {
        let manager = create_test_transport_manager();
        let addr = "/ip4/127.0.0.1/tcp/9012".to_string();
        
        let result = manager.auto_select_transport(&addr).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No suitable transport available"));
    }

    #[tokio::test]
    async fn test_forced_transport_selection() -> Result<()> {
        let mut manager = TransportManager::new(
            TransportSelection::Force(TransportType::TCP),
            TransportOptions::default()
        );
        let tcp_transport = Arc::new(MockTransport::new(TransportType::TCP));
        
        manager.register_transport(tcp_transport);

        let addr = "/ip4/127.0.0.1/tcp/9013".to_string();
        let selected = manager.select_transport(&addr).await?;
        
        assert_eq!(selected, TransportType::TCP);

        Ok(())
    }

    #[tokio::test]
    async fn test_forced_transport_unavailable() {
        let manager = TransportManager::new(
            TransportSelection::Force(TransportType::QUIC),
            TransportOptions::default()
        );

        let addr = "/ip4/127.0.0.1/tcp/9014".to_string();
        let result = manager.select_transport(&addr).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Forced transport QUIC not available"));
    }

    #[tokio::test]
    async fn test_preferred_transport_with_fallback() -> Result<()> {
        let mut manager = TransportManager::new(
            TransportSelection::Prefer(TransportType::QUIC),
            TransportOptions::default()
        );
        let tcp_transport = Arc::new(MockTransport::new(TransportType::TCP));
        
        manager.register_transport(tcp_transport);

        let addr = "/ip4/127.0.0.1/tcp/9015".to_string();
        let selected = manager.select_transport(&addr).await?;
        
        // Should fall back to TCP when preferred QUIC is not available
        assert_eq!(selected, TransportType::TCP);

        Ok(())
    }

    #[tokio::test]
    async fn test_mock_connection_lifecycle() -> Result<()> {
        let mut conn = MockConnection::new("/ip4/127.0.0.1/tcp/9016".to_string());

        assert!(conn.is_alive().await);

        // Test sending
        conn.send(b"test message").await?;
        assert_eq!(conn.bytes_sent.load(Ordering::Relaxed), 12);

        // Test receiving
        let received = conn.receive().await?;
        assert_eq!(received, b"mock_response");
        assert_eq!(conn.bytes_received.load(Ordering::Relaxed), 13);

        // Test connection info
        let info = conn.info().await;
        assert_eq!(info.transport_type, TransportType::QUIC);
        assert!(info.is_encrypted);

        // Test quality measurement
        let quality = conn.measure_quality().await?;
        assert_eq!(quality.latency, Duration::from_millis(10));

        // Test close
        conn.close().await?;
        assert!(!conn.is_alive().await);

        // Operations should fail after close
        let result = conn.send(b"test").await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_pool_max_connections() -> Result<()> {
        let mut pool = ConnectionPool::new(2); // Max 2 connections

        // Add first connection
        let conn1 = Box::new(MockConnection::new("/ip4/127.0.0.1/tcp/9017".to_string()));
        pool.add_connection(conn1).await?;
        assert_eq!(pool.connections.len(), 1);

        // Add second connection
        let conn2 = Box::new(MockConnection::new("/ip4/127.0.0.1/tcp/9018".to_string()));
        pool.add_connection(conn2).await?;
        assert_eq!(pool.connections.len(), 2);

        // Add third connection (should remove first)
        let conn3 = Box::new(MockConnection::new("/ip4/127.0.0.1/tcp/9019".to_string()));
        pool.add_connection(conn3).await?;
        assert_eq!(pool.connections.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_pool_round_robin() -> Result<()> {
        let mut pool = ConnectionPool::new(3);

        // Add connections
        for i in 0..3 {
            let conn = Box::new(MockConnection::new(format!("/ip4/127.0.0.1/tcp/{}", 9020 + i)));
            pool.add_connection(conn).await?;
        }

        // Test round-robin selection
        let conn1 = pool.get_connection()?;
        let conn2 = pool.get_connection()?;
        let conn3 = pool.get_connection()?;
        let conn4 = pool.get_connection()?; // Should wrap around

        // All connections should be different (until wraparound)
        assert_ne!(Arc::as_ptr(&conn1), Arc::as_ptr(&conn2));
        assert_ne!(Arc::as_ptr(&conn2), Arc::as_ptr(&conn3));
        // Fourth should be same as first (round-robin)
        assert_eq!(Arc::as_ptr(&conn1), Arc::as_ptr(&conn4));

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_pool_empty() {
        let mut pool = ConnectionPool::new(3);
        let result = pool.get_connection();
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("No connections available"));
        }
    }

    #[tokio::test]
    async fn test_transport_message_structure() {
        let message = TransportMessage {
            sender: "test_peer".to_string(),
            data: vec![1, 2, 3, 4],
            protocol: "/p2p/test/1.0.0".to_string(),
            received_at: Instant::now(),
        };

        assert_eq!(message.sender, "test_peer");
        assert_eq!(message.data, vec![1, 2, 3, 4]);
        assert_eq!(message.protocol, "/p2p/test/1.0.0");
    }

    #[tokio::test]
    async fn test_mock_transport_address_support() {
        let transport = MockTransport::new(TransportType::QUIC);
        
        assert!(transport.supports_address(&"/ip4/127.0.0.1/tcp/9000".to_string()));
        assert!(transport.supports_address(&"/ip6/::1/tcp/9000".to_string()));
        assert!(!transport.supports_address(&"/ip4/127.0.0.1/udp/9000".to_string()));

        let limited_transport = MockTransport::new(TransportType::QUIC).with_limited_support();
        assert!(limited_transport.supports_address(&"/ip4/127.0.0.1/tcp/9000".to_string()));
        assert!(!limited_transport.supports_address(&"/ip6/::1/tcp/9000".to_string()));
    }

    #[tokio::test]
    async fn test_mock_transport_supported_addresses() {
        let transport = MockTransport::new(TransportType::QUIC);
        let addresses = transport.supported_addresses();
        
        assert_eq!(addresses.len(), 2);
        assert!(addresses.contains(&"/ip4/0.0.0.0/tcp/0".to_string()));
        assert!(addresses.contains(&"/ip6/::/tcp/0".to_string()));

        let limited_transport = MockTransport::new(TransportType::QUIC).with_limited_support();
        let limited_addresses = limited_transport.supported_addresses();
        
        assert_eq!(limited_addresses.len(), 1);
        assert!(limited_addresses.contains(&"/ip4/0.0.0.0/tcp/0".to_string()));
    }

    #[tokio::test]
    async fn test_transport_options_configuration() {
        let options = TransportOptions {
            enable_0rtt: false,
            require_encryption: false,
            connect_timeout: Duration::from_secs(10),
            keep_alive: Duration::from_secs(30),
            max_message_size: 1024,
        };

        assert!(!options.enable_0rtt);
        assert!(!options.require_encryption);
        assert_eq!(options.connect_timeout, Duration::from_secs(10));
        assert_eq!(options.keep_alive, Duration::from_secs(30));
        assert_eq!(options.max_message_size, 1024);
    }
}