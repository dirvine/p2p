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

//! Threshold group management operations

use super::*;
use crate::quantum_crypto::types::*;
use std::collections::HashSet;

impl ThresholdGroup {
    /// Check if a participant has a specific permission
    pub fn check_permission(
        &self,
        participant_id: &ParticipantId,
        permission: Permission,
    ) -> Result<()> {
        let participant = self
            .active_participants
            .iter()
            .find(|p| &p.participant_id == participant_id)
            .ok_or_else(|| ThresholdError::ParticipantNotFound(participant_id.clone()))?;

        match (&participant.role, permission) {
            (ParticipantRole::Leader { permissions }, Permission::AddParticipant) => {
                if permissions.can_add_participants {
                    Ok(())
                } else {
                    Err(ThresholdError::Unauthorized(
                        "Cannot add participants".to_string(),
                    ))
                }
            }
            (ParticipantRole::Leader { permissions }, Permission::RemoveParticipant) => {
                if permissions.can_remove_participants {
                    Ok(())
                } else {
                    Err(ThresholdError::Unauthorized(
                        "Cannot remove participants".to_string(),
                    ))
                }
            }
            (ParticipantRole::Leader { permissions }, Permission::UpdateThreshold) => {
                if permissions.can_update_threshold {
                    Ok(())
                } else {
                    Err(ThresholdError::Unauthorized(
                        "Cannot update threshold".to_string(),
                    ))
                }
            }
            (ParticipantRole::Member { permissions }, Permission::Sign) => {
                if permissions.can_sign {
                    Ok(())
                } else {
                    Err(ThresholdError::Unauthorized("Cannot sign".to_string()))
                }
            }
            (ParticipantRole::Observer, _) => Err(ThresholdError::Unauthorized(
                "Observers have read-only access".to_string(),
            )),
            _ => Err(ThresholdError::Unauthorized(
                "Permission denied".to_string(),
            )),
        }
    }

    /// Get active participants (not suspended or pending removal)
    pub fn get_active_participants(&self) -> Vec<&ParticipantInfo> {
        self.active_participants
            .iter()
            .filter(|p| matches!(p.status, ParticipantStatus::Active))
            .collect()
    }

    /// Get number of active participants
    pub fn active_participant_count(&self) -> u16 {
        self.get_active_participants().len() as u16
    }

    /// Check if we have enough participants for threshold operations
    pub fn has_threshold_participants(&self) -> bool {
        self.active_participant_count() >= self.threshold
    }

    /// Add a new participant (pending until key ceremony)
    pub fn add_pending_participant(&mut self, participant: ParticipantInfo) -> Result<()> {
        // Check if participant already exists
        if self
            .active_participants
            .iter()
            .any(|p| p.participant_id == participant.participant_id)
        {
            return Err(ThresholdError::InvalidParameters(
                "Participant already exists".to_string(),
            ));
        }

        if self
            .pending_participants
            .iter()
            .any(|p| p.participant_id == participant.participant_id)
        {
            return Err(ThresholdError::InvalidParameters(
                "Participant already pending".to_string(),
            ));
        }

        self.pending_participants.push(participant);
        self.version += 1;
        self.last_updated = SystemTime::now();

        Ok(())
    }

    /// Mark participant for removal
    pub fn mark_for_removal(&mut self, participant_id: &ParticipantId) -> Result<()> {
        let participant = self
            .active_participants
            .iter_mut()
            .find(|p| &p.participant_id == participant_id)
            .ok_or_else(|| ThresholdError::ParticipantNotFound(participant_id.clone()))?;

        participant.status = ParticipantStatus::PendingRemoval;
        self.version += 1;
        self.last_updated = SystemTime::now();

        // Check if we still have enough participants
        if self.active_participant_count() < self.threshold {
            return Err(ThresholdError::InsufficientParticipants {
                required: self.threshold,
                available: self.active_participant_count(),
            });
        }

        Ok(())
    }

    /// Update participant role
    pub fn update_participant_role(
        &mut self,
        participant_id: &ParticipantId,
        new_role: ParticipantRole,
    ) -> Result<()> {
        let participant = self
            .active_participants
            .iter_mut()
            .find(|p| &p.participant_id == participant_id)
            .ok_or_else(|| ThresholdError::ParticipantNotFound(participant_id.clone()))?;

        participant.role = new_role;
        self.version += 1;
        self.last_updated = SystemTime::now();

        Ok(())
    }

