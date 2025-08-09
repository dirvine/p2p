// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Community synchronization protocol implementation

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use crate::identity::FourWordAddress;
use super::community_sync::*;
use super::events::{Event, EventType, EventPublisher};

/// Protocol handler for community synchronization
#[derive(Debug)]
pub struct SyncProtocolHandler {
    /// Local community storage
    storage: Arc<Mutex<CommunityStorage>>,
    /// Event publisher for protocol events
    event_publisher: Arc<EventPublisher>,
    /// Pending requests tracking
    pending_requests: Arc<Mutex<HashMap<Uuid, PendingRequest>>>,
    /// Protocol configuration
    config: ProtocolConfig,
}

/// Protocol configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Enable message compression
    pub enable_compression: bool,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
}

/// Pending request tracking
#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub request_id: Uuid,
    pub request_type: RequestType,
    pub peer: FourWordAddress,
    pub created_at: u64,
    pub timeout_at: u64,
    pub retry_count: u32,
}

/// Types of pending requests
#[derive(Debug, Clone, PartialEq)]
pub enum RequestType {
    CommunityList,
    CommunityData(Uuid),
    ConflictResolution(Uuid),
}

/// Protocol message envelope
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProtocolMessage {
    pub message_id: Uuid,
    pub sender: FourWordAddress,
    pub recipient: FourWordAddress,
    pub timestamp: u64,
    pub message_type: SyncMessage,
    pub signature: Option<Vec<u8>>, // For message integrity
}

/// Synchronization session manager
#[derive(Debug)]
pub struct SyncSession {
    pub session_id: Uuid,
    pub peer: FourWordAddress,
    pub state: SessionState,
    pub communities_to_sync: Vec<Uuid>,
    pub completed_communities: Vec<Uuid>,
    pub failed_communities: Vec<(Uuid, String)>,
    pub started_at: u64,
    pub last_activity: u64,
}

/// Session states
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Initializing,
    Negotiating,
    Syncing,
    Completed,
    Failed(String),
    Cancelled,
}

impl SyncProtocolHandler {
    /// Create a new protocol handler
    pub fn new(
        storage: Arc<Mutex<CommunityStorage>>,
        event_publisher: Arc<EventPublisher>,
    ) -> Self {
        SyncProtocolHandler {
            storage,
            event_publisher,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            config: ProtocolConfig::default(),
        }
    }

    /// Handle incoming protocol message
    pub async fn handle_message(&self, message: ProtocolMessage) -> Result<Option<ProtocolMessage>> {
        // Verify message integrity
        self.verify_message(&message)?;
        
        // Update last activity
        self.update_peer_activity(&message.sender);
        
        match &message.message_type {
            SyncMessage::CommunityListRequest { requester, request_id } => {
                self.handle_community_list_request(requester, *request_id).await
            }
            SyncMessage::CommunityListResponse { communities, request_id } => {
                self.handle_community_list_response(communities, *request_id).await;
                Ok(None)
            }
            SyncMessage::CommunityDataRequest { community_id, version, request_id } => {
                self.handle_community_data_request(*community_id, *version, *request_id).await
            }
            SyncMessage::CommunityDataResponse { community, request_id } => {
                self.handle_community_data_response(community.as_ref(), *request_id).await;
                Ok(None)
            }
            SyncMessage::CommunityUpdate { community_id, update, update_id } => {
                self.handle_community_update(*community_id, update, *update_id).await;
                Ok(None)
            }
            SyncMessage::ConflictResolution { community_id, conflicting_versions, resolution_id } => {
                self.handle_conflict_resolution(*community_id, conflicting_versions, *resolution_id).await
            }
        }
    }

    /// Send protocol message to peer
    pub async fn send_message(
        &self,
        recipient: FourWordAddress,
        message_type: SyncMessage,
    ) -> Result<Uuid> {
        let message_id = Uuid::new_v4();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        let message = ProtocolMessage {
            message_id,
            sender: self.get_local_address(),
            recipient: recipient.clone(),
            timestamp,
            message_type: message_type.clone(),
            signature: None, // TODO: Implement message signing
        };
        
        // Track pending request if applicable
        self.track_pending_request(&message)?;
        
        // Publish event for message sending
        let event = Event::new(
            EventType::SystemNotification {
                level: crate::communication::events::NotificationLevel::Info,
                message: format!("Sending sync message to {}", recipient),
            },
            "sync_protocol".to_string(),
        );
        self.event_publisher.publish(event)?;
        
        // TODO: Actually send message over network
        // This would integrate with the networking layer
        
        Ok(message_id)
    }

