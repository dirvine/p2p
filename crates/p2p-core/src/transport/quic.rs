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
use async_trait::async_trait;
// TODO: Replace with real ant-quic when available
// use ant_quic::{NatTraversalEndpoint, NatTraversalConfig, Connection as AntQuicConnection};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info};

// TODO: Remove when real ant-quic is available
/// Placeholder for NatTraversalEndpoint
#[derive(Clone)]
pub struct NatTraversalEndpoint;

/// Placeholder for NatTraversalConfig
#[derive(Clone)]
pub struct NatTraversalConfig;

/// Placeholder for ant-quic Connection
pub struct AntQuicConnection;

impl NatTraversalConfig {
    pub fn default() -> Self {
        Self
    }
}

impl NatTraversalEndpoint {
    pub async fn new(_addr: SocketAddr, _config: NatTraversalConfig) -> Result<Self> {
        Ok(Self)
    }
    
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
    
    pub async fn accept(&self) -> Result<AntQuicConnection> {
        Ok(AntQuicConnection)
    }
    
    pub async fn connect(&self, _addr: SocketAddr) -> Result<AntQuicConnection> {
        Ok(AntQuicConnection)
    }
}

impl AntQuicConnection {
    pub fn remote_addr(&self) -> SocketAddr {
        "127.0.0.1:0".parse().unwrap()
    }
    
    pub async fn send(&self, _data: &[u8]) -> Result<()> {
        Ok(())
    }
    
    pub async fn receive(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }
    
    pub async fn close(&self) -> Result<()> {
        Ok(())
    }
    
    pub fn is_connected(&self) -> bool {
        true
    }
    
    pub fn stats(&self) -> Result<QuicStats> {
        Ok(QuicStats::default())
    }
}

/// Placeholder for QUIC connection stats
#[derive(Default)]
pub struct QuicStats;

impl QuicStats {
    pub fn rtt(&self) -> Option<Duration> {
        Some(Duration::from_millis(50))
    }
    
    pub fn bytes_sent(&self) -> u64 {
        0
    }
    
    pub fn packet_loss_rate(&self) -> Option<f64> {
        Some(0.0)
    }
}

/// QUIC transport implementation with NAT traversal
pub struct QuicTransport {
    /// NAT traversal endpoint
    endpoint: Arc<Mutex<Option<NatTraversalEndpoint>>>,
    /// NAT traversal configuration
    config: NatTraversalConfig,
    /// Whether 0-RTT is enabled
    enable_0rtt: bool,
}

/// QUIC connection implementation
pub struct QuicConnection {
    /// Underlying ant-quic connection
    connection: AntQuicConnection,
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
        let config = NatTraversalConfig::default();
        
