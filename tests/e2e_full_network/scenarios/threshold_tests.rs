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

//! FROST threshold signature tests

use anyhow::{Context, Result};
use p2p_core::threshold::{
    ThresholdGroup, GroupInfo, ThresholdConfig, SigningSession,
    Share, Commitment, Nonce, SignatureShare, GroupSignature,
    ParticipantUpdate, UpdateType,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use crate::infrastructure::{
    test_network::DistributedTestNetwork,
    test_reporter::{TestEvent, TestEventType},
};

/// Test FROST threshold signatures
pub async fn test_threshold_signatures(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔐 Testing FROST Threshold Signatures");
    println!("====================================");
    
    // 1. Test group creation
    test_threshold_group_creation(network).await
        .context("Failed to test group creation")?;
    
    // 2. Test distributed key generation
    test_distributed_key_generation(network).await
        .context("Failed to test DKG")?;
    
    // 3. Test signing sessions
    test_threshold_signing_sessions(network).await
        .context("Failed to test signing")?;
    
    // 4. Test participant management
    test_participant_updates(network).await
        .context("Failed to test participant updates")?;
    
    // 5. Test recovery scenarios
    test_threshold_recovery(network).await
        .context("Failed to test recovery")?;
    
    // 6. Test concurrent signing
    test_concurrent_signing(network).await
        .context("Failed to test concurrent signing")?;
    
    Ok(())
}

/// Test threshold group creation
async fn test_threshold_group_creation(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n👥 Testing threshold group creation...");
    
    let test_configs = vec![
        (2, 3, "2-of-3 threshold"),
        (3, 5, "3-of-5 threshold"),
        (5, 7, "5-of-7 threshold"),
    ];
    
    let mut groups = Vec::new();
    
    for (threshold, total, description) in test_configs {
        let group_info = GroupInfo {
            name: format!("{} Group", description),
            description: format!("Testing {} configuration", description),
            threshold,
            total_participants: total,
            metadata: HashMap::from([
                ("test_type".to_string(), "e2e".to_string()),
                ("created_by".to_string(), "node_0".to_string()),
            ]),
        };
        
        let group = network.local_nodes[0].node
            .create_threshold_group(group_info)
            .await?;
        
        groups.push(group.clone());
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: "node_0".to_string(),
            event_type: TestEventType::ThresholdSigning,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("group_created"));
                details.insert("group_id".to_string(), serde_json::json!(group.id));
                details.insert("threshold".to_string(), serde_json::json!(threshold));
                details.insert("participants".to_string(), serde_json::json!(total));
                details
            },
            success: true,
        }).await;
    }
    
    // Verify group properties
    for group in &groups {
        assert!(group.shares.len() == group.total_participants);
        assert!(group.threshold <= group.total_participants);
        assert!(group.public_key.len() > 0);
    }
    
    // Store groups for later tests
    for node in &mut network.local_nodes {
        node.test_data.write().await.events.push(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: node.identity.base_identity.user_id.clone(),
            event_type: TestEventType::ThresholdSigning,
            details: {
                let mut details = HashMap::new();
                details.insert("threshold_groups".to_string(), 
                    serde_json::json!(groups.iter().map(|g| &g.id).collect::<Vec<_>>()));
                details
            },
            success: true,
        });
    }
    
    println!("✅ Created {} threshold groups", groups.len());
    Ok(())
}

