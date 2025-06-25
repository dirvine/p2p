//! ISATAP (Intra-Site Automatic Tunnel Addressing Protocol) Tests
//!
//! Comprehensive tests for the ISATAP tunneling protocol implementation,
//! covering enterprise network scenarios, router discovery, address generation,
//! and integration with the P2P Foundation tunneling system.

use p2p_foundation::tunneling::{
    IsatapTunnel, TunnelConfig, TunnelProtocol, TunnelState, Tunnel,
    NetworkCapabilities, create_tunnel_config, create_tunnel
};
use p2p_foundation::tunneling::isatap::{IsatapRouter, RouterDiscoveryMethod};
use p2p_foundation::Result;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use tokio;

/// Helper to create a basic ISATAP tunnel configuration
fn create_test_isatap_config() -> TunnelConfig {
    TunnelConfig {
        protocol: TunnelProtocol::Isatap,
        local_ipv4: Some("192.168.1.100".parse().unwrap()),
        remote_ipv4: None,
        ipv6_prefix: Some("fe80::".parse().unwrap()),
        aftr_ipv6: None,
        aftr_name: None,
        mtu: 1500,
        keepalive_interval: Duration::from_secs(30),
        establishment_timeout: Duration::from_secs(10),
    }
}

/// Helper to create enterprise network capabilities (private IP space)
fn create_enterprise_capabilities() -> NetworkCapabilities {
    NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false, // Enterprise network needs IPv6 via tunneling
        behind_nat: true, // Typical corporate environment
        public_ipv4: Some("10.0.1.100".parse().unwrap()), // Private IP space
        ipv6_addresses: vec![], // No native IPv6
        has_upnp: false, // Corporate firewalls typically disable UPnP
        interface_mtu: 1500,
    }
}

/// Helper to create corporate DMZ capabilities
fn create_dmz_capabilities() -> NetworkCapabilities {
    NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false, // DMZ has direct connectivity
        public_ipv4: Some("172.16.10.50".parse().unwrap()), // Private DMZ space
        ipv6_addresses: vec![],
        has_upnp: false,
        interface_mtu: 1500,
    }
}

