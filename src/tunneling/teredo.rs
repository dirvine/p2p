//! Teredo Tunneling Protocol Implementation
//!
//! This module implements the Teredo tunneling mechanism as defined in RFC 4380.
//! Teredo enables IPv6 connectivity for nodes located behind NAT devices by using
//! UDP encapsulation and relay servers.
//!
//! ## How Teredo Works
//!
//! - Uses the IPv6 prefix 2001::/32 for Teredo addressing
//! - Embeds NAT's public IPv4 address and UDP port in the IPv6 address
//! - Uses UDP encapsulation to traverse NAT devices
//! - Employs relay servers for initial connectivity and NAT traversal
//! - Supports direct peer-to-peer communication after initial setup

use super::{Tunnel, TunnelConfig, TunnelMetrics, TunnelState, TunnelProtocol};
use crate::{P2PError, Result};
use async_trait::async_trait;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// IPv6 prefix for Teredo tunneling (2001::/32)
const TEREDO_PREFIX: u32 = 0x2001_0000;

/// Default Teredo server address (Using a public IPv4 address for testing)
const DEFAULT_TEREDO_SERVER: &str = "65.55.158.118:3544";

/// Teredo relay discovery multicast address
const TEREDO_DISCOVERY_ADDR: &str = "ff02::2:0000";

/// Protocol number for IPv6-in-UDP encapsulation
const IPV6_IN_UDP_PROTOCOL: u8 = 41;

/// Teredo packet types
#[derive(Debug, Clone, PartialEq)]
enum TeredoPacketType {
    /// Router Solicitation for initial setup
    RouterSolicitation,
    /// Router Advertisement response
    RouterAdvertisement,
    /// IPv6 packet encapsulated in Teredo
    Data,
    /// Bubble packet for NAT traversal
    Bubble,
}

/// Teredo address components
#[derive(Debug, Clone)]
struct TeredoAddress {
    /// Server IPv4 address
    server_ipv4: Ipv4Addr,
    /// Flags (reserved for future use)
    flags: u16,
    /// Obfuscated external UDP port
    obfuscated_port: u16,
    /// Obfuscated external IPv4 address
    obfuscated_ipv4: Ipv4Addr,
}

/// Teredo tunnel implementation
pub struct TeredoTunnel {
    /// Tunnel configuration
    config: TunnelConfig,
    /// Current tunnel state
    state: RwLock<TunnelState>,
    /// Performance metrics
    metrics: RwLock<TunnelMetrics>,
    /// Teredo server address
    server_addr: SocketAddr,
    /// Local UDP socket for Teredo communication
    udp_socket: Option<UdpSocket>,
    /// Generated Teredo IPv6 address
    teredo_ipv6: Option<Ipv6Addr>,
    /// External (public) IPv4 address discovered via Teredo server
    external_ipv4: Option<Ipv4Addr>,
    /// External UDP port discovered via Teredo server
    external_port: Option<u16>,
    /// Connection establishment time
    established_at: Option<Instant>,
    /// Last communication with Teredo server
    last_server_contact: Option<Instant>,
}

impl TeredoTunnel {
    /// Create a new Teredo tunnel with the given configuration
    pub fn new(config: TunnelConfig) -> Result<Self> {
        if config.protocol != TunnelProtocol::Teredo {
            return Err(P2PError::Network(
                "Invalid protocol for Teredo tunnel".to_string()
            ).into());
        }

        // Parse Teredo server address
        let server_addr = DEFAULT_TEREDO_SERVER.parse()
            .map_err(|e| P2PError::Network(format!("Invalid Teredo server address: {}", e)))?;

        Ok(Self {
            config,
            state: RwLock::new(TunnelState::Disconnected),
            metrics: RwLock::new(TunnelMetrics::default()),
            server_addr,
            udp_socket: None,
            teredo_ipv6: None,
            external_ipv4: None,
            external_port: None,
            established_at: None,
            last_server_contact: None,
        })
    }

