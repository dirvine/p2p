//! IPv6/IPv4 Tunneling Implementation
//!
//! This module provides comprehensive tunneling solutions for enabling IPv6 connectivity
//! over IPv4 networks. It implements multiple tunneling protocols to ensure universal
//! connectivity for the P2P Foundation networking stack.
//!
//! ## Supported Protocols
//!
//! - **6to4**: Automatic tunneling of IPv6 traffic over IPv4 networks
//! - **Teredo**: NAT traversal for IPv6 connectivity through NAT devices
//! - **6in4**: Configured tunneling for IPv6 over IPv4 with explicit endpoints
//!
//! ## Architecture
//!
//! The tunneling system uses a trait-based architecture that allows for:
//! - Protocol-agnostic tunnel management
//! - Automatic protocol selection based on network conditions
//! - Seamless integration with the transport layer
//! - Performance monitoring and failover capabilities

use crate::{P2PError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Tunneling protocol types supported by the P2P Foundation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TunnelProtocol {
    /// 6to4 automatic tunneling protocol (RFC 3056)
    SixToFour,
    /// Teredo tunneling protocol for NAT traversal (RFC 4380)
    Teredo,
    /// 6in4 static tunneling protocol (RFC 4213)
    SixInFour,
}

/// Configuration for tunneling protocols
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Protocol to use for tunneling
    pub protocol: TunnelProtocol,
    /// Local IPv4 address for tunnel endpoint
    pub local_ipv4: Option<Ipv4Addr>,
    /// Remote IPv4 address for tunnel endpoint (6in4 only)
    pub remote_ipv4: Option<Ipv4Addr>,
    /// IPv6 prefix to use for the tunnel
    pub ipv6_prefix: Option<Ipv6Addr>,
    /// Maximum transmission unit for tunnel packets
    pub mtu: u16,
    /// Keepalive interval for maintaining tunnel state
    pub keepalive_interval: Duration,
    /// Maximum time to wait for tunnel establishment
    pub establishment_timeout: Duration,
}

/// Statistics and performance metrics for tunnel connections
#[derive(Debug, Clone)]
pub struct TunnelMetrics {
    /// Total bytes sent through the tunnel
    pub bytes_sent: u64,
    /// Total bytes received through the tunnel
    pub bytes_received: u64,
    /// Number of packets successfully transmitted
    pub packets_sent: u64,
    /// Number of packets successfully received
    pub packets_received: u64,
    /// Number of packets dropped due to errors
    pub packets_dropped: u64,
    /// Current round-trip time
    pub rtt: Option<Duration>,
    /// Tunnel establishment time
    pub establishment_time: Duration,
    /// Last activity timestamp
    pub last_activity: Instant,
}

/// Current state of a tunnel connection
#[derive(Debug, Clone, PartialEq)]
pub enum TunnelState {
    /// Tunnel is not yet established
    Disconnected,
    /// Tunnel is in the process of being established
    Connecting,
    /// Tunnel is established and ready for use
    Connected,
    /// Tunnel has failed and cannot be used
    Failed(String),
    /// Tunnel is being torn down
    Disconnecting,
}

/// Core trait for all tunneling protocol implementations
#[async_trait]
pub trait Tunnel: Send + Sync {
    /// Get the protocol type for this tunnel
    fn protocol(&self) -> TunnelProtocol;
    
    /// Get the current configuration
    fn config(&self) -> &TunnelConfig;
    
    /// Get the current tunnel state
    async fn state(&self) -> TunnelState;
    
    /// Get tunnel performance metrics
    async fn metrics(&self) -> TunnelMetrics;
    
    /// Establish the tunnel connection
    async fn connect(&mut self) -> Result<()>;
    
    /// Close the tunnel connection
    async fn disconnect(&mut self) -> Result<()>;
    
    /// Check if the tunnel is currently active and usable
    async fn is_active(&self) -> bool;
    
    /// Encapsulate IPv6 packet for transmission over IPv4
    async fn encapsulate(&self, ipv6_packet: &[u8]) -> Result<Vec<u8>>;
    
