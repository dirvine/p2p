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

//! Discuss system (Discourse-like) for long-form discussions and knowledge sharing
//! 
//! Features:
//! - Categories and subcategories for organization
//! - Topics with rich formatting and version history
//! - Threaded replies with voting
//! - Wiki mode for collaborative editing
//! - Moderation with threshold-based decisions
//! - Badges and reputation system

use crate::identity::enhanced::{EnhancedIdentity, OrganizationId};
use crate::storage::{StorageManager, keys, ttl};
// Removed unused ThresholdGroup import
use crate::quantum_crypto::types::GroupId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

/// Discuss errors
#[derive(Debug, Error)]
pub enum DiscussError {
    #[error("Storage error: {0}")]
    StorageError(#[from] crate::storage::StorageError),
    
    #[error("Category not found: {0}")]
    CategoryNotFound(String),
    
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

type Result<T> = std::result::Result<T, DiscussError>;

/// Category identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoryId(pub String);

impl Default for CategoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Topic identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopicId(pub String);

impl Default for TopicId {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Reply identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplyId(pub String);

impl Default for ReplyId {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplyId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// User ID type
pub type UserId = String;

/// Category for organizing discussions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub description: String,
    pub slug: String,
    pub parent_id: Option<CategoryId>,
    pub organization_id: Option<OrganizationId>,
    pub access_level: AccessLevel,
    pub moderator_group: Option<GroupId>,
    pub settings: CategorySettings,
    pub stats: CategoryStats,
    pub created_at: SystemTime,
    pub created_by: UserId,
}

/// Access level for categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessLevel {
    /// Anyone can view and post
    Public,
    
    /// Anyone can view, members can post
    Protected,
    
    /// Only members can view and post
    Private(GroupId),
    
    /// Read-only for everyone except moderators
    Announcement,
}

/// Category settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySettings {
    pub allow_polls: bool,
    pub allow_wiki_posts: bool,
    pub require_approval: bool,
    pub min_trust_level: TrustLevel,
    pub auto_close_days: Option<u32>,
    pub slow_mode_minutes: Option<u32>,
}

impl Default for CategorySettings {
    fn default() -> Self {
        Self {
            allow_polls: true,
            allow_wiki_posts: true,
            require_approval: false,
            min_trust_level: TrustLevel::Basic,
            auto_close_days: None,
            slow_mode_minutes: None,
        }
    }
}

/// Category statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryStats {
    pub topic_count: u64,
    pub post_count: u64,
    pub last_post_at: Option<SystemTime>,
}

/// User trust level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[derive(Default)]
pub enum TrustLevel {
    #[default]
    New = 0,
    Basic = 1,
    Member = 2,
    Regular = 3,
    Leader = 4,
}


/// Discussion topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: TopicId,
    pub category_id: CategoryId,
    pub title: String,
    pub slug: String,
    pub content: TopicContent,
    pub author: UserId,
    pub tags: Vec<Tag>,
    pub status: TopicStatus,
    pub topic_type: TopicType,
    pub stats: TopicStats,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub closed_at: Option<SystemTime>,
    pub deleted_at: Option<SystemTime>,
}

/// Topic content with version history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicContent {
    pub current_version: String,
    pub format: ContentFormat,
    pub versions: Vec<ContentVersion>,
    pub wiki_editors: Vec<UserId>,
}

/// Content format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentFormat {
    Markdown,
    Html,
    PlainText,
}

/// Content version for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentVersion {
    pub content: String,
    pub author: UserId,
    pub created_at: SystemTime,
    pub edit_reason: Option<String>,
}

/// Topic status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopicStatus {
    Open,
    Closed,
    Archived,
    Pinned,
    PinnedGlobally,
    Unlisted,
}

/// Topic type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopicType {
    Regular,
    Wiki,
    Poll,
    Question,
    Announcement,
}

/// Topic statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopicStats {
    pub view_count: u64,
    pub reply_count: u64,
    pub like_count: i64,
    pub bookmark_count: u64,
    pub unique_viewers: u64,
    pub last_reply_at: Option<SystemTime>,
}