    /// Request community list from peer
    pub async fn request_community_list(&self, peer: FourWordAddress) -> Result<Uuid> {
        let request_id = Uuid::new_v4();
        let message_type = SyncMessage::CommunityListRequest {
            requester: self.get_local_address(),
            request_id,
        };
        
        self.send_message(peer, message_type).await?;
        Ok(request_id)
    }

    /// Request specific community data
    pub async fn request_community_data(
        &self,
        peer: FourWordAddress,
        community_id: Uuid,
        version: Option<u64>,
    ) -> Result<Uuid> {
        let request_id = Uuid::new_v4();
        let message_type = SyncMessage::CommunityDataRequest {
            community_id,
            version,
            request_id,
        };
        
        self.send_message(peer, message_type).await?;
        Ok(request_id)
    }

    /// Start synchronization session with peer
    pub async fn start_sync_session(&self, peer: FourWordAddress) -> Result<SyncSession> {
        let session_id = Uuid::new_v4();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // Request community list first
        self.request_community_list(peer.clone()).await?;
        
        let session = SyncSession {
            session_id,
            peer,
            state: SessionState::Initializing,
            communities_to_sync: Vec::new(),
            completed_communities: Vec::new(),
            failed_communities: Vec::new(),
            started_at: now,
            last_activity: now,
        };
        
        Ok(session)
    }

