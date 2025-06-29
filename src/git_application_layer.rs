//! Git Application Layer
//!
//! This module provides high-level git operations for applications,
//! abstracting the complexity of git objects and DHT storage into
//! simple application-focused APIs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

use crate::git_content_addressing::{ContentHash, ObjectType, GitObject, GitResult, GitContentError};
use crate::git_objects::{
    BlobObject, TreeObject, CommitObject, TagObject, Reference, ReferenceType,
    CommitAuthor, CommitType, BranchState, TreeEntry, EntryMode,
};
use crate::git_dht_storage::GitDhtStorage;
use crate::storage::DataAccessLevel;

/// High-level git operations for applications
pub struct GitApplicationLayer {
    /// Git-DHT storage backend
    storage: GitDhtStorage,
}

/// Chat message structure for git integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub channel_id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub timestamp: SystemTime,
    pub reply_to: Option<ContentHash>,
    pub attachments: Vec<Attachment>,
    pub reactions: Vec<Reaction>,
}

/// File attachment for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub content_hash: ContentHash,
    pub size: u64,
    pub mime_type: Option<String>,
}

/// Message reaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub user_id: String,
    pub timestamp: SystemTime,
}

/// Forum post structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumPost {
    pub id: String,
    pub topic_id: String,
    pub author: String,
    pub author_name: String,
    pub title: String,
    pub content: String,
    pub timestamp: SystemTime,
    pub parent_post: Option<ContentHash>,
    pub tags: Vec<String>,
}

/// Document structure for collaborative editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub format: DocumentFormat,
    pub author: String,
    pub created_at: SystemTime,
    pub modified_at: SystemTime,
    pub version: u64,
}

/// Document formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentFormat {
    Markdown,
    PlainText,
    Json,
    Yaml,
    Code(String), // Language name
}

impl GitApplicationLayer {
    /// Create new git application layer
    pub fn new(storage: GitDhtStorage) -> Self {
        Self { storage }
    }

    // === Chat Operations ===