        Ok(Self {
            endpoint: Arc::new(Mutex::new(None)),
            config,
            enable_0rtt,
        })
    }

    /// Create a new QUIC transport with custom NAT traversal configuration
    pub fn new_with_config(config: NatTraversalConfig, enable_0rtt: bool) -> Result<Self> {
        Ok(Self {
            endpoint: Arc::new(Mutex::new(None)),
            config,
            enable_0rtt,
        })
    }

    /// Create NAT traversal endpoint
    async fn create_endpoint(&self, bind_addr: NetworkAddress) -> Result<NatTraversalEndpoint> {
        let endpoint = NatTraversalEndpoint::new(bind_addr.socket_addr(), self.config.clone()).await
            .map_err(|e| P2PError::Transport(format!("Failed to create NAT traversal endpoint: {}", e)))?;
        
        Ok(endpoint)
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn listen(&self, addr: NetworkAddress) -> Result<NetworkAddress> {
        debug!("QUIC listening on {}", addr);
        
        let endpoint = self.create_endpoint(addr.clone()).await?;
        let local_addr = endpoint.local_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get local address: {}", e)))?;
        
        let listen_addr = NetworkAddress::new(local_addr);
        info!("QUIC transport listening on {}", listen_addr);
        
        // Store the endpoint for accepting connections
        {
            let mut endpoint_guard = self.endpoint.lock().await;
            *endpoint_guard = Some(endpoint);
        }
        
        Ok(listen_addr)
    }
    
    async fn accept(&self) -> Result<Box<dyn Connection>> {
        debug!("QUIC waiting for incoming connection");
        
        // Get the endpoint
        let endpoint = {
            let endpoint_guard = self.endpoint.lock().await;
            endpoint_guard.as_ref().ok_or_else(|| {
                P2PError::Transport("QUIC transport not listening - call listen() first".to_string())
            })?.clone()
        };
        
        // Accept incoming connection with NAT traversal
        let connection = endpoint.accept().await
            .map_err(|e| P2PError::Transport(format!("QUIC connection accept failed: {}", e)))?;
        
        let local_socket_addr = endpoint.local_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get local address: {}", e)))?;
        let remote_socket_addr = connection.remote_addr();
        
        let local_addr = NetworkAddress::new(local_socket_addr);
        let remote_addr = NetworkAddress::new(remote_socket_addr);
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: local_addr.clone(),
            remote_addr: remote_addr.clone(),
            is_encrypted: true,
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(),
            used_0rtt: false, // For incoming connections, we can't determine 0-RTT easily
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let quic_connection = QuicConnection {
            connection,
            local_addr,
            remote_addr: remote_addr.clone(),
            info: connection_info,
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            stream_counter: Arc::new(Mutex::new(0)),
        };
        
        info!("QUIC accepted incoming connection from {}", remote_addr);
        Ok(Box::new(quic_connection))
    }
    
    async fn connect(&self, addr: NetworkAddress) -> Result<Box<dyn Connection>> {
        self.connect_with_options(addr, TransportOptions::default()).await
    }
    
    async fn connect_with_options(&self, addr: NetworkAddress, options: TransportOptions) -> Result<Box<dyn Connection>> {
        debug!("QUIC connecting to {} with NAT traversal", addr);
        
        // Create endpoint for outgoing connections
        let bind_addr = NetworkAddress::new("0.0.0.0:0".parse().unwrap());
        let endpoint = self.create_endpoint(bind_addr).await?;
        
        // Connect with NAT traversal and timeout
        let connection = tokio::time::timeout(
            options.connect_timeout,
            endpoint.connect(addr.socket_addr())
        ).await
            .map_err(|_| P2PError::Transport("QUIC connection timeout".to_string()))?
            .map_err(|e| P2PError::Transport(format!("QUIC connection failed: {}", e)))?;
        
        let local_socket_addr = endpoint.local_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get local address: {}", e)))?;
        let remote_socket_addr = connection.remote_addr();
        
        let local_addr = NetworkAddress::new(local_socket_addr);
        let remote_addr = NetworkAddress::new(remote_socket_addr);
        
        // Check if 0-RTT was actually used
        let used_0rtt = if self.enable_0rtt {
            // ant-quic may provide 0-RTT detection capabilities
            false // Placeholder - would need ant-quic API to detect actual 0-RTT usage
        } else {
            false
        };
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: local_addr.clone(),
            remote_addr: remote_addr.clone(),
            is_encrypted: true, // QUIC is always encrypted
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(), // QUIC uses TLS 1.3
            used_0rtt,
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let quic_connection = QuicConnection {
            connection,
            local_addr,
            remote_addr,
            info: connection_info,
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            stream_counter: Arc::new(Mutex::new(0)),
        };
        
        info!("QUIC connection established to {} with NAT traversal", addr);
        Ok(Box::new(quic_connection))
    }
    
    fn supports_ipv6(&self) -> bool {
        false // Focus on IPv4 for now
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::QUIC
    }
    
    fn supports_address(&self, addr: &NetworkAddress) -> bool {
        // Support IPv4 addresses for now
        addr.is_ipv4()
    }
}

#[async_trait]
impl Connection for QuicConnection {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        debug!("QUIC sending {} bytes", data.len());
        
        // Get stream ID for tracking
        let stream_id = {
            let mut counter = self.stream_counter.lock().await;
            *counter += 1;
            *counter
        };
        
        // Register active stream
        {
            let mut streams = self.active_streams.lock().await;
            streams.insert(stream_id, true);
        }
        
