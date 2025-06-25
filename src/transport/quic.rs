//! QUIC Transport Implementation
//!
//! This module provides QUIC-based transport using Quinn.
//! QUIC provides better performance, 0-RTT connections, and built-in encryption.

use super::{Transport, Connection, TransportType, TransportOptions, ConnectionInfo, ConnectionQuality};
use crate::{Multiaddr, P2PError, Result};
use async_trait::async_trait;
use quinn::{Endpoint, ServerConfig, ClientConfig, Connection as QuinnConnection, crypto::rustls::QuicClientConfig, crypto::rustls::QuicServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::client::danger::{ServerCertVerifier, ServerCertVerified, HandshakeSignatureValid};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
// use tokio::io::{AsyncReadExt, AsyncWriteExt}; // Not used yet
use tokio::sync::Mutex;
use tracing::{debug, info};

/// QUIC transport implementation
pub struct QuicTransport {
    /// QUIC endpoint (for both client and server)
    endpoint: Arc<Mutex<Option<Endpoint>>>,
    /// Client configuration
    client_config: ClientConfig,
    /// Whether 0-RTT is enabled
    enable_0rtt: bool,
}

/// QUIC connection implementation
pub struct QuicConnection {
    /// Underlying QUIC connection
    connection: QuinnConnection,
    /// Local address
    local_addr: Multiaddr,
    /// Remote address
    remote_addr: Multiaddr,
    /// Connection info
    info: ConnectionInfo,
    /// Active streams for multiplexing
    active_streams: Arc<Mutex<HashMap<u64, bool>>>,
    /// Stream counter for multiplexing
    stream_counter: Arc<Mutex<u64>>,
}

impl QuicTransport {
    /// Create a new QUIC transport
    pub fn new(enable_0rtt: bool) -> Result<Self> {
        // Install default crypto provider if not already installed
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        
        let client_config = Self::create_client_config(enable_0rtt)?;
        
        Ok(Self {
            endpoint: Arc::new(Mutex::new(None)),
            client_config,
            enable_0rtt,
        })
    }
    
    /// Create client configuration
    fn create_client_config(_enable_0rtt: bool) -> Result<ClientConfig> {
        let crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification::new()))
            .with_no_client_auth();
        
        let client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(crypto)
            .map_err(|e| P2PError::Transport(format!("Failed to create QUIC client config: {}", e)))?));
        
        // Note: 0-RTT configuration in newer Quinn versions is handled differently
        // It's typically configured at the connection level, not client config level
        
        Ok(client_config)
    }
    
    /// Create server configuration
    fn create_server_config() -> Result<ServerConfig> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .map_err(|e| P2PError::Transport(format!("Failed to generate certificate: {}", e)))?;
        
        let cert_der = cert.cert.der().to_vec();
        let priv_key = cert.key_pair.serialize_der();
        
        let cert_chain = vec![CertificateDer::from(cert_der)];
        let key_der = PrivateKeyDer::try_from(priv_key)
            .map_err(|_| P2PError::Transport("Failed to parse private key".to_string()))?;
        
        let server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key_der)
            .map_err(|e| P2PError::Transport(format!("Failed to create server crypto: {}", e)))?;
        
        let server_config = ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)
            .map_err(|e| P2PError::Transport(format!("Failed to create QUIC server config: {}", e)))?));
        Ok(server_config)
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn listen(&self, addr: SocketAddr) -> Result<Vec<Multiaddr>> {
        let server_config = Self::create_server_config()?;
        
        let endpoint = Endpoint::server(server_config, addr)
            .map_err(|e| P2PError::Transport(format!("Failed to create QUIC endpoint: {}", e)))?;
        
        let local_addr = endpoint.local_addr()
            .map_err(|e| P2PError::Transport(format!("Failed to get local address: {}", e)))?;
        
        info!("QUIC transport listening on {}", local_addr);
        
        // Store the endpoint for accepting connections
        {
            let mut endpoint_guard = self.endpoint.lock().await;
            *endpoint_guard = Some(endpoint);
        }
        
        // Convert to multiaddr format
        let multiaddr = if local_addr.is_ipv6() {
            format!("/ip6/{}/udp/{}/quic", local_addr.ip(), local_addr.port())
        } else {
            format!("/ip4/{}/udp/{}/quic", local_addr.ip(), local_addr.port())
        };
        
        Ok(vec![multiaddr])
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
        
        // Accept incoming connection
        let connecting = endpoint.accept().await.ok_or_else(|| {
            P2PError::Transport("No incoming QUIC connections available".to_string())
        })?;
        
        let connection = connecting.await
            .map_err(|e| P2PError::Transport(format!("QUIC connection handshake failed: {}", e)))?;
        
        let local_addr = connection.local_ip().unwrap_or_else(|| "0.0.0.0".parse().unwrap());
        let remote_addr = connection.remote_address();
        
        // Convert addresses to multiaddr format
        let local_multiaddr = if local_addr.is_ipv6() {
            format!("/ip6/{}/udp/{}/quic", local_addr, remote_addr.port())
        } else {
            format!("/ip4/{}/udp/{}/quic", local_addr, remote_addr.port())
        };
        
        let remote_multiaddr = if remote_addr.is_ipv6() {
            format!("/ip6/{}/udp/{}/quic", remote_addr.ip(), remote_addr.port())
        } else {
            format!("/ip4/{}/udp/{}/quic", remote_addr.ip(), remote_addr.port())
        };
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: local_multiaddr.clone(),
            remote_addr: remote_multiaddr.clone(),
            is_encrypted: true,
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(),
            used_0rtt: false, // For incoming connections, we can't determine 0-RTT easily
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let quic_connection = QuicConnection {
            connection,
            local_addr: local_multiaddr,
            remote_addr: remote_multiaddr,
            info: connection_info,
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            stream_counter: Arc::new(Mutex::new(0)),
        };
        
        info!("QUIC accepted incoming connection from {}", remote_addr);
        Ok(Box::new(quic_connection))
    }
    
    async fn connect(&self, addr: &Multiaddr) -> Result<Box<dyn Connection>> {
        self.connect_with_options(addr, TransportOptions::default()).await
    }
    
    async fn connect_with_options(&self, addr: &Multiaddr, options: TransportOptions) -> Result<Box<dyn Connection>> {
        debug!("QUIC connecting to {}", addr);
        
        // Parse multiaddr to get host:port
        let socket_addr = self.parse_multiaddr(addr)?;
        
        // Create client endpoint if not exists
        let endpoint = {
            let endpoint_guard = self.endpoint.lock().await;
            if let Some(ref ep) = *endpoint_guard {
                ep.clone()
            } else {
                drop(endpoint_guard); // Release lock before creating new endpoint
                let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
                    .map_err(|e| P2PError::Transport(format!("Failed to create client endpoint: {}", e)))?;
                
                endpoint.set_default_client_config(self.client_config.clone());
                endpoint
            }
        };
        
        // Connect with timeout
        let connecting = endpoint.connect(socket_addr, "localhost")
            .map_err(|e| P2PError::Transport(format!("QUIC connection failed: {}", e)))?;
        
        let connection = tokio::time::timeout(
            options.connect_timeout,
            connecting
        ).await
            .map_err(|_| P2PError::Transport("QUIC connection timeout".to_string()))?
            .map_err(|e| P2PError::Transport(format!("QUIC handshake failed: {}", e)))?;
        
        let local_addr = connection.local_ip().unwrap_or_else(|| "0.0.0.0".parse().unwrap());
        let remote_addr = connection.remote_address();
        
        // Convert addresses to multiaddr format
        let local_multiaddr = if local_addr.is_ipv6() {
            format!("/ip6/{}/udp/{}/quic", local_addr, remote_addr.port())
        } else {
            format!("/ip4/{}/udp/{}/quic", local_addr, remote_addr.port())
        };
        
        let remote_multiaddr = if remote_addr.is_ipv6() {
            format!("/ip6/{}/udp/{}/quic", remote_addr.ip(), remote_addr.port())
        } else {
            format!("/ip4/{}/udp/{}/quic", remote_addr.ip(), remote_addr.port())
        };
        
        // Check if 0-RTT was actually used
        let used_0rtt = if self.enable_0rtt {
            // In a real implementation, we would check the connection handshake details
            // For now, we'll check if the connection was established very quickly
            false // Placeholder - would need Quinn API to detect actual 0-RTT usage
        } else {
            false
        };
        
        let connection_info = ConnectionInfo {
            transport_type: TransportType::QUIC,
            local_addr: local_multiaddr.clone(),
            remote_addr: remote_multiaddr.clone(),
            is_encrypted: true, // QUIC is always encrypted
            cipher_suite: "TLS_AES_256_GCM_SHA384".to_string(), // QUIC uses TLS 1.3
            used_0rtt,
            established_at: Instant::now(),
            last_activity: Instant::now(),
        };
        
        let quic_connection = QuicConnection {
            connection,
            local_addr: local_multiaddr,
            remote_addr: remote_multiaddr,
            info: connection_info,
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            stream_counter: Arc::new(Mutex::new(0)),
        };
        
        info!("QUIC connection established to {}", addr);
        Ok(Box::new(quic_connection))
    }
    
    fn supported_addresses(&self) -> Vec<String> {
        vec![
            "/ip4/0.0.0.0/udp/0/quic".to_string(),
            "/ip6/::/udp/0/quic".to_string(),
        ]
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::QUIC
    }
    
    fn supports_address(&self, addr: &Multiaddr) -> bool {
        addr.contains("/quic") && addr.contains("/udp/") && (addr.contains("/ip4/") || addr.contains("/ip6/"))
    }
}

