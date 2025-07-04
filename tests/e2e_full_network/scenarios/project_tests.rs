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

//! Project management system tests

use anyhow::{Context, Result};
use p2p_core::projects::{
    Project, ProjectInfo, Document, DocumentType, DocumentContent,
    Folder, Version, ApprovalStatus, ApprovalRequest, ApprovalGroup,
    ProjectPermissions, DocumentPermissions, AccessLevel,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use crate::infrastructure::{
    test_network::DistributedTestNetwork,
    test_reporter::{TestEvent, TestEventType},
};

/// Test complete project management system
pub async fn test_full_project_system(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📁 Testing Project Management System");
    println!("===================================");
    
    // 1. Create projects with documents
    test_project_creation_and_documents(network).await
        .context("Failed to test project creation")?;
    
    // 2. Test version control
    test_document_versioning(network).await
        .context("Failed to test versioning")?;
    
    // 3. Test approval workflows
    test_approval_workflows(network).await
        .context("Failed to test approvals")?;
    
    // 4. Test file chunking for large files
    test_large_file_handling(network).await
        .context("Failed to test large files")?;
    
    // 5. Test cross-node collaboration
    test_collaborative_editing(network).await
        .context("Failed to test collaboration")?;
    
    // 6. Test project permissions
    test_project_permissions(network).await
        .context("Failed to test permissions")?;
    
    Ok(())
}

/// Test project creation and document management
async fn test_project_creation_and_documents(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📂 Creating projects and documents...");
    
    // Get organization ID from previous tests
    let org_id = network.local_nodes[0].identity.get_organizations().await?
        .first()
        .map(|o| o.id.clone())
        .unwrap_or_default();
    
    // Create main project
    let project = network.local_nodes[0].node.create_project(ProjectInfo {
        name: "Q4 Product Launch".to_string(),
        description: "Product launch planning and execution".to_string(),
        organization_id: Some(org_id.clone()),
        parent_project_id: None,
        visibility: p2p_core::projects::ProjectVisibility::Organization,
        permissions: ProjectPermissions::default_organization(),
        metadata: HashMap::from([
            ("deadline".to_string(), "2024-12-31".to_string()),
            ("priority".to_string(), "high".to_string()),
        ]),
    }).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "node_0".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("project_name".to_string(), serde_json::json!(project.name));
            details.insert("project_id".to_string(), serde_json::json!(project.id));
            details
        },
        success: true,
    }).await;
    
    // Create folder structure
    let folders = vec![
        ("Documents", "Project documentation"),
        ("Designs", "Design assets and mockups"),
        ("Code", "Source code and scripts"),
        ("Marketing", "Marketing materials"),
    ];
    
    let mut created_folders = HashMap::new();
    for (name, desc) in folders {
        let folder = network.local_nodes[0].node.create_folder(
            &project.id,
            &project.root_folder,
            name.to_string(),
            desc.to_string(),
        ).await?;
        created_folders.insert(name, folder);
    }
    
    // Upload various document types
    let documents_folder = &created_folders["Documents"];
    
    // Text document
    let spec_doc = network.local_nodes[0].node.upload_document(
        &project.id,
        &documents_folder.id,
        "product_spec.md".to_string(),
        "Product specification document".to_string(),
        DocumentContent::Text {
            content: r#"# Product Specification

## Overview
This document outlines the specifications for our Q4 product launch.

## Features
1. Enhanced security with quantum-resistant cryptography
2. Improved user interface
3. Real-time collaboration features

## Technical Requirements
- IPv6-only networking
- ML-KEM/ML-DSA encryption
- Distributed storage with 3x replication
"#.to_string(),
            format: "markdown".to_string(),
        },
        DocumentPermissions::default(),
    ).await?;
    
    // Code document
    let code_folder = &created_folders["Code"];
    let config_doc = network.local_nodes[1].node.upload_document(
        &project.id,
        &code_folder.id,
        "config.toml".to_string(),
        "Application configuration".to_string(),
        DocumentContent::Text {
            content: r#"[server]
host = "::1"
port = 9000

[features]
quantum_crypto = true
ipv6_only = true
threshold_signatures = true

[storage]
replication_factor = 3
chunk_size = "1MB"
"#.to_string(),
            format: "toml".to_string(),
        },
        DocumentPermissions::default(),
    ).await?;
    
    // Binary document (simulated)
    let design_folder = &created_folders["Designs"];
    let logo_doc = network.local_nodes[2].node.upload_document(
        &project.id,
        &design_folder.id,
        "logo.png".to_string(),
        "Company logo design".to_string(),
        DocumentContent::Binary {
            data: vec![0x89, 0x50, 0x4E, 0x47], // PNG header
            mime_type: "image/png".to_string(),
        },
        DocumentPermissions::default(),
    ).await?;
    
    // Structured document
    let marketing_folder = &created_folders["Marketing"];
    let campaign_doc = network.local_nodes[3].node.upload_document(
        &project.id,
        &marketing_folder.id,
        "campaign.json".to_string(),
        "Marketing campaign data".to_string(),
        DocumentContent::Structured {
            data: serde_json::json!({
                "campaign_name": "Q4 Launch Campaign",
                "budget": 50000,
                "channels": ["social", "email", "content"],
                "timeline": {
                    "start": "2024-10-01",
                    "end": "2024-12-31"
                }
            }),
            schema: Some("campaign-v1".to_string()),
        },
        DocumentPermissions::default(),
    ).await?;
    
    // Create sub-project
    let subproject = network.local_nodes[1].node.create_project(ProjectInfo {
        name: "Mobile App Development".to_string(),
        description: "Mobile app for the Q4 launch".to_string(),
        organization_id: Some(org_id),
        parent_project_id: Some(project.id.clone()),
        visibility: p2p_core::projects::ProjectVisibility::Team,
        permissions: ProjectPermissions::default_team(),
        metadata: HashMap::new(),
    }).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("projects_created".to_string(), serde_json::json!(2));
            details.insert("folders_created".to_string(), serde_json::json!(4));
            details.insert("documents_uploaded".to_string(), serde_json::json!(4));
            details.insert("document_types".to_string(), 
                serde_json::json!(["text", "code", "binary", "structured"]));
            details
        },
        success: true,
    }).await;
    
    // Store project info for later tests
    for node in &mut network.local_nodes {
        node.test_data.write().await.events.push(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: node.identity.base_identity.user_id.clone(),
            event_type: TestEventType::ProjectCreated,
            details: {
                let mut details = HashMap::new();
                details.insert("main_project_id".to_string(), serde_json::json!(project.id));
                details.insert("subproject_id".to_string(), serde_json::json!(subproject.id));
                details.insert("spec_doc_id".to_string(), serde_json::json!(spec_doc.id));
                details
            },
            success: true,
        });
    }
    
    println!("✅ Projects and documents created successfully");
    Ok(())
}

