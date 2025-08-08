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

//! Enhanced identity system with quantum threshold cryptography integration

use crate::identity::manager::{IdentityManager, UserIdentity};
use crate::quantum_crypto::{CryptoCapabilities, QuantumPeerIdentity, generate_keypair};
use crate::threshold::{ParticipantInfo, ParticipantRole, ThresholdGroupManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use thiserror::Error;

/// Enhanced identity errors
#[derive(Debug, Error)]
pub enum EnhancedIdentityError {
    #[error("Quantum crypto error: {0}")]
    QuantumError(#[from] crate::quantum_crypto::QuantumCryptoError),

    #[error("Threshold error: {0}")]
    ThresholdError(#[from] crate::threshold::ThresholdError),

    #[error("Organization error: {0}")]
    OrganizationError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("System time error: {0}")]
    SystemTime(String),
}

type Result<T> = std::result::Result<T, EnhancedIdentityError>;

/// Enhanced identity with quantum and threshold capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedIdentity {
    /// Base user identity (backward compatible)
    pub base_identity: UserIdentity,

    /// Quantum-resistant peer identity
    pub quantum_identity: QuantumPeerIdentity,

    /// Threshold groups this user belongs to
    pub threshold_groups: Vec<GroupMembership>,

    /// Organizations this user belongs to
    pub organizations: Vec<OrganizationMembership>,

    /// Device registry for multi-device support
    pub devices: DeviceRegistry,

    /// Last sync timestamp
    pub last_sync: SystemTime,
}

/// Group membership information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMembership {
    pub group_id: crate::quantum_crypto::types::GroupId,
    pub role: ParticipantRole,
    pub joined_at: SystemTime,
    pub permissions: Vec<Permission>,
}

/// Organization membership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMembership {
    pub org_id: OrganizationId,
    pub department: Option<DepartmentId>,
    pub team: Option<TeamId>,
    pub role: OrganizationRole,
    pub joined_at: SystemTime,
}

/// Organization identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrganizationId(pub String);

/// Department identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DepartmentId(pub String);

/// Team identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamId(pub String);

/// Organization role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrganizationRole {
    Owner,
    Admin,
    Manager,
    Member,
    Guest,
}

/// Permission types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    // Chat permissions
    ChatCreateChannel,
    ChatDeleteChannel,
    ChatInviteMembers,
    ChatModerate,
    ChatVoiceVideo,

    // Discuss permissions
    DiscussCreateCategory,
    DiscussModerate,
    DiscussPin,
    DiscussWikiEdit,

    // Project permissions
    ProjectCreate,
    ProjectDelete,
    ProjectManageMembers,
    ProjectUploadFiles,
    ProjectApprove,

    // Admin permissions
    AdminManageRoles,
    AdminViewAudit,
    AdminManageOrg,
}

/// Device registry for multi-device support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistry {
    /// Primary device ID (first device)
    pub primary_device_id: DeviceId,

    /// All registered devices
    pub devices: HashMap<DeviceId, DeviceInfo>,

    /// Device-specific keys (encrypted with master key)
    pub device_keys: HashMap<DeviceId, EncryptedDeviceKey>,
}

/// Device identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub device_type: DeviceType,
    pub last_seen: SystemTime,
    pub added_at: SystemTime,
    pub public_key: Vec<u8>,
}

/// Device type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Web,
}

/// Encrypted device key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedDeviceKey {
    pub encrypted_key: Vec<u8>,
    pub nonce: Vec<u8>,
}

/// Organization structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: String,
    pub root_group: crate::quantum_crypto::types::GroupId,
    pub departments: Vec<Department>,
    pub created_at: SystemTime,
    pub settings: OrganizationSettings,
}

/// Department structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub id: DepartmentId,
    pub name: String,
    pub description: String,
    pub manager_group: crate::quantum_crypto::types::GroupId,
    pub teams: Vec<Team>,
    pub parent_org: OrganizationId,
}

/// Team structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub description: String,
    pub lead_group: crate::quantum_crypto::types::GroupId,
    pub member_group: crate::quantum_crypto::types::GroupId,
    pub parent_dept: DepartmentId,
}

