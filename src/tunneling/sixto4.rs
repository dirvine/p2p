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

//! 6to4 Tunneling Protocol Implementation
//!
//! This module implements the 6to4 automatic tunneling mechanism as defined in RFC 3056.
//! 6to4 allows IPv6 packets to be transmitted over an IPv4 network without explicit tunnel setup.
//!
//! ## How 6to4 Works
//!
//! - Uses the IPv6 prefix 2002::/16 for automatic tunneling
//! - Embeds the IPv4 address in the IPv6 address format: 2002:WWXX:YYZZ::/48
//! - Automatically encapsulates IPv6 packets in IPv4 headers
//! - Provides bidirectional IPv6 connectivity over IPv4 infrastructure

use super::{Tunnel, TunnelConfig, TunnelMetrics, TunnelState, TunnelProtocol};
use crate::{P2PError, Result};
use async_trait::async_trait;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// IPv6 prefix for 6to4 tunneling (2002::/16)
const SIXTO4_PREFIX: u16 = 0x2002;

/// Protocol number for IPv6-in-IPv4 encapsulation (41)
const IPV6_IN_IPV4_PROTOCOL: u8 = 41;

/// 6to4 tunnel implementation
pub struct SixToFourTunnel {
    /// Tunnel configuration
    config: TunnelConfig,
    /// Current tunnel state
    state: RwLock<TunnelState>,
    /// Performance metrics
    metrics: RwLock<TunnelMetrics>,
    /// Local IPv4 address used for tunneling
    local_ipv4: Option<Ipv4Addr>,
    /// Generated IPv6 address for this tunnel
    local_ipv6: Option<Ipv6Addr>,
    /// Connection establishment time
    established_at: Option<Instant>,
}

impl SixToFourTunnel {
    /// Create a new 6to4 tunnel with the given configuration
    pub fn new(config: TunnelConfig) -> Result<Self> {
        if config.protocol != TunnelProtocol::SixToFour {
            return Err(P2PError::Network(
                "Invalid protocol for 6to4 tunnel".to_string()
            ).into());
        }

        Ok(Self {
            config,
            state: RwLock::new(TunnelState::Disconnected),
            metrics: RwLock::new(TunnelMetrics::default()),
            local_ipv4: None,
            local_ipv6: None,
            established_at: None,
        })
    }

    /// Generate a 6to4 IPv6 address from an IPv4 address
    fn generate_ipv6_address(ipv4: Ipv4Addr) -> Ipv6Addr {
        let octets = ipv4.octets();
        Ipv6Addr::from([
            (SIXTO4_PREFIX >> 8) as u8,  // 0x20
            (SIXTO4_PREFIX & 0xFF) as u8, // 0x02
            octets[0], octets[1],         // First two octets of IPv4
            octets[2], octets[3],         // Last two octets of IPv4
            0x00, 0x00,                   // Subnet ID (can be configured)
            0x00, 0x00, 0x00, 0x00,       // Interface ID
            0x00, 0x00, 0x00, 0x01,       // Interface ID (::1)
        ])
    }

    /// Extract IPv4 destination from a 6to4 IPv6 address
    fn extract_ipv4_destination(ipv6: &Ipv6Addr) -> Option<Ipv4Addr> {
        let segments = ipv6.segments();
        
        // Check if this is a 6to4 address (prefix 2002::/16)
        if segments[0] != SIXTO4_PREFIX {
            return None;
        }

        // Extract IPv4 address from segments 1 and 2
        let octet1 = (segments[1] >> 8) as u8;
        let octet2 = (segments[1] & 0xFF) as u8;
        let octet3 = (segments[2] >> 8) as u8;
        let octet4 = (segments[2] & 0xFF) as u8;

        Some(Ipv4Addr::new(octet1, octet2, octet3, octet4))
    }

    /// Create an IPv4 header for encapsulation
    fn create_ipv4_header(&self, dst_ipv4: Ipv4Addr, payload_len: usize) -> Vec<u8> {
        let mut header = vec![0u8; 20]; // Standard IPv4 header size

        // Version (4) and Header Length (5 * 4 = 20 bytes)
        header[0] = 0x45;
        
        // Type of Service
        header[1] = 0x00;
        
        // Total Length (header + payload)
        let total_len = 20 + payload_len;
        header[2] = (total_len >> 8) as u8;
        header[3] = (total_len & 0xFF) as u8;
        
        // Identification (can be random)
        header[4] = 0x00;
        header[5] = 0x01;
        
        // Flags and Fragment Offset
        header[6] = 0x40; // Don't Fragment bit set
        header[7] = 0x00;
        
        // Time to Live
        header[8] = 64;
        
        // Protocol (IPv6-in-IPv4)
        header[9] = IPV6_IN_IPV4_PROTOCOL;
        
        // Header Checksum (will be calculated)
        header[10] = 0x00;
        header[11] = 0x00;
        
        // Source IPv4 Address
        if let Some(src_ipv4) = self.local_ipv4 {
            let src_octets = src_ipv4.octets();
            header[12..16].copy_from_slice(&src_octets);
        }
        
        // Destination IPv4 Address
        let dst_octets = dst_ipv4.octets();
        header[16..20].copy_from_slice(&dst_octets);
        
        // Calculate checksum
        let checksum = self.calculate_ipv4_checksum(&header);
        header[10] = (checksum >> 8) as u8;
        header[11] = (checksum & 0xFF) as u8;

        header
    }

