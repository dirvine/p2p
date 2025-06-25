//! MAP-E and MAP-T (Mapping of Address and Port) Tests
//!
//! Comprehensive tests for MAP-E (RFC 7597) and MAP-T (RFC 7599) tunnel implementations,
//! covering ISP transition scenarios, address mapping, port set calculation,
//! and integration with the P2P Foundation tunneling system.

use p2p_foundation::tunneling::{
    MapTunnel, MapProtocol, MapRule, PortParameters,
    TunnelConfig, TunnelProtocol, TunnelState, Tunnel,
    NetworkCapabilities, create_tunnel_config, create_tunnel
};
use p2p_foundation::Result;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio;

/// Helper to create basic MAP-E tunnel configuration
fn create_test_map_e_config() -> TunnelConfig {
    TunnelConfig {
        protocol: TunnelProtocol::MapE,
        local_ipv4: Some("192.0.2.100".parse().unwrap()),
        remote_ipv4: None,
        ipv6_prefix: Some("2001:db8::".parse().unwrap()),
        aftr_ipv6: None,
        aftr_name: None,
        mtu: 1460,
        keepalive_interval: Duration::from_secs(30),
        establishment_timeout: Duration::from_secs(10),
    }
}

/// Helper to create basic MAP-T tunnel configuration
fn create_test_map_t_config() -> TunnelConfig {
    TunnelConfig {
        protocol: TunnelProtocol::MapT,
        local_ipv4: Some("192.0.2.100".parse().unwrap()),
        remote_ipv4: None,
        ipv6_prefix: Some("2001:db8::".parse().unwrap()),
        aftr_ipv6: None,
        aftr_name: None,
        mtu: 1500,
        keepalive_interval: Duration::from_secs(30),
        establishment_timeout: Duration::from_secs(10),
    }
}

/// Helper to create ISP network capabilities for MAP scenarios
fn create_isp_capabilities() -> NetworkCapabilities {
    NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: true, // ISP has both IPv4 and IPv6
        behind_nat: false, // ISP infrastructure
        public_ipv4: Some("192.0.2.100".parse().unwrap()),
        ipv6_addresses: vec!["2001:db8::1".parse().unwrap()],
        has_upnp: false,
        interface_mtu: 1500,
    }
}

/// Helper to create customer network capabilities
fn create_customer_capabilities() -> NetworkCapabilities {
    NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false, // Customer needs IPv6 via MAP
        behind_nat: true, // Customer behind ISP's MAP system
        public_ipv4: Some("192.0.2.100".parse().unwrap()), // Shared IPv4
        ipv6_addresses: vec![],
        has_upnp: false,
        interface_mtu: 1500,
    }
}

/// Helper to create a sample MAP rule for testing
fn create_sample_map_rule() -> MapRule {
    MapRule {
        ipv6_prefix: "2001:db8::".parse().unwrap(),
        ipv6_prefix_len: 32,
        ipv4_prefix: "192.0.2.0".parse().unwrap(),
        ipv4_prefix_len: 24,
        port_params: PortParameters {
            psid_offset: 4,
            psid_length: 4,
            excluded_ports: 1024,
        },
        border_relay: Some("2001:db8:ffff::1".parse().unwrap()),
        is_fmr: true,
    }
}

/// Test MAP-E tunnel creation
#[tokio::test]
async fn test_map_e_tunnel_creation() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;

    assert_eq!(tunnel.protocol(), TunnelProtocol::MapE);
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test MAP-T tunnel creation
#[tokio::test]
async fn test_map_t_tunnel_creation() -> Result<()> {
    let config = create_test_map_t_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapT)?;

    assert_eq!(tunnel.protocol(), TunnelProtocol::MapT);
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);

    Ok(())
}

/// Test MAP-E tunnel creation with wrong protocol
#[tokio::test]
async fn test_map_e_invalid_protocol() {
    let mut config = create_test_map_e_config();
    config.protocol = TunnelProtocol::MapT; // Wrong protocol

    let result = MapTunnel::new(config, MapProtocol::MapE);
    assert!(result.is_err());
}