    /// Decapsulate IPv4 packet to extract IPv6 content
    async fn decapsulate(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>>;
    
    /// Send a packet through the tunnel
    async fn send(&mut self, packet: &[u8]) -> Result<()>;
    
    /// Receive a packet from the tunnel
    async fn receive(&mut self) -> Result<Vec<u8>>;
    
    /// Perform periodic maintenance (keepalive, metrics update, etc.)
    async fn maintain(&mut self) -> Result<()>;
    
    /// Get the IPv6 address assigned to this tunnel
    async fn local_ipv6_addr(&self) -> Result<Ipv6Addr>;
    
    /// Get the IPv4 endpoint address for this tunnel
    async fn local_ipv4_addr(&self) -> Result<Ipv4Addr>;
    
    /// Test tunnel connectivity with a ping
    async fn ping(&mut self, timeout: Duration) -> Result<Duration>;
}

/// Manager for multiple tunnel connections with automatic failover
pub struct TunnelManager {
    /// Available tunnel implementations
    tunnels: RwLock<Vec<Box<dyn Tunnel>>>,
    /// Currently active tunnel
    active_tunnel: RwLock<Option<usize>>,
    /// Configuration for tunnel selection
    config: TunnelManagerConfig,
}

/// Configuration for the tunnel manager
#[derive(Debug, Clone)]
pub struct TunnelManagerConfig {
    /// Preferred protocol order for tunnel selection
    pub protocol_preference: Vec<TunnelProtocol>,
    /// How often to test tunnel connectivity
    pub health_check_interval: Duration,
    /// Timeout for tunnel health checks
    pub health_check_timeout: Duration,
    /// Whether to automatically failover to backup tunnels
    pub auto_failover: bool,
    /// Maximum number of concurrent tunnel attempts
    pub max_concurrent_attempts: usize,
}

/// Result of tunnel auto-selection process
#[derive(Debug, Clone)]
pub struct TunnelSelection {
    /// Selected protocol
    pub protocol: TunnelProtocol,
    /// Reason for selection
    pub reason: String,
    /// Time taken for selection process
    pub selection_time: Duration,
    /// Whether this was a fallback choice
    pub is_fallback: bool,
}

/// Capabilities of the current network environment for tunneling
#[derive(Debug, Clone)]
pub struct NetworkCapabilities {
    /// Whether IPv6 is natively available
    pub has_ipv6: bool,
    /// Whether IPv4 is available
    pub has_ipv4: bool,
    /// Whether the host is behind NAT
    pub behind_nat: bool,
    /// Detected public IPv4 address
    pub public_ipv4: Option<Ipv4Addr>,
    /// Available IPv6 addresses
    pub ipv6_addresses: Vec<Ipv6Addr>,
    /// Whether UPnP is available for port mapping
    pub has_upnp: bool,
    /// MTU of the primary network interface
    pub interface_mtu: u16,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            protocol: TunnelProtocol::SixToFour,
            local_ipv4: None,
            remote_ipv4: None,
            ipv6_prefix: None,
            mtu: 1280, // Minimum IPv6 MTU
            keepalive_interval: Duration::from_secs(30),
            establishment_timeout: Duration::from_secs(10),
        }
    }
}

impl Default for TunnelManagerConfig {
    fn default() -> Self {
        Self {
            protocol_preference: vec![
                TunnelProtocol::SixToFour,
                TunnelProtocol::Teredo,
                TunnelProtocol::SixInFour,
            ],
            health_check_interval: Duration::from_secs(60),
            health_check_timeout: Duration::from_secs(5),
            auto_failover: true,
            max_concurrent_attempts: 3,
        }
    }
}

impl Default for TunnelMetrics {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            packets_dropped: 0,
            rtt: None,
            establishment_time: Duration::ZERO,
            last_activity: Instant::now(),
        }
    }
}

impl TunnelManager {
    /// Create a new tunnel manager with default configuration
    pub fn new() -> Self {
        Self::with_config(TunnelManagerConfig::default())
    }
    
    /// Create a new tunnel manager with custom configuration
    pub fn with_config(config: TunnelManagerConfig) -> Self {
        Self {
            tunnels: RwLock::new(Vec::new()),
            active_tunnel: RwLock::new(None),
            config,
        }
    }
    
    /// Add a tunnel implementation to the manager
    pub async fn add_tunnel(&self, tunnel: Box<dyn Tunnel>) {
        let mut tunnels = self.tunnels.write().await;
        tunnels.push(tunnel);
    }
    
    /// Get the currently active tunnel
    pub async fn active_tunnel(&self) -> Option<TunnelProtocol> {
        let active_idx = self.active_tunnel.read().await;
        if let Some(idx) = *active_idx {
            let tunnels = self.tunnels.read().await;
            if let Some(tunnel) = tunnels.get(idx) {
                return Some(tunnel.protocol());
            }
        }
        None
    }
    