    /// Suspend a participant
    pub fn suspend_participant(
        &mut self,
        participant_id: &ParticipantId,
        reason: String,
        duration: std::time::Duration,
    ) -> Result<()> {
        let participant = self
            .active_participants
            .iter_mut()
            .find(|p| &p.participant_id == participant_id)
            .ok_or_else(|| ThresholdError::ParticipantNotFound(participant_id.clone()))?;

        participant.status = ParticipantStatus::Suspended {
            reason,
            until: SystemTime::now() + duration,
        };

        self.version += 1;
        self.last_updated = SystemTime::now();

        // Check if we still have enough participants
        if self.active_participant_count() < self.threshold {
            return Err(ThresholdError::InsufficientParticipants {
                required: self.threshold,
                available: self.active_participant_count(),
            });
        }

        Ok(())
    }

    /// Update threshold value
    pub fn update_threshold(&mut self, new_threshold: u16) -> Result<()> {
        if new_threshold == 0 {
            return Err(ThresholdError::InvalidParameters(
                "Threshold must be at least 1".to_string(),
            ));
        }

        if new_threshold > self.participants {
            return Err(ThresholdError::InvalidParameters(
                "Threshold cannot exceed total participants".to_string(),
            ));
        }

        if new_threshold > self.active_participant_count() {
            return Err(ThresholdError::InvalidParameters(
                "Threshold cannot exceed active participants".to_string(),
            ));
        }

        self.threshold = new_threshold;
        self.version += 1;
        self.last_updated = SystemTime::now();

        Ok(())
    }

    /// Get participants by role
    pub fn get_participants_by_role(&self, role_filter: RoleFilter) -> Vec<&ParticipantInfo> {
        self.active_participants
            .iter()
            .filter(|p| match (&p.role, &role_filter) {
                (ParticipantRole::Leader { .. }, RoleFilter::Leaders) => true,
                (ParticipantRole::Member { .. }, RoleFilter::Members) => true,
                (ParticipantRole::Observer, RoleFilter::Observers) => true,
                (_, RoleFilter::All) => true,
                _ => false,
            })
            .collect()
    }

    /// Get group hierarchy (if part of a larger structure)
    pub fn get_hierarchy(&self) -> GroupHierarchy {
        GroupHierarchy {
            group_id: self.group_id.clone(),
            parent: self.metadata.parent_group.clone(),
            name: self.metadata.name.clone(),
            threshold: self.threshold,
            participants: self.participants,
            purpose: self.metadata.purpose.clone(),
        }
    }

    /// Validate group state
    pub fn validate(&self) -> Result<()> {
        // Check basic constraints
        if self.threshold == 0 {
            return Err(ThresholdError::InvalidParameters(
                "Invalid threshold: must be at least 1".to_string(),
            ));
        }

        if self.threshold > self.participants {
            return Err(ThresholdError::InvalidParameters(
                "Invalid threshold: exceeds total participants".to_string(),
            ));
        }

        // Check for duplicate participant IDs
        let mut seen_ids = HashSet::new();
        for participant in &self.active_participants {
            if !seen_ids.insert(&participant.participant_id) {
                return Err(ThresholdError::InvalidParameters(
                    format!("Duplicate participant ID: {:?}", participant.participant_id).into(),
                ));
            }
        }

        // Verify we have at least one leader
        let has_leader = self
            .active_participants
            .iter()
            .any(|p| matches!(p.role, ParticipantRole::Leader { .. }));

        if !has_leader {
            return Err(ThresholdError::InvalidParameters(
                "Group must have at least one leader".to_string(),
            ));
        }

        Ok(())
    }

    /// Add audit entry
    pub fn add_audit_entry(&mut self, entry: GroupAuditEntry) {
        self.audit_log.push(entry);

        // Keep audit log size reasonable (last 1000 entries)
        if self.audit_log.len() > 1000 {
            self.audit_log.drain(0..100);
        }
    }
}

/// Permission types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Permission {
    AddParticipant,
    RemoveParticipant,
    UpdateThreshold,
    Sign,
    Vote,
    CreateSubgroup,
    AssignRoles,
}

/// Role filter for queries
#[derive(Debug, Clone, PartialEq)]
pub enum RoleFilter {
    All,
    Leaders,
    Members,
    Observers,
}

/// Group hierarchy information
#[derive(Debug, Clone)]
pub struct GroupHierarchy {
    pub group_id: GroupId,
    pub parent: Option<GroupId>,
    pub name: String,
    pub threshold: u16,
    pub participants: u16,
    pub purpose: GroupPurpose,
}

/// Group statistics
#[derive(Debug, Clone)]
pub struct GroupStats {
    pub total_participants: u16,
    pub active_participants: u16,
    pub pending_participants: u16,
    pub suspended_participants: u16,
    pub leaders: u16,
    pub members: u16,
    pub observers: u16,
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
}

