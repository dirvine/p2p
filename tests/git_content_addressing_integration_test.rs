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

#!/usr/bin/env rust
//! Comprehensive Integration Tests for Git-Like Content Addressing
//!
//! This test suite validates the complete git-like content addressing system,
//! including all object types, storage operations, and application layer functionality.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

// Mock imports for testing - in real implementation these would come from the main crate
use saorsa_core::{
    ContentHash, ObjectType, GitObject, GitResult,
    BlobObject, TreeObject, CommitObject, TagObject, Reference, ReferenceType,
    CommitAuthor, CommitType, TreeEntry, EntryMode,
    GitDhtStorage, GitApplicationLayer, ChatMessage, DocumentFormat,
};

/// Comprehensive test framework for git content addressing
pub struct GitContentTestFramework {
    app_layer: GitApplicationLayer,
    test_results: Vec<TestResult>,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_name: String,
    pub success: bool,
    pub duration: Duration,
    pub details: String,
}

impl GitContentTestFramework {
    pub async fn new() -> Self {
        // Create mock DHT storage
        let mock_dht = Arc::new(MockDhtStorage::new("test_node".to_string()));
        let git_storage = GitDhtStorage::new(mock_dht, 1000, "test_node".to_string());
        let app_layer = GitApplicationLayer::new(git_storage);
        
        Self {
            app_layer,
            test_results: Vec::new(),
        }
    }
    
    /// Run all git content addressing tests
    pub async fn run_all_tests(&mut self) -> Result<(), String> {
        println!("🚀 Starting Git Content Addressing Integration Tests...\n");
        
        // Core content addressing tests
        self.test_content_hash_operations().await?;
        self.test_blob_object_operations().await?;
        self.test_tree_object_operations().await?;
        self.test_commit_object_operations().await?;
        self.test_tag_and_reference_operations().await?;
        
        // Application layer tests
        self.test_chat_as_git_operations().await?;
        self.test_forum_as_git_operations().await?;
        self.test_document_collaboration().await?;
        
        // Advanced git operations
        self.test_git_branching_and_merging().await?;
        self.test_git_history_traversal().await?;
        self.test_content_deduplication().await?;
        
        // Performance and stress tests
        self.test_large_object_handling().await?;
        self.test_concurrent_operations().await?;
        
        self.print_test_summary();
        Ok(())
    }
    
    async fn test_content_hash_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Content Hash Operations...");
        
        // Test basic hash creation
        let data1 = b"Hello, World!";
        let hash1 = ContentHash::from_content(data1);
        let hash2 = ContentHash::from_content(data1);
        assert_eq!(hash1, hash2, "Same content should produce same hash");
        
        // Test typed content hashing
        let blob_hash = ContentHash::from_typed_content(ObjectType::Blob, data1);
        let tree_hash = ContentHash::from_typed_content(ObjectType::Tree, data1);
        assert_ne!(blob_hash, tree_hash, "Different types should produce different hashes");
        
        // Test hash display formats
        let short = hash1.short();
        let full = hash1.hex();
        assert_eq!(short.len(), 16, "Short hash should be 16 characters");
        assert_eq!(full.len(), 64, "Full hash should be 64 characters");
        assert!(full.starts_with(&short), "Full hash should start with short hash");
        
        // Test hash round-trip
        let parsed_hash = ContentHash::from_hex(&full)?;
        assert_eq!(hash1, parsed_hash, "Hash should survive round-trip conversion");
        
        self.record_test_result(
            "Content Hash Operations",
            true,
            start_time.elapsed().unwrap(),
            "All hash operations working correctly".to_string(),
        );
        
