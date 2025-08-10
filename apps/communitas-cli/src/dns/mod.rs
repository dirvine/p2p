// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Four-Word DNS system for DHT-based personal profiles

pub mod profile;
pub mod validator;
pub mod storage;
pub mod resolver;

use serde::{Deserialize, Serialize};

/// Re-export main components
pub use profile::{FourWordProfile, ProfileContent};
pub use validator::{ProfileValidator, ValidationResult};
pub use storage::{DHTProfileStorage, ProfilePacket, StorageStats, StorageQuery};
pub use resolver::{ProfileResolver, ResolutionResult, ResolutionQuery, BatchResolutionRequest, BatchResolutionResponse};

/// Four-Word DNS system error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DNSError {
    InvalidFourWords(String),
    HashMismatch { expected: String, actual: String },
    ProfileNotFound(String),
    InvalidSignature,
    StorageError(String),
    ValidationError(String),
}

impl std::fmt::Display for DNSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DNSError::InvalidFourWords(words) => write!(f, "Invalid four words: {}", words),
            DNSError::HashMismatch { expected, actual } => {
                write!(f, "Hash mismatch - expected: {}, actual: {}", expected, actual)
            },
            DNSError::ProfileNotFound(words) => write!(f, "Profile not found for: {}", words),
            DNSError::InvalidSignature => write!(f, "Invalid profile signature"),
            DNSError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            DNSError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for DNSError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] 
    fn test_dns_error_display() {
        let error = DNSError::InvalidFourWords("invalid words".to_string());
        assert_eq!(error.to_string(), "Invalid four words: invalid words");
        
        let error = DNSError::HashMismatch { 
            expected: "abc123".to_string(), 
            actual: "def456".to_string() 
        };
        assert_eq!(error.to_string(), "Hash mismatch - expected: abc123, actual: def456");
    }
}