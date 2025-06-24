//! MAP-E and MAP-T (Mapping of Address and Port) Implementation
//!
//! This module implements MAP-E (RFC 7597) and MAP-T (RFC 7599) protocols
//! for modern ISP IPv4/IPv6 transition mechanisms. These protocols enable
//! ISPs to provide IPv4 services over IPv6 infrastructure using deterministic
//! address and port mapping.
//!
//! ## Supported Protocols
//!
//! - **MAP-E**: IPv4-in-IPv6 encapsulation with mapping rules
//! - **MAP-T**: Stateless IPv4/IPv6 translation with mapping rules
//!
//! ## Key Features
//!
//! - Deterministic port set allocation per subscriber
//! - IPv4 address sharing across multiple customers
//! - Stateless operation suitable for high-performance ISP deployments
//! - Support for both encapsulation and translation modes
//! - Algorithmic mapping rule processing

use crate::tunneling::{Tunnel, TunnelConfig, TunnelState, TunnelMetrics, TunnelProtocol};
use crate::{Result, P2PError};
use async_trait::async_trait;
use std::net::{Ipv4Addr, Ipv6Addr, IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tracing::{info, warn, debug};
use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};

/// MAP protocol variant
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapProtocol {
    /// MAP-E: IPv4-in-IPv6 encapsulation
    MapE,
    /// MAP-T: IPv4/IPv6 stateless translation
    MapT,
}

/// MAP rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapRule {
    /// IPv6 prefix for this MAP domain
    pub ipv6_prefix: Ipv6Addr,
    /// IPv6 prefix length
    pub ipv6_prefix_len: u8,
    /// IPv4 prefix for address sharing
    pub ipv4_prefix: Ipv4Addr,
    /// IPv4 prefix length
    pub ipv4_prefix_len: u8,
    /// Port parameters
    pub port_params: PortParameters,
    /// Border Relay IPv6 address (MAP-E only)
    pub border_relay: Option<Ipv6Addr>,
    /// Forward Mapping Rule (FMR) vs Basic Mapping Rule (BMR)
    pub is_fmr: bool,
}

/// Port set parameters for MAP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortParameters {
    /// Port Set Identifier (PSID) offset
    pub psid_offset: u8,
    /// Port Set Identifier length
    pub psid_length: u8,
    /// Excluded ports (well-known ports)
    pub excluded_ports: u16,
}

/// Calculated port set for a MAP customer
#[derive(Debug, Clone)]
pub struct PortSet {
    /// Port Set Identifier
    pub psid: u16,
    /// Starting port in the set
    pub start_port: u16,
    /// Number of ports in the set
    pub port_count: u16,
    /// List of available ports
    pub available_ports: Vec<u16>,
}

/// MAP tunnel implementation supporting both MAP-E and MAP-T
pub struct MapTunnel {
    /// MAP protocol variant (MAP-E or MAP-T)
    protocol_variant: MapProtocol,
    /// Tunnel configuration
    config: TunnelConfig,
    /// Current tunnel state
    state: TunnelState,
    /// Performance metrics
    metrics: TunnelMetrics,
    /// Active MAP rules
    map_rules: Vec<MapRule>,
    /// Calculated port set for this CE
    port_set: Option<PortSet>,
    /// Local IPv4 address (customer side)
    local_ipv4: Option<Ipv4Addr>,
    /// Assigned IPv6 address
    assigned_ipv6: Option<Ipv6Addr>,
    /// UDP socket for communication
    socket: Option<UdpSocket>,
    /// Border Relay address (MAP-E)
    border_relay: Option<Ipv6Addr>,
    /// NAT translation table (MAP-T)
    translation_table: HashMap<(Ipv4Addr, u16), (Ipv6Addr, u16)>,
}