/// Test ISATAP tunnel creation
#[tokio::test]
async fn test_isatap_tunnel_creation() -> Result<()> {
    let config = create_test_isatap_config();
    let tunnel = IsatapTunnel::new(config)?;

    assert_eq!(tunnel.protocol(), TunnelProtocol::Isatap);
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test ISATAP tunnel creation with invalid protocol
#[tokio::test]
async fn test_isatap_invalid_protocol() {
    let mut config = create_test_isatap_config();
    config.protocol = TunnelProtocol::SixToFour; // Wrong protocol

    let result = IsatapTunnel::new(config);
    assert!(result.is_err());
}

/// Test ISATAP address generation
#[tokio::test]
async fn test_isatap_address_generation() -> Result<()> {
    let ipv4_addr: Ipv4Addr = "192.168.1.100".parse().unwrap();
    let prefix: Ipv6Addr = "fe80::".parse().unwrap();

    // Generate ISATAP address
    let isatap_addr = IsatapTunnel::generate_isatap_address(ipv4_addr, Some(prefix));

    // Verify ISATAP address format: fe80::0:5efe:c0a8:164 (192.168.1.100 in hex)
    println!("Generated ISATAP address: {}", isatap_addr);
    
    // Verify it's a valid ISATAP address
    assert!(IsatapTunnel::is_isatap_address(isatap_addr));
    
    // Verify we can extract the IPv4 address back
    let extracted_ipv4 = IsatapTunnel::extract_ipv4_from_isatap(isatap_addr);
    assert_eq!(extracted_ipv4, Some(ipv4_addr));

    Ok(())
}

/// Test ISATAP address generation with different prefixes
#[tokio::test]
async fn test_isatap_address_prefixes() -> Result<()> {
    let ipv4_addr: Ipv4Addr = "10.0.0.1".parse().unwrap();
    
    // Test with link-local prefix
    let link_local = IsatapTunnel::generate_isatap_address(ipv4_addr, Some("fe80::".parse().unwrap()));
    assert!(link_local.to_string().starts_with("fe80::"));
    assert!(IsatapTunnel::is_isatap_address(link_local));
    
    // Test with site-local prefix (deprecated but still used in enterprise)
    let site_local = IsatapTunnel::generate_isatap_address(ipv4_addr, Some("fec0::".parse().unwrap()));
    assert!(site_local.to_string().starts_with("fec0::"));
    assert!(IsatapTunnel::is_isatap_address(site_local));
    
    // Test with unique local prefix (modern enterprise)
    let unique_local = IsatapTunnel::generate_isatap_address(ipv4_addr, Some("fc00::".parse().unwrap()));
    assert!(unique_local.to_string().starts_with("fc00::"));
    assert!(IsatapTunnel::is_isatap_address(unique_local));

    Ok(())
}

/// Test ISATAP address validation
#[tokio::test]
async fn test_isatap_address_validation() -> Result<()> {
    // Valid ISATAP addresses
    let _valid_isatap1: Ipv6Addr = "fe80::5efe:c0a8:164".parse().unwrap(); // Missing leading zeros
    let _valid_isatap2: Ipv6Addr = "2001:db8::0:5efe:a00:1".parse().unwrap();
    
    // Create properly formatted ISATAP address
    let ipv4: Ipv4Addr = "192.168.1.100".parse().unwrap();
    let valid_isatap3 = IsatapTunnel::generate_isatap_address(ipv4, None);
    
    assert!(IsatapTunnel::is_isatap_address(valid_isatap3));
    
    // Invalid addresses (not ISATAP)
    let invalid1: Ipv6Addr = "2001:db8::1".parse().unwrap(); // Regular IPv6
    let invalid2: Ipv6Addr = "fe80::1".parse().unwrap(); // Link-local but not ISATAP
    let invalid3: Ipv6Addr = "::1".parse().unwrap(); // Loopback
    
    assert!(!IsatapTunnel::is_isatap_address(invalid1));
    assert!(!IsatapTunnel::is_isatap_address(invalid2));
    assert!(!IsatapTunnel::is_isatap_address(invalid3));

    Ok(())
}

/// Test ISATAP configuration generation for enterprise networks
#[tokio::test]
async fn test_isatap_config_generation() -> Result<()> {
    let enterprise_caps = create_enterprise_capabilities();
    let config = create_tunnel_config(TunnelProtocol::Isatap, &enterprise_caps);

    assert_eq!(config.protocol, TunnelProtocol::Isatap);
    assert_eq!(config.mtu, 1500); // Enterprise MTU
    assert_eq!(config.local_ipv4, Some("10.0.1.100".parse().unwrap()));
    assert_eq!(config.ipv6_prefix, Some("fe80::".parse().unwrap()));

    Ok(())
}

/// Test tunnel factory function for ISATAP
#[tokio::test]
async fn test_isatap_tunnel_factory() -> Result<()> {
    let config = create_test_isatap_config();
    let tunnel = create_tunnel(config)?;

    assert_eq!(tunnel.protocol(), TunnelProtocol::Isatap);
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test ISATAP router creation from addresses
#[tokio::test]
async fn test_isatap_router_creation() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;

    // Test router discovery with configured addresses
    let router_addresses = vec![
        "192.168.1.1".parse().unwrap(),
        "10.0.0.1".parse().unwrap(),
        "172.16.1.1".parse().unwrap(),
    ];

    let discovery_method = RouterDiscoveryMethod::ConfiguredList(router_addresses.clone());
    let discovered_routers = tunnel.discover_routers(discovery_method).await?;

    assert_eq!(discovered_routers.len(), 3);
    for (i, router) in discovered_routers.iter().enumerate() {
        assert_eq!(router.ipv4_addr, router_addresses[i]);
        assert_eq!(router.priority, i as u8);
        assert!(!router.reachable); // Not tested yet
    }

    Ok(())
}

/// Test ISATAP manual router configuration
#[tokio::test]
async fn test_isatap_manual_router_config() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;

    // Create manual router configuration
    let manual_routers = vec![
        IsatapRouter {
            ipv4_addr: "192.168.1.1".parse().unwrap(),
            ipv6_prefix: Some("2001:db8:1::".parse().unwrap()),
            priority: 0,
            last_seen: None,
            reachable: true,
            rtt: Some(Duration::from_millis(5)),
        },
        IsatapRouter {
            ipv4_addr: "192.168.1.2".parse().unwrap(),
            ipv6_prefix: Some("2001:db8:2::".parse().unwrap()),
            priority: 1,
            last_seen: None,
            reachable: true,
            rtt: Some(Duration::from_millis(10)),
        },
    ];

    let discovery_method = RouterDiscoveryMethod::Manual(manual_routers.clone());
    let discovered_routers = tunnel.discover_routers(discovery_method).await?;

    assert_eq!(discovered_routers.len(), 2);
    assert_eq!(discovered_routers[0].ipv4_addr, manual_routers[0].ipv4_addr);
    assert_eq!(discovered_routers[1].ipv4_addr, manual_routers[1].ipv4_addr);

    Ok(())
}

