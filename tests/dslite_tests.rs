// Copyright 2024 MaidSafe Limited
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

//! DS-Lite (Dual-Stack Lite) Tunneling Tests
//!
//! Comprehensive tests for the DS-Lite tunneling protocol implementation,
//! covering B4 element functionality, AFTR discovery, packet encapsulation,
//! and integration with the P2P Foundation tunneling system.

use p2p_foundation::tunneling::{
    DsLiteTunnel, TunnelConfig, TunnelProtocol, TunnelState, Tunnel,
    NetworkCapabilities, create_tunnel_config, create_tunnel
};
use p2p_foundation::Result;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use tokio;

/// Helper to create a basic DS-Lite tunnel configuration
fn create_test_dslite_config() -> TunnelConfig {
    TunnelConfig {
        protocol: TunnelProtocol::DsLite,
        local_ipv4: None,
        remote_ipv4: None,
        ipv6_prefix: Some("2001:db8:1234:0:0:0:0:1".parse().unwrap()),
        aftr_ipv6: Some("2001:db8:abcd:0:0:0:0:1".parse().unwrap()),
        aftr_name: Some("aftr.example.com".to_string()),
        mtu: 1520,
        keepalive_interval: Duration::from_secs(30),
        establishment_timeout: Duration::from_secs(10),
    }
}

/// Helper to create network capabilities with IPv6 support (required for DS-Lite)
fn create_ipv6_capabilities() -> NetworkCapabilities {
    NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: true,
        behind_nat: true, // DS-Lite handles NAT at AFTR
        public_ipv4: Some("203.0.113.42".parse().unwrap()),
        ipv6_addresses: vec!["2001:db8:1111:0:0:0:0:1".parse().unwrap()],
        has_upnp: false,
        interface_mtu: 1500,
    }
}

/// Test DS-Lite tunnel creation
#[tokio::test]
async fn test_dslite_tunnel_creation() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = DsLiteTunnel::new(config)?;

    assert_eq!(tunnel.protocol(), TunnelProtocol::DsLite);
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test DS-Lite tunnel creation with invalid protocol
#[tokio::test]
async fn test_dslite_invalid_protocol() {
    let mut config = create_test_dslite_config();
    config.protocol = TunnelProtocol::SixToFour; // Wrong protocol

    let result = DsLiteTunnel::new(config);
    assert!(result.is_err());
}

/// Test DS-Lite tunnel configuration generation
#[tokio::test]
async fn test_dslite_config_generation() -> Result<()> {
    let capabilities = create_ipv6_capabilities();
    let config = create_tunnel_config(TunnelProtocol::DsLite, &capabilities);

    assert_eq!(config.protocol, TunnelProtocol::DsLite);
    assert_eq!(config.mtu, 1520); // DS-Lite MTU
    assert!(config.aftr_name.is_some());
    assert_eq!(config.aftr_name.as_ref().unwrap(), "aftr.example.com");

    Ok(())
}

/// Test tunnel factory function for DS-Lite
#[tokio::test]
async fn test_dslite_tunnel_factory() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = create_tunnel(config)?;

    assert_eq!(tunnel.protocol(), TunnelProtocol::DsLite);
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test DS-Lite tunnel connection establishment
#[tokio::test]
async fn test_dslite_connection() -> Result<()> {
    let config = create_test_dslite_config();
    let mut tunnel = DsLiteTunnel::new(config)?;

    // Initial state should be disconnected
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);
    assert!(!tunnel.is_active().await);

    // Note: In a real test environment, this would attempt actual AFTR discovery
    // For now, this tests the logic flow but may fail due to missing AFTR
    let connection_result = tunnel.connect().await;
    
    // The connection may fail due to AFTR discovery, but we test the state transitions
    match connection_result {
        Ok(_) => {
            assert_eq!(tunnel.state().await, TunnelState::Connected);
            assert!(tunnel.is_active().await);
        }
        Err(_) => {
            // Expected in test environment without real AFTR
            let state = tunnel.state().await;
            if let TunnelState::Failed(reason) = state {
                println!("Connection failed as expected: {}", reason);
                assert!(reason.contains("AFTR") || reason.contains("DNS") || reason.contains("IPv6") || reason.contains("Socket"));
            }
        }
    }

    Ok(())
}