impl MapTunnel {
    /// Create a new MAP tunnel
    pub fn new(config: TunnelConfig, protocol_variant: MapProtocol) -> Result<Self> {
        match config.protocol {
            TunnelProtocol::MapE if protocol_variant != MapProtocol::MapE => {
                return Err(P2PError::Config("Protocol mismatch: expected MAP-E".to_string()));
            }
            TunnelProtocol::MapT if protocol_variant != MapProtocol::MapT => {
                return Err(P2PError::Config("Protocol mismatch: expected MAP-T".to_string()));
            }
            TunnelProtocol::MapE | TunnelProtocol::MapT => {
                // Correct protocol match
            }
            _ => {
                return Err(P2PError::Config("Invalid protocol for MAP tunnel".to_string()));
            }
        }

        info!("Creating MAP tunnel: {:?}", protocol_variant);

        Ok(Self {
            protocol_variant,
            config,
            state: TunnelState::Disconnected,
            metrics: TunnelMetrics::default(),
            map_rules: Vec::new(),
            port_set: None,
            local_ipv4: None,
            assigned_ipv6: None,
            socket: None,
            border_relay: None,
            translation_table: HashMap::new(),
        })
    }

    /// Add a MAP rule to the configuration
    pub fn add_map_rule(&mut self, rule: MapRule) {
        info!("Adding MAP rule: IPv6 prefix: {}/{}, IPv4 prefix: {}/{}", 
              rule.ipv6_prefix, rule.ipv6_prefix_len,
              rule.ipv4_prefix, rule.ipv4_prefix_len);
        
        if rule.border_relay.is_some() && self.protocol_variant == MapProtocol::MapE {
            self.border_relay = rule.border_relay;
        }
        
        self.map_rules.push(rule);
    }

    /// Calculate IPv6 address from IPv4 address and MAP rule
    pub fn calculate_ipv6_address(&self, ipv4_addr: Ipv4Addr, rule: &MapRule) -> Result<Ipv6Addr> {
        // Extract the interface identifier from IPv4 address
        let ipv4_bytes = ipv4_addr.octets();
        let ipv4_suffix = u32::from_be_bytes(ipv4_bytes);
        
        // Calculate PSID from IPv4 address
        let host_bits = 32 - rule.ipv4_prefix_len;
        let psid_bits = rule.port_params.psid_length;
        
        if host_bits < psid_bits {
            return Err(P2PError::Config("Invalid MAP rule: insufficient host bits for PSID".to_string()));
        }
        
        // Extract PSID from IPv4 address
        let psid_mask = (1u32 << psid_bits) - 1;
        let psid_shift = host_bits - psid_bits;
        let psid = (ipv4_suffix >> psid_shift) & psid_mask;
        
        // Construct IPv6 address: prefix + IPv4 + PSID
        let prefix_bytes = rule.ipv6_prefix.octets();
        let mut ipv6_bytes = [0u8; 16];
        
        // Copy IPv6 prefix
        let prefix_len = rule.ipv6_prefix_len as usize / 8;
        ipv6_bytes[..prefix_len].copy_from_slice(&prefix_bytes[..prefix_len]);
        
        // Embed IPv4 address
        ipv6_bytes[prefix_len..prefix_len + 4].copy_from_slice(&ipv4_bytes);
        
        // Embed PSID
        if psid_bits > 0 {
            let psid_bytes = (psid as u16).to_be_bytes();
            ipv6_bytes[prefix_len + 4..prefix_len + 6].copy_from_slice(&psid_bytes);
        }
        
        Ok(Ipv6Addr::from(ipv6_bytes))
    }

    /// Calculate port set for a given PSID
    pub fn calculate_port_set(&self, psid: u16, rule: &MapRule) -> PortSet {
        let port_params = &rule.port_params;
        
        // Calculate port range
        let total_ports = 65536u32;
        let excluded_ports = port_params.excluded_ports as u32;
        let ports_per_set = if port_params.psid_length > 0 {
            ((total_ports - excluded_ports) >> port_params.psid_length) as u16
        } else {
            (total_ports - excluded_ports) as u16
        };
        
        let start_port = port_params.excluded_ports + (psid * ports_per_set);
        
        // Generate available ports in the set
        let mut available_ports = Vec::new();
        for i in 0..ports_per_set {
            let port = start_port + i;
            if port > 0 && port < 65535 {
                available_ports.push(port);
            }
        }
        
        PortSet {
            psid,
            start_port,
            port_count: ports_per_set,
            available_ports,
        }
    }

