# Git-Like Content-Addressed DHT Storage Design

This design transforms your DHT into a **global, decentralized git-like system** where all data is content-addressed with cryptographic hashes, creating a massive distributed version control network for all types of data.

## Core Git-Like Concepts Applied to DHT

### 1. **Content-Addressed Storage (CAS) Foundation**

```rust
use sha2::{Sha256, Digest};
use blake3;

/// Content hash using BLAKE3 (faster than SHA-256, collision-resistant)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Create hash from content (like git's SHA-1, but using BLAKE3)
    pub fn from_content(data: &[u8]) -> Self {
        Self(blake3::hash(data).into())
    }
    
    /// Create hash with type prefix (like git object types)
    pub fn from_typed_content(object_type: ObjectType, data: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(object_type.as_bytes());
        hasher.update(b"\0");
        hasher.update(data);
        Self(hasher.finalize().into())
    }
    
    /// Convert to DHT key
    pub fn to_dht_key(&self) -> Key {
        Key::from_hash(self.0)
    }
    
    /// Short form for display (like git short hashes)
    pub fn short(&self) -> String {
        hex::encode(&self.0[..8])
    }
    
    /// Full hex representation
    pub fn hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// Git-like object types for different data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Blob,       // Raw content (files, messages, documents)
    Tree,       // Directory-like structure (channels, folders)
    Commit,     // State changes with history (message sends, edits)
    Tag,        // Named references (releases, bookmarks)
    Index,      // Indexes for discovery and querying
    Manifest,   // Application-specific manifests
}

impl ObjectType {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            ObjectType::Blob => b"blob",
            ObjectType::Tree => b"tree", 
            ObjectType::Commit => b"commit",
            ObjectType::Tag => b"tag",
            ObjectType::Index => b"index",
            ObjectType::Manifest => b"manifest",
        }
    }
}
```

### 2. **Git Object Model for P2P Applications**

```rust
/// Universal git-like object stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitObject {
    pub hash: ContentHash,
    pub object_type: ObjectType,
    pub size: u64,
    pub content: Vec<u8>,
    
    // P2P specific fields
    pub access_level: DataAccessLevel,
    pub created_at: SystemTime,
    pub creator: PeerId,
    
    // DHT replication metadata
    pub replication_factor: u8,  // K=8 by default
    pub ttl: Option<Duration>,
}

/// Blob object - raw content (equivalent to git blob)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobObject {
    pub content: Vec<u8>,
    pub mime_type: Option<String>,
    pub encoding: Option<String>,
}

/// Tree object - hierarchical structure (equivalent to git tree)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeObject {
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub hash: ContentHash,
    pub object_type: ObjectType,
    pub mode: EntryMode,  // Permissions/type
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryMode {
    File,           // Regular file/blob
    Directory,      // Subdirectory/tree
    Executable,     // Executable file
    Symlink,        // Symbolic link
    Submodule,      // Reference to another repository/namespace
}

/// Commit object - state changes with history (equivalent to git commit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitObject {
    pub tree: ContentHash,          // Root tree hash
    pub parents: Vec<ContentHash>,  // Parent commit hashes
    pub author: CommitAuthor,
    pub committer: CommitAuthor,
    pub message: String,
    pub timestamp: SystemTime,
    
    // P2P specific
    pub application: String,        // "chat", "forum", "docs", etc.
    pub namespace: String,          // channel_id, project_id, etc.
    pub commit_type: CommitType,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAuthor {
    pub peer_id: PeerId,
    pub name: String,
    pub email: Option<String>,
    pub timestamp: SystemTime,
}

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

/// Tag object - named references (equivalent to git tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagObject {
    pub name: String,
    pub target_hash: ContentHash,
    pub target_type: ObjectType,
    pub tagger: CommitAuthor,
    pub message: String,
    
    // P2P specific
    pub tag_type: TagType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TagType {
    Release,        // Version release
    Bookmark,       // User bookmark
    Milestone,      // Project milestone
    Branch,         // Branch head pointer
    Latest,         // Latest version pointer
}
```

