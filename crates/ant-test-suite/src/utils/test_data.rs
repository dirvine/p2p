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

//! Test data generation utilities
//!
//! Provides utilities for generating realistic test data for all
//! aspects of the P2P system, including users, messages, files,
//! and complex data structures.

use anyhow::Result;
use saorsa_core::{
    identity::manager::{UserProfile, UserPreferences, PrivacySettings, DiscoverabilitySettings, DefaultPermissions},
    chat::{ChannelId, MessageId},
    projects::{ProjectId, DocumentId},
    discuss::{CategoryId, TopicId},
    quantum_crypto::types::{GroupId, ParticipantId},
};
use fake::{Fake, Faker};
// Use basic fake generators that are available
use rand::{Rng, RngCore, SeedableRng};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Test data generator with configurable parameters
pub struct TestDataGenerator {
    /// Random number generator
    rng: StdRng,
    
    /// Seed for reproducible tests
    seed: Option<u64>,
}

impl TestDataGenerator {
    /// Create new generator with random seed
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
            seed: None,
        }
    }

    /// Create new generator with specific seed for reproducible tests
    pub fn with_seed(seed: u64) -> Self {
        use rand::SeedableRng;
        Self {
            rng: StdRng::seed_from_u64(seed),
            seed: Some(seed),
        }
    }

    /// Generate a realistic user profile
    pub fn generate_user_profile(&mut self) -> UserProfile {
        let display_name: String = format!("User{}", self.rng.gen::<u32>());
        let bio: Option<String> = if self.rng.gen_bool(0.7) {
            Some(format!("Test bio for user {}", self.rng.gen::<u16>()))
        } else {
            None
        };
        
        let mut custom_fields = HashMap::new();
        
        // Add some random custom fields
        if self.rng.gen_bool(0.5) {
            custom_fields.insert(
                "location".to_string(),
                serde_json::Value::String(format!("City{}", self.rng.gen::<u16>()))
            );
        }
        
        if self.rng.gen_bool(0.3) {
            custom_fields.insert(
                "website".to_string(),
                serde_json::Value::String(format!("https://{}.com", format!("user{}", self.rng.gen::<u16>())))
            );
        }

        UserProfile {
            user_id: Uuid::new_v4().to_string(),
            display_name,
            bio,
            avatar_url: if self.rng.gen_bool(0.6) {
                Some(format!("https://avatar.example.com/{}.jpg", Uuid::new_v4()))
            } else {
                None
            },
            avatar_hash: if self.rng.gen_bool(0.6) {
                Some(format!("{:x}", self.rng.gen::<u64>()))
            } else {
                None
            },
            status_message: if self.rng.gen_bool(0.4) {
                Some(format!("Status message {}", self.rng.gen::<u16>()))
            } else {
                None
            },
            public_key: self.generate_random_bytes(32),
            preferences: self.generate_user_preferences(),
            custom_fields,
            created_at: self.generate_past_time(Duration::from_secs(86400 * 365)), // Up to 1 year ago
            updated_at: SystemTime::now(),
        }
    }

    /// Generate realistic user preferences
    pub fn generate_user_preferences(&mut self) -> UserPreferences {
        UserPreferences {
            theme: if self.rng.gen_bool(0.6) { "dark".to_string() } else { "light".to_string() },
            language: self.random_language(),
            notifications_enabled: self.rng.gen_bool(0.8),
            auto_accept_friends: self.rng.gen_bool(0.3),
            discovery: self.generate_discoverability_settings(),
            privacy: self.generate_privacy_settings(),
            default_permissions: self.generate_default_permissions(),
        }
    }

    /// Generate discoverability settings
    pub fn generate_discoverability_settings(&mut self) -> DiscoverabilitySettings {
        DiscoverabilitySettings {
            discoverable_by_name: self.rng.gen_bool(0.7),
            discoverable_by_friends: self.rng.gen_bool(0.9),
            allow_contact_requests: self.rng.gen_bool(0.8),
            require_mutual_friends: self.rng.gen_bool(0.4),
            listed_in_directory: self.rng.gen_bool(0.5),
        }
    }

    /// Generate privacy settings
    pub fn generate_privacy_settings(&mut self) -> PrivacySettings {
        PrivacySettings {
            show_online_status: self.rng.gen_bool(0.6),
            show_last_seen: self.rng.gen_bool(0.5),
            allow_profile_view: self.rng.gen_bool(0.8),
            encrypted_messaging: self.rng.gen_bool(0.9),
            require_proof_of_humanity: self.rng.gen_bool(0.3),
            max_contact_request_age: Duration::from_secs(
                self.rng.gen_range(86400, 86400 * 90) // 1-90 days
            ),
            enable_forward_secrecy: self.rng.gen_bool(0.8),
            auto_rotate_keys: self.rng.gen_bool(0.7),
            key_rotation_interval: Duration::from_secs(
                self.rng.gen_range(86400 * 30, 86400 * 180) // 30-180 days
            ),
        }
    }

    /// Generate default permissions
    pub fn generate_default_permissions(&mut self) -> DefaultPermissions {
        DefaultPermissions {
            can_see_display_name: self.rng.gen_bool(0.9),
            can_see_avatar: self.rng.gen_bool(0.8),
            can_see_status: self.rng.gen_bool(0.7),
            can_see_contact_info: self.rng.gen_bool(0.4),
            can_see_last_seen: self.rng.gen_bool(0.5),
            can_see_custom_fields: self.rng.gen_bool(0.3),
        }
    }

    /// Generate random chat message content
    pub fn generate_chat_message(&mut self) -> TestChatMessage {
        let message_types = ["text", "emoji", "code", "quote"];
        let message_type = message_types[self.rng.gen_range(0, message_types.len())];
        
        let content = match message_type {
            "text" => format!("Test message {}", self.rng.gen::<u16>()),
            "emoji" => format!("😀 {} 🎉", format!("word{}", self.rng.gen::<u16>())),
            "code" => format!("```rust\nfn main() {{ println!(\"{}!\"); }}\n```", format!("word{}", self.rng.gen::<u16>())),
            "quote" => format!("> {}\n\n{}", format!("Test content {}", self.rng.gen::<u16>()), format!("Brief text {}", self.rng.gen::<u16>())),
            _ => format!("Test content {}", self.rng.gen::<u16>()),
        };

        TestChatMessage {
            id: MessageId::new(),
            channel_id: ChannelId::new(),
            sender_id: Uuid::new_v4().to_string(),
            content,
            message_type: message_type.to_string(),
            timestamp: self.generate_recent_time(Duration::from_secs(86400)), // Last 24 hours
            edited: self.rng.gen_bool(0.1),
            reply_to: if self.rng.gen_bool(0.2) {
                Some(MessageId::new())
            } else {
                None
            },
            attachments: if self.rng.gen_bool(0.15) {
                vec![self.generate_file_attachment()]
            } else {
                Vec::new()
            },
        }
    }

    /// Generate file attachment metadata
    pub fn generate_file_attachment(&mut self) -> TestFileAttachment {
        let file_types = [
            ("document", "pdf", "application/pdf"),
            ("image", "jpg", "image/jpeg"),
            ("image", "png", "image/png"),
            ("document", "txt", "text/plain"),
            ("archive", "zip", "application/zip"),
        ];
        
        let (category, extension, mime_type) = file_types[self.rng.gen_range(0, file_types.len())];
        let filename = format!("{}.{}", format!("word{}", self.rng.gen::<u16>()), extension);
        let size = match category {
            "image" => self.rng.gen_range(1024, 1024 * 1024 * 5), // 1KB - 5MB
            "document" => self.rng.gen_range(1024, 1024 * 1024), // 1KB - 1MB
            "archive" => self.rng.gen_range(1024 * 10, 1024 * 1024 * 50), // 10KB - 50MB
            _ => self.rng.gen_range(1024, 1024 * 1024),
        };

        TestFileAttachment {
            filename,
            content_type: mime_type.to_string(),
            size,
            hash: hex::encode(self.generate_random_bytes(32)),
            upload_url: format!("https://storage.example.com/{}", Uuid::new_v4()),
        }
    }

    /// Generate project document
    pub fn generate_project_document(&mut self) -> TestProjectDocument {
        let doc_types = ["markdown", "text", "code", "design", "specification"];
        let doc_type = doc_types[self.rng.gen_range(0, doc_types.len())];
        
        let content = match doc_type {
            "markdown" => self.generate_markdown_content(),
            "code" => self.generate_code_content(),
            "design" => self.generate_design_content(),
            "specification" => self.generate_specification_content(),
            _ => format!("Long content document {}", self.rng.gen::<u16>()),
        };

        TestProjectDocument {
            id: DocumentId::new(),
            project_id: ProjectId::new(),
            title: format!("Title {}", self.rng.gen::<u16>()),
            content,
            doc_type: doc_type.to_string(),
            version: self.rng.gen_range(1, 10),
            author_id: Uuid::new_v4().to_string(),
            created_at: self.generate_past_time(Duration::from_secs(86400 * 30)),
            updated_at: self.generate_recent_time(Duration::from_secs(86400 * 7)),
            tags: self.generate_tags(3..8),
        }
    }

    /// Generate discussion topic
    pub fn generate_discussion_topic(&mut self) -> TestDiscussionTopic {
        TestDiscussionTopic {
            id: TopicId::new(),
            category_id: CategoryId::new(),
            title: format!("Topic Title {}", self.rng.gen::<u16>()),
            content: format!("Discussion content {}", self.rng.gen::<u16>()),
            author_id: Uuid::new_v4().to_string(),
            created_at: self.generate_past_time(Duration::from_secs(86400 * 7)),
            updated_at: self.generate_recent_time(Duration::from_secs(86400)),
            view_count: self.rng.gen_range(0, 1000),
            reply_count: self.rng.gen_range(0, 50),
            vote_score: self.rng.gen_range(-10, 100),
            is_pinned: self.rng.gen_bool(0.05),
            is_locked: self.rng.gen_bool(0.02),
            tags: self.generate_tags(1..5),
        }
    }

    /// Generate binary test data of specified size
    pub fn generate_binary_data(&mut self, size: usize) -> Vec<u8> {
        let mut data = vec![0u8; size];
        self.rng.fill_bytes(&mut data);
        data
    }

    /// Generate structured test data that can be serialized
    pub fn generate_structured_data(&mut self) -> TestStructuredData {
        TestStructuredData {
            id: Uuid::new_v4(),
            name: format!("word{}", self.rng.gen::<u16>()),
            description: format!("Test bio for user {}", self.rng.gen::<u16>()),
            metadata: self.generate_metadata_map(),
            nested_data: TestNestedData {
                numbers: (0..self.rng.gen_range(5, 20)).map(|_| self.rng.gen::<f64>()).collect(),
                flags: (0..self.rng.gen_range(3, 10)).map(|_| self.rng.gen::<bool>()).collect(),
                timestamp: SystemTime::now(),
            },
            optional_field: if self.rng.gen_bool(0.6) {
                Some(format!("Brief text {}", self.rng.gen::<u16>()))
            } else {
                None
            },
        }
    }

    /// Generate threshold group test data
    pub fn generate_threshold_group_data(&mut self) -> TestThresholdGroup {
        let participant_count = self.rng.gen_range(3, 10);
        let threshold = self.rng.gen_range(2, participant_count + 1);
        
        TestThresholdGroup {
            group_id: GroupId([0u8; 32]), // TODO: Generate random bytes
            threshold,
            participants: (0..participant_count)
                .map(|i| ParticipantId(i as u16))
                .collect(),
            group_name: format!("word{}", self.rng.gen::<u16>()),
            description: format!("Group description {}", self.rng.gen::<u16>()),
            created_at: self.generate_past_time(Duration::from_secs(86400 * 30)),
            is_active: self.rng.gen_bool(0.8),
        }
    }

    // Helper methods

    fn generate_random_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; len];
        self.rng.fill_bytes(&mut bytes);
        bytes
    }

    fn generate_past_time(&mut self, max_age: Duration) -> SystemTime {
        let age_secs = self.rng.gen_range(0, max_age.as_secs());
        SystemTime::now() - Duration::from_secs(age_secs)
    }

    fn generate_recent_time(&mut self, max_age: Duration) -> SystemTime {
        let age_secs = self.rng.gen_range(0, max_age.as_secs());
        SystemTime::now() - Duration::from_secs(age_secs)
    }

    fn random_language(&mut self) -> String {
        let languages = ["en", "es", "fr", "de", "it", "pt", "ru", "zh", "ja", "ko"];
        languages[self.rng.gen_range(0, languages.len())].to_string()
    }

    fn generate_tags(&mut self, range: std::ops::Range<usize>) -> Vec<String> {
        let tag_count = self.rng.gen_range(range.start, range.end);
        (0..tag_count)
            .map(|_| format!("word{}", self.rng.gen::<u16>()))
            .collect()
    }

    fn generate_metadata_map(&mut self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        let field_count = self.rng.gen_range(2, 8);
        
        for _ in 0..field_count {
            let key: String = format!("word{}", self.rng.gen::<u16>());
            let value = match self.rng.gen_range(0, 4) {
                0 => serde_json::Value::String(format!("word{}", self.rng.gen::<u16>())),
                1 => serde_json::Value::Number(serde_json::Number::from(self.rng.gen::<i32>())),
                2 => serde_json::Value::Bool(self.rng.gen::<bool>()),
                _ => serde_json::Value::String(format!("Brief text {}", self.rng.gen::<u16>())),
            };
            map.insert(key, value);
        }
        
        map
    }

    fn generate_markdown_content(&mut self) -> String {
        format!(
            "# {}\n\n{}\n\n## {}\n\n{}\n\n- {}\n- {}\n- {}\n\n```rust\n{}\n```",
            format!("word{}", self.rng.gen::<u16>()),
            format!("Paragraph content {}", self.rng.gen::<u16>()),
            format!("word{}", self.rng.gen::<u16>()),
            format!("Description {}", self.rng.gen::<u16>()),
            format!("Item {}", self.rng.gen::<u16>()),
            format!("Item {}", self.rng.gen::<u16>()),
            format!("Item {}", self.rng.gen::<u16>()),
            "fn example() { println!(\"Hello, world!\"); }"
        )
    }

    fn generate_code_content(&mut self) -> String {
        format!(
            "fn {}() -> Result<(), Error> {{\n    // {}\n    let {} = {};\n    Ok(())\n}}",
            format!("word{}", self.rng.gen::<u16>()),
            format!("Status message {}", self.rng.gen::<u16>()),
            format!("word{}", self.rng.gen::<u16>()),
            self.rng.gen::<u32>()
        )
    }

    fn generate_design_content(&mut self) -> String {
        format!(
            "Design Document: {}\n\nObjective: {}\n\nRequirements:\n{}\n\nConstraints:\n{}",
            format!("Design title {}", self.rng.gen::<u16>()),
            format!("Design objective {}", self.rng.gen::<u16>()),
            format!("Requirements {}", self.rng.gen::<u16>()),
            format!("Constraints {}", self.rng.gen::<u16>())
        )
    }

    fn generate_specification_content(&mut self) -> String {
        format!(
            "Specification: {}\n\nVersion: {}.{}.{}\n\nDescription:\n{}\n\nAPI:\n{}",
            format!("word{}", self.rng.gen::<u16>()),
            self.rng.gen_range(0, 5),
            self.rng.gen_range(0, 10),
            self.rng.gen_range(0, 20),
            format!("API description {}", self.rng.gen::<u16>()),
            format!("Requirements {}", self.rng.gen::<u16>())
        )
    }
}