    /// Extract PSID from IPv4 address using MAP rule
    pub fn extract_psid(&self, ipv4_addr: Ipv4Addr, rule: &MapRule) -> u16 {
        let ipv4_bytes = ipv4_addr.octets();
        let ipv4_u32 = u32::from_be_bytes(ipv4_bytes);
        
        let host_bits = 32 - rule.ipv4_prefix_len;
        let psid_bits = rule.port_params.psid_length;
        
        if psid_bits == 0 {
            return 0;
        }
        
        let psid_mask = (1u32 << psid_bits) - 1;
        let psid_shift = host_bits - psid_bits;
        
        ((ipv4_u32 >> psid_shift) & psid_mask) as u16
    }

    /// Initialize local IPv4 address and generate ISATAP address (public for testing)
    pub async fn initialize_addresses(&mut self) -> Result<()> {
        self.initialize_map().await
    }

    /// Initialize MAP configuration
    async fn initialize_map(&mut self) -> Result<()> {
        if self.map_rules.is_empty() {
            return Err(P2PError::Config("No MAP rules configured".to_string()));
        }

        // Use the first rule for initialization (in practice, would select best rule)
        let rule = &self.map_rules[0].clone();
        
        // Get local IPv4 address
        let local_ipv4 = self.get_local_ipv4().await?;
        self.local_ipv4 = Some(local_ipv4);
        
        // Calculate IPv6 address
        let ipv6_addr = self.calculate_ipv6_address(local_ipv4, rule)?;
        self.assigned_ipv6 = Some(ipv6_addr);
        
        // Calculate port set
        let psid = self.extract_psid(local_ipv4, rule);
        let port_set = self.calculate_port_set(psid, rule);
        self.port_set = Some(port_set);
        
        info!("MAP initialization complete: IPv4={}, IPv6={}, PSID={}", 
              local_ipv4, ipv6_addr, psid);
        
        Ok(())
    }

