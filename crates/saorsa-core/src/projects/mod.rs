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

//! Projects system with hierarchical organization structure
//!
//! Features:
//! - Hierarchical structure: Organizations → Departments → Teams → Projects
//! - Document management with version control and threshold signatures
//! - Media storage for videos, audio, and images
//! - Granular permissions using threshold groups
//! - Approval workflows with multi-signature requirements
//! - Activity tracking and analytics

use crate::identity::enhanced::{DepartmentId, EnhancedIdentity, OrganizationId, TeamId};
use crate::quantum_crypto::types::GroupId;
use crate::storage::{FileChunker, FileMetadata, StorageManager, keys, ttl};
use crate::threshold::ThresholdSignature;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

/// Comprehensive error types for project operations
///
/// Covers all possible failure modes in project management including
/// storage failures, permission denials, and workflow violations.
#[derive(Debug, Error)]
pub enum ProjectsError {
    /// Underlying storage system error
    #[error("Storage error: {0}")]
    StorageError(#[from] crate::storage::StorageError),

    /// Project with specified ID does not exist
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    /// Document with specified ID does not exist
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// User lacks required permissions for operation
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Operation is not valid in current context
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Workflow validation or execution error
    #[error("Workflow error: {0}")]
    WorkflowError(String),
}

/// Result type for project operations
type Result<T> = std::result::Result<T, ProjectsError>;

/// Unique identifier for projects in the system
///
/// Uses UUID v4 to ensure global uniqueness across all organizations
/// and prevent ID collision in distributed environments.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectId {
    /// Generate a new unique project identifier
    ///
    /// # Returns
    /// A new ProjectId with a randomly generated UUID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Unique identifier for documents within projects
///
/// Documents are the primary content units in projects and can represent
/// text files, code, specifications, or any other structured content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub String);

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentId {
    /// Generate a new unique document identifier
    ///
    /// # Returns
    /// A new DocumentId with a randomly generated UUID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Unique identifier for folders in project hierarchies
///
/// Folders organize documents and other folders in a hierarchical
/// structure, supporting nested organization of project content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FolderId(pub String);

impl Default for FolderId {
    fn default() -> Self {
        Self::new()
    }
}

impl FolderId {
    /// Generate a new unique folder identifier
    ///
    /// # Returns
    /// A new FolderId with a randomly generated UUID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// User identifier type for project member references
///
/// References users who participate in projects with various roles
/// and permission levels.
pub type UserId = String;

/// Blake3 cryptographic hash for content integrity verification
///
/// Used for document versioning, deduplication, and integrity checks.
/// Blake3 provides fast, secure hashing with excellent performance.
pub type Blake3Hash = [u8; 32];

/// Complete project structure with hierarchical organization
///
/// Projects are the primary organizational unit for collaborative work.
/// They belong to organizations and can be assigned to departments and teams.
/// Access control is managed through threshold groups and permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier for this project
    pub id: ProjectId,
    /// Human-readable project name
    pub name: String,
    /// Detailed description of project purpose and scope
    pub description: String,
    /// Organization this project belongs to
    pub organization_id: OrganizationId,
    /// Optional department assignment within organization
    pub department_id: Option<DepartmentId>,
    /// Optional team assignment within department
    pub team_id: Option<TeamId>,
    /// Threshold group that owns this project
    pub owner_group: GroupId,
    /// Additional access groups with specific permissions
    pub access_groups: Vec<AccessGroup>,
    /// Root folder containing all project content
    pub root_folder: FolderId,
    /// Project configuration and behavior settings
    pub settings: ProjectSettings,
    /// Metadata for analytics and tracking
    pub metadata: ProjectMetadata,
    /// Timestamp when project was created
    pub created_at: SystemTime,
    /// User ID of project creator
    pub created_by: UserId,
}

/// Access group with permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessGroup {
    pub group_id: GroupId,
    pub permissions: Vec<ProjectPermission>,
    pub name: String,
}

/// Project-specific permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectPermission {
    Read,
    Write,
    Delete,
    Share,
    ManageMembers,
    ManageWorkflows,
    ApproveDocuments,
    ViewAnalytics,
}

