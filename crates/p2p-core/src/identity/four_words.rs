// Copyright 2024 P2P Foundation
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Four-word address implementation
//! 
//! This is a placeholder implementation until the four-word-networking crate is available.
//! The real implementation will use the actual crate.

use crate::{P2PError, Result};
use std::fmt;

/// Four-word address for human-readable network identification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FourWordAddress {
    words: Vec<String>,
    formatted: String,
}

impl FourWordAddress {
    /// Create from node ID bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Placeholder implementation
        // Real implementation will use four-word-networking crate
        
        // Use first 8 bytes of hash for deterministic word selection
        let hash = blake3::hash(bytes);
        let hash_bytes = hash.as_bytes();
        
        // Simple word list for testing
        let words = [
            "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
            "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa",
            "quebec", "romeo", "sierra", "tango", "uniform", "victor", "whiskey", "xray",
            "yankee", "zulu", "able", "baker", "cast", "dog", "easy", "fox",
        ];
        
        // Select 4 words based on hash
        let mut selected_words = Vec::new();
        for i in 0..4 {
            let index = hash_bytes[i] as usize % words.len();
            selected_words.push(words[index].to_string());
        }
        
        let formatted = selected_words.join("-");
        Ok(Self {
            words: selected_words,
            formatted,
        })
    }
    
    /// Parse from string format
    pub fn from_str(s: &str) -> Result<Self> {
        let words: Vec<String> = s.split('-')
            .map(|w| w.to_lowercase())
            .collect();
            
        if words.len() != 4 {
            return Err(P2PError::Identity(
                format!("Four-word address must have exactly 4 words, got {}", words.len())
            ));
        }
        
        // TODO: Validate words against dictionary when real crate is available
        
        let formatted = words.join("-");
        Ok(Self { words, formatted })
    }
    
    /// Get as hyphen-separated string
    pub fn as_str(&self) -> &str {
        &self.formatted
    }
    
    /// Get individual words
    pub fn words(&self) -> &[String] {
        &self.words
    }
}

impl fmt::Display for FourWordAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Word encoder for four-word addresses
pub struct WordEncoder;

impl WordEncoder {
    /// Encode bytes to four-word address
    pub fn encode(bytes: &[u8]) -> Result<FourWordAddress> {
        FourWordAddress::from_bytes(bytes)
    }
    
    /// Decode four-word address to bytes (returns hash prefix)
    pub fn decode(_addr: &FourWordAddress) -> Result<Vec<u8>> {
        // Placeholder - real implementation will decode to original bytes
        Err(P2PError::Identity("Decoding not implemented in placeholder".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_four_word_generation() {
        let data = b"test data";
        let addr = FourWordAddress::from_bytes(data).unwrap();
        
        // Should have 4 words
        assert_eq!(addr.words().len(), 4);
        
        // Should be deterministic
        let addr2 = FourWordAddress::from_bytes(data).unwrap();
        assert_eq!(addr, addr2);
    }
    
    #[test]
    fn test_four_word_parsing() {
        let addr_str = "alpha-bravo-charlie-delta";
        let addr = FourWordAddress::from_str(addr_str).unwrap();
        
        assert_eq!(addr.words().len(), 4);
        assert_eq!(addr.as_str(), addr_str);
    }
    
    #[test]
    fn test_invalid_word_count() {
        let addr_str = "alpha-bravo-charlie";
        let result = FourWordAddress::from_str(addr_str);
        
        assert!(result.is_err());
    }
}