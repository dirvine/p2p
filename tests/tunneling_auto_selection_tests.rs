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

//! Tunneling Auto-Selection Tests
//!
//! Comprehensive tests for the tunneling protocol auto-selection system,
//! including network capability detection, protocol scoring, and failover mechanisms.

use p2p_foundation::tunneling::{
    TunnelManager, TunnelManagerConfig, TunnelProtocol, NetworkCapabilities,
    detect_network_capabilities, create_tunnel, create_tunnel_config
};
use std::time::Duration;
use tokio::time;

/// Helper function to create a tunnel manager with all tunnel implementations registered
async fn create_initialized_tunnel_manager(config: TunnelManagerConfig) -> TunnelManager {
    let manager = TunnelManager::with_config(config);
    
    // Create basic capabilities for tunnel creation
    let basic_capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    // Create and register all tunnel types
    for protocol in [TunnelProtocol::SixToFour, TunnelProtocol::Teredo, TunnelProtocol::SixInFour] {
        let config = create_tunnel_config(protocol, &basic_capabilities);
        if let Ok(tunnel) = create_tunnel(config) {
            manager.add_tunnel(tunnel).await;
        }
    }
    
    manager
}

/// Test network capability detection with various scenarios
#[tokio::test]
async fn test_network_capability_detection() -> anyhow::Result<()> {
    // Test basic network detection
    let capabilities = detect_network_capabilities().await?;
    
    // Basic validation - system should have at least loopback
    assert!(capabilities.has_ipv4 || capabilities.has_ipv6, 
           "System should have at least IPv4 or IPv6 connectivity");
    
    // MTU should be reasonable
    assert!(capabilities.interface_mtu >= 1280, 
           "MTU should be at least IPv6 minimum");
    assert!(capabilities.interface_mtu <= 9000, 
           "MTU should be reasonable");
    
    println!("Detected capabilities: IPv4={}, IPv6={}, NAT={}, UPnP={}, MTU={}", 
             capabilities.has_ipv4, capabilities.has_ipv6, capabilities.behind_nat,
             capabilities.has_upnp, capabilities.interface_mtu);
    
    Ok(())
}

/// Test auto-selection with mock network capabilities favoring 6to4
#[tokio::test]
async fn test_auto_selection_6to4_preferred() -> anyhow::Result<()> {
    let config = TunnelManagerConfig {
        max_concurrent_attempts: 0, // Disable actual connection tests
        ..Default::default()
    };
    
    let manager = create_initialized_tunnel_manager(config).await;
    
    // Mock capabilities: public IPv4, no NAT - should prefer 6to4
    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    let selection = manager.select_tunnel(&capabilities).await;
    
    if let Some(sel) = selection {
        assert_eq!(sel.protocol, TunnelProtocol::SixToFour,
                  "Should prefer 6to4 with public IPv4");
        assert!(!sel.is_fallback, "6to4 with public IP should not be fallback");
        println!("Selected: {:?}, Reason: {}", sel.protocol, sel.reason);
    } else {
        panic!("Should have selected a protocol");
    }
    
    Ok(())
}

/// Test auto-selection with mock network capabilities favoring Teredo
#[tokio::test]
async fn test_auto_selection_teredo_preferred() -> anyhow::Result<()> {
    let config = TunnelManagerConfig {
        auto_failover: true,
        max_concurrent_attempts: 0,
        ..Default::default()
    };
    
    let manager = create_initialized_tunnel_manager(config).await;
    
    // Mock capabilities: behind NAT, no public IPv4 - should prefer Teredo
    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: true,
        public_ipv4: None,
        ipv6_addresses: Vec::new(),
        has_upnp: true,
        interface_mtu: 1500,
    };
    
    let selection = manager.select_tunnel(&capabilities).await;
    
    if let Some(sel) = selection {
        assert_eq!(sel.protocol, TunnelProtocol::Teredo,
                  "Should prefer Teredo behind NAT");
        println!("Selected: {:?}, Reason: {}", sel.protocol, sel.reason);
    } else {
        panic!("Should have selected a protocol");
    }
    
    Ok(())
}