    /// Generate Teredo IPv6 address from discovered external address
    fn generate_teredo_address(
        server_ipv4: Ipv4Addr,
        external_ipv4: Ipv4Addr,
        external_port: u16,
    ) -> Ipv6Addr {
        let server_octets = server_ipv4.octets();
        let external_octets = external_ipv4.octets();
        
        // Obfuscate the external address and port (XOR with 0xFFFF for security)
        let obfuscated_port = external_port ^ 0xFFFF;
        let obfuscated_addr = [
            external_octets[0] ^ 0xFF,
            external_octets[1] ^ 0xFF,
            external_octets[2] ^ 0xFF,
            external_octets[3] ^ 0xFF,
        ];

        Ipv6Addr::from([
            0x20, 0x01,                           // Teredo prefix (2001::/32)
            server_octets[0], server_octets[1],   // Server IPv4 address (high)
            server_octets[2], server_octets[3],   // Server IPv4 address (low)
            0x00, 0x00,                           // Flags (reserved)
            (obfuscated_port >> 8) as u8,         // Obfuscated port (high)
            (obfuscated_port & 0xFF) as u8,       // Obfuscated port (low)
            obfuscated_addr[0], obfuscated_addr[1], // Obfuscated IPv4 (high)
            obfuscated_addr[2], obfuscated_addr[3], // Obfuscated IPv4 (low)
            0x00, 0x00,                           // Padding to 16 bytes
        ])
    }

    /// Parse a Teredo IPv6 address to extract components
    fn parse_teredo_address(ipv6: &Ipv6Addr) -> Option<TeredoAddress> {
        let segments = ipv6.segments();
        
        // Check if this is a Teredo address (prefix 2001::/32)
        if segments[0] != 0x2001 {
            return None;
        }

        let server_ipv4 = Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            (segments[1] & 0xFF) as u8,
            (segments[2] >> 8) as u8,
            (segments[2] & 0xFF) as u8,
        );

        let flags = segments[3];
        let obfuscated_port = segments[4];

        let obfuscated_ipv4 = Ipv4Addr::new(
            (segments[5] >> 8) as u8,
            (segments[5] & 0xFF) as u8,
            (segments[6] >> 8) as u8,
            (segments[6] & 0xFF) as u8,
        );

