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

//! Git Object Implementations
//!
//! This module contains the specific implementations of git-like objects:
//! - BlobObject: Raw content storage
//! - TreeObject: Hierarchical directory-like structures
//! - CommitObject: State changes with history and metadata
//! - TagObject: Named references and bookmarks
//! - Reference: Git-like branches and mutable pointers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

use crate::git_content_addressing::{ContentHash, ObjectType, GitResult, GitContentError};

/// Blob object - raw content (equivalent to git blob)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobObject {
    /// Raw content bytes
    pub content: Vec<u8>,
    /// MIME type for content type detection
    pub mime_type: Option<String>,
    /// Character encoding (e.g., "utf-8")
    pub encoding: Option<String>,
}

impl BlobObject {
    /// Create a new blob object
    pub fn new(content: Vec<u8>) -> Self {
        Self {
            content,
            mime_type: None,
            encoding: None,
        }
    }
    
    /// Create a text blob with UTF-8 encoding
    pub fn from_text(text: &str) -> Self {
        Self {
            content: text.as_bytes().to_vec(),
            mime_type: Some("text/plain".to_string()),
            encoding: Some("utf-8".to_string()),
        }
    }
    
    /// Create a blob with specified MIME type
    pub fn with_mime_type(mut self, mime_type: &str) -> Self {
        self.mime_type = Some(mime_type.to_string());
        self
    }
    
    /// Create a blob with specified encoding
    pub fn with_encoding(mut self, encoding: &str) -> Self {
        self.encoding = Some(encoding.to_string());
        self
    }
    
    /// Get content as UTF-8 string if possible
    pub fn as_string(&self) -> GitResult<String> {
        String::from_utf8(self.content.clone())
            .map_err(|e| GitContentError::SerializationError(format!("Invalid UTF-8: {}", e)))
    }
    
    /// Get content size
    pub fn size(&self) -> u64 {
        self.content.len() as u64
    }
    
    /// Check if this is a text blob
    pub fn is_text(&self) -> bool {
        self.mime_type.as_ref()
            .map(|mt| mt.starts_with("text/"))
            .unwrap_or(false)
    }
}

/// Tree object - hierarchical structure (equivalent to git tree)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeObject {
    /// Tree entries (files and subdirectories)
    pub entries: Vec<TreeEntry>,
}

/// Entry in a tree object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Entry name (filename or directory name)
    pub name: String,
    /// Content hash pointing to the object
    pub hash: ContentHash,
    /// Type of object this entry points to
    pub object_type: ObjectType,
    /// Entry mode (permissions/type)
    pub mode: EntryMode,
    /// Size of the object in bytes
    pub size: u64,
}

/// Entry modes for tree entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryMode {
    File,           // Regular file/blob
    Directory,      // Subdirectory/tree
    Executable,     // Executable file
    Symlink,        // Symbolic link
    Submodule,      // Reference to another repository/namespace
}

impl TreeObject {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    
    /// Add an entry to the tree
    pub fn add_entry(&mut self, entry: TreeEntry) {
        self.entries.push(entry);
        // Sort entries by name for consistent ordering
        self.entries.sort_by(|a, b| a.name.cmp(&b.name));
    }
    
    /// Add a blob entry
    pub fn add_blob(&mut self, name: String, hash: ContentHash, size: u64) {
        self.add_entry(TreeEntry {
            name,
            hash,
            object_type: ObjectType::Blob,
            mode: EntryMode::File,
            size,
        });
    }
    
    /// Add a subtree entry
    pub fn add_tree(&mut self, name: String, hash: ContentHash, size: u64) {
        self.add_entry(TreeEntry {
            name,
            hash,
            object_type: ObjectType::Tree,
            mode: EntryMode::Directory,
            size,
        });
    }
    
    /// Find entry by name
    pub fn find_entry(&self, name: &str) -> Option<&TreeEntry> {
        self.entries.iter().find(|e| e.name == name)
    }
    
    /// Get total size of all entries
    pub fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.size).sum()
    }
    
    /// Get number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for TreeObject {
    fn default() -> Self {
        Self::new()
    }
}

