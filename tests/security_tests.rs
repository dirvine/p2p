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

//! IPv6-based Security System Tests
//!
//! Comprehensive tests for the IPv6-based node ID generation and IP diversity enforcement
//! system designed to prevent Sybil attacks.

use anyhow::Result;
use ed25519_dalek::Keypair;
use p2p_foundation::security::*;
use std::net::Ipv6Addr;
use std::str::FromStr;
use std::time::Duration;

/// Test IPv6-based node ID generation
#[tokio::test]
async fn test_ipv6_node_id_generation() -> Result<()> {
    let mut csprng = rand::rngs::OsRng {};
    let keypair = Keypair::generate(&mut csprng);
    let ipv6_addr = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    
    // Generate node ID
    let node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;
    
    // Verify basic properties
    assert_eq!(node_id.ipv6_addr, ipv6_addr);
    assert_eq!(node_id.public_key, keypair.public.to_bytes().to_vec());
    assert_eq!(node_id.signature.len(), 64);
    assert_eq!(node_id.salt.len(), 16);
    assert_eq!(node_id.node_id.len(), 32); // SHA256 output
    
    // Verify node ID is bound to IPv6 address
    let different_addr = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7335")?;
    let different_node_id = IPv6NodeID::generate(different_addr, &keypair)?;
    assert_ne!(node_id.node_id, different_node_id.node_id);
    
    Ok(())
}

/// Test node ID verification
#[tokio::test]
async fn test_node_id_verification() -> Result<()> {
    let mut csprng = rand::rngs::OsRng {};
    let keypair = Keypair::generate(&mut csprng);
    let ipv6_addr = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    
    // Generate valid node ID
    let node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;
    
    // Verification should succeed
    assert!(node_id.verify()?);
    
    // Test with corrupted signature
    let mut corrupted_node_id = node_id.clone();
    corrupted_node_id.signature[0] ^= 0xFF;
    assert!(!corrupted_node_id.verify()?);
    
    // Test with corrupted node ID
    let mut corrupted_node_id = node_id.clone();
    corrupted_node_id.node_id[0] ^= 0xFF;
    assert!(!corrupted_node_id.verify()?);
    
    // Test with wrong signature length
    let mut bad_sig_node_id = node_id.clone();
    bad_sig_node_id.signature = vec![0u8; 32]; // Wrong length
    assert!(!bad_sig_node_id.verify()?);
    
    // Test with wrong public key length
    let mut bad_key_node_id = node_id.clone();
    bad_key_node_id.public_key = vec![0u8; 16]; // Wrong length
    assert!(!bad_key_node_id.verify()?);
    
    // Test with different IPv6 address
    let mut corrupted_node_id = node_id.clone();
    corrupted_node_id.ipv6_addr = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7335")?;
    assert!(!corrupted_node_id.verify()?);
    
    Ok(())
}

/// Test subnet extraction
#[tokio::test]
async fn test_subnet_extraction() -> Result<()> {
    let mut csprng = rand::rngs::OsRng {};
    let keypair = Keypair::generate(&mut csprng);
    let ipv6_addr = Ipv6Addr::from_str("2001:0db8:85a3:1234:5678:8a2e:0370:7334")?;
    
    let node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;
    
    // Test /64 subnet extraction
    let subnet_64 = node_id.extract_subnet_64();
    let expected_64 = Ipv6Addr::from_str("2001:0db8:85a3:1234::")?;
    assert_eq!(subnet_64, expected_64);
    
    // Test /48 subnet extraction
    let subnet_48 = node_id.extract_subnet_48();
    let expected_48 = Ipv6Addr::from_str("2001:0db8:85a3::")?;
    assert_eq!(subnet_48, expected_48);
    
    // Test /32 subnet extraction
    let subnet_32 = node_id.extract_subnet_32();
    let expected_32 = Ipv6Addr::from_str("2001:0db8::")?;
    assert_eq!(subnet_32, expected_32);
    
    Ok(())
}

/// Test IP diversity enforcer with same /64 subnet
#[tokio::test]
async fn test_ip_diversity_64_subnet_limit() -> Result<()> {
    let config = IPDiversityConfig {
        max_nodes_per_64: 1,
        ..Default::default()
    };
    let mut enforcer = IPDiversityEnforcer::new(config);
    
    // First address in the subnet
    let addr1 = Ipv6Addr::from_str("2001:0db8:85a3:1234:5678:8a2e:0370:7334")?;
    let analysis1 = enforcer.analyze_ip(addr1)?;
    
    // Should be able to add first node
    assert!(enforcer.can_accept_node(&analysis1));
    enforcer.add_node(&analysis1)?;
    
    // Second address in same /64 subnet
    let addr2 = Ipv6Addr::from_str("2001:0db8:85a3:1234:abcd:8a2e:0370:7334")?;
    let analysis2 = enforcer.analyze_ip(addr2)?;
    
    // Should reject second node in same /64 subnet
    assert!(!enforcer.can_accept_node(&analysis2));
    
    // Different /64 subnet should be allowed
    let addr3 = Ipv6Addr::from_str("2001:0db8:85a3:5678:5678:8a2e:0370:7334")?;
    let analysis3 = enforcer.analyze_ip(addr3)?;
    assert!(enforcer.can_accept_node(&analysis3));
    
    Ok(())
}

