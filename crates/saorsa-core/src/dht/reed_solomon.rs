//! Reed-Solomon erasure coding for fault-tolerant DHT storage
//!
//! Provides configurable redundancy with dynamic adjustment based on network conditions.

use anyhow::{Result, anyhow};
use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for Reed-Solomon encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSConfig {
    /// Number of data chunks
    pub k: usize,
    /// Number of parity chunks
    pub m: usize,
    /// Maximum chunk size in bytes
    pub max_chunk_size: usize,
}

impl RSConfig {
    /// Create a new configuration
    pub fn new(k: usize, m: usize) -> Result<Self> {
        if k == 0 || m == 0 || k > 128 || m > 128 {
            return Err(anyhow!("Invalid Reed-Solomon parameters"));
        }
        Ok(Self {
            k,
            m,
            max_chunk_size: 1_048_576, // 1MB default
        })
    }
    
    /// Get total number of chunks (data + parity)
    pub fn n(&self) -> usize {
        self.k + self.m
    }
    
    /// Calculate redundancy overhead percentage
    pub fn overhead(&self) -> f64 {
        (self.m as f64 / self.k as f64) * 100.0
    }
}

/// Reed-Solomon encoder with configurable parameters
pub struct ReedSolomonEncoder {
    config: RSConfig,
    encoder: Arc<RwLock<ReedSolomon>>,
}

impl ReedSolomonEncoder {
    /// Create new encoder with specified configuration
    pub fn new(k: usize, m: usize) -> Result<Self> {
        let config = RSConfig::new(k, m)?;
        let encoder = ReedSolomon::new(k, m)
            .map_err(|e| anyhow!("Failed to create Reed-Solomon encoder: {:?}", e))?;
        
        Ok(Self {
            config,
            encoder: Arc::new(RwLock::new(encoder)),
        })
    }
    
    /// Encode data into data + parity chunks
    pub async fn encode(&self, data: Vec<u8>) -> Result<Vec<Vec<u8>>> {
        if data.is_empty() {
            return Err(anyhow!("Cannot encode empty data"));
        }
        
        // Split data into k chunks
        let chunk_size = (data.len() + self.config.k - 1) / self.config.k;
        if chunk_size > self.config.max_chunk_size {
            return Err(anyhow!("Chunk size exceeds maximum"));
        }
        
        let mut shards = Vec::with_capacity(self.config.n());
        
        // Create data shards
        for i in 0..self.config.k {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size, data.len());
            let mut shard = data[start..end].to_vec();
            
            // Pad last shard if necessary
            if shard.len() < chunk_size {
                shard.resize(chunk_size, 0);
            }
            shards.push(shard);
        }
        
        // Add parity shards
        for _ in 0..self.config.m {
            shards.push(vec![0u8; chunk_size]);
        }
        
        // Encode
        let encoder = self.encoder.write().await;
        encoder.encode(&mut shards)
            .map_err(|e| anyhow!("Encoding failed: {:?}", e))?;
        
        Ok(shards)
    }
    
    /// Decode original data from available chunks
    pub async fn decode(&self, mut chunks: Vec<Option<Vec<u8>>>) -> Result<Vec<u8>> {
        if chunks.len() != self.config.n() {
            return Err(anyhow!("Invalid number of chunks"));
        }
        
        // Check if we have enough chunks
        let available_count = chunks.iter().filter(|c| c.is_some()).count();
        if available_count < self.config.k {
            return Err(anyhow!("Insufficient chunks for recovery: {} < {}", 
                available_count, self.config.k));
        }
        
        // Reconstruct missing chunks
        let encoder = self.encoder.write().await;
        encoder.reconstruct(&mut chunks)
            .map_err(|e| anyhow!("Reconstruction failed: {:?}", e))?;
        
        // Combine data chunks
        let mut result = Vec::new();
        for i in 0..self.config.k {
            if let Some(chunk) = &chunks[i] {
                result.extend_from_slice(chunk);
            }
        }
        
        // Remove padding
        while result.last() == Some(&0) {
            result.pop();
        }
        
        Ok(result)
    }
    
    /// Check if recovery is possible with available chunks
    pub fn can_recover(&self, available_chunks: &[bool]) -> bool {
        available_chunks.iter().filter(|&&x| x).count() >= self.config.k
    }
    
    /// Adjust redundancy based on network reliability
    pub async fn adjust_redundancy(&mut self, network_reliability: f64) -> Result<()> {
        let (new_k, new_m) = if network_reliability < 0.7 {
            (4, 4) // Conservative: 50% overhead
        } else if network_reliability < 0.9 {
            (6, 3) // Balanced: 33% overhead
        } else {
            (8, 2) // Aggressive: 25% overhead
        };
        
        if new_k != self.config.k || new_m != self.config.m {
            self.config = RSConfig::new(new_k, new_m)?;
            let new_encoder = ReedSolomon::new(new_k, new_m)
                .map_err(|e| anyhow!("Failed to adjust encoder: {:?}", e))?;
            *self.encoder.write().await = new_encoder;
        }
        
        Ok(())
    }
}

