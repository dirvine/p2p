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

//! 6in4 Static Tunneling Protocol Implementation
//!
//! This module implements the 6in4 (IPv6-in-IPv4) static tunneling mechanism as defined 
//! in RFC 4213. 6in4 provides configured tunneling between IPv6 nodes over IPv4 infrastructure
//! with explicit tunnel endpoints.
//!
//! ## How 6in4 Works
//!
//! - Uses protocol 41 (IPv6-in-IPv4) for direct encapsulation
//! - Requires explicit configuration of remote tunnel endpoint
//! - Supports custom IPv6 prefix assignment
//! - No automatic address discovery - all parameters are configured
//! - Ideal for permanent tunnels between known endpoints

use super::{Tunnel, TunnelConfig, TunnelMetrics, TunnelState, TunnelProtocol};
use crate::{P2PError, Result};
use async_trait::async_trait;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Protocol number for IPv6-in-IPv4 encapsulation (41)
const IPV6_IN_IPV4_PROTOCOL: u8 = 41;

/// Default IPv6 prefix for 6in4 tunnels (can be configured)
const DEFAULT_IPV6_PREFIX: [u16; 4] = [0x2001, 0x0db8, 0x6140, 0x0000]; // 6in4 in hex approximation

/// 6in4 tunnel implementation
pub struct SixInFourTunnel {
    /// Tunnel configuration
    config: TunnelConfig,
    /// Current tunnel state
    state: RwLock<TunnelState>,
    /// Performance metrics
    metrics: RwLock<TunnelMetrics>,
    /// Local IPv4 address (tunnel source)
    local_ipv4: Option<Ipv4Addr>,
    /// Remote IPv4 address (tunnel destination)
    remote_ipv4: Option<Ipv4Addr>,
    /// Local IPv6 address assigned to tunnel interface
    local_ipv6: Option<Ipv6Addr>,
    /// IPv6 prefix for the tunnel network
    ipv6_prefix: Option<Ipv6Addr>,
    /// Connection establishment time
    established_at: Option<Instant>,
    /// Last successful communication
    last_activity: Option<Instant>,
}

impl SixInFourTunnel {
    /// Create a new 6in4 tunnel with the given configuration
    pub fn new(config: TunnelConfig) -> Result<Self> {
        if config.protocol != TunnelProtocol::SixInFour {
            return Err(P2PError::Network(
                "Invalid protocol for 6in4 tunnel".to_string()
            ).into());
        }

        Ok(Self {
            config,
            state: RwLock::new(TunnelState::Disconnected),
            metrics: RwLock::new(TunnelMetrics::default()),
            local_ipv4: None,
            remote_ipv4: None,
            local_ipv6: None,
            ipv6_prefix: None,
            established_at: None,
            last_activity: None,
        })
    }

    /// Generate a 6in4 IPv6 address from configuration
    fn generate_ipv6_address(
        prefix: Option<Ipv6Addr>,
        local_ipv4: Ipv4Addr,
    ) -> Ipv6Addr {
        if let Some(configured_prefix) = prefix {
            // Use configured prefix with interface ID based on IPv4
            let prefix_segments = configured_prefix.segments();
            let ipv4_octets = local_ipv4.octets();
            
            // Create interface ID from IPv4 address
            let interface_id = [
                prefix_segments[0], prefix_segments[1], prefix_segments[2], prefix_segments[3],
                0x0000, 0x0000,
                ((ipv4_octets[0] as u16) << 8) | (ipv4_octets[1] as u16),
                ((ipv4_octets[2] as u16) << 8) | (ipv4_octets[3] as u16),
            ];
            
            Ipv6Addr::from(interface_id)
        } else {
            // Use default prefix (2001:db8:6in4::/64) with IPv4-based interface ID
            let ipv4_octets = local_ipv4.octets();
            Ipv6Addr::new(
                DEFAULT_IPV6_PREFIX[0],
                DEFAULT_IPV6_PREFIX[1], 
                DEFAULT_IPV6_PREFIX[2],
                DEFAULT_IPV6_PREFIX[3],
                0x0000,
                0x0000,
                ((ipv4_octets[0] as u16) << 8) | (ipv4_octets[1] as u16),
                ((ipv4_octets[2] as u16) << 8) | (ipv4_octets[3] as u16),
            )
        }
    }

