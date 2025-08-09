// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Communication module for P2P messaging and notifications

pub mod message;
pub mod delivery;
pub mod messaging;
pub mod events;
pub mod notifications;
pub mod community_sync;
pub mod sync_protocol;
pub mod conflict_resolution;
pub mod file_metadata;
pub mod file_transfer;
pub mod transfer_protocol;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;
use crate::identity::{FourWordAddress, EnhancedIdentityManager};

// Re-export main components
pub use message::{Message, MessageEnvelope, MessageType, StoredMessage, DeliveryStatus};
pub use delivery::{MessageDelivery, DeliveryResult, QueueStats};
pub use messaging::{SecureMessaging, MessageStorage, ConversationSummary};
pub use events::{Event, EventType, EventHandler, EventPublisher, ConnectionQuality, NetworkStatus, NotificationLevel};
pub use notifications::{Notification as SystemNotification, NotificationAction, ActionType, NotificationConfig, NotificationManager, NotificationHandler};
pub use community_sync::{Community, CommunitySyncManager, CommunityStorage, CommunityPermissions, PermissionLevel, SyncStatus, SyncState, ConflictResolutionStrategy, CommunitySummary};
pub use sync_protocol::{SyncProtocolHandler, ProtocolMessage, SyncSession, SessionState};
pub use conflict_resolution::{ConflictResolver, ConflictAnalysis, MergeResult};
pub use file_metadata::{FileMetadata, ChunkMetadata, FilePermissions, FileChunker};
pub use file_transfer::{TransferSession, TransferStatus, TransferDirection, TransferProgress, FileTransferManager};
pub use transfer_protocol::{TransferMessage, ProtocolSession, TransferProtocolHandler, TransferProtocolListener};

/// Direct message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub id: Uuid,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: u64,
    pub encrypted: bool,
}

/// Notification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub message_type: NotificationType,
    pub content: String,
    pub timestamp: u64,
    pub read: bool,
}

/// Types of notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Message,
    FileShare,
    CommunityUpdate,
    System,
}

/// Communication manager for handling P2P messaging
pub struct CommunicationManager {
    #[cfg(feature = "network")]
    node: Option<saorsa_core::P2PNode>,
    /// Secure messaging system
    secure_messaging: Option<SecureMessaging>,
    /// Event publisher for notifications
    event_publisher: Option<std::sync::Arc<EventPublisher>>,
    /// Notification manager
    notification_manager: Option<std::sync::Arc<NotificationManager>>,
    /// Community synchronization manager
    community_sync_manager: Option<std::sync::Arc<std::sync::Mutex<CommunitySyncManager>>>,
    /// Synchronization protocol handler
    sync_protocol_handler: Option<std::sync::Arc<SyncProtocolHandler>>,
    /// Conflict resolver
    conflict_resolver: Option<std::sync::Arc<std::sync::Mutex<ConflictResolver>>>,
}

// Manual Debug implementation since P2PNode doesn't implement Debug
impl std::fmt::Debug for CommunicationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommunicationManager")
            .field("has_node", &self.has_node())
            .finish()
    }
}

impl CommunicationManager {
    /// Check if node is available
    fn has_node(&self) -> bool {
        #[cfg(feature = "network")]
        {
            self.node.is_some()
        }
        #[cfg(not(feature = "network"))]
        {
            false
        }
    }

    /// Create a new communication manager
    pub fn new() -> Self {
        CommunicationManager {
            #[cfg(feature = "network")]
            node: None,
            secure_messaging: None,
            event_publisher: None,
            notification_manager: None,
            community_sync_manager: None,
            sync_protocol_handler: None,
            conflict_resolver: None,
        }
    }
    