/// Commit object - state changes with history (equivalent to git commit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitObject {
    /// Root tree hash
    pub tree: ContentHash,
    /// Parent commit hashes
    pub parents: Vec<ContentHash>,
    /// Commit author information
    pub author: CommitAuthor,
    /// Committer information (can be different from author)
    pub committer: CommitAuthor,
    /// Commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: SystemTime,
    
    // P2P specific fields
    /// Application that created this commit
    pub application: String,
    /// Namespace (channel_id, project_id, etc.)
    pub namespace: String,
    /// Type of commit for application logic
    pub commit_type: CommitType,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Author/committer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAuthor {
    /// Peer ID of the author
    pub peer_id: String,
    /// Display name
    pub name: String,
    /// Email address (optional)
    pub email: Option<String>,
    /// Author timestamp
    pub timestamp: SystemTime,
}

/// Types of commits for different applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitType {
    MessageSent,        // New message in chat
    MessageEdited,      // Message edit
    MessageDeleted,     // Message deletion
    ChannelCreated,     // New channel
    ChannelUpdated,     // Channel settings change
    MemberAdded,        // User joined
    MemberRemoved,      // User left/kicked
    DocumentCreated,    // New document
    DocumentUpdated,    // Document edit
    ProjectCreated,     // New project
    Custom(String),     // Application-defined
}

impl CommitObject {
    /// Create a new commit
    pub fn new(
        tree: ContentHash,
        parents: Vec<ContentHash>,
        message: String,
        author: CommitAuthor,
        application: String,
        namespace: String,
        commit_type: CommitType,
    ) -> Self {
        Self {
            tree,
            parents,
            author: author.clone(),
            committer: author,
            message,
            timestamp: SystemTime::now(),
            application,
            namespace,
            commit_type,
            metadata: HashMap::new(),
        }
    }
    
    /// Check if this is the root commit (no parents)
    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }
    
    /// Get the main parent (first parent in merge commits)
    pub fn main_parent(&self) -> Option<&ContentHash> {
        self.parents.first()
    }
    
    /// Check if this is a merge commit (multiple parents)
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }
    
    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Tag object - named references (equivalent to git tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagObject {
    /// Tag name
    pub name: String,
    /// Hash of the target object
    pub target_hash: ContentHash,
    /// Type of the target object
    pub target_type: ObjectType,
    /// Tagger information
    pub tagger: CommitAuthor,
    /// Tag message/description
    pub message: String,
    
    // P2P specific
    /// Type of tag
    pub tag_type: TagType,
}

/// Types of tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TagType {
    Release,        // Version release
    Bookmark,       // User bookmark
    Milestone,      // Project milestone
    Branch,         // Branch head pointer
    Latest,         // Latest version pointer
}

impl TagObject {
    /// Create a new tag
    pub fn new(
        name: String,
        target_hash: ContentHash,
        target_type: ObjectType,
        tagger: CommitAuthor,
        message: String,
        tag_type: TagType,
    ) -> Self {
        Self {
            name,
            target_hash,
            target_type,
            tagger,
            message,
            tag_type,
        }
    }
    
    /// Create a release tag
    pub fn release_tag(
        name: String,
        target_hash: ContentHash,
        target_type: ObjectType,
        tagger: CommitAuthor,
        message: String,
    ) -> Self {
        Self::new(name, target_hash, target_type, tagger, message, TagType::Release)
    }
    
    /// Create a bookmark tag
    pub fn bookmark_tag(
        name: String,
        target_hash: ContentHash,
        target_type: ObjectType,
        tagger: CommitAuthor,
    ) -> Self {
        Self::new(name, target_hash, target_type, tagger, String::new(), TagType::Bookmark)
    }
}

/// Git-like references (branches, tags, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// Reference name (e.g., "main", "feature/chat", "v1.0.0")
    pub name: String,
    /// Points to commit/tag/tree hash
    pub target: ContentHash,
    /// Type of reference
    pub ref_type: ReferenceType,
    /// Namespace (e.g., "channel:general", "project:myapp")
    pub namespace: String,
    /// When this reference was last updated
    pub last_updated: SystemTime,
    /// Who updated this reference
    pub updated_by: String,
}

