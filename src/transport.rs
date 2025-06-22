//! Transport Layer
//!
//! This module provides transport protocol implementations for the P2P Foundation.
//! It supports QUIC and TCP transports with automatic selection, connection pooling,
//! and performance monitoring.

pub mod tcp;
pub mod quic;

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
    /// Connection info cache
    info_cache: HashMap<String, ConnectionInfo>,
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
            info_cache: HashMap::new(),
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