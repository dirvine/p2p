// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Community data synchronization system

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::identity::FourWordAddress;
use super::events::{Event, EventType, EventPublisher};

/// Community data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Community {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_by: FourWordAddress,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u64,
    pub members: HashSet<FourWordAddress>,
    pub permissions: CommunityPermissions,
    pub metadata: HashMap<String, String>,
}

/// Permission levels for community access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionLevel {
    /// Can read community data
    Read,
    /// Can add content and invite members
    Write,
    /// Can modify community settings and manage members
    Admin,
    /// Full control including deletion
    Owner,
}

/// Community permissions structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommunityPermissions {
    /// Member permissions
    pub member_permissions: HashMap<FourWordAddress, PermissionLevel>,
    /// Default permission for new members
    pub default_permission: PermissionLevel,
    /// Whether community is public or invite-only
    pub is_public: bool,
    /// Whether members can invite others
    pub members_can_invite: bool,
}

/// Community synchronization message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncMessage {
    /// Request list of communities from peer
    CommunityListRequest {
        requester: FourWordAddress,
        request_id: Uuid,
    },
    /// Response with list of available communities
    CommunityListResponse {
        communities: Vec<CommunitySummary>,
        request_id: Uuid,
    },
    /// Request specific community data
    CommunityDataRequest {
        community_id: Uuid,
        version: Option<u64>, // For incremental sync
        request_id: Uuid,
    },
    /// Full community data response
    CommunityDataResponse {
        community: Option<Community>,
        request_id: Uuid,
    },
    /// Incremental update
    CommunityUpdate {
        community_id: Uuid,
        update: CommunityUpdate,
        update_id: Uuid,
    },
    /// Conflict resolution request
    ConflictResolution {
        community_id: Uuid,
        conflicting_versions: Vec<Community>,
        resolution_id: Uuid,
    },
}

/// Community update for incremental sync
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommunityUpdate {
    pub update_type: UpdateType,
    pub timestamp: u64,
    pub updated_by: FourWordAddress,
    pub version: u64,
}

/// Types of community updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpdateType {
    /// Member added to community
    MemberAdded {
        member: FourWordAddress,
        permission: PermissionLevel,
    },
    /// Member removed from community
    MemberRemoved {
        member: FourWordAddress,
    },
    /// Member permission changed
    PermissionChanged {
        member: FourWordAddress,
        new_permission: PermissionLevel,
    },
    /// Community metadata updated
    MetadataUpdated {
        key: String,
        value: Option<String>, // None means deleted
    },
    /// Community description changed
    DescriptionChanged {
        new_description: String,
    },
    /// Community settings updated
    SettingsUpdated {
        field: String,
        value: String,
    },
}

/// Community summary for efficient listing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommunitySummary {
    pub id: Uuid,
    pub name: String,
    pub member_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u64,
    pub permission_level: PermissionLevel,
}

/// Sync status for tracking synchronization progress
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncStatus {
    pub community_id: Uuid,
    pub status: SyncState,
    pub last_sync_at: Option<u64>,
    pub peer_address: FourWordAddress,
    pub progress: SyncProgress,
    pub error_count: u32,
    pub last_error: Option<String>,
}

/// Synchronization states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncState {
    /// Not yet synchronized
    Pending,
    /// Currently synchronizing
    InProgress,
    /// Successfully synchronized
    Complete,
    /// Failed to synchronize
    Failed,
    /// Conflict detected, needs resolution
    Conflict,
    /// Synchronization paused
    Paused,
}

/// Synchronization progress tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncProgress {
    pub total_communities: u32,
    pub synced_communities: u32,
    pub current_community: Option<Uuid>,
    pub bytes_transferred: u64,
    pub estimated_remaining_time: Option<u64>,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolutionStrategy {
    /// Use the version with the latest timestamp
    LastWriterWins,
    /// Merge changes when possible
    AutoMerge,
    /// Require manual resolution
    Manual,
    /// Use version from specific peer (trusted peer)
    TrustedPeer(FourWordAddress),
}