/// Types of references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Branch,     // Mutable pointer (HEAD of development)
    Tag,        // Immutable pointer (release, milestone)
    Head,       // Current working state
    Remote,     // Reference to remote branch
}

impl Reference {
    /// Create a new branch reference
    pub fn new_branch(
        name: String,
        target: ContentHash,
        namespace: String,
        updated_by: String,
    ) -> Self {
        Self {
            name,
            target,
            ref_type: ReferenceType::Branch,
            namespace,
            last_updated: SystemTime::now(),
            updated_by,
        }
    }
    
    /// Create a new tag reference
    pub fn new_tag(
        name: String,
        target: ContentHash,
        namespace: String,
        updated_by: String,
    ) -> Self {
        Self {
            name,
            target,
            ref_type: ReferenceType::Tag,
            namespace,
            last_updated: SystemTime::now(),
            updated_by,
        }
    }
    
    /// Update the reference target
    pub fn update(&mut self, new_target: ContentHash, updated_by: String) {
        self.target = new_target;
        self.updated_by = updated_by;
        self.last_updated = SystemTime::now();
    }
    
    /// Get full reference name (namespace:name)
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }
    
    /// Check if this is a branch
    pub fn is_branch(&self) -> bool {
        matches!(self.ref_type, ReferenceType::Branch)
    }
    
    /// Check if this is a tag
    pub fn is_tag(&self) -> bool {
        matches!(self.ref_type, ReferenceType::Tag)
    }
}

/// Branch state management (like git branches)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchState {
    /// Namespace (channel_id, project_id, etc.)
    pub namespace: String,
    /// Branch name (e.g., "main", "feature/new-ui")
    pub branch_name: String,
    /// Latest commit on this branch
    pub head_commit: ContentHash,
    /// Branch point (where this branch diverged from)
    pub base_commit: Option<ContentHash>,
    /// Upstream branch name
    pub upstream: Option<String>,
    /// Peers tracking this branch
    pub tracking: Vec<String>,
    /// Access control for this branch
    pub access_control: BranchAccess,
}

/// Access control for branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchAccess {
    /// Peers with read access
    pub read_access: Vec<String>,
    /// Peers with write access
    pub write_access: Vec<String>,
    /// Peers with admin access
    pub admin_access: Vec<String>,
    /// Whether branch is publicly readable
    pub public_read: bool,
    /// Whether changes require review
    pub require_review: bool,
}

impl BranchState {
    /// Create a new branch
    pub fn new(
        namespace: String,
        branch_name: String,
        head_commit: ContentHash,
        creator: String,
    ) -> Self {
        Self {
            namespace,
            branch_name,
            head_commit,
            base_commit: None,
            upstream: None,
            tracking: vec![creator.clone()],
            access_control: BranchAccess {
                read_access: Vec::new(),
                write_access: vec![creator.clone()],
                admin_access: vec![creator],
                public_read: false,
                require_review: false,
            },
        }
    }
    
    /// Update branch head
    pub fn update_head(&mut self, new_head: ContentHash) {
        self.head_commit = new_head;
    }
    
    /// Add tracking peer
    pub fn add_tracker(&mut self, peer_id: String) {
        if !self.tracking.contains(&peer_id) {
            self.tracking.push(peer_id);
        }
    }
    
    /// Remove tracking peer
    pub fn remove_tracker(&mut self, peer_id: &str) {
        self.tracking.retain(|p| p != peer_id);
    }
    
    /// Check if peer can read
    pub fn can_read(&self, peer_id: &str) -> bool {
        self.access_control.public_read ||
        self.access_control.read_access.contains(&peer_id.to_string()) ||
        self.access_control.write_access.contains(&peer_id.to_string()) ||
        self.access_control.admin_access.contains(&peer_id.to_string())
    }
    
    /// Check if peer can write
    pub fn can_write(&self, peer_id: &str) -> bool {
        self.access_control.write_access.contains(&peer_id.to_string()) ||
        self.access_control.admin_access.contains(&peer_id.to_string())
    }
    
