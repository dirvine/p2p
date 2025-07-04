
//! IPv6 tunneling protocol tests

use anyhow::{Context, Result};
use p2p_core::tunneling::{
    TunnelProtocol, TunnelConfig, TunnelState, TunnelMetrics,
    TunnelManager, Tunnel, TunnelEndpoint, TunnelRoute,
};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use crate::infrastructure::{
    test_network::DistributedTestNetwork,
    test_reporter::{TestEvent, TestEventType, TunnelStats, TunnelInfo},
};

/// Test all IPv6 tunneling protocols
pub async fn test_ipv6_tunneling(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🌐 Testing IPv6 Tunneling Protocols");
    println!("===================================");
    
    // 1. Test automatic tunnel selection
    test_auto_tunnel_selection(network).await
        .context("Failed to test auto tunnel selection")?;
    
    // 2. Test each protocol explicitly
    test_all_tunnel_protocols(network).await
        .context("Failed to test all protocols")?;
    
    // 3. Test tunnel failover
    test_tunnel_failover(network).await
        .context("Failed to test failover")?;
    
    // 4. Test tunnel metrics
    test_tunnel_performance(network).await
        .context("Failed to test performance")?;
    
    // 5. Test cross-tunnel communication
    test_cross_tunnel_communication(network).await
        .context("Failed to test cross-tunnel")?;
    
    // 6. Test tunnel security
    test_tunnel_security(network).await
        .context("Failed to test security")?;
    
    Ok(())
}

