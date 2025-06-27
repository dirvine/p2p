//! TCP Transport Implementation (Minimal Fallback)
//!
//! This module provides minimal TCP-based transport for P2P connections.
//! It serves as a fallback when QUIC is not available.
//! TCP functionality is intentionally limited - QUIC is preferred.

use super::{Transport, Connection, TransportType, TransportOptions, ConnectionInfo, ConnectionQuality};
use crate::{Multiaddr, P2PError, Result};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// TCP transport implementation (minimal fallback)
pub struct TcpTransport {
    /// Whether to require TLS encryption (simplified - always false for fallback)
    require_tls: bool,
    /// TCP listener for accepting incoming connections
    listener: Arc<Mutex<Option<Arc<TcpListener>>>>,
}

/// TCP connection implementation
pub struct TcpConnection {
    /// Underlying TCP stream
    stream: TcpStream,
    /// Local address
    local_addr: Multiaddr,
    /// Remote address
    remote_addr: Multiaddr,
    /// Connection info
    info: ConnectionInfo,
    /// Whether the connection is encrypted (reserved for future TLS support)
    _encrypted: bool,
}

impl TcpTransport {
    /// Create a new TCP transport (fallback only - TLS disabled for simplicity)
    pub fn new(_require_tls: bool) -> Self {
        warn!("TCP transport created as fallback. QUIC is preferred for P2P networking.");
        Self { 
            require_tls: false, // Always disable TLS in fallback mode
            listener: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn listen(&self, addr: SocketAddr) -> Result<Vec<Multiaddr>> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| P2PError::Transport(format!("Failed to bind TCP listener: {}", e)))?;
        
        let local_addr = listener.local_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get local address: {}", e)))?;
        
        info!("TCP transport listening on {}", local_addr);
        
        // Store the listener for accepting connections
        {
            let mut listener_guard = self.listener.lock().await;
            *listener_guard = Some(Arc::new(listener));
        }
        
        // Convert to multiaddr format
        let multiaddr = if local_addr.is_ipv6() {
            format!("/ip6/{}/tcp/{}", local_addr.ip(), local_addr.port())
        } else {
            format!("/ip4/{}/tcp/{}", local_addr.ip(), local_addr.port())
        };
        
        Ok(vec![multiaddr])
    }
    
    async fn accept(&self) -> Result<Box<dyn Connection>> {
        debug!("TCP waiting for incoming connection");
        
        // Get the listener
        let listener = {
            let listener_guard = self.listener.lock().await;
            listener_guard.as_ref().ok_or_else(|| {
                P2PError::Transport("TCP transport not listening - call listen() first".to_string())
            })?.clone()
        };
        
        // Accept incoming connection
        let (stream, remote_addr) = listener.accept().await
            .map_err(|e| P2PError::Transport(format!("Failed to accept TCP connection: {}", e)))?;
        
        let local_addr = stream.local_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get local address: {}", e)))?;
        
        // Convert addresses to multiaddr format
        let local_multiaddr = if local_addr.is_ipv6() {
            format!("/ip6/{}/tcp/{}", local_addr.ip(), local_addr.port())
        } else {
            format!("/ip4/{}/tcp/{}", local_addr.ip(), local_addr.port())
        };
        
        let remote_multiaddr = if remote_addr.is_ipv6() {
            format!("/ip6/{}/tcp/{}", remote_addr.ip(), remote_addr.port())
        } else {
            format!("/ip4/{}/tcp/{}", remote_addr.ip(), remote_addr.port())
        };
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::TCP,
            local_addr: local_multiaddr.clone(),
            remote_addr: remote_multiaddr.clone(),
            is_encrypted: false,
            cipher_suite: String::new(),
            used_0rtt: false,
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let connection = TcpConnection {
            stream,
            local_addr: local_multiaddr,
            remote_addr: remote_multiaddr,
            info: connection_info,
            _encrypted: false,
        };
        
        info!("TCP accepted incoming connection from {}", remote_addr);
        Ok(Box::new(connection))
    }
    
    async fn connect(&self, addr: &Multiaddr) -> Result<Box<dyn Connection>> {
        self.connect_with_options(addr, TransportOptions::default()).await
    }
    
    async fn connect_with_options(&self, addr: &Multiaddr, options: TransportOptions) -> Result<Box<dyn Connection>> {
        debug!("TCP connecting to {}", addr);
        
        // Parse multiaddr to get host:port
        let socket_addr = self.parse_multiaddr(addr)?;
        
        // Connect with timeout
        let stream = tokio::time::timeout(
            options.connect_timeout,
            TcpStream::connect(socket_addr)
        ).await
            .map_err(|_| P2PError::Transport("TCP connection timeout".to_string()))?
            .map_err(|e| P2PError::Transport(format!("TCP connection failed: {}", e)))?;
        
        let local_addr = stream.local_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get local address: {}", e)))?;
        let remote_addr = stream.peer_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get remote address: {}", e)))?;
        
        // Convert addresses to multiaddr format
        let local_multiaddr = if local_addr.is_ipv6() {
            format!("/ip6/{}/tcp/{}", local_addr.ip(), local_addr.port())
        } else {
            format!("/ip4/{}/tcp/{}", local_addr.ip(), local_addr.port())
        };
        
        let remote_multiaddr = if remote_addr.is_ipv6() {
            format!("/ip6/{}/tcp/{}", remote_addr.ip(), remote_addr.port())
        } else {
            format!("/ip4/{}/tcp/{}", remote_addr.ip(), remote_addr.port())
        };
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::TCP,
            local_addr: local_multiaddr.clone(),
            remote_addr: remote_multiaddr.clone(),
            is_encrypted: false, // TODO: Add TLS support
            cipher_suite: if self.require_tls { "TLS_AES_256_GCM_SHA384".to_string() } else { String::new() },
            used_0rtt: false,
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let connection = TcpConnection {
            stream,
            local_addr: local_multiaddr,
            remote_addr: remote_multiaddr,
            info: connection_info,
            _encrypted: self.require_tls,
        };
        
        info!("TCP connection established to {}", addr);
        Ok(Box::new(connection))
    }
    