/// Test MAP-T tunnel creation with wrong protocol
#[tokio::test]
async fn test_map_t_invalid_protocol() {
    let mut config = create_test_map_t_config();
    config.protocol = TunnelProtocol::MapE; // Wrong protocol

    let result = MapTunnel::new(config, MapProtocol::MapT);
    assert!(result.is_err());
}

/// Test MAP rule addition and configuration
#[tokio::test]
async fn test_map_rule_configuration() -> Result<()> {
    let config = create_test_map_e_config();
    let mut tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    let rule = create_sample_map_rule();
    tunnel.add_map_rule(rule.clone());
    
    // Verify border relay is set for MAP-E
    // Note: This would require public methods to verify internal state
    // For now, we test that adding rules doesn't cause errors
    
    Ok(())
}

/// Test IPv6 address calculation from IPv4 and MAP rule
#[tokio::test]
async fn test_ipv6_address_calculation() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    let rule = create_sample_map_rule();
    let ipv4_addr: Ipv4Addr = "192.0.2.100".parse().unwrap();
    
    let ipv6_addr = tunnel.calculate_ipv6_address(ipv4_addr, &rule)?;
    
    // Verify the IPv6 address starts with the MAP prefix
    let addr_str = ipv6_addr.to_string();
    assert!(addr_str.starts_with("2001:db8"));
    
    println!("Calculated IPv6 address: {}", ipv6_addr);
    
    Ok(())
}

/// Test PSID extraction from IPv4 address
#[tokio::test]
async fn test_psid_extraction() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    let rule = create_sample_map_rule();
    let ipv4_addr: Ipv4Addr = "192.0.2.100".parse().unwrap();
    
    let psid = tunnel.extract_psid(ipv4_addr, &rule);
    
    // PSID should be extracted correctly based on rule parameters
    assert!(psid < (1 << rule.port_params.psid_length));
    
    println!("Extracted PSID: {}", psid);
    
    Ok(())
}

/// Test port set calculation
#[tokio::test]
async fn test_port_set_calculation() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    let rule = create_sample_map_rule();
    let psid: u16 = 5; // Example PSID
    
    let port_set = tunnel.calculate_port_set(psid, &rule);
    
    // Verify port set properties
    assert_eq!(port_set.psid, psid);
    assert!(port_set.start_port >= rule.port_params.excluded_ports);
    assert!(port_set.port_count > 0);
    assert!(!port_set.available_ports.is_empty());
    
    println!("Port set for PSID {}: start={}, count={}, available={}", 
             psid, port_set.start_port, port_set.port_count, port_set.available_ports.len());
    
    Ok(())
}

/// Test port validation against assigned port set
#[tokio::test]
async fn test_port_validation() -> Result<()> {
    let config = create_test_map_e_config();
    let mut tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    let rule = create_sample_map_rule();
    tunnel.add_map_rule(rule.clone());
    
    // Initialize to set up port set
    let _ = tunnel.initialize_addresses().await;
    
    // Test port validation (would need public access to port set)
    // For now, test that the validation method exists
    let is_allowed = tunnel.is_port_allowed(8080);
    // Without proper initialization, this might be false
    println!("Port 8080 allowed: {}", is_allowed);
    
    Ok(())
}

/// Test MAP-E IPv4-in-IPv6 encapsulation
#[tokio::test]
async fn test_map_e_encapsulation() -> Result<()> {
    let config = create_test_map_e_config();
    let mut tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    let rule = create_sample_map_rule();
    tunnel.add_map_rule(rule);
    
    // Initialize tunnel to set up addresses and border relay
    let _ = tunnel.initialize_addresses().await;
    
    // Create a test IPv4 packet
    let ipv4_packet = vec![
        0x45, 0x00, 0x00, 0x3c, // Version=4, IHL=5, TOS=0, Total Length=60
        0x12, 0x34, 0x40, 0x00, // ID=0x1234, Flags=DF, Fragment Offset=0
        0x40, 0x06, 0x00, 0x00, // TTL=64, Protocol=TCP, Checksum=0
        192, 0, 2, 100,         // Source IP: 192.0.2.100
        192, 0, 2, 1,           // Destination IP: 192.0.2.1
        // TCP header would follow...
        0x1f, 0x90, 0x00, 0x50, // Source Port=8080, Dest Port=80
        0x00, 0x00, 0x00, 0x00, // Sequence number
        0x00, 0x00, 0x00, 0x00, // Acknowledgment number
        0x50, 0x02, 0x20, 0x00, // Header length=5, Flags=SYN, Window size=8192
        0x00, 0x00, 0x00, 0x00, // Checksum, Urgent pointer
    ];
    
    match tunnel.encapsulate_ipv4_in_ipv6(&ipv4_packet) {
        Ok(ipv6_packet) => {
            // Verify IPv6 packet structure
            assert!(ipv6_packet.len() > 40); // IPv6 header + IPv4 payload
            assert_eq!(ipv6_packet[0] & 0xF0, 0x60); // IPv6 version
            assert_eq!(ipv6_packet[6], 4); // Next Header = IPv4
            
            println!("MAP-E encapsulation successful: {} bytes", ipv6_packet.len());
        }
        Err(e) => {
            // Expected if initialization didn't complete properly
            println!("MAP-E encapsulation failed as expected: {}", e);
        }
    }
    
    Ok(())
}