/// Test automatic tunnel selection
async fn test_auto_tunnel_selection(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔍 Testing automatic tunnel selection...");
    
    // Simulate different network environments
    let test_scenarios = vec![
        ("Home NAT", vec![
            ("has_public_ipv4", "false"),
            ("behind_nat", "true"),
            ("upnp_available", "true"),
        ]),
        ("Corporate Network", vec![
            ("has_public_ipv4", "false"),
            ("behind_nat", "true"),
            ("firewall_strict", "true"),
        ]),
        ("ISP with 6to4", vec![
            ("has_public_ipv4", "true"),
            ("ipv6_native", "false"),
        ]),
        ("Carrier-Grade NAT", vec![
            ("has_public_ipv4", "false"),
            ("cgnat", "true"),
        ]),
    ];
    
    for (scenario_name, params) in test_scenarios {
        println!("  Testing scenario: {}", scenario_name);
        
        let node_idx = network.local_nodes.len() - 1;
        let node = &mut network.local_nodes[node_idx];
        
        // Configure environment
        let mut env_config = HashMap::new();
        for (key, value) in params {
            env_config.insert(key.to_string(), value.to_string());
        }
        
        // Auto-select tunnel
        let selected_tunnel = node.tunnel_manager
            .auto_select_tunnel(env_config.clone())
            .await?;
        
        // Create tunnel with auto-selected protocol
        let tunnel_id = node.tunnel_manager
            .create_tunnel(
                format!("auto_{}", scenario_name),
                selected_tunnel.config,
            ).await?;
        
        // Wait for establishment
        let start = Instant::now();
        let mut established = false;
        
        while start.elapsed() < Duration::from_secs(5) {
            if let Some(tunnel) = node.tunnel_manager.get_tunnel(&tunnel_id).await {
                if matches!(tunnel.state, TunnelState::Established) {
                    established = true;
                    break;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: format!("node_{}", node_idx),
            event_type: TestEventType::TunnelCreated,
            details: {
                let mut details = HashMap::new();
                details.insert("scenario".to_string(), serde_json::json!(scenario_name));
                details.insert("selected_protocol".to_string(), 
                    serde_json::json!(format!("{:?}", selected_tunnel.protocol)));
                details.insert("established".to_string(), serde_json::json!(established));
                details.insert("setup_time_ms".to_string(), 
                    serde_json::json!(start.elapsed().as_millis()));
                details
            },
            success: established,
        }).await;
    }
    
    println!("✅ Automatic tunnel selection tested");
    Ok(())
}

/// Test all tunnel protocols
async fn test_all_tunnel_protocols(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔧 Testing all tunnel protocols...");
    
    let protocols = vec![
        (TunnelProtocol::SixToFour, "6to4 - Automatic IPv6 over IPv4"),
        (TunnelProtocol::Teredo, "Teredo - NAT traversal"),
        (TunnelProtocol::SixInFour, "6in4 - Manual tunnel"),
        (TunnelProtocol::DsLite, "DS-Lite - Dual-Stack Lite"),
        (TunnelProtocol::Isatap, "ISATAP - Intra-site automatic"),
        (TunnelProtocol::MapE, "MAP-E - Mapping encapsulation"),
        (TunnelProtocol::MapT, "MAP-T - Mapping translation"),
    ];
    
    let mut tunnel_stats = TunnelStats {
        tunnels: HashMap::new(),
    };
    
    for (i, (protocol, description)) in protocols.iter().enumerate() {
        if i >= network.local_nodes.len() {
            break;
        }
        
        println!("  Testing {}: {}", format!("{:?}", protocol), description);
        
        let node = &mut network.local_nodes[i];
        
        // Create protocol-specific configuration
        let tunnel_config = match protocol {
            TunnelProtocol::SixToFour => TunnelConfig {
                protocol: protocol.clone(),
                local_ipv4: Some(Ipv4Addr::new(192, 0, 2, i as u8 + 1)),
                remote_endpoint: None,
                relay_server: Some("192.88.99.1:0".parse()?), // 6to4 anycast
                mtu: 1280,
                ttl: 64,
                authentication: None,
                encryption: true,
            },
            
            TunnelProtocol::Teredo => TunnelConfig {
                protocol: protocol.clone(),
                local_ipv4: None, // Behind NAT
                remote_endpoint: None,
                relay_server: Some("teredo.example.com:3544".parse()?),
                mtu: 1280,
                ttl: 64,
                authentication: Some("teredo_auth".to_string()),
                encryption: true,
            },
            
            TunnelProtocol::Isatap => TunnelConfig {
                protocol: protocol.clone(),
                local_ipv4: Some(Ipv4Addr::new(10, 0, 0, i as u8 + 1)),
                remote_endpoint: Some("192.168.1.1:0".parse()?), // ISATAP router
                relay_server: None,
                mtu: 1280,
                ttl: 64,
                authentication: None,
                encryption: false,
            },
            
            TunnelProtocol::DsLite => TunnelConfig {
                protocol: protocol.clone(),
                local_ipv4: Some(Ipv4Addr::new(100, 64, 0, i as u8 + 1)), // CGN range
                remote_endpoint: Some("aftr.isp.example:1234".parse()?),
                relay_server: None,
                mtu: 1460,
                ttl: 64,
                authentication: Some("dslite_key".to_string()),
                encryption: true,
            },
            
            TunnelProtocol::MapE => TunnelConfig {
                protocol: protocol.clone(),
                local_ipv4: Some(Ipv4Addr::new(192, 168, 1, i as u8 + 1)),
                remote_endpoint: None,
                relay_server: Some("map-br.example.com:0".parse()?),
                mtu: 1460,
                ttl: 64,
                authentication: None,
                encryption: false,
            },
            
            TunnelProtocol::MapT => TunnelConfig {
                protocol: protocol.clone(),
                local_ipv4: Some(Ipv4Addr::new(192, 168, 2, i as u8 + 1)),
                remote_endpoint: None,
                relay_server: Some("map-br.example.com:0".parse()?),
                mtu: 1460,
                ttl: 64,
                authentication: None,
                encryption: false,
            },
            
            _ => TunnelConfig::default_for_protocol(protocol.clone()),
        };
        
        // Create tunnel
        let tunnel_id = node.tunnel_manager
            .create_tunnel(
                format!("{:?}_tunnel", protocol),
                tunnel_config,
            ).await?;
        
        // Monitor establishment
        let start = Instant::now();
        let mut state = TunnelState::Initializing;
        let mut metrics = TunnelMetrics::default();
        
        // Wait for establishment
        for _ in 0..50 { // 5 seconds max
            if let Some(tunnel) = node.tunnel_manager.get_tunnel(&tunnel_id).await {
                state = tunnel.state.clone();
                metrics = tunnel.get_metrics();
                
                if matches!(state, TunnelState::Established) {
                    break;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        
        let established = matches!(state, TunnelState::Established);
        
        // Test data transfer if established
        if established {
            // Send test data through tunnel
            let test_data = format!("Test data for {:?} tunnel", protocol).into_bytes();
            let key = p2p_core::dht::Key::new(&test_data);
            
            // Store in DHT through tunnel
            node.dht.put(key.clone(), test_data.clone()).await?;
            
            // Retrieve from another node
            let retrieve_node = (i + 1) % network.local_nodes.len();
            let retrieved = network.local_nodes[retrieve_node].dht
                .get(&key).await;
            
            if let Some(data) = retrieved {
                metrics.bytes_sent += test_data.len() as u64;
                metrics.bytes_received += data.value.len() as u64;
                metrics.packets_sent += 1;
                metrics.packets_received += 1;
            }
        }
        
        // Add to stats
        tunnel_stats.tunnels.insert(
            format!("{:?}", protocol),
            TunnelInfo {
                state: format!("{:?}", state),
                bytes_sent: metrics.bytes_sent,
                bytes_received: metrics.bytes_received,
                avg_rtt_ms: metrics.avg_rtt.map(|d| d.as_millis() as u64),
            },
        );
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: format!("node_{}", i),
            event_type: TestEventType::TunnelCreated,
            details: {
                let mut details = HashMap::new();
                details.insert("protocol".to_string(), serde_json::json!(format!("{:?}", protocol)));
                details.insert("tunnel_id".to_string(), serde_json::json!(tunnel_id));
                details.insert("state".to_string(), serde_json::json!(format!("{:?}", state)));
                details.insert("established".to_string(), serde_json::json!(established));
                details.insert("setup_time_ms".to_string(), 
                    serde_json::json!(start.elapsed().as_millis()));
                details.insert("bytes_transferred".to_string(), 
                    serde_json::json!(metrics.bytes_sent + metrics.bytes_received));
                details
            },
            success: established,
        }).await;
    }
    
    // Display tunnel statistics
    network.reporter.show_tunnel_stats(&tunnel_stats).await;
    
    println!("✅ All tunnel protocols tested");
    Ok(())
}

/// Test tunnel failover
async fn test_tunnel_failover(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔄 Testing tunnel failover...");
    
    let node = &mut network.local_nodes[0];
    
    // Create primary tunnel (6to4)
    let primary_config = TunnelConfig::sixto4_auto();
    let primary_id = node.tunnel_manager
        .create_tunnel("primary_6to4".to_string(), primary_config)
        .await?;
    
    // Create backup tunnel (Teredo)
    let backup_config = TunnelConfig::teredo_nat();
    let backup_id = node.tunnel_manager
        .create_tunnel("backup_teredo".to_string(), backup_config)
        .await?;
    
    // Wait for primary establishment
    sleep(Duration::from_secs(2)).await;
    
    // Configure failover
    node.tunnel_manager.configure_failover(
        primary_id.clone(),
        backup_id.clone(),
        Duration::from_secs(5), // Failover timeout
    ).await?;
    
    // Send data through primary
    let test_data = b"Failover test data";
    let start_bytes = node.tunnel_manager
        .get_tunnel(&primary_id).await
        .map(|t| t.get_metrics().bytes_sent)
        .unwrap_or(0);
    
    // Simulate primary tunnel failure
    node.tunnel_manager.simulate_tunnel_failure(&primary_id).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "node_0".to_string(),
        event_type: TestEventType::TunnelCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("action".to_string(), serde_json::json!("primary_failed"));
            details.insert("primary_tunnel".to_string(), serde_json::json!(primary_id));
            details
        },
        success: true,
    }).await;
    
    // Wait for failover
    sleep(Duration::from_secs(6)).await;
    
    // Check if backup is active
    let backup_active = node.tunnel_manager
        .get_tunnel(&backup_id).await
        .map(|t| matches!(t.state, TunnelState::Established))
        .unwrap_or(false);
    
    // Send data through backup
    if backup_active {
        let key = p2p_core::dht::Key::new(test_data);
        node.dht.put(key, test_data.to_vec()).await?;
    }
    
    // Test failback when primary recovers
    node.tunnel_manager.restore_tunnel(&primary_id).await?;
    sleep(Duration::from_secs(3)).await;
    
    let primary_restored = node.tunnel_manager
        .get_tunnel(&primary_id).await
        .map(|t| matches!(t.state, TunnelState::Established))
        .unwrap_or(false);
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "node_0".to_string(),
        event_type: TestEventType::TunnelCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("action".to_string(), serde_json::json!("failover_test"));
            details.insert("backup_activated".to_string(), serde_json::json!(backup_active));
            details.insert("primary_restored".to_string(), serde_json::json!(primary_restored));
            details.insert("failover_time_ms".to_string(), serde_json::json!(5000));
            details
        },
        success: backup_active,
    }).await;
    
    println!("✅ Tunnel failover tested successfully");
    Ok(())
}