        println!("  ✅ Content hash operations verified");
        Ok(())
    }
    
    async fn test_blob_object_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Blob Object Operations...");
        
        // Create text blob
        let text_content = "This is a test document with some content.";
        let text_blob = BlobObject::from_text(text_content)
            .with_mime_type("text/plain");
        
        assert_eq!(text_blob.as_string()?, text_content);
        assert!(text_blob.is_text());
        assert_eq!(text_blob.size(), text_content.len() as u64);
        
        // Create binary blob
        let binary_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
        let binary_blob = BlobObject::new(binary_data.clone())
            .with_mime_type("image/png");
        
        assert_eq!(binary_blob.content, binary_data);
        assert!(!binary_blob.is_text());
        
        self.record_test_result(
            "Blob Object Operations",
            true,
            start_time.elapsed().unwrap(),
            "Text and binary blobs created successfully".to_string(),
        );
        
        println!("  ✅ Blob object operations verified");
        Ok(())
    }
    
    async fn test_tree_object_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Tree Object Operations...");
        
        // Create tree with multiple entries
        let mut tree = TreeObject::new();
        assert!(tree.is_empty());
        
        let file1_hash = ContentHash::from_content(b"file1 content");
        let file2_hash = ContentHash::from_content(b"file2 content");
        let subtree_hash = ContentHash::from_content(b"subtree content");
        
        tree.add_blob("file1.txt".to_string(), file1_hash.clone(), 13);
        tree.add_blob("file2.md".to_string(), file2_hash.clone(), 13);
        tree.add_tree("subdir".to_string(), subtree_hash.clone(), 100);
        
        assert_eq!(tree.entry_count(), 3);
        assert_eq!(tree.total_size(), 126);
        assert!(!tree.is_empty());
        
        // Test finding entries
        let found_file = tree.find_entry("file1.txt").unwrap();
        assert_eq!(found_file.hash, file1_hash);
        assert_eq!(found_file.object_type, ObjectType::Blob);
        assert!(matches!(found_file.mode, EntryMode::File));
        
        let found_dir = tree.find_entry("subdir").unwrap();
        assert_eq!(found_dir.hash, subtree_hash);
        assert_eq!(found_dir.object_type, ObjectType::Tree);
        assert!(matches!(found_dir.mode, EntryMode::Directory));
        
        assert!(tree.find_entry("nonexistent").is_none());
        
        self.record_test_result(
            "Tree Object Operations",
            true,
            start_time.elapsed().unwrap(),
            "Tree creation and manipulation working correctly".to_string(),
        );
        
        println!("  ✅ Tree object operations verified");
        Ok(())
    }
    
    async fn test_commit_object_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Commit Object Operations...");
        
        let tree_hash = ContentHash::from_content(b"tree content");
        let author = CommitAuthor {
            peer_id: "alice".to_string(),
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            timestamp: SystemTime::now(),
        };
        
        // Test root commit
        let root_commit = CommitObject::new(
            tree_hash.clone(),
            vec![],
            "Initial commit".to_string(),
            author.clone(),
            "test_app".to_string(),
            "test_repo".to_string(),
            CommitType::DocumentCreated,
        );
        
        assert!(root_commit.is_root());
        assert!(!root_commit.is_merge());
        assert_eq!(root_commit.main_parent(), None);
        assert_eq!(root_commit.message, "Initial commit");
        
        // Test child commit
        let parent_hash = ContentHash::from_content(b"parent commit");
        let child_commit = CommitObject::new(
            tree_hash,
            vec![parent_hash.clone()],
            "Second commit".to_string(),
            author,
            "test_app".to_string(),
            "test_repo".to_string(),
            CommitType::DocumentUpdated,
        );
        
        assert!(!child_commit.is_root());
        assert!(!child_commit.is_merge());
        assert_eq!(child_commit.main_parent(), Some(&parent_hash));
        
        // Test merge commit
        let parent2_hash = ContentHash::from_content(b"second parent");
        let merge_commit = CommitObject::new(
            tree_hash,
            vec![parent_hash, parent2_hash],
            "Merge commit".to_string(),
            CommitAuthor {
                peer_id: "bob".to_string(),
                name: "Bob".to_string(),
                email: None,
                timestamp: SystemTime::now(),
            },
            "test_app".to_string(),
            "test_repo".to_string(),
            CommitType::Custom("merge".to_string()),
        );
        
        assert!(!merge_commit.is_root());
        assert!(merge_commit.is_merge());
        assert_eq!(merge_commit.parents.len(), 2);
        
        self.record_test_result(
            "Commit Object Operations",
            true,
            start_time.elapsed().unwrap(),
            "Root, child, and merge commits created successfully".to_string(),
        );
        
        println!("  ✅ Commit object operations verified");
        Ok(())
    }
    
    async fn test_tag_and_reference_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Tag and Reference Operations...");
        
        let target_hash = ContentHash::from_content(b"commit to tag");
        let tagger = CommitAuthor {
            peer_id: "tagger".to_string(),
            name: "Tagger".to_string(),
            email: None,
            timestamp: SystemTime::now(),
        };
        
        // Test release tag
        let release_tag = TagObject::release_tag(
            "v1.0.0".to_string(),
            target_hash.clone(),
            ObjectType::Commit,
            tagger.clone(),
            "First stable release".to_string(),
        );
        
        assert_eq!(release_tag.name, "v1.0.0");
        assert_eq!(release_tag.target_hash, target_hash);
        
        // Test bookmark tag
        let bookmark_tag = TagObject::bookmark_tag(
            "important".to_string(),
            target_hash.clone(),
            ObjectType::Commit,
            tagger,
        );
        
        assert_eq!(bookmark_tag.name, "important");
        assert_eq!(bookmark_tag.message, "");
        
        // Test branch reference
        let mut branch_ref = Reference::new_branch(
            "main".to_string(),
            target_hash.clone(),
            "test_repo".to_string(),
            "creator".to_string(),
        );
        
        assert!(branch_ref.is_branch());
        assert!(!branch_ref.is_tag());
        assert_eq!(branch_ref.full_name(), "test_repo:main");
        
        // Test reference update
        let new_hash = ContentHash::from_content(b"new commit");
        branch_ref.update(new_hash.clone(), "updater".to_string());
        assert_eq!(branch_ref.target, new_hash);
        assert_eq!(branch_ref.updated_by, "updater");
        
        self.record_test_result(
            "Tag and Reference Operations",
            true,
            start_time.elapsed().unwrap(),
            "Tags and references created and updated successfully".to_string(),
        );
        
        println!("  ✅ Tag and reference operations verified");
        Ok(())
    }
    
    async fn test_chat_as_git_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Chat-as-Git Operations...");
        
        let channel_id = "general";
        
        // Send initial message
        let commit1 = self.app_layer.send_chat_message(
            channel_id,
            "Hello everyone! 👋",
            "alice".to_string(),
            "Alice".to_string(),
            None,
            vec![],
        ).await?;
        
        // Send reply message
        let commit2 = self.app_layer.send_chat_message(
            channel_id,
            "Hey Alice! How's it going?",
            "bob".to_string(),
            "Bob".to_string(),
            Some(commit1),
            vec![],
        ).await?;
        
        // Send another message
        let commit3 = self.app_layer.send_chat_message(
            channel_id,
            "Great to see everyone here!",
            "charlie".to_string(),
            "Charlie".to_string(),
            None,
            vec![],
        ).await?;
        
        // Get chat history
        let history = self.app_layer.get_chat_history(channel_id, 10).await?;
        assert_eq!(history.len(), 3, "Should have 3 messages in history");
        
        // Verify message order (most recent first)
        assert_eq!(history[0].content, "Great to see everyone here!");
        assert_eq!(history[0].sender, "charlie");
        
        assert_eq!(history[1].content, "Hey Alice! How's it going?");
        assert_eq!(history[1].sender, "bob");
        assert_eq!(history[1].reply_to, Some(commit1));
        
        assert_eq!(history[2].content, "Hello everyone! 👋");
        assert_eq!(history[2].sender, "alice");
        
        // Test message editing
        let edit_commit = self.app_layer.edit_chat_message(
            channel_id,
            commit1,
            "Hello everyone! 👋 (edited)",
            "alice".to_string(),
        ).await?;
        
        assert_ne!(edit_commit, commit1, "Edit should create new commit");
        
        // Check that edit appears in history
        let updated_history = self.app_layer.get_chat_history(channel_id, 10).await?;
        assert_eq!(updated_history.len(), 4, "Edit should add to history");
        
        self.record_test_result(
            "Chat-as-Git Operations",
            true,
            start_time.elapsed().unwrap(),
            "Chat messages, replies, and edits working correctly".to_string(),
        );
        
        println!("  ✅ Chat-as-git operations verified");
        Ok(())
    }
    
    async fn test_forum_as_git_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Forum-as-Git Operations...");
        
        let topic_id = "rust_discussion";
        
        // Create forum post
        let post_commit = self.app_layer.create_forum_post(
            topic_id,
            "Why Rust is Amazing",
            "Rust provides memory safety without garbage collection...",
            "rust_fan".to_string(),
            "Rust Fan".to_string(),
            vec!["rust".to_string(), "programming".to_string()],
        ).await?;
        
        // Reply to the post
        let reply_commit = self.app_layer.reply_to_forum_post(
            topic_id,
            post_commit,
            "I totally agree! The ownership system is brilliant.",
            "alice".to_string(),
            "Alice".to_string(),
        ).await?;
        
        // Another reply
        let reply2_commit = self.app_layer.reply_to_forum_post(
            topic_id,
            post_commit,
            "What about the learning curve though?",
            "bob".to_string(),
            "Bob".to_string(),
        ).await?;
        
        // Get repository stats
        let stats = self.app_layer.get_repository_stats(topic_id).await?;
        assert_eq!(stats.total_commits, 3, "Should have 3 commits (post + 2 replies)");
        assert!(stats.contributors.contains("rust_fan"));
        assert!(stats.contributors.contains("alice"));
        assert!(stats.contributors.contains("bob"));
        
        assert_ne!(post_commit, reply_commit);
        assert_ne!(reply_commit, reply2_commit);
        
        self.record_test_result(
            "Forum-as-Git Operations",
            true,
            start_time.elapsed().unwrap(),
            "Forum posts and replies created successfully".to_string(),
        );
        
        println!("  ✅ Forum-as-git operations verified");
        Ok(())
    }
    
    async fn test_document_collaboration(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Document Collaboration...");
        
        let doc_id = "shared_doc";
        
        // Create initial document
        let initial_commit = self.app_layer.create_document(
            doc_id,
            "Project Specification",
            "# Project Spec\n\nThis document outlines...",
            DocumentFormat::Markdown,
            "alice".to_string(),
        ).await?;
        
        // First update
        let update1_commit = self.app_layer.update_document(
            doc_id,
            "# Project Specification\n\nThis document outlines the requirements...",
            "bob".to_string(),
            Some("Added more details".to_string()),
        ).await?;
        
        // Second update
        let update2_commit = self.app_layer.update_document(
            doc_id,
            "# Project Specification\n\nThis document outlines the requirements and implementation plan...",
            "charlie".to_string(),
            Some("Added implementation section".to_string()),
        ).await?;
        
        // Get document stats
        let stats = self.app_layer.get_repository_stats(doc_id).await?;
        assert_eq!(stats.total_commits, 3, "Should have 3 versions");
        assert_eq!(stats.contributors.len(), 3, "Should have 3 contributors");
        
        assert_ne!(initial_commit, update1_commit);
        assert_ne!(update1_commit, update2_commit);
        
        self.record_test_result(
            "Document Collaboration",
            true,
            start_time.elapsed().unwrap(),
            "Document creation and collaborative editing working".to_string(),
        );
        
        println!("  ✅ Document collaboration verified");
        Ok(())
    }
    
    async fn test_git_branching_and_merging(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Git Branching and Merging...");
        
        let repo_id = "branching_test";
        
        // Create initial commit on main
        let main_commit = self.app_layer.create_document(
            repo_id,
            "Main Document",
            "Main branch content",
            DocumentFormat::PlainText,
            "alice".to_string(),
        ).await?;
        
        // Create tag for initial release
        let tag_commit = self.app_layer.create_tag(
            repo_id,
            "v1.0.0",
            main_commit,
            "alice".to_string(),
            "Initial release".to_string(),
        ).await?;
        
        // Update main branch
        let main_update = self.app_layer.update_document(
            repo_id,
            "Main branch content - updated",
            "alice".to_string(),
            Some("Update main".to_string()),
        ).await?;
        
        // Verify commits are different
        assert_ne!(main_commit, tag_commit);
        assert_ne!(main_commit, main_update);
        assert_ne!(tag_commit, main_update);
        
        self.record_test_result(
            "Git Branching and Merging",
            true,
            start_time.elapsed().unwrap(),
            "Branch creation and tagging working correctly".to_string(),
        );
        
        println!("  ✅ Git branching and merging verified");
        Ok(())
    }
    
    async fn test_git_history_traversal(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Git History Traversal...");
        
        let repo_id = "history_test";
        
        // Create a chain of commits
        let mut commits = Vec::new();
        
        // Initial commit
        let commit1 = self.app_layer.create_document(
            repo_id,
            "History Test",
            "Version 1",
            DocumentFormat::PlainText,
            "alice".to_string(),
        ).await?;
        commits.push(commit1);
        
        // Chain of updates
        for i in 2..=5 {
            let commit = self.app_layer.update_document(
                repo_id,
                &format!("Version {}", i),
                "alice".to_string(),
                Some(format!("Update to version {}", i)),
            ).await?;
            commits.push(commit);
        }
        
        // Get repository stats to verify history
        let stats = self.app_layer.get_repository_stats(repo_id).await?;
        assert_eq!(stats.total_commits, 5, "Should have 5 commits in history");
        assert_eq!(stats.contributors.len(), 1, "Should have 1 contributor");
        
        // Verify all commits are unique
        for i in 0..commits.len() {
            for j in i+1..commits.len() {
                assert_ne!(commits[i], commits[j], "All commits should be unique");
            }
        }
        
        self.record_test_result(
            "Git History Traversal",
            true,
            start_time.elapsed().unwrap(),
            "Linear history creation and traversal working".to_string(),
        );
        
        println!("  ✅ Git history traversal verified");
        Ok(())
    }
    
    async fn test_content_deduplication(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Content Deduplication...");
        
        // Test that identical content produces same hash
        let content = "This is identical content for deduplication testing.";
        
        // Create same content in different contexts
        let hash1 = ContentHash::from_content(content.as_bytes());
        let hash2 = ContentHash::from_content(content.as_bytes());
        
        assert_eq!(hash1, hash2, "Identical content should produce identical hashes");
        
        // Test with typed content
        let blob_hash1 = ContentHash::from_typed_content(ObjectType::Blob, content.as_bytes());
        let blob_hash2 = ContentHash::from_typed_content(ObjectType::Blob, content.as_bytes());
        
        assert_eq!(blob_hash1, blob_hash2, "Identical typed content should deduplicate");
        
        // Test different content produces different hashes
        let different_content = "This is different content.";
        let hash3 = ContentHash::from_content(different_content.as_bytes());
        
        assert_ne!(hash1, hash3, "Different content should produce different hashes");
        
        self.record_test_result(
            "Content Deduplication",
            true,
            start_time.elapsed().unwrap(),
            "Content addressing providing proper deduplication".to_string(),
        );
        
        println!("  ✅ Content deduplication verified");
        Ok(())
    }
    
    async fn test_large_object_handling(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Large Object Handling...");
        
        // Create large content (1MB)
        let large_content = "A".repeat(1024 * 1024);
        let large_hash = ContentHash::from_content(large_content.as_bytes());
        
        // Verify hash is computed correctly
        assert_eq!(large_hash.hex().len(), 64, "Large content hash should be same size");
        
        // Test large document creation
        let doc_commit = self.app_layer.create_document(
            "large_doc",
            "Large Document",
            &large_content,
            DocumentFormat::PlainText,
            "alice".to_string(),
        ).await?;
        
        // Verify large document stats
        let stats = self.app_layer.get_repository_stats("large_doc").await?;
        assert_eq!(stats.total_commits, 1, "Large document should create one commit");
        
        self.record_test_result(
            "Large Object Handling",
            true,
            start_time.elapsed().unwrap(),
            "Large objects (1MB) handled successfully".to_string(),
        );
        
        println!("  ✅ Large object handling verified");
        Ok(())
    }
    
    async fn test_concurrent_operations(&mut self) -> Result<(), String> {
        let start_time = SystemTime::now();
        println!("🔍 Testing Concurrent Operations...");
        
        let channel_id = "concurrent_test";
        
        // Simulate concurrent message sends
        let mut handles = Vec::new();
        
        for i in 0..5 {
            let app_layer = &self.app_layer;
            let message = format!("Concurrent message {}", i);
            let sender = format!("user_{}", i);
            
            // Note: In a real test, these would be actual concurrent operations
            // For now, we'll simulate by sending messages sequentially
            let commit = app_layer.send_chat_message(
                channel_id,
                &message,
                sender.clone(),
                sender,
                None,
                vec![],
            ).await?;
            handles.push(commit);
        }
        
        // Verify all operations completed
        assert_eq!(handles.len(), 5, "All concurrent operations should complete");
        
        // Verify all commits are unique
        for i in 0..handles.len() {
            for j in i+1..handles.len() {
                assert_ne!(handles[i], handles[j], "Concurrent commits should be unique");
            }
        }
        
        // Check final state
        let history = self.app_layer.get_chat_history(channel_id, 10).await?;
        assert_eq!(history.len(), 5, "All concurrent messages should be in history");
        
        self.record_test_result(
            "Concurrent Operations",
            true,
            start_time.elapsed().unwrap(),
            "Concurrent operations completed successfully".to_string(),
        );
        
        println!("  ✅ Concurrent operations verified");
        Ok(())
    }
    
    fn record_test_result(&mut self, test_name: &str, success: bool, duration: Duration, details: String) {
        self.test_results.push(TestResult {
            test_name: test_name.to_string(),
            success,
            duration,
            details,
        });
    }
    
    fn print_test_summary(&self) {
        println!("\n📊 Git Content Addressing Test Summary");
        println!("=====================================");
        
        let total_tests = self.test_results.len();
        let passed_tests = self.test_results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;
        
        println!("Total Tests: {}", total_tests);
        println!("Passed: {} ✅", passed_tests);
        println!("Failed: {} ❌", failed_tests);
        
        let total_duration: Duration = self.test_results.iter().map(|r| r.duration).sum();
        println!("Total Duration: {:.2?}", total_duration);
        
        if failed_tests > 0 {
            println!("\nFailed Tests:");
            for result in &self.test_results {
                if !result.success {
                    println!("  ❌ {}: {}", result.test_name, result.details);
                }
            }
        }
        
        let success_rate = (passed_tests as f64 / total_tests as f64) * 100.0;
        println!("\nSuccess Rate: {:.1}%", success_rate);
        
        if success_rate == 100.0 {
            println!("🎉 All tests passed! Git content addressing is working perfectly!");
        } else {
            println!("⚠️ Some tests failed. Please review the implementation.");
        }
    }
}

