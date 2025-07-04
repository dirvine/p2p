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

//! DS-Lite (Dual-Stack Lite) Tunneling Protocol Implementation
//!
//! This module implements the DS-Lite tunneling mechanism as defined in RFC 6333.
//! DS-Lite enables IPv4 connectivity over IPv6-only networks using ISP-provided
//! infrastructure.
//!
//! ## Architecture
//!
//! DS-Lite consists of two main components:
//! - **B4 (Basic Bridging BroadBand) Element**: Client-side tunnel endpoint (this implementation)
//! - **AFTR (Address Family Transition Router)**: ISP-provided tunnel concentrator and NAT
//!
//! ## How DS-Lite Works
//!
//! 1. The B4 element creates an IPv4-in-IPv6 tunnel to the AFTR
//! 2. IPv4 packets are encapsulated in IPv6 headers for transport
//! 3. The AFTR performs centralized NAT44 for all clients
//! 4. DNS resolution happens over IPv6, not through the tunnel
//!
//! ## AFTR Discovery
//!
//! The AFTR can be discovered through:
//! - DHCPv6 option (AFTR_NAME)
//! - DNS resolution of well-known names
//! - Manual configuration

use super::{Tunnel, TunnelConfig, TunnelMetrics, TunnelState, TunnelProtocol};
use crate::{P2PError, Result};
use async_trait::async_trait;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::net::UdpSocket;
use tracing::{debug, info, warn, error};

/// Protocol number for IPv4-in-IPv6 encapsulation (4)
const IPV4_IN_IPV6_PROTOCOL: u8 = 4;

/// Well-known IPv4 address used by AFTR for tunnel configuration (192.0.0.1)
const AFTR_WELL_KNOWN_IPV4: Ipv4Addr = Ipv4Addr::new(192, 0, 0, 1);

/// Default MTU for DS-Lite tunnels (1520 bytes to account for IPv6 header)
#[allow(dead_code)]
const DEFAULT_DSLITE_MTU: u16 = 1520;

/// DS-Lite B4 element implementation
pub struct DsLiteTunnel {
    /// Tunnel configuration
    config: TunnelConfig,
    /// Current tunnel state
    state: RwLock<TunnelState>,
    /// Performance metrics
    metrics: RwLock<TunnelMetrics>,
    /// AFTR IPv6 address (discovered or configured)
    aftr_address: RwLock<Option<Ipv6Addr>>,
    /// Local IPv6 address for tunnel endpoint
    #[allow(dead_code)]
    local_ipv6: Option<Ipv6Addr>,
    /// UDP socket for tunnel communication
    socket: RwLock<Option<UdpSocket>>,
    /// Connection establishment time
    established_at: RwLock<Option<Instant>>,
}

impl DsLiteTunnel {
    /// Create a new DS-Lite tunnel with the given configuration
    pub fn new(config: TunnelConfig) -> Result<Self> {
        if config.protocol != TunnelProtocol::DsLite {
            return Err(P2PError::Network(
                "Invalid protocol for DS-Lite tunnel".to_string()
            ).into());
        }

        Ok(Self {
            config,
            state: RwLock::new(TunnelState::Disconnected),
            metrics: RwLock::new(TunnelMetrics::default()),
            aftr_address: RwLock::new(None),
            local_ipv6: None,
            socket: RwLock::new(None),
            established_at: RwLock::new(None),
        })
    }

    /// Discover AFTR address through various mechanisms
    pub async fn discover_aftr(&self) -> Result<Ipv6Addr> {
        // First, check if AFTR address is explicitly configured
        if let Some(aftr_addr) = self.config.aftr_ipv6 {
            debug!("Using configured AFTR address: {}", aftr_addr);
            return Ok(aftr_addr);
        }

        // Try DNS resolution if AFTR name is configured
        if let Some(aftr_name) = &self.config.aftr_name {
            debug!("Attempting DNS resolution for AFTR: {}", aftr_name);
            match self.resolve_aftr_dns(aftr_name).await {
                Ok(addr) => {
                    info!("Resolved AFTR {} to {}", aftr_name, addr);
                    return Ok(addr);
                }
                Err(e) => {
                    warn!("Failed to resolve AFTR via DNS: {}", e);
                }
            }
        }

        // Try well-known AFTR names as fallback
        let well_known_names = [
            "aftr.example.com",
            "dslite.example.com", 
            "b4.example.com",
        ];

        for name in &well_known_names {
            debug!("Trying well-known AFTR name: {}", name);
            match self.resolve_aftr_dns(name).await {
                Ok(addr) => {
                    info!("Resolved well-known AFTR {} to {}", name, addr);
                    return Ok(addr);
                }
                Err(e) => {
                    debug!("Failed to resolve {}: {}", name, e);
                }
            }
        }

        Err(P2PError::Network(
            "Could not discover AFTR address through any mechanism".to_string()
        ).into())
    }

