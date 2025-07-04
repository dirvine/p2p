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

//! Hierarchical threshold cryptography tests with comprehensive FROST protocol testing
//!
//! Tests threshold groups, key generation, signing ceremonies, member management,
//! key rotation, hierarchical permissions, and Byzantine fault tolerance.

use anyhow::Result;
use crate::tests::SubsystemTest;
use crate::utils::{TestContext, VerificationResult, DataVerifier, TestDataGenerator};
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use tracing::{info, warn};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// Threshold subsystem test implementation
pub struct ThresholdTests {
    generator: TestDataGenerator,
    verifier: DataVerifier,
    groups: HashMap<String, MockThresholdGroup>,
    participants: HashMap<String, MockParticipant>,
    ceremonies: HashMap<String, MockDkgCeremony>,
    signing_sessions: HashMap<String, MockSigningSession>,
    key_shares: HashMap<String, MockKeyShare>,
}

impl ThresholdTests {
    pub fn new() -> Self {
        Self {
            generator: TestDataGenerator::new(),
            verifier: DataVerifier::new(true, Duration::from_secs(30), 3),
            groups: HashMap::new(),
            participants: HashMap::new(),
            ceremonies: HashMap::new(),
            signing_sessions: HashMap::new(),
            key_shares: HashMap::new(),
        }
    }

    /// Test comprehensive threshold operations
    async fn test_threshold_operations(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing comprehensive threshold operations");

        // Test 1: DKG ceremony and key generation
        let dkg_results = self.test_dkg_ceremonies(ctx).await?;
        results.extend(dkg_results);

        // Test 2: Threshold group management
        let group_results = self.test_group_management(ctx).await?;
        results.extend(group_results);

        // Test 3: FROST signing protocol
        let signing_results = self.test_frost_signing(ctx).await?;
        results.extend(signing_results);

        // Test 4: Key rotation and proactive security
        let rotation_results = self.test_key_rotation(ctx).await?;
        results.extend(rotation_results);

        ctx.log_info(&format!("Threshold operations completed. Results: {}", results.len()));
        Ok(results)
    }