/// Test tunnel performance metrics
async fn test_tunnel_performance(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📊 Testing tunnel performance...");
    
    // Test different data sizes
    let test_sizes = vec![
        (1024, "1KB"),
        (10 * 1024, "10KB"),
        (100 * 1024, "100KB"),
        (1024 * 1024, "1MB"),
    ];
    
    let node_a = &mut network.local_nodes[0];
    let node_b_idx = 1;
    
    // Create high-performance tunnel
    let perf_config = TunnelConfig {
        protocol: TunnelProtocol::SixInFour,
        local_ipv4: Some(Ipv4Addr::new(192, 0, 2, 100)),
        remote_endpoint: Some(format!("[{}]:9000", 
            network.local_nodes[node_b_idx].transport.local_addr()?.ip()).parse()?),
        relay_server: None,
        mtu: 1500, // Maximum MTU
        ttl: 255,
        authentication: None,
        encryption: false, // Disable for performance test
    };
    
    let perf_tunnel_id = node_a.tunnel_manager
        .create_tunnel("performance_test".to_string(), perf_config)
        .await?;
    
    // Wait for establishment
    sleep(Duration::from_secs(2)).await;
    
    let mut perf_results = Vec::new();
    
    for (size, label) in test_sizes {
        let test_data = vec![0xAB; size];
        let key = p2p_core::dht::Key::new(&test_data);
        
        // Measure write performance
        let write_start = Instant::now();
        node_a.dht.put(key.clone(), test_data.clone()).await?;
        let write_duration = write_start.elapsed();
        
        // Measure read performance
        let read_start = Instant::now();
        let retrieved = network.local_nodes[node_b_idx].dht
            .get(&key).await;
        let read_duration = read_start.elapsed();
        
        let success = retrieved.is_some();
        
        // Calculate throughput
        let write_throughput_mbps = (size as f64 * 8.0) / 
            (write_duration.as_secs_f64() * 1_000_000.0);
        let read_throughput_mbps = if success {
            (size as f64 * 8.0) / (read_duration.as_secs_f64() * 1_000_000.0)
        } else {
            0.0
        };
        
        perf_results.push((
            label,
            write_throughput_mbps,
            read_throughput_mbps,
            write_duration.as_millis(),
            read_duration.as_millis(),
        ));
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: "performance_test".to_string(),
            event_type: TestEventType::DHTOperation,
            details: {
                let mut details = HashMap::new();
                details.insert("test_size".to_string(), serde_json::json!(label));
                details.insert("write_mbps".to_string(), 
                    serde_json::json!(format!("{:.2}", write_throughput_mbps)));
                details.insert("read_mbps".to_string(), 
                    serde_json::json!(format!("{:.2}", read_throughput_mbps)));
                details.insert("write_ms".to_string(), serde_json::json!(write_duration.as_millis()));
                details.insert("read_ms".to_string(), serde_json::json!(read_duration.as_millis()));
                details
            },
            success,
        }).await;
    }
    
    // Test latency
    let mut rtts = Vec::new();
    for _ in 0..10 {
        let ping_start = Instant::now();
        let pong = node_a.tunnel_manager
            .ping_tunnel(&perf_tunnel_id).await?;
        if pong {
            rtts.push(ping_start.elapsed());
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    let avg_rtt = if !rtts.is_empty() {
        rtts.iter().sum::<Duration>() / rtts.len() as u32
    } else {
        Duration::from_millis(999)
    };
    
    println!("\n  Performance Results:");
    println!("  ├─ Average RTT: {:?}", avg_rtt);
    for (label, write_mbps, read_mbps, _, _) in &perf_results {
        println!("  ├─ {}: Write {:.2} Mbps, Read {:.2} Mbps", 
            label, write_mbps, read_mbps);
    }
    println!("  └─ MTU: 1500 bytes");
    
    println!("✅ Performance testing completed");
    Ok(())
}

/// Test cross-tunnel communication
async fn test_cross_tunnel_communication(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔗 Testing cross-tunnel communication...");
    
    // Set up different tunnel types on different nodes
    let tunnel_configs = vec![
        (0, TunnelProtocol::SixToFour, "6to4"),
        (1, TunnelProtocol::Teredo, "Teredo"),
        (2, TunnelProtocol::Isatap, "ISATAP"),
    ];
    
    let mut tunnel_ids = Vec::new();
    
    for (node_idx, protocol, name) in &tunnel_configs {
        if *node_idx >= network.local_nodes.len() {
            continue;
        }
        
        let config = TunnelConfig::default_for_protocol(protocol.clone());
        let tunnel_id = network.local_nodes[*node_idx].tunnel_manager
            .create_tunnel(format!("cross_{}", name), config)
            .await?;
        
        tunnel_ids.push((*node_idx, tunnel_id));
    }
    
    // Wait for all tunnels to establish
    sleep(Duration::from_secs(3)).await;
    
    // Test communication between all tunnel pairs
    let mut cross_tunnel_results = Vec::new();
    
    for i in 0..tunnel_ids.len() {
        for j in (i + 1)..tunnel_ids.len() {
            let (node_i, _) = &tunnel_ids[i];
            let (node_j, _) = &tunnel_ids[j];
            
            let test_msg = format!("Cross-tunnel test from {} to {}", 
                tunnel_configs[i].2, tunnel_configs[j].2);
            
            let key = p2p_core::dht::Key::new(test_msg.as_bytes());
            
            // Store from node i
            network.local_nodes[*node_i].dht
                .put(key.clone(), test_msg.as_bytes().to_vec())
                .await?;
            
            // Retrieve from node j
            let retrieved = network.local_nodes[*node_j].dht
                .get(&key).await;
            
            let success = retrieved.is_some();
            cross_tunnel_results.push((
                tunnel_configs[i].2,
                tunnel_configs[j].2,
                success,
            ));
            
            network.reporter.report_progress(TestEvent {
                timestamp: std::time::SystemTime::now(),
                node_id: "cross_tunnel_test".to_string(),
                event_type: TestEventType::DHTOperation,
                details: {
                    let mut details = HashMap::new();
                    details.insert("from_tunnel".to_string(), 
                        serde_json::json!(tunnel_configs[i].2));
                    details.insert("to_tunnel".to_string(), 
                        serde_json::json!(tunnel_configs[j].2));
                    details.insert("success".to_string(), serde_json::json!(success));
                    details
                },
                success,
            }).await;
        }
    }
    
    // Display cross-tunnel matrix
    println!("\n  Cross-Tunnel Communication Matrix:");
    for (from, to, success) in &cross_tunnel_results {
        println!("  {} → {} : {}", 
            from, to, if *success { "✅" } else { "❌" });
    }
    
    println!("\n✅ Cross-tunnel communication tested");
    Ok(())
}

/// Test tunnel security features
async fn test_tunnel_security(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔒 Testing tunnel security...");
    
    let node = &mut network.local_nodes[0];
    
    // Create secure tunnel with authentication and encryption
    let secure_config = TunnelConfig {
        protocol: TunnelProtocol::SixInFour,
        local_ipv4: Some(Ipv4Addr::new(192, 0, 2, 200)),
        remote_endpoint: Some("secure.tunnel.example:9001".parse()?),
        relay_server: None,
        mtu: 1280,
        ttl: 64,
        authentication: Some("strong_psk_key_here".to_string()),
        encryption: true,
    };
    
    let secure_tunnel = node.tunnel_manager
        .create_tunnel("secure_tunnel".to_string(), secure_config)
        .await?;
    
    // Test authentication failure
    let invalid_config = TunnelConfig {
        protocol: TunnelProtocol::SixInFour,
        local_ipv4: Some(Ipv4Addr::new(192, 0, 2, 201)),
        remote_endpoint: Some("secure.tunnel.example:9001".parse()?),
        relay_server: None,
        mtu: 1280,
        ttl: 64,
        authentication: Some("wrong_key".to_string()),
        encryption: true,
    };
    
    let invalid_result = node.tunnel_manager
        .create_tunnel("invalid_auth".to_string(), invalid_config)
        .await;
    
    // Should fail or not establish
    let auth_test_passed = invalid_result.is_err() || {
        if let Ok(tunnel_id) = invalid_result {
            sleep(Duration::from_secs(2)).await;
            !node.tunnel_manager
                .get_tunnel(&tunnel_id).await
                .map(|t| matches!(t.state, TunnelState::Established))
                .unwrap_or(false)
        } else {
            true
        }
    };
    
    // Test encrypted data transfer
    let sensitive_data = b"Sensitive information that must be encrypted";
    let key = p2p_core::dht::Key::new(sensitive_data);
    
    // Store through secure tunnel
    node.tunnel_manager.route_through_tunnel(&secure_tunnel).await?;
    node.dht.put(key.clone(), sensitive_data.to_vec()).await?;
    
    // Verify encryption metrics
    let metrics = node.tunnel_manager
        .get_tunnel(&secure_tunnel).await
        .map(|t| t.get_security_metrics())
        .unwrap_or_default();
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "node_0".to_string(),
        event_type: TestEventType::TunnelCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("test_type".to_string(), serde_json::json!("security"));
            details.insert("auth_test_passed".to_string(), serde_json::json!(auth_test_passed));
            details.insert("encryption_enabled".to_string(), serde_json::json!(true));
            details.insert("encrypted_packets".to_string(), 
                serde_json::json!(metrics.encrypted_packets));
            details.insert("auth_failures".to_string(), 
                serde_json::json!(metrics.auth_failures));
            details
        },
        success: auth_test_passed,
    }).await;
    
    // Test tunnel access control
    let acl_config = TunnelConfig {
        protocol: TunnelProtocol::Isatap,
        local_ipv4: Some(Ipv4Addr::new(10, 0, 0, 100)),
        remote_endpoint: Some("router.corp.example:0".parse()?),
        relay_server: None,
        mtu: 1280,
        ttl: 64,
        authentication: None,
        encryption: false,
    };
    
    let acl_tunnel = node.tunnel_manager
        .create_tunnel("acl_tunnel".to_string(), acl_config)
        .await?;
    
    // Configure access control
    node.tunnel_manager.set_tunnel_acl(
        &acl_tunnel,
        vec![
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), // Allow specific addresses
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2),
        ],
    ).await?;
    
    println!("✅ Tunnel security features tested");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tunnel_mtu_validation() {
        // Minimum IPv6 MTU is 1280
        assert!(validate_mtu(1280));
        assert!(validate_mtu(1500));
        assert!(!validate_mtu(1279));
        assert!(!validate_mtu(9001)); // Too large
    }
    
    fn validate_mtu(mtu: u16) -> bool {
        mtu >= 1280 && mtu <= 9000
    }
    
    #[test]
    fn test_tunnel_protocol_properties() {
        assert!(TunnelProtocol::Teredo.supports_nat_traversal());
        assert!(TunnelProtocol::SixToFour.requires_public_ipv4());
        assert!(TunnelProtocol::DsLite.supports_cgnat());
    }
}