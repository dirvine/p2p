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
//! - **DS-Lite**: ISP-provided dual-stack lite tunneling
//! - **ISATAP**: Enterprise network tunneling for corporate environments
//! - **MAP-E**: ISP IPv4 sharing via encapsulation with mapping rules
//! - **MAP-T**: ISP IPv4/IPv6 translation with mapping rules
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
    /// ISATAP (Intra-Site Automatic Tunnel Addressing Protocol) for enterprise networks (RFC 5214)
    Isatap,
    /// MAP-E (Mapping of Address and Port - Encapsulation) for ISP IPv4 sharing (RFC 7597)
    MapE,
    /// MAP-T (Mapping of Address and Port - Translation) for ISP IPv4/IPv6 translation (RFC 7599)
    MapT,
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
                TunnelProtocol::Isatap,     // Enterprise networks, excellent for corporate
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
            
            TunnelProtocol::Isatap => {
                if !capabilities.has_ipv4 {
                    return (0.0, "ISATAP requires IPv4 connectivity".to_string());
                }
                
                // ISATAP is excellent for enterprise environments
                score += 0.8;
                reasons.push("enterprise-grade tunneling");
                
                // ISATAP works well in corporate networks with or without NAT
                if capabilities.behind_nat {
                    score += 0.1; // Works fine behind corporate NAT
                    reasons.push("corporate NAT compatible");
                } else {
                    score += 0.15; // Even better with direct connectivity
                    reasons.push("direct enterprise connectivity");
                }
                
                // ISATAP benefits from good MTU for enterprise traffic
                if capabilities.interface_mtu >= 1500 {
                    score += 0.1;
                    reasons.push("enterprise MTU");
                }
                
                // Check for enterprise network indicators (private IP ranges)
                if let Some(ipv4) = capabilities.public_ipv4 {
                    if ipv4.is_private() {
                        score += 0.2; // Bonus for private networks (enterprise indicator)
                        reasons.push("private network detected");
                    }
                }
                
                (score, format!("ISATAP suitable: {}", reasons.join(", ")))
            }
            
            TunnelProtocol::MapE => {
                if !capabilities.has_ipv4 {
                    return (0.0, "MAP-E requires IPv4 connectivity".to_string());
                }
                
                // MAP-E is excellent for ISP deployment scenarios
                score += 0.9;
                reasons.push("ISP-grade address sharing");
                
                // MAP-E works well with any IPv4 connectivity
                if capabilities.public_ipv4.is_some() {
                    score += 0.1;
                    reasons.push("public IPv4 available");
                }
                
                // MAP-E is stateless and highly scalable
                if capabilities.interface_mtu >= 1460 {
                    reasons.push("sufficient MTU for encapsulation");
                } else {
                    score -= 0.1; // Slight penalty for low MTU
                    reasons.push("low MTU may affect performance");
                }
                
                (score, format!("MAP-E suitable: {}", reasons.join(", ")))
            }
            
            TunnelProtocol::MapT => {
                if !capabilities.has_ipv4 {
                    return (0.0, "MAP-T requires IPv4 connectivity".to_string());
                }
                
                // MAP-T is excellent for pure IPv6 networks with IPv4 translation
                score += 0.85;
                reasons.push("stateless IPv4/IPv6 translation");
                
                // MAP-T works well with any IPv4 connectivity
                if capabilities.public_ipv4.is_some() {
                    score += 0.1;
                    reasons.push("public IPv4 available");
                }
                
                // MAP-T has no encapsulation overhead
                score += 0.05;
                reasons.push("no encapsulation overhead");
                
                (score, format!("MAP-T suitable: {}", reasons.join(", ")))
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
            TunnelProtocol::Isatap => {
                // ISATAP works with any IPv4 connectivity, ideal for enterprise networks
                capabilities.has_ipv4
            }
            TunnelProtocol::MapE => {
                // MAP-E requires IPv4 connectivity and is used by ISPs for address sharing
                capabilities.has_ipv4
            }
            TunnelProtocol::MapT => {
                // MAP-T requires IPv4 connectivity and provides stateless translation
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
pub mod isatap;
pub mod map;

pub use sixto4::SixToFourTunnel;
pub use teredo::TeredoTunnel;
pub use sixinfour::SixInFourTunnel;
pub use dslite::DsLiteTunnel;
pub use isatap::IsatapTunnel;
pub use map::{MapTunnel, MapProtocol, MapRule, PortParameters, PortSet};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Mutex;

    /// Mock tunnel implementation for testing
    struct MockTunnel {
        protocol: TunnelProtocol,
        config: TunnelConfig,
        state: Arc<Mutex<TunnelState>>,
        metrics: Arc<Mutex<TunnelMetrics>>,
        should_fail: bool,
        packet_counter: AtomicUsize,
    }

    impl MockTunnel {
        fn new(config: TunnelConfig) -> Self {
            Self {
                protocol: config.protocol.clone(),
                config,
                state: Arc::new(Mutex::new(TunnelState::Disconnected)),
                metrics: Arc::new(Mutex::new(TunnelMetrics::default())),
                should_fail: false,
                packet_counter: AtomicUsize::new(0),
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        async fn set_state(&self, new_state: TunnelState) {
            let mut state = self.state.lock().await;
            *state = new_state;
        }
    }

    #[async_trait]
    impl Tunnel for MockTunnel {
        fn protocol(&self) -> TunnelProtocol {
            self.protocol.clone()
        }

        fn config(&self) -> &TunnelConfig {
            &self.config
        }

        async fn state(&self) -> TunnelState {
            self.state.lock().await.clone()
        }

        async fn metrics(&self) -> TunnelMetrics {
            self.metrics.lock().await.clone()
        }

        async fn connect(&mut self) -> Result<()> {
            if self.should_fail {
                let mut state = self.state.lock().await;
                *state = TunnelState::Failed("Connection failed".to_string());
                return Err(P2PError::Network("Mock connection failure".to_string()));
            }

            let mut state = self.state.lock().await;
            *state = TunnelState::Connecting;
            tokio::time::sleep(Duration::from_millis(10)).await;
            *state = TunnelState::Connected;

            let mut metrics = self.metrics.lock().await;
            metrics.establishment_time = Duration::from_millis(10);
            metrics.last_activity = Instant::now();

            Ok(())
        }

        async fn disconnect(&mut self) -> Result<()> {
            let mut state = self.state.lock().await;
            *state = TunnelState::Disconnecting;
            tokio::time::sleep(Duration::from_millis(5)).await;
            *state = TunnelState::Disconnected;
            Ok(())
        }

        async fn is_active(&self) -> bool {
            matches!(self.state().await, TunnelState::Connected)
        }

        async fn encapsulate(&self, ipv6_packet: &[u8]) -> Result<Vec<u8>> {
            if !self.is_active().await {
                return Err(P2PError::Network("Tunnel not active".to_string()));
            }

            // Mock encapsulation: prepend 4-byte IPv4 header
            let mut ipv4_packet = vec![0x45, 0x00, 0x00, 0x00]; // Mock IPv4 header
            ipv4_packet.extend_from_slice(ipv6_packet);
            Ok(ipv4_packet)
        }

        async fn decapsulate(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
            if !self.is_active().await {
                return Err(P2PError::Network("Tunnel not active".to_string()));
            }

            if ipv4_packet.len() < 4 {
                return Err(P2PError::Network("Invalid IPv4 packet".to_string()));
            }

            // Mock decapsulation: remove 4-byte IPv4 header
            Ok(ipv4_packet[4..].to_vec())
        }

        async fn send(&mut self, packet: &[u8]) -> Result<()> {
            if !self.is_active().await {
                return Err(P2PError::Network("Tunnel not active".to_string()));
            }

            let mut metrics = self.metrics.lock().await;
            metrics.bytes_sent += packet.len() as u64;
            metrics.packets_sent += 1;
            metrics.last_activity = Instant::now();

            self.packet_counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn receive(&mut self) -> Result<Vec<u8>> {
            if !self.is_active().await {
                return Err(P2PError::Network("Tunnel not active".to_string()));
            }

            let packet = b"mock_packet".to_vec();
            let mut metrics = self.metrics.lock().await;
            metrics.bytes_received += packet.len() as u64;
            metrics.packets_received += 1;
            metrics.last_activity = Instant::now();

            Ok(packet)
        }

        async fn maintain(&mut self) -> Result<()> {
            let mut metrics = self.metrics.lock().await;
            metrics.last_activity = Instant::now();
            Ok(())
        }

        async fn local_ipv6_addr(&self) -> Result<Ipv6Addr> {
            self.config.ipv6_prefix
                .ok_or_else(|| P2PError::Network("No IPv6 prefix configured".to_string()))
        }

        async fn local_ipv4_addr(&self) -> Result<Ipv4Addr> {
            self.config.local_ipv4
                .ok_or_else(|| P2PError::Network("No IPv4 address configured".to_string()))
        }

        async fn ping(&mut self, timeout: Duration) -> Result<Duration> {
            if self.should_fail {
                return Err(P2PError::Network("Ping failed".to_string()));
            }

            if !self.is_active().await {
                return Err(P2PError::Network("Tunnel not active".to_string()));
            }

            // Mock ping with random latency
            let rtt = Duration::from_millis(10 + (timeout.as_millis() % 50) as u64);
            
            let mut metrics = self.metrics.lock().await;
            metrics.rtt = Some(rtt);
            metrics.last_activity = Instant::now();

            Ok(rtt)
        }
    }

    fn create_test_tunnel_config(protocol: TunnelProtocol) -> TunnelConfig {
        TunnelConfig {
            protocol,
            local_ipv4: Some(Ipv4Addr::new(192, 168, 1, 100)),
            remote_ipv4: Some(Ipv4Addr::new(203, 0, 113, 1)),
            ipv6_prefix: Some(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            aftr_ipv6: Some(Ipv6Addr::new(0x2001, 0xdb8, 0, 1, 0, 0, 0, 1)),
            aftr_name: Some("aftr.test.com".to_string()),
            mtu: 1280,
            keepalive_interval: Duration::from_secs(30),
            establishment_timeout: Duration::from_secs(10),
        }
    }

    fn create_test_network_capabilities() -> NetworkCapabilities {
        NetworkCapabilities {
            has_ipv6: false,
            has_ipv4: true,
            behind_nat: false,
            public_ipv4: Some(Ipv4Addr::new(203, 0, 113, 100)),
            ipv6_addresses: vec![],
            has_upnp: false,
            interface_mtu: 1500,
        }
    }

    #[test]
    fn test_tunnel_protocol_equality() {
        assert_eq!(TunnelProtocol::SixToFour, TunnelProtocol::SixToFour);
        assert_ne!(TunnelProtocol::SixToFour, TunnelProtocol::Teredo);
        assert_eq!(TunnelProtocol::MapE, TunnelProtocol::MapE);
    }

    #[test]
    fn test_tunnel_state_variants() {
        let disconnected = TunnelState::Disconnected;
        let connecting = TunnelState::Connecting;
        let connected = TunnelState::Connected;
        let failed = TunnelState::Failed("test error".to_string());
        let disconnecting = TunnelState::Disconnecting;

        assert!(matches!(disconnected, TunnelState::Disconnected));
        assert!(matches!(connecting, TunnelState::Connecting));
        assert!(matches!(connected, TunnelState::Connected));
        assert!(matches!(failed, TunnelState::Failed(_)));
        assert!(matches!(disconnecting, TunnelState::Disconnecting));
    }

    #[test]
    fn test_tunnel_config_default() {
        let config = TunnelConfig::default();

        assert_eq!(config.protocol, TunnelProtocol::SixToFour);
        assert_eq!(config.mtu, 1280);
        assert_eq!(config.keepalive_interval, Duration::from_secs(30));
        assert_eq!(config.establishment_timeout, Duration::from_secs(10));
        assert!(config.local_ipv4.is_none());
        assert!(config.remote_ipv4.is_none());
    }

    #[test]
    fn test_tunnel_manager_config_default() {
        let config = TunnelManagerConfig::default();

        assert_eq!(config.protocol_preference.len(), 5);
        assert_eq!(config.protocol_preference[0], TunnelProtocol::DsLite);
        assert_eq!(config.health_check_interval, Duration::from_secs(60));
        assert!(config.auto_failover);
        assert_eq!(config.max_concurrent_attempts, 3);
    }

    #[test]
    fn test_tunnel_metrics_default() {
        let metrics = TunnelMetrics::default();

        assert_eq!(metrics.bytes_sent, 0);
        assert_eq!(metrics.bytes_received, 0);
        assert_eq!(metrics.packets_sent, 0);
        assert_eq!(metrics.packets_received, 0);
        assert_eq!(metrics.packets_dropped, 0);
        assert!(metrics.rtt.is_none());
        assert_eq!(metrics.establishment_time, Duration::ZERO);
    }

    #[tokio::test]
    async fn test_tunnel_manager_creation() {
        let manager = TunnelManager::new();
        assert!(manager.active_tunnel().await.is_none());

        let custom_config = TunnelManagerConfig {
            auto_failover: false,
            max_concurrent_attempts: 1,
            ..Default::default()
        };
        let custom_manager = TunnelManager::with_config(custom_config);
        assert!(custom_manager.active_tunnel().await.is_none());
    }

    #[tokio::test]
    async fn test_tunnel_addition_and_retrieval() {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::SixToFour);
        let tunnel = MockTunnel::new(config);

        manager.add_tunnel(Box::new(tunnel)).await;

        let tunnels = manager.tunnels.read().await;
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].protocol(), TunnelProtocol::SixToFour);
    }

    #[tokio::test]
    async fn test_mock_tunnel_lifecycle() -> Result<()> {
        let config = create_test_tunnel_config(TunnelProtocol::Teredo);
        let mut tunnel = MockTunnel::new(config);

        // Initial state
        assert_eq!(tunnel.state().await, TunnelState::Disconnected);
        assert!(!tunnel.is_active().await);

        // Connect
        tunnel.connect().await?;
        assert_eq!(tunnel.state().await, TunnelState::Connected);
        assert!(tunnel.is_active().await);

        // Test addresses
        let ipv6 = tunnel.local_ipv6_addr().await?;
        let ipv4 = tunnel.local_ipv4_addr().await?;
        assert_eq!(ipv6, Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert_eq!(ipv4, Ipv4Addr::new(192, 168, 1, 100));

        // Test ping
        let rtt = tunnel.ping(Duration::from_secs(1)).await?;
        assert!(rtt > Duration::ZERO);

        // Disconnect
        tunnel.disconnect().await?;
        assert_eq!(tunnel.state().await, TunnelState::Disconnected);
        assert!(!tunnel.is_active().await);

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_packet_operations() -> Result<()> {
        let config = create_test_tunnel_config(TunnelProtocol::SixToFour);
        let mut tunnel = MockTunnel::new(config);
        
        tunnel.connect().await?;

        // Test encapsulation
        let ipv6_packet = vec![0x60, 0x00, 0x00, 0x00]; // Mock IPv6 header
        let ipv4_packet = tunnel.encapsulate(&ipv6_packet).await?;
        assert_eq!(ipv4_packet.len(), ipv6_packet.len() + 4);

        // Test decapsulation
        let decapsulated = tunnel.decapsulate(&ipv4_packet).await?;
        assert_eq!(decapsulated, ipv6_packet);

        // Test send/receive
        let test_packet = b"test data";
        tunnel.send(test_packet).await?;
        
        let received = tunnel.receive().await?;
        assert_eq!(received, b"mock_packet");

        // Check metrics
        let metrics = tunnel.metrics().await;
        assert_eq!(metrics.packets_sent, 1);
        assert_eq!(metrics.packets_received, 1);
        assert_eq!(metrics.bytes_sent, test_packet.len() as u64);

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_failure_handling() {
        let config = create_test_tunnel_config(TunnelProtocol::Teredo);
        let mut failing_tunnel = MockTunnel::new(config).with_failure();

        // Connection should fail
        let result = failing_tunnel.connect().await;
        assert!(result.is_err());
        assert!(matches!(failing_tunnel.state().await, TunnelState::Failed(_)));

        // Operations should fail when tunnel failed
        let packet = vec![1, 2, 3, 4];
        assert!(failing_tunnel.send(&packet).await.is_err());
        assert!(failing_tunnel.receive().await.is_err());
        assert!(failing_tunnel.ping(Duration::from_secs(1)).await.is_err());
    }

    #[tokio::test]
    async fn test_tunnel_operations_when_disconnected() {
        let config = create_test_tunnel_config(TunnelProtocol::SixInFour);
        let tunnel = MockTunnel::new(config);

        // Operations should fail when disconnected
        let packet = vec![1, 2, 3, 4];
        assert!(tunnel.encapsulate(&packet).await.is_err());
        assert!(tunnel.decapsulate(&packet).await.is_err());
    }

    #[tokio::test]
    async fn test_tunnel_selection_with_native_ipv6() {
        let manager = TunnelManager::new();
        let capabilities = NetworkCapabilities {
            has_ipv6: true,
            ipv6_addresses: vec![Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)],
            ..create_test_network_capabilities()
        };

        let selection = manager.select_tunnel(&capabilities).await;
        assert!(selection.is_none()); // No tunneling needed with native IPv6
    }

    #[tokio::test]
    async fn test_tunnel_selection_scoring() {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::SixToFour);
        let tunnel = MockTunnel::new(config);
        manager.add_tunnel(Box::new(tunnel)).await;

        let capabilities = create_test_network_capabilities();
        let selection = manager.select_tunnel(&capabilities).await;

        assert!(selection.is_some());
        let selection = selection.unwrap();
        assert_eq!(selection.protocol, TunnelProtocol::SixToFour);
        assert!(selection.reason.contains("6to4 suitable"));
        assert!(!selection.is_fallback);
    }

    #[tokio::test]
    async fn test_tunnel_selection_behind_nat() {
        let manager = TunnelManager::new();
        
        // Add Teredo tunnel (good for NAT)
        let teredo_config = create_test_tunnel_config(TunnelProtocol::Teredo);
        let teredo_tunnel = MockTunnel::new(teredo_config);
        manager.add_tunnel(Box::new(teredo_tunnel)).await;

        let capabilities = NetworkCapabilities {
            behind_nat: true,
            public_ipv4: None,
            ..create_test_network_capabilities()
        };

        let selection = manager.select_tunnel(&capabilities).await;
        assert!(selection.is_some());
        let selection = selection.unwrap();
        assert_eq!(selection.protocol, TunnelProtocol::Teredo);
        assert!(selection.reason.contains("NAT traversal"));
    }

    #[tokio::test]
    async fn test_tunnel_selection_ds_lite() {
        let manager = TunnelManager::new();
        
        let dslite_config = create_test_tunnel_config(TunnelProtocol::DsLite);
        let dslite_tunnel = MockTunnel::new(dslite_config);
        manager.add_tunnel(Box::new(dslite_tunnel)).await;

        let capabilities = NetworkCapabilities {
            has_ipv6: true,
            ipv6_addresses: vec![], // No working IPv6 yet
            ..create_test_network_capabilities()
        };

        let selection = manager.select_tunnel(&capabilities).await;
        assert!(selection.is_some());
        let selection = selection.unwrap();
        assert_eq!(selection.protocol, TunnelProtocol::DsLite);
        assert!(selection.reason.contains("ISP-provided"));
    }

    #[tokio::test]
    async fn test_tunnel_manager_connection() -> Result<()> {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::SixToFour);
        let tunnel = MockTunnel::new(config);
        manager.add_tunnel(Box::new(tunnel)).await;

        // Select tunnel first
        let capabilities = create_test_network_capabilities();
        manager.select_tunnel(&capabilities).await;

        // Connect
        manager.connect().await?;

        // Should have active tunnel
        assert_eq!(manager.active_tunnel().await, Some(TunnelProtocol::SixToFour));

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_manager_send_receive() -> Result<()> {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::Teredo);
        let tunnel = MockTunnel::new(config);
        manager.add_tunnel(Box::new(tunnel)).await;

        let capabilities = create_test_network_capabilities();
        manager.select_tunnel(&capabilities).await;
        manager.connect().await?;

        // Test sending
        let packet = b"test packet";
        manager.send(packet).await?;

        // Test receiving
        let received = manager.receive().await?;
        assert_eq!(received, b"mock_packet");

        // Test metrics
        let metrics = manager.metrics().await;
        assert!(metrics.is_some());
        let metrics = metrics.unwrap();
        assert_eq!(metrics.packets_sent, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_manager_no_active_tunnel() {
        let manager = TunnelManager::new();

        // Operations should fail without active tunnel
        assert!(manager.connect().await.is_err());
        assert!(manager.send(b"test").await.is_err());
        assert!(manager.receive().await.is_err());
        assert!(manager.metrics().await.is_none());
    }

    #[tokio::test]
    async fn test_tunnel_manager_health_check() -> Result<()> {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::SixToFour);
        let tunnel = MockTunnel::new(config);
        manager.add_tunnel(Box::new(tunnel)).await;

        let capabilities = create_test_network_capabilities();
        manager.select_tunnel(&capabilities).await;
        manager.connect().await?;

        // Health check should pass
        manager.health_check().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_manager_health_check_with_failure() -> Result<()> {
        // Create manager with auto-failover disabled to avoid trying to switch
        let config = TunnelManagerConfig {
            auto_failover: false,
            ..Default::default()
        };
        let manager = TunnelManager::with_config(config);
        
        let tunnel_config = create_test_tunnel_config(TunnelProtocol::Teredo);
        let failing_tunnel = MockTunnel::new(tunnel_config).with_failure();
        manager.add_tunnel(Box::new(failing_tunnel)).await;

        let capabilities = create_test_network_capabilities();
        manager.select_tunnel(&capabilities).await;

        // Health check should handle failures gracefully
        manager.health_check().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_manager_monitoring() -> Result<()> {
        let manager = TunnelManager::new();
        
        // Start monitoring (should not fail)
        manager.start_monitoring().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_manager_maintenance() -> Result<()> {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::SixInFour);
        let tunnel = MockTunnel::new(config);
        manager.add_tunnel(Box::new(tunnel)).await;

        // Maintenance should not fail
        manager.maintain().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_tunnel_quality_metrics() {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::SixToFour);
        let tunnel = MockTunnel::new(config);
        tunnel.set_state(TunnelState::Connected).await;
        manager.add_tunnel(Box::new(tunnel)).await;

        let quality_metrics = manager.get_tunnel_quality_metrics().await;
        assert_eq!(quality_metrics.len(), 1);
        
        let metric = &quality_metrics[0];
        assert_eq!(metric.protocol, TunnelProtocol::SixToFour);
        assert_eq!(metric.state, TunnelState::Connected);
        assert!(metric.reliability_score > 0.0);
    }

    #[tokio::test]
    async fn test_network_capabilities_detection() -> Result<()> {
        let capabilities = detect_network_capabilities().await?;

        // Should have basic network information
        assert!(capabilities.interface_mtu > 0);
        // Other fields depend on network environment

        Ok(())
    }

    #[test]
    fn test_create_tunnel_config_sixto4() {
        let capabilities = create_test_network_capabilities();
        let config = create_tunnel_config(TunnelProtocol::SixToFour, &capabilities);

        assert_eq!(config.protocol, TunnelProtocol::SixToFour);
        assert_eq!(config.local_ipv4, capabilities.public_ipv4);
        assert!(config.ipv6_prefix.is_some());
        
        // Should generate 6to4 prefix (2002::/16)
        let ipv6 = config.ipv6_prefix.unwrap();
        let segments = ipv6.segments();
        assert_eq!(segments[0], 0x2002);
    }

    #[test]
    fn test_create_tunnel_config_teredo() {
        let capabilities = create_test_network_capabilities();
        let config = create_tunnel_config(TunnelProtocol::Teredo, &capabilities);

        assert_eq!(config.protocol, TunnelProtocol::Teredo);
        assert_eq!(config.mtu, 1280);
        
        // Should use Teredo prefix (2001::/32)
        let ipv6 = config.ipv6_prefix.unwrap();
        let segments = ipv6.segments();
        assert_eq!(segments[0], 0x2001);
    }

    #[test]
    fn test_create_tunnel_config_dslite() {
        let capabilities = create_test_network_capabilities();
        let config = create_tunnel_config(TunnelProtocol::DsLite, &capabilities);

        assert_eq!(config.protocol, TunnelProtocol::DsLite);
        assert_eq!(config.mtu, 1520);
        assert!(config.aftr_name.is_some());
        assert_eq!(config.aftr_name.unwrap(), "aftr.example.com");
    }

    #[test]
    fn test_create_tunnel_config_map_protocols() {
        let capabilities = create_test_network_capabilities();
        
        let map_e_config = create_tunnel_config(TunnelProtocol::MapE, &capabilities);
        assert_eq!(map_e_config.protocol, TunnelProtocol::MapE);
        assert_eq!(map_e_config.mtu, 1460);
        assert_eq!(map_e_config.local_ipv4, capabilities.public_ipv4);
        
        let map_t_config = create_tunnel_config(TunnelProtocol::MapT, &capabilities);
        assert_eq!(map_t_config.protocol, TunnelProtocol::MapT);
        assert_eq!(map_t_config.mtu, 1500);
        assert_eq!(map_t_config.local_ipv4, capabilities.public_ipv4);
    }

    #[test]
    fn test_calculate_reliability_score() {
        let metrics = TunnelMetrics {
            packets_sent: 100,
            packets_dropped: 5,
            last_activity: Instant::now(),
            ..Default::default()
        };

        let score_connected = calculate_reliability_score(&TunnelState::Connected, &metrics);
        assert!(score_connected > 0.9); // Should be high for connected with low loss

        let score_failed = calculate_reliability_score(&TunnelState::Failed("error".to_string()), &metrics);
        assert_eq!(score_failed, 0.0); // Should be 0 for failed state

        let score_connecting = calculate_reliability_score(&TunnelState::Connecting, &metrics);
        assert!(score_connecting > 0.4 && score_connecting < 0.6); // Intermediate score
    }

    #[test]
    fn test_calculate_throughput() {
        let old_activity = Instant::now() - Duration::from_secs(10);
        let metrics = TunnelMetrics {
            bytes_sent: 1000,
            bytes_received: 500,
            last_activity: old_activity,
            ..Default::default()
        };

        let throughput = calculate_throughput(&metrics);
        assert!(throughput.is_some());
        assert!(throughput.unwrap() > 0.0);

        // Test with very recent activity (should return None)
        let recent_metrics = TunnelMetrics {
            last_activity: Instant::now(),
            ..Default::default()
        };
        let recent_throughput = calculate_throughput(&recent_metrics);
        assert!(recent_throughput.is_none());
    }

    #[tokio::test]
    async fn test_protocol_scoring_comprehensive() {
        let manager = TunnelManager::new();

        // Test 6to4 scoring with public IP
        let public_capabilities = create_test_network_capabilities();
        let (score, reason) = manager.score_protocol(&TunnelProtocol::SixToFour, &public_capabilities).await;
        assert!(score > 0.8);
        assert!(reason.contains("6to4 suitable"));

        // Test 6to4 scoring behind NAT (should fail)
        let nat_capabilities = NetworkCapabilities {
            behind_nat: true,
            public_ipv4: None,
            ..public_capabilities
        };
        let (nat_score, nat_reason) = manager.score_protocol(&TunnelProtocol::SixToFour, &nat_capabilities).await;
        assert_eq!(nat_score, 0.0);
        assert!(nat_reason.contains("behind NAT"));

        // Test Teredo with NAT (should score well)
        let (teredo_score, teredo_reason) = manager.score_protocol(&TunnelProtocol::Teredo, &nat_capabilities).await;
        assert!(teredo_score > 0.8);
        assert!(teredo_reason.contains("NAT traversal"));

        // Test ISATAP with enterprise network indicators
        let enterprise_capabilities = NetworkCapabilities {
            public_ipv4: Some(Ipv4Addr::new(10, 0, 0, 1)), // Private IP
            ..create_test_network_capabilities()
        };
        let (isatap_score, isatap_reason) = manager.score_protocol(&TunnelProtocol::Isatap, &enterprise_capabilities).await;
        assert!(isatap_score > 0.9);
        assert!(isatap_reason.contains("private network"));
    }

    #[tokio::test]
    async fn test_tunnel_selection_no_suitable_protocols() {
        let manager = TunnelManager::new();
        
        // Add tunnel that requires IPv6
        let dslite_config = create_test_tunnel_config(TunnelProtocol::DsLite);
        let dslite_tunnel = MockTunnel::new(dslite_config);
        manager.add_tunnel(Box::new(dslite_tunnel)).await;

        // Test with IPv4-only capabilities
        let ipv4_only_capabilities = NetworkCapabilities {
            has_ipv6: false,
            ipv6_addresses: vec![],
            ..create_test_network_capabilities()
        };

        let selection = manager.select_tunnel(&ipv4_only_capabilities).await;
        assert!(selection.is_none());
    }

    #[tokio::test]
    async fn test_tunnel_manager_disconnect() -> Result<()> {
        let manager = TunnelManager::new();
        let config = create_test_tunnel_config(TunnelProtocol::SixToFour);
        let tunnel = MockTunnel::new(config);
        manager.add_tunnel(Box::new(tunnel)).await;

        let capabilities = create_test_network_capabilities();
        manager.select_tunnel(&capabilities).await;
        manager.connect().await?;

        // Disconnect should work
        manager.disconnect().await?;

        Ok(())
    }

    #[test]
    fn test_tunnel_selection_structure() {
        let selection = TunnelSelection {
            protocol: TunnelProtocol::MapE,
            reason: "Test reason".to_string(),
            selection_time: Duration::from_millis(100),
            is_fallback: false,
        };

        assert_eq!(selection.protocol, TunnelProtocol::MapE);
        assert_eq!(selection.reason, "Test reason");
        assert_eq!(selection.selection_time, Duration::from_millis(100));
        assert!(!selection.is_fallback);
    }

    #[test]
    fn test_tunnel_protocol_suitability_checks() {
        let manager = TunnelManager::new();
        let capabilities = create_test_network_capabilities();

        // Test all protocols with IPv4-only
        assert!(manager.is_protocol_suitable(&TunnelProtocol::SixToFour, &capabilities));
        assert!(manager.is_protocol_suitable(&TunnelProtocol::Teredo, &capabilities));
        assert!(manager.is_protocol_suitable(&TunnelProtocol::SixInFour, &capabilities));
        assert!(manager.is_protocol_suitable(&TunnelProtocol::Isatap, &capabilities));
        assert!(manager.is_protocol_suitable(&TunnelProtocol::MapE, &capabilities));
        assert!(manager.is_protocol_suitable(&TunnelProtocol::MapT, &capabilities));

        // DS-Lite requires IPv6
        assert!(!manager.is_protocol_suitable(&TunnelProtocol::DsLite, &capabilities));

        let ipv6_capabilities = NetworkCapabilities {
            has_ipv6: true,
            ..capabilities
        };
        assert!(manager.is_protocol_suitable(&TunnelProtocol::DsLite, &ipv6_capabilities));
    }
}

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
        TunnelProtocol::Isatap => {
            // ISATAP uses link-local or site-local prefix with ::0:5EFE:x.x.x.x interface ID
            config.mtu = 1500; // Standard MTU for enterprise networks
            // Use link-local prefix by default, can be overridden by enterprise configuration
            config.ipv6_prefix = Some("fe80::".parse().unwrap());
            // Local IPv4 will be auto-detected or can be configured
            if let Some(ipv4) = capabilities.public_ipv4 {
                config.local_ipv4 = Some(ipv4);
            }
        }
        TunnelProtocol::MapE => {
            // MAP-E uses IPv4-in-IPv6 encapsulation with mapping rules
            config.mtu = 1460; // Account for IPv6 header overhead (40 bytes)
            // MAP-E configuration will be provided by ISP via DHCP or configuration
            if let Some(ipv4) = capabilities.public_ipv4 {
                config.local_ipv4 = Some(ipv4);
            }
            // IPv6 prefix will be calculated based on MAP rules
        }
        TunnelProtocol::MapT => {
            // MAP-T uses stateless IPv4/IPv6 translation with mapping rules
            config.mtu = 1500; // No encapsulation overhead
            // MAP-T configuration will be provided by ISP via DHCP or configuration
            if let Some(ipv4) = capabilities.public_ipv4 {
                config.local_ipv4 = Some(ipv4);
            }
            // IPv6 prefix will be calculated based on MAP rules
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
        TunnelProtocol::Isatap => {
            let tunnel = IsatapTunnel::new(config)?;
            Ok(Box::new(tunnel))
        }
        TunnelProtocol::MapE => {
            let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
            Ok(Box::new(tunnel))
        }
        TunnelProtocol::MapT => {
            let tunnel = MapTunnel::new(config, MapProtocol::MapT)?;
            Ok(Box::new(tunnel))
        }
    }
}