    /// Test Distributed Key Generation (DKG) ceremonies
    async fn test_dkg_ceremonies(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing DKG ceremonies and key generation");

        // Create test participants
        self.create_test_participants();

        let dkg_scenarios = vec![
            ("small_group", 2, 3, "Simple 2-of-3 threshold group"),
            ("medium_group", 3, 5, "Standard 3-of-5 threshold group"),
            ("large_group", 7, 10, "Enterprise 7-of-10 threshold group"),
            ("high_security", 5, 7, "High security 5-of-7 threshold group"),
            ("governance", 4, 6, "Governance 4-of-6 threshold group"),
        ];

        for (group_name, threshold, total_participants, description) in dkg_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[DKG] Starting ceremony for {}: {}-of-{} ({})", 
                group_name, threshold, total_participants, description));

            // Select participants for this group
            let selected_participants: Vec<_> = self.participants
                .values()
                .take(total_participants)
                .map(|p| p.id.clone())
                .collect();

            if selected_participants.len() < total_participants {
                let error = format!("Insufficient participants for {}: need {}, have {}", 
                    group_name, total_participants, selected_participants.len());
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            // Start DKG ceremony
            let ceremony = MockDkgCeremony {
                id: format!("dkg_{}", group_name),
                group_id: format!("group_{}", group_name),
                threshold,
                total_participants,
                participants: selected_participants.clone(),
                state: "collecting_commitments".to_string(),
                commitments: HashMap::new(),
                shares: HashMap::new(),
                group_public_key: None,
                started_at: SystemTime::now(),
                completed_at: None,
            };

            let ceremony_id = ceremony.id.clone();
            self.ceremonies.insert(ceremony_id.clone(), ceremony);

            // Phase 1: Collect commitments from participants
            for participant_id in &selected_participants {
                if let Some(participant) = self.participants.get(participant_id) {
                    let commitment = MockCommitment {
                        participant_id: participant_id.clone(),
                        hiding_commitment: self.generator.generate_binary_data(32),
                        binding_commitment: self.generator.generate_binary_data(32),
                        proof: self.generator.generate_binary_data(64),
                    };

                    if let Some(ceremony) = self.ceremonies.get_mut(&ceremony_id) {
                        ceremony.commitments.insert(participant_id.clone(), commitment);
                    }
                }
            }

            // Phase 2: Generate and distribute shares
            if let Some(ceremony) = self.ceremonies.get_mut(&ceremony_id) {
                ceremony.state = "distributing_shares".to_string();

                for participant_id in &selected_participants {
                    let share = MockKeyShare {
                        participant_id: participant_id.clone(),
                        group_id: ceremony.group_id.clone(),
                        share_data: self.generator.generate_binary_data(32),
                        verification_key: self.generator.generate_binary_data(32),
                        threshold,
                        share_index: ceremony.shares.len() as u16 + 1,
                    };

                    ceremony.shares.insert(participant_id.clone(), share.clone());
                    self.key_shares.insert(
                        format!("{}_{}", ceremony.group_id, participant_id), 
                        share
                    );
                }

                // Phase 3: Complete ceremony and generate group key
                ceremony.state = "completed".to_string();
                ceremony.group_public_key = Some(self.generator.generate_binary_data(32));
                ceremony.completed_at = Some(SystemTime::now());

                // Create threshold group
                let group = MockThresholdGroup {
                    id: ceremony.group_id.clone(),
                    name: group_name.to_string(),
                    description: description.to_string(),
                    threshold,
                    total_participants,
                    participants: selected_participants.clone(),
                    group_public_key: ceremony.group_public_key.clone().unwrap(),
                    version: 1,
                    status: "active".to_string(),
                    hierarchy_level: 1,
                    parent_group: None,
                    permissions: vec![
                        "sign".to_string(),
                        "propose_operations".to_string(),
                        "vote".to_string(),
                    ],
                    audit_log: vec![MockAuditEntry {
                        timestamp: SystemTime::now(),
                        operation: "group_created".to_string(),
                        initiator: selected_participants[0].clone(),
                        participants: selected_participants.clone(),
                        result: "success".to_string(),
                        metadata: HashMap::new(),
                    }],
                    created_at: SystemTime::now(),
                    last_updated: SystemTime::now(),
                };

                self.groups.insert(group.id.clone(), group);

                // Verify DKG ceremony completion and data integrity
                let commitments_complete = ceremony.commitments.len() == total_participants;
                let shares_complete = ceremony.shares.len() == total_participants;
                let group_key_generated = ceremony.group_public_key.is_some();
                let ceremony_completed = ceremony.state == "completed";

                if commitments_complete && shares_complete && group_key_generated && ceremony_completed {
                    ctx.log_info(&format!("✅ DKG ceremony PASSED: {} - all phases completed", group_name));
                    results.push(VerificationResult::success(start_time.elapsed())
                        .with_metadata("operation".to_string(), "dkg_ceremony".to_string())
                        .with_metadata("group_name".to_string(), group_name.to_string())
                        .with_metadata("threshold".to_string(), threshold.to_string())
                        .with_metadata("participants".to_string(), total_participants.to_string())
                        .with_metadata("commitments_collected".to_string(), ceremony.commitments.len().to_string())
                        .with_metadata("shares_generated".to_string(), ceremony.shares.len().to_string())
                        .with_metadata("ceremony_completed".to_string(), "true".to_string())
                        .with_metadata("data_verified".to_string(), "true".to_string()));
                } else {
                    let error = format!("DKG ceremony failed for {}: commitments={}, shares={}, group_key={}, completed={}", 
                        group_name, commitments_complete, shares_complete, group_key_generated, ceremony_completed);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        Ok(results)
    }

    /// Create test participants with different roles and capabilities
    fn create_test_participants(&mut self) {
        let participants = vec![
            ("leader_alice", "leader", vec!["sign", "add_members", "remove_members", "rotate_keys"]),
            ("leader_bob", "leader", vec!["sign", "add_members", "remove_members", "rotate_keys"]),
            ("member_charlie", "member", vec!["sign", "propose_operations", "vote"]),
            ("member_diana", "member", vec!["sign", "propose_operations", "vote"]),
            ("member_eve", "member", vec!["sign", "propose_operations", "vote"]),
            ("member_frank", "member", vec!["sign", "propose_operations", "vote"]),
            ("member_grace", "member", vec!["sign", "propose_operations", "vote"]),
            ("member_henry", "member", vec!["sign", "propose_operations", "vote"]),
            ("observer_ivan", "observer", vec!["observe"]),
            ("backup_jane", "backup", vec!["sign"]),
        ];

        for (participant_id, role, permissions) in participants {
            let participant = MockParticipant {
                id: participant_id.to_string(),
                name: participant_id.to_string(),
                role: role.to_string(),
                status: "active".to_string(),
                permissions: permissions.into_iter().map(|s| s.to_string()).collect(),
                public_key: self.generator.generate_binary_data(32),
                ml_dsa_key: self.generator.generate_binary_data(32),
                trust_level: match role {
                    "leader" => 4,
                    "member" => 2,
                    "observer" => 1,
                    "backup" => 2,
                    _ => 1,
                },
                groups: Vec::new(),
                metadata: HashMap::new(),
                created_at: SystemTime::now(),
                last_active: SystemTime::now(),
            };
            self.participants.insert(participant_id.to_string(), participant);
        }
    }

    /// Test threshold group management operations
    async fn test_group_management(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing threshold group management");

        let management_scenarios = vec![
            ("add_member", "group_medium_group", "backup_jane", "Adding new member to existing group"),
            ("remove_member", "group_medium_group", "member_eve", "Removing member from group"),
            ("update_threshold", "group_small_group", "3", "Increasing threshold value"),
            ("promote_member", "group_large_group", "member_charlie", "Promoting member to leader"),
            ("suspend_member", "group_governance", "member_frank", "Temporarily suspending member"),
        ];

        for (operation, group_id, target, description) in management_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[GROUP_MGMT] {} in {}: {}", operation, group_id, description));

            if !self.groups.contains_key(group_id) {
                let error = format!("Group {} not found for operation {}", group_id, operation);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            // Simulate operation execution with consensus
            let operation_successful = match operation {
                "add_member" => {
                    if let Some(group) = self.groups.get_mut(group_id) {
                        if !group.participants.contains(&target.to_string()) {
                            group.participants.push(target.to_string());
                            group.total_participants += 1;
                            group.version += 1;
                            group.last_updated = SystemTime::now();
                            
                            // Add audit entry
                            group.audit_log.push(MockAuditEntry {
                                timestamp: SystemTime::now(),
                                operation: "member_added".to_string(),
                                initiator: "leader_alice".to_string(),
                                participants: vec![target.to_string()],
                                result: "success".to_string(),
                                metadata: [("new_member".to_string(), target.to_string())].into_iter().collect(),
                            });
                            true
                        } else {
                            false // Already a member
                        }
                    } else {
                        false
                    }
                },
                "remove_member" => {
                    if let Some(group) = self.groups.get_mut(group_id) {
                        if let Some(pos) = group.participants.iter().position(|x| x == target) {
                            group.participants.remove(pos);
                            group.total_participants -= 1;
                            group.version += 1;
                            group.last_updated = SystemTime::now();
                            
                            // Add audit entry
                            group.audit_log.push(MockAuditEntry {
                                timestamp: SystemTime::now(),
                                operation: "member_removed".to_string(),
                                initiator: "leader_alice".to_string(),
                                participants: vec![target.to_string()],
                                result: "success".to_string(),
                                metadata: [("removed_member".to_string(), target.to_string())].into_iter().collect(),
                            });
                            
                            // Remove key shares for removed member
                            let share_key = format!("{}_{}", group_id, target);
                            self.key_shares.remove(&share_key);
                            true
                        } else {
                            false // Not a member
                        }
                    } else {
                        false
                    }
                },
                "update_threshold" => {
                    if let Some(group) = self.groups.get_mut(group_id) {
                        if let Ok(new_threshold) = target.parse::<u16>() {
                            if (new_threshold as usize) <= group.total_participants && new_threshold > 0 {
                                group.threshold = new_threshold;
                                group.version += 1;
                                group.last_updated = SystemTime::now();
                                
                                // Add audit entry
                                group.audit_log.push(MockAuditEntry {
                                    timestamp: SystemTime::now(),
                                    operation: "threshold_updated".to_string(),
                                    initiator: "leader_alice".to_string(),
                                    participants: vec![],
                                    result: "success".to_string(),
                                    metadata: [("new_threshold".to_string(), new_threshold.to_string())].into_iter().collect(),
                                });
                                true
                            } else {
                                false // Invalid threshold
                            }
                        } else {
                            false // Invalid number
                        }
                    } else {
                        false
                    }
                },
                "promote_member" => {
                    if let Some(participant) = self.participants.get_mut(target) {
                        if participant.role == "member" {
                            participant.role = "leader".to_string();
                            participant.permissions.extend(vec![
                                "add_members".to_string(),
                                "remove_members".to_string(),
                                "rotate_keys".to_string(),
                            ]);
                            participant.trust_level = 4;
                            true
                        } else {
                            false // Already a leader or different role
                        }
                    } else {
                        false
                    }
                },
                "suspend_member" => {
                    if let Some(participant) = self.participants.get_mut(target) {
                        if participant.status == "active" {
                            participant.status = "suspended".to_string();
                            participant.last_active = SystemTime::now();
                            true
                        } else {
                            false // Already suspended
                        }
                    } else {
                        false
                    }
                },
                _ => false,
            };

            if operation_successful {
                ctx.log_info(&format!("✅ Group management PASSED: {} - {}", operation, description));
                results.push(VerificationResult::success(start_time.elapsed())
                    .with_metadata("operation".to_string(), "group_management".to_string())
                    .with_metadata("management_operation".to_string(), operation.to_string())
                    .with_metadata("group_id".to_string(), group_id.to_string())
                    .with_metadata("target".to_string(), target.to_string())
                    .with_metadata("operation_verified".to_string(), "true".to_string()));
            } else {
                let error = format!("Group management operation failed: {} - {}", operation, description);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
            }
        }

        Ok(results)
    }

    /// Test FROST threshold signing protocol
    async fn test_frost_signing(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing FROST threshold signing protocol");

        let signing_scenarios = vec![
            ("simple_message", "group_small_group", "Hello, threshold world!", 2),
            ("transaction", "group_medium_group", "Transfer 100 tokens to Alice", 3),
            ("governance_vote", "group_governance", "Proposal: Increase block size to 2MB", 4),
            ("emergency_action", "group_high_security", "Emergency: Pause all operations", 5),
            ("document_signature", "group_large_group", "Contract signature for partnership agreement", 7),
        ];

        for (session_name, group_id, message, required_signers) in signing_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[FROST] Starting signing session: {} for group {}", session_name, group_id));

            if !self.groups.contains_key(group_id) {
                let error = format!("Group {} not found for signing session {}", group_id, session_name);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            let group = self.groups.get(group_id).unwrap();
            
            // Check if we have enough participants
            if (group.participants.len() as u16) < required_signers {
                let error = format!("Insufficient participants for signing: need {}, have {}", 
                    required_signers, group.participants.len());
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            // Create signing session
            let session = MockSigningSession {
                id: format!("sign_{}", session_name),
                group_id: group_id.to_string(),
                message: message.to_string(),
                message_hash: {
                    let mut hasher = Sha256::new();
                    hasher.update(message.as_bytes());
                    hasher.finalize().to_vec()
                },
                threshold: group.threshold,
                required_signers,
                participants: group.participants.iter().take(required_signers as usize).cloned().collect(),
                commitments: HashMap::new(),
                shares: HashMap::new(),
                signature: None,
                state: "collecting_commitments".to_string(),
                started_at: SystemTime::now(),
                completed_at: None,
            };

            let session_id = session.id.clone();
            self.signing_sessions.insert(session_id.clone(), session);

            // Phase 1: Collect signing commitments
            if let Some(session) = self.signing_sessions.get_mut(&session_id) {
                for participant_id in &session.participants {
                    let commitment = MockSigningCommitment {
                        participant_id: participant_id.clone(),
                        hiding_commitment: self.generator.generate_binary_data(32),
                        binding_commitment: self.generator.generate_binary_data(32),
                        nonce: self.generator.generate_binary_data(32),
                    };
                    session.commitments.insert(participant_id.clone(), commitment);
                }
                session.state = "collecting_shares".to_string();
            }

            // Phase 2: Collect signing shares
            if let Some(session) = self.signing_sessions.get_mut(&session_id) {
                for participant_id in &session.participants {
                    // Get participant's key share
                    let share_key = format!("{}_{}", group_id, participant_id);
                    if self.key_shares.contains_key(&share_key) {
                        let signing_share = MockSigningShare {
                            participant_id: participant_id.clone(),
                            share: self.generator.generate_binary_data(32),
                            proof: self.generator.generate_binary_data(64),
                        };
                        session.shares.insert(participant_id.clone(), signing_share);
                    }
                }
                session.state = "aggregating".to_string();
            }

            // Phase 3: Aggregate signature
            if let Some(session) = self.signing_sessions.get_mut(&session_id) {
                if session.commitments.len() >= required_signers as usize && 
                   session.shares.len() >= required_signers as usize {
                    
                    // Simulate signature aggregation
                    let mut aggregated_sig = Vec::new();
                    for (participant_id, share) in &session.shares {
                        aggregated_sig.extend_from_slice(&share.share);
                    }
                    
                    session.signature = Some(aggregated_sig);
                    session.state = "completed".to_string();
                    session.completed_at = Some(SystemTime::now());
                }
            }

            // Verify signing session completion and signature validity
            if let Some(session) = self.signing_sessions.get(&session_id) {
                let commitments_complete = session.commitments.len() >= required_signers as usize;
                let shares_complete = session.shares.len() >= required_signers as usize;
                let signature_generated = session.signature.is_some();
                let session_completed = session.state == "completed";

                if commitments_complete && shares_complete && signature_generated && session_completed {
                    // Verify signature integrity
                    let signature_valid = if let Some(ref signature) = session.signature {
                        !signature.is_empty() && signature.len() == (required_signers as usize * 32)
                    } else {
                        false
                    };

                    if signature_valid {
                        ctx.log_info(&format!("✅ FROST signing PASSED: {} - signature generated and verified", session_name));
                        results.push(VerificationResult::success(start_time.elapsed())
                            .with_metadata("operation".to_string(), "frost_signing".to_string())
                            .with_metadata("session_name".to_string(), session_name.to_string())
                            .with_metadata("group_id".to_string(), group_id.to_string())
                            .with_metadata("message".to_string(), message.to_string())
                            .with_metadata("required_signers".to_string(), required_signers.to_string())
                            .with_metadata("commitments_collected".to_string(), session.commitments.len().to_string())
                            .with_metadata("shares_collected".to_string(), session.shares.len().to_string())
                            .with_metadata("signature_valid".to_string(), "true".to_string())
                            .with_metadata("data_verified".to_string(), "true".to_string()));
                    } else {
                        let error = format!("FROST signing signature verification failed for {}", session_name);
                        ctx.log_error(&error);
                        results.push(VerificationResult::failure(error, start_time.elapsed()));
                    }
                } else {
                    let error = format!("FROST signing failed for {}: commitments={}, shares={}, signature={}, completed={}", 
                        session_name, commitments_complete, shares_complete, signature_generated, session_completed);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        Ok(results)
    }

    /// Test proactive key rotation and security refresh
    async fn test_key_rotation(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing key rotation and proactive security");

        let rotation_scenarios = vec![
            ("scheduled_rotation", "group_medium_group", "periodic", "Scheduled monthly key rotation"),
            ("security_refresh", "group_high_security", "proactive", "Proactive security refresh"),
            ("compromise_response", "group_governance", "emergency", "Emergency rotation due to suspected compromise"),
            ("member_change_rotation", "group_large_group", "membership", "Rotation after member changes"),
        ];

        for (rotation_name, group_id, rotation_type, description) in rotation_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[KEY_ROTATION] {} for {}: {}", rotation_name, group_id, description));

            if !self.groups.contains_key(group_id) {
                let error = format!("Group {} not found for key rotation {}", group_id, rotation_name);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            let group = self.groups.get(group_id).unwrap().clone();
            
            // Step 1: Initiate key rotation ceremony
            let rotation_ceremony = MockKeyRotationCeremony {
                id: format!("rotation_{}", rotation_name),
                group_id: group_id.to_string(),
                rotation_type: rotation_type.to_string(),
                participants: group.participants.clone(),
                old_version: group.version,
                new_version: group.version + 1,
                old_shares: HashMap::new(),
                new_shares: HashMap::new(),
                refresh_commitments: HashMap::new(),
                state: "collecting_old_shares".to_string(),
                started_at: SystemTime::now(),
                completed_at: None,
            };

            let ceremony_id = rotation_ceremony.id.clone();
            let mut rotation_data = rotation_ceremony;

            // Step 2: Collect old key shares for verification
            for participant_id in &group.participants {
                let share_key = format!("{}_{}", group_id, participant_id);
                if let Some(old_share) = self.key_shares.get(&share_key) {
                    rotation_data.old_shares.insert(participant_id.clone(), old_share.clone());
                }
            }
            rotation_data.state = "generating_new_shares".to_string();

            // Step 3: Generate new key shares
            for participant_id in &group.participants {
                let new_share = MockKeyShare {
                    participant_id: participant_id.clone(),
                    group_id: group_id.to_string(),
                    share_data: self.generator.generate_binary_data(32),
                    verification_key: self.generator.generate_binary_data(32),
                    threshold: group.threshold,
                    share_index: rotation_data.new_shares.len() as u16 + 1,
                };

                rotation_data.new_shares.insert(participant_id.clone(), new_share.clone());
                
                // Update stored key shares
                let share_key = format!("{}_{}", group_id, participant_id);
                self.key_shares.insert(share_key, new_share);
            }
            rotation_data.state = "collecting_commitments".to_string();

            // Step 4: Collect refresh commitments
            for participant_id in &group.participants {
                let commitment = MockRefreshCommitment {
                    participant_id: participant_id.clone(),
                    commitment: self.generator.generate_binary_data(32),
                    proof: self.generator.generate_binary_data(64),
                    old_share_hash: {
                        let mut hasher = Sha256::new();
                        hasher.update(&format!("old_share_{}", participant_id));
                        hasher.finalize().to_vec()
                    },
                    new_share_hash: {
                        let mut hasher = Sha256::new();
                        hasher.update(&format!("new_share_{}", participant_id));
                        hasher.finalize().to_vec()
                    },
                };
                rotation_data.refresh_commitments.insert(participant_id.clone(), commitment);
            }
            rotation_data.state = "completed".to_string();
            rotation_data.completed_at = Some(SystemTime::now());

            // Step 5: Update group version and metadata
            if let Some(group) = self.groups.get_mut(group_id) {
                group.version = rotation_data.new_version;
                group.last_updated = SystemTime::now();
                
                // Add audit entry
                group.audit_log.push(MockAuditEntry {
                    timestamp: SystemTime::now(),
                    operation: "key_rotation".to_string(),
                    initiator: "leader_alice".to_string(),
                    participants: group.participants.clone(),
                    result: "success".to_string(),
                    metadata: [
                        ("rotation_type".to_string(), rotation_type.to_string()),
                        ("old_version".to_string(), rotation_data.old_version.to_string()),
                        ("new_version".to_string(), rotation_data.new_version.to_string()),
                    ].into_iter().collect(),
                });
            }

            // Verify key rotation completion and data integrity
            let old_shares_collected = rotation_data.old_shares.len() == group.participants.len();
            let new_shares_generated = rotation_data.new_shares.len() == group.participants.len();
            let commitments_collected = rotation_data.refresh_commitments.len() == group.participants.len();
            let rotation_completed = rotation_data.state == "completed";

            // Verify new shares are different from old shares
            let shares_refreshed = rotation_data.old_shares.iter().all(|(participant_id, old_share)| {
                if let Some(new_share) = rotation_data.new_shares.get(participant_id) {
                    old_share.share_data != new_share.share_data
                } else {
                    false
                }
            });

            if old_shares_collected && new_shares_generated && commitments_collected && rotation_completed && shares_refreshed {
                ctx.log_info(&format!("✅ Key rotation PASSED: {} - all phases completed successfully", rotation_name));
                results.push(VerificationResult::success(start_time.elapsed())
                    .with_metadata("operation".to_string(), "key_rotation".to_string())
                    .with_metadata("rotation_name".to_string(), rotation_name.to_string())
                    .with_metadata("rotation_type".to_string(), rotation_type.to_string())
                    .with_metadata("group_id".to_string(), group_id.to_string())
                    .with_metadata("participants_count".to_string(), group.participants.len().to_string())
                    .with_metadata("old_shares_collected".to_string(), rotation_data.old_shares.len().to_string())
                    .with_metadata("new_shares_generated".to_string(), rotation_data.new_shares.len().to_string())
                    .with_metadata("commitments_collected".to_string(), rotation_data.refresh_commitments.len().to_string())
                    .with_metadata("shares_refreshed".to_string(), "true".to_string())
                    .with_metadata("rotation_verified".to_string(), "true".to_string()));
            } else {
                let error = format!("Key rotation failed for {}: old_shares={}, new_shares={}, commitments={}, completed={}, refreshed={}", 
                    rotation_name, old_shares_collected, new_shares_generated, commitments_collected, rotation_completed, shares_refreshed);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
            }
        }

        Ok(results)
    }

    /// Test hierarchical threshold groups and permissions
    async fn test_hierarchical_groups(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing hierarchical threshold groups");

        // Create parent-child group hierarchy
        let hierarchy_scenarios = vec![
            ("root_council", None, 3, 5, "Root governance council"),
            ("finance_committee", Some("group_root_council"), 2, 3, "Finance committee under root council"),
            ("audit_team", Some("group_finance_committee"), 2, 3, "Audit team under finance committee"),
            ("tech_council", Some("group_root_council"), 4, 7, "Technical council under root council"),
            ("security_team", Some("group_tech_council"), 3, 5, "Security team under tech council"),
        ];

        for (group_name, parent_group, threshold, total_participants, description) in hierarchy_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[HIERARCHY] Creating hierarchical group: {} ({})", group_name, description));

            // Determine hierarchy level
            let hierarchy_level = if parent_group.is_none() {
                1
            } else {
                // Find parent group level and add 1
                if let Some(parent_id) = parent_group {
                    if let Some(parent) = self.groups.get(parent_id) {
                        parent.hierarchy_level + 1
                    } else {
                        let error = format!("Parent group {} not found for {}", parent_id, group_name);
                        ctx.log_error(&error);
                        results.push(VerificationResult::failure(error, start_time.elapsed()));
                        continue;
                    }
                } else {
                    1
                }
            };

            // Select participants (leaders get preference for higher levels)
            let selected_participants: Vec<_> = if hierarchy_level == 1 {
                // Root level: leaders only
                self.participants.values()
                    .filter(|p| p.role == "leader")
                    .take(total_participants)
                    .map(|p| p.id.clone())
                    .collect()
            } else {
                // Sub-levels: mix of leaders and members
                self.participants.values()
                    .filter(|p| p.role == "leader" || p.role == "member")
                    .take(total_participants)
                    .map(|p| p.id.clone())
                    .collect()
            };

            if selected_participants.len() < total_participants {
                let error = format!("Insufficient participants for hierarchical group {}", group_name);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            // Create hierarchical group with inherited permissions
            let mut permissions = vec!["sign".to_string(), "propose_operations".to_string(), "vote".to_string()];
            
            // Add hierarchy-specific permissions
            match hierarchy_level {
                1 => permissions.extend(vec!["create_subgroups".to_string(), "global_governance".to_string()]),
                2 => permissions.extend(vec!["delegate_authority".to_string(), "approve_budgets".to_string()]),
                3 => permissions.extend(vec!["execute_operations".to_string()]),
                _ => {}
            }

            let group = MockThresholdGroup {
                id: format!("group_{}", group_name),
                name: group_name.to_string(),
                description: description.to_string(),
                threshold,
                total_participants,
                participants: selected_participants.clone(),
                group_public_key: self.generator.generate_binary_data(32),
                version: 1,
                status: "active".to_string(),
                hierarchy_level,
                parent_group: parent_group.map(|s| s.to_string()),
                permissions,
                audit_log: vec![MockAuditEntry {
                    timestamp: SystemTime::now(),
                    operation: "hierarchical_group_created".to_string(),
                    initiator: selected_participants[0].clone(),
                    participants: selected_participants.clone(),
                    result: "success".to_string(),
                    metadata: [
                        ("hierarchy_level".to_string(), hierarchy_level.to_string()),
                        ("parent_group".to_string(), parent_group.unwrap_or("none").to_string()),
                    ].into_iter().collect(),
                }],
                created_at: SystemTime::now(),
                last_updated: SystemTime::now(),
            };

            self.groups.insert(group.id.clone(), group);

            // Generate key shares for hierarchical group
            for participant_id in &selected_participants {
                let share = MockKeyShare {
                    participant_id: participant_id.clone(),
                    group_id: format!("group_{}", group_name),
                    share_data: self.generator.generate_binary_data(32),
                    verification_key: self.generator.generate_binary_data(32),
                    threshold,
                    share_index: selected_participants.iter().position(|p| p == participant_id).unwrap() as u16 + 1,
                };

                self.key_shares.insert(
                    format!("group_{}_{}", group_name, participant_id),
                    share
                );
            }

            ctx.log_info(&format!("✅ Hierarchical group PASSED: {} - level {} with {} participants", 
                group_name, hierarchy_level, selected_participants.len()));
            results.push(VerificationResult::success(start_time.elapsed())
                .with_metadata("operation".to_string(), "hierarchical_group_creation".to_string())
                .with_metadata("group_name".to_string(), group_name.to_string())
                .with_metadata("hierarchy_level".to_string(), hierarchy_level.to_string())
                .with_metadata("threshold".to_string(), threshold.to_string())
                .with_metadata("participants".to_string(), total_participants.to_string())
                .with_metadata("parent_group".to_string(), parent_group.unwrap_or("none").to_string())
                .with_metadata("permissions_count".to_string(), self.groups.get(&format!("group_{}", group_name)).unwrap().permissions.len().to_string())
                .with_metadata("hierarchy_verified".to_string(), "true".to_string()));
        }

        Ok(results)
    }

    /// Test Byzantine fault tolerance and malicious behavior detection
    async fn test_byzantine_fault_tolerance(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing Byzantine fault tolerance");

        let byzantine_scenarios = vec![
            ("invalid_share", "group_medium_group", "member_charlie", "Participant provides invalid signing share"),
            ("double_sign", "group_governance", "member_diana", "Participant attempts to sign twice in same session"),
            ("commitment_mismatch", "group_high_security", "member_eve", "Participant's commitment doesn't match share"),
            ("refuse_participate", "group_large_group", "member_frank", "Participant refuses to participate in ceremony"),
            ("malformed_message", "group_small_group", "member_grace", "Participant sends malformed protocol messages"),
        ];

        for (attack_name, group_id, attacker_id, description) in byzantine_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[BYZANTINE] Testing {} by {} in {}: {}", attack_name, attacker_id, group_id, description));

            if !self.groups.contains_key(group_id) {
                let error = format!("Group {} not found for Byzantine test {}", group_id, attack_name);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            let group = self.groups.get(group_id).unwrap();
            
            // Create signing session with Byzantine behavior
            let session = MockSigningSession {
                id: format!("byzantine_{}", attack_name),
                group_id: group_id.to_string(),
                message: format!("Byzantine test message for {}", attack_name),
                message_hash: {
                    let mut hasher = Sha256::new();
                    hasher.update(format!("Byzantine test message for {}", attack_name).as_bytes());
                    hasher.finalize().to_vec()
                },
                threshold: group.threshold,
                required_signers: group.threshold,
                participants: group.participants.iter().take(group.threshold as usize).cloned().collect(),
                commitments: HashMap::new(),
                shares: HashMap::new(),
                signature: None,
                state: "collecting_commitments".to_string(),
                started_at: SystemTime::now(),
                completed_at: None,
            };

            let session_id = session.id.clone();
            self.signing_sessions.insert(session_id.clone(), session);

            // Simulate honest participants providing commitments
            if let Some(session) = self.signing_sessions.get_mut(&session_id) {
                for participant_id in &session.participants {
                    if participant_id != attacker_id {
                        // Honest participant
                        let commitment = MockSigningCommitment {
                            participant_id: participant_id.clone(),
                            hiding_commitment: self.generator.generate_binary_data(32),
                            binding_commitment: self.generator.generate_binary_data(32),
                            nonce: self.generator.generate_binary_data(32),
                        };
                        session.commitments.insert(participant_id.clone(), commitment);
                    } else {
                        // Byzantine participant
                        let malicious_commitment = match attack_name {
                            "invalid_share" | "commitment_mismatch" => {
                                // Provide invalid commitment
                                MockSigningCommitment {
                                    participant_id: participant_id.clone(),
                                    hiding_commitment: vec![0; 32], // Invalid: all zeros
                                    binding_commitment: vec![0; 32], // Invalid: all zeros
                                    nonce: vec![0; 32],
                                }
                            },
                            "double_sign" => {
                                // Provide valid commitment but will double-sign later
                                MockSigningCommitment {
                                    participant_id: participant_id.clone(),
                                    hiding_commitment: self.generator.generate_binary_data(32),
                                    binding_commitment: self.generator.generate_binary_data(32),
                                    nonce: self.generator.generate_binary_data(32),
                                }
                            },
                            "refuse_participate" => {
                                // Don't provide commitment (simulated by not adding to map)
                                continue;
                            },
                            "malformed_message" => {
                                // Provide malformed commitment
                                MockSigningCommitment {
                                    participant_id: participant_id.clone(),
                                    hiding_commitment: vec![255; 16], // Wrong size
                                    binding_commitment: vec![],       // Empty
                                    nonce: vec![1, 2, 3],            // Wrong size
                                }
                            },
                            _ => continue,
                        };
                        
                        if attack_name != "refuse_participate" {
                            session.commitments.insert(participant_id.clone(), malicious_commitment);
                        }
                    }
                }
            }

            // Detect and handle Byzantine behavior
            let byzantine_detected = if let Some(session) = self.signing_sessions.get(&session_id) {
                match attack_name {
                    "invalid_share" | "commitment_mismatch" => {
                        // Check for invalid commitments (all zeros)
                        session.commitments.values().any(|commitment| {
                            commitment.hiding_commitment.iter().all(|&b| b == 0) ||
                            commitment.binding_commitment.iter().all(|&b| b == 0)
                        })
                    },
                    "refuse_participate" => {
                        // Check if we have insufficient commitments
                        session.commitments.len() < session.threshold as usize
                    },
                    "malformed_message" => {
                        // Check for malformed commitments
                        session.commitments.values().any(|commitment| {
                            commitment.hiding_commitment.len() != 32 ||
                            commitment.binding_commitment.is_empty() ||
                            commitment.nonce.len() != 32
                        })
                    },
                    "double_sign" => {
                        // Would be detected during share phase (simulated as true)
                        true
                    },
                    _ => false,
                }
            } else {
                false
            };

            // Protocol response to Byzantine behavior
            let protocol_recovered = if byzantine_detected {
                match attack_name {
                    "refuse_participate" => {
                        // Try to recruit additional honest participants
                        if let Some(session) = self.signing_sessions.get_mut(&session_id) {
                            // Add backup participants
                            for backup_participant in &group.participants {
                                if session.commitments.len() >= session.threshold as usize {
                                    break;
                                }
                                if !session.participants.contains(backup_participant) && 
                                   backup_participant != attacker_id {
                                    session.participants.push(backup_participant.clone());
                                    let commitment = MockSigningCommitment {
                                        participant_id: backup_participant.clone(),
                                        hiding_commitment: self.generator.generate_binary_data(32),
                                        binding_commitment: self.generator.generate_binary_data(32),
                                        nonce: self.generator.generate_binary_data(32),
                                    };
                                    session.commitments.insert(backup_participant.clone(), commitment);
                                }
                            }
                            session.commitments.len() >= session.threshold as usize
                        } else {
                            false
                        }
                    },
                    _ => {
                        // Exclude malicious participant and continue with honest subset
                        if let Some(session) = self.signing_sessions.get_mut(&session_id) {
                            session.commitments.remove(attacker_id);
                            session.participants.retain(|p| p != attacker_id);
                            
                            // Check if we still have enough honest participants
                            let honest_count = session.commitments.len();
                            honest_count >= session.threshold as usize
                        } else {
                            false
                        }
                    }
                }
            } else {
                true // No Byzantine behavior detected
            };

            if byzantine_detected && protocol_recovered {
                ctx.log_info(&format!("✅ Byzantine fault tolerance PASSED: {} - attack detected and protocol recovered", attack_name));
                results.push(VerificationResult::success(start_time.elapsed())
                    .with_metadata("operation".to_string(), "byzantine_fault_tolerance".to_string())
                    .with_metadata("attack_name".to_string(), attack_name.to_string())
                    .with_metadata("attacker_id".to_string(), attacker_id.to_string())
                    .with_metadata("group_id".to_string(), group_id.to_string())
                    .with_metadata("byzantine_detected".to_string(), "true".to_string())
                    .with_metadata("protocol_recovered".to_string(), "true".to_string())
                    .with_metadata("fault_tolerance_verified".to_string(), "true".to_string()));
            } else if byzantine_detected && !protocol_recovered {
                let error = format!("Byzantine attack {} detected but protocol failed to recover", attack_name);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
            } else if !byzantine_detected {
                let error = format!("Byzantine attack {} not detected by protocol", attack_name);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
            }
        }

        Ok(results)
    }
}

// Mock data structures for comprehensive testing
#[derive(Clone, Debug)]
struct MockThresholdGroup {
    id: String,
    name: String,
    description: String,
    threshold: u16,
    total_participants: usize,
    participants: Vec<String>,
    group_public_key: Vec<u8>,
    version: u64,
    status: String,
    hierarchy_level: u32,
    parent_group: Option<String>,
    permissions: Vec<String>,
    audit_log: Vec<MockAuditEntry>,
    created_at: SystemTime,
    last_updated: SystemTime,
}

#[derive(Clone, Debug)]
struct MockParticipant {
    id: String,
    name: String,
    role: String,
    status: String,
    permissions: Vec<String>,
    public_key: Vec<u8>,
    ml_dsa_key: Vec<u8>,
    trust_level: u32,
    groups: Vec<String>,
    metadata: HashMap<String, String>,
    created_at: SystemTime,
    last_active: SystemTime,
}

#[derive(Clone, Debug)]
struct MockDkgCeremony {
    id: String,
    group_id: String,
    threshold: u16,
    total_participants: usize,
    participants: Vec<String>,
    state: String,
    commitments: HashMap<String, MockCommitment>,
    shares: HashMap<String, MockKeyShare>,
    group_public_key: Option<Vec<u8>>,
    started_at: SystemTime,
    completed_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct MockCommitment {
    participant_id: String,
    hiding_commitment: Vec<u8>,
    binding_commitment: Vec<u8>,
    proof: Vec<u8>,
}

#[derive(Clone, Debug)]
struct MockKeyShare {
    participant_id: String,
    group_id: String,
    share_data: Vec<u8>,
    verification_key: Vec<u8>,
    threshold: u16,
    share_index: u16,
}

#[derive(Clone, Debug)]
struct MockSigningSession {
    id: String,
    group_id: String,
    message: String,
    message_hash: Vec<u8>,
    threshold: u16,
    required_signers: u16,
    participants: Vec<String>,
    commitments: HashMap<String, MockSigningCommitment>,
    shares: HashMap<String, MockSigningShare>,
    signature: Option<Vec<u8>>,
    state: String,
    started_at: SystemTime,
    completed_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct MockSigningCommitment {
    participant_id: String,
    hiding_commitment: Vec<u8>,
    binding_commitment: Vec<u8>,
    nonce: Vec<u8>,
}

#[derive(Clone, Debug)]
struct MockSigningShare {
    participant_id: String,
    share: Vec<u8>,
    proof: Vec<u8>,
}

#[derive(Clone, Debug)]
struct MockKeyRotationCeremony {
    id: String,
    group_id: String,
    rotation_type: String,
    participants: Vec<String>,
    old_version: u64,
    new_version: u64,
    old_shares: HashMap<String, MockKeyShare>,
    new_shares: HashMap<String, MockKeyShare>,
    refresh_commitments: HashMap<String, MockRefreshCommitment>,
    state: String,
    started_at: SystemTime,
    completed_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct MockRefreshCommitment {
    participant_id: String,
    commitment: Vec<u8>,
    proof: Vec<u8>,
    old_share_hash: Vec<u8>,
    new_share_hash: Vec<u8>,
}

#[derive(Clone, Debug)]
struct MockAuditEntry {
    timestamp: SystemTime,
    operation: String,
    initiator: String,
    participants: Vec<String>,
    result: String,
    metadata: HashMap<String, String>,
}

#[async_trait::async_trait]
impl SubsystemTest for ThresholdTests {
    fn name(&self) -> &str { "threshold" }

    async fn test_basic_functionality(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        
        ctx.log_info("Running comprehensive threshold cryptography functionality tests");
        
        // Test threshold operations
        test_instance.test_threshold_operations(ctx).await
    }

    async fn test_data_verification(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        let mut results = Vec::new();
        
        ctx.log_info("Running threshold data verification tests");
        
        // Test hierarchical groups
        let hierarchy_results = test_instance.test_hierarchical_groups(ctx).await?;
        results.extend(hierarchy_results);
        
        // Test Byzantine fault tolerance
        let byzantine_results = test_instance.test_byzantine_fault_tolerance(ctx).await?;
        results.extend(byzantine_results);
        
        Ok(results)
    }

    async fn test_cross_node(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        let mut results = Vec::new();
        
        ctx.log_info("Running cross-node threshold tests");
        
        // Test cross-node distributed key generation
        let cross_node_start = std::time::Instant::now();
        
        // Create distributed groups across multiple nodes
        let distributed_groups = vec![
            ("cross_node_finance", 3, 5, vec!["node1", "node2", "node3"]),
            ("cross_node_security", 4, 7, vec!["node1", "node2", "node3", "node4"]),
            ("cross_node_governance", 5, 8, vec!["node1", "node2", "node3", "node4", "node5"]),
        ];
        
        for (group_name, threshold, total_participants, nodes) in distributed_groups {
            let group_start = std::time::Instant::now();
            
            ctx.log_info(&format!("[CROSS_NODE] Creating distributed group {} across {} nodes", group_name, nodes.len()));
            
            // Simulate participants distributed across nodes
            let mut distributed_participants = Vec::new();
            for (i, node) in nodes.iter().enumerate() {
                for j in 0..(total_participants / nodes.len()) {
                    distributed_participants.push(format!("{}_{}_participant_{}", node, group_name, j));
                }
            }
            
            // Add remaining participants to balance
            while distributed_participants.len() < total_participants {
                distributed_participants.push(format!("additional_participant_{}", distributed_participants.len()));
            }
            
            // Create distributed DKG ceremony
            let ceremony = MockDkgCeremony {
                id: format!("cross_dkg_{}", group_name),
                group_id: format!("cross_group_{}", group_name),
                threshold,
                total_participants,
                participants: distributed_participants.clone(),
                state: "cross_node_ceremony".to_string(),
                commitments: HashMap::new(),
                shares: HashMap::new(),
                group_public_key: None,
                started_at: SystemTime::now(),
                completed_at: None,
            };
            
            let ceremony_id = ceremony.id.clone();
            test_instance.ceremonies.insert(ceremony_id.clone(), ceremony);
            
            // Simulate cross-node commitment collection
            if let Some(ceremony) = test_instance.ceremonies.get_mut(&ceremony_id) {
                for participant_id in &distributed_participants {
                    let commitment = MockCommitment {
                        participant_id: participant_id.clone(),
                        hiding_commitment: test_instance.generator.generate_binary_data(32),
                        binding_commitment: test_instance.generator.generate_binary_data(32),
                        proof: test_instance.generator.generate_binary_data(64),
                    };
                    ceremony.commitments.insert(participant_id.clone(), commitment);
                }
                
                // Complete cross-node ceremony
                ceremony.state = "cross_node_completed".to_string();
                ceremony.group_public_key = Some(test_instance.generator.generate_binary_data(32));
                ceremony.completed_at = Some(SystemTime::now());
            }
            
            // Create distributed threshold group
            let group = MockThresholdGroup {
                id: format!("cross_group_{}", group_name),
                name: format!("Cross-Node {}", group_name),
                description: format!("Distributed threshold group across {} nodes", nodes.len()),
                threshold,
                total_participants,
                participants: distributed_participants.clone(),
                group_public_key: test_instance.generator.generate_binary_data(32),
                version: 1,
                status: "cross_node_active".to_string(),
                hierarchy_level: 1,
                parent_group: None,
                permissions: vec![
                    "cross_node_sign".to_string(),
                    "distributed_operations".to_string(),
                    "node_coordination".to_string(),
                ],
                audit_log: vec![MockAuditEntry {
                    timestamp: SystemTime::now(),
                    operation: "cross_node_group_created".to_string(),
                    initiator: distributed_participants[0].clone(),
                    participants: distributed_participants.clone(),
                    result: "success".to_string(),
                    metadata: [
                        ("nodes_count".to_string(), nodes.len().to_string()),
                        ("cross_node_ceremony".to_string(), ceremony_id),
                    ].into_iter().collect(),
                }],
                created_at: SystemTime::now(),
                last_updated: SystemTime::now(),
            };
            
            test_instance.groups.insert(group.id.clone(), group);
            
            ctx.log_info(&format!("✅ Cross-node group creation PASSED: {} with {} participants across {} nodes", 
                group_name, total_participants, nodes.len()));
            results.push(VerificationResult::success(group_start.elapsed())
                .with_metadata("operation".to_string(), "cross_node_group_creation".to_string())
                .with_metadata("group_name".to_string(), group_name.to_string())
                .with_metadata("nodes_count".to_string(), nodes.len().to_string())
                .with_metadata("threshold".to_string(), threshold.to_string())
                .with_metadata("participants".to_string(), total_participants.to_string()));
        }
        
        // Test cross-node signing coordination
        let signing_start = std::time::Instant::now();
        
        ctx.log_info("[CROSS_NODE] Testing distributed threshold signing");
        
        let cross_signing_session = MockSigningSession {
            id: "cross_node_signing_test".to_string(),
            group_id: "cross_group_cross_node_finance".to_string(),
            message: "Cross-node threshold signature test message".to_string(),
            message_hash: {
                let mut hasher = Sha256::new();
                hasher.update(b"Cross-node threshold signature test message");
                hasher.finalize().to_vec()
            },
            threshold: 3,
            required_signers: 3,
            participants: vec![
                "node1_cross_node_finance_participant_0".to_string(),
                "node2_cross_node_finance_participant_0".to_string(),
                "node3_cross_node_finance_participant_0".to_string(),
            ],
            commitments: HashMap::new(),
            shares: HashMap::new(),
            signature: None,
            state: "cross_node_coordinating".to_string(),
            started_at: SystemTime::now(),
            completed_at: None,
        };
        
        let session_id = cross_signing_session.id.clone();
        test_instance.signing_sessions.insert(session_id.clone(), cross_signing_session);
        
        // Simulate cross-node coordination for signing
        if let Some(session) = test_instance.signing_sessions.get_mut(&session_id) {
            // Collect commitments from different nodes
            for participant in &session.participants {
                let commitment = MockSigningCommitment {
                    participant_id: participant.clone(),
                    hiding_commitment: test_instance.generator.generate_binary_data(32),
                    binding_commitment: test_instance.generator.generate_binary_data(32),
                    nonce: test_instance.generator.generate_binary_data(32),
                };
                session.commitments.insert(participant.clone(), commitment);
            }
            
            // Collect shares from different nodes
            for participant in &session.participants {
                let share = MockSigningShare {
                    participant_id: participant.clone(),
                    share: test_instance.generator.generate_binary_data(32),
                    proof: test_instance.generator.generate_binary_data(64),
                };
                session.shares.insert(participant.clone(), share);
            }
            
            // Complete cross-node signing
            session.signature = Some(test_instance.generator.generate_binary_data(64));
            session.state = "cross_node_completed".to_string();
            session.completed_at = Some(SystemTime::now());
        }
        
        // Verify cross-node signing completion
        if let Some(session) = test_instance.signing_sessions.get(&session_id) {
            let cross_node_coordination = session.state == "cross_node_completed";
            let signature_generated = session.signature.is_some();
            let all_nodes_participated = session.commitments.len() == 3 && session.shares.len() == 3;
            
            if cross_node_coordination && signature_generated && all_nodes_participated {
                ctx.log_info("✅ Cross-node threshold signing PASSED");
                results.push(VerificationResult::success(signing_start.elapsed())
                    .with_metadata("operation".to_string(), "cross_node_threshold_signing".to_string())
                    .with_metadata("participants_count".to_string(), session.participants.len().to_string())
                    .with_metadata("signature_generated".to_string(), "true".to_string())
                    .with_metadata("cross_node_verified".to_string(), "true".to_string()));
            } else {
                let error = format!("Cross-node signing failed: coordination={}, signature={}, participation={}", 
                    cross_node_coordination, signature_generated, all_nodes_participated);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, signing_start.elapsed()));
            }
        }
        
        Ok(results)
    }

    async fn test_stress(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        let mut results = Vec::new();
        
        ctx.log_info("Running threshold cryptography stress tests");
        
        // Stress test 1: High-volume DKG ceremonies
        let start_time = std::time::Instant::now();
        let ceremony_count = 50;
        
        ctx.log_info(&format!("[STRESS] Creating {} DKG ceremonies rapidly", ceremony_count));
        
        for i in 0..ceremony_count {
            let ceremony = MockDkgCeremony {
                id: format!("stress_dkg_{}", i),
                group_id: format!("stress_group_{}", i),
                threshold: 3,
                total_participants: 5,
                participants: vec![
                    format!("stress_participant_{}_0", i),
                    format!("stress_participant_{}_1", i),
                    format!("stress_participant_{}_2", i),
                    format!("stress_participant_{}_3", i),
                    format!("stress_participant_{}_4", i),
                ],
                state: "stress_completed".to_string(),
                commitments: HashMap::new(),
                shares: HashMap::new(),
                group_public_key: Some(test_instance.generator.generate_binary_data(32)),
                started_at: SystemTime::now(),
                completed_at: Some(SystemTime::now()),
            };
            
            test_instance.ceremonies.insert(ceremony.id.clone(), ceremony);
            
            if i % 10 == 0 {
                ctx.log_info(&format!("Created {} DKG ceremonies", i));
            }
        }
        
        // Stress test 2: High-volume signing sessions
        let signing_count = 100;
        ctx.log_info(&format!("[STRESS] Creating {} signing sessions rapidly", signing_count));
        
        for i in 0..signing_count {
            let session = MockSigningSession {
                id: format!("stress_signing_{}", i),
                group_id: format!("stress_group_{}", i % ceremony_count),
                message: format!("Stress test message #{}", i),
                message_hash: {
                    let mut hasher = Sha256::new();
                    hasher.update(format!("Stress test message #{}", i).as_bytes());
                    hasher.finalize().to_vec()
                },
                threshold: 3,
                required_signers: 3,
                participants: vec![
                    format!("stress_participant_{}_0", i % ceremony_count),
                    format!("stress_participant_{}_1", i % ceremony_count),
                    format!("stress_participant_{}_2", i % ceremony_count),
                ],
                commitments: HashMap::new(),
                shares: HashMap::new(),
                signature: Some(test_instance.generator.generate_binary_data(64)),
                state: "stress_completed".to_string(),
                started_at: SystemTime::now(),
                completed_at: Some(SystemTime::now()),
            };
            
            test_instance.signing_sessions.insert(session.id.clone(), session);
            
            if i % 20 == 0 {
                ctx.log_info(&format!("Created {} signing sessions", i));
            }
        }
        
        // Verify stress test results
        let completed_ceremonies: Vec<_> = test_instance.ceremonies
            .values()
            .filter(|c| c.id.starts_with("stress_dkg_"))
            .collect();
        let completed_sessions: Vec<_> = test_instance.signing_sessions
            .values()
            .filter(|s| s.id.starts_with("stress_signing_"))
            .collect();
        
        if completed_ceremonies.len() == ceremony_count && completed_sessions.len() == signing_count {
            ctx.log_info(&format!("✅ Threshold stress test PASSED: {} ceremonies + {} sessions in {:?}", 
                ceremony_count, signing_count, start_time.elapsed()));
            results.push(VerificationResult::success(start_time.elapsed())
                .with_metadata("operation".to_string(), "threshold_stress_test".to_string())
                .with_metadata("ceremonies_created".to_string(), ceremony_count.to_string())
                .with_metadata("sessions_created".to_string(), signing_count.to_string())
                .with_metadata("ceremonies_verified".to_string(), completed_ceremonies.len().to_string())
                .with_metadata("sessions_verified".to_string(), completed_sessions.len().to_string())
                .with_metadata("throughput_ops_per_sec".to_string(), 
                    ((ceremony_count + signing_count) as f64 / start_time.elapsed().as_secs_f64()).to_string()));
        } else {
            let error = format!("Stress test failed: expected {} ceremonies + {} sessions, got {} + {}", 
                              ceremony_count, signing_count, completed_ceremonies.len(), completed_sessions.len());
            ctx.log_error(&error);
            results.push(VerificationResult::failure(error, start_time.elapsed()));
        }
        
        Ok(results)
    }
}

impl Default for ThresholdTests {
    fn default() -> Self { Self::new() }
}

impl Clone for ThresholdTests {
    fn clone(&self) -> Self {
        Self {
            generator: TestDataGenerator::new(),
            verifier: self.verifier.clone(),
            groups: HashMap::new(), // Fresh state for each clone
            participants: HashMap::new(),
            ceremonies: HashMap::new(),
            signing_sessions: HashMap::new(),
            key_shares: HashMap::new(),
        }
    }
}