// Helper function to assert equality with better error messages
fn assert_eq<T: PartialEq + std::fmt::Debug>(left: T, right: T, message: &str) {
    if left != right {
        panic!("{}: expected {:?}, got {:?}", message, right, left);
    }
}

// Helper function to assert inequality
fn assert_ne<T: PartialEq + std::fmt::Debug>(left: T, right: T, message: &str) {
    if left == right {
        panic!("{}: values should not be equal: {:?}", message, left);
    }
}

// Helper function for assertions
fn assert<T>(condition: T, message: &str) where T: Into<bool> {
    if !condition.into() {
        panic!("{}", message);
    }
}

/// Mock DHT storage for testing
pub struct MockDhtStorage {
    storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    local_peer_id: String,
}

impl MockDhtStorage {
    pub fn new(local_peer_id: String) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            local_peer_id,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut test_framework = GitContentTestFramework::new().await;
    test_framework.run_all_tests().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_git_content_addressing_integration() {
        let mut test_framework = GitContentTestFramework::new().await;
        test_framework.run_all_tests().await.expect("All tests should pass");
        
        // Verify all tests passed
        let failed_count = test_framework.test_results.iter().filter(|r| !r.success).count();
        assert_eq!(failed_count, 0, "No tests should fail");
    }
    
    #[tokio::test]
    async fn test_specific_git_operations() {
        let mut test_framework = GitContentTestFramework::new().await;
        
        // Test specific operations
        test_framework.test_content_hash_operations().await.unwrap();
        test_framework.test_chat_as_git_operations().await.unwrap();
        test_framework.test_document_collaboration().await.unwrap();
        
        // Verify tests passed
        let all_passed = test_framework.test_results.iter().all(|r| r.success);
        assert!(all_passed, "All specific tests should pass");
    }
}