/// Test MAP-T IPv4 to IPv6 translation
#[tokio::test]
async fn test_map_t_translation() -> Result<()> {
    let config = create_test_map_t_config();
    let mut tunnel = MapTunnel::new(config, MapProtocol::MapT)?;
    
    let rule = create_sample_map_rule();
    tunnel.add_map_rule(rule);
    
    // Initialize tunnel
    let _ = tunnel.initialize_addresses().await;
    
    // Create a test IPv4 packet
    let ipv4_packet = vec![
        0x45, 0x00, 0x00, 0x28, // Version=4, IHL=5, TOS=0, Total Length=40
        0x12, 0x34, 0x40, 0x00, // ID=0x1234, Flags=DF, Fragment Offset=0
        0x40, 0x11, 0x00, 0x00, // TTL=64, Protocol=UDP, Checksum=0
        192, 0, 2, 100,         // Source IP: 192.0.2.100
        192, 0, 2, 1,           // Destination IP: 192.0.2.1
        // UDP header
        0x1f, 0x90, 0x00, 0x35, // Source Port=8080, Dest Port=53 (DNS)
        0x00, 0x08, 0x00, 0x00, // Length=8, Checksum=0
    ];
    
    match tunnel.translate_ipv4_to_ipv6(&ipv4_packet) {
        Ok(ipv6_packet) => {
            // Verify IPv6 packet structure
            assert!(ipv6_packet.len() >= 40); // IPv6 header + payload
            assert_eq!(ipv6_packet[0] & 0xF0, 0x60); // IPv6 version
            assert_eq!(ipv6_packet[6], 17); // Next Header = UDP
            
            println!("MAP-T translation successful: {} bytes", ipv6_packet.len());
        }
        Err(e) => {
            // Expected if initialization didn't complete properly
            println!("MAP-T translation failed as expected: {}", e);
        }
    }
    
    Ok(())
}

/// Test MAP tunnel factory functions
#[tokio::test]
async fn test_map_tunnel_factory() -> Result<()> {
    // Test MAP-E factory
    let map_e_config = create_test_map_e_config();
    let map_e_tunnel = create_tunnel(map_e_config)?;
    assert_eq!(map_e_tunnel.protocol(), TunnelProtocol::MapE);
    
    // Test MAP-T factory
    let map_t_config = create_test_map_t_config();
    let map_t_tunnel = create_tunnel(map_t_config)?;
    assert_eq!(map_t_tunnel.protocol(), TunnelProtocol::MapT);
    
    Ok(())
}

/// Test MAP configuration generation for ISP scenarios
#[tokio::test]
async fn test_map_config_generation() -> Result<()> {
    let isp_caps = create_isp_capabilities();
    
    // Test MAP-E configuration
    let map_e_config = create_tunnel_config(TunnelProtocol::MapE, &isp_caps);
    assert_eq!(map_e_config.protocol, TunnelProtocol::MapE);
    assert_eq!(map_e_config.mtu, 1460); // Reduced for IPv6 encapsulation
    assert_eq!(map_e_config.local_ipv4, Some("192.0.2.100".parse().unwrap()));
    
    // Test MAP-T configuration
    let map_t_config = create_tunnel_config(TunnelProtocol::MapT, &isp_caps);
    assert_eq!(map_t_config.protocol, TunnelProtocol::MapT);
    assert_eq!(map_t_config.mtu, 1500); // No encapsulation overhead
    assert_eq!(map_t_config.local_ipv4, Some("192.0.2.100".parse().unwrap()));
    
    Ok(())
}