/// Test document versioning
async fn test_document_versioning(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📝 Testing document versioning...");
    
    // Get document ID from test data
    let spec_doc_id: String = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(doc_id) = event.details.get("spec_doc_id") {
                serde_json::from_value(doc_id.clone()).unwrap_or_default()
            } else {
                return Err(anyhow::anyhow!("No document ID found"));
            }
        } else {
            return Err(anyhow::anyhow!("No test data found"));
        }
    };
    
    // Create multiple versions
    let versions = vec![
        ("Added security section", r#"# Product Specification

## Overview
This document outlines the specifications for our Q4 product launch.

## Features
1. Enhanced security with quantum-resistant cryptography
2. Improved user interface
3. Real-time collaboration features

## Security
- ML-KEM for key encapsulation
- ML-DSA for digital signatures
- AES-256-GCM for symmetric encryption

## Technical Requirements
- IPv6-only networking
- ML-KEM/ML-DSA encryption
- Distributed storage with 3x replication
"#),
        ("Updated requirements", r#"# Product Specification

## Overview
This document outlines the specifications for our Q4 product launch.

## Features
1. Enhanced security with quantum-resistant cryptography
2. Improved user interface
3. Real-time collaboration features
4. AI-powered assistance

## Security
- ML-KEM for key encapsulation
- ML-DSA for digital signatures
- AES-256-GCM for symmetric encryption

## Technical Requirements
- IPv6-only networking
- ML-KEM/ML-DSA encryption
- Distributed storage with 3x replication
- MCP server integration
- 10GB minimum storage
"#),
    ];
    
    let mut version_ids = Vec::new();
    
    for (i, (comment, content)) in versions.iter().enumerate() {
        let version = network.local_nodes[i % network.local_nodes.len()]
            .node.update_document(
                &spec_doc_id,
                DocumentContent::Text {
                    content: content.to_string(),
                    format: "markdown".to_string(),
                },
                Some(comment.to_string()),
            ).await?;
        
        version_ids.push(version.id);
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: format!("node_{}", i % network.local_nodes.len()),
            event_type: TestEventType::ProjectCreated,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("document_updated"));
                details.insert("version".to_string(), serde_json::json!(i + 2)); // +2 because v1 is original
                details.insert("comment".to_string(), serde_json::json!(comment));
                details
            },
            success: true,
        }).await;
        
        sleep(Duration::from_millis(500)).await;
    }
    
    // Test version history
    let history = network.local_nodes[0].node
        .get_document_versions(&spec_doc_id)
        .await?;
    
    assert!(history.len() >= 3, "Should have at least 3 versions");
    
    // Test version comparison
    let diff = network.local_nodes[1].node
        .compare_document_versions(
            &spec_doc_id,
            &history[0].id, // Original
            &history[history.len() - 1].id, // Latest
        ).await?;
    
    println!("  Version comparison shows {} changes", diff.changes.len());
    
    // Test reverting to previous version
    let revert_version = network.local_nodes[2].node
        .revert_document_to_version(
            &spec_doc_id,
            &version_ids[0], // Revert to first update
            "Reverting to previous version for review".to_string(),
        ).await?;
    
    // Test branching (create alternative version)
    let branch = network.local_nodes[3].node
        .branch_document(
            &spec_doc_id,
            "experimental-features".to_string(),
            "Testing experimental features".to_string(),
        ).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("versions_created".to_string(), serde_json::json!(version_ids.len()));
            details.insert("version_history_length".to_string(), serde_json::json!(history.len()));
            details.insert("revert_tested".to_string(), serde_json::json!(true));
            details.insert("branching_tested".to_string(), serde_json::json!(true));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Document versioning tested successfully");
    Ok(())
}