    /// Calculate IPv4 header checksum
    fn calculate_ipv4_checksum(&self, header: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        
        // Sum all 16-bit words in the header
        for i in (0..header.len()).step_by(2) {
            if i + 1 < header.len() {
                let word = ((header[i] as u32) << 8) + (header[i + 1] as u32);
                sum = sum.wrapping_add(word);
            }
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        // One's complement
        (!sum) as u16
    }

    /// Parse IPv6 packet to extract destination address
    fn parse_ipv6_destination(&self, packet: &[u8]) -> Option<Ipv6Addr> {
        if packet.len() < 40 { // Minimum IPv6 header size
            return None;
        }

        // IPv6 destination address is at bytes 24-39
        let mut addr_bytes = [0u8; 16];
        addr_bytes.copy_from_slice(&packet[24..40]);
        Some(Ipv6Addr::from(addr_bytes))
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
}

#[async_trait]
impl Tunnel for SixToFourTunnel {
    fn protocol(&self) -> TunnelProtocol {
        TunnelProtocol::SixToFour
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
        info!("Establishing 6to4 tunnel connection");
        
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Connecting;
        }

        // For 6to4, we need a public IPv4 address
        let local_ipv4 = self.config.local_ipv4.ok_or_else(|| {
            P2PError::Network("6to4 requires a local IPv4 address".to_string())
        })?;

        // Generate the corresponding IPv6 address
        let local_ipv6 = Self::generate_ipv6_address(local_ipv4);

        // Update instance state
        self.local_ipv4 = Some(local_ipv4);
        self.local_ipv6 = Some(local_ipv6);
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

        info!("6to4 tunnel established: {} -> {}", local_ipv4, local_ipv6);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting 6to4 tunnel");
        
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Disconnecting;
        }

        // Reset state
        self.local_ipv4 = None;
        self.local_ipv6 = None;
        self.established_at = None;

        {
            let mut state = self.state.write().await;
            *state = TunnelState::Disconnected;
        }

        debug!("6to4 tunnel disconnected");
        Ok(())
    }