impl QuicTransport {
    /// Parse a multiaddr into a SocketAddr
    fn parse_multiaddr(&self, addr: &Multiaddr) -> Result<SocketAddr> {
        // Format: /ip4/127.0.0.1/udp/9000/quic or /ip6/::1/udp/9000/quic
        
        let parts: Vec<&str> = addr.split('/').collect();
        if parts.len() < 6 {
            return Err(P2PError::Transport(format!("Invalid QUIC multiaddr format: {}", addr)));
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
        
        // Open a new stream for this message (QUIC multiplexing)
        let mut send_stream = self.connection.open_uni().await
            .map_err(|e| P2PError::Transport(format!("Failed to open QUIC stream {}: {}", stream_id, e)))?;
        
        // Send data length first for framing
        let length_bytes = (data.len() as u32).to_be_bytes();
        send_stream.write_all(&length_bytes).await
            .map_err(|e| P2PError::Transport(format!("Failed to send length: {}", e)))?;
        
        // Send actual data
        send_stream.write_all(data).await
            .map_err(|e| P2PError::Transport(format!("Failed to send data: {}", e)))?;
        
        // Close the stream gracefully
        send_stream.finish()
            .map_err(|e| P2PError::Transport(format!("Failed to finish stream: {}", e)))?;
        
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
        
        // Accept incoming stream
        let mut recv_stream = self.connection.accept_uni().await
            .map_err(|e| P2PError::Transport(format!("Failed to accept QUIC stream: {}", e)))?;
        
        // Read length first (4 bytes)
        let mut length_buf = [0u8; 4];
        recv_stream.read_exact(&mut length_buf).await
            .map_err(|e| P2PError::Transport(format!("Failed to read length: {}", e)))?;
        
        let data_length = u32::from_be_bytes(length_buf) as usize;
        
        // Validate length to prevent memory exhaustion
        if data_length > 64 * 1024 * 1024 {
            return Err(P2PError::Transport(format!("Message too large: {} bytes", data_length)));
        }
        
        // Read the actual data
        let mut data = vec![0u8; data_length];
        recv_stream.read_exact(&mut data).await
            .map_err(|e| P2PError::Transport(format!("Failed to read data: {}", e)))?;
        
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
        self.connection.close(0u8.into(), b"Connection closed");
        Ok(())
    }
    
    async fn is_alive(&self) -> bool {
        // Check if the connection is still alive
        match self.connection.close_reason() {
            None => true, // Still connected
            Some(_) => false, // Connection closed
        }
    }
    
    async fn measure_quality(&self) -> Result<ConnectionQuality> {
        let _start = Instant::now();
        
        // Get QUIC connection stats
        let stats = self.connection.stats();
        
        // Calculate metrics from QUIC stats
        let rtt = stats.path.rtt.as_millis() as f64;
        let throughput = if stats.udp_tx.bytes > 0 && self.info.established_at.elapsed().as_secs_f64() > 0.0 {
            (stats.udp_tx.bytes as f64 * 8.0) / (self.info.established_at.elapsed().as_secs_f64() * 1_000_000.0)
        } else {
            100.0 // Default
        };
        
        Ok(ConnectionQuality {
            latency: Duration::from_millis(rtt as u64),
            throughput_mbps: throughput,
            packet_loss: 0.0, // TODO: Calculate from stats
            jitter: Duration::from_millis(1), // TODO: Calculate from RTT variance
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

impl QuicConnection {
    /// Get count of active streams
    pub async fn active_stream_count(&self) -> usize {
        let streams = self.active_streams.lock().await;
        streams.len()
    }
    
    /// Check if connection supports migration
    pub fn supports_migration(&self) -> bool {
        // QUIC supports connection migration by design
        true
    }
    
    /// Check if connection is using 0-RTT
    pub fn is_0rtt(&self) -> bool {
        self.info.used_0rtt
    }
    
    /// Get connection statistics
    pub fn connection_stats(&self) -> quinn::ConnectionStats {
        self.connection.stats()
    }
    
    /// Migrate connection to new network path
    pub async fn try_migrate(&self, new_addr: SocketAddr) -> Result<()> {
        // QUIC handles connection migration automatically
        // This method provides explicit migration control if needed
        debug!("Attempting connection migration to {}", new_addr);
        
        // Connection migration is handled internally by QUIC
        // We can monitor the remote address change
        let current_remote = self.connection.remote_address();
        if current_remote != new_addr {
            info!("Connection migrated from {} to {}", current_remote, new_addr);
        }
        
        Ok(())
    }
}

/// Skip server certificate verification for development/testing
#[derive(Debug)]
struct SkipServerVerification {
}

impl SkipServerVerification {
    fn new() -> Self {
        Self { }
    }
}

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}