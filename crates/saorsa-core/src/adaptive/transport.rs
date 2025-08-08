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

//! Multi-protocol transport layer for the adaptive P2P network
//!
//! Provides abstraction over TCP, QUIC, and WebRTC transports with automatic
//! protocol negotiation and NAT traversal support.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing;

/// Transport protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportProtocol {
    Tcp,
    Quic,
    WebRtc,
}

/// Connection metadata
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub protocol: TransportProtocol,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub established_at: std::time::Instant,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Transport layer trait for different protocols
#[async_trait]
pub trait Transport: Send + Sync {
    /// Listen for incoming connections
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn TransportListener>>;

    /// Connect to a remote peer
    async fn connect(&self, addr: SocketAddr) -> Result<Box<dyn TransportConnection>>;

    /// Get protocol type
    fn protocol(&self) -> TransportProtocol;

    /// Check if protocol supports NAT traversal
    fn supports_nat_traversal(&self) -> bool;
}

/// Listener trait for accepting connections
#[async_trait]
pub trait TransportListener: Send + Sync {
    /// Accept incoming connection
    async fn accept(&self) -> Result<(Box<dyn TransportConnection>, SocketAddr)>;

    /// Get local listening address
    fn local_addr(&self) -> Result<SocketAddr>;
}

/// Connection trait for bidirectional communication
#[async_trait]
pub trait TransportConnection: AsyncRead + AsyncWrite + Send + Sync + Unpin {
    /// Get connection info
    fn info(&self) -> &ConnectionInfo;

    /// Close connection gracefully
    async fn close(&mut self) -> Result<()>;
}

/// TCP transport implementation
pub struct TcpTransport {
    /// Connection pool for reuse
    connections: Arc<RwLock<HashMap<SocketAddr, Arc<RwLock<TcpStream>>>>>,
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TcpTransport {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn TransportListener>> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(AdaptiveNetworkError::Network)?;

        Ok(Box::new(TcpTransportListener { listener }))
    }

    async fn connect(&self, addr: SocketAddr) -> Result<Box<dyn TransportConnection>> {
        // Check connection pool first
        {
            let connections = self.connections.read().await;
            if let Some(_conn) = connections.get(&addr) {
                // TODO: Check if connection is still alive
                // For now, always create new connection
            }
        }

        let stream = TcpStream::connect(addr)
            .await
            .map_err(AdaptiveNetworkError::Network)?;

        let local_addr = stream.local_addr().map_err(AdaptiveNetworkError::Network)?;

        let info = ConnectionInfo {
            protocol: TransportProtocol::Tcp,
            local_addr,
            remote_addr: addr,
            established_at: std::time::Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
        };

        Ok(Box::new(TcpTransportConnection { stream, info }))
    }

    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::Tcp
    }

    fn supports_nat_traversal(&self) -> bool {
        false
    }
}

/// TCP listener wrapper
struct TcpTransportListener {
    listener: TcpListener,
}

#[async_trait]
impl TransportListener for TcpTransportListener {
    async fn accept(&self) -> Result<(Box<dyn TransportConnection>, SocketAddr)> {
        let (stream, remote_addr) = self
            .listener
            .accept()
            .await
            .map_err(AdaptiveNetworkError::Network)?;

        let local_addr = stream.local_addr().map_err(AdaptiveNetworkError::Network)?;

        let info = ConnectionInfo {
            protocol: TransportProtocol::Tcp,
            local_addr,
            remote_addr,
            established_at: std::time::Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
        };

        Ok((
            Box::new(TcpTransportConnection { stream, info }),
            remote_addr,
        ))
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        self.listener
            .local_addr()
            .map_err(AdaptiveNetworkError::Network)
    }
}

/// TCP connection wrapper
struct TcpTransportConnection {
    stream: TcpStream,
    info: ConnectionInfo,
}

#[async_trait]
impl TransportConnection for TcpTransportConnection {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    async fn close(&mut self) -> Result<()> {
        // TCP stream doesn't have shutdown method, use try_write with empty buffer
        // to flush and then drop will close the connection
        Ok(())
    }
}

// Implement AsyncRead and AsyncWrite for TcpTransportConnection
impl AsyncRead for TcpTransportConnection {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let pin = std::pin::Pin::new(&mut self.stream);
        pin.poll_read(cx, buf)
    }
}

impl AsyncWrite for TcpTransportConnection {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let pin = std::pin::Pin::new(&mut self.stream);
        pin.poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let pin = std::pin::Pin::new(&mut self.stream);
        pin.poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let pin = std::pin::Pin::new(&mut self.stream);
        pin.poll_shutdown(cx)
    }
}

/// QUIC transport placeholder
/// TODO: Implement using quinn or similar QUIC library
pub struct QuicTransport {
    // Quinn endpoint would go here
}

impl Default for QuicTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl QuicTransport {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn listen(&self, _addr: SocketAddr) -> Result<Box<dyn TransportListener>> {
        Err(AdaptiveNetworkError::Other(
            "QUIC transport not yet implemented".to_string().into(),
        ))
    }