    /// Select the best available tunnel based on network capabilities
    pub async fn select_tunnel(&self, capabilities: &NetworkCapabilities) -> Option<TunnelSelection> {
        let start_time = Instant::now();
        
        // If IPv6 is natively available, no tunneling needed
        if capabilities.has_ipv6 && !capabilities.ipv6_addresses.is_empty() {
            return None;
        }
        
        let tunnels = self.tunnels.read().await;
        
        // Try protocols in preference order
        for preferred_protocol in &self.config.protocol_preference {
            for (idx, tunnel) in tunnels.iter().enumerate() {
                if tunnel.protocol() == *preferred_protocol {
                    if self.is_protocol_suitable(preferred_protocol, capabilities) {
                        let mut active = self.active_tunnel.write().await;
                        *active = Some(idx);
                        
                        return Some(TunnelSelection {
                            protocol: preferred_protocol.clone(),
                            reason: format!("Best available protocol for current network conditions"),
                            selection_time: start_time.elapsed(),
                            is_fallback: false,
                        });
                    }
                }
            }
        }
        
        // No suitable tunnel found
        None
    }
    
    /// Check if a protocol is suitable for the current network conditions
    fn is_protocol_suitable(&self, protocol: &TunnelProtocol, capabilities: &NetworkCapabilities) -> bool {
        match protocol {
            TunnelProtocol::SixToFour => {
                // 6to4 requires a public IPv4 address
                capabilities.has_ipv4 && capabilities.public_ipv4.is_some() && !capabilities.behind_nat
            }
            TunnelProtocol::Teredo => {
                // Teredo works behind NAT and with private IPv4
                capabilities.has_ipv4
            }
            TunnelProtocol::SixInFour => {
                // 6in4 requires explicit configuration
                capabilities.has_ipv4
            }
        }
    }
    
    /// Connect using the currently selected tunnel
    pub async fn connect(&self) -> Result<()> {
        let active_idx = {
            let active = self.active_tunnel.read().await;
            *active
        };
        
        if let Some(idx) = active_idx {
            let mut tunnels = self.tunnels.write().await;
            if let Some(tunnel) = tunnels.get_mut(idx) {
                tunnel.connect().await?;
                info!("Successfully connected using {} tunnel", 
                      format!("{:?}", tunnel.protocol()));
                return Ok(());
            }
        }
        
        Err(P2PError::Network("No active tunnel selected".to_string()).into())
    }
    
    /// Disconnect the currently active tunnel
    pub async fn disconnect(&self) -> Result<()> {
        let active_idx = {
            let active = self.active_tunnel.read().await;
            *active
        };
        
        if let Some(idx) = active_idx {
            let mut tunnels = self.tunnels.write().await;
            if let Some(tunnel) = tunnels.get_mut(idx) {
                tunnel.disconnect().await?;
                debug!("Disconnected {} tunnel", format!("{:?}", tunnel.protocol()));
            }
        }
        
        Ok(())
    }
    
    /// Send a packet through the active tunnel
    pub async fn send(&self, packet: &[u8]) -> Result<()> {
        let active_idx = {
            let active = self.active_tunnel.read().await;
            *active
        };
        
        if let Some(idx) = active_idx {
            let mut tunnels = self.tunnels.write().await;
            if let Some(tunnel) = tunnels.get_mut(idx) {
                return tunnel.send(packet).await;
            }
        }
        
        Err(P2PError::Network("No active tunnel for sending".to_string()).into())
    }
    
    /// Receive a packet from the active tunnel
    pub async fn receive(&self) -> Result<Vec<u8>> {
        let active_idx = {
            let active = self.active_tunnel.read().await;
            *active
        };
        
        if let Some(idx) = active_idx {
            let mut tunnels = self.tunnels.write().await;
            if let Some(tunnel) = tunnels.get_mut(idx) {
                return tunnel.receive().await;
            }
        }
        
        Err(P2PError::Network("No active tunnel for receiving".to_string()).into())
    }
    
    /// Get metrics for the currently active tunnel
    pub async fn metrics(&self) -> Option<TunnelMetrics> {
        let active_idx = {
            let active = self.active_tunnel.read().await;
            *active
        };
        
        if let Some(idx) = active_idx {
            let tunnels = self.tunnels.read().await;
            if let Some(tunnel) = tunnels.get(idx) {
                return Some(tunnel.metrics().await);
            }
        }
        
        None
    }
    