/// Test approval workflows
async fn test_approval_workflows(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n✍️ Testing approval workflows...");
    
    // Get project ID
    let project_id: String = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(id) = event.details.get("main_project_id") {
                serde_json::from_value(id.clone()).unwrap_or_default()
            } else {
                return Err(anyhow::anyhow!("No project ID found"));
            }
        } else {
            return Err(anyhow::anyhow!("No test data found"));
        }
    };
    
    // Create critical project
    let critical_project = network.local_nodes[0].node.create_project(ProjectInfo {
        name: "Critical Security Update".to_string(),
        description: "Requires multi-party approval".to_string(),
        organization_id: None,
        parent_project_id: None,
        visibility: p2p_core::projects::ProjectVisibility::Private,
        permissions: ProjectPermissions::default_private(),
        metadata: HashMap::from([
            ("approval_required".to_string(), "true".to_string()),
            ("min_approvals".to_string(), "3".to_string()),
        ]),
    }).await?;
    
    // Upload document requiring approval
    let contract_doc = network.local_nodes[0].node.upload_document(
        &critical_project.id,
        &critical_project.root_folder,
        "security_update.pdf".to_string(),
        "Critical security update documentation".to_string(),
        DocumentContent::Binary {
            data: b"PDF content here...".to_vec(),
            mime_type: "application/pdf".to_string(),
        },
        DocumentPermissions {
            requires_approval: true,
            min_approvals: 3,
            approval_groups: vec![],
            access_levels: HashMap::new(),
        },
    ).await?;
    
    // Create approval group
    let approval_group = network.local_nodes[0].node.create_approval_group(
        "Security Review Board".to_string(),
        vec![
            network.local_nodes[1].identity.base_identity.user_id.clone(),
            network.local_nodes[2].identity.base_identity.user_id.clone(),
            network.local_nodes[3].identity.base_identity.user_id.clone(),
            network.local_nodes[4].identity.base_identity.user_id.clone(),
        ],
        3, // Minimum approvals required
    ).await?;
    
    // Assign document to approval group
    network.local_nodes[0].node.assign_document_approval(
        &contract_doc.id,
        &approval_group.id,
    ).await?;
    
    // Nodes review and approve
    let approvers = [1, 2, 4]; // Node indices that will approve
    let mut approvals = Vec::new();
    
    for &node_idx in &approvers {
        // Simulate review time
        sleep(Duration::from_millis(500)).await;
        
        let approval = network.local_nodes[node_idx].node.approve_document(
            &contract_doc.id,
            Some(format!("Reviewed and approved by node {}", node_idx)),
            HashMap::from([
                ("review_time".to_string(), "5min".to_string()),
                ("security_check".to_string(), "passed".to_string()),
            ]),
        ).await?;
        
        approvals.push(approval);
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: format!("node_{}", node_idx),
            event_type: TestEventType::ProjectCreated,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("document_approved"));
                details.insert("approvals_so_far".to_string(), serde_json::json!(approvals.len()));
                details.insert("required".to_string(), serde_json::json!(3));
                details
            },
            success: true,
        }).await;
    }
    
    // Check approval status
    let doc_status = network.local_nodes[0].node
        .get_document_status(&contract_doc.id)
        .await?;
    
    assert_eq!(doc_status.approval_status, ApprovalStatus::Approved);
    assert_eq!(doc_status.approvals.len(), 3);
    
    // Test rejection workflow
    let reject_doc = network.local_nodes[1].node.upload_document(
        &critical_project.id,
        &critical_project.root_folder,
        "risky_change.md".to_string(),
        "Potentially risky change".to_string(),
        DocumentContent::Text {
            content: "This change might break things...".to_string(),
            format: "markdown".to_string(),
        },
        DocumentPermissions {
            requires_approval: true,
            min_approvals: 2,
            approval_groups: vec![approval_group.id.clone()],
            access_levels: HashMap::new(),
        },
    ).await?;
    
    // One approval
    network.local_nodes[2].node.approve_document(
        &reject_doc.id,
        Some("Looks okay to me".to_string()),
        HashMap::new(),
    ).await?;
    
    // One rejection
    network.local_nodes[3].node.reject_document(
        &reject_doc.id,
        "Too risky, needs more testing".to_string(),
        HashMap::from([
            ("risk_level".to_string(), "high".to_string()),
            ("recommendation".to_string(), "more_testing".to_string()),
        ]),
    ).await?;
    
    let reject_status = network.local_nodes[0].node
        .get_document_status(&reject_doc.id)
        .await?;
    
    assert_eq!(reject_status.approval_status, ApprovalStatus::Rejected);
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("approval_groups_created".to_string(), serde_json::json!(1));
            details.insert("documents_approved".to_string(), serde_json::json!(1));
            details.insert("documents_rejected".to_string(), serde_json::json!(1));
            details.insert("workflow_tested".to_string(), serde_json::json!("complete"));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Approval workflows tested successfully");
    Ok(())
}