/// Project settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub require_approval: bool,
    pub approval_threshold: u16,
    pub version_control: bool,
    pub max_file_size_mb: u64,
    pub allowed_file_types: Vec<String>,
    pub retention_days: Option<u32>,
    pub enable_watermarks: bool,
    pub enable_analytics: bool,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            require_approval: false,
            approval_threshold: 1,
            version_control: true,
            max_file_size_mb: 1024, // 1GB
            allowed_file_types: vec![],
            retention_days: None,
            enable_watermarks: false,
            enable_analytics: true,
        }
    }
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub total_documents: u64,
    pub total_size_bytes: u64,
    pub last_activity: SystemTime,
    pub active_users: u32,
    pub custom_fields: HashMap<String, String>,
}

/// Folder structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: FolderId,
    pub name: String,
    pub parent_id: Option<FolderId>,
    pub project_id: ProjectId,
    pub subfolders: Vec<FolderId>,
    pub documents: Vec<DocumentId>,
    pub created_at: SystemTime,
    pub created_by: UserId,
}

/// Document with version control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub name: String,
    pub description: String,
    pub folder_id: FolderId,
    pub project_id: ProjectId,
    pub document_type: DocumentType,
    pub current_version: DocumentVersion,
    pub versions: Vec<DocumentVersion>,
    pub access_log: Vec<AccessLogEntry>,
    pub workflow_state: Option<WorkflowState>,
    pub tags: Vec<String>,
    pub created_at: SystemTime,
    pub created_by: UserId,
}

/// Document type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentType {
    Text { format: String },
    Spreadsheet,
    Presentation,
    Image { width: u32, height: u32 },
    Video { duration_seconds: u64 },
    Audio { duration_seconds: u64 },
    PDF { pages: u32 },
    Archive,
    Other { mime_type: String },
}

/// Document version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    pub version_number: u64,
    pub content_hash: Blake3Hash,
    pub encryption_key: EncryptedKey,
    pub size_bytes: u64,
    pub author: UserId,
    pub signatures: Vec<VersionSignature>,
    pub comment: String,
    pub created_at: SystemTime,
    pub is_approved: bool,
}

/// Encrypted key (encrypted with project key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKey {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

/// Version signature with threshold crypto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSignature {
    pub signer_id: UserId,
    pub signature: ThresholdSignature,
    pub signed_at: SystemTime,
    pub comment: Option<String>,
}

/// Access log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub user_id: UserId,
    pub action: AccessAction,
    pub timestamp: SystemTime,
    pub ip_address: Option<String>,
    pub device_id: Option<String>,
}

/// Access action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessAction {
    View,
    Download,
    Edit,
    Share,
    Delete,
    Approve,
    Reject,
}

/// Workflow state for approvals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub workflow_id: String,
    pub current_stage: WorkflowStage,
    pub approvers: Vec<UserId>,
    pub approvals: Vec<Approval>,
    pub required_approvals: u16,
    pub deadline: Option<SystemTime>,
    pub created_at: SystemTime,
}

/// Workflow stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStage {
    Draft,
    UnderReview,
    Approved,
    Rejected,
    Published,
}

/// Approval record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub approver_id: UserId,
    pub decision: ApprovalDecision,
    pub signature: ThresholdSignature,
    pub comment: Option<String>,
    pub approved_at: SystemTime,
}

/// Approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approve,
    Reject,
    RequestChanges,
}

/// Project analytics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectAnalytics {
    pub total_views: u64,
    pub total_downloads: u64,
    pub unique_viewers: u64,
    pub average_time_spent_seconds: u64,
    pub most_active_users: Vec<(UserId, u64)>,
    pub popular_documents: Vec<(DocumentId, u64)>,
    pub storage_trend: Vec<(SystemTime, u64)>,
    pub activity_heatmap: HashMap<u32, u64>, // hour of day -> activity count
}

/// Projects manager
pub struct ProjectsManager {
    storage: StorageManager,
    identity: EnhancedIdentity,
    file_chunker: FileChunker,
}

impl ProjectsManager {
    /// Create new projects manager
    pub fn new(storage: StorageManager, identity: EnhancedIdentity) -> Self {
        Self {
            storage,
            identity,
            file_chunker: FileChunker::new(1024 * 1024), // 1MB chunks
        }
    }