### 3. **Git-Like References and Branches**

```rust
/// Git-like references (branches, tags, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub name: String,              // "main", "feature/chat", "v1.0.0"
    pub target: ContentHash,       // Points to commit/tag/tree
    pub ref_type: ReferenceType,
    pub namespace: String,         // "channel:general", "project:myapp"
    pub last_updated: SystemTime,
    pub updated_by: PeerId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Branch,     // Mutable pointer (HEAD of development)
    Tag,        // Immutable pointer (release, milestone)
    Head,       // Current working state
    Remote,     // Reference to remote branch
}

/// Branch state management (like git branches)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchState {
    pub namespace: String,          // channel_id, project_id, etc.
    pub branch_name: String,        // "main", "feature/new-ui"
    pub head_commit: ContentHash,   // Latest commit on this branch
    pub base_commit: Option<ContentHash>, // Branch point
    pub upstream: Option<String>,   // Upstream branch name
    pub tracking: Vec<PeerId>,      // Who's tracking this branch
    pub access_control: BranchAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchAccess {
    pub read_access: Vec<PeerId>,
    pub write_access: Vec<PeerId>,
    pub admin_access: Vec<PeerId>,
    pub public_read: bool,
    pub require_review: bool,        // Like GitHub protected branches
}
```

### 4. **Application-Specific Git Patterns**

#### **Chat as Git Repository**
```rust
/// Chat channel as a git-like repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRepository {
    pub channel_id: String,
    pub main_branch: BranchState,           // Main conversation thread
    pub thread_branches: Vec<BranchState>,  // Reply threads
    pub head_commit: ContentHash,           // Latest state
    pub message_history: Vec<ContentHash>,  // All message commits
    pub members: TreeObject,                // Member list as tree
    pub settings: BlobObject,               // Channel settings as blob
}

/// Message as a git commit
impl From<ChatMessage> for CommitObject {
    fn from(message: ChatMessage) -> Self {
        // Message content as blob
        let content_blob = BlobObject {
            content: message.content.into_bytes(),
            mime_type: Some("text/plain".to_string()),
            encoding: Some("utf-8".to_string()),
        };
        
        // Create tree with message blob + attachments
        let mut tree_entries = vec![
            TreeEntry {
                name: "message.txt".to_string(),
                hash: ContentHash::from_typed_content(ObjectType::Blob, &bincode::serialize(&content_blob).unwrap()),
                object_type: ObjectType::Blob,
                mode: EntryMode::File,
                size: content_blob.content.len() as u64,
            }
        ];
        
        // Add attachments to tree
        for (i, attachment) in message.attachments.iter().enumerate() {
            tree_entries.push(TreeEntry {
                name: format!("attachment_{}", i),
                hash: attachment.content_hash.clone(),
                object_type: ObjectType::Blob,
                mode: EntryMode::File,
                size: attachment.size,
            });
        }
        
        let tree = TreeObject { entries: tree_entries };
        let tree_hash = ContentHash::from_typed_content(ObjectType::Tree, &bincode::serialize(&tree).unwrap());
        
        CommitObject {
            tree: tree_hash,
            parents: message.reply_to.map(|h| vec![h]).unwrap_or_default(),
            author: CommitAuthor {
                peer_id: message.sender,
                name: message.sender_name.unwrap_or_default(),
                email: None,
                timestamp: message.timestamp,
            },
            committer: CommitAuthor {
                peer_id: message.sender,
                name: message.sender_name.unwrap_or_default(),
                email: None,
                timestamp: message.timestamp,
            },
            message: format!("Message: {}", message.content.chars().take(50).collect::<String>()),
            timestamp: message.timestamp,
            application: "chat".to_string(),
            namespace: message.channel_id,
            commit_type: CommitType::MessageSent,
            metadata: HashMap::new(),
        }
    }
}
```