    fn supported_addresses(&self) -> Vec<String> {
        warn!("TCP transport is fallback only. Consider using QUIC for better performance.");
        vec![
            "/ip4/0.0.0.0/tcp/0".to_string(),
            "/ip6/::/tcp/0".to_string(),
        ]
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::TCP
    }
    
    fn supports_address(&self, addr: &Multiaddr) -> bool {
        let supports = addr.contains("/tcp/") && (addr.contains("/ip4/") || addr.contains("/ip6/"));
        if supports {
            warn!("Using TCP fallback for address: {}. QUIC would provide better performance.", addr);
        }
        supports
    }
}

impl TcpTransport {
    /// Parse a multiaddr into a SocketAddr
    fn parse_multiaddr(&self, addr: &Multiaddr) -> Result<SocketAddr> {
        // Simple parsing for now - in reality this would use a proper multiaddr parser
        // Format: /ip4/127.0.0.1/tcp/9000 or /ip6/::1/tcp/9000
        
        let parts: Vec<&str> = addr.split('/').collect();
        if parts.len() < 5 {
            return Err(P2PError::Transport(format!("Invalid multiaddr format: {}", addr)));
        }
        
        let ip_str = parts[2];
        let port_str = parts[4];
        
        let port: u16 = port_str.parse()
            .map_err(|_| P2PError::Transport(format!("Invalid port in multiaddr: {}", port_str)))?;
        
        let socket_addr: SocketAddr = format!("{}:{}", ip_str, port).parse()
            .map_err(|_| P2PError::Transport(format!("Invalid address in multiaddr: {}", addr)))?;
        
        Ok(socket_addr)
    }
}

#[async_trait]
impl Connection for TcpConnection {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        debug!("TCP sending {} bytes", data.len());
        
        // Send length prefix + data
        let len = data.len() as u32;
        self.stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| P2PError::Transport(format!("Failed to send length: {}", e)))?;
        
        self.stream.write_all(data).await
            .map_err(|e| P2PError::Transport(format!("Failed to send data: {}", e)))?;
        
        self.stream.flush().await
            .map_err(|e| P2PError::Transport(format!("Failed to flush: {}", e)))?;
        
        // Update last activity
        self.info.last_activity = Instant::now();
        
        debug!("TCP sent {} bytes successfully", data.len());
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<Vec<u8>> {
        debug!("TCP receiving data");
        
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes).await
            .map_err(|e| P2PError::Transport(format!("Failed to read length: {}", e)))?;
        
        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > 64 * 1024 * 1024 { // 64MB max
            return Err(P2PError::Transport(format!("Message too large: {} bytes", len)));
        }
        
        // Read data
        let mut data = vec![0u8; len];
        self.stream.read_exact(&mut data).await
            .map_err(|e| P2PError::Transport(format!("Failed to read data: {}", e)))?;
        
        // Update last activity
        self.info.last_activity = Instant::now();
        
        debug!("TCP received {} bytes", data.len());
        Ok(data)
    }
    
    async fn info(&self) -> ConnectionInfo {
        self.info.clone()
    }
    
    async fn close(&mut self) -> Result<()> {
        debug!("Closing TCP connection");
        self.stream.shutdown().await
            .map_err(|e| P2PError::Transport(format!("Failed to close connection: {}", e)))?;
        Ok(())
    }
    
    async fn is_alive(&self) -> bool {
        // Check if the connection is still alive by checking the stream state
        // This is a simple implementation - in reality we might send a ping
        self.info.last_activity.elapsed() < Duration::from_secs(300) // 5 minutes
    }
    
    async fn measure_quality(&self) -> Result<ConnectionQuality> {
        let start = Instant::now();
        
        // Simulate quality measurement
        // In a real implementation, this would:
        // 1. Send ping packets and measure RTT
        // 2. Measure throughput with test data
        // 3. Check for packet loss
        
        tokio::time::sleep(Duration::from_millis(1)).await; // Simulate measurement
        
        let latency = start.elapsed();
        
        Ok(ConnectionQuality {
            latency,
            throughput_mbps: 100.0, // Placeholder
            packet_loss: 0.0,
            jitter: Duration::from_millis(2),
            connect_time: self.info.established_at.elapsed(),
        })
    }
    
    fn local_addr(&self) -> Multiaddr {
        self.local_addr.clone()
    }
    
    fn remote_addr(&self) -> Multiaddr {
        self.remote_addr.clone()
    }
}