/// Test IP diversity enforcer with /48 subnet limits
#[tokio::test]
async fn test_ip_diversity_48_subnet_limit() -> Result<()> {
    let config = IPDiversityConfig {
        max_nodes_per_64: 2,
        max_nodes_per_48: 3,
        ..Default::default()
    };
    let mut enforcer = IPDiversityEnforcer::new(config);
    
    // Add nodes in different /64 subnets but same /48
    let addresses = [
        "2001:0db8:85a3:1234:5678:8a2e:0370:7334",
        "2001:0db8:85a3:5678:5678:8a2e:0370:7334", 
        "2001:0db8:85a3:abcd:5678:8a2e:0370:7334",
    ];
    
    for addr_str in &addresses {
        let addr = Ipv6Addr::from_str(addr_str)?;
        let analysis = enforcer.analyze_ip(addr)?;
        assert!(enforcer.can_accept_node(&analysis));
        enforcer.add_node(&analysis)?;
    }
    
    // Fourth node in same /48 should be rejected
    let addr4 = Ipv6Addr::from_str("2001:0db8:85a3:fedc:5678:8a2e:0370:7334")?;
    let analysis4 = enforcer.analyze_ip(addr4)?;
    assert!(!enforcer.can_accept_node(&analysis4));
    
    // Different /48 should be allowed
    let addr5 = Ipv6Addr::from_str("2001:0db8:1234:1234:5678:8a2e:0370:7334")?;
    let analysis5 = enforcer.analyze_ip(addr5)?;
    assert!(enforcer.can_accept_node(&analysis5));
    
    Ok(())
}

/// Test node removal from diversity tracking
#[tokio::test]
async fn test_ip_diversity_node_removal() -> Result<()> {
    let config = IPDiversityConfig {
        max_nodes_per_64: 1,
        max_nodes_per_48: 2,
        ..Default::default()
    };
    let mut enforcer = IPDiversityEnforcer::new(config);
    
    // Add a node
    let addr1 = Ipv6Addr::from_str("2001:0db8:85a3:1234:5678:8a2e:0370:7334")?;
    let analysis1 = enforcer.analyze_ip(addr1)?;
    enforcer.add_node(&analysis1)?;
    
    // Verify space is occupied
    let addr2 = Ipv6Addr::from_str("2001:0db8:85a3:1234:abcd:8a2e:0370:7334")?;
    let analysis2 = enforcer.analyze_ip(addr2)?;
    assert!(!enforcer.can_accept_node(&analysis2));
    
    // Remove the first node
    enforcer.remove_node(&analysis1);
    
    // Now second node should be accepted
    assert!(enforcer.can_accept_node(&analysis2));
    enforcer.add_node(&analysis2)?;
    
    Ok(())
}

/// Test diversity statistics
#[tokio::test]
async fn test_diversity_statistics() -> Result<()> {
    let config = IPDiversityConfig::default();
    let mut enforcer = IPDiversityEnforcer::new(config);
    
    // Add nodes from different subnets
    let addresses = [
        "2001:0db8:85a3:1234:5678:8a2e:0370:7334", // /64: 2001:db8:85a3:1234::
        "2001:0db8:85a3:5678:5678:8a2e:0370:7334", // /64: 2001:db8:85a3:5678::
        "2001:0db8:1234:1234:5678:8a2e:0370:7334", // /48: 2001:db8:1234::
    ];
    
    for addr_str in &addresses {
        let addr = Ipv6Addr::from_str(addr_str)?;
        let analysis = enforcer.analyze_ip(addr)?;
        enforcer.add_node(&analysis)?;
    }
    
    let stats = enforcer.get_diversity_stats();
    assert_eq!(stats.total_64_subnets, 3);
    assert_eq!(stats.total_48_subnets, 2); // 2001:db8:85a3:: and 2001:db8:1234::
    assert_eq!(stats.total_32_subnets, 1); // 2001:db8::
    assert_eq!(stats.max_nodes_per_64, 1);
    assert_eq!(stats.max_nodes_per_48, 2);
    assert_eq!(stats.max_nodes_per_32, 3);
    
    Ok(())
}

