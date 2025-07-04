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

//! ISATAP (Intra-Site Automatic Tunnel Addressing Protocol) Implementation
//!
//! ISATAP provides automatic IPv6 connectivity over IPv4 infrastructure within
//! enterprise networks. This implementation follows RFC 5214 and provides:
//!
//! - Automatic ISATAP address generation
//! - ISATAP router discovery and Potential Router List (PRL) management
//! - IPv6-in-IPv4 tunneling for enterprise environments
//! - Integration with corporate network infrastructure
//!
//! ISATAP is particularly well-suited for:
//! - Enterprise networks with existing IPv4 infrastructure
//! - Corporate environments requiring IPv6 connectivity
//! - P2P networks operating within organizational boundaries
//! - Environments with dedicated ISATAP routers

use crate::tunneling::{Tunnel, TunnelConfig, TunnelState, TunnelMetrics, TunnelProtocol};
use crate::{Result, P2PError};
use async_trait::async_trait;
use std::net::{Ipv4Addr, Ipv6Addr, IpAddr, SocketAddr};
use std::time::{Duration, Instant, SystemTime};
use tracing::{info, warn, debug};
use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};

/// Default ISATAP interface identifier prefix (::0:5EFE:)
const ISATAP_IID_PREFIX: u64 = 0x00005EFE;

/// Default ISATAP router discovery interval
const DEFAULT_ROUTER_DISCOVERY_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

/// Maximum number of ISATAP routers in PRL
const MAX_PRL_SIZE: usize = 10;

/// ISATAP tunnel implementation
/// 
/// Provides IPv6 connectivity over IPv4 infrastructure using ISATAP protocol.
/// Designed for enterprise environments with corporate network integration.
pub struct IsatapTunnel {
    /// Tunnel configuration
    config: TunnelConfig,
    /// Current tunnel state
    state: TunnelState,
    /// Performance and usage metrics
    metrics: TunnelMetrics,
    /// Local IPv4 address for tunnel endpoint
    local_ipv4: Option<Ipv4Addr>,
    /// Generated ISATAP IPv6 address
    isatap_ipv6: Option<Ipv6Addr>,
    /// UDP socket for tunnel communication
    socket: Option<UdpSocket>,
    /// Potential Router List (PRL) for ISATAP routers
    potential_router_list: Vec<IsatapRouter>,
    /// Last router discovery time
    last_router_discovery: Option<Instant>,
    /// Current active ISATAP router
    active_router: Option<IsatapRouter>,
}

/// ISATAP router information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsatapRouter {
    /// Router IPv4 address
    pub ipv4_addr: Ipv4Addr,
    /// Router IPv6 prefix (if known)
    pub ipv6_prefix: Option<Ipv6Addr>,
    /// Router priority (lower = higher priority)
    pub priority: u8,
    /// Last successful communication time
    pub last_seen: Option<SystemTime>,
    /// Router reachability status
    pub reachable: bool,
    /// Round-trip time to router
    pub rtt: Option<Duration>,
}

/// ISATAP router discovery method
#[derive(Debug, Clone)]
pub enum RouterDiscoveryMethod {
    /// Use well-known DNS name (isatap.domain.com)
    DnsWellKnown(String),
    /// Use configured router addresses
    ConfiguredList(Vec<Ipv4Addr>),
    /// Use DHCP option (not implemented)
    Dhcp,
    /// Manual configuration
    Manual(Vec<IsatapRouter>),
}

impl IsatapTunnel {
    /// Create a new ISATAP tunnel
    pub fn new(config: TunnelConfig) -> Result<Self> {
        if config.protocol != TunnelProtocol::Isatap {
            return Err(P2PError::Config(
                "Invalid protocol for ISATAP tunnel".to_string()
            ));
        }

        info!("Creating ISATAP tunnel for enterprise IPv6 connectivity");

        Ok(Self {
            config,
            state: TunnelState::Disconnected,
            metrics: TunnelMetrics::default(),
            local_ipv4: None,
            isatap_ipv6: None,
            socket: None,
            potential_router_list: Vec::new(),
            last_router_discovery: None,
            active_router: None,
        })
    }