    /// Send message to chat (creates git commit)
    pub async fn send_chat_message(
        &self,
        channel_id: &str,
        message_content: &str,
        sender: String,
        sender_name: String,
        reply_to: Option<ContentHash>,
        attachments: Vec<Attachment>,
    ) -> GitResult<ContentHash> {
        // Create message blob
        let message_blob = BlobObject::from_text(message_content)
            .with_mime_type("text/plain");
        
        // Store message blob
        let blob_content = bincode::serialize(&message_blob)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let blob_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            blob_content,
            DataAccessLevel::GroupShared {
                encrypted_data: Default::default(),
                threshold_metadata: Default::default(),
                group_id: channel_id.to_string(),
                required_shares: 2,
            },
            sender.clone(),
            Some(std::time::Duration::from_secs(90 * 24 * 60 * 60)), // 90 days
        )).await?;

        // Create tree with message and attachments
        let mut tree = TreeObject::new();
        tree.add_blob("message.txt".to_string(), blob_hash, message_content.len() as u64);

        // Add attachments to tree
        for (i, attachment) in attachments.iter().enumerate() {
            tree.add_blob(
                format!("attachment_{}", i),
                attachment.content_hash.clone(),
                attachment.size,
            );
        }

        // Get current branch head as parent
        let parents = if let Some(current_ref) = self.storage.get_reference(channel_id, "main").await? {
            vec![current_ref.target]
        } else {
            vec![]
        };

        // Add reply-to parent if specified
        let mut all_parents = parents;
        if let Some(reply_hash) = reply_to {
            if !all_parents.contains(&reply_hash) {
                all_parents.push(reply_hash);
            }
        }

        // Create commit
        let commit_hash = self.storage.create_commit(
            tree,
            all_parents,
            format!("Message: {}", message_content.chars().take(50).collect::<String>()),
            CommitAuthor {
                peer_id: sender,
                name: sender_name,
                email: None,
                timestamp: SystemTime::now(),
            },
            "chat".to_string(),
            channel_id.to_string(),
            CommitType::MessageSent,
        ).await?;

        // Update branch head
        self.storage.update_branch(channel_id, "main", commit_hash).await?;

        Ok(commit_hash)
    }

    /// Get chat history (git log for channel)
    pub async fn get_chat_history(&self, channel_id: &str, limit: usize) -> GitResult<Vec<ChatMessage>> {
        // Get branch head
        let branch = self.storage.get_reference(channel_id, "main").await?
            .ok_or_else(|| GitContentError::ObjectNotFound(format!("branch:{}:main", channel_id)))?;

        // Get commit history
        let commits = self.storage.get_commit_history(&branch.target, limit).await?;

        let mut messages = Vec::new();
        for commit in commits {
            // Extract message from commit tree
            if let Some(tree_object) = self.storage.get_object(&commit.tree).await? {
                let tree: TreeObject = bincode::deserialize(&tree_object.content)
                    .map_err(|e| GitContentError::SerializationError(e.to_string()))?;

                let mut message_content = String::new();
                let mut attachments = Vec::new();

                // Process tree entries
                for entry in tree.entries {
                    if entry.name == "message.txt" {
                        if let Some(blob_object) = self.storage.get_object(&entry.hash).await? {
                            let blob: BlobObject = bincode::deserialize(&blob_object.content)
                                .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
                            message_content = blob.as_string()?;
                        }
                    } else if entry.name.starts_with("attachment_") {
                        attachments.push(Attachment {
                            name: entry.name.clone(),
                            content_hash: entry.hash,
                            size: entry.size,
                            mime_type: None,
                        });
                    }
                }

                messages.push(ChatMessage {
                    id: commit.tree.short(),
                    channel_id: channel_id.to_string(),
                    sender: commit.author.peer_id,
                    sender_name: Some(commit.author.name),
                    content: message_content,
                    timestamp: commit.timestamp,
                    reply_to: commit.parents.first().cloned(),
                    attachments,
                    reactions: Vec::new(), // TODO: Add reaction support
                });
            }
        }

        Ok(messages)
    }

    /// Edit a chat message (creates new commit)
    pub async fn edit_chat_message(
        &self,
        channel_id: &str,
        original_commit: ContentHash,
        new_content: &str,
        editor: String,
    ) -> GitResult<ContentHash> {
        // Create new message blob
        let message_blob = BlobObject::from_text(new_content);
        let blob_content = bincode::serialize(&message_blob)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let blob_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            blob_content,
            DataAccessLevel::GroupShared {
                encrypted_data: Default::default(),
                threshold_metadata: Default::default(),
                group_id: channel_id.to_string(),
                required_shares: 2,
            },
            editor.clone(),
            Some(std::time::Duration::from_secs(90 * 24 * 60 * 60)),
        )).await?;

        // Create tree with edited message
        let mut tree = TreeObject::new();
        tree.add_blob("message.txt".to_string(), blob_hash, new_content.len() as u64);

        // Get current branch head
        let current_ref = self.storage.get_reference(channel_id, "main").await?
            .ok_or_else(|| GitContentError::ObjectNotFound(format!("branch:{}:main", channel_id)))?;

        // Create edit commit with both current head and original message as parents
        let commit_hash = self.storage.create_commit(
            tree,
            vec![current_ref.target, original_commit],
            format!("Edit message: {}", new_content.chars().take(50).collect::<String>()),
            CommitAuthor {
                peer_id: editor,
                name: "Editor".to_string(), // TODO: Get actual name
                email: None,
                timestamp: SystemTime::now(),
            },
            "chat".to_string(),
            channel_id.to_string(),
            CommitType::MessageEdited,
        ).await?;

        // Update branch head
        self.storage.update_branch(channel_id, "main", commit_hash).await?;

        Ok(commit_hash)
    }

    // === Forum Operations ===

    /// Create forum post (creates git repository)
    pub async fn create_forum_post(
        &self,
        topic_id: &str,
        title: &str,
        content: &str,
        author: String,
        author_name: String,
        tags: Vec<String>,
    ) -> GitResult<ContentHash> {
        // Create post content blob
        let post_blob = BlobObject::from_text(content)
            .with_mime_type("text/markdown");

        // Store post blob
        let blob_content = bincode::serialize(&post_blob)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let blob_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            blob_content,
            DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            author.clone(),
            Some(std::time::Duration::from_secs(365 * 24 * 60 * 60)), // 1 year
        )).await?;

        // Create metadata blob for title and tags
        let metadata = HashMap::from([
            ("title".to_string(), title.to_string()),
            ("tags".to_string(), tags.join(",")),
            ("format".to_string(), "markdown".to_string()),
        ]);
        let metadata_content = bincode::serialize(&metadata)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let metadata_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            metadata_content,
            DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            author.clone(),
            Some(std::time::Duration::from_secs(365 * 24 * 60 * 60)),
        )).await?;

        // Create tree with post and metadata
        let mut tree = TreeObject::new();
        tree.add_blob("post.md".to_string(), blob_hash, content.len() as u64);
        tree.add_blob("metadata.json".to_string(), metadata_hash, metadata_content.len() as u64);

        // Create initial commit
        let commit_hash = self.storage.create_commit(
            tree,
            vec![], // No parents for initial commit
            format!("Created topic: {}", title),
            CommitAuthor {
                peer_id: author,
                name: author_name,
                email: None,
                timestamp: SystemTime::now(),
            },
            "forum".to_string(),
            topic_id.to_string(),
            CommitType::DocumentCreated,
        ).await?;

        // Create main branch
        self.storage.update_branch(topic_id, "main", commit_hash).await?;

        Ok(commit_hash)
    }

    /// Reply to forum post
    pub async fn reply_to_forum_post(
        &self,
        topic_id: &str,
        parent_commit: ContentHash,
        content: &str,
        author: String,
        author_name: String,
    ) -> GitResult<ContentHash> {
        // Create reply content blob
        let reply_blob = BlobObject::from_text(content)
            .with_mime_type("text/markdown");

        let blob_content = bincode::serialize(&reply_blob)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let blob_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            blob_content,
            DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            author.clone(),
            Some(std::time::Duration::from_secs(365 * 24 * 60 * 60)),
        )).await?;

        // Create tree with reply
        let mut tree = TreeObject::new();
        tree.add_blob("reply.md".to_string(), blob_hash, content.len() as u64);

        // Get current branch head
        let current_ref = self.storage.get_reference(topic_id, "main").await?
            .ok_or_else(|| GitContentError::ObjectNotFound(format!("branch:{}:main", topic_id)))?;

        // Create reply commit
        let commit_hash = self.storage.create_commit(
            tree,
            vec![current_ref.target, parent_commit],
            format!("Reply: {}", content.chars().take(50).collect::<String>()),
            CommitAuthor {
                peer_id: author,
                name: author_name,
                email: None,
                timestamp: SystemTime::now(),
            },
            "forum".to_string(),
            topic_id.to_string(),
            CommitType::MessageSent,
        ).await?;

        // Update branch head
        self.storage.update_branch(topic_id, "main", commit_hash).await?;

        Ok(commit_hash)
    }

    // === Document Operations ===

    /// Create a new document
    pub async fn create_document(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
        format: DocumentFormat,
        author: String,
    ) -> GitResult<ContentHash> {
        // Determine MIME type based on format
        let mime_type = match format {
            DocumentFormat::Markdown => "text/markdown",
            DocumentFormat::PlainText => "text/plain",
            DocumentFormat::Json => "application/json",
            DocumentFormat::Yaml => "application/yaml",
            DocumentFormat::Code(_) => "text/plain",
        };

        // Create document content blob
        let doc_blob = BlobObject::from_text(content)
            .with_mime_type(mime_type);

        let blob_content = bincode::serialize(&doc_blob)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let blob_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            blob_content,
            DataAccessLevel::GroupShared {
                encrypted_data: Default::default(),
                threshold_metadata: Default::default(),
                group_id: document_id.to_string(),
                required_shares: 2,
            },
            author.clone(),
            Some(std::time::Duration::from_secs(365 * 24 * 60 * 60)),
        )).await?;

        // Create document metadata
        let metadata = HashMap::from([
            ("title".to_string(), title.to_string()),
            ("format".to_string(), serde_json::to_string(&format).unwrap_or_default()),
            ("version".to_string(), "1".to_string()),
        ]);
        let metadata_content = bincode::serialize(&metadata)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let metadata_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            metadata_content,
            DataAccessLevel::GroupShared {
                encrypted_data: Default::default(),
                threshold_metadata: Default::default(),
                group_id: document_id.to_string(),
                required_shares: 2,
            },
            author.clone(),
            Some(std::time::Duration::from_secs(365 * 24 * 60 * 60)),
        )).await?;

        // Create tree with document and metadata
        let mut tree = TreeObject::new();
        tree.add_blob("document".to_string(), blob_hash, content.len() as u64);
        tree.add_blob("metadata.json".to_string(), metadata_hash, metadata_content.len() as u64);

        // Create initial commit
        let commit_hash = self.storage.create_commit(
            tree,
            vec![],
            format!("Created document: {}", title),
            CommitAuthor {
                peer_id: author,
                name: "Author".to_string(), // TODO: Get actual name
                email: None,
                timestamp: SystemTime::now(),
            },
            "document".to_string(),
            document_id.to_string(),
            CommitType::DocumentCreated,
        ).await?;

        // Create main branch
        self.storage.update_branch(document_id, "main", commit_hash).await?;

        Ok(commit_hash)
    }

    /// Update a document (creates new version)
    pub async fn update_document(
        &self,
        document_id: &str,
        new_content: &str,
        editor: String,
        commit_message: Option<String>,
    ) -> GitResult<ContentHash> {
        // Get current document to preserve metadata
        let current_ref = self.storage.get_reference(document_id, "main").await?
            .ok_or_else(|| GitContentError::ObjectNotFound(format!("branch:{}:main", document_id)))?;

        // Create new content blob
        let doc_blob = BlobObject::from_text(new_content);
        let blob_content = bincode::serialize(&doc_blob)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let blob_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            blob_content,
            DataAccessLevel::GroupShared {
                encrypted_data: Default::default(),
                threshold_metadata: Default::default(),
                group_id: document_id.to_string(),
                required_shares: 2,
            },
            editor.clone(),
            Some(std::time::Duration::from_secs(365 * 24 * 60 * 60)),
        )).await?;

        // TODO: Preserve and update metadata from previous version
        let metadata = HashMap::from([
            ("version".to_string(), "updated".to_string()),
            ("last_modified".to_string(), SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string()),
        ]);
        let metadata_content = bincode::serialize(&metadata)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let metadata_hash = self.storage.store_object(GitObject::new(
            ObjectType::Blob,
            metadata_content,
            DataAccessLevel::GroupShared {
                encrypted_data: Default::default(),
                threshold_metadata: Default::default(),
                group_id: document_id.to_string(),
                required_shares: 2,
            },
            editor.clone(),
            Some(std::time::Duration::from_secs(365 * 24 * 60 * 60)),
        )).await?;

        // Create tree with updated content
        let mut tree = TreeObject::new();
        tree.add_blob("document".to_string(), blob_hash, new_content.len() as u64);
        tree.add_blob("metadata.json".to_string(), metadata_hash, metadata_content.len() as u64);

        // Create update commit
        let message = commit_message.unwrap_or_else(|| "Update document".to_string());
        let commit_hash = self.storage.create_commit(
            tree,
            vec![current_ref.target],
            message,
            CommitAuthor {
                peer_id: editor,
                name: "Editor".to_string(), // TODO: Get actual name
                email: None,
                timestamp: SystemTime::now(),
            },
            "document".to_string(),
            document_id.to_string(),
            CommitType::DocumentUpdated,
        ).await?;

        // Update branch head
        self.storage.update_branch(document_id, "main", commit_hash).await?;

        Ok(commit_hash)
    }

    // === Utility Operations ===

    /// Get repository statistics
    pub async fn get_repository_stats(&self, namespace: &str) -> GitResult<RepositoryStats> {
        let branch = self.storage.get_reference(namespace, "main").await?
            .ok_or_else(|| GitContentError::ObjectNotFound(format!("branch:{}:main", namespace)))?;

        let commits = self.storage.get_commit_history(&branch.target, 1000).await?;
        
        let mut stats = RepositoryStats {
            namespace: namespace.to_string(),
            total_commits: commits.len(),
            latest_commit: branch.target,
            created_at: None,
            last_activity: None,
            contributors: HashSet::new(),
            commit_types: HashMap::new(),
        };

        for commit in commits {
            stats.contributors.insert(commit.author.peer_id);
            *stats.commit_types.entry(format!("{:?}", commit.commit_type)).or_insert(0) += 1;

            if stats.created_at.is_none() || commit.timestamp < stats.created_at.unwrap() {
                stats.created_at = Some(commit.timestamp);
            }
            if stats.last_activity.is_none() || commit.timestamp > stats.last_activity.unwrap() {
                stats.last_activity = Some(commit.timestamp);
            }
        }

        Ok(stats)
    }

    /// Create a tag/bookmark
    pub async fn create_tag(
        &self,
        namespace: &str,
        tag_name: &str,
        target_commit: ContentHash,
        tagger: String,
        message: String,
    ) -> GitResult<ContentHash> {
        let tag = TagObject::release_tag(
            tag_name.to_string(),
            target_commit,
            ObjectType::Commit,
            CommitAuthor {
                peer_id: tagger.clone(),
                name: "Tagger".to_string(), // TODO: Get actual name
                email: None,
                timestamp: SystemTime::now(),
            },
            message,
        );

        let tag_content = bincode::serialize(&tag)
            .map_err(|e| GitContentError::SerializationError(e.to_string()))?;
        let tag_hash = self.storage.store_object(GitObject::new(
            ObjectType::Tag,
            tag_content,
            DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            tagger.clone(),
            None, // Tags don't expire
        )).await?;

        // Store tag reference
        let tag_ref = Reference::new_tag(
            tag_name.to_string(),
            tag_hash,
            namespace.to_string(),
            tagger,
        );
        self.storage.store_reference(tag_ref).await?;

        Ok(tag_hash)
    }
}

