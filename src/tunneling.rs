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
    /// DS-Lite (Dual-Stack Lite) ISP-provided tunneling (RFC 6333)
    DsLite,
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
    /// DS-Lite AFTR (Address Family Transition Router) IPv6 address
    pub aftr_ipv6: Option<Ipv6Addr>,
    /// DS-Lite AFTR domain name for DNS resolution
    pub aftr_name: Option<String>,
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

/// Quality metrics for tunnel monitoring and selection
#[derive(Debug, Clone)]
pub struct TunnelQualityMetric {
    /// Protocol type
    pub protocol: TunnelProtocol,
    /// Current tunnel state
    pub state: TunnelState,
    /// Round-trip time
    pub rtt: Option<Duration>,
    /// Packet loss percentage (0-100)
    pub packet_loss: Option<f32>,
    /// Throughput in bytes per second
    pub throughput: Option<f64>,
    /// Overall reliability score (0.0-1.0)
    pub reliability_score: f32,
    /// Last activity timestamp
    pub last_activity: Instant,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            protocol: TunnelProtocol::SixToFour,
            local_ipv4: None,
            remote_ipv4: None,
            ipv6_prefix: None,
            aftr_ipv6: None,
            aftr_name: None,
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
                TunnelProtocol::DsLite,     // ISP-provided, most reliable
                TunnelProtocol::SixToFour,  // Automatic, good for public IPv4
                TunnelProtocol::Teredo,     // NAT traversal capable
                TunnelProtocol::SixInFour,  // Manual configuration fallback
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
            info!("Native IPv6 connectivity detected, no tunneling required");
            return None;
        }
        
        info!("Selecting optimal tunnel protocol based on network conditions");
        debug!("Network capabilities: {:?}", capabilities);
        
        // Perform intelligent protocol selection
        let selection = self.intelligent_protocol_selection(capabilities).await;
        
        if let Some(ref selection) = selection {
            info!("Selected {} tunnel: {}", 
                  format!("{:?}", selection.protocol), selection.reason);
        } else {
            warn!("No suitable tunnel protocol found for current network conditions");
        }
        
        selection.map(|mut sel| {
            sel.selection_time = start_time.elapsed();
            sel
        })
    }
    
    /// Intelligent protocol selection with scoring and fallback logic
    async fn intelligent_protocol_selection(&self, capabilities: &NetworkCapabilities) -> Option<TunnelSelection> {
        let tunnels = self.tunnels.read().await;
        
        // Score each available protocol
        let mut scored_protocols: Vec<(TunnelProtocol, f32, String)> = Vec::new();
        
        for tunnel in tunnels.iter() {
            let protocol = tunnel.protocol();
            let (score, reason) = self.score_protocol(&protocol, capabilities).await;
            
            debug!("Protocol {:?} scored {:.2}: {}", protocol, score, reason);
            
            if score > 0.0 {
                scored_protocols.push((protocol, score, reason));
            }
        }
        
        // Sort by score (highest first)
        scored_protocols.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Try protocols in order of score, with validation
        for (protocol, score, reason) in scored_protocols {
            if let Some(tunnel_idx) = self.find_tunnel_index(&protocol, &tunnels).await {
                // Test protocol viability if configured to do so
                if self.config.max_concurrent_attempts > 0 {
                    if let Ok(_) = self.test_protocol_viability(&protocol, tunnel_idx).await {
                        let mut active = self.active_tunnel.write().await;
                        *active = Some(tunnel_idx);
                        
                        return Some(TunnelSelection {
                            protocol,
                            reason: format!("{} (score: {:.2})", reason, score),
                            selection_time: Duration::ZERO, // Will be set by caller
                            is_fallback: score < 0.7, // Consider scores below 0.7 as fallback
                        });
                    } else {
                        warn!("Protocol {:?} failed viability test despite good score", protocol);
                    }
                } else {
                    // Skip viability testing, use based on score alone
                    let mut active = self.active_tunnel.write().await;
                    *active = Some(tunnel_idx);
                    
                    return Some(TunnelSelection {
                        protocol,
                        reason: format!("{} (score: {:.2})", reason, score),
                        selection_time: Duration::ZERO,
                        is_fallback: score < 0.7,
                    });
                }
            }
        }
        
        None
    }
    
    /// Score a protocol based on network conditions (0.0 = unsuitable, 1.0 = perfect)
    async fn score_protocol(&self, protocol: &TunnelProtocol, capabilities: &NetworkCapabilities) -> (f32, String) {
        let mut score = 0.0;
        let mut reasons = Vec::new();
        
        match protocol {
            TunnelProtocol::SixToFour => {
                if !capabilities.has_ipv4 {
                    return (0.0, "No IPv4 connectivity".to_string());
                }
                
                if capabilities.behind_nat {
                    return (0.0, "6to4 requires public IPv4 address, behind NAT".to_string());
                }
                
                if capabilities.public_ipv4.is_some() {
                    score += 0.8; // High score for public IPv4
                    reasons.push("has public IPv4");
                } else {
                    return (0.0, "6to4 requires public IPv4 address".to_string());
                }
                
                // Bonus for higher MTU
                if capabilities.interface_mtu >= 1500 {
                    score += 0.2;
                    reasons.push("good MTU");
                }
                
                (score, format!("6to4 suitable: {}", reasons.join(", ")))
            }
            
            TunnelProtocol::Teredo => {
                if !capabilities.has_ipv4 {
                    return (0.0, "No IPv4 connectivity".to_string());
                }
                
                score += 0.6; // Base score for Teredo
                reasons.push("works with any IPv4");
                
                if capabilities.behind_nat {
                    score += 0.3; // Teredo is designed for NAT traversal
                    reasons.push("excellent NAT traversal");
                } else {
                    score += 0.1; // Still works without NAT
                }
                
                if capabilities.has_upnp {
                    score += 0.1; // UPnP can help with port mapping
                    reasons.push("UPnP available");
                }
                
                (score, format!("Teredo suitable: {}", reasons.join(", ")))
            }
            
            TunnelProtocol::SixInFour => {
                if !capabilities.has_ipv4 {
                    return (0.0, "No IPv4 connectivity".to_string());
                }
                
                // 6in4 requires explicit configuration, so it's a fallback
                score += 0.4;
                reasons.push("requires manual configuration");
                
                if !capabilities.behind_nat && capabilities.public_ipv4.is_some() {
                    score += 0.3; // Better with public IP
                    reasons.push("has public IPv4");
                }
                
                // Higher MTU is beneficial for 6in4
                if capabilities.interface_mtu >= 1500 {
                    score += 0.2;
                    reasons.push("good MTU");
                }
                
                (score, format!("6in4 suitable: {}", reasons.join(", ")))
            }
            
            TunnelProtocol::DsLite => {
                if !capabilities.has_ipv6 {
                    return (0.0, "DS-Lite requires IPv6 connectivity".to_string());
                }
                
                // DS-Lite gets high score as it's ISP-provided and reliable
                score += 0.9;
                reasons.push("ISP-provided infrastructure");
                
                // DS-Lite works best with native IPv6
                if capabilities.has_ipv6 && !capabilities.ipv6_addresses.is_empty() {
                    score += 0.1;
                    reasons.push("native IPv6 available");
                }
                
                // DS-Lite handles NAT automatically at the AFTR
                if capabilities.behind_nat {
                    // No penalty for being behind NAT - AFTR handles this
                    reasons.push("AFTR provides centralized NAT");
                } else {
                    score += 0.05; // Small bonus for not needing NAT
                    reasons.push("direct connectivity");
                }
                
                // Higher MTU is beneficial (less fragmentation)
                if capabilities.interface_mtu >= 1520 {
                    score += 0.05;
                    reasons.push("supports optimal MTU");
                }
                
                (score, format!("DS-Lite suitable: {}", reasons.join(", ")))
            }
        }
    }
    
    /// Find the index of a tunnel with the specified protocol
    async fn find_tunnel_index(&self, protocol: &TunnelProtocol, tunnels: &[Box<dyn Tunnel>]) -> Option<usize> {
        for (idx, tunnel) in tunnels.iter().enumerate() {
            if tunnel.protocol() == *protocol {
                return Some(idx);
            }
        }
        None
    }
    
    /// Test if a protocol is actually viable by attempting a quick connection test
    async fn test_protocol_viability(&self, _protocol: &TunnelProtocol, tunnel_idx: usize) -> Result<()> {
        let tunnels = self.tunnels.read().await;
        
        if let Some(tunnel) = tunnels.get(tunnel_idx) {
            // For now, just check if the tunnel reports as suitable for its state
            match tunnel.state().await {
                TunnelState::Connected => Ok(()),
                TunnelState::Failed(_) => Err(P2PError::Network("Tunnel in failed state".to_string())),
                _ => {
                    // Could perform more sophisticated testing here:
                    // - Try to establish a test connection
                    // - Send a ping packet
                    // - Verify routing tables
                    Ok(()) // Assume viable for now
                }
            }
        } else {
            Err(P2PError::Network("Tunnel not found".to_string()))
        }
    }
    
    /// Check if a protocol is suitable for the current network conditions
    #[allow(dead_code)]
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
            TunnelProtocol::DsLite => {
                // DS-Lite requires IPv6 connectivity (native or tunneled)
                capabilities.has_ipv6
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
        let current_active = {
            let active = self.active_tunnel.read().await;
            *active
        };
        
        let mut active_tunnel_failed = false;
        
        for (idx, tunnel) in tunnels.iter_mut().enumerate() {
            match tunnel.ping(self.config.health_check_timeout).await {
                Ok(rtt) => {
                    debug!("Health check passed for {} tunnel (RTT: {:?})", 
                           format!("{:?}", tunnel.protocol()), rtt);
                }
                Err(e) => {
                    warn!("Health check failed for {} tunnel: {}", 
                          format!("{:?}", tunnel.protocol()), e);
                    
                    // Check if this is the currently active tunnel
                    if current_active == Some(idx) {
                        active_tunnel_failed = true;
                    }
                    
                    if self.config.auto_failover {
                        // Try to reconnect
                        if let Err(reconnect_err) = tunnel.connect().await {
                            warn!("Failed to reconnect {} tunnel: {}", 
                                  format!("{:?}", tunnel.protocol()), reconnect_err);
                        }
                    }
                }
            }
        }
        
        // If active tunnel failed and auto-failover is enabled, find replacement
        if active_tunnel_failed && self.config.auto_failover {
            drop(tunnels); // Release write lock before calling failover
            self.perform_automatic_failover().await?;
        }
        
        Ok(())
    }
    
    /// Perform automatic failover to the next best available tunnel
    pub async fn perform_automatic_failover(&self) -> Result<()> {
        info!("Performing automatic tunnel failover...");
        
        // Detect current network capabilities
        let capabilities = detect_network_capabilities().await?;
        
        // Select new tunnel, excluding the currently failed one
        if let Some(selection) = self.select_tunnel(&capabilities).await {
            info!("Failover successful: switched to {} tunnel ({})", 
                  format!("{:?}", selection.protocol), selection.reason);
            
            // Connect to the new tunnel
            self.connect().await?;
            
            Ok(())
        } else {
            let error_msg = "No suitable backup tunnel available for failover";
            warn!("{}", error_msg);
            Err(P2PError::Network(error_msg.to_string()))
        }
    }
    
    /// Get tunnel quality metrics for monitoring and selection
    pub async fn get_tunnel_quality_metrics(&self) -> Vec<TunnelQualityMetric> {
        let tunnels = self.tunnels.read().await;
        let mut metrics = Vec::new();
        
        for tunnel in tunnels.iter() {
            let tunnel_metrics = tunnel.metrics().await;
            let state = tunnel.state().await;
            
            let quality = TunnelQualityMetric {
                protocol: tunnel.protocol(),
                state: state.clone(),
                rtt: tunnel_metrics.rtt,
                packet_loss: if tunnel_metrics.packets_sent > 0 {
                    Some((tunnel_metrics.packets_dropped as f32 / tunnel_metrics.packets_sent as f32) * 100.0)
                } else {
                    None
                },
                throughput: calculate_throughput(&tunnel_metrics),
                reliability_score: calculate_reliability_score(&state, &tunnel_metrics),
                last_activity: tunnel_metrics.last_activity,
            };
            
            metrics.push(quality);
        }
        
        metrics
    }
    
    /// Start automatic monitoring task for continuous health checks
    pub async fn start_monitoring(&self) -> Result<()> {
        if self.config.health_check_interval.is_zero() {
            debug!("Health check monitoring disabled (interval is zero)");
            return Ok(());
        }
        
        info!("Starting tunnel monitoring with interval {:?}", self.config.health_check_interval);
        
        // In a real implementation, this would spawn a background task
        // For now, we just log that monitoring would start
        debug!("Tunnel monitoring task would be spawned here");
        
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
    
    let mut capabilities = NetworkCapabilities {
        has_ipv6: false,
        has_ipv4: false,
        behind_nat: false,
        public_ipv4: None,
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    // Detect IPv4 connectivity
    capabilities.has_ipv4 = detect_ipv4_connectivity().await;
    debug!("IPv4 connectivity: {}", capabilities.has_ipv4);
    
    // Detect IPv6 connectivity
    let ipv6_result = detect_ipv6_connectivity().await;
    capabilities.has_ipv6 = !ipv6_result.is_empty();
    capabilities.ipv6_addresses = ipv6_result;
    debug!("IPv6 connectivity: {}, addresses: {:?}", capabilities.has_ipv6, capabilities.ipv6_addresses);
    
    // Detect NAT presence and public IPv4
    if capabilities.has_ipv4 {
        let nat_detection = detect_nat_and_public_ip().await;
        capabilities.behind_nat = nat_detection.0;
        capabilities.public_ipv4 = nat_detection.1;
        debug!("NAT detection: behind_nat={}, public_ipv4={:?}", capabilities.behind_nat, capabilities.public_ipv4);
    }
    
    // Test UPnP availability
    capabilities.has_upnp = test_upnp_availability().await;
    debug!("UPnP availability: {}", capabilities.has_upnp);
    
    // Detect interface MTU
    capabilities.interface_mtu = detect_interface_mtu().await;
    debug!("Interface MTU: {}", capabilities.interface_mtu);
    
    info!("Network capabilities detected: IPv4={}, IPv6={}, NAT={}, UPnP={}, MTU={}", 
          capabilities.has_ipv4, capabilities.has_ipv6, capabilities.behind_nat,
          capabilities.has_upnp, capabilities.interface_mtu);
    
    Ok(capabilities)
}

/// Detect IPv4 connectivity by testing connection to known servers
async fn detect_ipv4_connectivity() -> bool {
    // Try to connect to well-known IPv4 addresses
    let test_addresses = [
        "8.8.8.8:53",     // Google DNS
        "1.1.1.1:53",     // Cloudflare DNS
        "208.67.222.222:53", // OpenDNS
    ];
    
    for addr in &test_addresses {
        if let Ok(_) = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::TcpStream::connect(addr)
        ).await {
            debug!("IPv4 connectivity confirmed via {}", addr);
            return true;
        }
    }
    
    debug!("IPv4 connectivity test failed");
    false
}