#### **Forum as Git Repository**
```rust
/// Forum topic as git repository with branches for different discussion threads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumRepository {
    pub topic_id: String,
    pub main_branch: BranchState,       // Original post + direct replies
    pub discussion_branches: Vec<BranchState>, // Side discussions
    pub original_post: ContentHash,     // Root commit
    pub latest_activity: ContentHash,   // HEAD commit
    pub participants: TreeObject,       // Participant list
    pub tags: Vec<TagObject>,          // Topic tags, categories
}

/// Document editing as git repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRepository {
    pub document_id: String,
    pub main_branch: BranchState,       // Published version
    pub draft_branches: Vec<BranchState>, // User drafts
    pub revision_history: Vec<ContentHash>, // All document versions
    pub collaborators: TreeObject,      // Collaborator permissions
    pub current_version: ContentHash,   // Latest published version
}
```

### 5. **DHT Storage Integration**

```rust
/// Git-aware DHT storage manager
pub struct GitDhtStorage {
    pub dht: DHT,
    pub object_cache: LruCache<ContentHash, GitObject>,
    pub ref_cache: LruCache<String, Reference>,
    pub serialization: SerializationManager,
}

impl GitDhtStorage {
    /// Store git object with content-addressed key
    pub async fn store_object(&mut self, object: GitObject) -> Result<ContentHash> {
        let hash = object.hash.clone();
        
        // Serialize object
        let serialized = self.serialization.serialize(&object, Some(SerializationFormat::Bincode))?;
        
        // Create enhanced DHT record
        let record = EnhancedDhtRecord {
            key: hash.to_dht_key(),
            value: serialized,
            publisher: object.creator.clone(),
            created_at: object.created_at,
            expires_at: object.created_at + object.ttl.unwrap_or(Duration::from_secs(365 * 24 * 60 * 60)),
            access_level: object.access_level.clone(),
            content_type: self.object_type_to_content_type(&object.object_type),
            version_vector: VersionVector::new(),
            parent_hash: None,
            application_metadata: self.create_git_metadata(&object)?,
            integrity_proof: IntegrityProof::ContentAddressed { hash: hash.clone() },
            threshold_signatures: Vec::new(),
        };
        
        // Store with K=8 replication
        self.dht.store_secure_record(record).await?;
        
        // Cache locally
        self.object_cache.put(hash.clone(), object);
        
        Ok(hash)
    }
    
    /// Retrieve git object by content hash
    pub async fn get_object(&self, hash: &ContentHash) -> Result<Option<GitObject>> {
        // Check cache first
        if let Some(object) = self.object_cache.get(hash) {
            return Ok(Some(object.clone()));
        }
        
        // Query DHT with K=8 consistency
        let dht_key = hash.to_dht_key();
        let access_context = AccessContext::default();
        let requester = self.dht.local_id.to_hex();
        
        if let Some(record) = self.dht.get_secure_record_with_k_consistency(&dht_key, &requester, &access_context).await? {
            let object: GitObject = self.serialization.deserialize(&record.value, SerializationFormat::Bincode)?;
            
            // Verify content hash integrity
            if object.hash != *hash {
                return Err(StorageError::CorruptedData(format!("Hash mismatch: expected {}, got {}", hash.hex(), object.hash.hex())));
            }
            
            Ok(Some(object))
        } else {
            Ok(None)
        }
    }
    
    /// Store/update reference (branch, tag, etc.)
    pub async fn store_reference(&mut self, reference: Reference) -> Result<()> {
        let ref_key = format!("ref:{}:{}", reference.namespace, reference.name);
        
        // Store reference with appropriate access level
        let access_level = match reference.ref_type {
            ReferenceType::Branch | ReferenceType::Head => DataAccessLevel::GroupShared {
                encrypted_data: EncryptedData::default(),
                threshold_metadata: ThresholdEncryptionMeta::default(),
                group_id: reference.namespace.clone(),
                required_shares: 2,
            },
            ReferenceType::Tag => DataAccessLevel::Public {
                signature: MlDsaSignature::default(),
                content_hash: [0u8; 32],
            },
            ReferenceType::Remote => DataAccessLevel::UserPrivate {
                encrypted_data: EncryptedData::default(),
                ml_kem_session_key: Vec::new(),
                user_key_id: reference.updated_by.clone(),
            },
        };
        
        // Use existing storage with git-specific TTL
        self.store_with_access_control(
            &ref_key,
            &reference,
            access_level,
            ContentType::GitReference,
            Duration::from_secs(30 * 24 * 60 * 60), // 30 days for refs
        ).await?;
        
        // Cache reference
        self.ref_cache.put(ref_key, reference);
        
        Ok(())
    }
    
    /// Get reference by name
    pub async fn get_reference(&self, namespace: &str, name: &str) -> Result<Option<Reference>> {
        let ref_key = format!("ref:{}:{}", namespace, name);
        
        // Check cache
        if let Some(reference) = self.ref_cache.get(&ref_key) {
            return Ok(Some(reference.clone()));
        }
        
        // Query DHT
        let access_context = AccessContext::default();
        self.get_with_access_control(&ref_key, &access_context).await
    }
    
    /// Traverse git object tree (like git ls-tree)
    pub async fn traverse_tree(&self, tree_hash: &ContentHash) -> Result<Vec<(String, GitObject)>> {
        let tree_object = self.get_object(tree_hash).await?
            .ok_or_else(|| StorageError::KeyNotFound(tree_hash.hex()))?;
        
        if tree_object.object_type != ObjectType::Tree {
            return Err(StorageError::InvalidFormat("Expected tree object".to_string()));
        }
        
        let tree: TreeObject = bincode::deserialize(&tree_object.content)?;
        let mut results = Vec::new();
        
        for entry in tree.entries {
            if let Some(object) = self.get_object(&entry.hash).await? {
                results.push((entry.name, object));
            }
        }
        
        Ok(results)
    }
    
    /// Get commit history (like git log)
    pub async fn get_commit_history(&self, start_hash: &ContentHash, limit: usize) -> Result<Vec<CommitObject>> {
        let mut history = Vec::new();
        let mut current_hash = *start_hash;
        let mut visited = HashSet::new();
        
        while history.len() < limit && !visited.contains(&current_hash) {
            visited.insert(current_hash);
            
            let commit_object = self.get_object(&current_hash).await?
                .ok_or_else(|| StorageError::KeyNotFound(current_hash.hex()))?;
            
            if commit_object.object_type != ObjectType::Commit {
                break;
            }
            
            let commit: CommitObject = bincode::deserialize(&commit_object.content)?;
            history.push(commit.clone());
            
            // Follow first parent for linear history
            if let Some(parent_hash) = commit.parents.first() {
                current_hash = *parent_hash;
            } else {
                break;
            }
        }
        
        Ok(history)
    }
    
    /// Create new commit (like git commit)
    pub async fn create_commit(
        &mut self,
        tree: TreeObject,
        parents: Vec<ContentHash>,
        message: String,
        author: CommitAuthor,
        application: String,
        namespace: String,
        commit_type: CommitType,
    ) -> Result<ContentHash> {
        // Store tree object first
        let tree_content = bincode::serialize(&tree)?;
        let tree_hash = ContentHash::from_typed_content(ObjectType::Tree, &tree_content);
        
        let tree_object = GitObject {
            hash: tree_hash.clone(),
            object_type: ObjectType::Tree,
            size: tree_content.len() as u64,
            content: tree_content,
            access_level: DataAccessLevel::GroupShared {
                encrypted_data: EncryptedData::default(),
                threshold_metadata: ThresholdEncryptionMeta::default(),
                group_id: namespace.clone(),
                required_shares: 2,
            },
            created_at: SystemTime::now(),
            creator: author.peer_id.clone(),
            replication_factor: 8,
            ttl: Some(Duration::from_secs(365 * 24 * 60 * 60)),
        };
        
        self.store_object(tree_object).await?;
        
        // Create commit object
        let commit = CommitObject {
            tree: tree_hash,
            parents,
            author: author.clone(),
            committer: author,
            message,
            timestamp: SystemTime::now(),
            application,
            namespace,
            commit_type,
            metadata: HashMap::new(),
        };
        
        let commit_content = bincode::serialize(&commit)?;
        let commit_hash = ContentHash::from_typed_content(ObjectType::Commit, &commit_content);
        
        let commit_object = GitObject {
            hash: commit_hash.clone(),
            object_type: ObjectType::Commit,
            size: commit_content.len() as u64,
            content: commit_content,
            access_level: DataAccessLevel::GroupShared {
                encrypted_data: EncryptedData::default(),
                threshold_metadata: ThresholdEncryptionMeta::default(),
                group_id: commit.namespace.clone(),
                required_shares: 2,
            },
            created_at: commit.timestamp,
            creator: commit.author.peer_id.clone(),
            replication_factor: 8,
            ttl: Some(Duration::from_secs(365 * 24 * 60 * 60)),
        };
        
        self.store_object(commit_object).await?;
        
        Ok(commit_hash)
    }
    
    /// Update branch head (like git branch update)
    pub async fn update_branch(&mut self, namespace: &str, branch_name: &str, new_head: ContentHash) -> Result<()> {
        // Get current branch state
        let mut branch = if let Some(existing) = self.get_reference(namespace, branch_name).await? {
            existing
        } else {
            // Create new branch
            Reference {
                name: branch_name.to_string(),
                target: new_head,
                ref_type: ReferenceType::Branch,
                namespace: namespace.to_string(),
                last_updated: SystemTime::now(),
                updated_by: self.dht.local_id.to_hex(),
            }
        };
        
        // Update branch head
        branch.target = new_head;
        branch.last_updated = SystemTime::now();
        branch.updated_by = self.dht.local_id.to_hex();
        
        // Store updated reference
        self.store_reference(branch).await?;
        
        Ok(())
    }
}

// New content types for git objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    // Existing types...
    
    // Git-specific types
    GitBlob,
    GitTree,
    GitCommit,
    GitTag,
    GitReference,
    GitIndex,
    GitManifest,
}
```