/// Main community synchronization manager
#[derive(Debug)]
pub struct CommunitySyncManager {
    /// Storage for community data
    storage: Arc<Mutex<CommunityStorage>>,
    /// Event publisher for sync events
    event_publisher: Arc<EventPublisher>,
    /// Active sync operations
    active_syncs: Arc<Mutex<HashMap<FourWordAddress, SyncStatus>>>,
    /// Conflict resolution strategy
    resolution_strategy: ConflictResolutionStrategy,
    /// Sync configuration
    config: SyncConfig,
}

impl CommunitySyncManager {
    /// Create a new community sync manager
    pub fn new(
        storage_path: PathBuf,
        event_publisher: Arc<EventPublisher>,
    ) -> Result<Self> {
        let storage = Arc::new(Mutex::new(CommunityStorage::new(storage_path)));
        
        Ok(CommunitySyncManager {
            storage,
            event_publisher,
            active_syncs: Arc::new(Mutex::new(HashMap::new())),
            resolution_strategy: ConflictResolutionStrategy::LastWriterWins,
            config: SyncConfig::default(),
        })
    }
    
    /// Get storage reference
    pub fn get_storage(&self) -> Arc<Mutex<CommunityStorage>> {
        self.storage.clone()
    }
    
    /// Create a new community
    pub fn create_community(
        &self,
        name: String,
        description: String,
        creator: FourWordAddress,
    ) -> Result<Uuid> {
        let community = Community::new(name, description, creator);
        let community_id = community.id;
        
        {
            let mut storage = self.storage.lock().unwrap();
            storage.communities.insert(community_id, community.clone());
            
            // Update member index
            storage.member_communities
                .entry(community.created_by.clone())
                .or_insert_with(Vec::new)
                .push(community_id);
        }
        
        // Publish event
        let event = Event::new(
            EventType::SystemNotification {
                level: crate::communication::events::NotificationLevel::Info,
                message: format!("Created new community: {}", community.name),
            },
            "community_sync".to_string(),
        );
        let _ = self.event_publisher.publish(event);
        
        Ok(community_id)
    }
    
    /// Get community by ID
    pub fn get_community(&self, community_id: &Uuid) -> Option<Community> {
        let storage = self.storage.lock().unwrap();
        storage.communities.get(community_id).cloned()
    }
    
    /// Get all communities for a member
    pub fn get_communities_for_member(&self, member: &FourWordAddress) -> Vec<Community> {
        let storage = self.storage.lock().unwrap();
        storage.get_member_communities(member)
            .into_iter()
            .cloned()
            .collect()
    }
    
    /// List all communities
    pub fn list_communities(&self) -> Vec<CommunitySummary> {
        let storage = self.storage.lock().unwrap();
        let local_address = FourWordAddress::generate().unwrap(); // TODO: Get from identity manager
        
        storage.communities
            .values()
            .map(|community| community.summary(&local_address))
            .collect()
    }
    
