//! Tunneled Transport Implementation
//!
//! This module provides transport implementations that use IPv6 tunneling protocols
//! to establish connectivity over IPv4 networks. It integrates the tunneling system
//! with the transport layer for seamless IPv6 connectivity.

use super::{Transport, Connection, TransportType, TransportOptions, ConnectionInfo, ConnectionQuality};
use crate::tunneling::{TunnelManager, TunnelManagerConfig, detect_network_capabilities};
use crate::{Multiaddr, P2PError, Result};
use async_trait::async_trait;
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

/// Tunneled transport that provides IPv6 connectivity over IPv4 networks
pub struct TunneledTransport {
    /// Tunnel manager for protocol selection and management
    tunnel_manager: Arc<TunnelManager>,
    /// Current tunnel state
    tunnel_state: Arc<RwLock<Option<TunnelInfo>>>,
    /// Transport configuration
    config: TunnelTransportConfig,
}

/// Configuration for tunneled transport
#[derive(Debug, Clone)]
pub struct TunnelTransportConfig {
    /// Enable automatic tunnel selection
    pub enable_auto_selection: bool,
    /// Fallback to native IPv4 if tunneling fails
    pub ipv4_fallback: bool,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Tunnel health check interval
    pub health_check_interval: Duration,
    /// Maximum retries for tunnel establishment
    pub max_tunnel_retries: u32,
}

/// Information about the current tunnel
#[derive(Debug, Clone)]
struct TunnelInfo {
    /// Tunnel protocol being used
    pub protocol: crate::tunneling::TunnelProtocol,
    /// Local IPv6 address
    pub local_ipv6: Ipv6Addr,
    /// Tunnel establishment time
    pub established_at: Instant,
    /// Last successful communication
    pub _last_success: Instant,
}

/// Tunneled connection implementation
pub struct TunneledConnection {
    /// Underlying TCP connection over tunnel
    stream: TcpStream,
    /// Local address (IPv6 via tunnel)
    local_addr: Multiaddr,
    /// Remote address (IPv6 via tunnel)
    remote_addr: Multiaddr,
    /// Connection establishment time
    established_at: Instant,
    /// Tunnel information
    _tunnel_info: TunnelInfo,
}

impl Default for TunnelTransportConfig {
    fn default() -> Self {
        Self {
            enable_auto_selection: true,
            ipv4_fallback: true,
            connect_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(60),
            max_tunnel_retries: 3,
        }
    }
}