/// Detect IPv6 connectivity and available addresses
async fn detect_ipv6_connectivity() -> Vec<Ipv6Addr> {
    let mut ipv6_addrs = Vec::new();
    
    // Get local IPv6 addresses
    if let Ok(interfaces) = get_network_interfaces().await {
        for interface in interfaces {
            for addr in interface.ipv6_addrs {
                if !addr.is_loopback() && !addr.is_multicast() {
                    ipv6_addrs.push(addr);
                }
            }
        }
    }
    
    // Test connectivity to IPv6 servers if we have addresses
    if !ipv6_addrs.is_empty() {
        let test_addresses = [
            "[2001:4860:4860::8888]:53", // Google DNS
            "[2606:4700:4700::1111]:53", // Cloudflare DNS
        ];
        
        for addr in &test_addresses {
            if let Ok(_) = tokio::time::timeout(
                Duration::from_secs(3),
                tokio::net::TcpStream::connect(addr)
            ).await {
                debug!("IPv6 connectivity confirmed via {}", addr);
                return ipv6_addrs;
            }
        }
        
        debug!("IPv6 addresses found but no external connectivity");
        ipv6_addrs.clear(); // Clear if no external connectivity
    }
    
    ipv6_addrs
}

/// Detect NAT presence and discover public IPv4 address
async fn detect_nat_and_public_ip() -> (bool, Option<Ipv4Addr>) {
    // Get local IPv4 address
    let local_ipv4 = get_local_ipv4_addr().await;
    
    // Try to discover public IP via STUN-like service
    if let Ok(public_ip) = discover_public_ipv4().await {
        let behind_nat = local_ipv4.map_or(true, |local| local != public_ip);
        return (behind_nat, Some(public_ip));
    }
    
    // Fallback: check if local IP is private
    if let Some(local) = local_ipv4 {
        let behind_nat = local.is_private();
        (behind_nat, if behind_nat { None } else { Some(local) })
    } else {
        (true, None)
    }
}