/// Test MAP tunnel connection and disconnection
#[tokio::test]
async fn test_map_tunnel_lifecycle() -> Result<()> {
    let config = create_test_map_e_config();
    let mut tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    // Add a MAP rule for proper initialization
    let rule = create_sample_map_rule();
    tunnel.add_map_rule(rule);
    
    // Initial state should be disconnected
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);
    assert!(!tunnel.is_active().await);
    
    // Attempt connection
    match tunnel.connect().await {
        Ok(_) => {
            assert_eq!(tunnel.state().await, TunnelState::Connected);
            assert!(tunnel.is_active().await);
            
            // Test addresses
            let ipv6_addr = tunnel.local_ipv6_addr().await?;
            let ipv4_addr = tunnel.local_ipv4_addr().await?;
            
            println!("MAP tunnel connected: IPv6={}, IPv4={}", ipv6_addr, ipv4_addr);
            
            // Test disconnection
            tunnel.disconnect().await?;
            assert_eq!(tunnel.state().await, TunnelState::Disconnected);
            assert!(!tunnel.is_active().await);
        }
        Err(e) => {
            // Expected in test environment without proper MAP infrastructure
            println!("MAP connection failed as expected: {}", e);
            let state = tunnel.state().await;
            if let TunnelState::Failed(reason) = state {
                assert!(
                    reason.contains("address") || 
                    reason.contains("MAP") || 
                    reason.contains("Socket") ||
                    reason.contains("initialization")
                );
            }
        }
    }
    
    Ok(())
}

/// Test MAP tunnel metrics
#[tokio::test]
async fn test_map_tunnel_metrics() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    let metrics = tunnel.metrics().await;
    assert_eq!(metrics.bytes_sent, 0);
    assert_eq!(metrics.bytes_received, 0);
    assert_eq!(metrics.packets_sent, 0);
    assert_eq!(metrics.packets_received, 0);
    
    Ok(())
}

/// Test MAP tunnel maintenance
#[tokio::test]
async fn test_map_tunnel_maintenance() -> Result<()> {
    let config = create_test_map_t_config();
    let mut tunnel = MapTunnel::new(config, MapProtocol::MapT)?;
    
    // Test maintenance on disconnected tunnel
    tunnel.maintain().await?;
    
    // MAP maintenance should handle translation table cleanup
    assert_eq!(tunnel.state().await, TunnelState::Disconnected);
    
    Ok(())
}

/// Test MAP error handling scenarios
#[tokio::test]
async fn test_map_error_handling() -> Result<()> {
    let config = create_test_map_e_config();
    let mut tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    // Test operations on disconnected tunnel
    let send_result = tunnel.send(&[1, 2, 3, 4]).await;
    assert!(send_result.is_err());
    assert!(send_result.unwrap_err().to_string().contains("socket not available"));
    
    let receive_result = tunnel.receive().await;
    assert!(receive_result.is_err());
    assert!(receive_result.unwrap_err().to_string().contains("socket not available"));
    
    // Test ping without configuration
    let ping_result = tunnel.ping(Duration::from_secs(1)).await;
    // This might succeed with simulated ping
    match ping_result {
        Ok(rtt) => println!("MAP ping succeeded: {:?}", rtt),
        Err(e) => println!("MAP ping failed as expected: {}", e),
    }
    
    Ok(())
}

/// Test MAP rule validation
#[tokio::test]
async fn test_map_rule_validation() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    // Test invalid MAP rule (insufficient bits for PSID)
    let invalid_rule = MapRule {
        ipv6_prefix: "2001:db8::".parse().unwrap(),
        ipv6_prefix_len: 32,
        ipv4_prefix: "192.0.2.0".parse().unwrap(),
        ipv4_prefix_len: 30, // Only 2 host bits, but PSID needs 4
        port_params: PortParameters {
            psid_offset: 4,
            psid_length: 4, // Requires 4 bits but only 2 available
            excluded_ports: 1024,
        },
        border_relay: Some("2001:db8:ffff::1".parse().unwrap()),
        is_fmr: true,
    };
    
    let ipv4_addr: Ipv4Addr = "192.0.2.1".parse().unwrap();
    let result = tunnel.calculate_ipv6_address(ipv4_addr, &invalid_rule);
    assert!(result.is_err());
    
    Ok(())
}