### 6. **Git Operations for Applications**

```rust
/// High-level git operations for applications
pub struct GitApplicationLayer {
    storage: GitDhtStorage,
}

impl GitApplicationLayer {
    /// Send message to chat (creates git commit)
    pub async fn send_chat_message(
        &mut self,
        channel_id: &str,
        message_content: &str,
        sender: PeerId,
        sender_name: String,
        reply_to: Option<ContentHash>,
    ) -> Result<ContentHash> {
        // Create message blob
        let message_blob = BlobObject {
            content: message_content.as_bytes().to_vec(),
            mime_type: Some("text/plain".to_string()),
            encoding: Some("utf-8".to_string()),
        };
        
        // Store blob
        let blob_content = bincode::serialize(&message_blob)?;
        let blob_hash = self.storage.store_object(GitObject {
            hash: ContentHash::from_typed_content(ObjectType::Blob, &blob_content),
            object_type: ObjectType::Blob,
            size: blob_content.len() as u64,
            content: blob_content,
            access_level: DataAccessLevel::GroupShared {
                encrypted_data: EncryptedData::default(),
                threshold_metadata: ThresholdEncryptionMeta::default(),
                group_id: channel_id.to_string(),
                required_shares: 2,
            },
            created_at: SystemTime::now(),
            creator: sender.clone(),
            replication_factor: 8,
            ttl: Some(Duration::from_secs(90 * 24 * 60 * 60)),
        }).await?;
        
        // Create tree with message
        let tree = TreeObject {
            entries: vec![
                TreeEntry {
                    name: "message.txt".to_string(),
                    hash: blob_hash,
                    object_type: ObjectType::Blob,
                    mode: EntryMode::File,
                    size: message_content.len() as u64,
                }
            ],
        };
        
        // Get current branch head as parent
        let parents = if let Some(current_ref) = self.storage.get_reference(channel_id, "main").await? {
            vec![current_ref.target]
        } else {
            vec![]
        };
        
        // Create commit
        let commit_hash = self.storage.create_commit(
            tree,
            parents,
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
    pub async fn get_chat_history(&self, channel_id: &str, limit: usize) -> Result<Vec<ChatMessage>> {
        // Get branch head
        let branch = self.storage.get_reference(channel_id, "main").await?
            .ok_or_else(|| StorageError::KeyNotFound(format!("branch:{}:main", channel_id)))?;
        
        // Get commit history
        let commits = self.storage.get_commit_history(&branch.target, limit).await?;
        
        let mut messages = Vec::new();
        for commit in commits {
            // Extract message from commit tree
            if let Some(tree_object) = self.storage.get_object(&commit.tree).await? {
                let tree: TreeObject = bincode::deserialize(&tree_object.content)?;
                
                // Find message blob in tree
                for entry in tree.entries {
                    if entry.name == "message.txt" {
                        if let Some(blob_object) = self.storage.get_object(&entry.hash).await? {
                            let blob: BlobObject = bincode::deserialize(&blob_object.content)?;
                            let content = String::from_utf8(blob.content)?;
                            
                            messages.push(ChatMessage {
                                id: commit.tree.short(),
                                channel_id: channel_id.to_string(),
                                sender: commit.author.peer_id,
                                sender_name: Some(commit.author.name),
                                content,
                                timestamp: commit.timestamp,
                                reply_to: commit.parents.first().cloned(),
                                attachments: Vec::new(),
                                reactions: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(messages)
    }
    
    /// Create forum post (creates git repository)
    pub async fn create_forum_post(
        &mut self,
        topic_id: &str,
        title: &str,
        content: &str,
        author: PeerId,
        author_name: String,
    ) -> Result<ContentHash> {
        // Create post content blob
        let post_blob = BlobObject {
            content: content.as_bytes().to_vec(),
            mime_type: Some("text/markdown".to_string()),
            encoding: Some("utf-8".to_string()),
        };
        
        // Store post blob
        let blob_content = bincode::serialize(&post_blob)?;
        let blob_hash = self.storage.store_object(GitObject {
            hash: ContentHash::from_typed_content(ObjectType::Blob, &blob_content),
            object_type: ObjectType::Blob,
            size: blob_content.len() as u64,
            content: blob_content,
            access_level: DataAccessLevel::Public {
                signature: MlDsaSignature::default(),
                content_hash: [0u8; 32],
            },
            created_at: SystemTime::now(),
            creator: author.clone(),
            replication_factor: 8,
            ttl: Some(Duration::from_secs(365 * 24 * 60 * 60)),
        }).await?;
        
        // Create tree with post and metadata
        let tree = TreeObject {
            entries: vec![
                TreeEntry {
                    name: "post.md".to_string(),
                    hash: blob_hash,
                    object_type: ObjectType::Blob,
                    mode: EntryMode::File,
                    size: content.len() as u64,
                }
            ],
        };
        
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
}
```

## Benefits of Git-Like DHT Architecture

### 🔄 **Universal Version Control**
- **Every data change** is a commit with full history
- **Branching and merging** for collaborative editing
- **Content integrity** guaranteed by cryptographic hashes
- **Immutable history** with full audit trails

### 🌐 **Decentralized Collaboration**
- **No central server** - fully distributed like git
- **Offline capability** with eventual consistency
- **Conflict resolution** using git merge strategies
- **Cross-device sync** via content addressing

### 🔒 **Security & Privacy**
- **Content-addressed** prevents tampering
- **Threshold signatures** for group operations
- **Access control** at object level
- **Quantum-resistant** cryptography throughout

### 📈 **Scalability & Performance**
- **K=8 replication** for high availability
- **Content deduplication** across entire network
- **Efficient delta sync** like git
- **Local caching** of frequently accessed objects

This creates a **global, decentralized git network** where all applications share the same underlying version control infrastructure, enabling unprecedented collaboration and data integrity across your entire P2P ecosystem!