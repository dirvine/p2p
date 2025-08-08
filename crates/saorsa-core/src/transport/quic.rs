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

//! QUIC Transport Implementation (DEPRECATED - using ant-quic instead)
//!
//! This module is deprecated in favor of ant-quic integration.
//! See ant_quic_adapter.rs for the new implementation.

// This module is temporarily disabled during ant-quic migration
// TODO: Remove this module completely once ant-quic integration is complete

/*
// This entire module is commented out during ant-quic migration
// All the quinn-based code below should be removed once migration is complete

use super::{Transport, Connection, TransportType, TransportOptions, ConnectionInfo, ConnectionQuality};
use crate::NetworkAddress;
use crate::error::{P2PError as P2PError, P2pResult as Result, TransportError};
use crate::identity::NodeIdentity;
use crate::validation::{ValidationContext, validate_message_size};
use async_trait::async_trait;
// use quinn::{Endpoint, ServerConfig, ClientConfig, VarInt}; // Removed - using ant-quic now
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

/// QUIC transport implementation using Quinn
pub struct QuicTransport {
    /// Local identity for authentication
    _identity: Option<Arc<NodeIdentity>>,
    /// Quinn endpoint
    endpoint: Arc<Mutex<Option<Endpoint>>>,
    /// Server configuration
    server_config: Option<ServerConfig>,
    /// Client configuration
    _client_config: ClientConfig,
    /// Bootstrap nodes
    _bootstrap_nodes: Vec<SocketAddr>,
    /// Whether 0-RTT is enabled
    _enable_0rtt: bool,
    /// Listen address
    _listen_addr: Option<NetworkAddress>,
}

/// QUIC connection implementation
pub struct QuicConnection {
    /// Remote peer ID
    _peer_id: String,
    /// Quinn connection
    connection: quinn::Connection,
    /// Local address
    local_addr: NetworkAddress,
    /// Remote address
    remote_addr: NetworkAddress,
    /// Connection establishment time
    established_at: Instant,
}

impl QuicTransport {
    /// Create a new QUIC transport
    pub fn new(options: TransportOptions) -> Result<Self> {
        // Generate self-signed certificate for now
        let (cert, key) = generate_self_signed_cert()?;

        // Configure server - always create server config
        let server_config = {
            let mut config = ServerConfig::with_single_cert(vec![cert.clone()], key.into())
                .map_err(|e| P2PError::Transport(TransportError::SetupFailed(format!("Failed to create server config: {e}").into())))?;

            let transport_config = Arc::get_mut(&mut config.transport)
                .ok_or_else(|| P2PError::Transport(TransportError::SetupFailed(
                    "Failed to get mutable reference to transport config".into()
                )))?;
            transport_config.max_concurrent_uni_streams(VarInt::from_u32(0));
            transport_config.max_idle_timeout(Some(VarInt::from_u32(60_000).into()));

            config
        };

        // Configure client
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert.clone())
            .map_err(|e| P2PError::Transport(TransportError::SetupFailed(format!("Failed to add root cert: {e}").into())))?;

        let client_config = ClientConfig::with_root_certificates(Arc::new(roots))
            .map_err(|e| P2PError::Transport(TransportError::SetupFailed(format!("Failed to create client config: {e}").into())))?;

        Ok(Self {
            _identity: None,
            endpoint: Arc::new(Mutex::new(None)),
            server_config: Some(server_config),
            _client_config: client_config,
            _bootstrap_nodes: Vec::new(), // Default to empty for now
            _enable_0rtt: options.enable_0rtt,
            _listen_addr: None,
        })
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn listen(&self, addr: NetworkAddress) -> Result<NetworkAddress> {
        let socket_addr = addr.socket_addr();

        let endpoint = match &self.server_config {
            Some(config) => Endpoint::server(config.clone(), socket_addr),
            None => Endpoint::client(socket_addr),
        }.map_err(|e| P2PError::Transport(TransportError::BindError(format!("Failed to bind to {socket_addr}: {e}").into())))?;

        let actual_addr = endpoint.local_addr()
            .map_err(|e| P2PError::Transport(TransportError::BindError(format!("Failed to get local address: {e}").into())))?;

        info!("QUIC transport listening on {}", actual_addr);
        *self.endpoint.lock().await = Some(endpoint);

        let listen_addr = NetworkAddress::new(actual_addr);
        Ok(listen_addr)
    }

    async fn accept(&self) -> Result<Box<dyn Connection>> {
        let endpoint = self.endpoint.lock().await
            .as_ref()
            .ok_or_else(|| P2PError::Transport(TransportError::NotListening))?
            .clone();

        let connecting = endpoint.accept().await
            .ok_or_else(|| P2PError::Transport(TransportError::NotListening))?;

        let connection = connecting.await
            .map_err(|e| P2PError::Transport(TransportError::AcceptFailed(e.to_string().into())))?;

        let local_addr = connection.local_ip()
            .ok_or_else(|| P2PError::Transport(TransportError::AcceptFailed("No local address".to_string().into())))?;

        let local_addr = NetworkAddress::new(SocketAddr::new(local_addr, 0));
        let remote_addr = NetworkAddress::new(connection.remote_address());

        info!("Accepted connection from {}", remote_addr);

        Ok(Box::new(QuicConnection {
            _peer_id: remote_addr.to_string(),
            connection,
            local_addr,
            remote_addr,
            established_at: Instant::now(),
        }))
    }

    async fn connect(&self, addr: NetworkAddress) -> Result<Box<dyn Connection>> {
        let endpoint = self.endpoint.lock().await
            .as_ref()
            .ok_or_else(|| P2PError::Transport(TransportError::NotInitialized))?
            .clone();

        let socket_addr = addr.socket_addr();
        debug!("Connecting to {}", socket_addr);

        // Use the server name from configuration or derive from address
        let server_name = if let Ok(config) = crate::config::Config::load() {
            config.transport.server_name
        } else {
            // Fallback to address-based server name
            match socket_addr {
                SocketAddr::V4(addr) => addr.ip().to_string(),
                SocketAddr::V6(addr) => format!("[{}]", addr.ip()),
            }
        };

        let connecting = endpoint.connect(socket_addr, &server_name)
            .map_err(|e| P2PError::Transport(TransportError::ConnectionFailed {
                addr: socket_addr,
                reason: e.to_string().into()
            }))?;

        let connection = connecting.await
            .map_err(|e| P2PError::Transport(TransportError::ConnectionFailed {
                addr: socket_addr,
                reason: e.to_string().into()
            }))?;

        let local_addr = connection.local_ip()
            .ok_or_else(|| P2PError::Transport(TransportError::ConnectionFailed {
                addr: socket_addr,
                reason: "No local address".to_string().into(),
            }))?;

        let local_addr = NetworkAddress::new(SocketAddr::new(local_addr, 0));
        let remote_addr = NetworkAddress::new(connection.remote_address());

        info!("Connected to {} via QUIC", socket_addr);

        Ok(Box::new(QuicConnection {
            _peer_id: socket_addr.to_string(),
            connection,
            local_addr,
            remote_addr,
            established_at: Instant::now(),
        }))
    }

    async fn connect_with_options(&self, addr: NetworkAddress, _options: TransportOptions) -> Result<Box<dyn Connection>> {
        // For now, just delegate to connect
        self.connect(addr).await
    }

    fn supports_ipv6(&self) -> bool {
        // Deprecated - we focus on IPv4
        false
    }

    fn transport_type(&self) -> TransportType {
        TransportType::QUIC
    }

    fn supports_address(&self, addr: &NetworkAddress) -> bool {
        // QUIC supports any valid socket address
        addr.socket_addr().port() > 0
    }
}

#[async_trait]
impl Connection for QuicConnection {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        // Validate message size before sending
        let ctx = ValidationContext::default();
        validate_message_size(data.len(), ctx.max_message_size)?;

        if data.is_empty() {
            warn!("Attempting to send empty message");
            return Ok(());
        }

        let mut stream = self.connection.open_uni().await
            .map_err(|e| P2PError::Transport(TransportError::StreamError(e.to_string().into())))?;

        stream.write_all(data).await
            .map_err(|e| P2PError::Transport(TransportError::StreamError(e.to_string().into())))?;

        stream.finish()
            .map_err(|e| P2PError::Transport(TransportError::StreamError(e.to_string().into())))?;

        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        let mut stream = self.connection.accept_uni().await
            .map_err(|e| P2PError::Transport(TransportError::StreamError(e.to_string().into())))?;

        let mut buf = Vec::new();
        let ctx = ValidationContext::default();

        // Read all data from the stream with size validation
        loop {
            match stream.read_chunk(usize::MAX, true).await {
                Ok(Some(chunk)) => {
                    // Check if adding this chunk would exceed max message size
                    let new_size = buf.len() + chunk.bytes.len();
                    validate_message_size(new_size, ctx.max_message_size)?;

                    buf.extend_from_slice(&chunk.bytes);
                },
                Ok(None) => break, // End of stream
                Err(e) => return Err(P2PError::Transport(TransportError::StreamError(e.to_string().into()))),
            }
        }

        // Final validation of complete message
        validate_message_size(buf.len(), ctx.max_message_size)?;

        Ok(buf)
    }

    async fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: self.local_addr.clone(),
            remote_addr: self.remote_addr.clone(),
            is_encrypted: true,
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(),
            used_0rtt: false,
            established_at: self.established_at,
            last_activity: Instant::now(),
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.connection.close(VarInt::from_u32(0), b"close");
        Ok(())
    }

    async fn is_alive(&self) -> bool {
        // Check if connection is still open by attempting to open a bidirectional stream
        // If the connection is closed, this will fail immediately
        tokio::select! {
            result = self.connection.open_bi() => result.is_ok(),
            _ = tokio::time::sleep(Duration::from_millis(1)) => true,
        }
    }

    fn local_addr(&self) -> NetworkAddress {
        self.local_addr.clone()
    }

    fn remote_addr(&self) -> NetworkAddress {
        self.remote_addr.clone()
    }

    async fn measure_quality(&self) -> Result<ConnectionQuality> {
        // TODO: Implement actual quality metrics
        Ok(ConnectionQuality {
            latency: Duration::from_millis(10),
            throughput_mbps: 100.0,
            packet_loss: 0.0,
            jitter: Duration::from_millis(1),
            connect_time: self.established_at.elapsed(),
        })
    }
}

/// Generate a self-signed certificate for testing
fn generate_self_signed_cert() -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>)> {
    // For now, use a dummy certificate
    // In production, this should use proper certificate generation
    let cert = CertificateDer::from(vec![]);
    let key = PrivatePkcs8KeyDer::from(vec![]);
    Ok((cert, key))
}
*/