/// Repository statistics
#[derive(Debug, Clone)]
pub struct RepositoryStats {
    pub namespace: String,
    pub total_commits: usize,
    pub latest_commit: ContentHash,
    pub created_at: Option<SystemTime>,
    pub last_activity: Option<SystemTime>,
    pub contributors: std::collections::HashSet<String>,
    pub commit_types: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_dht_storage::{GitDhtStorage, MockDhtStorage};
    use std::sync::Arc;

    async fn create_test_app_layer() -> GitApplicationLayer {
        let mock_dht = Arc::new(MockDhtStorage::new("test_peer".to_string()));
        let storage = GitDhtStorage::new(mock_dht, 100, "test_peer".to_string());
        GitApplicationLayer::new(storage)
    }

    #[tokio::test]
    async fn test_send_chat_message() {
        let app_layer = create_test_app_layer().await;
        
        let commit_hash = app_layer.send_chat_message(
            "general",
            "Hello, World!",
            "alice".to_string(),
            "Alice".to_string(),
            None,
            vec![],
        ).await.unwrap();
        
        // Verify message was stored
        let history = app_layer.get_chat_history("general", 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hello, World!");
        assert_eq!(history[0].sender, "alice");
    }

    #[tokio::test]
    async fn test_forum_post_creation() {
        let app_layer = create_test_app_layer().await;
        
        let commit_hash = app_layer.create_forum_post(
            "topic_123",
            "My First Post",
            "This is the content of my post",
            "bob".to_string(),
            "Bob".to_string(),
            vec!["rust".to_string(), "p2p".to_string()],
        ).await.unwrap();
        
        // Verify post was created
        let stats = app_layer.get_repository_stats("topic_123").await.unwrap();
        assert_eq!(stats.total_commits, 1);
        assert!(stats.contributors.contains("bob"));
    }

    #[tokio::test]
    async fn test_document_operations() {
        let app_layer = create_test_app_layer().await;
        
        // Create document
        let initial_commit = app_layer.create_document(
            "doc_456",
            "My Document",
            "# Hello\n\nThis is my document.",
            DocumentFormat::Markdown,
            "charlie".to_string(),
        ).await.unwrap();
        
        // Update document
        let updated_commit = app_layer.update_document(
            "doc_456",
            "# Hello World\n\nThis is my updated document.",
            "charlie".to_string(),
            Some("Updated content".to_string()),
        ).await.unwrap();
        
        assert_ne!(initial_commit, updated_commit);
        
        // Check stats
        let stats = app_layer.get_repository_stats("doc_456").await.unwrap();
        assert_eq!(stats.total_commits, 2);
        assert!(stats.contributors.contains("charlie"));
    }

    #[tokio::test]
    async fn test_message_editing() {
        let app_layer = create_test_app_layer().await;
        
        // Send original message
        let original_commit = app_layer.send_chat_message(
            "test_channel",
            "Original message",
            "dave".to_string(),
            "Dave".to_string(),
            None,
            vec![],
        ).await.unwrap();
        
        // Edit the message
        let edit_commit = app_layer.edit_chat_message(
            "test_channel",
            original_commit,
            "Edited message",
            "dave".to_string(),
        ).await.unwrap();
        
        assert_ne!(original_commit, edit_commit);
        
        // Check history shows edit
        let history = app_layer.get_chat_history("test_channel", 10).await.unwrap();
        assert_eq!(history.len(), 2); // Original + edit
    }

    #[tokio::test]
    async fn test_tag_creation() {
        let app_layer = create_test_app_layer().await;
        
        // Create a document first
        let commit_hash = app_layer.create_document(
            "tagged_doc",
            "Document to Tag",
            "Content for tagging",
            DocumentFormat::PlainText,
            "eve".to_string(),
        ).await.unwrap();
        
        // Create a tag
        let tag_hash = app_layer.create_tag(
            "tagged_doc",
            "v1.0.0",
            commit_hash,
            "eve".to_string(),
            "First release".to_string(),
        ).await.unwrap();
        
        // Verify tag was created
        assert_ne!(tag_hash, commit_hash);
    }
}