    /// Check if peer is admin
    pub fn is_admin(&self, peer_id: &str) -> bool {
        self.access_control.admin_access.contains(&peer_id.to_string())
    }
}

impl Default for BranchAccess {
    fn default() -> Self {
        Self {
            read_access: Vec::new(),
            write_access: Vec::new(),
            admin_access: Vec::new(),
            public_read: false,
            require_review: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_content_addressing::ContentHash;

    #[test]
    fn test_blob_object() {
        let content = b"Hello, World!";
        let blob = BlobObject::new(content.to_vec());
        
        assert_eq!(blob.content, content);
        assert_eq!(blob.size(), content.len() as u64);
        assert_eq!(blob.mime_type, None);
        
        let text_blob = BlobObject::from_text("Hello, World!");
        assert_eq!(text_blob.as_string().unwrap(), "Hello, World!");
        assert!(text_blob.is_text());
    }
    
    #[test]
    fn test_tree_object() {
        let mut tree = TreeObject::new();
        assert!(tree.is_empty());
        
        let hash = ContentHash::from_content(b"test");
        tree.add_blob("test.txt".to_string(), hash.clone(), 4);
        
        assert_eq!(tree.entry_count(), 1);
        assert_eq!(tree.total_size(), 4);
        
        let entry = tree.find_entry("test.txt").unwrap();
        assert_eq!(entry.hash, hash);
        assert_eq!(entry.size, 4);
    }
    
    #[test]
    fn test_commit_object() {
        let tree_hash = ContentHash::from_content(b"tree content");
        let author = CommitAuthor {
            peer_id: "test_peer".to_string(),
            name: "Test User".to_string(),
            email: Some("test@example.com".to_string()),
            timestamp: SystemTime::now(),
        };
        
        let commit = CommitObject::new(
            tree_hash.clone(),
            vec![],
            "Initial commit".to_string(),
            author,
            "test_app".to_string(),
            "test_namespace".to_string(),
            CommitType::DocumentCreated,
        );
        
        assert_eq!(commit.tree, tree_hash);
        assert!(commit.is_root());
        assert!(!commit.is_merge());
        assert_eq!(commit.message, "Initial commit");
    }
    
    #[test]
    fn test_tag_object() {
        let target_hash = ContentHash::from_content(b"commit content");
        let tagger = CommitAuthor {
            peer_id: "test_peer".to_string(),
            name: "Test User".to_string(),
            email: None,
            timestamp: SystemTime::now(),
        };
        
        let tag = TagObject::release_tag(
            "v1.0.0".to_string(),
            target_hash.clone(),
            ObjectType::Commit,
            tagger,
            "First release".to_string(),
        );
        
        assert_eq!(tag.name, "v1.0.0");
        assert_eq!(tag.target_hash, target_hash);
        assert!(matches!(tag.tag_type, TagType::Release));
    }
    
    #[test]
    fn test_reference() {
        let target_hash = ContentHash::from_content(b"commit content");
        let mut reference = Reference::new_branch(
            "main".to_string(),
            target_hash.clone(),
            "test_namespace".to_string(),
            "test_peer".to_string(),
        );
        
        assert!(reference.is_branch());
        assert!(!reference.is_tag());
        assert_eq!(reference.full_name(), "test_namespace:main");
        
        let new_hash = ContentHash::from_content(b"new commit");
        reference.update(new_hash.clone(), "test_peer".to_string());
        assert_eq!(reference.target, new_hash);
    }
    
    #[test]
    fn test_branch_state() {
        let head_commit = ContentHash::from_content(b"commit");
        let mut branch = BranchState::new(
            "test_namespace".to_string(),
            "main".to_string(),
            head_commit.clone(),
            "creator".to_string(),
        );
        
        assert!(branch.can_write("creator"));
        assert!(branch.is_admin("creator"));
        assert!(!branch.can_read("other_peer"));
        
        branch.add_tracker("other_peer".to_string());
        assert!(branch.tracking.contains(&"other_peer".to_string()));
    }
}