    /// Initialize secure messaging with storage path and identity manager
    pub fn initialize_messaging(
        &mut self, 
        storage_path: PathBuf, 
        identity_manager: EnhancedIdentityManager
    ) -> Result<()> {
        let messaging = SecureMessaging::new(storage_path.clone())
            .with_identity_manager(identity_manager);
        self.secure_messaging = Some(messaging);
        
        // Initialize notification system
        let event_publisher = std::sync::Arc::new(EventPublisher::new());
        let notification_storage_path = storage_path.join("notifications");
        let notification_manager = std::sync::Arc::new(NotificationManager::new(
            notification_storage_path,
            event_publisher.clone(),
        ));
        
        notification_manager.initialize()?;
        
        // Initialize community synchronization
        let community_storage_path = storage_path.join("communities");
        let community_sync_manager = CommunitySyncManager::new(
            community_storage_path,
            event_publisher.clone(),
        )?;
        
        let community_storage = community_sync_manager.get_storage();
        let sync_protocol_handler = SyncProtocolHandler::new(
            community_storage.clone(),
            event_publisher.clone(),
        );
        
        let conflict_resolver = ConflictResolver::new(ConflictResolutionStrategy::LastWriterWins);
        
        self.event_publisher = Some(event_publisher);
        self.notification_manager = Some(notification_manager);
        self.community_sync_manager = Some(std::sync::Arc::new(std::sync::Mutex::new(community_sync_manager)));
        self.sync_protocol_handler = Some(std::sync::Arc::new(sync_protocol_handler));
        self.conflict_resolver = Some(std::sync::Arc::new(std::sync::Mutex::new(conflict_resolver)));
        
        Ok(())
    }

    /// Initialize with P2P node when network feature is enabled
    #[cfg(feature = "network")]
    pub async fn with_node(node: saorsa_core::P2PNode) -> Self {
        CommunicationManager {
            node: Some(node),
        }
    }

    /// Send a direct message (legacy interface)
    pub async fn send_message(&self, _message: DirectMessage) -> Result<()> {
        // Convert legacy DirectMessage to new format if possible
        if let Some(_messaging) = &self.secure_messaging {
            // This is read-only, need mutable access for actual sending
            // Will be handled by separate send_secure_message method
        }
        
        #[cfg(feature = "network")]
        {
            if let Some(_node) = &self.node {
                // Network implementation will be added later
                todo!("Implement network direct messaging")
            }
        }
        
        // Fallback for non-network mode
        Ok(())
    }
    
    /// Send a secure message using four-word addresses
    pub async fn send_secure_message(
        &mut self, 
        to: FourWordAddress, 
        content: String
    ) -> Result<Uuid> {
        if let Some(messaging) = &mut self.secure_messaging {
            messaging.send_message(to, content).await
        } else {
            anyhow::bail!("Secure messaging not initialized")
        }
    }
    
    /// Receive a message envelope
    pub async fn receive_message_envelope(
        &mut self, 
        envelope: MessageEnvelope
    ) -> Result<Option<Message>> {
        if let Some(messaging) = &mut self.secure_messaging {
            messaging.receive_message_envelope(envelope).await
        } else {
            Ok(None)
        }
    }
    
    /// Get conversation history with a peer
    pub fn get_conversation_history(&self, peer: &FourWordAddress) -> Vec<StoredMessage> {
        if let Some(messaging) = &self.secure_messaging {
            messaging.get_conversation_history(peer)
        } else {
            Vec::new()
        }
    }
    
    /// Get all conversations summary
    pub fn get_conversations_summary(&self) -> Vec<ConversationSummary> {
        if let Some(messaging) = &self.secure_messaging {
            messaging.get_conversations_summary()
        } else {
            Vec::new()
        }
    }
    