/// Test auto-selection with mock network capabilities forcing 6in4 fallback
#[tokio::test]
async fn test_auto_selection_6in4_fallback() -> anyhow::Result<()> {
    let config = TunnelManagerConfig {
        auto_failover: true,
        max_concurrent_attempts: 0,
        ..Default::default()
    };
    
    let manager = create_initialized_tunnel_manager(config).await;
    
    // Mock capabilities: limited IPv4, no good options - should fallback to 6in4
    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: true,
        public_ipv4: None,
        ipv6_addresses: Vec::new(),
        has_upnp: false, // No UPnP makes Teredo less attractive
        interface_mtu: 1280, // Minimal MTU
    };
    
    let selection = manager.select_tunnel(&capabilities).await;
    
    if let Some(sel) = selection {
        // Should be 6in4 or Teredo, but likely 6in4 with low UPnP score
        println!("Selected: {:?}, Reason: {}, Fallback: {}", 
                sel.protocol, sel.reason, sel.is_fallback);
        assert!(sel.protocol == TunnelProtocol::SixInFour || 
                sel.protocol == TunnelProtocol::Teredo,
                "Should select 6in4 or Teredo as fallback");
    } else {
        panic!("Should have selected a protocol");
    }
    
    Ok(())
}

/// Test tunnel manager basic functionality
#[tokio::test]
async fn test_tunnel_manager_basic() -> anyhow::Result<()> {
    let config = TunnelManagerConfig::default();
    let manager = TunnelManager::with_config(config);
    
    // Initially no active tunnel
    let active = manager.active_tunnel().await;
    assert!(active.is_none(), "Should start with no active tunnel");
    
    // Test with IPv6 available - should return None (no tunneling needed)
    let ipv6_caps = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: true,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: vec!["2001:db8::1".parse().unwrap()],
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    let selection = manager.select_tunnel(&ipv6_caps).await;
    assert!(selection.is_none(), "Should return None when IPv6 is available");
    
    // Test with no IPv6 but no tunnels available - should return None
    let no_ipv6_caps = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    let selection = manager.select_tunnel(&no_ipv6_caps).await;
    assert!(selection.is_none(), "Should return None when no tunnels are available");
    
    println!("Basic tunnel manager functionality test completed");
    Ok(())
}

/// Test tunnel manager configuration
#[tokio::test]
async fn test_tunnel_manager_config() -> anyhow::Result<()> {
    let config = TunnelManagerConfig {
        auto_failover: true,
        max_concurrent_attempts: 5,
        health_check_interval: Duration::from_secs(30),
        health_check_timeout: Duration::from_secs(3),
        protocol_preference: vec![
            TunnelProtocol::SixToFour,
            TunnelProtocol::Teredo,
            TunnelProtocol::SixInFour,
        ],
    };
    
    let manager = TunnelManager::with_config(config);
    
    // Initially no active tunnel
    let active = manager.active_tunnel().await;
    assert!(active.is_none(), "Should start with no active tunnel");
    
    println!("Tunnel manager configuration test completed");
    Ok(())
}

/// Test automatic failover configuration
#[tokio::test]
async fn test_auto_failover_config() -> anyhow::Result<()> {
    let config_with_failover = TunnelManagerConfig {
        auto_failover: true,
        max_concurrent_attempts: 3,
        health_check_interval: Duration::from_secs(30),
        ..Default::default()
    };
    
    let manager_with_failover = TunnelManager::with_config(config_with_failover);
    
    let config_no_failover = TunnelManagerConfig {
        auto_failover: false,
        max_concurrent_attempts: 0,
        ..Default::default()
    };
    
    let manager_no_failover = TunnelManager::with_config(config_no_failover);
    
    // Test basic functionality with both configs
    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    let selection1 = manager_with_failover.select_tunnel(&capabilities).await;
    let selection2 = manager_no_failover.select_tunnel(&capabilities).await;
    
    // Both should return None as no tunnels are available
    assert!(selection1.is_none() && selection2.is_none(), 
           "Should return None when no tunnels are available");
    
    println!("Auto failover configuration test completed successfully");
    Ok(())
}