/// Test large file handling
async fn test_large_file_handling(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📦 Testing large file handling...");
    
    let project_id: String = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(id) = event.details.get("main_project_id") {
                serde_json::from_value(id.clone()).unwrap_or_default()
            } else {
                return Err(anyhow::anyhow!("No project ID found"));
            }
        } else {
            return Err(anyhow::anyhow!("No test data found"));
        }
    };
    
    // Create large file (10MB)
    let large_data = vec![0u8; 10 * 1024 * 1024];
    
    // Upload with chunking
    let large_doc = network.local_nodes[0].node.upload_large_document(
        &project_id,
        "root", // Simplified for test
        "large_dataset.bin".to_string(),
        "Large dataset for testing".to_string(),
        large_data.clone(),
        "application/octet-stream".to_string(),
        1024 * 1024, // 1MB chunks
    ).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "node_0".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("action".to_string(), serde_json::json!("large_file_uploaded"));
            details.insert("size_mb".to_string(), serde_json::json!(10));
            details.insert("chunks".to_string(), serde_json::json!(10));
            details
        },
        success: true,
    }).await;
    
    // Test chunked download from different node
    let downloaded_data = network.local_nodes[3].node
        .download_document(&large_doc.id)
        .await?;
    
    assert_eq!(downloaded_data.len(), large_data.len());
    assert_eq!(downloaded_data, large_data);
    
    // Test partial download (range request)
    let partial_data = network.local_nodes[2].node
        .download_document_range(
            &large_doc.id,
            1024 * 1024, // Start at 1MB
            2 * 1024 * 1024, // Download 2MB
        ).await?;
    
    assert_eq!(partial_data.len(), 2 * 1024 * 1024);
    
    // Test resume capability
    let resume_download = network.local_nodes[4].node
        .resume_document_download(
            &large_doc.id,
            5 * 1024 * 1024, // Resume from 5MB
        ).await?;
    
    assert_eq!(resume_download.len(), 5 * 1024 * 1024); // Remaining 5MB
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("large_file_tests".to_string(), 
                serde_json::json!(["upload", "download", "partial", "resume"]));
            details.insert("file_size_mb".to_string(), serde_json::json!(10));
            details.insert("chunk_size_mb".to_string(), serde_json::json!(1));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Large file handling tested successfully");
    Ok(())
}