    /// Get local IPv4 address
    async fn get_local_ipv4(&self) -> Result<Ipv4Addr> {
        if let Some(addr) = self.config.local_ipv4 {
            return Ok(addr);
        }

        // Auto-detect local IPv4 address
        match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => {
                if socket.connect("8.8.8.8:53").await.is_ok() {
                    if let Ok(local_addr) = socket.local_addr() {
                        if let IpAddr::V4(ipv4) = local_addr.ip() {
                            return Ok(ipv4);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to detect local IPv4 address: {}", e);
            }
        }

        Err(P2PError::Network("Could not determine local IPv4 address".to_string()))
    }

    /// Create UDP socket for MAP communication
    async fn create_socket(&mut self) -> Result<()> {
        let local_ipv4 = self.local_ipv4.ok_or_else(|| {
            P2PError::Config("Local IPv4 address not initialized".to_string())
        })?;

        let bind_addr = SocketAddr::new(IpAddr::V4(local_ipv4), 0);
        let socket = UdpSocket::bind(bind_addr).await
            .map_err(|e| P2PError::Network(format!("Failed to create MAP socket: {}", e)))?;

        info!("Created MAP socket on: {}", socket.local_addr().unwrap());
        self.socket = Some(socket);
        Ok(())
    }

    /// Encapsulate IPv4 packet in IPv6 for MAP-E
    pub fn encapsulate_ipv4_in_ipv6(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        if self.protocol_variant != MapProtocol::MapE {
            return Err(P2PError::Transport("MAP-E encapsulation requires MAP-E protocol".to_string()));
        }

        if ipv4_packet.len() < 20 {
            return Err(P2PError::Transport("IPv4 packet too short".to_string()));
        }

        let local_ipv6 = self.assigned_ipv6.ok_or_else(|| {
            P2PError::Network("Local IPv6 address not available".to_string())
        })?;

        let border_relay = self.border_relay.ok_or_else(|| {
            P2PError::Config("Border relay not configured".to_string())
        })?;

        // Create IPv6 header for encapsulation
        let mut ipv6_packet = Vec::with_capacity(40 + ipv4_packet.len());
        
        // IPv6 header (40 bytes)
        ipv6_packet.push(0x60); // Version=6, Traffic Class=0
        ipv6_packet.extend_from_slice(&[0x00, 0x00, 0x00]); // Traffic Class + Flow Label
        ipv6_packet.extend_from_slice(&((ipv4_packet.len()) as u16).to_be_bytes()); // Payload Length
        ipv6_packet.push(4); // Next Header = IPv4
        ipv6_packet.push(64); // Hop Limit
        ipv6_packet.extend_from_slice(&local_ipv6.octets()); // Source Address
        ipv6_packet.extend_from_slice(&border_relay.octets()); // Destination Address

        // Append IPv4 payload
        ipv6_packet.extend_from_slice(ipv4_packet);

        Ok(ipv6_packet)
    }

    /// Translate IPv4 packet to IPv6 for MAP-T
    pub fn translate_ipv4_to_ipv6(&mut self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        if self.protocol_variant != MapProtocol::MapT {
            return Err(P2PError::Transport("MAP-T translation requires MAP-T protocol".to_string()));
        }

        if ipv4_packet.len() < 20 {
            return Err(P2PError::Transport("IPv4 packet too short".to_string()));
        }

        // Parse IPv4 header
        let src_ipv4 = Ipv4Addr::from(<[u8; 4]>::try_from(&ipv4_packet[12..16]).unwrap());
        let dst_ipv4 = Ipv4Addr::from(<[u8; 4]>::try_from(&ipv4_packet[16..20]).unwrap());
        
        // Get port information if TCP/UDP
        let protocol = ipv4_packet[9];
        let (src_port, _dst_port) = if protocol == 6 || protocol == 17 { // TCP or UDP
            if ipv4_packet.len() >= 24 {
                let src_port = u16::from_be_bytes([ipv4_packet[20], ipv4_packet[21]]);
                let dst_port = u16::from_be_bytes([ipv4_packet[22], ipv4_packet[23]]);
                (src_port, dst_port)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        // Calculate corresponding IPv6 addresses using MAP rules
        let rule = &self.map_rules[0]; // Use first rule
        let src_ipv6 = self.calculate_ipv6_address(src_ipv4, rule)?;
        let dst_ipv6 = self.calculate_ipv6_address(dst_ipv4, rule)?;

        // Store translation for return traffic
        self.translation_table.insert((src_ipv4, src_port), (src_ipv6, src_port));

        // Create IPv6 packet
        let payload_len = ipv4_packet.len() - 20; // Remove IPv4 header
        let mut ipv6_packet = Vec::with_capacity(40 + payload_len);
        
        // IPv6 header
        ipv6_packet.push(0x60); // Version=6
        ipv6_packet.extend_from_slice(&[0x00, 0x00, 0x00]); // Traffic Class + Flow Label
        ipv6_packet.extend_from_slice(&(payload_len as u16).to_be_bytes()); // Payload Length
        ipv6_packet.push(protocol); // Next Header (same as IPv4 protocol)
        ipv6_packet.push(64); // Hop Limit
        ipv6_packet.extend_from_slice(&src_ipv6.octets()); // Source Address
        ipv6_packet.extend_from_slice(&dst_ipv6.octets()); // Destination Address

        // Append payload (everything after IPv4 header)
        ipv6_packet.extend_from_slice(&ipv4_packet[20..]);

        Ok(ipv6_packet)
    }

    /// Validate port against assigned port set
    pub fn is_port_allowed(&self, port: u16) -> bool {
        if let Some(ref port_set) = self.port_set {
            port_set.available_ports.contains(&port)
        } else {
            false
        }
    }
}

#[async_trait]
impl Tunnel for MapTunnel {
    fn protocol(&self) -> TunnelProtocol {
        match self.protocol_variant {
            MapProtocol::MapE => TunnelProtocol::MapE,
            MapProtocol::MapT => TunnelProtocol::MapT,
        }
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
        info!("Connecting MAP tunnel: {:?}", self.protocol_variant);
        self.state = TunnelState::Connecting;

        // Initialize MAP configuration
        if let Err(e) = self.initialize_map().await {
            self.state = TunnelState::Failed(format!("MAP initialization failed: {}", e));
            return Err(e);
        }

        // Create communication socket
        if let Err(e) = self.create_socket().await {
            self.state = TunnelState::Failed(format!("Socket creation failed: {}", e));
            return Err(e);
        }

        self.state = TunnelState::Connected;
        info!("MAP tunnel connected successfully: {:?}", self.protocol_variant);
        Ok(())
    }

    async fn is_active(&self) -> bool {
        matches!(self.state, TunnelState::Connected)
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting MAP tunnel");
        
        if let Some(socket) = self.socket.take() {
            drop(socket);
        }

        self.state = TunnelState::Disconnected;
        self.translation_table.clear();
        info!("MAP tunnel disconnected");
        Ok(())
    }

    async fn encapsulate(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("MAP tunnel not connected".to_string()));
        }

        match self.protocol_variant {
            MapProtocol::MapE => self.encapsulate_ipv4_in_ipv6(ipv4_packet),
            MapProtocol::MapT => {
                // For MAP-T, we need mutable access for translation table
                Err(P2PError::Transport("MAP-T encapsulation requires mutable access".to_string()))
            }
        }
    }

    async fn decapsulate(&self, packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("MAP tunnel not connected".to_string()));
        }

        match self.protocol_variant {
            MapProtocol::MapE => {
                // For MAP-E, extract IPv4 from IPv6
                if packet.len() < 40 {
                    return Err(P2PError::Transport("IPv6 packet too short".to_string()));
                }
                if packet[6] != 4 { // Next Header must be IPv4
                    return Err(P2PError::Transport("Not an IPv4-in-IPv6 packet".to_string()));
                }
                Ok(packet[40..].to_vec())
            }
            MapProtocol::MapT => {
                // For MAP-T, this would be IPv6-to-IPv4 translation
                Err(P2PError::Transport("MAP-T decapsulation not implemented in immutable context".to_string()))
            }
        }
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        let socket = self.socket.as_ref().ok_or_else(|| {
            P2PError::Network("MAP socket not available".to_string())
        })?;

        // For MAP, determine destination based on protocol variant
        let dest_addr = match self.protocol_variant {
            MapProtocol::MapE => {
                // Send to border relay
                let br = self.border_relay.ok_or_else(|| {
                    P2PError::Config("Border relay not configured".to_string())
                })?;
                SocketAddr::new(IpAddr::V6(br), 0)
            }
            MapProtocol::MapT => {
                // For MAP-T, destination depends on packet content
                return Err(P2PError::Transport("MAP-T send not implemented".to_string()));
            }
        };

        socket.send_to(packet, dest_addr).await
            .map_err(|e| P2PError::Network(format!("Failed to send MAP packet: {}", e)))?;

        self.metrics.packets_sent += 1;
        self.metrics.bytes_sent += packet.len() as u64;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        let socket = self.socket.as_ref().ok_or_else(|| {
            P2PError::Network("MAP socket not available".to_string())
        })?;

        let mut buffer = vec![0u8; 1500];
        let (size, _) = socket.recv_from(&mut buffer).await
            .map_err(|e| P2PError::Network(format!("Failed to receive MAP packet: {}", e)))?;

        buffer.truncate(size);
        self.metrics.packets_received += 1;
        self.metrics.bytes_received += size as u64;
        Ok(buffer)
    }