    /// Create an IPv4 header for 6in4 encapsulation
    fn create_ipv4_header(&self, payload_len: usize) -> Result<Vec<u8>> {
        let local_ipv4 = self.local_ipv4.ok_or_else(|| {
            P2PError::Network("Local IPv4 address not configured".to_string())
        })?;
        
        let remote_ipv4 = self.remote_ipv4.ok_or_else(|| {
            P2PError::Network("Remote IPv4 address not configured".to_string())
        })?;

        let mut header = vec![0u8; 20]; // Standard IPv4 header size

        // Version (4) and Header Length (5 * 4 = 20 bytes)
        header[0] = 0x45;
        
        // Type of Service (can be set based on IPv6 traffic class)
        header[1] = 0x00;
        
        // Total Length (header + payload)
        let total_len = 20 + payload_len;
        header[2] = (total_len >> 8) as u8;
        header[3] = (total_len & 0xFF) as u8;
        
        // Identification (simple counter - in production might use proper ID management)
        header[4] = 0x00;
        header[5] = 0x01;
        
        // Flags and Fragment Offset
        header[6] = 0x40; // Don't Fragment bit set
        header[7] = 0x00;
        
        // Time to Live (64 is common)
        header[8] = 64;
        
        // Protocol (IPv6-in-IPv4)
        header[9] = IPV6_IN_IPV4_PROTOCOL;
        
        // Header Checksum (will be calculated)
        header[10] = 0x00;
        header[11] = 0x00;
        
        // Source IPv4 Address
        let src_octets = local_ipv4.octets();
        header[12..16].copy_from_slice(&src_octets);
        
        // Destination IPv4 Address
        let dst_octets = remote_ipv4.octets();
        header[16..20].copy_from_slice(&dst_octets);
        
        // Calculate checksum
        let checksum = self.calculate_ipv4_checksum(&header);
        header[10] = (checksum >> 8) as u8;
        header[11] = (checksum & 0xFF) as u8;

        Ok(header)
    }

