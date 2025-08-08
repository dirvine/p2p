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

//! Chat system (Slack-like) with channels, threads, and real-time messaging
//!
//! Features:
//! - Public and private channels with threshold access control
//! - Direct messages with E2E encryption
//! - Threaded conversations
//! - Voice/video calls via WebRTC
//! - Rich media support

use crate::identity::enhanced::{EnhancedIdentity, OrganizationId};
use crate::storage::{StorageManager, keys, ttl};
// Removed unused ThresholdGroup import
use crate::quantum_crypto::types::GroupId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

/// Chat errors
#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Storage error: {0}")]
    StorageError(#[from] crate::storage::StorageError),

    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

type Result<T> = std::result::Result<T, ChatError>;

/// Channel identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl Default for ChannelId {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelId {
    /// Generate new channel ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Message identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageId {
    /// Generate new message ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Thread identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreadId(pub String);

/// User ID type
pub type UserId = String;

/// Channel type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    /// Public channel - anyone in org can join
    Public,

    /// Private channel - controlled by threshold group
    Private {
        access_group: GroupId,
        visibility: ChannelVisibility,
    },

    /// Direct message between users
    Direct { participants: Vec<UserId> },

    /// Group DM with multiple users
    GroupDirect {
        participants: Vec<UserId>,
        name: Option<String>,
    },
}

/// Channel visibility for private channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelVisibility {
    /// Visible to all, but join requires approval
    Listed,

    /// Not visible unless member
    Hidden,

    /// Invite-only
    Secret,
}

/// Channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    pub description: String,
    pub channel_type: ChannelType,
    pub organization_id: Option<OrganizationId>,
    pub created_by: UserId,
    pub created_at: SystemTime,
    pub members: Vec<ChannelMember>,
    pub settings: ChannelSettings,
    pub metadata: HashMap<String, String>,
}

/// Channel member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMember {
    pub user_id: UserId,
    pub role: ChannelRole,
    pub joined_at: SystemTime,
    pub last_read: Option<MessageId>,
    pub notifications: NotificationSettings,
}

/// Channel role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelRole {
    Owner,
    Admin,
    Moderator,
    Member,
    Guest,
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub all_messages: bool,
    pub mentions_only: bool,
    pub muted: bool,
    pub muted_until: Option<SystemTime>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            all_messages: true,
            mentions_only: false,
            muted: false,
            muted_until: None,
        }
    }
}

/// Channel settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSettings {
    pub allow_threads: bool,
    pub allow_reactions: bool,
    pub allow_files: bool,
    pub allow_voice_video: bool,
    pub message_retention_days: Option<u32>,
    pub max_message_length: usize,
    pub slow_mode_seconds: Option<u32>,
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self {
            allow_threads: true,
            allow_reactions: true,
            allow_files: true,
            allow_voice_video: true,
            message_retention_days: Some(90),
            max_message_length: 4000,
            slow_mode_seconds: None,
        }
    }
}

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub thread_id: Option<ThreadId>,
    pub author: UserId,
    pub content: MessageContent,
    pub created_at: SystemTime,
    pub edited_at: Option<SystemTime>,
    pub deleted_at: Option<SystemTime>,
    pub reactions: Vec<Reaction>,
    pub mentions: Vec<Mention>,
    pub attachments: Vec<Attachment>,
    pub reply_to: Option<MessageId>,
}

/// Message content type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text message
    Text(String),

    /// Rich text with formatting
    RichText {
        text: String,
        formatting: Vec<TextFormat>,
    },

    /// System message (join/leave/etc)
    System(SystemMessage),

    /// Encrypted content (for DMs)
    Encrypted {
        ciphertext: Vec<u8>,
        algorithm: String,
    },
}

/// Text formatting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFormat {
    pub start: usize,
    pub end: usize,
    pub format_type: FormatType,
}

/// Format type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormatType {
    Bold,
    Italic,
    Code,
    Strike,
    Link(String),
    Mention(UserId),
    ChannelRef(ChannelId),
}