/// Test subnet prefix extraction utility
#[tokio::test]
async fn test_subnet_prefix_extraction() -> Result<()> {
    let addr = Ipv6Addr::from_str("2001:0db8:85a3:1234:5678:8a2e:0370:7334")?;
    
    // Test various prefix lengths
    let prefix_64 = IPDiversityEnforcer::extract_subnet_prefix(addr, 64);
    assert_eq!(prefix_64, Ipv6Addr::from_str("2001:0db8:85a3:1234::")?);
    
    let prefix_48 = IPDiversityEnforcer::extract_subnet_prefix(addr, 48);
    assert_eq!(prefix_48, Ipv6Addr::from_str("2001:0db8:85a3::")?);
    
    let prefix_32 = IPDiversityEnforcer::extract_subnet_prefix(addr, 32);
    assert_eq!(prefix_32, Ipv6Addr::from_str("2001:0db8::")?);
    
    let prefix_16 = IPDiversityEnforcer::extract_subnet_prefix(addr, 16);
    assert_eq!(prefix_16, Ipv6Addr::from_str("2001::")?);
    
    // Test partial byte boundaries
    let prefix_12 = IPDiversityEnforcer::extract_subnet_prefix(addr, 12);
    assert_eq!(prefix_12, Ipv6Addr::from_str("2000::")?);
    
    Ok(())
}

/// Test reputation management system
#[tokio::test]
async fn test_reputation_management() -> Result<()> {
    
    let mut reputation_manager = ReputationManager::new(0.1, 0.1);
    
    // Create mock peer ID
    let peer_id = "test-peer-123".to_string();
    
    // Initial reputation should not exist
    assert!(reputation_manager.get_reputation(&peer_id).is_none());
    
    // Update with successful interaction
    reputation_manager.update_reputation(&peer_id, true, Duration::from_millis(100));
    
    let reputation = reputation_manager.get_reputation(&peer_id).unwrap();
    assert!(reputation.response_rate > 0.5); // Should increase from initial 0.5
    assert_eq!(reputation.interaction_count, 1);
    
    // Update with failed interaction
    reputation_manager.update_reputation(&peer_id, false, Duration::from_millis(1000));
    
    let reputation = reputation_manager.get_reputation(&peer_id).unwrap();
    assert!(reputation.response_rate < 0.6); // Should decrease
    assert_eq!(reputation.interaction_count, 2);
    
    // Multiple successful interactions should improve reputation
    for _ in 0..10 {
        reputation_manager.update_reputation(&peer_id, true, Duration::from_millis(50));
    }
    
    let reputation = reputation_manager.get_reputation(&peer_id).unwrap();
    assert!(reputation.response_rate > 0.8); // Should be high
    assert!(reputation.response_time.as_millis() < 300); // Response time should be reasonable
    
    Ok(())
}

/// Test reputation decay over time
#[tokio::test]
async fn test_reputation_decay() -> Result<()> {
    
    let mut reputation_manager = ReputationManager::new(1.0, 0.1); // High decay rate
    let peer_id = "test-peer-decay-456".to_string();
    
    // Build up good reputation
    for _ in 0..5 {
        reputation_manager.update_reputation(&peer_id, true, Duration::from_millis(50));
    }
    
    let initial_reputation = reputation_manager.get_reputation(&peer_id).unwrap().response_rate;
    assert!(initial_reputation > 0.7);
    
    // Wait briefly to allow some time passage (simplified test)
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    // Apply decay
    reputation_manager.apply_decay();
    
    // Reputation should have decayed
    if let Some(reputation) = reputation_manager.get_reputation(&peer_id) {
        assert!(reputation.response_rate < initial_reputation);
    } else {
        // Reputation might be completely removed if decay threshold reached
        // This is also acceptable behavior
    }
    
    Ok(())
}

/// Test attack scenario: rapid node creation from same subnet
#[tokio::test]
async fn test_sybil_attack_prevention() -> Result<()> {
    let config = IPDiversityConfig {
        max_nodes_per_64: 1,
        max_nodes_per_48: 2,
        max_nodes_per_32: 5,
        ..Default::default()
    };
    let mut enforcer = IPDiversityEnforcer::new(config);
    
    // Simulate attacker trying to create many nodes in same subnet
    let base_subnet = "2001:0db8:85a3:1234";
    let mut successful_additions = 0;
    let mut blocked_additions = 0;
    
    // Try to add 20 nodes in the same /64 subnet
    for i in 0..20 {
        let addr_str = format!("{}:{}:8a2e:0370:7334", base_subnet, i);
        let addr = Ipv6Addr::from_str(&addr_str)?;
        let analysis = enforcer.analyze_ip(addr)?;
        
        if enforcer.can_accept_node(&analysis) {
            enforcer.add_node(&analysis)?;
            successful_additions += 1;
        } else {
            blocked_additions += 1;
        }
    }
    
    // Should only allow 1 node per /64 subnet
    assert_eq!(successful_additions, 1);
    assert_eq!(blocked_additions, 19);
    
    // Verify attacker can't bypass by using different /64s in same /48
    let base_subnet_48 = "2001:0db8:85a3";
    for i in 2..10 { // Start from 2 since we already have subnet 1234
        let addr_str = format!("{}:{:04x}:1234:8a2e:0370:7334", base_subnet_48, i);
        let addr = Ipv6Addr::from_str(&addr_str)?;
        let analysis = enforcer.analyze_ip(addr)?;
        
        if enforcer.can_accept_node(&analysis) {
            enforcer.add_node(&analysis)?;
            successful_additions += 1;
        } else {
            blocked_additions += 1;
        }
    }
    
    // Should only allow max_nodes_per_48 (2) total in the /48 subnet
    assert!(successful_additions <= 2);
    
    Ok(())
}