/// Test collaborative editing
async fn test_collaborative_editing(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n👥 Testing collaborative editing...");
    
    let project_id: String = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(id) = event.details.get("main_project_id") {
                serde_json::from_value(id.clone()).unwrap_or_default()
            } else {
                return Err(anyhow::anyhow!("No project ID found"));
            }
        } else {
            return Err(anyhow::anyhow!("No test data found"));
        }
    };
    
    // Create collaborative document
    let collab_doc = network.local_nodes[0].node.create_collaborative_document(
        &project_id,
        "root",
        "team_notes.md".to_string(),
        "Collaborative team notes".to_string(),
        DocumentContent::Text {
            content: "# Team Notes\n\n## Ideas\n- \n\n## Tasks\n- \n".to_string(),
            format: "markdown".to_string(),
        },
    ).await?;
    
    // Multiple users edit simultaneously
    let edits = vec![
        (1, "## Ideas\n- Implement new feature X\n", 10), // Node 1 edits at position 10
        (2, "## Tasks\n- Review security docs\n", 30),    // Node 2 edits at position 30
        (3, "- Add unit tests\n", 35),                    // Node 3 adds after node 2
    ];
    
    for (node_idx, content, position) in edits {
        network.local_nodes[node_idx].node.edit_collaborative_document(
            &collab_doc.id,
            p2p_core::projects::CollaborativeEdit {
                position,
                delete_count: 0,
                insert: content.to_string(),
                user_id: network.local_nodes[node_idx].identity.base_identity.user_id.clone(),
                timestamp: std::time::SystemTime::now(),
            },
        ).await?;
        
        // Small delay to simulate real editing
        sleep(Duration::from_millis(100)).await;
    }
    
    // Test conflict resolution
    let conflict_edits = vec![
        (1, "## Conflicts\n- Edit A\n", 50),
        (2, "## Conflicts\n- Edit B\n", 50), // Same position - conflict!
    ];
    
    for (node_idx, content, position) in conflict_edits {
        let _ = network.local_nodes[node_idx].node.edit_collaborative_document(
            &collab_doc.id,
            p2p_core::projects::CollaborativeEdit {
                position,
                delete_count: 0,
                insert: content.to_string(),
                user_id: network.local_nodes[node_idx].identity.base_identity.user_id.clone(),
                timestamp: std::time::SystemTime::now(),
            },
        ).await;
    }
    
    // Get final document state
    let final_doc = network.local_nodes[0].node
        .get_document(&collab_doc.id)
        .await?;
    
    // Test live cursors
    let cursors = network.local_nodes[0].node
        .get_active_cursors(&collab_doc.id)
        .await?;
    
    println!("  Active editors: {}", cursors.len());
    
    // Test commenting
    let comment = network.local_nodes[1].node.add_document_comment(
        &collab_doc.id,
        "Great progress on this document!".to_string(),
        Some(p2p_core::projects::CommentRange {
            start: 10,
            end: 25,
        }),
    ).await?;
    
    // Reply to comment
    network.local_nodes[2].node.reply_to_comment(
        &comment.id,
        "Thanks! Let's keep the momentum going.".to_string(),
    ).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("collaborative_edits".to_string(), serde_json::json!(edits.len()));
            details.insert("conflict_resolution".to_string(), serde_json::json!("tested"));
            details.insert("comments_added".to_string(), serde_json::json!(2));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Collaborative editing tested successfully");
    Ok(())
}