    async fn is_active(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, TunnelState::Connected)
    }

    async fn encapsulate(&self, ipv6_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("6to4 tunnel not active".to_string()).into());
        }

        // Parse the IPv6 packet to get the destination
        let ipv6_dst = self.parse_ipv6_destination(ipv6_packet)
            .ok_or_else(|| P2PError::Network("Invalid IPv6 packet".to_string()))?;

        // Extract IPv4 destination from 6to4 address
        let ipv4_dst = Self::extract_ipv4_destination(&ipv6_dst)
            .ok_or_else(|| P2PError::Network("Destination is not a 6to4 address".to_string()))?;

        // Create IPv4 header
        let ipv4_header = self.create_ipv4_header(ipv4_dst, ipv6_packet.len());

        // Combine header and payload
        let mut encapsulated = Vec::with_capacity(ipv4_header.len() + ipv6_packet.len());
        encapsulated.extend_from_slice(&ipv4_header);
        encapsulated.extend_from_slice(ipv6_packet);

        debug!("Encapsulated {} bytes for 6to4 transmission to {}", 
               encapsulated.len(), ipv4_dst);

        Ok(encapsulated)
    }

    async fn decapsulate(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("6to4 tunnel not active".to_string()).into());
        }

        if ipv4_packet.len() < 20 {
            return Err(P2PError::Network("IPv4 packet too short".to_string()).into());
        }

        // Check if this is an IPv6-in-IPv4 packet
        if ipv4_packet[9] != IPV6_IN_IPV4_PROTOCOL {
            return Err(P2PError::Network("Not an IPv6-in-IPv4 packet".to_string()).into());
        }

        // Extract header length (in 4-byte words)
        let header_len = ((ipv4_packet[0] & 0x0F) * 4) as usize;
        
        if ipv4_packet.len() <= header_len {
            return Err(P2PError::Network("IPv4 packet has no payload".to_string()).into());
        }

        // Extract the IPv6 payload
        let ipv6_payload = ipv4_packet[header_len..].to_vec();

        debug!("Decapsulated {} bytes from 6to4 packet", ipv6_payload.len());

        Ok(ipv6_payload)
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        let encapsulated = self.encapsulate(packet).await?;
        
        // In a real implementation, this would send the packet via raw sockets
        // For now, we simulate the send operation
        debug!("Sending {} bytes via 6to4 tunnel", encapsulated.len());
        
        // Simulate network transmission delay
        tokio::time::sleep(Duration::from_millis(1)).await;
        
        self.update_send_metrics(encapsulated.len()).await;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        // In a real implementation, this would receive packets from raw sockets
        // For now, we simulate receiving a packet
        debug!("Receiving packet via 6to4 tunnel");
        
        // Simulate network reception delay
        tokio::time::sleep(Duration::from_millis(2)).await;
        
        // Simulate a received IPv4 packet containing IPv6 data
        let simulated_packet = vec![0u8; 64]; // Placeholder
        
        let decapsulated = self.decapsulate(&simulated_packet).await?;
        self.update_receive_metrics(decapsulated.len()).await;
        
        Ok(decapsulated)
    }

    async fn maintain(&mut self) -> Result<()> {
        if !self.is_active().await {
            return Ok(());
        }

        debug!("Performing 6to4 tunnel maintenance");
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            if let Some(established) = self.established_at {
                metrics.establishment_time = established.elapsed();
            }
        }

        // In a real implementation, this might:
        // - Check tunnel connectivity
        // - Refresh route table entries
        // - Update firewall rules
        // - Monitor for IPv4 address changes

        Ok(())
    }

    async fn local_ipv6_addr(&self) -> Result<Ipv6Addr> {
        self.local_ipv6.ok_or_else(|| {
            P2PError::Network("6to4 tunnel not established".to_string()).into()
        })
    }

    async fn local_ipv4_addr(&self) -> Result<Ipv4Addr> {
        self.local_ipv4.ok_or_else(|| {
            P2PError::Network("6to4 tunnel not established".to_string()).into()
        })
    }

    async fn ping(&mut self, timeout: Duration) -> Result<Duration> {
        if !self.is_active().await {
            return Err(P2PError::Network("6to4 tunnel not active".to_string()).into());
        }

        let start = Instant::now();
        
        // Simulate a ping operation
        debug!("Pinging via 6to4 tunnel with timeout {:?}", timeout);
        
        // Simulate network round trip time
        let simulated_rtt = Duration::from_millis(20 + (rand::random::<u64>() % 30));
        tokio::time::sleep(simulated_rtt).await;
        
        let actual_rtt = start.elapsed();
        
        // Update metrics with RTT
        {
            let mut metrics = self.metrics.write().await;
            metrics.rtt = Some(actual_rtt);
        }

        debug!("6to4 tunnel ping successful: RTT = {:?}", actual_rtt);
        Ok(actual_rtt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv6_address_generation() {
        let ipv4 = Ipv4Addr::new(192, 168, 1, 1);
        let ipv6 = SixToFourTunnel::generate_ipv6_address(ipv4);
        
        // Should be 2002:c0a8:0101::1
        assert_eq!(ipv6.segments()[0], 0x2002);
        assert_eq!(ipv6.segments()[1], 0xc0a8);  // 192.168
        assert_eq!(ipv6.segments()[2], 0x0101);  // 1.1
        assert_eq!(ipv6.segments()[7], 0x0001);  // ::1
    }

    #[test]
    fn test_ipv4_extraction() {
        let ipv6 = Ipv6Addr::new(0x2002, 0xc0a8, 0x0101, 0, 0, 0, 0, 1);
        let ipv4 = SixToFourTunnel::extract_ipv4_destination(&ipv6).unwrap();
        
        assert_eq!(ipv4, Ipv4Addr::new(192, 168, 1, 1));
    }

    #[test]
    fn test_non_6to4_address() {
        let ipv6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let result = SixToFourTunnel::extract_ipv4_destination(&ipv6);
        
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_tunnel_creation() {
        let config = TunnelConfig {
            protocol: TunnelProtocol::SixToFour,
            local_ipv4: Some(Ipv4Addr::new(203, 0, 113, 1)),
            ..Default::default()
        };

        let tunnel = SixToFourTunnel::new(config).unwrap();
        assert_eq!(tunnel.protocol(), TunnelProtocol::SixToFour);
        assert_eq!(tunnel.state().await, TunnelState::Disconnected);
    }

    #[tokio::test]
    async fn test_tunnel_connection() {
        let config = TunnelConfig {
            protocol: TunnelProtocol::SixToFour,
            local_ipv4: Some(Ipv4Addr::new(203, 0, 113, 1)),
            ..Default::default()
        };

        let mut tunnel = SixToFourTunnel::new(config).unwrap();
        
        assert!(!tunnel.is_active().await);
        
        tunnel.connect().await.unwrap();
        
        assert!(tunnel.is_active().await);
        assert_eq!(tunnel.state().await, TunnelState::Connected);
        
        let ipv6_addr = tunnel.local_ipv6_addr().await.unwrap();
        assert_eq!(ipv6_addr.segments()[0], 0x2002);
    }
}