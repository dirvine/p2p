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

//! License checking and enforcement

use super::{Feature, LicenseError, LicenseStatus, LicenseType, Result};
use std::sync::{Arc, RwLock};

/// License checker for runtime license validation and feature gating
pub struct LicenseChecker {
    status: Arc<RwLock<LicenseStatus>>,
}

impl LicenseChecker {
    /// Create a new license checker with AGPL license by default
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(LicenseStatus::default())),
        }
    }

    /// Create a license checker with a specific license status
    pub fn with_status(status: LicenseStatus) -> Self {
        Self {
            status: Arc::new(RwLock::new(status)),
        }
    }

    /// Get the current license status
    pub fn status(&self) -> Result<LicenseStatus> {
        self.status
            .read()
            .map(|s| s.clone())
            .map_err(|_| LicenseError::ValidationFailed("Failed to read license status".into()))
    }

    /// Update the license status
    pub fn update_status(&self, status: LicenseStatus) -> Result<()> {
        let mut current = self.status.write()
            .map_err(|_| LicenseError::ValidationFailed("Failed to update license status".into()))?;
        *current = status;
        Ok(())
    }

    /// Check if a feature is available
    pub fn is_feature_available(&self, feature: Feature) -> Result<bool> {
        let status = self.status()?;
        if !status.is_valid {
            return Err(LicenseError::ValidationFailed("License is not valid".into()));
        }
        Ok(feature.is_available_for(status.license_type))
    }

    /// Require a feature to be available, returning an error if not
    pub fn require_feature(&self, feature: Feature) -> Result<()> {
        if !self.is_feature_available(feature)? {
            return Err(LicenseError::FeatureNotAvailable { feature });
        }
        Ok(())
    }

    /// Check if the current user count is within limits
    pub fn check_user_limit(&self, user_count: usize) -> Result<()> {
        let status = self.status()?;
        if let Some(limit) = status.license_type.user_limit() {
            if user_count > limit {
                return Err(LicenseError::UserLimitExceeded {
                    limit,
                    current: user_count,
                });
            }
        }
        Ok(())
    }

    /// Check if the license is expired
    pub fn check_expiration(&self) -> Result<()> {
        let status = self.status()?;
        if let Some(expires_at) = status.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(LicenseError::Expired);
            }
        }
        Ok(())
    }

    /// Perform full license validation
    pub fn validate(&self) -> Result<()> {
        let status = self.status()?;
        
        // Check if license is marked as valid
        if !status.is_valid {
            return Err(LicenseError::ValidationFailed("License marked as invalid".into()));
        }

        // Check expiration for commercial licenses
        if status.license_type.is_commercial() {
            self.check_expiration()?;
        }

        // Check user limits
        self.check_user_limit(status.user_count)?;

        Ok(())
    }

    /// Get the current license type
    pub fn license_type(&self) -> Result<LicenseType> {
        Ok(self.status()?.license_type)
    }

    /// Check if using a commercial license
    pub fn is_commercial(&self) -> Result<bool> {
        Ok(self.license_type()?.is_commercial())
    }

    /// Set the current user count
    pub fn set_user_count(&self, count: usize) -> Result<()> {
        let mut status = self.status.write()
            .map_err(|_| LicenseError::ValidationFailed("Failed to update user count".into()))?;
        status.user_count = count;
        
        // Check limit immediately
        if let Some(limit) = status.license_type.user_limit() {
            if count > limit {
                return Err(LicenseError::UserLimitExceeded {
                    limit,
                    current: count,
                });
            }
        }
        
        Ok(())
    }
}

impl Default for LicenseChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Global license checker instance
static GLOBAL_CHECKER: std::sync::OnceLock<LicenseChecker> = std::sync::OnceLock::new();

/// Get the global license checker instance
pub fn global_checker() -> &'static LicenseChecker {
    GLOBAL_CHECKER.get_or_init(LicenseChecker::new)
}

/// Initialize the global license checker with a specific status
pub fn init_global_checker(status: LicenseStatus) -> Result<()> {
    let checker = GLOBAL_CHECKER.get_or_init(|| LicenseChecker::with_status(status.clone()));
    checker.update_status(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_default_license_checker() {
        let checker = LicenseChecker::new();
        let status = checker.status().unwrap();
        assert_eq!(status.license_type, LicenseType::Agpl);
        assert!(status.is_valid);
    }

    #[test]
    fn test_feature_checking() {
        let checker = LicenseChecker::new();
        
        // Core features should be available for AGPL
        assert!(checker.is_feature_available(Feature::Core).unwrap());
        
        // Premium features should not be available for AGPL
        assert!(!checker.is_feature_available(Feature::PriorityBootstrap).unwrap());
        assert!(!checker.is_feature_available(Feature::WhiteLabel).unwrap());
    }

    #[test]
    fn test_commercial_features() {
        let status = LicenseStatus {
            license_type: LicenseType::CommercialEnterprise,
            is_valid: true,
            ..Default::default()
        };
        let checker = LicenseChecker::with_status(status);
        
        assert!(checker.is_feature_available(Feature::Core).unwrap());
        assert!(checker.is_feature_available(Feature::PriorityBootstrap).unwrap());
        assert!(checker.is_feature_available(Feature::Analytics).unwrap());
        assert!(!checker.is_feature_available(Feature::WhiteLabel).unwrap());
    }

    #[test]
    fn test_user_limits() {
        let status = LicenseStatus {
            license_type: LicenseType::CommercialSmb,
            user_count: 30,
            is_valid: true,
            ..Default::default()
        };
        let checker = LicenseChecker::with_status(status);
        
        // Should pass with 30 users (under 50 limit)
        assert!(checker.check_user_limit(30).is_ok());
        
        // Should fail with 51 users (over 50 limit)
        assert!(checker.check_user_limit(51).is_err());
    }

    #[test]
    fn test_expiration() {
        let status = LicenseStatus {
            license_type: LicenseType::CommercialEnterprise,
            expires_at: Some(Utc::now() - Duration::days(1)),
            is_valid: true,
            ..Default::default()
        };
        let checker = LicenseChecker::with_status(status);
        
        // Should fail validation due to expiration
        assert!(matches!(checker.check_expiration(), Err(LicenseError::Expired)));
    }

    #[test]
    fn test_require_feature() {
        let checker = LicenseChecker::new();
        
        // Should succeed for core features
        assert!(checker.require_feature(Feature::Core).is_ok());
        
        // Should fail for premium features with AGPL license
        assert!(matches!(
            checker.require_feature(Feature::PriorityBootstrap),
            Err(LicenseError::FeatureNotAvailable { .. })
        ));
    }
}