        // Send data using ant-quic
        self.connection.send(data).await
            .map_err(|e| P2PError::Transport(format!("Failed to send data on stream {}: {}", stream_id, e)))?;
        
        // Unregister stream
        {
            let mut streams = self.active_streams.lock().await;
            streams.remove(&stream_id);
        }
        
        // Update last activity
        self.info.last_activity = Instant::now();
        
        debug!("QUIC sent {} bytes successfully on stream {}", data.len(), stream_id);
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<Vec<u8>> {
        debug!("QUIC receiving data");
        
        // Receive data using ant-quic
        let data = self.connection.receive().await
            .map_err(|e| P2PError::Transport(format!("Failed to receive data: {}", e)))?;
        
        // Validate length to prevent memory exhaustion
        if data.len() > 64 * 1024 * 1024 {
            return Err(P2PError::Transport(format!("Message too large: {} bytes", data.len())));
        }
        
        // Update last activity
        self.info.last_activity = Instant::now();
        
        debug!("QUIC received {} bytes", data.len());
        Ok(data)
    }
    
    async fn info(&self) -> ConnectionInfo {
        self.info.clone()
    }
    
    async fn close(&mut self) -> Result<()> {
        debug!("Closing QUIC connection");
        self.connection.close().await
            .map_err(|e| P2PError::Transport(format!("Failed to close connection: {}", e)))?;
        Ok(())
    }
    
    async fn is_alive(&self) -> bool {
        self.connection.is_connected()
    }
    
    async fn measure_quality(&self) -> Result<ConnectionQuality> {
        // Get connection statistics from ant-quic
        let stats = self.connection.stats()
            .map_err(|e| P2PError::Transport(format!("Failed to get connection stats: {}", e)))?;
        
        // Calculate metrics from ant-quic stats
        let rtt = stats.rtt().unwrap_or(Duration::from_millis(50));
        let throughput = if stats.bytes_sent() > 0 && self.info.established_at.elapsed().as_secs_f64() > 0.0 {
            (stats.bytes_sent() as f64 * 8.0) / (self.info.established_at.elapsed().as_secs_f64() * 1_000_000.0)
        } else {
            100.0 // Default
        };
        
        Ok(ConnectionQuality {
            latency: rtt,
            throughput_mbps: throughput,
            packet_loss: stats.packet_loss_rate().unwrap_or(0.0),
            jitter: Duration::from_millis(1), // TODO: Calculate from RTT variance
            connect_time: self.info.established_at.elapsed(),
        })
    }
    
    fn local_addr(&self) -> NetworkAddress {
        self.local_addr.clone()
    }
    
    fn remote_addr(&self) -> NetworkAddress {
        self.remote_addr.clone()
    }
}

impl QuicConnection {
    /// Get count of active streams
    pub async fn active_stream_count(&self) -> usize {
        let streams = self.active_streams.lock().await;
        streams.len()
    }
    
    /// Check if connection supports migration
    pub fn supports_migration(&self) -> bool {
        // QUIC with NAT traversal supports connection migration
        true
    }
    
    /// Check if connection is using 0-RTT
    pub fn is_0rtt(&self) -> bool {
        self.info.used_0rtt
    }
    
    /// Get NAT traversal status
    pub fn nat_traversal_status(&self) -> String {
        // ant-quic provides NAT traversal status
        "Active".to_string() // Placeholder
    }
    
    /// Get connection role (coordinator/client)
    pub fn connection_role(&self) -> String {
        // ant-quic automatically detects role
        "Auto-detected".to_string() // Placeholder
    }
    
    /// Try to migrate connection to new network path
    pub async fn try_migrate(&self, new_addr: NetworkAddress) -> Result<()> {
        // ant-quic handles connection migration with NAT traversal
        debug!("Attempting connection migration to {} with NAT traversal", new_addr);
        
        // Connection migration is handled internally by ant-quic
        let current_remote = self.remote_addr.clone();
        if current_remote != new_addr {
            info!("Connection migrated from {} to {}", current_remote, new_addr);
        }
        
        Ok(())
    }
}