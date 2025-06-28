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

//! Commercial license management

use super::{LicenseError, LicenseStatus, LicenseType, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Commercial license information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommercialLicense {
    /// License key
    pub key: String,
    /// Organization name
    pub organization: String,
    /// License type (SMB, Enterprise, OEM)
    pub tier: CommercialLicenseTier,
    /// Issue date
    pub issued_at: chrono::DateTime<chrono::Utc>,
    /// Expiration date
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Maximum users (if applicable)
    pub max_users: Option<usize>,
    /// Contact email
    pub contact_email: String,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Commercial license tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommercialLicenseTier {
    /// Small & Medium Business
    Smb,
    /// Enterprise
    Enterprise,
    /// OEM/Reseller
    Oem,
}

impl CommercialLicenseTier {
    /// Convert to LicenseType
    pub fn to_license_type(&self) -> LicenseType {
        match self {
            Self::Smb => LicenseType::CommercialSmb,
            Self::Enterprise => LicenseType::CommercialEnterprise,
            Self::Oem => LicenseType::CommercialOem,
        }
    }
}

impl CommercialLicense {
    /// Validate the license
    pub fn validate(&self) -> Result<()> {
        // Check expiration
        if chrono::Utc::now() > self.expires_at {
            return Err(LicenseError::Expired);
        }

        // Validate key format (basic check)
        if self.key.len() < 16 {
            return Err(LicenseError::InvalidKey);
        }

        Ok(())
    }

    /// Convert to LicenseStatus
    pub fn to_status(&self, user_count: usize) -> LicenseStatus {
        LicenseStatus {
            license_type: self.tier.to_license_type(),
            organization: Some(self.organization.clone()),
            expires_at: Some(self.expires_at),
            user_count,
            is_valid: self.validate().is_ok(),
            license_key: Some(self.key.clone()),
        }
    }

    /// Load license from file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let license: Self = serde_json::from_str(&content)
            .map_err(|e| LicenseError::ValidationFailed(format!("Invalid license file: {}", e)))?;
        license.validate()?;
        Ok(license)
    }

    /// Save license to file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| LicenseError::ValidationFailed(format!("Failed to serialize license: {}", e)))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default license file path
    pub fn default_path() -> Result<PathBuf> {
        let dirs = dirs::config_dir()
            .ok_or_else(|| LicenseError::ValidationFailed("Could not find config directory".into()))?;
        Ok(dirs.join("p2p-foundation").join("license.json"))
    }

    /// Check if license file exists at default location
    pub fn exists() -> bool {
        Self::default_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Load from default location
    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;
        Self::load_from_file(path)
    }

    /// Save to default location
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.save_to_file(path)
    }
}

/// License verification service (mock implementation)
pub struct LicenseVerificationService {
    endpoint: String,
}

impl LicenseVerificationService {
    /// Create a new verification service
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    /// Verify a license key with the remote service
    pub async fn verify_license(&self, key: &str) -> Result<CommercialLicense> {
        // In a real implementation, this would:
        // 1. Connect to the license server
        // 2. Send the license key for verification
        // 3. Receive and validate the response
        // 4. Return the full license information
        
        // For now, return a validation error
        Err(LicenseError::Network(format!(
            "License verification not implemented. Would connect to: {}",
            self.endpoint
        )))
    }

    /// Check license status (for periodic validation)
    pub async fn check_status(&self, license: &CommercialLicense) -> Result<bool> {
        // In a real implementation, this would:
        // 1. Connect to the license server
        // 2. Check if the license is still valid
        // 3. Check for any updates or revocations
        
        // For now, just do local validation
        license.validate().map(|_| true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn create_test_license() -> CommercialLicense {
        CommercialLicense {
            key: "TEST-1234-5678-90AB-CDEF".to_string(),
            organization: "Test Corp".to_string(),
            tier: CommercialLicenseTier::Enterprise,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(365),
            max_users: None,
            contact_email: "test@example.com".to_string(),
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_license_validation() {
        let license = create_test_license();
        assert!(license.validate().is_ok());

        // Test expired license
        let mut expired = license.clone();
        expired.expires_at = Utc::now() - Duration::days(1);
        assert!(matches!(expired.validate(), Err(LicenseError::Expired)));

        // Test invalid key
        let mut invalid = license;
        invalid.key = "SHORT".to_string();
        assert!(matches!(invalid.validate(), Err(LicenseError::InvalidKey)));
    }

    #[test]
    fn test_license_to_status() {
        let license = create_test_license();
        let status = license.to_status(100);

        assert_eq!(status.license_type, LicenseType::CommercialEnterprise);
        assert_eq!(status.organization, Some("Test Corp".to_string()));
        assert_eq!(status.user_count, 100);
        assert!(status.is_valid);
    }

    #[test]
    fn test_tier_conversion() {
        assert_eq!(
            CommercialLicenseTier::Smb.to_license_type(),
            LicenseType::CommercialSmb
        );
        assert_eq!(
            CommercialLicenseTier::Enterprise.to_license_type(),
            LicenseType::CommercialEnterprise
        );
        assert_eq!(
            CommercialLicenseTier::Oem.to_license_type(),
            LicenseType::CommercialOem
        );
    }
}