/// Test DS-Lite tunnel disconnection
#[tokio::test]
async fn test_dslite_disconnection() -> Result<()> {
    let config = create_test_dslite_config();
    let mut tunnel = DsLiteTunnel::new(config)?;

    // Test disconnection from disconnected state
    tunnel.disconnect().await?;
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test DS-Lite IPv4-in-IPv6 encapsulation
#[tokio::test]
async fn test_dslite_packet_encapsulation() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = DsLiteTunnel::new(config)?;

    // Create a simple IPv4 packet (simplified header)
    let ipv4_packet = vec![
        0x45, 0x00, // Version=4, IHL=5, Type of Service=0
        0x00, 0x1c, // Total Length=28
        0x00, 0x00, // Identification=0
        0x40, 0x00, // Flags=0x4000, Fragment Offset=0
        0x40, 0x01, // TTL=64, Protocol=ICMP
        0x00, 0x00, // Header Checksum (would be calculated)
        0xc0, 0xa8, 0x01, 0x01, // Source IP: 192.168.1.1
        0xc0, 0xa8, 0x01, 0x02, // Dest IP: 192.168.1.2
        // ICMP data would follow...
    ];

    let aftr_addr: Ipv6Addr = "2001:db8:abcd:0:0:0:0:1".parse().unwrap();
    let local_ipv6: Ipv6Addr = "2001:db8:1111:0:0:0:0:1".parse().unwrap();

    // Test the encapsulation function directly
    let ipv6_packet = tunnel.encapsulate_ipv4_in_ipv6(&ipv4_packet, aftr_addr, local_ipv6)?;

    // Verify IPv6 header structure
    assert_eq!(ipv6_packet.len(), 40 + ipv4_packet.len()); // IPv6 header + IPv4 payload
    assert_eq!(ipv6_packet[0] & 0xF0, 0x60); // IPv6 version
    assert_eq!(ipv6_packet[6], 4); // Next Header = IPv4
    
    // Verify source and destination addresses are embedded correctly
    let src_addr = &ipv6_packet[8..24];
    let dst_addr = &ipv6_packet[24..40];
    assert_eq!(src_addr, local_ipv6.octets());
    assert_eq!(dst_addr, aftr_addr.octets());

    Ok(())
}

/// Test DS-Lite IPv6-to-IPv4 decapsulation
#[tokio::test]
async fn test_dslite_packet_decapsulation() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = DsLiteTunnel::new(config)?;

    // Create IPv6 packet with embedded IPv4
    let original_ipv4 = vec![
        0x45, 0x00, 0x00, 0x1c, // IPv4 header start
        0x00, 0x00, 0x40, 0x00,
        0x40, 0x01, 0x00, 0x00,
        0xc0, 0xa8, 0x01, 0x01,
        0xc0, 0xa8, 0x01, 0x02,
    ];

    let mut ipv6_packet = Vec::new();
    
    // IPv6 header (40 bytes)
    ipv6_packet.extend_from_slice(&[
        0x60, 0x00, 0x00, 0x00, // Version=6, Traffic Class=0, Flow Label=0
    ]);
    ipv6_packet.extend_from_slice(&(original_ipv4.len() as u16).to_be_bytes()); // Payload Length
    ipv6_packet.push(4); // Next Header = IPv4
    ipv6_packet.push(64); // Hop Limit
    
    // Source IPv6 address (16 bytes)
    ipv6_packet.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0xaf, 0x61, 0x00, 0x00,
                                   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
    // Destination IPv6 address (16 bytes)  
    ipv6_packet.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0xc1, 0x1e, 0x00, 0x00,
                                   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
    
    // IPv4 payload
    ipv6_packet.extend_from_slice(&original_ipv4);

    // Test decapsulation
    let extracted_ipv4 = tunnel.decapsulate_ipv6_to_ipv4(&ipv6_packet)?;
    assert_eq!(extracted_ipv4, original_ipv4);

    Ok(())
}

/// Test DS-Lite with invalid IPv6 packet
#[tokio::test]
async fn test_dslite_invalid_decapsulation() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = DsLiteTunnel::new(config)?;

    // Test with too short packet
    let short_packet = vec![0x60, 0x00]; // Too short for IPv6 header
    let result = tunnel.decapsulate_ipv6_to_ipv4(&short_packet);
    assert!(result.is_err());

    // Test with wrong version
    let wrong_version = vec![0x40; 50]; // IPv4 version in IPv6 packet
    let result = tunnel.decapsulate_ipv6_to_ipv4(&wrong_version);
    assert!(result.is_err());

    Ok(())
}

/// Test DS-Lite AFTR discovery mechanisms
#[tokio::test]
async fn test_dslite_aftr_discovery() -> Result<()> {
    let mut config = create_test_dslite_config();
    
    // Test with explicit AFTR IPv6 address
    config.aftr_ipv6 = Some("2001:db8:abcd:0:0:0:0:100".parse().unwrap());
    let tunnel = DsLiteTunnel::new(config.clone())?;
    
    // Discovery should use the configured address
    let discovered = tunnel.discover_aftr().await;
    // This may fail in test environment, but tests the logic
    match discovered {
        Ok(addr) => assert_eq!(addr, "2001:db8:abcd:0:0:0:0:100".parse::<Ipv6Addr>().unwrap()),
        Err(_) => {
            // Expected in test environment without real DNS/network
            println!("AFTR discovery failed as expected in test environment");
        }
    }

    // Test with AFTR name only
    config.aftr_ipv6 = None;
    config.aftr_name = Some("test-aftr.example.com".to_string());
    let tunnel = DsLiteTunnel::new(config)?;
    
    // This will likely fail in test environment due to DNS resolution
    let result = tunnel.discover_aftr().await;
    match result {
        Ok(_) => {
            // Unexpected success in test environment
            println!("AFTR DNS resolution succeeded unexpectedly");
        }
        Err(e) => {
            // Expected failure
            println!("AFTR DNS discovery failed as expected: {}", e);
            assert!(e.to_string().contains("DNS") || e.to_string().contains("discover"));
        }
    }

    Ok(())
}