    /// Process delivery queue
    pub async fn process_delivery_queue(&mut self) -> Result<Vec<DeliveryResult>> {
        if let Some(messaging) = &mut self.secure_messaging {
            messaging.process_delivery_queue().await
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Publish an event to the notification system
    pub fn publish_event(&self, event: Event) -> Result<()> {
        if let Some(publisher) = &self.event_publisher {
            publisher.publish(event)
        } else {
            Ok(())
        }
    }
    
    /// Get all system notifications
    pub fn get_system_notifications(&self) -> Vec<SystemNotification> {
        if let Some(manager) = &self.notification_manager {
            manager.get_notifications()
        } else {
            Vec::new()
        }
    }
    
    /// Get unread system notifications
    pub fn get_unread_system_notifications(&self) -> Vec<SystemNotification> {
        if let Some(manager) = &self.notification_manager {
            manager.get_unread_notifications()
        } else {
            Vec::new()
        }
    }
    
    /// Get system notifications by category
    pub fn get_system_notifications_by_category(&self, category: &str) -> Vec<SystemNotification> {
        if let Some(manager) = &self.notification_manager {
            manager.get_notifications_by_category(category)
        } else {
            Vec::new()
        }
    }
    
    /// Mark notification as read
    pub fn mark_notification_read(&self, notification_id: Uuid) -> bool {
        if let Some(manager) = &self.notification_manager {
            manager.mark_notification_read(notification_id)
        } else {
            false
        }
    }
    
    /// Mark all notifications as read
    pub fn mark_all_notifications_read(&self) {
        if let Some(manager) = &self.notification_manager {
            manager.mark_all_notifications_read()
        }
    }
    
    /// Remove a notification
    pub fn remove_notification(&self, notification_id: Uuid) -> bool {
        if let Some(manager) = &self.notification_manager {
            manager.remove_notification(notification_id)
        } else {
            false
        }
    }
    
    /// Clear all notifications
    pub fn clear_all_notifications(&self) {
        if let Some(manager) = &self.notification_manager {
            manager.clear_all_notifications()
        }
    }
    
    /// Get notification count
    pub fn notification_count(&self) -> usize {
        if let Some(manager) = &self.notification_manager {
            manager.notification_count()
        } else {
            0
        }
    }
    
    /// Get unread notification count
    pub fn unread_notification_count(&self) -> usize {
        if let Some(manager) = &self.notification_manager {
            manager.unread_notification_count()
        } else {
            0
        }
    }
    
    /// Subscribe to notifications
    pub fn subscribe_to_notifications(&self, handler: std::sync::Arc<dyn NotificationHandler>) {
        if let Some(manager) = &self.notification_manager {
            manager.subscribe(handler)
        }
    }
    
    /// Update notification configuration
    pub fn update_notification_config(&self, config: NotificationConfig) {
        if let Some(manager) = &self.notification_manager {
            manager.update_config(config)
        }
    }
    
    /// Get notification configuration
    pub fn get_notification_config(&self) -> Option<NotificationConfig> {
        if let Some(manager) = &self.notification_manager {
            Some(manager.get_config())
        } else {
            None
        }
    }
    
    /// Perform notification system maintenance
    pub fn perform_notification_maintenance(&self) {
        if let Some(manager) = &self.notification_manager {
            manager.maintenance()
        }
    }
    
    // Community synchronization methods
    
    /// Create a new community
    pub fn create_community(
        &self,
        name: String,
        description: String,
        creator: FourWordAddress,
    ) -> Result<Uuid> {
        if let Some(sync_manager) = &self.community_sync_manager {
            let manager = sync_manager.lock().unwrap();
            manager.create_community(name, description, creator)
        } else {
            anyhow::bail!("Community sync manager not initialized")
        }
    }
    
    /// Get community by ID
    pub fn get_community(&self, community_id: &Uuid) -> Option<Community> {
        if let Some(sync_manager) = &self.community_sync_manager {
            let manager = sync_manager.lock().unwrap();
            manager.get_community(community_id)
        } else {
            None
        }
    }
    
    /// List all communities
    pub fn list_communities(&self) -> Vec<CommunitySummary> {
        if let Some(sync_manager) = &self.community_sync_manager {
            let manager = sync_manager.lock().unwrap();
            manager.list_communities()
        } else {
            Vec::new()
        }
    }
    
    /// Get communities for a member
    pub fn get_communities_for_member(&self, member: &FourWordAddress) -> Vec<Community> {
        if let Some(sync_manager) = &self.community_sync_manager {
            let manager = sync_manager.lock().unwrap();
            manager.get_communities_for_member(member)
        } else {
            Vec::new()
        }
    }
    
    /// Add member to community
    pub fn add_member_to_community(
        &self,
        community_id: &Uuid,
        member: FourWordAddress,
        permission: PermissionLevel,
    ) -> Result<bool> {
        if let Some(sync_manager) = &self.community_sync_manager {
            let manager = sync_manager.lock().unwrap();
            manager.add_member_to_community(community_id, member, permission)
        } else {
            anyhow::bail!("Community sync manager not initialized")
        }
    }
    
    /// Update community metadata
    pub fn update_community_metadata(
        &self,
        community_id: &Uuid,
        key: String,
        value: String,
    ) -> Result<bool> {
        if let Some(sync_manager) = &self.community_sync_manager {
            let manager = sync_manager.lock().unwrap();
            manager.update_community_metadata(community_id, key, value)
        } else {
            anyhow::bail!("Community sync manager not initialized")
        }
    }
    
    /// Get sync status for a peer
    pub fn get_sync_status(&self, peer: &FourWordAddress) -> Option<SyncStatus> {
        if let Some(sync_manager) = &self.community_sync_manager {
            let manager = sync_manager.lock().unwrap();
            manager.get_sync_status(peer)
        } else {
            None
        }
    }
    
    /// Start community synchronization with peer
    pub async fn start_community_sync(&self, peer: FourWordAddress) -> Result<SyncSession> {
        if let Some(protocol_handler) = &self.sync_protocol_handler {
            protocol_handler.start_sync_session(peer).await
        } else {
            anyhow::bail!("Sync protocol handler not initialized")
        }
    }
    
    /// Handle incoming protocol message
    pub async fn handle_protocol_message(&self, message: ProtocolMessage) -> Result<Option<ProtocolMessage>> {
        if let Some(protocol_handler) = &self.sync_protocol_handler {
            protocol_handler.handle_message(message).await
        } else {
            Ok(None)
        }
    }
    
    /// Resolve conflicts for a community
    pub fn resolve_community_conflicts(
        &self,
        versions: &[Community],
        strategy: Option<ConflictResolutionStrategy>,
    ) -> Result<MergeResult> {
        if let Some(conflict_resolver) = &self.conflict_resolver {
            let mut resolver = conflict_resolver.lock().unwrap();
            resolver.resolve_conflicts(versions, strategy)
        } else {
            anyhow::bail!("Conflict resolver not initialized")
        }
    }

    /// Receive messages
    pub async fn receive_messages(&self) -> Result<Vec<DirectMessage>> {
        #[cfg(feature = "network")]
        {
            if let Some(_node) = &self.node {
                // Implementation will be added in Task 5
                todo!("Implement message receiving")
            }
        }
        
        // Fallback for non-network mode
        Ok(Vec::new())
    }

    /// Get notifications
    pub async fn get_notifications(&self) -> Result<Vec<Notification>> {
        #[cfg(feature = "network")]
        {
            if let Some(_node) = &self.node {
                // Implementation will be added in Task 6
                todo!("Implement notification system")
            }
        }
        
        // Fallback for non-network mode
        Ok(Vec::new())
    }
}

impl Default for CommunicationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = DirectMessage {
            id: Uuid::new_v4(),
            from: "alice".to_string(),
            to: "bob".to_string(),
            content: "Hello!".to_string(),
            timestamp: 0,
            encrypted: false,
        };
        
        assert_eq!(msg.from, "alice");
        assert_eq!(msg.to, "bob");
        assert_eq!(msg.content, "Hello!");
    }

    #[test]
    fn test_notification_creation() {
        let notif = Notification {
            id: Uuid::new_v4(),
            message_type: NotificationType::Message,
            content: "New message received".to_string(),
            timestamp: 0,
            read: false,
        };
        
        assert!(!notif.read);
        assert!(matches!(notif.message_type, NotificationType::Message));
    }

    #[test]
    fn test_communication_manager_creation() {
        let manager = CommunicationManager::new();
        // Basic creation test
        let _ = format!("{:?}", manager);
    }
}