    /// Perform health checks on all tunnels
    pub async fn health_check(&self) -> Result<()> {
        let mut tunnels = self.tunnels.write().await;
        
        for tunnel in tunnels.iter_mut() {
            match tunnel.ping(self.config.health_check_timeout).await {
                Ok(rtt) => {
                    debug!("Health check passed for {} tunnel (RTT: {:?})", 
                           format!("{:?}", tunnel.protocol()), rtt);
                }
                Err(e) => {
                    warn!("Health check failed for {} tunnel: {}", 
                          format!("{:?}", tunnel.protocol()), e);
                    
                    if self.config.auto_failover {
                        // Try to reconnect or failover
                        if let Err(reconnect_err) = tunnel.connect().await {
                            warn!("Failed to reconnect {} tunnel: {}", 
                                  format!("{:?}", tunnel.protocol()), reconnect_err);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Perform maintenance on all tunnels
    pub async fn maintain(&self) -> Result<()> {
        let mut tunnels = self.tunnels.write().await;
        
        for tunnel in tunnels.iter_mut() {
            if let Err(e) = tunnel.maintain().await {
                warn!("Maintenance failed for {} tunnel: {}", 
                      format!("{:?}", tunnel.protocol()), e);
            }
        }
        
        Ok(())
    }
}

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect current network capabilities for tunnel selection
pub async fn detect_network_capabilities() -> Result<NetworkCapabilities> {
    debug!("Detecting network capabilities...");
    
    // This is a simplified implementation - in production, this would:
    // 1. Test for IPv6 connectivity
    // 2. Detect NAT presence
    // 3. Discover public IP addresses
    // 4. Test UPnP availability
    // 5. Measure interface MTU
    
    // For now, simulate basic detection
    let capabilities = NetworkCapabilities {
        has_ipv6: false, // Most networks don't have native IPv6 yet
        has_ipv4: true,  // Assume IPv4 is always available
        behind_nat: true, // Assume NAT in most cases
        public_ipv4: None, // Would be detected via STUN or similar
        ipv6_addresses: Vec::new(),
        has_upnp: false, // Would be tested
        interface_mtu: 1500, // Standard Ethernet MTU
    };
    
    info!("Network capabilities detected: IPv4={}, IPv6={}, NAT={}", 
          capabilities.has_ipv4, capabilities.has_ipv6, capabilities.behind_nat);
    
    Ok(capabilities)
}

// Tunneling protocol implementations
pub mod sixto4;
pub mod teredo;

pub use sixto4::SixToFourTunnel;
pub use teredo::TeredoTunnel;

/// Create a tunnel configuration for a specific protocol
pub fn create_tunnel_config(protocol: TunnelProtocol, capabilities: &NetworkCapabilities) -> TunnelConfig {
    let mut config = TunnelConfig::default();
    config.protocol = protocol.clone();
    
    match protocol {
        TunnelProtocol::SixToFour => {
            // 6to4 uses well-known prefix 2002::/16
            if let Some(ipv4) = capabilities.public_ipv4 {
                config.local_ipv4 = Some(ipv4);
                // Generate 6to4 IPv6 address: 2002:WWXX:YYZZ::/48
                let octets = ipv4.octets();
                let ipv6_bytes = [
                    0x20, 0x02, // 6to4 prefix
                    octets[0], octets[1], octets[2], octets[3], // IPv4 address
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01 // Interface ID
                ];
                config.ipv6_prefix = Some(Ipv6Addr::from(ipv6_bytes));
            }
        }
        TunnelProtocol::Teredo => {
            // Teredo uses prefix 2001::/32
            let teredo_prefix = Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 1);
            config.ipv6_prefix = Some(teredo_prefix);
            config.mtu = 1280; // Teredo MTU is typically lower
        }
        TunnelProtocol::SixInFour => {
            // 6in4 requires manual configuration
            config.mtu = 1480; // Account for IPv4 header overhead
        }
    }
    
    config
}

/// Create a tunnel instance for a specific protocol
pub fn create_tunnel(config: TunnelConfig) -> Result<Box<dyn Tunnel>> {
    match config.protocol {
        TunnelProtocol::SixToFour => {
            let tunnel = SixToFourTunnel::new(config)?;
            Ok(Box::new(tunnel))
        }
        TunnelProtocol::Teredo => {
            let tunnel = TeredoTunnel::new(config)?;
            Ok(Box::new(tunnel))
        }
        TunnelProtocol::SixInFour => {
            // TODO: Implement 6in4 tunnel
            Err(P2PError::Network("6in4 tunnel not yet implemented".to_string()).into())
        }
    }
}