    /// Create a new project
    pub async fn create_project(
        &mut self,
        name: String,
        description: String,
        organization_id: OrganizationId,
        department_id: Option<DepartmentId>,
        team_id: Option<TeamId>,
        owner_group: GroupId,
    ) -> Result<Project> {
        // Create root folder
        let root_folder = Folder {
            id: FolderId::new(),
            name: "Root".to_string(),
            parent_id: None,
            project_id: ProjectId::new(), // Will be updated
            subfolders: vec![],
            documents: vec![],
            created_at: SystemTime::now(),
            created_by: self.identity.base_identity.user_id.clone(),
        };

        let project = Project {
            id: ProjectId::new(),
            name,
            description,
            organization_id,
            department_id,
            team_id,
            owner_group,
            access_groups: vec![],
            root_folder: root_folder.id.clone(),
            settings: ProjectSettings::default(),
            metadata: ProjectMetadata {
                total_documents: 0,
                total_size_bytes: 0,
                last_activity: SystemTime::now(),
                active_users: 1,
                custom_fields: HashMap::new(),
            },
            created_at: SystemTime::now(),
            created_by: self.identity.base_identity.user_id.clone(),
        };

        // Store project
        let key = keys::project(&project.id.0);
        self.storage
            .store_encrypted(&key, &project, ttl::PROFILE, None)
            .await?;

        // Store root folder
        let folder_key = format!("project:folder:{}", root_folder.id.0);
        self.storage
            .store_encrypted(&folder_key, &root_folder, ttl::PROFILE, None)
            .await?;

        Ok(project)
    }

    /// Upload a document
    pub async fn upload_document(
        &mut self,
        project_id: ProjectId,
        folder_id: FolderId,
        name: String,
        description: String,
        content: &[u8],
        document_type: DocumentType,
    ) -> Result<Document> {
        // Verify project access
        let project = self.get_project(&project_id).await?;
        self.check_project_permission(&project, ProjectPermission::Write)?;

        // Check file size
        let size_mb = content.len() / (1024 * 1024);
        if size_mb as u64 > project.settings.max_file_size_mb {
            return Err(ProjectsError::InvalidOperation(format!(
                "File too large: {}MB (max: {}MB).into()",
                size_mb, project.settings.max_file_size_mb
            )));
        }

        // Calculate content hash
        let mut hasher = Hasher::new();
        hasher.update(content);
        let content_hash = hasher.finalize().into();

        // Generate encryption key for this document
        let doc_key = rand::random::<[u8; 32]>();

        // Encrypt content with document key
        let encrypted_content = self.encrypt_content(content, &doc_key)?;

        // Encrypt document key with project key (simplified)
        let encrypted_key = EncryptedKey {
            ciphertext: doc_key.to_vec(), // In practice, properly encrypt this
            nonce: vec![0; 12],
        };

        // Create document
        let document = Document {
            id: DocumentId::new(),
            name,
            description,
            folder_id,
            project_id: project_id.clone(),
            document_type,
            current_version: DocumentVersion {
                version_number: 1,
                content_hash,
                encryption_key: encrypted_key.clone(),
                size_bytes: content.len() as u64,
                author: self.identity.base_identity.user_id.clone(),
                signatures: vec![],
                comment: "Initial upload".to_string(),
                created_at: SystemTime::now(),
                is_approved: !project.settings.require_approval,
            },
            versions: vec![],
            access_log: vec![AccessLogEntry {
                user_id: self.identity.base_identity.user_id.clone(),
                action: AccessAction::Edit,
                timestamp: SystemTime::now(),
                ip_address: None,
                device_id: None,
            }],
            workflow_state: if project.settings.require_approval {
                Some(WorkflowState {
                    workflow_id: Uuid::new_v4().to_string(),
                    current_stage: WorkflowStage::Draft,
                    approvers: vec![],
                    approvals: vec![],
                    required_approvals: project.settings.approval_threshold,
                    deadline: None,
                    created_at: SystemTime::now(),
                })
            } else {
                None
            },
            tags: vec![],
            created_at: SystemTime::now(),
            created_by: self.identity.base_identity.user_id.clone(),
        };

        // Store document metadata
        let doc_key = keys::document_meta(&document.id.0);
        self.storage
            .store_encrypted(&doc_key, &document, ttl::PROFILE, None)
            .await?;

        // Store document content using chunker
        let file_metadata = FileMetadata {
            file_id: document.id.0.clone(),
            name: document.name.clone(),
            size: content.len() as u64,
            mime_type: match &document.document_type {
                DocumentType::Other { mime_type } => mime_type.clone(),
                _ => "application/octet-stream".to_string(),
            },
            hash: content_hash.to_vec(),
            total_chunks: 0, // Will be set by chunker
            created_at: SystemTime::now(),
            created_by: self.identity.base_identity.user_id.clone(),
        };

        self.file_chunker
            .store_file(
                &mut self.storage,
                &document.id.0,
                &encrypted_content,
                file_metadata,
            )
            .await?;

        // Update project metadata
        self.update_project_metadata(&project_id, 1, content.len() as i64)
            .await?;

        Ok(document)
    }