    /// Add member to community
    pub fn add_member_to_community(
        &self,
        community_id: &Uuid,
        member: FourWordAddress,
        permission: PermissionLevel,
    ) -> Result<bool> {
        let mut storage = self.storage.lock().unwrap();
        
        if let Some(community) = storage.communities.get_mut(community_id) {
            community.add_member(member.clone(), permission)?;
            
            // Update member index
            storage.member_communities
                .entry(member)
                .or_insert_with(Vec::new)
                .push(*community_id);
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Update community metadata
    pub fn update_community_metadata(
        &self,
        community_id: &Uuid,
        key: String,
        value: String,
    ) -> Result<bool> {
        let mut storage = self.storage.lock().unwrap();
        
        if let Some(community) = storage.communities.get_mut(community_id) {
            community.update_metadata(key, value);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get sync status for a peer
    pub fn get_sync_status(&self, peer: &FourWordAddress) -> Option<SyncStatus> {
        let active_syncs = self.active_syncs.lock().unwrap();
        active_syncs.get(peer).cloned()
    }
    
    /// Update sync status for a peer
    pub fn update_sync_status(&self, peer: FourWordAddress, status: SyncStatus) {
        let mut active_syncs = self.active_syncs.lock().unwrap();
        active_syncs.insert(peer, status);
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &SyncConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: SyncConfig) {
        self.config = config;
    }
    
    /// Get resolution strategy
    pub fn get_resolution_strategy(&self) -> &ConflictResolutionStrategy {
        &self.resolution_strategy
    }
    
    /// Set resolution strategy
    pub fn set_resolution_strategy(&mut self, strategy: ConflictResolutionStrategy) {
        self.resolution_strategy = strategy;
    }
}

/// Community data storage
#[derive(Debug)]
pub struct CommunityStorage {
    /// Storage path
    pub storage_path: PathBuf,
    /// Communities by ID
    pub communities: HashMap<Uuid, Community>,
    /// Communities by member (for quick lookup)
    pub member_communities: HashMap<FourWordAddress, Vec<Uuid>>,
    /// Sync metadata
    pub sync_metadata: HashMap<Uuid, SyncMetadata>,
}

/// Sync metadata for each community
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub last_sync: Option<u64>,
    pub version: u64,
    pub checksum: String,
    pub sync_peers: HashSet<FourWordAddress>,
}

/// Synchronization configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum concurrent syncs
    pub max_concurrent_syncs: usize,
    /// Sync interval in seconds
    pub sync_interval: u64,
    /// Timeout for sync operations in seconds
    pub sync_timeout: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable automatic conflict resolution
    pub auto_resolve_conflicts: bool,
}

// Implementation will be added in the next phase
impl Community {
    /// Create a new community
    pub fn new(
        name: String,
        description: String,
        creator: FourWordAddress,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let mut permissions = CommunityPermissions {
            member_permissions: HashMap::new(),
            default_permission: PermissionLevel::Read,
            is_public: false,
            members_can_invite: false,
        };
        
        permissions.member_permissions.insert(creator.clone(), PermissionLevel::Owner);
        
        let mut members = HashSet::new();
        members.insert(creator.clone());

        Community {
            id: Uuid::new_v4(),
            name,
            description,
            created_by: creator,
            created_at: now,
            updated_at: now,
            version: 1,
            members,
            permissions,
            metadata: HashMap::new(),
        }
    }
    
    /// Check if a member has the required permission level
    pub fn has_permission(&self, member: &FourWordAddress, required: &PermissionLevel) -> bool {
        if let Some(member_permission) = self.permissions.member_permissions.get(member) {
            permission_level_value(member_permission) >= permission_level_value(required)
        } else {
            permission_level_value(&self.permissions.default_permission) >= permission_level_value(required)
        }
    }
    
    /// Add a member to the community
    pub fn add_member(&mut self, member: FourWordAddress, permission: PermissionLevel) -> Result<()> {
        self.members.insert(member.clone());
        self.permissions.member_permissions.insert(member, permission);
        self.update_version();
        Ok(())
    }
    
    /// Remove a member from the community
    pub fn remove_member(&mut self, member: &FourWordAddress) -> Result<bool> {
        let removed_from_members = self.members.remove(member);
        let removed_from_permissions = self.permissions.member_permissions.remove(member).is_some();
        
        if removed_from_members || removed_from_permissions {
            self.update_version();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Update community metadata
    pub fn update_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.update_version();
    }
    
    /// Update community description
    pub fn update_description(&mut self, description: String) {
        self.description = description;
        self.update_version();
    }
    
    /// Update version and timestamp
    fn update_version(&mut self) {
        self.version += 1;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// Create a summary of the community
    pub fn summary(&self, viewer: &FourWordAddress) -> CommunitySummary {
        let permission_level = self.permissions.member_permissions
            .get(viewer)
            .cloned()
            .unwrap_or(self.permissions.default_permission.clone());
            
        CommunitySummary {
            id: self.id,
            name: self.name.clone(),
            member_count: self.members.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            permission_level,
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        SyncConfig {
            max_concurrent_syncs: 5,
            sync_interval: 300, // 5 minutes
            sync_timeout: 60,   // 1 minute
            max_retries: 3,
            auto_resolve_conflicts: true,
        }
    }
}

// Helper function to convert permission level to numeric value for comparison
fn permission_level_value(level: &PermissionLevel) -> u8 {
    match level {
        PermissionLevel::Read => 0,
        PermissionLevel::Write => 1,
        PermissionLevel::Admin => 2,
        PermissionLevel::Owner => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_community_creation() {
        let creator = FourWordAddress::generate().unwrap();
        let community = Community::new(
            "Test Community".to_string(),
            "A test community".to_string(),
            creator.clone(),
        );
        
        assert_eq!(community.name, "Test Community");
        assert_eq!(community.description, "A test community");
        assert_eq!(community.created_by, creator);
        assert_eq!(community.version, 1);
        assert!(community.members.contains(&creator));
        assert_eq!(
            community.permissions.member_permissions.get(&creator),
            Some(&PermissionLevel::Owner)
        );
    }
    
    #[test]
    fn test_permission_checking() {
        let owner = FourWordAddress::generate().unwrap();
        let user = FourWordAddress::generate().unwrap();
        let community = Community::new(
            "Test".to_string(),
            "Test".to_string(),
            owner.clone(),
        );
        
        // Owner should have all permissions
        assert!(community.has_permission(&owner, &PermissionLevel::Read));
        assert!(community.has_permission(&owner, &PermissionLevel::Write));
        assert!(community.has_permission(&owner, &PermissionLevel::Admin));
        assert!(community.has_permission(&owner, &PermissionLevel::Owner));
        
        // Non-member should have default permission (Read)
        assert!(community.has_permission(&user, &PermissionLevel::Read));
        assert!(!community.has_permission(&user, &PermissionLevel::Write));
        assert!(!community.has_permission(&user, &PermissionLevel::Admin));
        assert!(!community.has_permission(&user, &PermissionLevel::Owner));
    }
    
    #[test]
    fn test_member_management() {
        let owner = FourWordAddress::generate().unwrap();
        let user = FourWordAddress::generate().unwrap();
        let mut community = Community::new(
            "Test".to_string(),
            "Test".to_string(),
            owner.clone(),
        );
        
        let initial_version = community.version;
        
        // Add member
        community.add_member(user.clone(), PermissionLevel::Write).unwrap();
        assert!(community.members.contains(&user));
        assert_eq!(
            community.permissions.member_permissions.get(&user),
            Some(&PermissionLevel::Write)
        );
        assert!(community.version > initial_version);
        
        // Remove member
        let version_after_add = community.version;
        let removed = community.remove_member(&user).unwrap();
        assert!(removed);
        assert!(!community.members.contains(&user));
        assert!(!community.permissions.member_permissions.contains_key(&user));
        assert!(community.version > version_after_add);
        
        // Remove non-existent member
        let removed_again = community.remove_member(&user).unwrap();
        assert!(!removed_again);
    }
    
    #[test]
    fn test_metadata_updates() {
        let owner = FourWordAddress::generate().unwrap();
        let mut community = Community::new(
            "Test".to_string(),
            "Test".to_string(),
            owner,
        );
        
        let initial_version = community.version;
        
        // Update metadata
        community.update_metadata("category".to_string(), "development".to_string());
        assert_eq!(community.metadata.get("category"), Some(&"development".to_string()));
        assert!(community.version > initial_version);
        
        // Update description
        let version_after_metadata = community.version;
        community.update_description("Updated description".to_string());
        assert_eq!(community.description, "Updated description");
        assert!(community.version > version_after_metadata);
    }
    
    #[test]
    fn test_community_summary() {
        let owner = FourWordAddress::generate().unwrap();
        let user = FourWordAddress::generate().unwrap();
        let mut community = Community::new(
            "Test Community".to_string(),
            "Test description".to_string(),
            owner.clone(),
        );
        
        community.add_member(user.clone(), PermissionLevel::Write).unwrap();
        
        let owner_summary = community.summary(&owner);
        assert_eq!(owner_summary.name, "Test Community");
        assert_eq!(owner_summary.member_count, 2);
        assert_eq!(owner_summary.permission_level, PermissionLevel::Owner);
        
        let user_summary = community.summary(&user);
        assert_eq!(user_summary.permission_level, PermissionLevel::Write);
        
        let stranger = FourWordAddress::generate().unwrap();
        let stranger_summary = community.summary(&stranger);
        assert_eq!(stranger_summary.permission_level, PermissionLevel::Read);
    }
    
    #[test]
    fn test_sync_message_serialization() {
        let requester = FourWordAddress::generate().unwrap();
        let request_id = Uuid::new_v4();
        
        let message = SyncMessage::CommunityListRequest {
            requester,
            request_id,
        };
        
        // Should serialize and deserialize without error
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: SyncMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(message, deserialized);
    }
    
    #[test]
    fn test_community_update_types() {
        let member = FourWordAddress::generate().unwrap();
        let updater = FourWordAddress::generate().unwrap();
        
        let update = CommunityUpdate {
            update_type: UpdateType::MemberAdded {
                member: member.clone(),
                permission: PermissionLevel::Write,
            },
            timestamp: 1234567890,
            updated_by: updater,
            version: 2,
        };
        
        // Should serialize properly
        let serialized = serde_json::to_string(&update).unwrap();
        let deserialized: CommunityUpdate = serde_json::from_str(&serialized).unwrap();
        assert_eq!(update, deserialized);
        
        if let UpdateType::MemberAdded { member: added_member, permission } = update.update_type {
            assert_eq!(added_member, member);
            assert_eq!(permission, PermissionLevel::Write);
        } else {
            panic!("Expected MemberAdded update type");
        }
    }
    
    #[test]
    fn test_sync_status_tracking() {
        let peer = FourWordAddress::generate().unwrap();
        let community_id = Uuid::new_v4();
        
        let status = SyncStatus {
            community_id,
            status: SyncState::InProgress,
            last_sync_at: Some(1234567890),
            peer_address: peer,
            progress: SyncProgress {
                total_communities: 10,
                synced_communities: 3,
                current_community: Some(community_id),
                bytes_transferred: 1024,
                estimated_remaining_time: Some(300),
            },
            error_count: 0,
            last_error: None,
        };
        
        assert_eq!(status.status, SyncState::InProgress);
        assert_eq!(status.progress.total_communities, 10);
        assert_eq!(status.progress.synced_communities, 3);
    }
    
    #[test]
    fn test_permission_level_ordering() {
        assert!(permission_level_value(&PermissionLevel::Owner) > permission_level_value(&PermissionLevel::Admin));
        assert!(permission_level_value(&PermissionLevel::Admin) > permission_level_value(&PermissionLevel::Write));
        assert!(permission_level_value(&PermissionLevel::Write) > permission_level_value(&PermissionLevel::Read));
    }
    
    #[test]
    fn test_conflict_resolution_strategies() {
        let peer = FourWordAddress::generate().unwrap();
        
        let strategies = vec![
            ConflictResolutionStrategy::LastWriterWins,
            ConflictResolutionStrategy::AutoMerge,
            ConflictResolutionStrategy::Manual,
            ConflictResolutionStrategy::TrustedPeer(peer),
        ];
        
        for strategy in strategies {
            // Should serialize/deserialize
            let serialized = serde_json::to_string(&strategy).unwrap();
            let _deserialized: ConflictResolutionStrategy = serde_json::from_str(&serialized).unwrap();
        }
    }
    
    #[test]
    fn test_sync_config_defaults() {
        let config = SyncConfig::default();
        assert_eq!(config.max_concurrent_syncs, 5);
        assert_eq!(config.sync_interval, 300);
        assert_eq!(config.sync_timeout, 60);
        assert_eq!(config.max_retries, 3);
        assert!(config.auto_resolve_conflicts);
    }
}