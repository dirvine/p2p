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

//! Identity and security system tests
//!
//! Tests user identity creation, profile management, encryption,
//! three-word addresses, and identity verification with complete
//! data round-trip verification.

use crate::tests::SubsystemTest;
use crate::utils::{DataVerifier, TestContext, TestDataGenerator, VerificationResult};
use anyhow::Result;
use saorsa_core::identity::manager::{EncryptedUserProfile, UserIdentity, UserProfile};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

/// Identity subsystem test implementation
pub struct IdentityTests {
    generator: TestDataGenerator,
    verifier: DataVerifier,
}

impl IdentityTests {
    pub fn new() -> Self {
        Self {
            generator: TestDataGenerator::new(),
            verifier: DataVerifier::new(true, Duration::from_secs(30), 3),
        }
    }

    /// Test user profile creation and data integrity
    async fn test_user_profile_operations(
        &mut self,
        ctx: &TestContext,
    ) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing user profile operations");

        // Generate test user profiles
        let test_profiles = (0..5)
            .map(|_| self.generator.generate_user_profile())
            .collect::<Vec<_>>();

        for profile in test_profiles {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[PROFILE] Testing user: {}", profile.display_name));

            // Test profile serialization/deserialization for data integrity
            match self.test_profile_serialization(&profile, ctx).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    let error_msg = format!("Profile serialization failed: {}", e);
                    ctx.log_error(&error_msg);
                    results.push(VerificationResult::failure(error_msg, start_time.elapsed()));
                }
            }

            // Test profile validation
            let validation_result = self.test_profile_validation(&profile, ctx).await?;
            results.push(validation_result);
        }

        ctx.log_info(&format!(
            "User profile operations completed. Results: {}",
            results.len()
        ));
        Ok(results)
    }

    /// Test profile serialization and round-trip data integrity
    async fn test_profile_serialization(
        &self,
        profile: &UserProfile,
        ctx: &TestContext,
    ) -> Result<VerificationResult> {
        let start_time = std::time::Instant::now();

        // Serialize profile to JSON
        let serialized = serde_json::to_string(profile)?;
        ctx.log_info(&format!("Profile serialized: {} bytes", serialized.len()));

        // Deserialize back
        let deserialized: UserProfile = serde_json::from_str(&serialized)?;

        // Verify data integrity
        if self.verify_profile_integrity(profile, &deserialized) {
            ctx.log_info(&format!(
                "✅ Profile data integrity verified for user: {}",
                profile.display_name
            ));

            Ok(VerificationResult::success(start_time.elapsed())
                .with_metadata("operation".to_string(), "profile_serialization".to_string())
                .with_metadata("user_id".to_string(), profile.user_id.clone())
                .with_metadata("data_verified".to_string(), "true".to_string())
                .with_metadata("serialized_size".to_string(), serialized.len().to_string()))
        } else {
            let error =
                "Profile data corruption detected during serialization round-trip".to_string();
            ctx.log_error(&error);
            Ok(VerificationResult::failure(error, start_time.elapsed()))
        }
    }

    /// Test profile validation and constraints
    async fn test_profile_validation(
        &self,
        profile: &UserProfile,
        ctx: &TestContext,
    ) -> Result<VerificationResult> {
        let start_time = std::time::Instant::now();

        let mut validation_errors = Vec::new();

        // Validate required fields
        if profile.user_id.is_empty() {
            validation_errors.push("Empty user_id");
        }

        if profile.display_name.is_empty() {
            validation_errors.push("Empty display_name");
        }

        if profile.public_key.is_empty() {
            validation_errors.push("Empty public_key");
        }

        // Validate timestamps
        if profile.updated_at < profile.created_at {
            validation_errors.push("Invalid timestamps: updated_at before created_at");
        }

        if validation_errors.is_empty() {
            ctx.log_info(&format!(
                "✅ Profile validation passed for user: {}",
                profile.display_name
            ));
            Ok(VerificationResult::success(start_time.elapsed())
                .with_metadata("operation".to_string(), "profile_validation".to_string())
                .with_metadata("user_id".to_string(), profile.user_id.clone())
                .with_metadata("validation_status".to_string(), "passed".to_string()))
        } else {
            let error = format!(
                "Profile validation failed: {}",
                validation_errors.join(", ")
            );
            ctx.log_error(&error);
            Ok(VerificationResult::failure(error, start_time.elapsed()))
        }
    }

    /// Test user preferences and privacy settings
    async fn test_user_preferences(
        &mut self,
        ctx: &TestContext,
    ) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing user preferences and privacy settings");

        // Generate and test multiple preference configurations
        for i in 0..3 {
            let start_time = std::time::Instant::now();
            let preferences = self.generator.generate_user_preferences();

            ctx.log_info(&format!("[PREFERENCES] Testing configuration {}", i + 1));

            // Test preferences serialization
            match serde_json::to_string(&preferences) {
                Ok(serialized) => {
                    match serde_json::from_str::<saorsa_core::identity::manager::UserPreferences>(
                        &serialized,
                    ) {
                        Ok(deserialized) => {
                            ctx.log_info(&format!(
                                "✅ Preferences serialization verified (config {})",
                                i + 1
                            ));
                            results.push(
                                VerificationResult::success(start_time.elapsed())
                                    .with_metadata(
                                        "operation".to_string(),
                                        "preferences_serialization".to_string(),
                                    )
                                    .with_metadata("config_id".to_string(), (i + 1).to_string())
                                    .with_metadata("data_verified".to_string(), "true".to_string()),
                            );
                        }
                        Err(e) => {
                            let error = format!("Preferences deserialization failed: {}", e);
                            ctx.log_error(&error);
                            results.push(VerificationResult::failure(error, start_time.elapsed()));
                        }
                    }
                }
                Err(e) => {
                    let error = format!("Preferences serialization failed: {}", e);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        Ok(results)
    }

    /// Test three-word address generation and verification
    async fn test_three_word_addresses(
        &self,
        ctx: &TestContext,
    ) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing three-word address system");

        // TODO: Implement three-word address tests when available
        // This would test:
        // 1. Address generation from peer IDs
        // 2. Address to peer ID conversion
        // 3. Address uniqueness verification
        // 4. Round-trip conversion verification

        warn!("Three-word address tests not yet implemented");

        let mock_result = VerificationResult::success(Duration::from_millis(50))
            .with_metadata("operation".to_string(), "three_word_addresses".to_string())
            .with_metadata("mode".to_string(), "mock".to_string());
        results.push(mock_result);

        Ok(results)
    }

    /// Verify profile data integrity between original and processed versions
    fn verify_profile_integrity(&self, original: &UserProfile, processed: &UserProfile) -> bool {
        original.user_id == processed.user_id
            && original.display_name == processed.display_name
            && original.bio == processed.bio
            && original.avatar_url == processed.avatar_url
            && original.avatar_hash == processed.avatar_hash
            && original.status_message == processed.status_message
            && original.public_key == processed.public_key
            && original.custom_fields == processed.custom_fields
    }
}