    async fn maintain(&mut self) -> Result<()> {
        // MAP protocols are generally stateless, minimal maintenance needed
        debug!("MAP tunnel maintenance: translation table size: {}", 
               self.translation_table.len());
        
        // Clean old translation entries (simple timeout-based cleanup)
        if self.translation_table.len() > 10000 {
            warn!("MAP translation table growing large, consider cleanup");
            // In production, implement proper LRU or timeout-based cleanup
        }
        
        Ok(())
    }

    async fn local_ipv6_addr(&self) -> Result<Ipv6Addr> {
        self.assigned_ipv6.ok_or_else(|| {
            P2PError::Network("Local IPv6 address not available".to_string())
        })
    }

    async fn local_ipv4_addr(&self) -> Result<Ipv4Addr> {
        self.local_ipv4.ok_or_else(|| {
            P2PError::Network("Local IPv4 address not available".to_string())
        })
    }

    async fn ping(&mut self, _timeout: Duration) -> Result<Duration> {
        // For MAP protocols, ping would go through the mapping mechanism
        let start = Instant::now();
        
        match self.protocol_variant {
            MapProtocol::MapE => {
                if self.border_relay.is_none() {
                    return Err(P2PError::Network("No border relay configured for ping".to_string()));
                }
                // Would implement actual ping to border relay
            }
            MapProtocol::MapT => {
                // For MAP-T, ping would be translated and sent
                // Would implement actual translated ping
            }
        }
        
        // Simulated ping response
        tokio::time::sleep(Duration::from_millis(20)).await;
        Ok(start.elapsed())
    }
}