    /// Generate ISATAP IPv6 address from IPv4 address
    /// 
    /// ISATAP addresses use the format: prefix::0:5EFE:x.x.x.x
    /// where x.x.x.x is the IPv4 address in hexadecimal
    pub fn generate_isatap_address(ipv4_addr: Ipv4Addr, prefix: Option<Ipv6Addr>) -> Ipv6Addr {
        let prefix = prefix.unwrap_or_else(|| "fe80::".parse().unwrap());
        
        // Convert IPv4 to the ISATAP interface identifier
        let ipv4_bytes = ipv4_addr.octets();
        let ipv4_u32 = u32::from_be_bytes(ipv4_bytes);
        
        // Create ISATAP interface identifier: ::0:5EFE:x.x.x.x
        let iid_high = ISATAP_IID_PREFIX;
        // Interface identifier low part is included in ipv4_u32
        
        // Combine prefix with ISATAP interface identifier
        let prefix_bytes = prefix.octets();
        let mut addr_bytes = [0u8; 16];
        
        // Copy prefix (first 8 bytes)
        addr_bytes[..8].copy_from_slice(&prefix_bytes[..8]);
        
        // Set ISATAP interface identifier
        addr_bytes[8..12].copy_from_slice(&iid_high.to_be_bytes()[4..]);
        addr_bytes[12..16].copy_from_slice(&ipv4_u32.to_be_bytes());
        
        Ipv6Addr::from(addr_bytes)
    }

    /// Validate if an IPv6 address is a valid ISATAP address
    pub fn is_isatap_address(addr: Ipv6Addr) -> bool {
        let segments = addr.segments();
        // Check for ISATAP interface identifier: xxxx:xxxx:xxxx:xxxx:0000:5EFE:xxxx:xxxx
        segments[4] == 0x0000 && segments[5] == 0x5EFE
    }

    /// Extract IPv4 address from ISATAP IPv6 address
    pub fn extract_ipv4_from_isatap(addr: Ipv6Addr) -> Option<Ipv4Addr> {
        if !Self::is_isatap_address(addr) {
            return None;
        }

        let segments = addr.segments();
        let ipv4_bytes = [
            ((segments[6] >> 8) & 0xFF) as u8,
            (segments[6] & 0xFF) as u8,
            ((segments[7] >> 8) & 0xFF) as u8,
            (segments[7] & 0xFF) as u8,
        ];

        Some(Ipv4Addr::from(ipv4_bytes))
    }

    /// Discover ISATAP routers using various methods
    pub async fn discover_routers(&mut self, method: RouterDiscoveryMethod) -> Result<Vec<IsatapRouter>> {
        info!("Discovering ISATAP routers using: {:?}", method);

        let discovered_routers = match method {
            RouterDiscoveryMethod::DnsWellKnown(domain) => {
                self.discover_routers_dns(&domain).await?
            }
            RouterDiscoveryMethod::ConfiguredList(addresses) => {
                self.create_routers_from_addresses(addresses)
            }
            RouterDiscoveryMethod::Manual(routers) => routers,
            RouterDiscoveryMethod::Dhcp => {
                warn!("DHCP router discovery not yet implemented");
                Vec::new()
            }
        };

        // Update PRL with discovered routers
        self.update_potential_router_list(discovered_routers.clone());
        self.last_router_discovery = Some(Instant::now());

        info!("Discovered {} ISATAP routers", discovered_routers.len());
        Ok(discovered_routers)
    }

    /// Discover ISATAP routers via DNS
    async fn discover_routers_dns(&self, domain: &str) -> Result<Vec<IsatapRouter>> {
        let isatap_hostname = format!("isatap.{}", domain);
        debug!("Looking up ISATAP router at: {}", isatap_hostname);

        match tokio::net::lookup_host(format!("{}:80", isatap_hostname)).await {
            Ok(addrs) => {
                let mut routers = Vec::new();
                for addr in addrs {
                    if let IpAddr::V4(ipv4) = addr.ip() {
                        routers.push(IsatapRouter {
                            ipv4_addr: ipv4,
                            ipv6_prefix: None,
                            priority: 10, // Default priority
                            last_seen: None,
                            reachable: false,
                            rtt: None,
                        });
                    }
                }
                Ok(routers)
            }
            Err(e) => {
                warn!("Failed to resolve ISATAP router '{}': {}", isatap_hostname, e);
                Ok(Vec::new())
            }
        }
    }