    /// Handle community list request
    async fn handle_community_list_request(
        &self,
        requester: &FourWordAddress,
        request_id: Uuid,
    ) -> Result<Option<ProtocolMessage>> {
        let storage = self.storage.lock().unwrap();
        let mut communities = Vec::new();
        
        // Get communities that requester has access to
        for (_, community) in &storage.communities {
            if community.has_permission(requester, &PermissionLevel::Read) {
                communities.push(community.summary(requester));
            }
        }
        
        drop(storage);
        
        let response = SyncMessage::CommunityListResponse {
            communities,
            request_id,
        };
        
        let message = ProtocolMessage {
            message_id: Uuid::new_v4(),
            sender: self.get_local_address(),
            recipient: requester.clone(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            message_type: response,
            signature: None,
        };
        
        Ok(Some(message))
    }

    /// Handle community list response
    async fn handle_community_list_response(
        &self,
        communities: &[CommunitySummary],
        request_id: Uuid,
    ) {
        // Remove from pending requests
        let mut pending = self.pending_requests.lock().unwrap();
        pending.remove(&request_id);
        drop(pending);
        
        // Publish event for received community list
        let event = Event::new(
            EventType::SystemNotification {
                level: crate::communication::events::NotificationLevel::Info,
                message: format!("Received {} communities from peer", communities.len()),
            },
            "sync_protocol".to_string(),
        );
        let _ = self.event_publisher.publish(event);
    }

    /// Handle community data request
    async fn handle_community_data_request(
        &self,
        community_id: Uuid,
        version: Option<u64>,
        request_id: Uuid,
    ) -> Result<Option<ProtocolMessage>> {
        let storage = self.storage.lock().unwrap();
        let community = storage.communities.get(&community_id).cloned();
        drop(storage);
        
        // Check if we need to send full data or just updates
        let community_data = match (community, version) {
            (Some(community), Some(requested_version)) if community.version == requested_version => {
                // Already up to date
                None
            }
            (Some(community), _) => {
                // Send full community data
                Some(community)
            }
            (None, _) => {
                // Community not found
                None
            }
        };
        
        let response = SyncMessage::CommunityDataResponse {
            community: community_data,
            request_id,
        };
        
        let message = ProtocolMessage {
            message_id: Uuid::new_v4(),
            sender: self.get_local_address(),
            recipient: self.get_local_address(), // TODO: Get actual recipient
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            message_type: response,
            signature: None,
        };
        
        Ok(Some(message))
    }

    /// Handle community data response
    async fn handle_community_data_response(
        &self,
        community: Option<&Community>,
        request_id: Uuid,
    ) {
        // Remove from pending requests
        let mut pending = self.pending_requests.lock().unwrap();
        pending.remove(&request_id);
        drop(pending);
        
        if let Some(community) = community {
            // Store or update community
            let mut storage = self.storage.lock().unwrap();
            storage.communities.insert(community.id, community.clone());
            
            // Update member index
            for member in &community.members {
                storage.member_communities
                    .entry(member.clone())
                    .or_insert_with(Vec::new)
                    .push(community.id);
            }
            
            drop(storage);
            
            // Publish event for updated community
            let event = Event::new(
                EventType::SystemNotification {
                    level: crate::communication::events::NotificationLevel::Info,
                    message: format!("Updated community: {}", community.name),
                },
                "sync_protocol".to_string(),
            );
            let _ = self.event_publisher.publish(event);
        }
    }

    /// Handle community update
    async fn handle_community_update(
        &self,
        community_id: Uuid,
        update: &CommunityUpdate,
        _update_id: Uuid,
    ) {
        let mut storage = self.storage.lock().unwrap();
        
        if let Some(community) = storage.communities.get_mut(&community_id) {
            // Apply update if version is newer
            if update.version > community.version {
                self.apply_community_update(community, update);
                
                // Publish event for community update
                let event = Event::new(
                    EventType::SystemNotification {
                        level: crate::communication::events::NotificationLevel::Info,
                        message: format!("Applied update to community: {}", community.name),
                    },
                    "sync_protocol".to_string(),
                );
                let _ = self.event_publisher.publish(event);
            }
        }
    }

    /// Handle conflict resolution
    async fn handle_conflict_resolution(
        &self,
        community_id: Uuid,
        conflicting_versions: &[Community],
        _resolution_id: Uuid,
    ) -> Result<Option<ProtocolMessage>> {
        // Implement conflict resolution logic based on strategy
        let resolved_community = self.resolve_conflict(conflicting_versions)?;
        
        // Update local storage
        let mut storage = self.storage.lock().unwrap();
        storage.communities.insert(community_id, resolved_community.clone());
        drop(storage);
        
        // Publish event for conflict resolution
        let event = Event::new(
            EventType::SystemNotification {
                level: crate::communication::events::NotificationLevel::Info,
                message: format!("Resolved conflict for community: {}", resolved_community.name),
            },
            "sync_protocol".to_string(),
        );
        let _ = self.event_publisher.publish(event);
        
        Ok(None)
    }

    /// Apply community update to existing community
    fn apply_community_update(&self, community: &mut Community, update: &CommunityUpdate) {
        match &update.update_type {
            UpdateType::MemberAdded { member, permission } => {
                community.members.insert(member.clone());
                community.permissions.member_permissions.insert(member.clone(), permission.clone());
            }
            UpdateType::MemberRemoved { member } => {
                community.members.remove(member);
                community.permissions.member_permissions.remove(member);
            }
            UpdateType::PermissionChanged { member, new_permission } => {
                if community.members.contains(member) {
                    community.permissions.member_permissions.insert(member.clone(), new_permission.clone());
                }
            }
            UpdateType::MetadataUpdated { key, value } => {
                if let Some(value) = value {
                    community.metadata.insert(key.clone(), value.clone());
                } else {
                    community.metadata.remove(key);
                }
            }
            UpdateType::DescriptionChanged { new_description } => {
                community.description = new_description.clone();
            }
            UpdateType::SettingsUpdated { field: _, value: _ } => {
                // Handle settings updates
            }
        }
        
        community.version = update.version;
        community.updated_at = update.timestamp;
    }

    /// Resolve conflicts between community versions
    fn resolve_conflict(&self, conflicting_versions: &[Community]) -> Result<Community> {
        if conflicting_versions.is_empty() {
            return Err(anyhow!("No conflicting versions provided"));
        }
        
        // For now, use last writer wins based on updated_at
        let resolved = conflicting_versions
            .iter()
            .max_by_key(|c| c.updated_at)
            .unwrap()
            .clone();
        
        Ok(resolved)
    }

    /// Track pending request
    fn track_pending_request(&self, message: &ProtocolMessage) -> Result<()> {
        let (request_type, request_id) = match &message.message_type {
            SyncMessage::CommunityListRequest { request_id, .. } => (Some(RequestType::CommunityList), *request_id),
            SyncMessage::CommunityDataRequest { community_id, request_id, .. } => (Some(RequestType::CommunityData(*community_id)), *request_id),
            SyncMessage::ConflictResolution { community_id, resolution_id, .. } => (Some(RequestType::ConflictResolution(*community_id)), *resolution_id),
            _ => (None, Uuid::new_v4()),
        };
        
        if let Some(request_type) = request_type {
            let pending_request = PendingRequest {
                request_id,
                request_type,
                peer: message.recipient.clone(),
                created_at: message.timestamp,
                timeout_at: message.timestamp + self.config.request_timeout,
                retry_count: 0,
            };
            
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(request_id, pending_request);
        }
        
        Ok(())
    }

    /// Verify message integrity
    fn verify_message(&self, _message: &ProtocolMessage) -> Result<()> {
        // TODO: Implement message signature verification
        Ok(())
    }

    /// Update peer activity timestamp
    fn update_peer_activity(&self, _peer: &FourWordAddress) {
        // TODO: Track peer activity for connection management
    }

    /// Get local address (placeholder)
    fn get_local_address(&self) -> FourWordAddress {
        // TODO: Get actual local address from identity manager
        FourWordAddress::generate().unwrap()
    }

    /// Clean up expired requests
    pub fn cleanup_expired_requests(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut pending = self.pending_requests.lock().unwrap();
        
        pending.retain(|_, request| request.timeout_at > now);
    }
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        ProtocolConfig {
            request_timeout: 30,
            max_message_size: 1024 * 1024, // 1MB
            enable_compression: true,
            max_concurrent_requests: 10,
        }
    }
}