    /// Calculate IPv4 header checksum
    fn calculate_ipv4_checksum(&self, header: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        
        // Sum all 16-bit words in the header (excluding checksum field)
        for i in (0..header.len()).step_by(2) {
            if i + 1 < header.len() {
                // Skip checksum field (bytes 10-11)
                if i == 10 {
                    continue;
                }
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

    /// Validate tunnel configuration
    fn validate_configuration(&self) -> Result<()> {
        if self.config.local_ipv4.is_none() {
            return Err(P2PError::Network(
                "6in4 tunnel requires local IPv4 address".to_string()
            ).into());
        }

        if self.config.remote_ipv4.is_none() {
            return Err(P2PError::Network(
                "6in4 tunnel requires remote IPv4 address".to_string()
            ).into());
        }

        // Validate that local and remote are different
        if self.config.local_ipv4 == self.config.remote_ipv4 {
            return Err(P2PError::Network(
                "Local and remote IPv4 addresses must be different".to_string()
            ).into());
        }

        Ok(())
    }

    /// Test tunnel connectivity by sending a ping packet
    async fn test_connectivity(&mut self) -> Result<Duration> {
        let start = Instant::now();
        
        // Create a simple ICMPv6 echo request packet
        let ping_packet = self.create_icmpv6_ping()?;
        
        // Send through tunnel
        self.send(&ping_packet).await?;
        
        // Simulate response time (in real implementation would wait for response)
        let simulated_rtt = Duration::from_millis(10 + (rand::random::<u64>() % 40));
        tokio::time::sleep(simulated_rtt).await;
        
        let actual_rtt = start.elapsed();
        self.last_activity = Some(Instant::now());
        
        debug!("6in4 tunnel connectivity test successful: RTT = {:?}", actual_rtt);
        Ok(actual_rtt)
    }

    /// Create a simple ICMPv6 ping packet for connectivity testing
    fn create_icmpv6_ping(&self) -> Result<Vec<u8>> {
        let local_ipv6 = self.local_ipv6.ok_or_else(|| {
            P2PError::Network("Local IPv6 address not assigned".to_string())
        })?;

        let mut packet = vec![0u8; 48]; // IPv6 header (40) + ICMPv6 echo (8)
        
        // IPv6 header
        packet[0] = 0x60; // Version (6) + Traffic Class (0)
        packet[1] = 0x00; // Traffic Class + Flow Label
        packet[2] = 0x00; // Flow Label
        packet[3] = 0x00; // Flow Label
        packet[4] = 0x00; // Payload Length (8 bytes)
        packet[5] = 0x08;
        packet[6] = 0x3A; // Next Header (ICMPv6)
        packet[7] = 0x40; // Hop Limit (64)
        
        // Source address (our local IPv6)
        let src_bytes = local_ipv6.octets();
        packet[8..24].copy_from_slice(&src_bytes);
        
        // Destination address (same as source for loopback test)
        packet[24..40].copy_from_slice(&src_bytes);
        
        // ICMPv6 Echo Request
        packet[40] = 0x80; // Type (Echo Request)
        packet[41] = 0x00; // Code
        packet[42] = 0x00; // Checksum (high) - simplified, should calculate
        packet[43] = 0x00; // Checksum (low)
        packet[44] = 0x00; // Identifier (high)
        packet[45] = 0x01; // Identifier (low)
        packet[46] = 0x00; // Sequence Number (high)
        packet[47] = 0x01; // Sequence Number (low)
        
        Ok(packet)
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
impl Tunnel for SixInFourTunnel {
    fn protocol(&self) -> TunnelProtocol {
        TunnelProtocol::SixInFour
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
        info!("Establishing 6in4 tunnel connection");
        
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Connecting;
        }

        // Validate configuration
        self.validate_configuration()?;

        // Extract configuration
        self.local_ipv4 = self.config.local_ipv4;
        self.remote_ipv4 = self.config.remote_ipv4;
        
        // Generate or use configured IPv6 address
        if let Some(local_ipv4) = self.local_ipv4 {
            self.local_ipv6 = Some(Self::generate_ipv6_address(
                self.config.ipv6_prefix, 
                local_ipv4
            ));
            self.ipv6_prefix = self.config.ipv6_prefix;
        }

        // Update tunnel state first before testing connectivity
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Connected;
        }

        // Test basic connectivity
        let rtt = self.test_connectivity().await?;
        
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
            metrics.rtt = Some(rtt);
        }

        info!("6in4 tunnel established: {:?} -> {:?} (IPv6: {:?})", 
              self.local_ipv4, self.remote_ipv4, self.local_ipv6);
        
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting 6in4 tunnel");
        
        {
            let mut state = self.state.write().await;
            *state = TunnelState::Disconnecting;
        }

        // Reset state
        self.local_ipv4 = None;
        self.remote_ipv4 = None;
        self.local_ipv6 = None;
        self.ipv6_prefix = None;
        self.established_at = None;
        self.last_activity = None;

        {
            let mut state = self.state.write().await;
            *state = TunnelState::Disconnected;
        }

        debug!("6in4 tunnel disconnected");
        Ok(())
    }

    async fn is_active(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, TunnelState::Connected)
    }

    async fn encapsulate(&self, ipv6_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("6in4 tunnel not active".to_string()).into());
        }

        // Create IPv4 header for encapsulation
        let ipv4_header = self.create_ipv4_header(ipv6_packet.len())?;
        
        // Combine header and IPv6 packet
        let mut encapsulated = Vec::with_capacity(ipv4_header.len() + ipv6_packet.len());
        encapsulated.extend_from_slice(&ipv4_header);
        encapsulated.extend_from_slice(ipv6_packet);

        debug!("Encapsulated {} bytes for 6in4 transmission to {:?}", 
               encapsulated.len(), self.remote_ipv4);