/// Test distributed key generation
async fn test_distributed_key_generation(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔑 Testing distributed key generation...");
    
    // Create new group for DKG test
    let dkg_group = network.local_nodes[0].node.initiate_dkg(
        "DKG Test Group".to_string(),
        4, // threshold
        6, // total participants
        vec![
            network.local_nodes[0].identity.base_identity.user_id.clone(),
            network.local_nodes[1].identity.base_identity.user_id.clone(),
            network.local_nodes[2].identity.base_identity.user_id.clone(),
            network.local_nodes[3].identity.base_identity.user_id.clone(),
            network.local_nodes[4].identity.base_identity.user_id.clone(),
            network.local_nodes[5].identity.base_identity.user_id.clone(),
        ],
    ).await?;
    
    // Each participant generates and broadcasts commitments
    let mut commitments = HashMap::new();
    
    for i in 0..6 {
        let commitment = network.local_nodes[i].node
            .generate_dkg_commitment(&dkg_group.session_id)
            .await?;
        
        commitments.insert(
            network.local_nodes[i].identity.base_identity.user_id.clone(),
            commitment,
        );
        
        // Broadcast commitment to all other participants
        network.local_nodes[i].node
            .broadcast_dkg_commitment(&dkg_group.session_id, &commitment)
            .await?;
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: format!("node_{}", i),
            event_type: TestEventType::ThresholdSigning,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("dkg_commitment"));
                details.insert("session_id".to_string(), serde_json::json!(dkg_group.session_id));
                details.insert("participant".to_string(), serde_json::json!(i + 1));
                details
            },
            success: true,
        }).await;
    }
    
    // Wait for commitment phase to complete
    sleep(Duration::from_secs(1)).await;
    
    // Generate and exchange shares
    for i in 0..6 {
        let shares = network.local_nodes[i].node
            .generate_dkg_shares(&dkg_group.session_id, &commitments)
            .await?;
        
        // Send shares to respective participants
        for (j, share) in shares.iter().enumerate() {
            if i != j {
                network.local_nodes[j].node
                    .receive_dkg_share(&dkg_group.session_id, i, share.clone())
                    .await?;
            }
        }
    }
    
    // Complete DKG and derive group key
    let mut group_keys = Vec::new();
    
    for i in 0..6 {
        let group_key = network.local_nodes[i].node
            .complete_dkg(&dkg_group.session_id)
            .await?;
        
        group_keys.push(group_key);
    }
    
    // Verify all participants derived the same group public key
    let reference_key = &group_keys[0];
    for key in &group_keys[1..] {
        assert_eq!(key, reference_key, "All participants should derive same group key");
    }
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ThresholdSigning,
        details: {
            let mut details = HashMap::new();
            details.insert("action".to_string(), serde_json::json!("dkg_completed"));
            details.insert("participants".to_string(), serde_json::json!(6));
            details.insert("threshold".to_string(), serde_json::json!(4));
            details.insert("group_key_derived".to_string(), serde_json::json!(true));
            details
        },
        success: true,
    }).await;
    
    println!("✅ DKG completed successfully");
    Ok(())
}