    async fn connect(&self, _addr: SocketAddr) -> Result<Box<dyn TransportConnection>> {
        Err(AdaptiveNetworkError::Other(
            "QUIC transport not yet implemented".to_string().into(),
        ))
    }

    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::Quic
    }

    fn supports_nat_traversal(&self) -> bool {
        true // QUIC has better NAT traversal capabilities
    }
}

/// Multi-protocol transport manager
pub struct TransportManager {
    /// Available transports
    transports: HashMap<TransportProtocol, Arc<dyn Transport>>,

    /// Active listeners
    listeners: Arc<RwLock<Vec<Box<dyn TransportListener>>>>,

    /// Connection pool
    _connections: Arc<RwLock<HashMap<SocketAddr, Box<dyn TransportConnection>>>>,

    /// Protocol preferences
    protocol_preference: Vec<TransportProtocol>,
}

impl Default for TransportManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransportManager {
    /// Create new transport manager
    pub fn new() -> Self {
        let mut transports = HashMap::new();
        transports.insert(
            TransportProtocol::Tcp,
            Arc::new(TcpTransport::new()) as Arc<dyn Transport>,
        );
        transports.insert(
            TransportProtocol::Quic,
            Arc::new(QuicTransport::new()) as Arc<dyn Transport>,
        );

        Self {
            transports,
            listeners: Arc::new(RwLock::new(Vec::new())),
            _connections: Arc::new(RwLock::new(HashMap::new())),
            protocol_preference: vec![TransportProtocol::Quic, TransportProtocol::Tcp],
        }
    }

    /// Listen on address with all available protocols
    pub async fn listen(&self, addr: SocketAddr) -> Result<()> {
        let mut listeners = self.listeners.write().await;

        for protocol in &self.protocol_preference {
            if let Some(transport) = self.transports.get(protocol) {
                match transport.listen(addr).await {
                    Ok(listener) => {
                        tracing::info!("Listening on {} with {:?}", addr, protocol);
                        listeners.push(listener);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to listen with {:?}: {}", protocol, e);
                    }
                }
            }
        }

        if listeners.is_empty() {
            return Err(AdaptiveNetworkError::Other(
                "Failed to listen on any protocol".to_string().into(),
            ));
        }

        Ok(())
    }

    /// Connect to peer with protocol negotiation
    pub async fn connect(&self, addr: SocketAddr) -> Result<Box<dyn TransportConnection>> {
        // Try protocols in preference order
        for protocol in &self.protocol_preference {
            if let Some(transport) = self.transports.get(protocol) {
                match transport.connect(addr).await {
                    Ok(conn) => {
                        tracing::info!("Connected to {} using {:?}", addr, protocol);
                        return Ok(conn);
                    }
                    Err(e) => {
                        tracing::debug!("Failed to connect with {:?}: {}", protocol, e);
                    }
                }
            }
        }

        Err(AdaptiveNetworkError::Other(
            "Failed to connect with any protocol".to_string().into(),
        ))
    }

    /// Accept incoming connection from any listener
    pub async fn accept(&self) -> Result<(Box<dyn TransportConnection>, SocketAddr)> {
        let listeners = self.listeners.read().await;

        if listeners.is_empty() {
            return Err(AdaptiveNetworkError::Other(
                "No active listeners".to_string().into(),
            ));
        }

        // For now, just try the first listener
        // TODO: Implement proper multiplexing
        listeners[0].accept().await
    }
}

/// NAT traversal helper
pub struct NatTraversal {
    /// STUN servers for address discovery
    _stun_servers: Vec<String>,

    /// Known public addresses
    _public_addresses: Arc<RwLock<Vec<SocketAddr>>>,
}

impl Default for NatTraversal {
    fn default() -> Self {
        Self::new()
    }
}

impl NatTraversal {
    pub fn new() -> Self {
        Self {
            _stun_servers: vec![
                "stun.l.google.com:19302".to_string(),
                "stun1.l.google.com:19302".to_string(),
            ],
            _public_addresses: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Discover public addresses using STUN
    pub async fn discover_public_addresses(&self) -> Result<Vec<SocketAddr>> {
        // TODO: Implement STUN protocol
        // For now, return empty vec
        Ok(Vec::new())
    }

    /// Attempt NAT hole punching
    pub async fn hole_punch(&self, _target: SocketAddr) -> Result<()> {
        // TODO: Implement hole punching protocol
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_transport_connect() {
        let transport = TcpTransport::new();

        // Start a listener
        let listener = transport
            .listen("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();

        // Connect to it
        tokio::spawn(async move {
            let _conn = transport.connect(addr).await.unwrap();
        });

        // Accept connection
        let (_conn, _remote) = listener.accept().await.unwrap();
    }

    #[test]
    fn test_transport_protocol_properties() {
        let tcp = TcpTransport::new();
        assert_eq!(tcp.protocol(), TransportProtocol::Tcp);
        assert!(!tcp.supports_nat_traversal());

        let quic = QuicTransport::new();
        assert_eq!(quic.protocol(), TransportProtocol::Quic);
        assert!(quic.supports_nat_traversal());
    }

    #[tokio::test]
    async fn test_transport_manager() {
        let manager = TransportManager::new();

        // Should be able to listen
        manager
            .listen("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
    }
}