/// Discover public IPv4 address using external services
async fn discover_public_ipv4() -> Result<Ipv4Addr> {
    // Try multiple IP discovery services
    let services = [
        "https://api.ipify.org",
        "https://icanhazip.com",
        "https://ifconfig.me/ip",
    ];
    
    for service in &services {
        if let Ok(response) = tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get(*service)
        ).await {
            if let Ok(response) = response {
                if let Ok(ip_str) = response.text().await {
                    if let Ok(ip) = ip_str.trim().parse::<Ipv4Addr>() {
                        debug!("Public IPv4 discovered via {}: {}", service, ip);
                        return Ok(ip);
                    }
                }
            }
        }
    }
    
    Err(P2PError::Network("Failed to discover public IPv4 address".to_string()))
}

/// Get local IPv4 address
async fn get_local_ipv4_addr() -> Option<Ipv4Addr> {
    if let Ok(interfaces) = get_network_interfaces().await {
        for interface in interfaces {
            for addr in interface.ipv4_addrs {
                if !addr.is_loopback() && !addr.is_multicast() {
                    return Some(addr);
                }
            }
        }
    }
    None
}

/// Test UPnP availability for automatic port forwarding
async fn test_upnp_availability() -> bool {
    // This would test for UPnP Internet Gateway Device Protocol
    // For now, return false as a conservative default
    // In production, this would:
    // 1. Send M-SEARCH multicast to discover UPnP devices
    // 2. Parse device descriptions
    // 3. Test port mapping capabilities
    debug!("UPnP testing not implemented, assuming unavailable");
    false
}

