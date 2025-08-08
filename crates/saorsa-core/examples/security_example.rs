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

//! Example of using the security module in the adaptive P2P network

use anyhow::Result;
use saorsa_core::adaptive::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a node identity
    let identity = NodeIdentity::generate()?;
    println!("Generated node identity: {:?}", identity.node_id());

    // Configure security settings
    let mut security_config = SecurityConfig::default();
    security_config.rate_limit.node_requests_per_window = 10;
    security_config.rate_limit.window_duration = Duration::from_secs(60);
    security_config.blacklist.max_size = 1000;
    security_config.eclipse_detection.min_diversity_score = 0.6;

    // Create security manager
    let security_manager = SecurityManager::new(security_config, identity);

    // Example: Validate a node join request
    let new_node = NodeDescriptor {
        id: NodeId { hash: [1u8; 32] },
        public_key: ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32]).unwrap(),
        addresses: vec!["192.168.1.10:8000".to_string()],
        hyperbolic: None,
        som_position: None,
        trust: 0.5,
        capabilities: NodeCapabilities {
            storage: 1000,
            compute: 100,
            bandwidth: 100,
        },
    };

    match security_manager.validate_node_join(&new_node).await {
        Ok(()) => println!("Node join validated successfully"),
        Err(e) => println!("Node join validation failed: {}", e),
    }

    // Example: Check rate limits
    let peer_id = NodeId { hash: [2u8; 32] };
    for i in 0..12 {
        if security_manager.check_rate_limit(&peer_id).await {
            println!("Request {} allowed", i + 1);
        } else {
            println!("Request {} rate limited!", i + 1);
        }
    }

    // Example: Verify data integrity
    let data = b"Important network data";
    let hash = security_manager.compute_hash(data).await;

    if security_manager.verify_integrity(data, &hash).await? {
        println!("Data integrity verified");
    } else {
        println!("Data integrity check failed!");
    }

    // Example: Check for eclipse attacks
    let routing_table = vec![
        NodeId { hash: [10u8; 32] },
        NodeId { hash: [20u8; 32] },
        NodeId { hash: [30u8; 32] },
        NodeId { hash: [40u8; 32] },
        NodeId { hash: [50u8; 32] },
    ];

    match security_manager.check_eclipse_attack(&routing_table).await {
        Ok(()) => println!("No eclipse attack detected"),
        Err(e) => println!("Eclipse attack warning: {}", e),
    }

    // Example: Get security metrics
    let metrics = security_manager.get_metrics().await;
    println!("\nSecurity Metrics:");
    println!("  Rate limit violations: {}", metrics.rate_limit_violations);
    println!("  Blacklisted nodes: {}", metrics.blacklisted_nodes);
    println!("  Verification failures: {}", metrics.verification_failures);
    println!("  Eclipse detections: {}", metrics.eclipse_detections);
    println!("  Audit entries: {}", metrics.audit_entries);

    // Example: Export audit report
    let report = security_manager.export_audit_report().await;
    println!("\nAudit Report:");
    println!("  Total entries: {}", report.total_entries);
    println!("  Critical events: {}", report.recent_critical_events.len());

    Ok(())
}