    /// Resolve AFTR address via DNS
    async fn resolve_aftr_dns(&self, hostname: &str) -> Result<Ipv6Addr> {
        use tokio::net::lookup_host;

        let addresses: Vec<SocketAddr> = lookup_host(format!("{}:0", hostname))
            .await
            .map_err(|e| P2PError::Network(format!("DNS resolution failed: {}", e)))?
            .collect();

        // Look for IPv6 addresses
        for addr in addresses {
            if let SocketAddr::V6(addr_v6) = addr {
                return Ok(*addr_v6.ip());
            }
        }

        Err(P2PError::Network(
            format!("No IPv6 addresses found for AFTR {}", hostname)
        ).into())
    }

    /// Get local IPv6 address for tunnel endpoint
    async fn get_local_ipv6(&self) -> Result<Ipv6Addr> {
        // In a real implementation, this would:
        // 1. Check for configured IPv6 address
        // 2. Use IPv6 address from network interface
        // 3. Use link-local address as fallback

        // For now, use a placeholder that would be replaced with actual network detection
        if let Some(configured_ipv6) = self.config.ipv6_prefix {
            Ok(configured_ipv6)
        } else {
            // Use link-local address as fallback
            Ok("fe80::1".parse().unwrap())
        }
    }

    /// Encapsulate IPv4 packet in IPv6 header for DS-Lite transport
    pub fn encapsulate_ipv4_in_ipv6(&self, ipv4_packet: &[u8], aftr_addr: Ipv6Addr, local_ipv6: Ipv6Addr) -> Result<Vec<u8>> {
        // IPv6 header is 40 bytes
        let mut ipv6_packet = Vec::with_capacity(40 + ipv4_packet.len());

        // IPv6 Header construction
        // Version (4 bits) = 6, Traffic Class (8 bits) = 0, Flow Label (20 bits) = 0
        ipv6_packet.push(0x60); // Version 6, Traffic Class upper 4 bits
        ipv6_packet.push(0x00); // Traffic Class lower 4 bits, Flow Label upper 4 bits
        ipv6_packet.push(0x00); // Flow Label middle 8 bits
        ipv6_packet.push(0x00); // Flow Label lower 8 bits

        // Payload Length (16 bits) - length of IPv4 packet
        let payload_len = ipv4_packet.len() as u16;
        ipv6_packet.extend_from_slice(&payload_len.to_be_bytes());

        // Next Header (8 bits) - IPv4 protocol number
        ipv6_packet.push(IPV4_IN_IPV6_PROTOCOL);

        // Hop Limit (8 bits) - set to 64
        ipv6_packet.push(64);

        // Source Address (128 bits) - local IPv6 address
        ipv6_packet.extend_from_slice(&local_ipv6.octets());

        // Destination Address (128 bits) - AFTR IPv6 address
        ipv6_packet.extend_from_slice(&aftr_addr.octets());

        // Append IPv4 payload
        ipv6_packet.extend_from_slice(ipv4_packet);

        debug!("Encapsulated {} byte IPv4 packet in {} byte IPv6 packet", 
               ipv4_packet.len(), ipv6_packet.len());

        Ok(ipv6_packet)
    }