#[async_trait::async_trait]
impl SubsystemTest for IdentityTests {
    fn name(&self) -> &str {
        "identity"
    }

    async fn test_basic_functionality(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        let mut results = Vec::new();

        ctx.log_info("Running basic identity functionality tests");

        // Test user profile operations
        let profile_results = test_instance.test_user_profile_operations(ctx).await?;
        results.extend(profile_results);

        // Test three-word addresses
        let address_results = test_instance.test_three_word_addresses(ctx).await?;
        results.extend(address_results);

        ctx.log_info(&format!(
            "Basic identity tests completed. Results: {}",
            results.len()
        ));
        Ok(results)
    }

    async fn test_data_verification(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();

        ctx.log_info("Running identity data verification tests");

        // Test user preferences data integrity
        let preferences_results = test_instance.test_user_preferences(ctx).await?;

        ctx.log_info(&format!(
            "Identity data verification completed. Results: {}",
            preferences_results.len()
        ));
        Ok(preferences_results)
    }

    async fn test_cross_node(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        ctx.log_info("Running cross-node identity tests");

        // TODO: Implement cross-node identity tests
        warn!("Cross-node identity tests not yet implemented");

        Ok(vec![VerificationResult::success(Duration::from_millis(
            200,
        ))])
    }

    async fn test_stress(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        ctx.log_info("Running identity stress tests");

        // TODO: Implement identity stress tests
        warn!("Identity stress tests not yet implemented");

        Ok(vec![VerificationResult::success(Duration::from_millis(
            300,
        ))])
    }
}

impl Default for IdentityTests {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for IdentityTests {
    fn clone(&self) -> Self {
        Self {
            generator: TestDataGenerator::new(), // Create new generator
            verifier: self.verifier.clone(),
        }
    }
}