impl CommunityStorage {
    /// Create new community storage
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        CommunityStorage {
            storage_path,
            communities: HashMap::new(),
            member_communities: HashMap::new(),
            sync_metadata: HashMap::new(),
        }
    }
    
    /// Get communities for a member
    pub fn get_member_communities(&self, member: &FourWordAddress) -> Vec<&Community> {
        if let Some(community_ids) = self.member_communities.get(member) {
            community_ids
                .iter()
                .filter_map(|id| self.communities.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Update sync metadata
    pub fn update_sync_metadata(&mut self, community_id: Uuid, metadata: SyncMetadata) {
        self.sync_metadata.insert(community_id, metadata);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

    fn create_test_handler() -> (SyncProtocolHandler, Arc<Mutex<CommunityStorage>>) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Mutex::new(CommunityStorage::new(temp_dir.path().to_path_buf())));
        let event_publisher = Arc::new(crate::communication::events::EventPublisher::new());
        let handler = SyncProtocolHandler::new(storage.clone(), event_publisher);
        (handler, storage)
    }

    #[tokio::test]
    async fn test_protocol_handler_creation() {
        let (handler, _) = create_test_handler();
        assert_eq!(handler.config.request_timeout, 30);
        assert_eq!(handler.config.max_message_size, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_community_list_request() {
        let (handler, storage) = create_test_handler();
        let peer = FourWordAddress::generate().unwrap();
        
        // Add a test community
        let owner = FourWordAddress::generate().unwrap();
        let community = Community::new("Test".to_string(), "Test community".to_string(), owner.clone());
        {
            let mut storage = storage.lock().unwrap();
            storage.communities.insert(community.id, community.clone());
        }
        
        // Request community list
        let request_id = handler.request_community_list(peer).await.unwrap();
        assert!(!request_id.is_nil());
        
        // Check that request is tracked
        let pending = handler.pending_requests.lock().unwrap();
        assert!(pending.contains_key(&request_id));
    }

    #[tokio::test]
    async fn test_community_data_request() {
        let (handler, _) = create_test_handler();
        let peer = FourWordAddress::generate().unwrap();
        let community_id = Uuid::new_v4();
        
        let request_id = handler.request_community_data(peer, community_id, Some(1)).await.unwrap();
        assert!(!request_id.is_nil());
        
        // Check that request is tracked
        let pending = handler.pending_requests.lock().unwrap();
        if let Some(request) = pending.get(&request_id) {
            assert_eq!(request.request_type, RequestType::CommunityData(community_id));
        }
    }

    #[tokio::test]
    async fn test_protocol_message_serialization() {
        let sender = FourWordAddress::generate().unwrap();
        let recipient = FourWordAddress::generate().unwrap();
        let request_id = Uuid::new_v4();
        
        let message = ProtocolMessage {
            message_id: Uuid::new_v4(),
            sender: sender.clone(),
            recipient: recipient.clone(),
            timestamp: 1234567890,
            message_type: SyncMessage::CommunityListRequest {
                requester: sender,
                request_id,
            },
            signature: None,
        };
        
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: ProtocolMessage = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(message.message_id, deserialized.message_id);
        assert_eq!(message.sender, deserialized.sender);
        assert_eq!(message.recipient, deserialized.recipient);
    }

    #[tokio::test]
    async fn test_sync_session_creation() {
        let (handler, _) = create_test_handler();
        let peer = FourWordAddress::generate().unwrap();
        
        let session = handler.start_sync_session(peer.clone()).await.unwrap();
        
        assert_eq!(session.peer, peer);
        assert_eq!(session.state, SessionState::Initializing);
        assert!(session.communities_to_sync.is_empty());
        assert!(session.completed_communities.is_empty());
    }

    #[tokio::test]
    async fn test_community_update_application() {
        let (handler, storage) = create_test_handler();
        let owner = FourWordAddress::generate().unwrap();
        let new_member = FourWordAddress::generate().unwrap();
        
        // Create and store a community
        let mut community = Community::new("Test".to_string(), "Test".to_string(), owner.clone());
        let community_id = community.id;
        
        {
            let mut storage = storage.lock().unwrap();
            storage.communities.insert(community_id, community.clone());
        }
        
        // Create an update
        let update = CommunityUpdate {
            update_type: UpdateType::MemberAdded {
                member: new_member.clone(),
                permission: PermissionLevel::Write,
            },
            timestamp: 1234567890,
            updated_by: owner,
            version: 2,
        };
        
        // Apply update
        handler.apply_community_update(&mut community, &update);
        
        // Verify update was applied
        assert!(community.members.contains(&new_member));
        assert_eq!(
            community.permissions.member_permissions.get(&new_member),
            Some(&PermissionLevel::Write)
        );
        assert_eq!(community.version, 2);
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let (handler, _) = create_test_handler();
        let owner = FourWordAddress::generate().unwrap();
        
        // Create conflicting versions
        let mut version1 = Community::new("Test".to_string(), "Version 1".to_string(), owner.clone());
        version1.updated_at = 1000;
        
        let mut version2 = Community::new("Test".to_string(), "Version 2".to_string(), owner.clone());
        version2.id = version1.id; // Same community
        version2.updated_at = 2000; // Newer timestamp
        
        let conflicting_versions = vec![version1, version2.clone()];
        let resolved = handler.resolve_conflict(&conflicting_versions).unwrap();
        
        // Should resolve to the newer version
        assert_eq!(resolved.description, "Version 2");
        assert_eq!(resolved.updated_at, 2000);
    }

    #[tokio::test]
    async fn test_cleanup_expired_requests() {
        let (handler, _) = create_test_handler();
        
        // Add an expired request
        let expired_request = PendingRequest {
            request_id: Uuid::new_v4(),
            request_type: RequestType::CommunityList,
            peer: FourWordAddress::generate().unwrap(),
            created_at: 1000,
            timeout_at: 2000, // Already expired
            retry_count: 0,
        };
        
        {
            let mut pending = handler.pending_requests.lock().unwrap();
            pending.insert(expired_request.request_id, expired_request);
        }
        
        // Add a valid request
        let valid_request = PendingRequest {
            request_id: Uuid::new_v4(),
            request_type: RequestType::CommunityList,
            peer: FourWordAddress::generate().unwrap(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            timeout_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 60,
            retry_count: 0,
        };
        
        {
            let mut pending = handler.pending_requests.lock().unwrap();
            pending.insert(valid_request.request_id, valid_request.clone());
        }
        
        // Cleanup expired requests
        handler.cleanup_expired_requests();
        
        // Check that only valid request remains
        let pending = handler.pending_requests.lock().unwrap();
        assert_eq!(pending.len(), 1);
        assert!(pending.contains_key(&valid_request.request_id));
    }

    #[test]
    fn test_community_storage_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = CommunityStorage::new(temp_dir.path().to_path_buf());
        
        let owner = FourWordAddress::generate().unwrap();
        let community = Community::new("Test".to_string(), "Test".to_string(), owner.clone());
        let community_id = community.id;
        
        // Store community
        storage.communities.insert(community_id, community);
        storage.member_communities.insert(owner.clone(), vec![community_id]);
        
        // Get member communities
        let member_communities = storage.get_member_communities(&owner);
        assert_eq!(member_communities.len(), 1);
        assert_eq!(member_communities[0].id, community_id);
        
        // Update sync metadata
        let metadata = SyncMetadata {
            last_sync: Some(1234567890),
            version: 1,
            checksum: "abc123".to_string(),
            sync_peers: HashSet::new(),
        };
        storage.update_sync_metadata(community_id, metadata.clone());
        
        assert_eq!(storage.sync_metadata.get(&community_id).unwrap().version, 1);
    }

    #[test]
    fn test_session_states() {
        let session_states = vec![
            SessionState::Initializing,
            SessionState::Negotiating,
            SessionState::Syncing,
            SessionState::Completed,
            SessionState::Failed("Error".to_string()),
            SessionState::Cancelled,
        ];
        
        for state in session_states {
            // Should be constructible and comparable
            match state {
                SessionState::Failed(msg) => assert_eq!(msg, "Error"),
                _ => {} // Other states don't have data
            }
        }
    }

    #[test]
    fn test_protocol_config_defaults() {
        let config = ProtocolConfig::default();
        assert_eq!(config.request_timeout, 30);
        assert_eq!(config.max_message_size, 1024 * 1024);
        assert!(config.enable_compression);
        assert_eq!(config.max_concurrent_requests, 10);
    }
}