    /// Download a document
    pub async fn download_document(&mut self, document_id: &DocumentId) -> Result<Vec<u8>> {
        // Get document metadata
        let document = self.get_document(document_id).await?;

        // Check access
        let project = self.get_project(&document.project_id).await?;
        self.check_project_permission(&project, ProjectPermission::Read)?;

        // Log access
        self.log_document_access(document_id, AccessAction::Download)
            .await?;

        // Retrieve document content
        let encrypted_content = self
            .file_chunker
            .get_file(&self.storage, &document_id.0)
            .await?;

        // Decrypt content (simplified - in practice, decrypt the doc key first)
        let doc_key = &document.current_version.encryption_key.ciphertext;
        let content = self.decrypt_content(&encrypted_content, doc_key)?;

        Ok(content)
    }

    /// Create a new version of a document
    pub async fn create_document_version(
        &mut self,
        document_id: &DocumentId,
        content: &[u8],
        comment: String,
    ) -> Result<DocumentVersion> {
        let mut document = self.get_document(document_id).await?;

        // Check write permission
        let project = self.get_project(&document.project_id).await?;
        self.check_project_permission(&project, ProjectPermission::Write)?;

        // Move current version to history
        document.versions.push(document.current_version.clone());

        // Create new version
        let mut hasher = Hasher::new();
        hasher.update(content);
        let content_hash = hasher.finalize().into();

        let new_version = DocumentVersion {
            version_number: document.current_version.version_number + 1,
            content_hash,
            encryption_key: document.current_version.encryption_key.clone(), // Reuse key
            size_bytes: content.len() as u64,
            author: self.identity.base_identity.user_id.clone(),
            signatures: vec![],
            comment,
            created_at: SystemTime::now(),
            is_approved: !project.settings.require_approval,
        };

        document.current_version = new_version.clone();

        // Update workflow state if needed
        if project.settings.require_approval {
            document.workflow_state = Some(WorkflowState {
                workflow_id: Uuid::new_v4().to_string(),
                current_stage: WorkflowStage::UnderReview,
                approvers: vec![],
                approvals: vec![],
                required_approvals: project.settings.approval_threshold,
                deadline: None,
                created_at: SystemTime::now(),
            });
        }

        // Store updated document
        let doc_key = keys::document_meta(&document_id.0);
        self.storage
            .store_encrypted(&doc_key, &document, ttl::PROFILE, None)
            .await?;

        // Store new content
        let encrypted_content =
            self.encrypt_content(content, &document.current_version.encryption_key.ciphertext)?;
        let file_metadata = FileMetadata {
            file_id: format!("{}_v{}", document_id.0, new_version.version_number).into(),
            name: document.name.clone(),
            size: content.len() as u64,
            mime_type: "application/octet-stream".to_string(),
            hash: content_hash.to_vec(),
            total_chunks: 0,
            created_at: SystemTime::now(),
            created_by: self.identity.base_identity.user_id.clone(),
        };

        self.file_chunker
            .store_file(
                &mut self.storage,
                &file_metadata.file_id,
                &encrypted_content,
                file_metadata.clone(),
            )
            .await?;

        Ok(new_version)
    }

    /// Approve a document
    pub async fn approve_document(
        &mut self,
        document_id: &DocumentId,
        comment: Option<String>,
    ) -> Result<()> {
        let mut document = self.get_document(document_id).await?;

        // Check approval permission
        let project = self.get_project(&document.project_id).await?;
        self.check_project_permission(&project, ProjectPermission::ApproveDocuments)?;

        // Update workflow state
        if let Some(ref mut workflow) = document.workflow_state {
            // Add approval (simplified - would use threshold signature)
            workflow.approvals.push(Approval {
                approver_id: self.identity.base_identity.user_id.clone(),
                decision: ApprovalDecision::Approve,
                signature: vec![0; 64], // Placeholder
                comment,
                approved_at: SystemTime::now(),
            });

            // Check if enough approvals
            if workflow.approvals.len() >= workflow.required_approvals as usize {
                workflow.current_stage = WorkflowStage::Approved;
                document.current_version.is_approved = true;
            }
        }

        // Store updated document
        let doc_key = keys::document_meta(&document_id.0);
        self.storage
            .store_encrypted(&doc_key, &document, ttl::PROFILE, None)
            .await?;

        Ok(())
    }