    /// Create router entries from IPv4 addresses
    fn create_routers_from_addresses(&self, addresses: Vec<Ipv4Addr>) -> Vec<IsatapRouter> {
        addresses.into_iter().enumerate().map(|(i, addr)| {
            IsatapRouter {
                ipv4_addr: addr,
                ipv6_prefix: None,
                priority: i as u8,
                last_seen: None,
                reachable: false,
                rtt: None,
            }
        }).collect()
    }

    /// Update Potential Router List (PRL)
    fn update_potential_router_list(&mut self, new_routers: Vec<IsatapRouter>) {
        // Merge with existing PRL, avoiding duplicates
        for new_router in new_routers {
            if !self.potential_router_list.iter().any(|r| r.ipv4_addr == new_router.ipv4_addr) {
                self.potential_router_list.push(new_router);
            }
        }

        // Sort by priority and limit size
        self.potential_router_list.sort_by_key(|r| r.priority);
        self.potential_router_list.truncate(MAX_PRL_SIZE);

        debug!("Updated PRL with {} routers", self.potential_router_list.len());
    }

    /// Test reachability of ISATAP router
    pub async fn test_router_reachability(&self, router: &IsatapRouter) -> Result<Duration> {
        let target = format!("{}:80", router.ipv4_addr);
        let start = Instant::now();

        match tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(&target)).await {
            Ok(Ok(_)) => {
                let rtt = start.elapsed();
                debug!("ISATAP router {} reachable, RTT: {:?}", router.ipv4_addr, rtt);
                Ok(rtt)
            }
            Ok(Err(e)) => {
                debug!("ISATAP router {} unreachable: {}", router.ipv4_addr, e);
                Err(P2PError::Network(format!("Router unreachable: {}", e)))
            }
            Err(_) => {
                debug!("ISATAP router {} timeout", router.ipv4_addr);
                Err(P2PError::Network("Router reachability timeout".to_string()))
            }
        }
    }

    /// Select best ISATAP router from PRL
    pub async fn select_router(&mut self) -> Result<IsatapRouter> {
        if self.potential_router_list.is_empty() {
            return Err(P2PError::Config("No ISATAP routers available".to_string()));
        }

        info!("Selecting best ISATAP router from {} candidates", self.potential_router_list.len());

        // Test reachability and select best router
        let mut best_router = None;
        let mut best_rtt = Duration::from_secs(u64::MAX);

        // Test each router and collect results
        let mut test_results = Vec::new();
        for (i, router) in self.potential_router_list.iter().enumerate() {
            let rtt_result = self.test_router_reachability(router).await;
            test_results.push((i, rtt_result));
        }

        // Update router states and find best
        for (i, rtt_result) in test_results {
            let router = &mut self.potential_router_list[i];
            match rtt_result {
                Ok(rtt) => {
                    router.reachable = true;
                    router.rtt = Some(rtt);
                    router.last_seen = Some(SystemTime::now());

                    if rtt < best_rtt {
                        best_rtt = rtt;
                        best_router = Some(router.clone());
                    }
                }
                Err(_) => {
                    router.reachable = false;
                    router.rtt = None;
                }
            }
        }

        match best_router {
            Some(router) => {
                info!("Selected ISATAP router: {} (RTT: {:?})", router.ipv4_addr, router.rtt);
                self.active_router = Some(router.clone());
                Ok(router)
            }
            None => Err(P2PError::Network("No reachable ISATAP routers found".to_string()))
        }
    }

    /// Initialize local IPv4 address and generate ISATAP address
    pub async fn initialize_addresses(&mut self) -> Result<()> {
        // Get local IPv4 address
        let local_ipv4 = self.get_local_ipv4().await?;
        self.local_ipv4 = Some(local_ipv4);

        // Generate ISATAP IPv6 address
        let prefix = self.get_ipv6_prefix();
        let isatap_addr = Self::generate_isatap_address(local_ipv4, prefix);
        self.isatap_ipv6 = Some(isatap_addr);

        info!("Initialized ISATAP addresses: IPv4={}, IPv6={}", local_ipv4, isatap_addr);
        Ok(())
    }

    /// Get local IPv4 address for ISATAP tunnel
    async fn get_local_ipv4(&self) -> Result<Ipv4Addr> {
        // Try to get configured local address
        if let Some(addr) = self.config.local_ipv4 {
            return Ok(addr);
        }

        // Auto-detect local IPv4 address by connecting to a remote address
        match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => {
                // Connect to well-known address to determine local interface
                if socket.connect("8.8.8.8:53").await.is_ok() {
                    if let Ok(local_addr) = socket.local_addr() {
                        if let IpAddr::V4(ipv4) = local_addr.ip() {
                            return Ok(ipv4);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to auto-detect local IPv4: {}", e);
            }
        }

        Err(P2PError::Network("Could not determine local IPv4 address".to_string()))
    }

    /// Get IPv6 prefix for ISATAP address generation
    fn get_ipv6_prefix(&self) -> Option<Ipv6Addr> {
        self.config.ipv6_prefix.or_else(|| {
            // Use link-local prefix as default
            Some("fe80::".parse().unwrap())
        })
    }

    /// Create UDP socket for tunnel communication
    async fn create_socket(&mut self) -> Result<()> {
        let local_ipv4 = self.local_ipv4.ok_or_else(|| {
            P2PError::Config("Local IPv4 address not initialized".to_string())
        })?;

        let bind_addr = SocketAddr::new(IpAddr::V4(local_ipv4), 0);
        let socket = UdpSocket::bind(bind_addr).await
            .map_err(|e| P2PError::Network(format!("Failed to create ISATAP socket: {}", e)))?;

        info!("Created ISATAP socket on: {}", socket.local_addr().unwrap());
        self.socket = Some(socket);
        Ok(())
    }

    /// Encapsulate IPv6 packet in IPv4 for ISATAP tunnel
    pub fn encapsulate_ipv6_in_ipv4(&self, ipv6_packet: &[u8], dest_ipv4: Ipv4Addr) -> Result<Vec<u8>> {
        if ipv6_packet.len() < 40 {
            return Err(P2PError::Transport("IPv6 packet too short".to_string()));
        }

        // Verify it's an IPv6 packet
        if (ipv6_packet[0] & 0xF0) != 0x60 {
            return Err(P2PError::Transport("Not an IPv6 packet".to_string()));
        }

        let local_ipv4 = self.local_ipv4.ok_or_else(|| {
            P2PError::Network("Local IPv4 address not available".to_string())
        })?;

        // Create IPv4 header for encapsulation
        let mut ipv4_packet = Vec::with_capacity(20 + ipv6_packet.len());
        
        // IPv4 header (20 bytes)
        ipv4_packet.push(0x45); // Version=4, IHL=5
        ipv4_packet.push(0x00); // Type of Service
        ipv4_packet.extend_from_slice(&((20 + ipv6_packet.len()) as u16).to_be_bytes()); // Total Length
        ipv4_packet.extend_from_slice(&[0x00, 0x00]); // Identification
        ipv4_packet.extend_from_slice(&[0x40, 0x00]); // Flags=DF, Fragment Offset=0
        ipv4_packet.push(64); // TTL
        ipv4_packet.push(41); // Protocol = IPv6
        ipv4_packet.extend_from_slice(&[0x00, 0x00]); // Header Checksum (will calculate)
        ipv4_packet.extend_from_slice(&local_ipv4.octets()); // Source IP
        ipv4_packet.extend_from_slice(&dest_ipv4.octets()); // Destination IP

        // Calculate and set checksum
        let checksum = Self::calculate_ipv4_checksum(&ipv4_packet);
        ipv4_packet[10..12].copy_from_slice(&checksum.to_be_bytes());

        // Append IPv6 payload
        ipv4_packet.extend_from_slice(ipv6_packet);

        Ok(ipv4_packet)
    }

    /// Decapsulate IPv4 packet to extract IPv6 payload
    pub fn decapsulate_ipv4_to_ipv6(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        if ipv4_packet.len() < 20 {
            return Err(P2PError::Transport("IPv4 packet too short".to_string()));
        }

        // Verify IPv4 header
        if (ipv4_packet[0] & 0xF0) != 0x40 {
            return Err(P2PError::Transport("Not an IPv4 packet".to_string()));
        }

        // Check protocol field for IPv6 (41)
        if ipv4_packet[9] != 41 {
            return Err(P2PError::Transport("IPv4 packet does not contain IPv6".to_string()));
        }

        // Extract IPv4 header length
        let ihl = (ipv4_packet[0] & 0x0F) as usize * 4;
        if ipv4_packet.len() <= ihl {
            return Err(P2PError::Transport("IPv4 header length invalid".to_string()));
        }

        // Extract IPv6 payload
        let ipv6_packet = ipv4_packet[ihl..].to_vec();

        // Verify IPv6 packet
        if ipv6_packet.is_empty() || (ipv6_packet[0] & 0xF0) != 0x60 {
            return Err(P2PError::Transport("Invalid IPv6 payload".to_string()));
        }

        Ok(ipv6_packet)
    }

    /// Calculate IPv4 header checksum
    fn calculate_ipv4_checksum(header: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        
        // Sum all 16-bit words in header (except checksum field)
        for i in (0..20).step_by(2) {
            if i == 10 { continue; } // Skip checksum field
            
            let word = if i + 1 < header.len() {
                u16::from_be_bytes([header[i], header[i + 1]])
            } else {
                u16::from_be_bytes([header[i], 0])
            };
            sum += word as u32;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        !sum as u16
    }
}

#[async_trait]
impl Tunnel for IsatapTunnel {
    fn protocol(&self) -> TunnelProtocol {
        TunnelProtocol::Isatap
    }

    fn config(&self) -> &TunnelConfig {
        &self.config
    }

    async fn state(&self) -> TunnelState {
        self.state.clone()
    }

    async fn metrics(&self) -> TunnelMetrics {
        self.metrics.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        info!("Connecting ISATAP tunnel for enterprise IPv6 connectivity");
        self.state = TunnelState::Connecting;

        // Initialize local addresses
        if let Err(e) = self.initialize_addresses().await {
            self.state = TunnelState::Failed(format!("Address initialization failed: {}", e));
            return Err(e);
        }

        // Discover ISATAP routers
        let discovery_method = if let Some(domain) = std::env::var("ISATAP_DOMAIN").ok() {
            RouterDiscoveryMethod::DnsWellKnown(domain)
        } else {
            // Use well-known corporate addresses as fallback
            RouterDiscoveryMethod::ConfiguredList(vec![
                "192.168.1.1".parse().unwrap(),
                "10.0.0.1".parse().unwrap(),
            ])
        };

        if let Err(e) = self.discover_routers(discovery_method).await {
            warn!("Router discovery failed: {}", e);
        }

        // Select best router
        if self.select_router().await.is_err() {
            warn!("No ISATAP router available, operating in host-to-host mode");
        }

        // Create tunnel socket
        if let Err(e) = self.create_socket().await {
            self.state = TunnelState::Failed(format!("Socket creation failed: {}", e));
            return Err(e);
        }

        self.state = TunnelState::Connected;
        info!("ISATAP tunnel connected successfully");
        Ok(())
    }

    async fn is_active(&self) -> bool {
        matches!(self.state, TunnelState::Connected)
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting ISATAP tunnel");
        
        if let Some(socket) = self.socket.take() {
            drop(socket);
        }

        self.state = TunnelState::Disconnected;
        self.active_router = None;
        info!("ISATAP tunnel disconnected");
        Ok(())
    }

    async fn encapsulate(&self, ipv6_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("ISATAP tunnel not connected".to_string()));
        }

        // Determine destination IPv4 address
        let dest_ipv4 = if let Some(router) = &self.active_router {
            router.ipv4_addr
        } else {
            // Try to extract from destination IPv6 address
            if ipv6_packet.len() >= 40 {
                let dest_ipv6_bytes = &ipv6_packet[24..40];
                let dest_ipv6 = Ipv6Addr::from(<[u8; 16]>::try_from(dest_ipv6_bytes).unwrap());
                
                if let Some(ipv4) = Self::extract_ipv4_from_isatap(dest_ipv6) {
                    ipv4
                } else {
                    return Err(P2PError::Network("No route to IPv6 destination".to_string()));
                }
            } else {
                return Err(P2PError::Transport("IPv6 packet too short".to_string()));
            }
        };

        self.encapsulate_ipv6_in_ipv4(ipv6_packet, dest_ipv4)
    }

    async fn decapsulate(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("ISATAP tunnel not connected".to_string()));
        }

        self.decapsulate_ipv4_to_ipv6(ipv4_packet)
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        let socket = self.socket.as_ref().ok_or_else(|| {
            P2PError::Network("ISATAP socket not available".to_string())
        })?;

        // For ISATAP, we need to determine the destination from the packet
        if packet.len() < 20 {
            return Err(P2PError::Transport("Packet too short".to_string()));
        }

        // Extract destination IPv4 address from packet header
        let dest_bytes = &packet[16..20];
        let dest_addr = Ipv4Addr::from(<[u8; 4]>::try_from(dest_bytes).unwrap());
        let dest_port = 41; // IPv6-in-IPv4 doesn't use ports, but UDP requires one

        socket.send_to(packet, (dest_addr, dest_port)).await
            .map_err(|e| P2PError::Network(format!("Failed to send packet: {}", e)))?;

        self.metrics.packets_sent += 1;
        self.metrics.bytes_sent += packet.len() as u64;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        let socket = self.socket.as_ref().ok_or_else(|| {
            P2PError::Network("ISATAP socket not available".to_string())
        })?;

        let mut buffer = vec![0u8; 1500]; // Standard MTU
        let (size, _) = socket.recv_from(&mut buffer).await
            .map_err(|e| P2PError::Network(format!("Failed to receive packet: {}", e)))?;

        buffer.truncate(size);
        self.metrics.packets_received += 1;
        self.metrics.bytes_received += size as u64;
        Ok(buffer)
    }

    async fn maintain(&mut self) -> Result<()> {
        // Periodically rediscover routers
        if let Some(last_discovery) = self.last_router_discovery {
            if last_discovery.elapsed() > DEFAULT_ROUTER_DISCOVERY_INTERVAL {
                debug!("Performing periodic ISATAP router discovery");
                
                let discovery_method = RouterDiscoveryMethod::DnsWellKnown(
                    std::env::var("ISATAP_DOMAIN").unwrap_or_else(|_| "corp.local".to_string())
                );
                
                if let Err(e) = self.discover_routers(discovery_method).await {
                    warn!("Periodic router discovery failed: {}", e);
                }
            }
        }

        // Test active router reachability
        if let Some(router) = &self.active_router {
            if let Err(_) = self.test_router_reachability(router).await {
                warn!("Active ISATAP router {} became unreachable", router.ipv4_addr);
                self.active_router = None;
                
                // Try to select new router
                if let Err(e) = self.select_router().await {
                    warn!("Failed to select new ISATAP router: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn local_ipv6_addr(&self) -> Result<Ipv6Addr> {
        self.isatap_ipv6.ok_or_else(|| {
            P2PError::Network("ISATAP IPv6 address not available".to_string())
        })
    }

    async fn local_ipv4_addr(&self) -> Result<Ipv4Addr> {
        self.local_ipv4.ok_or_else(|| {
            P2PError::Network("Local IPv4 address not available".to_string())
        })
    }

    async fn ping(&mut self, _timeout: Duration) -> Result<Duration> {
        let router = self.active_router.as_ref().ok_or_else(|| {
            P2PError::Network("No active ISATAP router for ping".to_string())
        })?;

        self.test_router_reachability(router).await
    }
}