    /// Decapsulate IPv6 packet to extract IPv4 content
    pub fn decapsulate_ipv6_to_ipv4(&self, ipv6_packet: &[u8]) -> Result<Vec<u8>> {
        if ipv6_packet.len() < 40 {
            return Err(P2PError::Network("IPv6 packet too short".to_string()).into());
        }

        // Verify this is an IPv6 packet (version field)
        if (ipv6_packet[0] >> 4) != 6 {
            return Err(P2PError::Network("Not an IPv6 packet".to_string()).into());
        }

        // Check Next Header field for IPv4 protocol
        if ipv6_packet[6] != IPV4_IN_IPV6_PROTOCOL {
            return Err(P2PError::Network(
                format!("Unexpected protocol in IPv6 header: {}", ipv6_packet[6])
            ).into());
        }

        // Extract payload length
        let payload_len = u16::from_be_bytes([ipv6_packet[4], ipv6_packet[5]]) as usize;
        
        if ipv6_packet.len() < 40 + payload_len {
            return Err(P2PError::Network("IPv6 packet truncated".to_string()).into());
        }

        // Extract IPv4 packet (skip 40-byte IPv6 header)
        let ipv4_packet = ipv6_packet[40..40 + payload_len].to_vec();

        debug!("Decapsulated {} byte IPv6 packet to {} byte IPv4 packet", 
               ipv6_packet.len(), ipv4_packet.len());

        Ok(ipv4_packet)
    }

    /// Update tunnel metrics
    async fn update_metrics(&self, bytes_sent: u64, bytes_received: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.bytes_sent += bytes_sent;
        metrics.bytes_received += bytes_received;
        metrics.last_activity = Instant::now();
        if bytes_sent > 0 {
            metrics.packets_sent += 1;
        }
        if bytes_received > 0 {
            metrics.packets_received += 1;
        }
    }
}
#[async_trait]
impl Tunnel for DsLiteTunnel {
    fn protocol(&self) -> TunnelProtocol {
        TunnelProtocol::DsLite
    }

    fn config(&self) -> &TunnelConfig {
        &self.config
    }

    async fn state(&self) -> TunnelState {
        self.state.read().await.clone()
    }

    async fn metrics(&self) -> TunnelMetrics {
        self.metrics.read().await.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        info!("Establishing DS-Lite tunnel connection");
        *self.state.write().await = TunnelState::Connecting;

        let start_time = Instant::now();

        // Discover AFTR address
        let aftr_addr = match self.discover_aftr().await {
            Ok(addr) => {
                info!("AFTR discovery successful: {}", addr);
                addr
            }
            Err(e) => {
                error!("AFTR discovery failed: {}", e);
                *self.state.write().await = TunnelState::Failed(format!("AFTR discovery failed: {}", e));
                return Err(e);
            }
        };

        *self.aftr_address.write().await = Some(aftr_addr);

        // Get local IPv6 address
        let local_ipv6 = match self.get_local_ipv6().await {
            Ok(addr) => {
                debug!("Using local IPv6 address: {}", addr);
                addr
            }
            Err(e) => {
                error!("Failed to get local IPv6 address: {}", e);
                *self.state.write().await = TunnelState::Failed(format!("Local IPv6 setup failed: {}", e));
                return Err(e);
            }
        };

        // Create UDP socket for tunnel communication
        let socket_addr = SocketAddrV6::new(local_ipv6, 0, 0, 0);
        let socket = match UdpSocket::bind(socket_addr).await {
            Ok(sock) => {
                debug!("Created DS-Lite tunnel socket on {}", socket_addr);
                sock
            }
            Err(e) => {
                error!("Failed to create tunnel socket: {}", e);
                *self.state.write().await = TunnelState::Failed(format!("Socket creation failed: {}", e));
                return Err(P2PError::Network(format!("Socket creation failed: {}", e)).into());
            }
        };

        *self.socket.write().await = Some(socket);
        *self.established_at.write().await = Some(start_time);

        // Update metrics
        let establishment_time = start_time.elapsed();
        self.metrics.write().await.establishment_time = establishment_time;

        *self.state.write().await = TunnelState::Connected;
        info!("DS-Lite tunnel established successfully in {:?}", establishment_time);

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting DS-Lite tunnel");
        *self.state.write().await = TunnelState::Disconnecting;

        // Close socket
        *self.socket.write().await = None;
        *self.aftr_address.write().await = None;
        *self.established_at.write().await = None;

        *self.state.write().await = TunnelState::Disconnected;
        info!("DS-Lite tunnel disconnected");

        Ok(())
    }

    async fn is_active(&self) -> bool {
        matches!(*self.state.read().await, TunnelState::Connected)
    }

    async fn encapsulate(&self, _ipv6_packet: &[u8]) -> Result<Vec<u8>> {
        return Err(P2PError::Network(
            "DS-Lite does not encapsulate IPv6 packets - it encapsulates IPv4 packets in IPv6".to_string()
        ).into());
    }