/// Tag for categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub color: String,
}

/// Reply to a topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub id: ReplyId,
    pub topic_id: TopicId,
    pub author: UserId,
    pub content: String,
    pub reply_to: Option<ReplyId>,
    pub votes: VoteCount,
    pub accepted_answer: bool,
    pub created_at: SystemTime,
    pub edited_at: Option<SystemTime>,
    pub deleted_at: Option<SystemTime>,
    pub reactions: Vec<Reaction>,
}

/// Vote count
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoteCount {
    pub upvotes: u64,
    pub downvotes: u64,
    pub score: i64,
    pub voters: HashMap<UserId, VoteType>,
}

/// Vote type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VoteType {
    Up,
    Down,
}

/// Reaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub users: Vec<UserId>,
}

/// Poll in a topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poll {
    pub id: String,
    pub topic_id: TopicId,
    pub question: String,
    pub options: Vec<PollOption>,
    pub poll_type: PollType,
    pub closes_at: Option<SystemTime>,
    pub results_visible: PollResultsVisibility,
    pub voters: HashMap<UserId, Vec<usize>>,
}

/// Poll option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollOption {
    pub text: String,
    pub votes: u64,
}

/// Poll type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PollType {
    Single,
    Multiple { max_choices: usize },
    Ranked,
}

/// Poll results visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PollResultsVisibility {
    Always,
    OnVote,
    OnClose,
    Staff,
}

/// User badge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub badge_type: BadgeType,
    pub granted_at: SystemTime,
}

/// Badge type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BadgeType {
    Bronze,
    Silver,
    Gold,
    Special(String),
}

/// User statistics in discussions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserStats {
    pub topics_created: u64,
    pub replies_posted: u64,
    pub likes_given: u64,
    pub likes_received: u64,
    pub solutions_accepted: u64,
    pub days_visited: u64,
    pub posts_read: u64,
    pub trust_level: TrustLevel,
    pub badges: Vec<Badge>,
}

/// Moderation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationAction {
    pub id: String,
    pub action_type: ModerationType,
    pub target: ModerationTarget,
    pub moderator: UserId,
    pub reason: String,
    pub created_at: SystemTime,
    pub expires_at: Option<SystemTime>,
}

/// Moderation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModerationType {
    Close,
    Pin,
    Unlist,
    Delete,
    Move { to_category: CategoryId },
    Merge { with_topic: TopicId },
    Split { new_topic_id: TopicId },
}

/// Moderation target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModerationTarget {
    Topic(TopicId),
    Reply(ReplyId),
    User(UserId),
}

/// Discuss manager
pub struct DiscussManager {
    storage: StorageManager,
    identity: EnhancedIdentity,
}

impl DiscussManager {
    /// Create new discuss manager
    pub fn new(storage: StorageManager, identity: EnhancedIdentity) -> Self {
        Self { storage, identity }
    }
    
    /// Create a new category
    pub async fn create_category(
        &mut self,
        name: String,
        description: String,
        parent_id: Option<CategoryId>,
        access_level: AccessLevel,
        organization_id: Option<OrganizationId>,
    ) -> Result<Category> {
        let category = Category {
            id: CategoryId::new(),
            name: name.clone(),
            description,
            slug: self.slugify(&name),
            parent_id,
            organization_id,
            access_level,
            moderator_group: None,
            settings: CategorySettings::default(),
            stats: CategoryStats::default(),
            created_at: SystemTime::now(),
            created_by: self.identity.base_identity.user_id.clone(),
        };
        
        // Store category
        let key = format!("discuss:category:{}", category.id.0);
        self.storage.store_encrypted(
            &key,
            &category,
            ttl::PROFILE,
            None,
        ).await?;
        
        Ok(category)
    }
    