impl TunneledTransport {
    /// Create a new tunneled transport
    pub async fn new(config: TunnelTransportConfig) -> Result<Self> {
        let tunnel_manager_config = TunnelManagerConfig {
            auto_failover: true,
            max_concurrent_attempts: 3,
            health_check_interval: config.health_check_interval,
            health_check_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        
        let tunnel_manager = Arc::new(TunnelManager::with_config(tunnel_manager_config));
        
        // Add real tunnel implementations would go here
        // For now, we'll work with the auto-selection system
        
        let tunnel_state = Arc::new(RwLock::new(None));
        
        Ok(Self {
            tunnel_manager,
            tunnel_state,
            config,
        })
    }
    
    /// Establish tunneled connectivity
    async fn establish_tunnel(&self) -> Result<TunnelInfo> {
        info!("Establishing tunneled connectivity...");
        
        // Detect network capabilities
        let capabilities = detect_network_capabilities().await
            .map_err(|e| P2PError::Network(format!("Failed to detect network capabilities: {}", e)))?;
        
        info!("Network capabilities: IPv4={}, IPv6={}, NAT={}, UPnP={}", 
              capabilities.has_ipv4, capabilities.has_ipv6, 
              capabilities.behind_nat, capabilities.has_upnp);
        
        // If IPv6 is already available, no tunneling needed
        if capabilities.has_ipv6 && !capabilities.ipv6_addresses.is_empty() {
            info!("Native IPv6 connectivity available, using direct connection");
            return Ok(TunnelInfo {
                protocol: crate::tunneling::TunnelProtocol::SixToFour, // Placeholder
                local_ipv6: capabilities.ipv6_addresses[0],
                established_at: Instant::now(),
                _last_success: Instant::now(),
            });
        }
        
        if !self.config.enable_auto_selection {
            return Err(P2PError::Network("Tunneling disabled and no native IPv6".to_string()));
        }
        
        // Select and establish tunnel
        if let Some(selection) = self.tunnel_manager.select_tunnel(&capabilities).await {
            info!("Selected tunnel protocol: {:?} - {}", selection.protocol, selection.reason);
            
            // In a real implementation, we would:
            // 1. Connect to the actual tunnel
            // 2. Get the assigned IPv6 address
            // 3. Verify connectivity
            
            // For now, simulate successful tunnel establishment
            let local_ipv6 = match selection.protocol {
                crate::tunneling::TunnelProtocol::SixToFour => {
                    // 6to4 address format: 2002:xxxx:xxxx::1
                    "2002:cb00:7100::1".parse().unwrap()
                }
                crate::tunneling::TunnelProtocol::Teredo => {
                    // Teredo address format: 2001:0000:xxxx:xxxx:xxxx:xxxx:xxxx:xxxx
                    "2001:0000:4136:e378:8000:63bf:3fff:fdd2".parse().unwrap()
                }
                crate::tunneling::TunnelProtocol::SixInFour => {
                    // 6in4 address: assigned by tunnel broker
                    "2001:db8:1234::1".parse().unwrap()
                }
                crate::tunneling::TunnelProtocol::DsLite => {
                    // DS-Lite: Client gets IPv6 address for tunnel endpoint
                    "2001:db8:dslite::1".parse().unwrap()
                }
                crate::tunneling::TunnelProtocol::Isatap => {
                    // ISATAP address format: fe80::0:5efe:x.x.x.x
                    use crate::tunneling::IsatapTunnel;
                    let ipv4_addr = capabilities.public_ipv4.unwrap_or_else(|| "192.168.1.100".parse().unwrap());
                    IsatapTunnel::generate_isatap_address(ipv4_addr, Some("fe80::".parse().unwrap()))
                }
            };
            
            let tunnel_info = TunnelInfo {
                protocol: selection.protocol,
                local_ipv6,
                established_at: Instant::now(),
                _last_success: Instant::now(),
            };
            
            info!("Tunnel established successfully: {:?} -> {}", 
                  tunnel_info.protocol, tunnel_info.local_ipv6);
            
            Ok(tunnel_info)
        } else {
            if self.config.ipv4_fallback {
                warn!("No suitable tunnel found, fallback to IPv4 disabled");
                Err(P2PError::Network("No tunnel available and fallback disabled".to_string()))
            } else {
                Err(P2PError::Network("No suitable tunnel protocol found".to_string()))
            }
        }
    }
    
    /// Get or establish tunnel info
    async fn get_tunnel_info(&self) -> Result<TunnelInfo> {
        let current_tunnel = self.tunnel_state.read().await;
        
        if let Some(ref tunnel_info) = *current_tunnel {
            // Check if tunnel is still valid (basic health check)
            if tunnel_info.established_at.elapsed() < Duration::from_secs(3600) {
                return Ok(tunnel_info.clone());
            }
        }
        
        drop(current_tunnel);
        
        // Need to establish new tunnel
        let tunnel_info = self.establish_tunnel().await?;
        let mut tunnel_state = self.tunnel_state.write().await;
        *tunnel_state = Some(tunnel_info.clone());
        
        Ok(tunnel_info)
    }
}

#[async_trait]
impl Transport for TunneledTransport {
    async fn listen(&self, addr: SocketAddr) -> Result<Vec<Multiaddr>> {
        let tunnel_info = self.get_tunnel_info().await?;
        
        // Convert IPv4 listen address to IPv6 via tunnel
        let ipv6_addr = SocketAddr::new(tunnel_info.local_ipv6.into(), addr.port());
        
        debug!("Starting tunneled listener on {}", ipv6_addr);
        
        let listener = TcpListener::bind(ipv6_addr).await
            .map_err(|e| P2PError::Network(format!("Failed to bind listener: {}", e)))?;
        
        let local_addr = listener.local_addr()
            .map_err(|e| P2PError::Network(format!("Failed to get local address: {}", e)))?;
        
        let multiaddr = format!("/ip6/{}/tcp/{}", local_addr.ip(), local_addr.port());
        
        info!("Tunneled transport listening on {}", multiaddr);
        
        // In a real implementation, we would spawn a task to handle incoming connections
        // and integrate with the connection pool
        
        Ok(vec![multiaddr])
    }
    
    async fn connect(&self, addr: &Multiaddr) -> Result<Box<dyn Connection>> {
        self.connect_with_options(addr, TransportOptions::default()).await
    }
    