/// Manager for adaptive redundancy based on network conditions
pub struct AdaptiveRedundancyManager {
    encoder: Arc<RwLock<ReedSolomonEncoder>>,
    network_reliability: Arc<RwLock<f64>>,
}

impl AdaptiveRedundancyManager {
    /// Create new manager with initial configuration
    pub fn new(k: usize, m: usize) -> Result<Self> {
        Ok(Self {
            encoder: Arc::new(RwLock::new(ReedSolomonEncoder::new(k, m)?)),
            network_reliability: Arc::new(RwLock::new(0.9)),
        })
    }
    
    /// Update network reliability metric
    pub async fn update_reliability(&self, reliability: f64) -> Result<()> {
        *self.network_reliability.write().await = reliability.clamp(0.0, 1.0);
        
        // Adjust encoder if needed
        let mut encoder = self.encoder.write().await;
        encoder.adjust_redundancy(reliability).await?;
        
        Ok(())
    }
    
    /// Get current configuration
    pub async fn get_config(&self) -> RSConfig {
        let encoder = self.encoder.read().await;
        encoder.config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_encoding_decoding_roundtrip() -> Result<()> {
        let encoder = ReedSolomonEncoder::new(4, 2)?;
        let original_data = vec![1u8; 1000];
        
        let encoded = encoder.encode(original_data.clone()).await?;
        assert_eq!(encoded.len(), 6); // k + m
        
        // Simulate losing 2 chunks
        let mut corrupted = encoded.into_iter().map(Some).collect::<Vec<_>>();
        corrupted[0] = None;
        corrupted[1] = None;
        
        let decoded = encoder.decode(corrupted).await?;
        assert_eq!(decoded.len(), original_data.len());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_maximum_recoverable_failures() -> Result<()> {
        let encoder = ReedSolomonEncoder::new(4, 2)?;
        let data = vec![42u8; 1000];
        
        let encoded = encoder.encode(data.clone()).await?;
        
        // Can recover from m failures
        let mut chunks = encoded.clone().into_iter().map(Some).collect::<Vec<_>>();
        for i in 0..2 {
            chunks[i] = None;
        }
        assert!(encoder.decode(chunks).await.is_ok());
        
        // Cannot recover from m+1 failures
        let mut chunks = encoded.into_iter().map(Some).collect::<Vec<_>>();
        for i in 0..3 {
            chunks[i] = None;
        }
        assert!(encoder.decode(chunks).await.is_err());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_redundancy_adjustment() -> Result<()> {
        let mut encoder = ReedSolomonEncoder::new(6, 3)?;
        
        // Adjust to conservative mode
        encoder.adjust_redundancy(0.5).await?;
        assert_eq!(encoder.config.k, 4);
        assert_eq!(encoder.config.m, 4);
        
        // Adjust to aggressive mode
        encoder.adjust_redundancy(0.95).await?;
        assert_eq!(encoder.config.k, 8);
        assert_eq!(encoder.config.m, 2);
        
        Ok(())
    }
}