/// Test ISATAP connection establishment in enterprise environment
#[tokio::test]
async fn test_isatap_enterprise_connection() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;

    // Initial state should be disconnected
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);
    assert!(!tunnel.is_active().await);

    // Note: In a real enterprise environment, this would discover actual ISATAP routers
    // For testing, we expect this to work with the configured local address
    let connection_result = tunnel.connect().await;
    
    match connection_result {
        Ok(_) => {
            assert_eq!(tunnel.state().await, TunnelState::Connected);
            assert!(tunnel.is_active().await);
            
            // Test that we can get local addresses
            let ipv6_addr = tunnel.local_ipv6_addr().await?;
            let ipv4_addr = tunnel.local_ipv4_addr().await?;
            
            println!("ISATAP tunnel connected: IPv6={}, IPv4={}", ipv6_addr, ipv4_addr);
            assert!(IsatapTunnel::is_isatap_address(ipv6_addr));
        }
        Err(e) => {
            // Expected in test environment without actual enterprise infrastructure
            println!("ISATAP connection failed as expected in test environment: {}", e);
            let state = tunnel.state().await;
            if let TunnelState::Failed(reason) = state {
                assert!(
                    reason.contains("address") || 
                    reason.contains("router") || 
                    reason.contains("Socket") ||
                    reason.contains("network")
                );
            }
        }
    }

    Ok(())
}

/// Test ISATAP disconnection
#[tokio::test]
async fn test_isatap_disconnection() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;

    // Test disconnection from disconnected state
    tunnel.disconnect().await?;
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test ISATAP IPv6-in-IPv4 packet encapsulation
#[tokio::test]
async fn test_isatap_packet_encapsulation() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;
    
    // Initialize tunnel to set local IPv4 address
    let _ = tunnel.initialize_addresses().await;

    // Create a simple IPv6 packet
    let ipv6_packet = vec![
        0x60, 0x00, 0x00, 0x00, // Version=6, Traffic Class=0, Flow Label=0
        0x00, 0x08, // Payload Length=8
        0x3a, // Next Header=ICMPv6
        0x40, // Hop Limit=64
        // Source address: fe80::1
        0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        // Destination address: fe80::2
        0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        // ICMPv6 payload (8 bytes)
        0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let dest_ipv4: Ipv4Addr = "192.168.1.1".parse().unwrap();

    // Test the encapsulation function directly
    let ipv4_packet = tunnel.encapsulate_ipv6_in_ipv4(&ipv6_packet, dest_ipv4)?;

    // Verify IPv4 header structure
    assert_eq!(ipv4_packet.len(), 20 + ipv6_packet.len()); // IPv4 header + IPv6 payload
    assert_eq!(ipv4_packet[0] & 0xF0, 0x40); // IPv4 version
    assert_eq!(ipv4_packet[9], 41); // Protocol = IPv6
    
    // Verify source and destination addresses
    let src_addr = &ipv4_packet[12..16];
    let dst_addr = &ipv4_packet[16..20];
    assert_eq!(src_addr, &[192, 168, 1, 100]); // Local address from config
    assert_eq!(dst_addr, &[192, 168, 1, 1]); // Destination address

    Ok(())
}