/// System message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMessage {
    UserJoined(UserId),
    UserLeft(UserId),
    UserInvited {
        inviter: UserId,
        invitee: UserId,
    },
    ChannelRenamed {
        old_name: String,
        new_name: String,
    },
    ChannelDescriptionChanged,
    CallStarted {
        call_id: String,
    },
    CallEnded {
        call_id: String,
        duration_seconds: u64,
    },
}

/// Reaction to a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub users: Vec<UserId>,
}

/// Mention in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mention {
    User(UserId),
    Channel,
    Here,
    Everyone,
}

/// File attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size: u64,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub metadata: AttachmentMetadata,
}

/// Attachment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentMetadata {
    Image {
        width: u32,
        height: u32,
    },
    Video {
        duration_seconds: u64,
        width: u32,
        height: u32,
    },
    Audio {
        duration_seconds: u64,
    },
    Document {
        page_count: Option<u32>,
    },
    Other,
}

/// Thread information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    pub channel_id: ChannelId,
    pub parent_message_id: MessageId,
    pub reply_count: u32,
    pub participant_count: u32,
    pub last_reply_at: Option<SystemTime>,
    pub participants: Vec<UserId>,
}

/// Voice/video call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub id: String,
    pub channel_id: ChannelId,
    pub call_type: CallType,
    pub started_by: UserId,
    pub started_at: SystemTime,
    pub ended_at: Option<SystemTime>,
    pub participants: Vec<CallParticipant>,
    pub recording_url: Option<String>,
}

/// Call type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallType {
    Voice,
    Video,
    ScreenShare,
}

/// Call participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallParticipant {
    pub user_id: UserId,
    pub joined_at: SystemTime,
    pub left_at: Option<SystemTime>,
    pub is_muted: bool,
    pub is_video_on: bool,
    pub is_screen_sharing: bool,
}

/// Chat manager
pub struct ChatManager {
    storage: StorageManager,
    identity: EnhancedIdentity,
}

impl ChatManager {
    /// Create new chat manager
    pub fn new(storage: StorageManager, identity: EnhancedIdentity) -> Self {
        Self { storage, identity }
    }

    /// Create a new channel
    pub async fn create_channel(
        &mut self,
        name: String,
        description: String,
        channel_type: ChannelType,
        organization_id: Option<OrganizationId>,
    ) -> Result<Channel> {
        // Check permissions
        if let Some(_org_id) = &organization_id {
            // Verify user has permission to create channels in org
            // TODO: Implement permission check
        }

        let channel = Channel {
            id: ChannelId::new(),
            name,
            description,
            channel_type,
            organization_id,
            created_by: self.identity.base_identity.user_id.clone(),
            created_at: SystemTime::now(),
            members: vec![ChannelMember {
                user_id: self.identity.base_identity.user_id.clone(),
                role: ChannelRole::Owner,
                joined_at: SystemTime::now(),
                last_read: None,
                notifications: NotificationSettings::default(),
            }],
            settings: ChannelSettings::default(),
            metadata: HashMap::new(),
        };

        // Store channel in DHT
        let key = keys::chat_channel(&channel.id.0);
        self.storage
            .store_encrypted(&key, &channel, ttl::PROFILE, None)
            .await?;

        // Add to user's channel list
        self.add_user_channel(&channel.id).await?;

        // If public, add to public channel list
        if matches!(channel.channel_type, ChannelType::Public) {
            self.add_public_channel(&channel.id, &channel.name).await?;
        }

        Ok(channel)
    }