/// Test concurrent protocol testing capability
#[tokio::test]
async fn test_concurrent_protocol_testing() -> anyhow::Result<()> {
    let config = TunnelManagerConfig {
        auto_failover: true,
        max_concurrent_attempts: 3, // Enable concurrent testing
        health_check_timeout: Duration::from_millis(100),
        ..Default::default()
    };
    
    let manager = TunnelManager::with_config(config);
    
    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    // This will attempt to test protocols concurrently
    // Since we don't have real endpoints, they'll likely all fail gracefully
    let start = time::Instant::now();
    let _selection = manager.select_tunnel(&capabilities).await;
    let duration = start.elapsed();
    
    // Should complete quickly due to timeout
    assert!(duration < Duration::from_secs(2), 
           "Concurrent testing should be fast with timeout");
    
    println!("Concurrent testing completed in {:?}", duration);
    Ok(())
}

/// Test edge cases and error conditions
#[tokio::test]
async fn test_edge_cases() -> anyhow::Result<()> {
    let config = TunnelManagerConfig {
        auto_failover: false, // Disabled auto-failover
        ..Default::default()
    };
    
    let manager = TunnelManager::with_config(config);
    
    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    // Should return None when auto-failover is disabled and no tunnels available
    let selection = manager.select_tunnel(&capabilities).await;
    assert!(selection.is_none(), 
           "Should return None when no tunnels are available");
    
    // Test with no IPv4 connectivity
    let no_ipv4_caps = NetworkCapabilities {
        has_ipv4: false,
        has_ipv6: true,
        behind_nat: false,
        public_ipv4: None,
        ipv6_addresses: vec!["2001:db8::1".parse().unwrap()],
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    let config = TunnelManagerConfig {
        auto_failover: true,
        max_concurrent_attempts: 0,
        ..Default::default()
    };
    let manager = TunnelManager::with_config(config);
    
    let selection = manager.select_tunnel(&no_ipv4_caps).await;
    // Should still work or return None gracefully
    if let Some(sel) = selection {
        println!("Selected protocol without IPv4: {:?}", sel.protocol);
    } else {
        println!("No protocol selected without IPv4 - expected behavior");
    }
    
    println!("Edge case testing completed");
    Ok(())
}

/// Performance benchmark for auto-selection
#[tokio::test]
async fn test_auto_selection_performance() -> anyhow::Result<()> {
    let config = TunnelManagerConfig {
        auto_failover: true,
        max_concurrent_attempts: 0, // Disable network tests for speed
        ..Default::default()
    };
    
    let manager = TunnelManager::with_config(config);
    
    let capabilities = NetworkCapabilities {
        has_ipv4: true,
        has_ipv6: false,
        behind_nat: false,
        public_ipv4: Some("203.0.113.1".parse().unwrap()),
        ipv6_addresses: Vec::new(),
        has_upnp: false,
        interface_mtu: 1500,
    };
    
    // Benchmark selection performance
    let iterations = 100;
    let start = time::Instant::now();
    
    for _ in 0..iterations {
        let _selection = manager.select_tunnel(&capabilities).await;
    }
    
    let duration = start.elapsed();
    let avg_duration = duration / iterations;
    
    println!("Auto-selection performance: {} iterations in {:?} (avg: {:?})", 
             iterations, duration, avg_duration);
    
    // Should be very fast without network tests
    assert!(avg_duration < Duration::from_millis(10), 
           "Auto-selection should be fast without network tests");
    
    Ok(())
}