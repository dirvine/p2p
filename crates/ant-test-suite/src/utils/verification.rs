//! Data verification utilities
//!
//! Core patterns and utilities for ensuring data integrity across
//! all storage and communication operations in the P2P network.

use anyhow::{Context, Result};
use ant_core::{Key, Record, PeerId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, error, info, warn};

/// Result of a data verification operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether verification passed
    pub success: bool,
    
    /// Error message if verification failed
    pub error: Option<String>,
    
    /// Time taken for verification
    pub duration: Duration,
    
    /// Additional metadata about the verification
    pub metadata: HashMap<String, String>,
    
    /// Timestamp when verification was performed
    pub timestamp: SystemTime,
}

impl VerificationResult {
    pub fn success(duration: Duration) -> Self {
        Self {
            success: true,
            error: None,
            duration,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }

    pub fn failure(error: String, duration: Duration) -> Self {
        Self {
            success: false,
            error: Some(error),
            duration,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Comprehensive data verification patterns
#[derive(Clone)]
pub struct DataVerifier {
    /// Enable strict verification mode (zero tolerance for corruption)
    strict_mode: bool,
    
    /// Verification timeout
    timeout: Duration,
    
    /// Number of retry attempts
    retries: u32,
}

impl DataVerifier {
    pub fn new(strict_mode: bool, timeout: Duration, retries: u32) -> Self {
        Self {
            strict_mode,
            timeout,
            retries,
        }
    }

    /// Basic round-trip verification pattern
    /// Store data, immediately read it back, and verify exact match
    pub async fn verify_round_trip<T>(
        &self,
        data: &T,
        store_fn: impl Fn(&T) -> Result<Key>,
        read_fn: impl Fn(&Key) -> Result<T>,
    ) -> Result<VerificationResult>
    where
        T: PartialEq + Debug + Clone,
    {
        let start = Instant::now();
        
        // Store the data
        let key = store_fn(data)
            .context("Failed to store data for round-trip verification")?;
        
        // Read back the data
        let retrieved = read_fn(&key)
            .context("Failed to read data for round-trip verification")?;
        
        // Verify exact match
        let success = *data == retrieved;
        let duration = start.elapsed();
        
        if success {
            debug!("Round-trip verification passed in {:?}", duration);
            Ok(VerificationResult::success(duration))
        } else {
            let error = format!("Round-trip verification failed: data mismatch");
            error!("{}", error);
            
            if self.strict_mode {
                return Err(anyhow::anyhow!(error));
            }
            
            Ok(VerificationResult::failure(error, duration))
        }
    }

    /// Cross-node verification pattern
    /// Store on one node, read from another, verify consistency
    pub async fn verify_cross_node<T>(
        &self,
        data: &T,
        local_store_fn: impl Fn(&T) -> Result<Key>,
        remote_read_fn: impl Fn(&Key) -> Result<T>,
    ) -> Result<VerificationResult>
    where
        T: PartialEq + Debug + Clone,
    {
        let start = Instant::now();
        
        // Store on local node
        let key = local_store_fn(data)
            .context("Failed to store data on local node")?;
        
        // Wait for DHT propagation
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Read from remote node with retries
        let mut last_error = None;
        for attempt in 0..=self.retries {
            match remote_read_fn(&key) {
                Ok(retrieved) => {
                    let success = *data == retrieved;
                    let duration = start.elapsed();
                    
                    if success {
                        debug!("Cross-node verification passed in {:?} (attempt {})", duration, attempt + 1);
                        return Ok(VerificationResult::success(duration)
                            .with_metadata("attempts".to_string(), (attempt + 1).to_string()));
                    } else {
                        let error = format!("Cross-node verification failed: data mismatch on attempt {}", attempt + 1);
                        error!("{}", error);
                        
                        if self.strict_mode {
                            return Err(anyhow::anyhow!(error));
                        }
                        
                        return Ok(VerificationResult::failure(error, duration));
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.retries {
                        warn!("Cross-node read attempt {} failed, retrying...", attempt + 1);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
        
        let error = format!("Cross-node verification failed after {} attempts: {:?}", 
                          self.retries + 1, last_error);
        error!("{}", error);
        
        if self.strict_mode {
            Err(anyhow::anyhow!(error))
        } else {
            Ok(VerificationResult::failure(error, start.elapsed()))
        }
    }

    /// Hash verification pattern
    /// Verify data integrity using cryptographic hashes
    pub fn verify_hash<T>(
        &self,
        data: &T,
        expected_hash: &[u8],
        hash_fn: impl Fn(&T) -> Vec<u8>,
    ) -> Result<VerificationResult>
    where
        T: Debug,
    {
        let start = Instant::now();
        
        let computed_hash = hash_fn(data);
        let success = computed_hash == expected_hash;
        let duration = start.elapsed();
        
        if success {
            debug!("Hash verification passed in {:?}", duration);
            Ok(VerificationResult::success(duration))
        } else {
            let error = format!(
                "Hash verification failed: expected {}, got {}",
                hex::encode(expected_hash),
                hex::encode(&computed_hash)
            );
            error!("{}", error);
            
            if self.strict_mode {
                Err(anyhow::anyhow!(error))
            } else {
                Ok(VerificationResult::failure(error, duration))
            }
        }
    }

    /// Bulk verification pattern
    /// Verify large numbers of items efficiently
    pub async fn verify_bulk<T>(
        &self,
        items: Vec<(T, Key)>,
        read_fn: impl Fn(&Key) -> Result<T>,
    ) -> Result<BulkVerificationResult>
    where
        T: PartialEq + Debug + Clone,
    {
        let start = Instant::now();
        let total_items = items.len();
        let mut successful = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        
        info!("Starting bulk verification of {} items", total_items);
        
        for (i, (original, key)) in items.into_iter().enumerate() {
            match read_fn(&key) {
                Ok(retrieved) => {
                    if original == retrieved {
                        successful += 1;
                    } else {
                        failed += 1;
                        let error = format!("Item {} data mismatch", i);
                        errors.push(error);
                        
                        if self.strict_mode {
                            return Err(anyhow::anyhow!("Bulk verification failed on item {}", i));
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    let error = format!("Item {} read failed: {:?}", i, e);
                    errors.push(error);
                    
                    if self.strict_mode {
                        return Err(anyhow::anyhow!("Bulk verification failed on item {}: {:?}", i, e));
                    }
                }
            }
            
            // Progress reporting for large batches
            if i % 100 == 0 && i > 0 {
                debug!("Bulk verification progress: {}/{} items", i, total_items);
            }
        }
        
        let duration = start.elapsed();
        let success_rate = successful as f64 / total_items as f64;
        
        info!(
            "Bulk verification completed: {}/{} successful ({:.1}%) in {:?}",
            successful, total_items, success_rate * 100.0, duration
        );
        
        Ok(BulkVerificationResult {
            total_items,
            successful,
            failed,
            success_rate,
            duration,
            errors,
        })
    }

    /// Signature verification pattern
    /// Verify cryptographic signatures on stored data
    pub fn verify_signature(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<VerificationResult> {
        let start = Instant::now();
        
        // TODO: Implement actual signature verification using ant-core crypto
        // This is a placeholder implementation
        let success = signature.len() == 64 && public_key.len() == 32;
        let duration = start.elapsed();
        
        if success {
            debug!("Signature verification passed in {:?}", duration);
            Ok(VerificationResult::success(duration))
        } else {
            let error = "Signature verification failed".to_string();
            error!("{}", error);
            
            if self.strict_mode {
                Err(anyhow::anyhow!(error))
            } else {
                Ok(VerificationResult::failure(error, duration))
            }
        }
    }

    /// Content integrity verification using SHA256
    pub fn compute_content_hash<T: AsRef<[u8]>>(data: T) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_ref());
        hasher.finalize().to_vec()
    }
}

/// Result of bulk verification operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkVerificationResult {
    pub total_items: usize,
    pub successful: usize,
    pub failed: usize,
    pub success_rate: f64,
    pub duration: Duration,
    pub errors: Vec<String>,
}

impl BulkVerificationResult {
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }

    pub fn summary(&self) -> String {
        format!(
            "Bulk verification: {}/{} successful ({:.1}%), {} failed",
            self.successful,
            self.total_items,
            self.success_rate * 100.0,
            self.failed
        )
    }
}

/// Verification statistics tracking
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VerificationStats {
    pub total_verifications: u64,
    pub successful_verifications: u64,
    pub failed_verifications: u64,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub last_verification: Option<SystemTime>,
}

impl VerificationStats {
    pub fn add_result(&mut self, result: &VerificationResult) {
        self.total_verifications += 1;
        self.total_duration += result.duration;
        self.average_duration = self.total_duration / self.total_verifications as u32;
        self.last_verification = Some(SystemTime::now());
        
        if result.success {
            self.successful_verifications += 1;
        } else {
            self.failed_verifications += 1;
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_verifications == 0 {
            0.0
        } else {
            self.successful_verifications as f64 / self.total_verifications as f64
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Verification Stats: {}/{} successful ({:.1}%), avg duration: {:?}",
            self.successful_verifications,
            self.total_verifications,
            self.success_rate() * 100.0,
            self.average_duration
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verification_result() {
        let duration = Duration::from_millis(100);
        
        let success = VerificationResult::success(duration);
        assert!(success.success);
        assert!(success.error.is_none());
        assert_eq!(success.duration, duration);
        
        let failure = VerificationResult::failure("test error".to_string(), duration);
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }

    #[test]
    fn test_content_hash() {
        let data = b"test data";
        let hash1 = DataVerifier::compute_content_hash(data);
        let hash2 = DataVerifier::compute_content_hash(data);
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32); // SHA256 hash length
    }

    #[test]
    fn test_verification_stats() {
        let mut stats = VerificationStats::default();
        
        let success_result = VerificationResult::success(Duration::from_millis(100));
        let failure_result = VerificationResult::failure("error".to_string(), Duration::from_millis(50));
        
        stats.add_result(&success_result);
        stats.add_result(&failure_result);
        
        assert_eq!(stats.total_verifications, 2);
        assert_eq!(stats.successful_verifications, 1);
        assert_eq!(stats.failed_verifications, 1);
        assert_eq!(stats.success_rate(), 0.5);
    }
}