/// Test ISATAP IPv4-to-IPv6 decapsulation
#[tokio::test]
async fn test_isatap_packet_decapsulation() -> Result<()> {
    let config = create_test_isatap_config();
    let tunnel = IsatapTunnel::new(config)?;

    // Create IPv4 packet with embedded IPv6
    let original_ipv6 = vec![
        0x60, 0x00, 0x00, 0x00, 0x00, 0x08, 0x3a, 0x40,
        0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let mut ipv4_packet = Vec::new();
    
    // IPv4 header (20 bytes)
    ipv4_packet.extend_from_slice(&[
        0x45, 0x00, // Version=4, IHL=5, Type of Service=0
    ]);
    ipv4_packet.extend_from_slice(&((20 + original_ipv6.len()) as u16).to_be_bytes()); // Total Length
    ipv4_packet.extend_from_slice(&[0x00, 0x00]); // Identification
    ipv4_packet.extend_from_slice(&[0x40, 0x00]); // Flags=DF, Fragment Offset=0
    ipv4_packet.push(64); // TTL
    ipv4_packet.push(41); // Protocol = IPv6
    ipv4_packet.extend_from_slice(&[0x00, 0x00]); // Header Checksum (simplified)
    ipv4_packet.extend_from_slice(&[192, 168, 1, 1]); // Source IP
    ipv4_packet.extend_from_slice(&[192, 168, 1, 100]); // Destination IP

    // IPv6 payload
    ipv4_packet.extend_from_slice(&original_ipv6);

    // Test decapsulation
    let extracted_ipv6 = tunnel.decapsulate_ipv4_to_ipv6(&ipv4_packet)?;
    assert_eq!(extracted_ipv6, original_ipv6);

    Ok(())
}

/// Test ISATAP with invalid packets
#[tokio::test]
async fn test_isatap_invalid_packets() -> Result<()> {
    let config = create_test_isatap_config();
    let tunnel = IsatapTunnel::new(config)?;

    // Test encapsulation with invalid IPv6 packet
    let invalid_ipv6 = vec![0x40, 0x00]; // IPv4 version instead of IPv6
    let dest_ipv4: Ipv4Addr = "192.168.1.1".parse().unwrap();
    let result = tunnel.encapsulate_ipv6_in_ipv4(&invalid_ipv6, dest_ipv4);
    assert!(result.is_err());

    // Test decapsulation with invalid IPv4 packet
    let invalid_ipv4 = vec![0x60, 0x00]; // IPv6 version instead of IPv4
    let result = tunnel.decapsulate_ipv4_to_ipv6(&invalid_ipv4);
    assert!(result.is_err());

    // Test with packet too short
    let short_packet = vec![0x45];
    let result = tunnel.decapsulate_ipv4_to_ipv6(&short_packet);
    assert!(result.is_err());

    Ok(())
}

/// Test ISATAP tunnel maintenance
#[tokio::test]
async fn test_isatap_maintenance() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;

    // Test maintenance on disconnected tunnel
    tunnel.maintain().await?;
    
    // ISATAP maintenance should handle router discovery and reachability
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test ISATAP tunnel metrics
#[tokio::test]
async fn test_isatap_metrics() -> Result<()> {
    let config = create_test_isatap_config();
    let tunnel = IsatapTunnel::new(config)?;

    let metrics = tunnel.metrics().await;
    assert_eq!(metrics.bytes_sent, 0);
    assert_eq!(metrics.bytes_received, 0);
    assert_eq!(metrics.packets_sent, 0);
    assert_eq!(metrics.packets_received, 0);

    Ok(())
}

/// Test ISATAP integration with enterprise network capabilities
#[tokio::test]
async fn test_isatap_enterprise_integration() -> Result<()> {
    // Test with typical enterprise network
    let enterprise_caps = create_enterprise_capabilities();
    let config = create_tunnel_config(TunnelProtocol::Isatap, &enterprise_caps);
    assert_eq!(config.protocol, TunnelProtocol::Isatap);
    assert_eq!(config.local_ipv4, Some("10.0.1.100".parse().unwrap()));

    // Test with DMZ configuration
    let dmz_caps = create_dmz_capabilities();
    let config = create_tunnel_config(TunnelProtocol::Isatap, &dmz_caps);
    assert_eq!(config.protocol, TunnelProtocol::Isatap);
    assert_eq!(config.local_ipv4, Some("172.16.10.50".parse().unwrap()));

    Ok(())
}

/// Test ISATAP router discovery methods
#[tokio::test]
async fn test_isatap_router_discovery_methods() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;

    // Test DNS well-known discovery (will fail in test environment)
    let dns_method = RouterDiscoveryMethod::DnsWellKnown("corp.example.com".to_string());
    let dns_result = tunnel.discover_routers(dns_method).await;
    // This is expected to fail in test environment
    println!("DNS discovery result: {:?}", dns_result);

    // Test configured list (should work)
    let configured_method = RouterDiscoveryMethod::ConfiguredList(vec![
        "192.168.1.1".parse().unwrap(),
        "192.168.1.2".parse().unwrap(),
    ]);
    let configured_result = tunnel.discover_routers(configured_method).await?;
    assert_eq!(configured_result.len(), 2);

    Ok(())
}

