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

//! License management and enforcement for P2P Foundation
//! 
//! This module provides runtime license detection, validation, and feature gating
//! based on the active license type (AGPL-3.0 or Commercial).

pub mod checker;
pub mod commercial;
pub mod validation;

use serde::{Deserialize, Serialize};
use std::fmt;

pub use checker::LicenseChecker;
pub use commercial::CommercialLicense;
pub use validation::LicenseValidator;

/// Type of license in use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseType {
    /// AGPL-3.0 open source license
    Agpl,
    /// Commercial license (SMB tier)
    CommercialSmb,
    /// Commercial license (Enterprise tier)
    CommercialEnterprise,
    /// Commercial license (OEM tier)
    CommercialOem,
}

impl LicenseType {
    /// Check if this is a commercial license
    pub fn is_commercial(&self) -> bool {
        !matches!(self, Self::Agpl)
    }

    /// Get the user limit for this license type
    pub fn user_limit(&self) -> Option<usize> {
        match self {
            Self::Agpl => None,
            Self::CommercialSmb => Some(50),
            Self::CommercialEnterprise => None,
            Self::CommercialOem => None,
        }
    }

    /// Check if priority support is included
    pub fn has_priority_support(&self) -> bool {
        matches!(self, Self::CommercialEnterprise | Self::CommercialOem)
    }

    /// Check if white-label features are available
    pub fn has_white_label(&self) -> bool {
        matches!(self, Self::CommercialOem)
    }
}

impl fmt::Display for LicenseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agpl => write!(f, "AGPL-3.0"),
            Self::CommercialSmb => write!(f, "Commercial (SMB)"),
            Self::CommercialEnterprise => write!(f, "Commercial (Enterprise)"),
            Self::CommercialOem => write!(f, "Commercial (OEM)"),
        }
    }
}

/// License status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    /// Type of license
    pub license_type: LicenseType,
    /// Organization name (for commercial licenses)
    pub organization: Option<String>,
    /// License expiration date (for commercial licenses)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of users (for tracking against limits)
    pub user_count: usize,
    /// Whether the license is currently valid
    pub is_valid: bool,
    /// License key (for commercial licenses)
    pub license_key: Option<String>,
}

impl Default for LicenseStatus {
    fn default() -> Self {
        Self {
            license_type: LicenseType::Agpl,
            organization: None,
            expires_at: None,
            user_count: 0,
            is_valid: true,
            license_key: None,
        }
    }
}

/// Features that may be gated by license type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// Basic P2P functionality
    Core,
    /// Priority bootstrap nodes
    PriorityBootstrap,
    /// Enhanced rate limits
    EnhancedRateLimits,
    /// Custom branding
    WhiteLabel,
    /// Advanced analytics
    Analytics,
    /// API access
    ApiAccess,
    /// Premium support
    PremiumSupport,
}

impl Feature {
    /// Check if this feature is available for the given license type
    pub fn is_available_for(&self, license_type: LicenseType) -> bool {
        match self {
            Self::Core => true, // Available for all
            Self::PriorityBootstrap => license_type.is_commercial(),
            Self::EnhancedRateLimits => license_type.is_commercial(),
            Self::WhiteLabel => license_type.has_white_label(),
            Self::Analytics => matches!(
                license_type,
                LicenseType::CommercialEnterprise | LicenseType::CommercialOem
            ),
            Self::ApiAccess => license_type.is_commercial(),
            Self::PremiumSupport => license_type.has_priority_support(),
        }
    }
}

/// Error types for licensing operations
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("Invalid license key")]
    InvalidKey,
    
    #[error("License expired")]
    Expired,
    
    #[error("User limit exceeded (limit: {limit}, current: {current})")]
    UserLimitExceeded { limit: usize, current: usize },
    
    #[error("Feature not available for license type: {feature:?}")]
    FeatureNotAvailable { feature: Feature },
    
    #[error("License validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for licensing operations
pub type Result<T> = std::result::Result<T, LicenseError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_type_properties() {
        assert!(!LicenseType::Agpl.is_commercial());
        assert!(LicenseType::CommercialSmb.is_commercial());
        
        assert_eq!(LicenseType::CommercialSmb.user_limit(), Some(50));
        assert_eq!(LicenseType::CommercialEnterprise.user_limit(), None);
        
        assert!(!LicenseType::CommercialSmb.has_priority_support());
        assert!(LicenseType::CommercialEnterprise.has_priority_support());
        
        assert!(!LicenseType::CommercialEnterprise.has_white_label());
        assert!(LicenseType::CommercialOem.has_white_label());
    }

    #[test]
    fn test_feature_availability() {
        assert!(Feature::Core.is_available_for(LicenseType::Agpl));
        assert!(!Feature::PriorityBootstrap.is_available_for(LicenseType::Agpl));
        assert!(Feature::PriorityBootstrap.is_available_for(LicenseType::CommercialSmb));
        assert!(!Feature::WhiteLabel.is_available_for(LicenseType::CommercialEnterprise));
        assert!(Feature::WhiteLabel.is_available_for(LicenseType::CommercialOem));
    }
}