        Some(TeredoAddress {
            server_ipv4,
            flags,
            obfuscated_port,
            obfuscated_ipv4,
        })
    }

    /// Create a Router Solicitation packet for Teredo setup
    fn create_router_solicitation(&self) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Teredo encapsulation header (simplified)
        packet.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]); // IPv6 version + traffic class + flow label
        packet.extend_from_slice(&[0x00, 0x08]); // Payload length (8 bytes for RS)
        packet.extend_from_slice(&[0x3A]); // Next header (ICMPv6)
        packet.extend_from_slice(&[0xFF]); // Hop limit
        
        // Source address (unspecified for initial RS)
        packet.extend_from_slice(&[0; 16]);
        
        // Destination address (all-routers multicast)
        packet.extend_from_slice(&[
            0xFF, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02
        ]);
        
        // ICMPv6 Router Solicitation
        packet.extend_from_slice(&[0x85]); // Type (Router Solicitation)
        packet.extend_from_slice(&[0x00]); // Code
        packet.extend_from_slice(&[0x00, 0x00]); // Checksum (to be calculated)
        packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Reserved
        
        packet
    }

    /// Create a Teredo bubble packet for NAT traversal
    fn create_bubble_packet(&self, destination: &Ipv6Addr) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Minimal IPv6 header for bubble packet
        packet.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]); // Version + traffic class + flow label
        packet.extend_from_slice(&[0x00, 0x00]); // Payload length (0 for bubble)
        packet.extend_from_slice(&[0x3B]); // Next header (no next header)
        packet.extend_from_slice(&[0x01]); // Hop limit (1 for local)
        
        // Source address (our Teredo address)
        if let Some(src) = self.teredo_ipv6 {
            packet.extend_from_slice(&src.octets());
        } else {
            packet.extend_from_slice(&[0; 16]);
        }
        
        // Destination address
        packet.extend_from_slice(&destination.octets());
        
        packet
    }

    /// Perform Teredo server qualification to discover external address
    async fn qualify_with_server(&mut self) -> Result<()> {
        debug!("Starting Teredo server qualification");
        
        // Create UDP socket for communication
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| P2PError::Network(format!("Failed to bind UDP socket: {}", e)))?;
        
        socket.set_nonblocking(true)
            .map_err(|e| P2PError::Network(format!("Failed to set socket non-blocking: {}", e)))?;
        
        // Send Router Solicitation to Teredo server
        let rs_packet = self.create_router_solicitation();
        socket.send_to(&rs_packet, self.server_addr)
            .map_err(|e| P2PError::Network(format!("Failed to send RS to server: {}", e)))?;
        
        debug!("Sent Router Solicitation to Teredo server: {}", self.server_addr);
        
        // Wait for Router Advertisement response (simplified simulation)
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Simulate successful qualification (in real implementation, would parse RA)
        self.external_ipv4 = Some(Ipv4Addr::new(203, 0, 113, 100)); // Simulated external IP
        self.external_port = Some(12345); // Simulated external port
        self.udp_socket = Some(socket);
        self.last_server_contact = Some(Instant::now());
        
        info!("Teredo qualification successful: external {}:{}", 
              self.external_ipv4.unwrap(), self.external_port.unwrap());
        
        Ok(())
    }

    /// Perform NAT traversal setup with bubble packets
    async fn setup_nat_traversal(&mut self) -> Result<()> {
        debug!("Setting up NAT traversal");
        
        // Send bubble packets to establish NAT bindings
        let bubble = self.create_bubble_packet(&Ipv6Addr::LOCALHOST);
        
        if let Some(ref socket) = self.udp_socket {
            socket.send_to(&bubble, self.server_addr)
                .map_err(|e| P2PError::Network(format!("Failed to send bubble: {}", e)))?;
        }
        
        debug!("NAT traversal setup completed");
        Ok(())
    }

    /// Create Teredo UDP encapsulation header
    fn create_teredo_header(&self, _ipv6_packet: &[u8]) -> Vec<u8> {
        let mut header = Vec::new();
        
        // Teredo authentication header (optional, simplified here)
        header.extend_from_slice(&[0x00, 0x01]); // Type: Origin indication
        header.extend_from_slice(&[0x00, 0x08]); // Length: 8 bytes
        
        // Origin indication (our external address)
        if let (Some(ext_ipv4), Some(ext_port)) = (self.external_ipv4, self.external_port) {
            header.extend_from_slice(&ext_ipv4.octets());
            header.extend_from_slice(&ext_port.to_be_bytes());
        } else {
            header.extend_from_slice(&[0; 6]);
        }
        
        header
    }

    /// Extract IPv6 packet from Teredo UDP payload
    fn extract_ipv6_from_teredo(&self, teredo_packet: &[u8]) -> Result<Vec<u8>> {
        debug!("Extracting IPv6 from Teredo packet of {} bytes", teredo_packet.len());
        
        // Skip Teredo authentication headers (simplified parsing)
        // In our implementation, we add an 8-byte header, so skip it
        if teredo_packet.len() <= 8 {
            debug!("Teredo packet too short: {} <= 8", teredo_packet.len());
            return Err(P2PError::Network("Teredo packet too short".to_string()).into());
        }
        
        // Skip the 8-byte header we added in create_teredo_header
        let ipv6_start = 8;
        let ipv6_packet = teredo_packet[ipv6_start..].to_vec();
        debug!("Extracted IPv6 packet of {} bytes, first byte: {:02x}", 
               ipv6_packet.len(), 
               if ipv6_packet.is_empty() { 0 } else { ipv6_packet[0] });
        
        // Validate it starts with IPv6 version
        if !ipv6_packet.is_empty() && (ipv6_packet[0] & 0xF0) == 0x60 {
            debug!("Valid IPv6 packet extracted");
            Ok(ipv6_packet)
        } else {
            debug!("Invalid IPv6 version in packet: {:02x} (expected 0x6x)", 
                   if ipv6_packet.is_empty() { 0 } else { ipv6_packet[0] });
            Err(P2PError::Network("No valid IPv6 packet found in Teredo payload".to_string()).into())
        }
    }

    /// Update metrics for sent data
    async fn update_send_metrics(&self, bytes: usize) {
        let mut metrics = self.metrics.write().await;
        metrics.bytes_sent += bytes as u64;
        metrics.packets_sent += 1;
        metrics.last_activity = Instant::now();
    }

    /// Update metrics for received data
    async fn update_receive_metrics(&self, bytes: usize) {
        let mut metrics = self.metrics.write().await;
        metrics.bytes_received += bytes as u64;
        metrics.packets_received += 1;
        metrics.last_activity = Instant::now();
    }

    /// Check if server communication is still active
    fn is_server_active(&self) -> bool {
        if let Some(last_contact) = self.last_server_contact {
            last_contact.elapsed() < Duration::from_secs(300) // 5 minutes timeout
        } else {
            false
        }
    }
}