        Ok(encapsulated)
    }

    async fn decapsulate(&self, ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("6in4 tunnel not active".to_string()).into());
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

        debug!("Decapsulated {} bytes from 6in4 packet", ipv6_payload.len());
        Ok(ipv6_payload)
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        let encapsulated = self.encapsulate(packet).await?;
        
        // In a real implementation, this would send the packet via raw sockets
        // For now, we simulate the send operation
        debug!("Sending {} bytes via 6in4 tunnel to {:?}", 
               encapsulated.len(), self.remote_ipv4);
        
        // Simulate network transmission delay
        tokio::time::sleep(Duration::from_millis(2)).await;
        
        self.update_send_metrics(encapsulated.len()).await;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        // In a real implementation, this would receive packets from raw sockets
        // For now, we simulate receiving a packet
        debug!("Receiving packet via 6in4 tunnel");
        
        // Simulate network reception delay
        tokio::time::sleep(Duration::from_millis(3)).await;
        
        // Simulate a received IPv4 packet containing IPv6 data
        let simulated_packet = vec![0u8; 68]; // IPv4 header + IPv6 packet
        
        let decapsulated = self.decapsulate(&simulated_packet).await?;
        self.update_receive_metrics(decapsulated.len()).await;
        
        Ok(decapsulated)
    }

    async fn maintain(&mut self) -> Result<()> {
        if !self.is_active().await {
            return Ok(());
        }

        debug!("Performing 6in4 tunnel maintenance");
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            if let Some(established) = self.established_at {
                metrics.establishment_time = established.elapsed();
            }
        }

        // Perform periodic connectivity check
        if let Some(last_activity) = self.last_activity {
            if last_activity.elapsed() > Duration::from_secs(30) {
                debug!("Testing 6in4 tunnel connectivity (periodic check)");
                match self.test_connectivity().await {
                    Ok(rtt) => {
                        let mut metrics = self.metrics.write().await;
                        metrics.rtt = Some(rtt);
                    }
                    Err(e) => {
                        debug!("6in4 connectivity test failed: {}", e);
                        // Don't fail maintenance, just log the issue
                    }
                }
            }
        }

        // In a real implementation, this might also:
        // - Check for IPv4 address changes
        // - Update routing table entries
        // - Monitor tunnel interface status
        // - Refresh tunnel configuration

        Ok(())
    }

    async fn local_ipv6_addr(&self) -> Result<Ipv6Addr> {
        self.local_ipv6.ok_or_else(|| {
            P2PError::Network("6in4 tunnel not established".to_string()).into()
        })
    }

    async fn local_ipv4_addr(&self) -> Result<Ipv4Addr> {
        self.local_ipv4.ok_or_else(|| {
            P2PError::Network("6in4 tunnel not established".to_string()).into()
        })
    }

    async fn ping(&mut self, timeout: Duration) -> Result<Duration> {
        if !self.is_active().await {
            return Err(P2PError::Network("6in4 tunnel not active".to_string()).into());
        }

        debug!("Pinging via 6in4 tunnel with timeout {:?}", timeout);
        
        // Perform connectivity test with timeout
        let rtt = tokio::time::timeout(timeout, self.test_connectivity()).await
            .map_err(|_| P2PError::Network("6in4 ping timeout".to_string()))?
            .map_err(|e| P2PError::Network(format!("6in4 ping failed: {}", e)))?;
        
        // Update metrics with RTT
        {
            let mut metrics = self.metrics.write().await;
            metrics.rtt = Some(rtt);
        }

        debug!("6in4 tunnel ping successful: RTT = {:?}", rtt);
        Ok(rtt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv6_address_generation() {
        let local_ipv4 = Ipv4Addr::new(10, 0, 0, 1);
        let ipv6 = SixInFourTunnel::generate_ipv6_address(None, local_ipv4);
        
        // Should use default prefix 2001:db8:6140::
        assert_eq!(ipv6.segments()[0], 0x2001);
        assert_eq!(ipv6.segments()[1], 0x0db8);
        assert_eq!(ipv6.segments()[2], 0x6140);
        
        // Interface ID should be based on IPv4 address (10.0.0.1)
        assert_eq!(ipv6.segments()[6], 0x0a00); // 10.0
        assert_eq!(ipv6.segments()[7], 0x0001); // 0.1
    }

    #[test]
    fn test_ipv6_address_generation_with_prefix() {
        let local_ipv4 = Ipv4Addr::new(192, 168, 1, 100);
        let custom_prefix = Ipv6Addr::new(0x2001, 0x470, 0x1234, 0x5678, 0, 0, 0, 0);
        let ipv6 = SixInFourTunnel::generate_ipv6_address(Some(custom_prefix), local_ipv4);
        
        // Should use custom prefix
        assert_eq!(ipv6.segments()[0], 0x2001);
        assert_eq!(ipv6.segments()[1], 0x470);
        assert_eq!(ipv6.segments()[2], 0x1234);
        assert_eq!(ipv6.segments()[3], 0x5678);
        
        // Interface ID should be based on IPv4 address (192.168.1.100)
        assert_eq!(ipv6.segments()[6], 0xc0a8); // 192.168
        assert_eq!(ipv6.segments()[7], 0x0164); // 1.100
    }

    #[tokio::test]
    async fn test_tunnel_creation() {
        let config = TunnelConfig {
            protocol: TunnelProtocol::SixInFour,
            local_ipv4: Some(Ipv4Addr::new(10, 0, 0, 1)),
            remote_ipv4: Some(Ipv4Addr::new(10, 0, 0, 2)),
            ..Default::default()
        };

        let tunnel = SixInFourTunnel::new(config).unwrap();
        assert_eq!(tunnel.protocol(), TunnelProtocol::SixInFour);
        assert_eq!(tunnel.state().await, TunnelState::Disconnected);
    }

    #[tokio::test]
    async fn test_invalid_protocol() {
        let config = TunnelConfig {
            protocol: TunnelProtocol::SixToFour, // Wrong protocol
            ..Default::default()
        };

        let result = SixInFourTunnel::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_configuration_validation() {
        // Test missing local IPv4
        let config1 = TunnelConfig {
            protocol: TunnelProtocol::SixInFour,
            local_ipv4: None,
            remote_ipv4: Some(Ipv4Addr::new(10, 0, 0, 2)),
            ..Default::default()
        };

        let mut tunnel1 = SixInFourTunnel::new(config1).unwrap();
        assert!(tunnel1.connect().await.is_err());

        // Test missing remote IPv4
        let config2 = TunnelConfig {
            protocol: TunnelProtocol::SixInFour,
            local_ipv4: Some(Ipv4Addr::new(10, 0, 0, 1)),
            remote_ipv4: None,
            ..Default::default()
        };

        let mut tunnel2 = SixInFourTunnel::new(config2).unwrap();
        assert!(tunnel2.connect().await.is_err());

        // Test same local and remote IPv4
        let config3 = TunnelConfig {
            protocol: TunnelProtocol::SixInFour,
            local_ipv4: Some(Ipv4Addr::new(10, 0, 0, 1)),
            remote_ipv4: Some(Ipv4Addr::new(10, 0, 0, 1)),
            ..Default::default()
        };

        let mut tunnel3 = SixInFourTunnel::new(config3).unwrap();
        assert!(tunnel3.connect().await.is_err());
    }

    #[tokio::test]
    async fn test_tunnel_connection() {
        let config = TunnelConfig {
            protocol: TunnelProtocol::SixInFour,
            local_ipv4: Some(Ipv4Addr::new(203, 0, 113, 1)),
            remote_ipv4: Some(Ipv4Addr::new(203, 0, 113, 2)),
            ..Default::default()
        };

        let mut tunnel = SixInFourTunnel::new(config).unwrap();
        
        assert!(!tunnel.is_active().await);
        
        tunnel.connect().await.unwrap();
        
        assert!(tunnel.is_active().await);
        assert_eq!(tunnel.state().await, TunnelState::Connected);
        
        let ipv6_addr = tunnel.local_ipv6_addr().await.unwrap();
        assert_eq!(ipv6_addr.segments()[0], 0x2001); // Default prefix
        
        let ipv4_addr = tunnel.local_ipv4_addr().await.unwrap();
        assert_eq!(ipv4_addr, Ipv4Addr::new(203, 0, 113, 1));
    }
}