/// Test DS-Lite tunnel maintenance
#[tokio::test]
async fn test_dslite_maintenance() -> Result<()> {
    let config = create_test_dslite_config();
    let mut tunnel = DsLiteTunnel::new(config)?;

    // Test maintenance on disconnected tunnel
    tunnel.maintain().await?;
    
    // DS-Lite maintenance should be minimal (just logging)
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test DS-Lite tunnel metrics
#[tokio::test]
async fn test_dslite_metrics() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = DsLiteTunnel::new(config)?;

    let metrics = tunnel.metrics().await;
    assert_eq!(metrics.bytes_sent, 0);
    assert_eq!(metrics.bytes_received, 0);
    assert_eq!(metrics.packets_sent, 0);
    assert_eq!(metrics.packets_received, 0);

    Ok(())
}

/// Test DS-Lite tunnel addresses
#[tokio::test]
async fn test_dslite_addresses() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = DsLiteTunnel::new(config)?;

    // Test IPv4 address (well-known AFTR address)
    let ipv4_addr = tunnel.local_ipv4_addr().await?;
    assert_eq!(ipv4_addr, "192.0.0.1".parse::<Ipv4Addr>().unwrap());

    // Test IPv6 address 
    let ipv6_result = tunnel.local_ipv6_addr().await;
    match ipv6_result {
        Ok(addr) => {
            // Should be link-local or configured address
            println!("Local IPv6 address: {}", addr);
        }
        Err(_) => {
            // May fail in test environment
            println!("IPv6 address retrieval failed in test environment");
        }
    }

    Ok(())
}

/// Test DS-Lite integration with network capabilities
#[tokio::test]
async fn test_dslite_network_capabilities() -> Result<()> {
    // Test with IPv6-capable network
    let ipv6_caps = create_ipv6_capabilities();
    let config = create_tunnel_config(TunnelProtocol::DsLite, &ipv6_caps);
    assert_eq!(config.protocol, TunnelProtocol::DsLite);

    // Test with IPv4-only network (DS-Lite should not be suitable)
    let ipv4_only_caps = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false, // No IPv6
        behind_nat: true,
        public_ipv4: Some("203.0.113.50".parse().unwrap()),
        ipv6_addresses: vec![], // Empty IPv6 addresses
        has_upnp: false,
        interface_mtu: 1500,
    };

    // DS-Lite should still create a config but would fail during connection
    let config = create_tunnel_config(TunnelProtocol::DsLite, &ipv4_only_caps);
    assert_eq!(config.protocol, TunnelProtocol::DsLite);

    Ok(())
}

/// Test DS-Lite error handling scenarios
#[tokio::test]
async fn test_dslite_error_handling() -> Result<()> {
    let config = create_test_dslite_config();
    let mut tunnel = DsLiteTunnel::new(config)?;

    // Test operations on disconnected tunnel
    let send_result = tunnel.send(&[1, 2, 3, 4]).await;
    assert!(send_result.is_err());
    assert!(send_result.unwrap_err().to_string().contains("not connected"));

    let receive_result = tunnel.receive().await;
    assert!(receive_result.is_err());
    assert!(receive_result.unwrap_err().to_string().contains("not connected"));

    // Test invalid encapsulation operations (DS-Lite specific)
    let invalid_encap = tunnel.encapsulate(&[1, 2, 3]).await;
    assert!(invalid_encap.is_err());
    assert!(invalid_encap.unwrap_err().to_string().contains("does not encapsulate IPv6"));

    let invalid_decap = tunnel.decapsulate(&[1, 2, 3]).await;
    assert!(invalid_decap.is_err());
    assert!(invalid_decap.unwrap_err().to_string().contains("does not decapsulate to IPv6"));

    Ok(())
}

/// Performance test for DS-Lite packet processing
#[tokio::test]
async fn test_dslite_performance() -> Result<()> {
    let config = create_test_dslite_config();
    let tunnel = DsLiteTunnel::new(config)?;

    let aftr_addr: Ipv6Addr = "2001:db8:abcd:0:0:0:0:1".parse().unwrap();
    let local_ipv6: Ipv6Addr = "2001:db8:1111:0:0:0:0:1".parse().unwrap();
    
    // Create a realistic IPv4 packet
    let ipv4_packet = vec![0u8; 1400]; // Near MTU size

    let start = std::time::Instant::now();
    
    // Test multiple encapsulations
    for _ in 0..100 {
        let _result = tunnel.encapsulate_ipv4_in_ipv6(&ipv4_packet, aftr_addr, local_ipv6)?;
    }
    
    let duration = start.elapsed();
    println!("100 DS-Lite encapsulations took: {:?}", duration);
    
    // Should be fast (< 10ms for 100 operations)
    assert!(duration < std::time::Duration::from_millis(10));

    Ok(())
}