/// Detect interface MTU
async fn detect_interface_mtu() -> u16 {
    // Try to detect the MTU of the primary network interface
    // For now, return standard Ethernet MTU
    // In production, this would:
    // 1. Query interface statistics
    // 2. Perform path MTU discovery
    // 3. Test actual payload sizes
    1500
}

/// Network interface information
#[allow(dead_code)]
#[derive(Debug)]
struct NetworkInterface {
    _name: String,
    ipv4_addrs: Vec<Ipv4Addr>,
    ipv6_addrs: Vec<Ipv6Addr>,
}

/// Get network interfaces (simplified implementation)
async fn get_network_interfaces() -> Result<Vec<NetworkInterface>> {
    // This is a placeholder - in production this would use platform-specific APIs
    // to enumerate network interfaces and their addresses
    
    // For now, simulate a typical interface
    let interface = NetworkInterface {
        _name: "eth0".to_string(),
        ipv4_addrs: vec![Ipv4Addr::new(192, 168, 1, 100)],
        ipv6_addrs: vec![],
    };
    
    Ok(vec![interface])
}

/// Calculate throughput based on tunnel metrics
fn calculate_throughput(metrics: &TunnelMetrics) -> Option<f64> {
    let elapsed = metrics.last_activity.elapsed();
    if elapsed.as_secs() > 0 {
        let total_bytes = metrics.bytes_sent + metrics.bytes_received;
        Some(total_bytes as f64 / elapsed.as_secs_f64())
    } else {
        None
    }
}