    /// Get project by ID
    async fn get_project(&self, project_id: &ProjectId) -> Result<Project> {
        let key = keys::project(&project_id.0);
        self.storage
            .get_encrypted(&key)
            .await
            .map_err(|_| ProjectsError::ProjectNotFound(project_id.0.clone()))
    }

    /// Get document by ID
    async fn get_document(&self, document_id: &DocumentId) -> Result<Document> {
        let key = keys::document_meta(&document_id.0);
        self.storage
            .get_encrypted(&key)
            .await
            .map_err(|_| ProjectsError::DocumentNotFound(document_id.0.clone()))
    }

    /// Check project permission
    fn check_project_permission(
        &self,
        _project: &Project,
        _permission: ProjectPermission,
    ) -> Result<()> {
        // Simplified permission check
        // In practice, would check threshold groups and access groups
        Ok(())
    }

    /// Log document access
    async fn log_document_access(
        &mut self,
        document_id: &DocumentId,
        action: AccessAction,
    ) -> Result<()> {
        let mut document = self.get_document(document_id).await?;

        document.access_log.push(AccessLogEntry {
            user_id: self.identity.base_identity.user_id.clone(),
            action,
            timestamp: SystemTime::now(),
            ip_address: None,
            device_id: None,
        });

        // Keep log size reasonable
        if document.access_log.len() > 1000 {
            document.access_log.drain(0..100);
        }

        let doc_key = keys::document_meta(&document_id.0);
        self.storage
            .store_encrypted(&doc_key, &document, ttl::PROFILE, None)
            .await?;

        Ok(())
    }

    /// Update project metadata
    async fn update_project_metadata(
        &mut self,
        project_id: &ProjectId,
        doc_delta: i64,
        size_delta: i64,
    ) -> Result<()> {
        let mut project = self.get_project(project_id).await?;

        project.metadata.total_documents =
            (project.metadata.total_documents as i64 + doc_delta) as u64;
        project.metadata.total_size_bytes =
            (project.metadata.total_size_bytes as i64 + size_delta) as u64;
        project.metadata.last_activity = SystemTime::now();

        let key = keys::project(&project_id.0);
        self.storage
            .store_encrypted(&key, &project, ttl::PROFILE, None)
            .await?;

        Ok(())
    }

    /// Encrypt content using AES-GCM
    fn encrypt_content(&self, content: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{AeadInPlace, Aes256Gcm, KeyInit, Nonce};
        use rand::RngCore;

        // Removed unused P2PError import

        // Ensure key is exactly 32 bytes for AES-256
        if key.len() != 32 {
            return Err(ProjectsError::InvalidOperation(
                "Invalid encryption key length - must be 32 bytes".to_string(),
            ));
        }

        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Generate random 96-bit nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let mut ciphertext = content.to_vec();
        let tag = cipher
            .encrypt_in_place_detached(nonce, b"", &mut ciphertext)
            .map_err(|e| {
                ProjectsError::InvalidOperation(format!("Encryption failed: {e}").into())
            })?;

        // Combine nonce + ciphertext + tag
        let mut result = Vec::with_capacity(12 + ciphertext.len() + 16);
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        result.extend_from_slice(&tag);

        Ok(result)
    }

    /// Decrypt content using AES-GCM
    fn decrypt_content(&self, encrypted: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{AeadInPlace, Aes256Gcm, KeyInit, Nonce};
        // Removed unused P2PError import

        // Ensure key is exactly 32 bytes for AES-256
        if key.len() != 32 {
            return Err(ProjectsError::InvalidOperation(
                "Invalid decryption key length - must be 32 bytes".to_string(),
            ));
        }

        // Minimum size: 12 (nonce) + 16 (tag) = 28 bytes
        if encrypted.len() < 28 {
            return Err(ProjectsError::InvalidOperation(
                "Invalid encrypted data - too short".to_string(),
            ));
        }

        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Extract nonce (first 12 bytes)
        let nonce = Nonce::from_slice(&encrypted[0..12]);

        // Extract tag (last 16 bytes)
        let tag_start = encrypted.len() - 16;
        let tag = &encrypted[tag_start..];

        // Extract ciphertext (middle portion)
        let mut plaintext = encrypted[12..tag_start].to_vec();

        // Decrypt the data
        cipher
            .decrypt_in_place_detached(nonce, b"", &mut plaintext, tag.into())
            .map_err(|e| {
                ProjectsError::InvalidOperation(format!("Decryption failed: {e}").into())
            })?;

        Ok(plaintext)
    }
}