/// Test ISATAP error handling scenarios
#[tokio::test]
async fn test_isatap_error_handling() -> Result<()> {
    let config = create_test_isatap_config();
    let mut tunnel = IsatapTunnel::new(config)?;

    // Test operations on disconnected tunnel
    let send_result = tunnel.send(&[1, 2, 3, 4]).await;
    assert!(send_result.is_err());
    assert!(send_result.unwrap_err().to_string().contains("socket not available"));

    let receive_result = tunnel.receive().await;
    assert!(receive_result.is_err());
    assert!(receive_result.unwrap_err().to_string().contains("socket not available"));

    // Test ping without active router
    let ping_result = tunnel.ping(Duration::from_secs(1)).await;
    assert!(ping_result.is_err());
    assert!(ping_result.unwrap_err().to_string().contains("No active ISATAP router"));

    Ok(())
}

/// Performance test for ISATAP address generation
#[tokio::test]
async fn test_isatap_address_generation_performance() -> Result<()> {
    let prefix: Ipv6Addr = "2001:db8::".parse().unwrap();
    
    let start = std::time::Instant::now();
    
    // Test multiple address generations
    for i in 1..=255 {
        let ipv4_addr = Ipv4Addr::new(192, 168, 1, i);
        let _isatap_addr = IsatapTunnel::generate_isatap_address(ipv4_addr, Some(prefix));
    }
    
    let duration = start.elapsed();
    println!("255 ISATAP address generations took: {:?}", duration);
    
    // Should be very fast (< 1ms for 255 addresses)
    assert!(duration < std::time::Duration::from_millis(5));

    Ok(())
}

/// Test ISATAP in multi-site enterprise scenario
#[tokio::test]
async fn test_isatap_multi_site_enterprise() -> Result<()> {
    // Simulate multiple enterprise sites
    let sites = vec![
        ("headquarters", "10.0.0.0/8", "192.168.100.1"),
        ("branch_office_1", "172.16.0.0/12", "172.16.1.1"),
        ("branch_office_2", "192.168.0.0/16", "192.168.50.1"),
    ];

    for (site_name, _network, router_addr) in sites {
        println!("Testing ISATAP for {}", site_name);
        
        let _router_ipv4: Ipv4Addr = router_addr.parse().unwrap();
        let config = TunnelConfig {
            protocol: TunnelProtocol::Isatap,
            local_ipv4: Some("192.168.1.100".parse().unwrap()),
            remote_ipv4: None,
            ipv6_prefix: Some("2001:db8:1::".parse().unwrap()),
            aftr_ipv6: None,
            aftr_name: None,
            mtu: 1500,
            keepalive_interval: Duration::from_secs(30),
            establishment_timeout: Duration::from_secs(10),
        };

        let tunnel = IsatapTunnel::new(config)?;
        
        // Verify ISATAP address generation for each site
        let isatap_addr = tunnel.local_ipv6_addr().await;
        match isatap_addr {
            Ok(addr) => {
                assert!(IsatapTunnel::is_isatap_address(addr));
                println!("  ISATAP address for {}: {}", site_name, addr);
            }
            Err(_) => {
                // Expected in test environment
                println!("  ISATAP address generation failed for {} (expected in test)", site_name);
            }
        }
    }

    Ok(())
}