    /// Send a message
    pub async fn send_message(
        &mut self,
        channel_id: &ChannelId,
        content: MessageContent,
        thread_id: Option<ThreadId>,
        attachments: Vec<Attachment>,
    ) -> Result<Message> {
        // Verify user is member of channel
        let channel = self.get_channel(channel_id).await?;
        if !channel
            .members
            .iter()
            .any(|m| m.user_id == self.identity.base_identity.user_id)
        {
            return Err(ChatError::PermissionDenied(
                "Not a member of channel".to_string(),
            ));
        }

        let message = Message {
            id: MessageId::new(),
            channel_id: channel_id.clone(),
            thread_id,
            author: self.identity.base_identity.user_id.clone(),
            content,
            created_at: SystemTime::now(),
            edited_at: None,
            deleted_at: None,
            reactions: vec![],
            mentions: vec![], // TODO: Extract mentions from content
            attachments,
            reply_to: None,
        };

        // Store message
        let msg_key = keys::chat_message(&channel_id.0, &message.id.0);
        self.storage
            .store_encrypted(&msg_key, &message, ttl::MESSAGE, None)
            .await?;

        // Add to message index for pagination
        let timestamp = message
            .created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| ChatError::InvalidOperation(format!("Invalid message timestamp: {}", e)))?
            .as_secs();
        let index_key = keys::chat_index(&channel_id.0, timestamp);
        self.storage
            .store_encrypted(&index_key, &message.id, ttl::MESSAGE, None)
            .await?;

        Ok(message)
    }

    /// Get channel by ID
    pub async fn get_channel(&self, channel_id: &ChannelId) -> Result<Channel> {
        let key = keys::chat_channel(&channel_id.0);
        self.storage
            .get_encrypted(&key)
            .await
            .map_err(|_| ChatError::ChannelNotFound(channel_id.0.clone()))
    }

    /// Get user's channels
    pub async fn get_user_channels(&self) -> Result<Vec<ChannelId>> {
        let key = keys::user_channels(&self.identity.base_identity.user_id);
        Ok(self
            .storage
            .get_encrypted(&key)
            .await
            .unwrap_or_else(|_| vec![]))
    }

    /// Add channel to user's list
    async fn add_user_channel(&mut self, channel_id: &ChannelId) -> Result<()> {
        let mut channels = self.get_user_channels().await.unwrap_or_default();
        if !channels.contains(channel_id) {
            channels.push(channel_id.clone());

            let key = keys::user_channels(&self.identity.base_identity.user_id);
            self.storage
                .store_encrypted(&key, &channels, ttl::PROFILE, None)
                .await?;
        }
        Ok(())
    }

    /// Add channel to public list
    async fn add_public_channel(&mut self, channel_id: &ChannelId, name: &str) -> Result<()> {
        let key = keys::public_channel_list();
        let mut public_channels: HashMap<String, String> =
            self.storage.get_public(&key).await.unwrap_or_default();

        public_channels.insert(channel_id.0.clone(), name.to_string());

        self.storage
            .store_public(&key, &public_channels, ttl::PROFILE)
            .await?;

        Ok(())
    }

    /// Create a thread
    pub async fn create_thread(
        &mut self,
        channel_id: &ChannelId,
        parent_message_id: &MessageId,
    ) -> Result<Thread> {
        let thread = Thread {
            id: ThreadId(Uuid::new_v4().to_string()),
            channel_id: channel_id.clone(),
            parent_message_id: parent_message_id.clone(),
            reply_count: 0,
            participant_count: 1,
            last_reply_at: None,
            participants: vec![self.identity.base_identity.user_id.clone()],
        };

        // Store thread (in practice, would be part of message structure)
        Ok(thread)
    }

    /// Add reaction to message
    pub async fn add_reaction(
        &mut self,
        channel_id: &ChannelId,
        message_id: &MessageId,
        emoji: String,
    ) -> Result<()> {
        let msg_key = keys::chat_message(&channel_id.0, &message_id.0);
        let mut message: Message = self.storage.get_encrypted(&msg_key).await?;

        // Find or create reaction
        let user_id = &self.identity.base_identity.user_id;
        if let Some(reaction) = message.reactions.iter_mut().find(|r| r.emoji == emoji) {
            if !reaction.users.contains(user_id) {
                reaction.users.push(user_id.clone());
            }
        } else {
            message.reactions.push(Reaction {
                emoji,
                users: vec![user_id.clone()],
            });
        }

        // Store updated message
        self.storage
            .store_encrypted(&msg_key, &message, ttl::MESSAGE, None)
            .await?;

        Ok(())
    }
}