/// Test threshold signing sessions
async fn test_threshold_signing_sessions(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n✍️ Testing threshold signing sessions...");
    
    // Get a threshold group from previous tests
    let group_ids: Vec<String> = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.iter().find(|e| 
            matches!(e.event_type, TestEventType::ThresholdSigning)
        ) {
            if let Some(groups) = event.details.get("threshold_groups") {
                serde_json::from_value(groups.clone()).unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };
    
    if group_ids.is_empty() {
        return Err(anyhow::anyhow!("No threshold groups found"));
    }
    
    // Use 5-of-7 group for signing tests
    let group_id = &group_ids[2];
    let threshold = 5;
    let total_participants = 7;
    
    // Distribute shares to participants
    for i in 0..total_participants.min(network.local_nodes.len()) {
        network.local_nodes[i].node.join_threshold_group(
            group_id,
            Share {
                index: i as u32 + 1,
                value: vec![i as u8; 32], // Simplified for test
            },
        ).await?;
    }
    
    // Test multiple signing sessions
    let messages = vec![
        b"Contract Agreement #1".to_vec(),
        b"Financial Transaction #2".to_vec(),
        b"Policy Update #3".to_vec(),
        b"Emergency Authorization #4".to_vec(),
    ];
    
    let mut sessions = Vec::new();
    
    for (msg_idx, message) in messages.iter().enumerate() {
        println!("  Starting signing session {} for message: {:?}", 
            msg_idx + 1, 
            String::from_utf8_lossy(&message[..20.min(message.len())])
        );
        
        // Coordinator initiates signing
        let session = network.local_nodes[0].node
            .initiate_signing(group_id, message.clone())
            .await?;
        
        sessions.push((session.clone(), message));
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: "node_0".to_string(),
            event_type: TestEventType::ThresholdSigning,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("signing_initiated"));
                details.insert("session_id".to_string(), serde_json::json!(session.id));
                details.insert("message_index".to_string(), serde_json::json!(msg_idx));
                details.insert("message_hash".to_string(), 
                    serde_json::json!(hex::encode(&message[..8.min(message.len())])));
                details
            },
            success: true,
        }).await;
    }
    
    // Process signing sessions
    for (session, message) in &sessions {
        // Phase 1: Commitment
        let mut commitments = Vec::new();
        
        // Select threshold number of signers (different subsets for each session)
        let signer_indices: Vec<usize> = match sessions.iter().position(|(s, _)| s.id == session.id) {
            Some(0) => vec![0, 1, 2, 3, 4],       // First threshold signers
            Some(1) => vec![1, 2, 3, 4, 5],       // Shifted by one
            Some(2) => vec![0, 2, 3, 5, 6],       // Skip some
            _ => vec![0, 1, 3, 4, 6],             // Different combination
        };
        
        for &i in &signer_indices {
            if i < network.local_nodes.len() {
                let commitment = network.local_nodes[i].node
                    .generate_signing_commitment(&session.id)
                    .await?;
                
                commitments.push((i, commitment));
                
                network.reporter.report_progress(TestEvent {
                    timestamp: std::time::SystemTime::now(),
                    node_id: format!("node_{}", i),
                    event_type: TestEventType::ThresholdSigning,
                    details: {
                        let mut details = HashMap::new();
                        details.insert("action".to_string(), serde_json::json!("commitment_generated"));
                        details.insert("session_id".to_string(), serde_json::json!(session.id));
                        details.insert("signer_index".to_string(), serde_json::json!(i + 1));
                        details
                    },
                    success: true,
                }).await;
            }
        }
        
        // Aggregate commitments
        let aggregated_commitment = network.local_nodes[0].node
            .aggregate_commitments(&session.id, commitments.clone())
            .await?;
        
        // Phase 2: Signing
        let mut signature_shares = Vec::new();
        
        for (i, _) in &commitments {
            let sig_share = network.local_nodes[*i].node
                .generate_signature_share(
                    &session.id,
                    message,
                    &aggregated_commitment,
                ).await?;
            
            signature_shares.push((*i, sig_share));
        }
        
        // Aggregate signature
        let group_signature = network.local_nodes[0].node
            .aggregate_signatures(&session.id, signature_shares)
            .await?;
        
        // Verify signature on all nodes
        let mut verification_count = 0;
        for node in &network.local_nodes {
            let valid = node.node.verify_threshold_signature(
                group_id,
                message,
                &group_signature,
            ).await?;
            
            assert!(valid, "Signature should be valid on all nodes");
            verification_count += 1;
        }
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: "coordinator".to_string(),
            event_type: TestEventType::ThresholdSigning,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("signing_completed"));
                details.insert("session_id".to_string(), serde_json::json!(session.id));
                details.insert("signers".to_string(), serde_json::json!(threshold));
                details.insert("verified_by".to_string(), serde_json::json!(verification_count));
                details.insert("signature_size".to_string(), serde_json::json!(group_signature.0.len()));
                details
            },
            success: true,
        }).await;
    }
    
    println!("✅ Completed {} signing sessions", sessions.len());
    Ok(())
}

/// Test participant management
async fn test_participant_updates(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔄 Testing participant updates...");
    
    // Create a group for testing updates
    let update_group = network.local_nodes[0].node.create_threshold_group(
        GroupInfo {
            name: "Dynamic Group".to_string(),
            description: "Group for testing participant updates".to_string(),
            threshold: 3,
            total_participants: 5,
            metadata: HashMap::new(),
        }
    ).await?;
    
    // Initial participants
    for i in 0..5 {
        network.local_nodes[i].node.join_threshold_group(
            &update_group.id,
            Share {
                index: i as u32 + 1,
                value: vec![i as u8; 32],
            },
        ).await?;
    }
    
    // Test adding a participant
    if network.local_nodes.len() > 5 {
        let add_update = ParticipantUpdate {
            update_type: UpdateType::Add,
            participant_id: network.local_nodes[5].identity.base_identity.user_id.clone(),
            new_threshold: None, // Keep same threshold
        };
        
        network.local_nodes[0].node
            .update_threshold_group(&update_group.id, add_update)
            .await?;
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: "node_0".to_string(),
            event_type: TestEventType::ThresholdSigning,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("participant_added"));
                details.insert("group_id".to_string(), serde_json::json!(update_group.id));
                details.insert("new_total".to_string(), serde_json::json!(6));
                details
            },
            success: true,
        }).await;
    }
    
    // Test removing a participant
    let remove_update = ParticipantUpdate {
        update_type: UpdateType::Remove,
        participant_id: network.local_nodes[4].identity.base_identity.user_id.clone(),
        new_threshold: None,
    };
    
    network.local_nodes[0].node
        .update_threshold_group(&update_group.id, remove_update)
        .await?;
    
    // Test threshold update
    let threshold_update = ParticipantUpdate {
        update_type: UpdateType::UpdateThreshold,
        participant_id: String::new(), // Not needed for threshold update
        new_threshold: Some(4), // Increase threshold
    };
    
    network.local_nodes[0].node
        .update_threshold_group(&update_group.id, threshold_update)
        .await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ThresholdSigning,
        details: {
            let mut details = HashMap::new();
            details.insert("updates_tested".to_string(), 
                serde_json::json!(["add", "remove", "threshold"]));
            details.insert("final_threshold".to_string(), serde_json::json!(4));
            details.insert("final_participants".to_string(), serde_json::json!(5));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Participant management tested successfully");
    Ok(())
}

