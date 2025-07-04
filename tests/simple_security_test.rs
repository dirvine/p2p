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

//! Simple security test to verify basic functionality

use anyhow::Result;
use p2p_foundation::security::*;
use std::net::Ipv6Addr;
use std::str::FromStr;

#[tokio::test]
async fn test_ip_diversity_basic() -> Result<()> {
    let config = IPDiversityConfig::default();
    let enforcer = IPDiversityEnforcer::new(config);
    
    let addr = Ipv6Addr::from_str("2001:0db8:85a3:1234:5678:8a2e:0370:7334")?;
    let analysis = enforcer.analyze_ip(addr)?;
    
    // Should be able to accept first node
    assert!(enforcer.can_accept_node(&analysis));
    
    Ok(())
}

#[tokio::test]
async fn test_subnet_prefix_extraction() -> Result<()> {
    let addr = Ipv6Addr::from_str("2001:0db8:85a3:1234:5678:8a2e:0370:7334")?;
    
    let prefix_64 = IPDiversityEnforcer::extract_subnet_prefix(addr, 64);
    let expected_64 = Ipv6Addr::from_str("2001:0db8:85a3:1234::")?;
    assert_eq!(prefix_64, expected_64);
    
    let prefix_48 = IPDiversityEnforcer::extract_subnet_prefix(addr, 48);
    let expected_48 = Ipv6Addr::from_str("2001:0db8:85a3::")?;
    assert_eq!(prefix_48, expected_48);
    
    Ok(())
}

#[tokio::test]
async fn test_diversity_enforcement() -> Result<()> {
    let config = IPDiversityConfig {
        max_nodes_per_64: 1,
        max_nodes_per_48: 2,
        ..Default::default()
    };
    let mut enforcer = IPDiversityEnforcer::new(config);
    
    // Add first node
    let addr1 = Ipv6Addr::from_str("2001:0db8:85a3:1234:5678:8a2e:0370:7334")?;
    let analysis1 = enforcer.analyze_ip(addr1)?;
    assert!(enforcer.can_accept_node(&analysis1));
    enforcer.add_node(&analysis1)?;
    
    // Try to add second node in same /64 subnet - should be rejected
    let addr2 = Ipv6Addr::from_str("2001:0db8:85a3:1234:abcd:8a2e:0370:7334")?;
    let analysis2 = enforcer.analyze_ip(addr2)?;
    assert!(!enforcer.can_accept_node(&analysis2));
    
    // Add node in different /64 subnet - should be accepted
    let addr3 = Ipv6Addr::from_str("2001:0db8:85a3:5678:5678:8a2e:0370:7334")?;
    let analysis3 = enforcer.analyze_ip(addr3)?;
    assert!(enforcer.can_accept_node(&analysis3));
    enforcer.add_node(&analysis3)?;
    
    // Stats should show 2 different /64 subnets, 1 /48 subnet
    let stats = enforcer.get_diversity_stats();
    assert_eq!(stats.total_64_subnets, 2);
    assert_eq!(stats.total_48_subnets, 1);
    
    Ok(())
}