    async fn decapsulate(&self, _ipv4_packet: &[u8]) -> Result<Vec<u8>> {
        return Err(P2PError::Network(
            "DS-Lite does not decapsulate to IPv6 - it decapsulates IPv6 packets to IPv4".to_string()
        ).into());
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        if !self.is_active().await {
            return Err(P2PError::Network("Tunnel not connected".to_string()).into());
        }

        let aftr_addr = self.aftr_address.read().await
            .ok_or_else(|| P2PError::Network("AFTR address not available".to_string()))?;

        let local_ipv6 = self.get_local_ipv6().await?;

        // Encapsulate IPv4 packet in IPv6
        let ipv6_packet = self.encapsulate_ipv4_in_ipv6(packet, aftr_addr, local_ipv6)?;

        // Send via socket
        let socket_guard = self.socket.read().await;
        if let Some(socket) = socket_guard.as_ref() {
            let aftr_socket_addr = SocketAddrV6::new(aftr_addr, 0, 0, 0);
            socket.send_to(&ipv6_packet, aftr_socket_addr).await
                .map_err(|e| P2PError::Network(format!("Send failed: {}", e)))?;

            // Update metrics
            self.update_metrics(ipv6_packet.len() as u64, 0).await;

            debug!("Sent {} byte packet through DS-Lite tunnel", packet.len());
            Ok(())
        } else {
            Err(P2PError::Network("Socket not available".to_string()).into())
        }
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        if !self.is_active().await {
            return Err(P2PError::Network("Tunnel not connected".to_string()).into());
        }

        let socket_guard = self.socket.read().await;
        if let Some(socket) = socket_guard.as_ref() {
            let mut buffer = vec![0u8; 65536]; // Max UDP packet size
            let (len, _addr) = socket.recv_from(&mut buffer).await
                .map_err(|e| P2PError::Network(format!("Receive failed: {}", e)))?;

            buffer.truncate(len);

            // Decapsulate IPv6 packet to extract IPv4 content
            let ipv4_packet = self.decapsulate_ipv6_to_ipv4(&buffer)?;

            // Update metrics
            self.update_metrics(0, len as u64).await;

            debug!("Received {} byte packet through DS-Lite tunnel", ipv4_packet.len());
            Ok(ipv4_packet)
        } else {
            Err(P2PError::Network("Socket not available".to_string()).into())
        }
    }

    async fn maintain(&mut self) -> Result<()> {
        // DS-Lite tunnels typically don't require active maintenance
        // The AFTR handles NAT state and keepalive is not needed
        // We could optionally check AFTR reachability here
        debug!("DS-Lite tunnel maintenance - no action required");
        Ok(())
    }

    async fn local_ipv6_addr(&self) -> Result<Ipv6Addr> {
        self.get_local_ipv6().await
    }

    async fn local_ipv4_addr(&self) -> Result<Ipv4Addr> {
        // DS-Lite uses the well-known IPv4 address for configuration
        Ok(AFTR_WELL_KNOWN_IPV4)
    }

    async fn ping(&mut self, timeout: Duration) -> Result<Duration> {
        let start = Instant::now();

        // For DS-Lite, we can check AFTR reachability
        if let Some(aftr_addr) = *self.aftr_address.read().await {
            // Simple reachability test - attempt to connect to AFTR
            let socket_addr = SocketAddrV6::new(aftr_addr, 0, 0, 0);
            
            // This is a simplified ping - in practice you'd send an ICMPv6 echo request
            tokio::time::timeout(timeout, async {
                // Attempt to create a connection to test reachability
                let test_socket = UdpSocket::bind("[::]:0").await
                    .map_err(|e| P2PError::Network(format!("Ping test failed: {}", e)))?;
                
                test_socket.connect(socket_addr).await
                    .map_err(|e| P2PError::Network(format!("AFTR unreachable: {}", e)))?;

                Ok::<(), P2PError>(())
            }).await
            .map_err(|_| P2PError::Network("Ping timeout".to_string()))??;

            let rtt = start.elapsed();
            self.metrics.write().await.rtt = Some(rtt);
            Ok(rtt)
        } else {
            Err(P2PError::Network("AFTR address not available for ping".to_string()).into())
        }
    }
}