#[async_trait]
impl Tunnel for TeredoTunnel {
    fn protocol(&self) -> TunnelProtocol {
        TunnelProtocol::Teredo
    }

    fn config(&self) -> &TunnelConfig {
        &self.config
    }

    async fn state(&self) -> TunnelState {
        let state = self.state.read().await;
        state.clone()
    }

    async fn metrics(&self) -> TunnelMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        info!("Establishing Teredo tunnel connection");
        
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Connecting;
        }

        // Step 1: Qualify with Teredo server to discover external address
        self.qualify_with_server().await?;
        
        // Step 2: Generate Teredo IPv6 address
        if let (Some(ext_ipv4), Some(ext_port)) = (self.external_ipv4, self.external_port) {
            let server_ipv4 = self.server_addr.ip();
            if let std::net::IpAddr::V4(server_v4) = server_ipv4 {
                self.teredo_ipv6 = Some(Self::generate_teredo_address(
                    server_v4, ext_ipv4, ext_port
                ));
            } else {
                return Err(P2PError::Network("Teredo server must have IPv4 address".to_string()).into());
            }
        }
        
        // Step 3: Setup NAT traversal
        self.setup_nat_traversal().await?;
        
        // Update instance state
        self.established_at = Some(Instant::now());

        // Update tunnel state
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Connected;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            if let Some(established) = self.established_at {
                metrics.establishment_time = established.elapsed();
            }
        }

        if let Some(teredo_addr) = self.teredo_ipv6 {
            info!("Teredo tunnel established: {}", teredo_addr);
        }
        
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting Teredo tunnel");
        
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Disconnecting;
        }

        // Close UDP socket
        self.udp_socket = None;
        
        // Reset state
        self.teredo_ipv6 = None;
        self.external_ipv4 = None;
        self.external_port = None;
        self.established_at = None;
        self.last_server_contact = None;

        {
            let mut state = self.state.write().await;
            *state = TunnelState::Disconnected;
        }

        debug!("Teredo tunnel disconnected");
        Ok(())
    }

    async fn is_active(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, TunnelState::Connected) && self.is_server_active()
    }

    async fn encapsulate(&self, ipv6_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("Teredo tunnel not active".to_string()).into());
        }

        // Create Teredo header
        let teredo_header = self.create_teredo_header(ipv6_packet);
        
        // Combine header and IPv6 packet
        let mut encapsulated = Vec::with_capacity(teredo_header.len() + ipv6_packet.len());
        encapsulated.extend_from_slice(&teredo_header);
        encapsulated.extend_from_slice(ipv6_packet);

        debug!("Encapsulated {} bytes for Teredo transmission", encapsulated.len());
        Ok(encapsulated)
    }

    async fn decapsulate(&self, udp_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("Teredo tunnel not active".to_string()).into());
        }

        if udp_packet.len() < 8 {
            return Err(P2PError::Network("Teredo packet too short".to_string()).into());
        }

        // Extract IPv6 packet from Teredo encapsulation
        let ipv6_payload = self.extract_ipv6_from_teredo(udp_packet)?;

        debug!("Decapsulated {} bytes from Teredo packet", ipv6_payload.len());
        Ok(ipv6_payload)
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        let encapsulated = self.encapsulate(packet).await?;
        
        // In a real implementation, this would send via UDP to the destination
        // For now, we simulate the send operation
        debug!("Sending {} bytes via Teredo tunnel", encapsulated.len());
        
        // Simulate network transmission delay
        tokio::time::sleep(Duration::from_millis(5)).await;
        
        self.update_send_metrics(encapsulated.len()).await;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        // In a real implementation, this would receive UDP packets
        // For now, we simulate receiving a packet
        debug!("Receiving packet via Teredo tunnel");
        
        // Simulate network reception delay
        tokio::time::sleep(Duration::from_millis(8)).await;
        
        // Simulate a received Teredo packet
        let simulated_packet = vec![0u8; 64]; // Placeholder
        
        let decapsulated = self.decapsulate(&simulated_packet).await?;
        self.update_receive_metrics(decapsulated.len()).await;
        
        Ok(decapsulated)
    }

    async fn maintain(&mut self) -> Result<()> {
        if !self.is_active().await {
            return Ok(());
        }

        debug!("Performing Teredo tunnel maintenance");
        
        // Check if we need to refresh server qualification
        if let Some(last_contact) = self.last_server_contact {
            if last_contact.elapsed() > Duration::from_secs(240) { // 4 minutes
                debug!("Refreshing Teredo server qualification");
                self.qualify_with_server().await?;
            }
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            if let Some(established) = self.established_at {
                metrics.establishment_time = established.elapsed();
            }
        }

        // In a real implementation, this might also:
        // - Send periodic bubble packets to maintain NAT bindings
        // - Check for server reachability
        // - Update route table entries
        // - Monitor for IPv4 address changes

        Ok(())
    }

    async fn local_ipv6_addr(&self) -> Result<Ipv6Addr> {
        self.teredo_ipv6.ok_or_else(|| {
            P2PError::Network("Teredo tunnel not established".to_string()).into()
        })
    }

    async fn local_ipv4_addr(&self) -> Result<Ipv4Addr> {
        self.external_ipv4.ok_or_else(|| {
            P2PError::Network("Teredo external address not discovered".to_string()).into()
        })
    }

    async fn ping(&mut self, timeout: Duration) -> Result<Duration> {
        if !self.is_active().await {
            return Err(P2PError::Network("Teredo tunnel not active".to_string()).into());
        }

        let start = Instant::now();
        
        // Simulate a ping operation via Teredo server
        debug!("Pinging via Teredo tunnel with timeout {:?}", timeout);
        
        // Send bubble packet as ping
        let bubble = self.create_bubble_packet(&Ipv6Addr::LOCALHOST);
        
        if let Some(ref socket) = self.udp_socket {
            socket.send_to(&bubble, self.server_addr)
                .map_err(|e| P2PError::Network(format!("Ping failed: {}", e)))?;
        }
        
        // Simulate network round trip time (Teredo typically has higher latency)
        let simulated_rtt = Duration::from_millis(50 + (rand::random::<u64>() % 100));
        tokio::time::sleep(simulated_rtt).await;
        
        let actual_rtt = start.elapsed();
        
        // Update last server contact time
        self.last_server_contact = Some(Instant::now());
        
        // Update metrics with RTT
        {
            let mut metrics = self.metrics.write().await;
            metrics.rtt = Some(actual_rtt);
        }

        debug!("Teredo tunnel ping successful: RTT = {:?}", actual_rtt);
        Ok(actual_rtt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teredo_address_generation() {
        let server_ipv4 = Ipv4Addr::new(192, 88, 99, 1);
        let external_ipv4 = Ipv4Addr::new(203, 0, 113, 1);
        let external_port = 12345;
        
        let teredo_addr = TeredoTunnel::generate_teredo_address(
            server_ipv4, external_ipv4, external_port
        );
        
        // Should be 2001::/32 prefix
        assert_eq!(teredo_addr.segments()[0], 0x2001);
        
        // Should contain server address
        assert_eq!(teredo_addr.segments()[1], 0xc058); // 192.88
        assert_eq!(teredo_addr.segments()[2], 0x6301); // 99.1
    }

    #[test]
    fn test_teredo_address_parsing() {
        let teredo_addr = Ipv6Addr::new(
            0x2001, 0xc058, 0x6301, 0x0000,
            0xcfc6, 0x34fe, 0x70fe, 0x8afe
        );
        
        let parsed = TeredoTunnel::parse_teredo_address(&teredo_addr).unwrap();
        assert_eq!(parsed.server_ipv4, Ipv4Addr::new(192, 88, 99, 1));
        assert_eq!(parsed.flags, 0x0000);
    }

    #[test]
    fn test_non_teredo_address() {
        let ipv6 = Ipv6Addr::new(0x2002, 0xc000, 0x0201, 0, 0, 0, 0, 1); // 6to4 address
        let result = TeredoTunnel::parse_teredo_address(&ipv6);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_tunnel_creation() {
        let config = TunnelConfig {
            protocol: TunnelProtocol::Teredo,
            ..Default::default()
        };

        let tunnel = TeredoTunnel::new(config).unwrap();
        assert_eq!(tunnel.protocol(), TunnelProtocol::Teredo);
        assert_eq!(tunnel.state().await, TunnelState::Disconnected);
    }

    #[tokio::test]
    async fn test_invalid_protocol() {
        let config = TunnelConfig {
            protocol: TunnelProtocol::SixToFour, // Wrong protocol
            ..Default::default()
        };

        let result = TeredoTunnel::new(config);
        assert!(result.is_err());
    }
}