/// Organization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    pub default_chat_encryption: bool,
    pub require_2fa: bool,
    pub session_timeout: std::time::Duration,
    pub allowed_domains: Vec<String>,
    pub features: OrganizationFeatures,
}

/// Organization features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationFeatures {
    pub chat_enabled: bool,
    pub discuss_enabled: bool,
    pub projects_enabled: bool,
    pub voice_video_enabled: bool,
    pub ai_enabled: bool,
}

impl Default for OrganizationFeatures {
    fn default() -> Self {
        Self {
            chat_enabled: true,
            discuss_enabled: true,
            projects_enabled: true,
            voice_video_enabled: true,
            ai_enabled: true,
        }
    }
}

/// Enhanced identity manager
pub struct EnhancedIdentityManager {
    /// Base identity manager
    pub base_manager: IdentityManager,

    /// Threshold group manager
    pub group_manager: ThresholdGroupManager,

    /// Organization registry
    pub organizations: HashMap<OrganizationId, Organization>,
}

impl EnhancedIdentityManager {
    /// Create new enhanced identity manager
    pub fn new(base_manager: IdentityManager) -> Self {
        // Create a temporary quantum identity for the group manager
        let temp_quantum_identity = QuantumPeerIdentity {
            peer_id: crate::quantum_crypto::types::PeerId(vec![0; 32]),
            ml_dsa_public_key: crate::quantum_crypto::types::MlDsaPublicKey(vec![0; 32]),
            ml_kem_public_key: crate::quantum_crypto::types::MlKemPublicKey(vec![0; 32]),
            frost_public_key: None,
            legacy_key: None,
            capabilities: CryptoCapabilities::default(),
            created_at: SystemTime::now(),
        };

        Self {
            base_manager,
            group_manager: ThresholdGroupManager::new(temp_quantum_identity),
            organizations: HashMap::new(),
        }
    }

    /// Create enhanced identity from base identity
    pub async fn create_enhanced_identity(
        &mut self,
        base_identity: UserIdentity,
        device_name: String,
        device_type: DeviceType,
    ) -> Result<EnhancedIdentity> {
        // Generate quantum-resistant keys
        let capabilities = CryptoCapabilities::default();
        let keypair = generate_keypair(&capabilities).await?;

        // Create quantum peer identity
        let peer_id =
            crate::quantum_crypto::types::PeerId(base_identity.user_id.as_bytes().to_vec());

        let quantum_identity = QuantumPeerIdentity {
            peer_id,
            ml_dsa_public_key: keypair
                .public
                .ml_dsa
                .clone()
                .unwrap_or_else(|| crate::quantum_crypto::types::MlDsaPublicKey(vec![0u8; 32])),
            ml_kem_public_key: keypair
                .public
                .ml_kem
                .clone()
                .unwrap_or_else(|| crate::quantum_crypto::types::MlKemPublicKey(vec![0u8; 32])),
            frost_public_key: keypair.public.frost.clone(),
            legacy_key: keypair.public.ed25519.clone(),
            capabilities,
            created_at: SystemTime::now(),
        };

        // Update group manager with the new quantum identity
        self.group_manager.local_identity = quantum_identity.clone();

        // Create device registry
        let device_id = DeviceId(format!(
            "{}-{}",
            base_identity.user_id,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| EnhancedIdentityError::SystemTime(
                    format!("System time error: {}", e).into()
                ))?
                .as_secs()
        ));

        let device_info = DeviceInfo {
            name: device_name,
            device_type,
            last_seen: SystemTime::now(),
            added_at: SystemTime::now(),
            public_key: keypair
                .public
                .ed25519
                .map(|k| k.0.to_vec())
                .unwrap_or_else(|| vec![0u8; 32]),
        };

        let mut devices = HashMap::new();
        devices.insert(device_id.clone(), device_info);

        let device_registry = DeviceRegistry {
            primary_device_id: device_id,
            devices,
            device_keys: HashMap::new(),
        };

