// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Four-word address system for identity management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Four-word address for human-readable identity
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FourWordAddress {
    address: String,
}

impl FourWordAddress {
    /// Create a new four-word address from a string
    pub fn new(address: String) -> Result<Self> {
        // Validate it has exactly 4 words
        let words: Vec<&str> = address.split('-').collect();
        if words.len() != 4 {
            anyhow::bail!("Four-word address must have exactly 4 words, got {}", words.len());
        }
        
        // Basic validation - each word should be non-empty and alphabetic
        for word in &words {
            if word.is_empty() {
                anyhow::bail!("Words in address cannot be empty");
            }
            if !word.chars().all(|c| c.is_alphabetic()) {
                anyhow::bail!("Words must contain only letters");
            }
        }
        
        Ok(Self {
            address: address.to_lowercase(),
        })
    }
    
    /// Create a new address without dictionary validation (for testing)
    #[cfg(test)]
    pub fn new_unchecked(address: String) -> Result<Self> {
        Self::new(address) // Basic format validation only
    }
    
    /// Generate a random four-word address using the four-word-networking crate
    pub fn generate() -> Result<Self> {
        use four_word_networking::FourWordAdaptiveEncoder;
        
        let encoder = FourWordAdaptiveEncoder::new()
            .map_err(|e| anyhow::anyhow!("Failed to create encoder: {}", e))?;
        
        let words = encoder.get_random_words(4);
        let address = words.join("-");
        
        Self::new(address)
    }
    
    /// Parse a four-word address from a string
    pub fn from_string(address: &str) -> Result<Self> {
        Self::new(address.to_string())
    }
    
    /// Get the full address string
    pub fn as_string(&self) -> String {
        self.address.clone()
    }
    
    /// Get individual words
    pub fn words(&self) -> Vec<String> {
        self.address
            .split('-')
            .map(|s| s.to_string())
            .collect()
    }
    
    /// Validate address format (basic validation for compatibility)
    pub fn is_valid(&self) -> bool {
        let words: Vec<&str> = self.address.split('-').collect();
        words.len() == 4 && 
        words.iter().all(|w| !w.is_empty() && w.chars().all(|c| c.is_alphabetic()))
    }
    
    /// Check if words are in the four-word-networking dictionary
    pub fn is_dictionary_valid(&self) -> bool {
        if let Ok(encoder) = four_word_networking::FourWordAdaptiveEncoder::new() {
            let words: Vec<&str> = self.address.split('-').collect();
            words.iter().all(|word| encoder.is_valid_word(word))
        } else {
            false
        }
    }
}

impl fmt::Display for FourWordAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.address)
    }
}

impl TryFrom<String> for FourWordAddress {
    type Error = anyhow::Error;
    
    fn try_from(value: String) -> Result<Self> {
        Self::from_string(&value)
    }
}

impl TryFrom<&str> for FourWordAddress {
    type Error = anyhow::Error;
    
    fn try_from(value: &str) -> Result<Self> {
        Self::from_string(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_four_word_address_generation() {
        let address = FourWordAddress::generate().unwrap();
        assert_eq!(address.words().len(), 4);
        assert!(address.is_valid());
        
        // Test that address string is properly formatted
        let addr_string = address.as_string();
        let parts: Vec<&str> = addr_string.split('-').collect();
        assert_eq!(parts.len(), 4);
        
        // Each part should be non-empty and contain only valid characters
        for part in parts {
            assert!(!part.is_empty());
            assert!(part.chars().all(|c| c.is_alphabetic()));
        }
    }
    
    #[test]
    fn test_four_word_address_from_string() {
        // Generate a valid address first, then parse it back
        let original = FourWordAddress::generate().unwrap();
        let addr_string = original.as_string();
        
        let parsed = FourWordAddress::from_string(&addr_string).unwrap();
        assert_eq!(parsed.as_string(), addr_string);
        assert_eq!(parsed.words().len(), 4);
        assert!(parsed.is_valid());
    }
    
    #[test]
    fn test_display_trait() {
        let address = FourWordAddress::generate().unwrap();
        let display_string = format!("{}", address);
        let as_string = address.as_string();
        assert_eq!(display_string, as_string);
    }
    
    #[test]
    fn test_try_from_implementations() {
        let original = FourWordAddress::generate().unwrap();
        let addr_string = original.as_string();
        
        let address1 = FourWordAddress::try_from(addr_string.as_str()).unwrap();
        let address2 = FourWordAddress::try_from(addr_string.clone()).unwrap();
        assert_eq!(address1.as_string(), address2.as_string());
    }
    
    #[test]
    fn test_validation_errors() {
        // Test too few words
        let result = FourWordAddress::new("alpha-beta-gamma".to_string());
        assert!(result.is_err());
        
        // Test too many words
        let result = FourWordAddress::new("alpha-beta-gamma-delta-echo".to_string());
        assert!(result.is_err());
        
        // Test empty word
        let result = FourWordAddress::new("alpha--gamma-delta".to_string());
        assert!(result.is_err());
        
        // Test invalid characters
        let result = FourWordAddress::new("alpha-beta2-gamma-delta".to_string());
        assert!(result.is_err());
    }
    
    #[test]
    fn test_manual_creation() {
        let address = FourWordAddress::new("alpha-beta-gamma-delta".to_string()).unwrap();
        assert_eq!(address.as_string(), "alpha-beta-gamma-delta");
        assert_eq!(address.words().len(), 4);
        assert!(address.is_valid());
        
        // Test dictionary validation separately
        let generated = FourWordAddress::generate().unwrap();
        assert_eq!(generated.words().len(), 4);
        assert!(generated.is_valid());
        assert!(generated.is_dictionary_valid()); // Generated words should be in dictionary
    }
    
    #[test]
    fn test_multiple_generation_unique() {
        // Generate multiple addresses and ensure they're different
        let addr1 = FourWordAddress::generate().unwrap();
        let addr2 = FourWordAddress::generate().unwrap();
        let addr3 = FourWordAddress::generate().unwrap();
        
        // While it's theoretically possible for addresses to be the same,
        // it's extremely unlikely with a proper word dictionary
        assert_ne!(addr1.as_string(), addr2.as_string());
        assert_ne!(addr2.as_string(), addr3.as_string());
        assert_ne!(addr1.as_string(), addr3.as_string());
    }
    
    #[test]
    fn test_dictionary_validation() {
        // Generated addresses should be dictionary valid
        let generated = FourWordAddress::generate().unwrap();
        assert!(generated.is_dictionary_valid());
        
        // Manual addresses may not be dictionary valid
        let manual = FourWordAddress::new("alpha-beta-gamma-delta".to_string()).unwrap();
        assert!(manual.is_valid()); // Basic format is valid
        // Don't assert is_dictionary_valid() as these words may not be in the dictionary
    }
}