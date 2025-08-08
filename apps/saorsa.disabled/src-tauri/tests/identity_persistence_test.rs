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

//! Test identity creation and serialization

use ed25519_dalek::Keypair;
use saorsa_core::identity::manager::{
    DefaultPermissions, DiscoverabilitySettings, PrivacySettings, UserIdentity, UserPreferences,
    UserProfile, VerificationLevel,
};
use std::time::SystemTime;
use tokio;

#[tokio::test]
async fn test_identity_creation() -> anyhow::Result<()> {
    // Create test identity
    let keypair = Keypair::generate(&mut rand::rngs::OsRng);
    let identity = UserIdentity {
        user_id: "test_user_123".to_string(),
        public_key: keypair.public.to_bytes().to_vec(),
        display_name_hint: "Test User".to_string(),
        three_word_address: "apple-banana-cherry".to_string(),
        created_at: SystemTime::now(),
        version: 1,
        verification_level: VerificationLevel::SelfSigned,
    };

    // Verify identity fields
    assert!(!identity.user_id.is_empty());
    assert!(!identity.public_key.is_empty());
    assert_eq!(identity.display_name_hint, "Test User");
    assert_eq!(identity.three_word_address, "apple-banana-cherry");
    assert_eq!(identity.version, 1);

    Ok(())
}

#[tokio::test]
async fn test_user_profile_creation() -> anyhow::Result<()> {
    let keypair = Keypair::generate(&mut rand::rngs::OsRng);

    let profile = UserProfile {
        user_id: "test_user_123".to_string(),
        display_name: "Test User".to_string(),
        bio: Some("A test user profile".to_string()),
        avatar_url: None,
        avatar_hash: None,
        status_message: Some("Testing persistence".to_string()),
        public_key: keypair.public.to_bytes().to_vec(),
        preferences: UserPreferences {
            theme: "dark".to_string(),
            language: "en".to_string(),
            notifications_enabled: true,
            auto_accept_friends: false,
            discovery: DiscoverabilitySettings {
                discoverable_by_name: true,
                discoverable_by_friends: true,
                allow_contact_requests: true,
                require_mutual_friends: false,
                listed_in_directory: false,
            },
            privacy: PrivacySettings::default(),
            default_permissions: DefaultPermissions::default(),
        },
        custom_fields: Default::default(),
        created_at: SystemTime::now(),
        updated_at: SystemTime::now(),
    };

    // Verify profile fields
    assert_eq!(profile.user_id, "test_user_123");
    assert_eq!(profile.display_name, "Test User");
    assert_eq!(profile.bio, Some("A test user profile".to_string()));
    assert_eq!(
        profile.status_message,
        Some("Testing persistence".to_string())
    );
    assert!(!profile.public_key.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_identity_serialization() -> anyhow::Result<()> {
    let keypair = Keypair::generate(&mut rand::rngs::OsRng);
    let identity = UserIdentity {
        user_id: "test_user_456".to_string(),
        public_key: keypair.public.to_bytes().to_vec(),
        display_name_hint: "Serialization Test".to_string(),
        three_word_address: "dog-cat-mouse".to_string(),
        created_at: SystemTime::now(),
        version: 1,
        verification_level: VerificationLevel::SelfSigned,
    };

    // Test serialization to JSON
    let serialized = serde_json::to_string(&identity)?;
    assert!(!serialized.is_empty());

    // Test deserialization from JSON
    let deserialized: UserIdentity = serde_json::from_str(&serialized)?;
    assert_eq!(identity.user_id, deserialized.user_id);
    assert_eq!(identity.public_key, deserialized.public_key);
    assert_eq!(identity.display_name_hint, deserialized.display_name_hint);
    assert_eq!(identity.three_word_address, deserialized.three_word_address);

    Ok(())
}