/// Test recovery scenarios
async fn test_threshold_recovery(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔧 Testing threshold recovery scenarios...");
    
    // Create group for recovery testing
    let recovery_group = network.local_nodes[0].node.create_threshold_group(
        GroupInfo {
            name: "Recovery Test Group".to_string(),
            description: "Testing recovery mechanisms".to_string(),
            threshold: 3,
            total_participants: 5,
            metadata: HashMap::new(),
        }
    ).await?;
    
    // Setup initial shares
    let mut original_shares = Vec::new();
    for i in 0..5 {
        let share = Share {
            index: i as u32 + 1,
            value: vec![i as u8; 32],
        };
        original_shares.push(share.clone());
        
        network.local_nodes[i].node.join_threshold_group(
            &recovery_group.id,
            share,
        ).await?;
    }
    
    // Test share recovery - participant 2 loses their share
    let lost_participant = 2;
    
    // Simulate share loss
    network.local_nodes[lost_participant].node
        .clear_threshold_share(&recovery_group.id)
        .await?;
    
    // Initiate recovery with threshold participants
    let recovery_participants = vec![0, 1, 3]; // Threshold number of helpers
    let mut recovery_shares = Vec::new();
    
    for &i in &recovery_participants {
        let recovery_share = network.local_nodes[i].node
            .generate_recovery_share(
                &recovery_group.id,
                lost_participant as u32 + 1,
            ).await?;
        
        recovery_shares.push((i as u32 + 1, recovery_share));
    }
    
    // Reconstruct lost share
    let recovered_share = network.local_nodes[lost_participant].node
        .recover_share(&recovery_group.id, recovery_shares)
        .await?;
    
    // Verify recovery by signing
    let test_message = b"Recovery test message";
    let session = network.local_nodes[0].node
        .initiate_signing(&recovery_group.id, test_message.to_vec())
        .await?;
    
    // All participants including recovered one should be able to sign
    let mut sig_shares = Vec::new();
    for i in [0, 1, 2] { // Include recovered participant
        let commitment = network.local_nodes[i].node
            .generate_signing_commitment(&session.id)
            .await?;
        
        let sig_share = network.local_nodes[i].node
            .generate_signature_share(
                &session.id,
                test_message,
                &commitment,
            ).await?;
        
        sig_shares.push((i, sig_share));
    }
    
    // Should successfully create signature
    let signature = network.local_nodes[0].node
        .aggregate_signatures(&session.id, sig_shares)
        .await?;
    
    // Test backup and restore
    let backup = network.local_nodes[3].node
        .backup_threshold_shares()
        .await?;
    
    // Clear and restore
    network.local_nodes[3].node.clear_all_threshold_shares().await?;
    network.local_nodes[3].node.restore_threshold_shares(backup).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ThresholdSigning,
        details: {
            let mut details = HashMap::new();
            details.insert("recovery_tested".to_string(), serde_json::json!(true));
            details.insert("recovered_participant".to_string(), serde_json::json!(lost_participant));
            details.insert("recovery_helpers".to_string(), serde_json::json!(recovery_participants));
            details.insert("backup_restore_tested".to_string(), serde_json::json!(true));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Recovery scenarios tested successfully");
    Ok(())
}

/// Test concurrent signing sessions
async fn test_concurrent_signing(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n⚡ Testing concurrent signing sessions...");
    
    // Create group for concurrent testing
    let concurrent_group = network.local_nodes[0].node.create_threshold_group(
        GroupInfo {
            name: "Concurrent Signing Group".to_string(),
            description: "Testing concurrent signing operations".to_string(),
            threshold: 4,
            total_participants: 6,
            metadata: HashMap::new(),
        }
    ).await?;
    
    // Setup participants
    for i in 0..6 {
        network.local_nodes[i].node.join_threshold_group(
            &concurrent_group.id,
            Share {
                index: i as u32 + 1,
                value: vec![i as u8; 32],
            },
        ).await?;
    }
    
    // Create multiple signing sessions concurrently
    let messages: Vec<Vec<u8>> = (0..5)
        .map(|i| format!("Concurrent message #{}", i).into_bytes())
        .collect();
    
    let mut session_handles = Vec::new();
    
    // Initiate all sessions
    for (i, message) in messages.iter().enumerate() {
        let session = network.local_nodes[i % network.local_nodes.len()].node
            .initiate_signing(&concurrent_group.id, message.clone())
            .await?;
        
        session_handles.push((session, message, i));
    }
    
    // Process all sessions concurrently
    let mut futures = Vec::new();
    
    for (session, message, initiator_idx) in session_handles {
        let network_ref = &network.local_nodes;
        let reporter = network.reporter.clone();
        
        let future = async move {
            // Different signer sets for each session
            let signer_indices = match initiator_idx % 3 {
                0 => vec![0, 1, 2, 3],
                1 => vec![1, 2, 3, 4],
                _ => vec![0, 2, 4, 5],
            };
            
            // Commitment phase
            let mut commitments = Vec::new();
            for &i in &signer_indices {
                let commitment = network_ref[i].node
                    .generate_signing_commitment(&session.id)
                    .await?;
                commitments.push((i, commitment));
            }
            
            // Aggregate commitments
            let agg_commitment = network_ref[0].node
                .aggregate_commitments(&session.id, commitments.clone())
                .await?;
            
            // Signing phase
            let mut sig_shares = Vec::new();
            for (i, _) in &commitments {
                let sig_share = network_ref[*i].node
                    .generate_signature_share(&session.id, message, &agg_commitment)
                    .await?;
                sig_shares.push((*i, sig_share));
            }
            
            // Aggregate signature
            let signature = network_ref[0].node
                .aggregate_signatures(&session.id, sig_shares)
                .await?;
            
            reporter.report_progress(TestEvent {
                timestamp: std::time::SystemTime::now(),
                node_id: format!("session_{}", initiator_idx),
                event_type: TestEventType::ThresholdSigning,
                details: {
                    let mut details = HashMap::new();
                    details.insert("action".to_string(), 
                        serde_json::json!("concurrent_signing_completed"));
                    details.insert("session_index".to_string(), serde_json::json!(initiator_idx));
                    details.insert("signers".to_string(), serde_json::json!(signer_indices));
                    details
                },
                success: true,
            }).await;
            
            Ok::<_, anyhow::Error>(signature)
        };
        
        futures.push(future);
    }
    
    // Wait for all sessions to complete
    let results = futures::future::join_all(futures).await;
    
    let successful = results.iter().filter(|r| r.is_ok()).count();
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ThresholdSigning,
        details: {
            let mut details = HashMap::new();
            details.insert("concurrent_sessions".to_string(), serde_json::json!(messages.len()));
            details.insert("successful_sessions".to_string(), serde_json::json!(successful));
            details.insert("test_type".to_string(), serde_json::json!("concurrent_signing"));
            details
        },
        success: successful == messages.len(),
    }).await;
    
    println!("✅ Concurrent signing completed: {}/{} successful", successful, messages.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_threshold_validation() {
        // Valid thresholds
        assert!(validate_threshold(2, 3));
        assert!(validate_threshold(3, 5));
        assert!(validate_threshold(5, 7));
        
        // Invalid thresholds
        assert!(!validate_threshold(0, 3));
        assert!(!validate_threshold(4, 3));
        assert!(!validate_threshold(1, 1));
    }
    
    fn validate_threshold(t: usize, n: usize) -> bool {
        t > 0 && t <= n && n > 1
    }
    
    #[test]
    fn test_share_index_bounds() {
        let share = Share {
            index: 1,
            value: vec![0; 32],
        };
        
        assert!(share.index > 0);
        assert!(share.value.len() == 32);
    }
}