impl ThresholdGroup {
    /// Get group statistics
    pub fn get_stats(&self) -> GroupStats {
        let mut stats = GroupStats {
            total_participants: self.participants,
            active_participants: 0,
            pending_participants: self.pending_participants.len() as u16,
            suspended_participants: 0,
            leaders: 0,
            members: 0,
            observers: 0,
            total_operations: self.audit_log.len(),
            successful_operations: 0,
            failed_operations: 0,
        };

        for participant in &self.active_participants {
            match &participant.status {
                ParticipantStatus::Active => stats.active_participants += 1,
                ParticipantStatus::Suspended { .. } => stats.suspended_participants += 1,
                _ => {}
            }

            match &participant.role {
                ParticipantRole::Leader { .. } => stats.leaders += 1,
                ParticipantRole::Member { .. } => stats.members += 1,
                ParticipantRole::Observer => stats.observers += 1,
            }
        }

        for entry in &self.audit_log {
            match &entry.result {
                OperationResult::Success => stats.successful_operations += 1,
                OperationResult::Failed(_) => stats.failed_operations += 1,
                OperationResult::Pending => {}
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_group() -> ThresholdGroup {
        let participant1 = ParticipantInfo {
            participant_id: ParticipantId(1),
            public_key: MlDsaPublicKey(vec![1; 32]),
            frost_share_commitment: FrostCommitment(vec![1; 32]),
            role: ParticipantRole::Leader {
                permissions: LeaderPermissions::default(),
            },
            status: ParticipantStatus::Active,
            joined_at: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let participant2 = ParticipantInfo {
            participant_id: ParticipantId(2),
            public_key: MlDsaPublicKey(vec![2; 32]),
            frost_share_commitment: FrostCommitment(vec![2; 32]),
            role: ParticipantRole::Member {
                permissions: MemberPermissions::default(),
            },
            status: ParticipantStatus::Active,
            joined_at: SystemTime::now(),
            metadata: HashMap::new(),
        };

        ThresholdGroup {
            group_id: GroupId([0; 32]),
            threshold: 2,
            participants: 2,
            frost_group_key: FrostGroupPublicKey(vec![0; 32]),
            active_participants: vec![participant1, participant2],
            pending_participants: vec![],
            version: 1,
            metadata: GroupMetadata {
                name: "Test Group".to_string(),
                description: "Test group for unit tests".to_string(),
                purpose: GroupPurpose::MultiSig,
                parent_group: None,
                custom_data: HashMap::new(),
            },
            audit_log: vec![],
            created_at: SystemTime::now(),
            last_updated: SystemTime::now(),
        }
    }

    #[test]
    fn test_permission_checking() {
        let group = create_test_group();

        // Leader can add participants
        assert!(
            group
                .check_permission(&ParticipantId(1), Permission::AddParticipant)
                .is_ok()
        );

        // Member cannot add participants
        assert!(
            group
                .check_permission(&ParticipantId(2), Permission::AddParticipant)
                .is_err()
        );

        // Member can sign
        assert!(
            group
                .check_permission(&ParticipantId(2), Permission::Sign)
                .is_ok()
        );
    }

    #[test]
    fn test_group_validation() {
        let mut group = create_test_group();

        // Valid group
        assert!(group.validate().is_ok());

        // Invalid threshold
        group.threshold = 0;
        assert!(group.validate().is_err());

        group.threshold = 3; // More than participants
        assert!(group.validate().is_err());
    }

    #[test]
    fn test_participant_management() {
        let mut group = create_test_group();

        // Add pending participant
        let new_participant = ParticipantInfo {
            participant_id: ParticipantId(3),
            public_key: MlDsaPublicKey(vec![3; 32]),
            frost_share_commitment: FrostCommitment(vec![3; 32]),
            role: ParticipantRole::Member {
                permissions: MemberPermissions::default(),
            },
            status: ParticipantStatus::PendingJoin,
            joined_at: SystemTime::now(),
            metadata: HashMap::new(),
        };

        assert!(group.add_pending_participant(new_participant).is_ok());
        assert_eq!(group.pending_participants.len(), 1);

        // Cannot add duplicate
        let duplicate = ParticipantInfo {
            participant_id: ParticipantId(1),
            public_key: MlDsaPublicKey(vec![1; 32]),
            frost_share_commitment: FrostCommitment(vec![1; 32]),
            role: ParticipantRole::Member {
                permissions: MemberPermissions::default(),
            },
            status: ParticipantStatus::PendingJoin,
            joined_at: SystemTime::now(),
            metadata: HashMap::new(),
        };

        assert!(group.add_pending_participant(duplicate).is_err());
    }
}