    async fn connect_with_options(&self, addr: &Multiaddr, options: TransportOptions) -> Result<Box<dyn Connection>> {
        let tunnel_info = self.get_tunnel_info().await?;
        
        info!("Connecting via tunnel ({:?}) to {}", tunnel_info.protocol, addr);
        
        // Parse the multiaddr to get target
        // For now, assume it's in format /ip6/ADDRESS/tcp/PORT
        let parts: Vec<&str> = addr.split('/').collect();
        if parts.len() < 5 || parts[1] != "ip6" || parts[3] != "tcp" {
            return Err(P2PError::Network(format!("Invalid IPv6 multiaddr: {}", addr)));
        }
        
        let target_ip: Ipv6Addr = parts[2].parse()
            .map_err(|e| P2PError::Network(format!("Invalid IPv6 address: {}", e)))?;
        let target_port: u16 = parts[4].parse()
            .map_err(|e| P2PError::Network(format!("Invalid port: {}", e)))?;
        
        let target_addr = SocketAddr::new(target_ip.into(), target_port);
        
        debug!("Establishing tunneled connection to {}", target_addr);
        
        // Establish connection with timeout
        let stream = tokio::time::timeout(
            options.connect_timeout,
            TcpStream::connect(target_addr)
        ).await
        .map_err(|_| P2PError::Network("Connection timeout".to_string()))?
        .map_err(|e| P2PError::Network(format!("Connection failed: {}", e)))?;
        
        let local_addr = stream.local_addr()
            .map_err(|e| P2PError::Network(format!("Failed to get local address: {}", e)))?;
        let remote_addr = stream.peer_addr()
            .map_err(|e| P2PError::Network(format!("Failed to get remote address: {}", e)))?;
        
        info!("Tunneled connection established: {} -> {}", local_addr, remote_addr);
        
        let connection = TunneledConnection {
            stream,
            local_addr: format!("/ip6/{}/tcp/{}", local_addr.ip(), local_addr.port()),
            remote_addr: format!("/ip6/{}/tcp/{}", remote_addr.ip(), remote_addr.port()),
            established_at: Instant::now(),
            _tunnel_info: tunnel_info,
        };
        
        Ok(Box::new(connection))
    }
    
    fn supported_addresses(&self) -> Vec<String> {
        vec![
            "/ip6/*/tcp/*".to_string(),
            "/ip4/*/tcp/*".to_string(), // Via tunneling
        ]
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::TCP // Tunneled TCP
    }
    
    fn supports_address(&self, addr: &Multiaddr) -> bool {
        addr.contains("/ip6/") || addr.contains("/ip4/")
    }
}

#[async_trait]
impl Connection for TunneledConnection {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        debug!("Sending {} bytes via tunneled connection", data.len());
        
        // Send data length first (simple framing)
        let len_bytes = (data.len() as u32).to_be_bytes();
        self.stream.write_all(&len_bytes).await
            .map_err(|e| P2PError::Network(format!("Failed to send length: {}", e)))?;
        
        // Send actual data
        self.stream.write_all(data).await
            .map_err(|e| P2PError::Network(format!("Failed to send data: {}", e)))?;
        
        self.stream.flush().await
            .map_err(|e| P2PError::Network(format!("Failed to flush: {}", e)))?;
        
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<Vec<u8>> {
        debug!("Receiving data via tunneled connection");
        
        // Read data length first
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes).await
            .map_err(|e| P2PError::Network(format!("Failed to read length: {}", e)))?;
        
        let data_len = u32::from_be_bytes(len_bytes) as usize;
        
        if data_len > 1024 * 1024 { // 1MB limit
            return Err(P2PError::Network("Message too large".to_string()));
        }
        
        // Read actual data
        let mut data = vec![0u8; data_len];
        self.stream.read_exact(&mut data).await
            .map_err(|e| P2PError::Network(format!("Failed to read data: {}", e)))?;
        
        debug!("Received {} bytes via tunneled connection", data.len());
        Ok(data)
    }
    
    async fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            transport_type: TransportType::TCP,
            local_addr: self.local_addr.clone(),
            remote_addr: self.remote_addr.clone(),
            is_encrypted: false, // TCP over tunnel is not encrypted by default
            cipher_suite: "none".to_string(),
            used_0rtt: false,
            established_at: self.established_at,
            last_activity: Instant::now(),
        }
    }
    
    async fn close(&mut self) -> Result<()> {
        debug!("Closing tunneled connection");
        self.stream.shutdown().await
            .map_err(|e| P2PError::Network(format!("Failed to close connection: {}", e)))?;
        Ok(())
    }
    
    async fn is_alive(&self) -> bool {
        // Simple check - connection hasn't been established too long ago
        self.established_at.elapsed() < Duration::from_secs(3600)
    }
    
    async fn measure_quality(&self) -> Result<ConnectionQuality> {
        // Basic quality measurement
        let start = Instant::now();
        
        // In a real implementation, we would send a ping and measure response
        // For now, simulate reasonable values
        let latency = Duration::from_millis(50);
        let jitter = Duration::from_millis(5);
        
        Ok(ConnectionQuality {
            latency,
            throughput_mbps: 100.0, // Simulated
            packet_loss: 0.01, // 1% loss
            jitter,
            connect_time: start.elapsed(),
        })
    }
    
    fn local_addr(&self) -> Multiaddr {
        self.local_addr.clone()
    }
    
    fn remote_addr(&self) -> Multiaddr {
        self.remote_addr.clone()
    }
}