        Ok(EnhancedIdentity {
            base_identity,
            quantum_identity,
            threshold_groups: Vec::new(),
            organizations: Vec::new(),
            devices: device_registry,
            last_sync: SystemTime::now(),
        })
    }

    /// Create new organization
    pub async fn create_organization(
        &mut self,
        name: String,
        description: String,
        owner_identity: &EnhancedIdentity,
    ) -> Result<Organization> {
        // Create root threshold group for organization
        let owner_info = ParticipantInfo {
            participant_id: crate::quantum_crypto::types::ParticipantId(0),
            public_key: owner_identity.quantum_identity.ml_dsa_public_key.clone(),
            frost_share_commitment: crate::quantum_crypto::types::FrostCommitment(vec![0; 32]),
            role: ParticipantRole::Leader {
                permissions: crate::threshold::LeaderPermissions::default(),
            },
            status: crate::threshold::ParticipantStatus::Active,
            joined_at: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let group_config = crate::threshold::GroupConfig {
            threshold: 1,
            participants: vec![owner_info],
            metadata: crate::threshold::GroupMetadata {
                name: format!("{name} Root Group").into(),
                description: "Organization root authority".to_string(),
                purpose: crate::threshold::GroupPurpose::Governance,
                parent_group: None,
                custom_data: HashMap::new(),
            },
        };

        let root_group = self.group_manager.create_group(group_config).await?;

        let org = Organization {
            id: OrganizationId(uuid::Uuid::new_v4().to_string()),
            name,
            description,
            root_group: root_group.group_id,
            departments: Vec::new(),
            created_at: SystemTime::now(),
            settings: OrganizationSettings {
                default_chat_encryption: true,
                require_2fa: false,
                session_timeout: std::time::Duration::from_secs(86400), // 24 hours
                allowed_domains: Vec::new(),
                features: OrganizationFeatures::default(),
            },
        };

        self.organizations.insert(org.id.clone(), org.clone());

        Ok(org)
    }

    /// Add user to organization
    pub async fn add_user_to_organization(
        &mut self,
        user: &mut EnhancedIdentity,
        org_id: &OrganizationId,
        role: OrganizationRole,
    ) -> Result<()> {
        let _org = self.organizations.get(org_id).ok_or_else(|| {
            EnhancedIdentityError::OrganizationError("Organization not found".to_string())
        })?;

        let membership = OrganizationMembership {
            org_id: org_id.clone(),
            department: None,
            team: None,
            role,
            joined_at: SystemTime::now(),
        };

        user.organizations.push(membership);

        Ok(())
    }

    /// Check permission for user in organization
    pub fn check_permission(
        &self,
        user: &EnhancedIdentity,
        org_id: &OrganizationId,
        permission: Permission,
    ) -> Result<()> {
        let membership = user
            .organizations
            .iter()
            .find(|m| &m.org_id == org_id)
            .ok_or_else(|| {
                EnhancedIdentityError::PermissionDenied("User not in organization".to_string())
            })?;

        // Check role-based permissions
        let allowed = match (&membership.role, &permission) {
            (OrganizationRole::Owner, _) => true,
            (OrganizationRole::Admin, Permission::AdminManageOrg) => false,
            (OrganizationRole::Admin, _) => true,
            (OrganizationRole::Manager, Permission::AdminManageRoles) => false,
            (OrganizationRole::Manager, Permission::AdminViewAudit) => false,
            (OrganizationRole::Manager, Permission::AdminManageOrg) => false,
            (OrganizationRole::Manager, _) => true,
            (OrganizationRole::Member, perm) => matches!(
                perm,
                Permission::ChatCreateChannel
                    | Permission::ChatInviteMembers
                    | Permission::ChatVoiceVideo
                    | Permission::DiscussWikiEdit
                    | Permission::ProjectUploadFiles
            ),
            (OrganizationRole::Guest, perm) => matches!(perm, Permission::ChatVoiceVideo),
        };

        if allowed {
            Ok(())
        } else {
            Err(EnhancedIdentityError::PermissionDenied(
                format!(
                    "Permission {:?} denied for role {:?}",
                    permission, membership.role
                )
                .into(),
            ))
        }
    }
}