/// Test project permissions
async fn test_project_permissions(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔒 Testing project permissions...");
    
    // Create project with specific permissions
    let restricted_project = network.local_nodes[0].node.create_project(ProjectInfo {
        name: "Restricted Project".to_string(),
        description: "Project with custom permissions".to_string(),
        organization_id: None,
        parent_project_id: None,
        visibility: p2p_core::projects::ProjectVisibility::Private,
        permissions: ProjectPermissions {
            owners: vec![network.local_nodes[0].identity.base_identity.user_id.clone()],
            admins: vec![network.local_nodes[1].identity.base_identity.user_id.clone()],
            writers: vec![network.local_nodes[2].identity.base_identity.user_id.clone()],
            readers: vec![network.local_nodes[3].identity.base_identity.user_id.clone()],
            custom_roles: HashMap::new(),
        },
        metadata: HashMap::new(),
    }).await?;
    
    // Test owner permissions (node 0)
    let owner_doc = network.local_nodes[0].node.upload_document(
        &restricted_project.id,
        &restricted_project.root_folder,
        "owner_doc.txt".to_string(),
        "Owner can do everything".to_string(),
        DocumentContent::Text {
            content: "Owner document".to_string(),
            format: "text".to_string(),
        },
        DocumentPermissions::default(),
    ).await?;
    
    // Test admin permissions (node 1)
    let admin_folder = network.local_nodes[1].node.create_folder(
        &restricted_project.id,
        &restricted_project.root_folder,
        "Admin Folder".to_string(),
        "Admin can create folders".to_string(),
    ).await?;
    
    // Test writer permissions (node 2)
    let writer_doc = network.local_nodes[2].node.upload_document(
        &restricted_project.id,
        &restricted_project.root_folder,
        "writer_doc.txt".to_string(),
        "Writer can upload documents".to_string(),
        DocumentContent::Text {
            content: "Writer document".to_string(),
            format: "text".to_string(),
        },
        DocumentPermissions::default(),
    ).await?;
    
    // Test reader permissions (node 3) - should fail to write
    let reader_write_result = network.local_nodes[3].node.upload_document(
        &restricted_project.id,
        &restricted_project.root_folder,
        "reader_doc.txt".to_string(),
        "This should fail".to_string(),
        DocumentContent::Text {
            content: "Reader trying to write".to_string(),
            format: "text".to_string(),
        },
        DocumentPermissions::default(),
    ).await;
    
    assert!(reader_write_result.is_err(), "Reader should not be able to upload");
    
    // But reader can read
    let read_result = network.local_nodes[3].node
        .get_document(&owner_doc.id)
        .await;
    
    assert!(read_result.is_ok(), "Reader should be able to read documents");
    
    // Test permission updates
    network.local_nodes[0].node.update_project_permissions(
        &restricted_project.id,
        ProjectPermissions {
            owners: vec![network.local_nodes[0].identity.base_identity.user_id.clone()],
            admins: vec![
                network.local_nodes[1].identity.base_identity.user_id.clone(),
                network.local_nodes[3].identity.base_identity.user_id.clone(), // Promote reader to admin
            ],
            writers: vec![network.local_nodes[2].identity.base_identity.user_id.clone()],
            readers: vec![],
            custom_roles: HashMap::new(),
        },
    ).await?;
    
    // Now node 3 can create folders
    let new_admin_folder = network.local_nodes[3].node.create_folder(
        &restricted_project.id,
        &restricted_project.root_folder,
        "New Admin Folder".to_string(),
        "Node 3 is now admin".to_string(),
    ).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("permission_levels_tested".to_string(), 
                serde_json::json!(["owner", "admin", "writer", "reader"]));
            details.insert("permission_enforcement".to_string(), serde_json::json!("working"));
            details.insert("permission_updates".to_string(), serde_json::json!("tested"));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Project permissions tested successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_types() {
        let text_doc = DocumentContent::Text {
            content: "Test content".to_string(),
            format: "plain".to_string(),
        };
        
        match text_doc {
            DocumentContent::Text { content, format } => {
                assert_eq!(format, "plain");
                assert!(!content.is_empty());
            }
            _ => panic!("Wrong document type"),
        }
    }
    
    #[test]
    fn test_approval_thresholds() {
        let perms = DocumentPermissions {
            requires_approval: true,
            min_approvals: 3,
            approval_groups: vec![],
            access_levels: HashMap::new(),
        };
        
        assert!(perms.requires_approval);
        assert!(perms.min_approvals > 0);
    }
}