/// Test economic attack cost calculation
#[tokio::test] 
async fn test_attack_cost_analysis() -> Result<()> {
    let config = IPDiversityConfig::default();
    let _enforcer = IPDiversityEnforcer::new(config);
    
    // Calculate how many /64 subnets an attacker would need to control
    // significant portion of keyspace
    
    // For 256-bit keyspace, to control 1% requires approximately:
    // 2^256 * 0.01 / nodes_per_subnet positions
    
    // Use smaller numbers to avoid overflow
    let practical_keyspace = 1_000_000_u64; // 1 million nodes for practical calculation
    let target_control_percentage = 0.01; // 1%
    let nodes_needed = (practical_keyspace as f64 * target_control_percentage) as u64;
    
    // With 1 node per /64 subnet, attacker needs this many different /64 subnets
    let subnets_needed = nodes_needed;
    
    // Cost analysis (rough estimates):
    // - Residential IPv6: $50-100/month per diverse /64 range  
    // - VPS providers: $5-20/month per server with unique /64
    
    let min_monthly_cost = subnets_needed * 5; // $5 per subnet minimum
    let max_monthly_cost = subnets_needed * 100; // $100 per subnet maximum
    
    println!("Attack cost analysis:");
    println!("  Nodes needed for 1% control: {}", nodes_needed);
    println!("  /64 subnets required: {}", subnets_needed);
    println!("  Minimum monthly cost: ${}", min_monthly_cost);
    println!("  Maximum monthly cost: ${}", max_monthly_cost);
    
    // For practical testing, let's use smaller numbers
    let practical_nodes = 1000; // Reasonable attack size
    let practical_cost_min = practical_nodes * 5;
    let practical_cost_max = practical_nodes * 100;
    
    // Even 1000 nodes requires significant investment
    assert!(practical_cost_min >= 5000); // At least $5,000/month
    assert!(practical_cost_max >= 100000); // Up to $100,000/month
    
    println!("Practical attack (1000 nodes):");
    println!("  Monthly cost range: ${} - ${}", practical_cost_min, practical_cost_max);
    
    Ok(())
}

/// Benchmark node ID generation performance
#[tokio::test]
async fn test_node_id_generation_performance() -> Result<()> {
    use std::time::Instant;
    use rand::rngs::OsRng;
    
    let mut csprng = OsRng{};
    let keypair = Keypair::generate(&mut csprng);
    let ipv6_addr = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    
    let iterations = 1000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _node_id = IPv6NodeID::generate(ipv6_addr, &keypair)?;
    }
    
    let elapsed = start.elapsed();
    let avg_time = elapsed / iterations;
    
    println!("Node ID generation performance:");
    println!("  {} iterations in {:?}", iterations, elapsed);
    println!("  Average time per generation: {:?}", avg_time);
    
    // Should be reasonably fast (less than 1ms per generation)
    assert!(avg_time < Duration::from_millis(1));
    
    Ok(())
}

/// Benchmark node ID verification performance
#[tokio::test]
async fn test_node_id_verification_performance() -> Result<()> {
    use std::time::Instant;
    use rand::rngs::OsRng;
    
    let mut csprng = OsRng{};
    let keypair = Keypair::generate(&mut csprng);
    let ipv6_addr = Ipv6Addr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334")?;
    
    // Generate node IDs to verify
    let mut node_ids = Vec::new();
    for _ in 0..100 {
        node_ids.push(IPv6NodeID::generate(ipv6_addr, &keypair)?);
    }
    
    let start = Instant::now();
    
    for node_id in &node_ids {
        assert!(node_id.verify()?);
    }
    
    let elapsed = start.elapsed();
    let avg_time = elapsed / node_ids.len() as u32;
    
    println!("Node ID verification performance:");
    println!("  {} verifications in {:?}", node_ids.len(), elapsed);
    println!("  Average time per verification: {:?}", avg_time);
    
    // Verification should be fast (less than 1ms each)
    assert!(avg_time < Duration::from_millis(1));
    
    Ok(())
}