    /// Create a new topic
    pub async fn create_topic(
        &mut self,
        category_id: CategoryId,
        title: String,
        content: String,
        topic_type: TopicType,
        tags: Vec<Tag>,
    ) -> Result<Topic> {
        // Verify category exists and user has access
        let category = self.get_category(&category_id).await?;
        self.check_category_access(&category, false).await?;
        
        let topic = Topic {
            id: TopicId::new(),
            category_id: category_id.clone(),
            title: title.clone(),
            slug: self.slugify(&title),
            content: TopicContent {
                current_version: content.clone(),
                format: ContentFormat::Markdown,
                versions: vec![ContentVersion {
                    content,
                    author: self.identity.base_identity.user_id.clone(),
                    created_at: SystemTime::now(),
                    edit_reason: None,
                }],
                wiki_editors: vec![],
            },
            author: self.identity.base_identity.user_id.clone(),
            tags,
            status: TopicStatus::Open,
            topic_type,
            stats: TopicStats::default(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            closed_at: None,
            deleted_at: None,
        };
        
        // Store topic
        let key = keys::discuss_topic(&topic.id.0);
        self.storage.store_encrypted(
            &key,
            &topic,
            ttl::PROFILE,
            None,
        ).await?;
        
        // Update category stats
        self.update_category_stats(&category_id, 1, 0).await?;
        
        Ok(topic)
    }
    
    /// Post a reply
    pub async fn post_reply(
        &mut self,
        topic_id: TopicId,
        content: String,
        reply_to: Option<ReplyId>,
    ) -> Result<Reply> {
        // Verify topic exists and is open
        let topic = self.get_topic(&topic_id).await?;
        if !matches!(topic.status, TopicStatus::Open) {
            return Err(DiscussError::InvalidOperation("Topic is not open".to_string()));
        }
        
        let reply = Reply {
            id: ReplyId::new(),
            topic_id: topic_id.clone(),
            author: self.identity.base_identity.user_id.clone(),
            content,
            reply_to,
            votes: VoteCount::default(),
            accepted_answer: false,
            created_at: SystemTime::now(),
            edited_at: None,
            deleted_at: None,
            reactions: vec![],
        };
        
        // Store reply
        let key = keys::discuss_reply(&topic_id.0, &reply.id.0);
        self.storage.store_encrypted(
            &key,
            &reply,
            ttl::MESSAGE,
            None,
        ).await?;
        
        // Update topic stats
        self.update_topic_stats(&topic_id, 0, 1).await?;
        
        Ok(reply)
    }
    
    /// Vote on content
    pub async fn vote(
        &mut self,
        target: VoteTarget,
        vote_type: VoteType,
    ) -> Result<()> {
        match target {
            VoteTarget::Topic(topic_id) => {
                let mut topic = self.get_topic(&topic_id).await?;
                // Update vote count (simplified)
                match vote_type {
                    VoteType::Up => topic.stats.like_count += 1,
                    VoteType::Down => topic.stats.like_count -= 1,
                }
                
                let key = keys::discuss_topic(&topic_id.0);
                self.storage.store_encrypted(
                    &key,
                    &topic,
                    ttl::PROFILE,
                    None,
                ).await?;
            }
            VoteTarget::Reply(topic_id, reply_id) => {
                let key = keys::discuss_reply(&topic_id.0, &reply_id.0);
                let mut reply: Reply = self.storage.get_encrypted(&key).await?;
                
                let user_id = &self.identity.base_identity.user_id;
                reply.votes.voters.insert(user_id.clone(), vote_type);
                
                // Recalculate score
                reply.votes.upvotes = reply.votes.voters.values()
                    .filter(|&&v| matches!(v, VoteType::Up)).count() as u64;
                reply.votes.downvotes = reply.votes.voters.values()
                    .filter(|&&v| matches!(v, VoteType::Down)).count() as u64;
                reply.votes.score = reply.votes.upvotes as i64 - reply.votes.downvotes as i64;
                
                self.storage.store_encrypted(
                    &key,
                    &reply,
                    ttl::MESSAGE,
                    None,
                ).await?;
            }
        }
        
        Ok(())
    }
    
    /// Edit topic (wiki mode)
    pub async fn edit_topic(
        &mut self,
        topic_id: &TopicId,
        new_content: String,
        edit_reason: Option<String>,
    ) -> Result<()> {
        let mut topic = self.get_topic(topic_id).await?;
        
        // Check if user can edit
        let user_id = &self.identity.base_identity.user_id;
        let can_edit = topic.author == *user_id || 
                      topic.content.wiki_editors.contains(user_id) ||
                      matches!(topic.topic_type, TopicType::Wiki);
        
        if !can_edit {
            return Err(DiscussError::PermissionDenied("Cannot edit topic".to_string()));
        }
        
        // Add new version
        topic.content.versions.push(ContentVersion {
            content: new_content.clone(),
            author: user_id.clone(),
            created_at: SystemTime::now(),
            edit_reason,
        });
        topic.content.current_version = new_content;
        topic.updated_at = SystemTime::now();
        
        // Store updated topic
        let key = keys::discuss_topic(&topic_id.0);
        self.storage.store_encrypted(
            &key,
            &topic,
            ttl::PROFILE,
            None,
        ).await?;
        
        Ok(())
    }
    
    /// Get category by ID
    async fn get_category(&self, category_id: &CategoryId) -> Result<Category> {
        let key = format!("discuss:category:{}", category_id.0);
        self.storage.get_encrypted(&key).await
            .map_err(|_| DiscussError::CategoryNotFound(category_id.0.clone()))
    }
    
    /// Get topic by ID
    async fn get_topic(&self, topic_id: &TopicId) -> Result<Topic> {
        let key = keys::discuss_topic(&topic_id.0);
        self.storage.get_encrypted(&key).await
            .map_err(|_| DiscussError::TopicNotFound(topic_id.0.clone()))
    }
    
    /// Check category access
    async fn check_category_access(&self, category: &Category, write: bool) -> Result<()> {
        match &category.access_level {
            AccessLevel::Public => Ok(()),
            AccessLevel::Protected => {
                if write {
                    // Check if user is member
                    // TODO: Implement membership check
                    Ok(())
                } else {
                    Ok(())
                }
            }
            AccessLevel::Private(_group_id) => {
                // Check if user is in threshold group
                // TODO: Implement group membership check
                Ok(())
            }
            AccessLevel::Announcement => {
                if write {
                    // Check if user is moderator
                    // TODO: Implement moderator check
                    Err(DiscussError::PermissionDenied("Announcement category".to_string()))
                } else {
                    Ok(())
                }
            }
        }
    }
    
    /// Update category statistics
    async fn update_category_stats(
        &mut self,
        category_id: &CategoryId,
        topic_delta: i64,
        post_delta: i64,
    ) -> Result<()> {
        let mut category = self.get_category(category_id).await?;
        
        category.stats.topic_count = (category.stats.topic_count as i64 + topic_delta) as u64;
        category.stats.post_count = (category.stats.post_count as i64 + post_delta) as u64;
        category.stats.last_post_at = Some(SystemTime::now());
        
        let key = format!("discuss:category:{}", category_id.0);
        self.storage.store_encrypted(
            &key,
            &category,
            ttl::PROFILE,
            None,
        ).await?;
        
        Ok(())
    }
    
    /// Update topic statistics
    async fn update_topic_stats(
        &mut self,
        topic_id: &TopicId,
        view_delta: i64,
        reply_delta: i64,
    ) -> Result<()> {
        let mut topic = self.get_topic(topic_id).await?;
        
        topic.stats.view_count = (topic.stats.view_count as i64 + view_delta) as u64;
        topic.stats.reply_count = (topic.stats.reply_count as i64 + reply_delta) as u64;
        if reply_delta > 0 {
            topic.stats.last_reply_at = Some(SystemTime::now());
        }
        
        let key = keys::discuss_topic(&topic_id.0);
        self.storage.store_encrypted(
            &key,
            &topic,
            ttl::PROFILE,
            None,
        ).await?;
        
        Ok(())
    }
    
    /// Convert string to URL-friendly slug
    fn slugify(&self, text: &str) -> String {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

/// Vote target
pub enum VoteTarget {
    Topic(TopicId),
    Reply(TopicId, ReplyId),
}