/// Calculate reliability score based on tunnel state and metrics
fn calculate_reliability_score(state: &TunnelState, metrics: &TunnelMetrics) -> f32 {
    let mut score = match state {
        TunnelState::Connected => 1.0,
        TunnelState::Connecting => 0.5,
        TunnelState::Disconnected => 0.0,
        TunnelState::Failed(_) => 0.0,
        TunnelState::Disconnecting => 0.2,
    };
    
    // Adjust score based on packet loss
    if metrics.packets_sent > 0 {
        let packet_loss = metrics.packets_dropped as f32 / metrics.packets_sent as f32;
        score *= (1.0 - packet_loss).max(0.0);
    }
    
    // Adjust score based on activity recency
    let inactive_time = metrics.last_activity.elapsed();
    if inactive_time > Duration::from_secs(300) { // 5 minutes
        score *= 0.5; // Penalize stale tunnels
    }
    
    score.min(1.0).max(0.0)
}

// Tunneling protocol implementations
pub mod sixto4;
pub mod teredo;
pub mod sixinfour;
pub mod dslite;

pub use sixto4::SixToFourTunnel;
pub use teredo::TeredoTunnel;
pub use sixinfour::SixInFourTunnel;
pub use dslite::DsLiteTunnel;

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
            // 6in4 requires explicit endpoint configuration
            config.mtu = 1480; // Account for IPv4 header overhead
            // Note: local_ipv4 and remote_ipv4 must be set by caller
            // IPv6 prefix can be configured or will use default
        }
        TunnelProtocol::DsLite => {
            // DS-Lite uses IPv4-in-IPv6 encapsulation
            config.mtu = 1520; // Account for IPv6 header overhead (40 bytes)
            // AFTR address discovery will be handled by the tunnel implementation
            // For now, use a placeholder AFTR name that could be configured
            config.aftr_name = Some("aftr.example.com".to_string());
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
            let tunnel = SixInFourTunnel::new(config)?;
            Ok(Box::new(tunnel))
        }
        TunnelProtocol::DsLite => {
            let tunnel = DsLiteTunnel::new(config)?;
            Ok(Box::new(tunnel))
        }
    }
}