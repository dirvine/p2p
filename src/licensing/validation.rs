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

//! License validation and enforcement utilities

use super::{CommercialLicense, LicenseError, LicenseStatus, LicenseType, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// License validator for cryptographic validation
pub struct LicenseValidator {
    /// Public key for signature verification (in real impl)
    public_key: Option<Vec<u8>>,
}

impl LicenseValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self { public_key: None }
    }

    /// Create a validator with a public key
    pub fn with_public_key(public_key: Vec<u8>) -> Self {
        Self {
            public_key: Some(public_key),
        }
    }

    /// Validate a license key format
    pub fn validate_key_format(key: &str) -> Result<()> {
        // Expected format: XXXX-XXXX-XXXX-XXXX-XXXX
        let parts: Vec<&str> = key.split('-').collect();
        if parts.len() != 5 {
            return Err(LicenseError::InvalidKey);
        }

        for part in parts {
            if part.len() != 4 || !part.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(LicenseError::InvalidKey);
            }
        }

        Ok(())
    }

    /// Generate a license key (for testing/demo purposes)
    pub fn generate_key(org: &str, tier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(org.as_bytes());
        hasher.update(tier.as_bytes());
        hasher.update(chrono::Utc::now().timestamp().to_be_bytes());
        
        let hash = hasher.finalize();
        let key_bytes = &hash[..20]; // Take first 20 bytes
        
        // Convert to key format
        let hex = hex::encode_upper(key_bytes);
        format!(
            "{}-{}-{}-{}-{}",
            &hex[0..4],
            &hex[4..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20]
        )
    }

    /// Validate license signature (mock implementation)
    pub fn validate_signature(&self, license: &CommercialLicense) -> Result<()> {
        // In a real implementation, this would:
        // 1. Extract the signature from the license
        // 2. Verify it using the public key
        // 3. Ensure the license hasn't been tampered with
        
        // For now, just validate the key format
        Self::validate_key_format(&license.key)?;
        Ok(())
    }

    /// Check if organization is on blocklist
    pub fn check_blocklist(&self, organization: &str) -> Result<()> {
        // In a real implementation, this would check against a blocklist
        let blocklist = ["BadCorp", "ScamInc"];
        
        if blocklist.contains(&organization) {
            return Err(LicenseError::ValidationFailed(
                "Organization is blocklisted".into()
            ));
        }
        
        Ok(())
    }

    /// Validate a commercial license fully
    pub fn validate_commercial(&self, license: &CommercialLicense) -> Result<()> {
        // Basic validation
        license.validate()?;
        
        // Signature validation
        self.validate_signature(license)?;
        
        // Blocklist check
        self.check_blocklist(&license.organization)?;
        
        // Additional business logic validation
        match license.tier {
            super::CommercialLicenseTier::Smb => {
                if license.max_users.unwrap_or(0) > 50 {
                    return Err(LicenseError::ValidationFailed(
                        "SMB license cannot exceed 50 users".into()
                    ));
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    /// Create a demo license for testing
    pub fn create_demo_license(organization: &str) -> CommercialLicense {
        use chrono::{Duration, Utc};
        
        CommercialLicense {
            key: Self::generate_key(organization, "demo"),
            organization: organization.to_string(),
            tier: super::CommercialLicenseTier::Smb,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
            max_users: Some(5),
            contact_email: format!("demo@{}.example", organization.to_lowercase()),
            metadata: {
                let mut map = HashMap::new();
                map.insert("type".to_string(), "demo".to_string());
                map.insert("features".to_string(), "limited".to_string());
                map
            },
        }
    }
}

impl Default for LicenseValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// License enforcement utilities
pub struct LicenseEnforcer;

impl LicenseEnforcer {
    /// Check if we should enforce commercial licensing
    pub fn should_enforce() -> bool {
        // Check environment variable
        if let Ok(enforce) = std::env::var("P2P_ENFORCE_LICENSE") {
            return enforce.to_lowercase() == "true" || enforce == "1";
        }
        
        // Check if commercial feature is enabled
        #[cfg(feature = "commercial")]
        return true;
        
        #[cfg(not(feature = "commercial"))]
        false
    }

    /// Get grace period for expired licenses
    pub fn grace_period_days() -> i64 {
        std::env::var("P2P_LICENSE_GRACE_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30)
    }

    /// Check if we're in development mode (relaxed enforcement)
    pub fn is_development_mode() -> bool {
        cfg!(debug_assertions) || 
        std::env::var("P2P_DEV_MODE").map(|v| v == "1").unwrap_or(false)
    }

    /// Apply enforcement policy to a license status
    pub fn enforce_policy(status: &LicenseStatus) -> Result<()> {
        // Skip enforcement in development mode
        if Self::is_development_mode() {
            return Ok(());
        }

        // Skip enforcement if not enabled
        if !Self::should_enforce() {
            return Ok(());
        }

        // For AGPL license, no enforcement needed
        if status.license_type == LicenseType::Agpl {
            return Ok(());
        }

        // Check if license is valid
        if !status.is_valid {
            return Err(LicenseError::ValidationFailed(
                "Commercial license is not valid".into()
            ));
        }

        // Check expiration with grace period
        if let Some(expires_at) = status.expires_at {
            let grace_period = chrono::Duration::days(Self::grace_period_days());
            if chrono::Utc::now() > expires_at + grace_period {
                return Err(LicenseError::Expired);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_format_validation() {
        // Valid keys
        assert!(LicenseValidator::validate_key_format("ABCD-1234-WXYZ-5678-QRST").is_ok());
        assert!(LicenseValidator::validate_key_format("0000-0000-0000-0000-0000").is_ok());
        
        // Invalid keys
        assert!(LicenseValidator::validate_key_format("ABCD-123-WXYZ-5678-QRST").is_err());
        assert!(LicenseValidator::validate_key_format("ABCD_1234_WXYZ_5678_QRST").is_err());
        assert!(LicenseValidator::validate_key_format("ABCD-1234-WXYZ-5678").is_err());
        assert!(LicenseValidator::validate_key_format("").is_err());
    }

    #[test]
    fn test_key_generation() {
        let key1 = LicenseValidator::generate_key("TestCorp", "enterprise");
        let key2 = LicenseValidator::generate_key("TestCorp", "enterprise");
        
        // Keys should be different (due to timestamp)
        assert_ne!(key1, key2);
        
        // But both should be valid format
        assert!(LicenseValidator::validate_key_format(&key1).is_ok());
        assert!(LicenseValidator::validate_key_format(&key2).is_ok());
    }

    #[test]
    fn test_demo_license_creation() {
        let demo = LicenseValidator::create_demo_license("DemoOrg");
        
        assert_eq!(demo.organization, "DemoOrg");
        assert_eq!(demo.tier, super::CommercialLicenseTier::Smb);
        assert_eq!(demo.max_users, Some(5));
        assert!(demo.validate().is_ok());
    }

    #[test]
    fn test_blocklist_check() {
        let validator = LicenseValidator::new();
        
        assert!(validator.check_blocklist("GoodCorp").is_ok());
        assert!(validator.check_blocklist("BadCorp").is_err());
        assert!(validator.check_blocklist("ScamInc").is_err());
    }
}