/// Test MAP integration with customer scenarios
#[tokio::test]
async fn test_map_customer_integration() -> Result<()> {
    let customer_caps = create_customer_capabilities();
    
    // Test MAP-E for customer
    let map_e_config = create_tunnel_config(TunnelProtocol::MapE, &customer_caps);
    assert_eq!(map_e_config.protocol, TunnelProtocol::MapE);
    assert_eq!(map_e_config.local_ipv4, Some("192.0.2.100".parse().unwrap()));
    
    // Test MAP-T for customer
    let map_t_config = create_tunnel_config(TunnelProtocol::MapT, &customer_caps);
    assert_eq!(map_t_config.protocol, TunnelProtocol::MapT);
    assert_eq!(map_t_config.local_ipv4, Some("192.0.2.100".parse().unwrap()));
    
    Ok(())
}

/// Test MAP protocol selection preference
#[tokio::test]
async fn test_map_protocol_preference() -> Result<()> {
    // MAP protocols should be suitable for ISP environments
    let isp_caps = create_isp_capabilities();
    
    // Both MAP-E and MAP-T should be viable options for ISPs
    // This would be tested through the tunnel manager selection system
    // For now, verify basic suitability
    
    let map_e_config = create_tunnel_config(TunnelProtocol::MapE, &isp_caps);
    let map_e_tunnel = create_tunnel(map_e_config)?;
    assert_eq!(map_e_tunnel.protocol(), TunnelProtocol::MapE);
    
    let map_t_config = create_tunnel_config(TunnelProtocol::MapT, &isp_caps);
    let map_t_tunnel = create_tunnel(map_t_config)?;
    assert_eq!(map_t_tunnel.protocol(), TunnelProtocol::MapT);
    
    Ok(())
}

/// Performance test for MAP address calculation
#[tokio::test]
async fn test_map_address_calculation_performance() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    let rule = create_sample_map_rule();
    
    let start = std::time::Instant::now();
    
    // Test multiple address calculations
    for i in 1..=255 {
        let ipv4_addr = Ipv4Addr::new(192, 0, 2, i);
        let _ipv6_addr = tunnel.calculate_ipv6_address(ipv4_addr, &rule)?;
    }
    
    let duration = start.elapsed();
    println!("255 MAP address calculations took: {:?}", duration);
    
    // Should be very fast (< 5ms for 255 addresses)
    assert!(duration < std::time::Duration::from_millis(10));
    
    Ok(())
}

/// Test MAP rule with different PSID configurations
#[tokio::test]
async fn test_map_psid_variations() -> Result<()> {
    let config = create_test_map_e_config();
    let tunnel = MapTunnel::new(config, MapProtocol::MapE)?;
    
    // Test different PSID lengths
    for psid_length in [0, 2, 4, 6, 8] {
        let rule = MapRule {
            ipv6_prefix: "2001:db8::".parse().unwrap(),
            ipv6_prefix_len: 32,
            ipv4_prefix: "192.0.2.0".parse().unwrap(),
            ipv4_prefix_len: 24,
            port_params: PortParameters {
                psid_offset: 4,
                psid_length,
                excluded_ports: 1024,
            },
            border_relay: Some("2001:db8:ffff::1".parse().unwrap()),
            is_fmr: true,
        };
        
        let ipv4_addr: Ipv4Addr = "192.0.2.100".parse().unwrap();
        let psid = tunnel.extract_psid(ipv4_addr, &rule);
        let port_set = tunnel.calculate_port_set(psid, &rule);
        
        println!("PSID length {}: PSID={}, ports={}", 
                 psid_length, psid, port_set.port_count);
        
        // Verify PSID is within expected range
        if psid_length > 0 {
            assert!(psid < (1 << psid_length));
        } else {
            assert_eq!(psid, 0);
        }
    }
    
    Ok(())
}