impl Default for TestDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// Test data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestChatMessage {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub sender_id: String,
    pub content: String,
    pub message_type: String,
    pub timestamp: SystemTime,
    pub edited: bool,
    pub reply_to: Option<MessageId>,
    pub attachments: Vec<TestFileAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFileAttachment {
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub hash: String,
    pub upload_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProjectDocument {
    pub id: DocumentId,
    pub project_id: ProjectId,
    pub title: String,
    pub content: String,
    pub doc_type: String,
    pub version: u32,
    pub author_id: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDiscussionTopic {
    pub id: TopicId,
    pub category_id: CategoryId,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub view_count: u32,
    pub reply_count: u32,
    pub vote_score: i32,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStructuredData {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub nested_data: TestNestedData,
    pub optional_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestNestedData {
    pub numbers: Vec<f64>,
    pub flags: Vec<bool>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestThresholdGroup {
    pub group_id: GroupId,
    pub threshold: usize,
    pub participants: Vec<ParticipantId>,
    pub group_name: String,
    pub description: String,
    pub created_at: SystemTime,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_profile() {
        let mut generator = TestDataGenerator::with_seed(12345);
        let profile = generator.generate_user_profile();
        
        assert!(!profile.display_name.is_empty());
        assert!(!profile.user_id.is_empty());
        assert_eq!(profile.public_key.len(), 32);
    }

    #[test]
    fn test_generate_chat_message() {
        let mut generator = TestDataGenerator::with_seed(12345);
        let message = generator.generate_chat_message();
        
        assert!(!message.content.is_empty());
        assert!(!message.sender_id.is_empty());
    }

    #[test]
    fn test_reproducible_generation() {
        let mut gen1 = TestDataGenerator::with_seed(12345);
        let mut gen2 = TestDataGenerator::with_seed(12345);
        
        let profile1 = gen1.generate_user_profile();
        let profile2 = gen2.generate_user_profile();
        
        assert_eq!(profile1.display_name, profile2.display_name);
        assert_eq!(profile1.public_key, profile2.public_key);
    }
}