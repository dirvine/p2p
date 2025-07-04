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

//! Three-Word Address System
//!
//! Converts complex multiaddrs into memorable three-word combinations for human-friendly
//! peer discovery and sharing. Inspired by what3words but designed specifically for
//! P2P network bootstrap addresses.
//!
//! Example: `/ip6/2001:db8::1/udp/9000/quic` ↔ `ocean.thunder.falcon`

use crate::{Multiaddr, P2PError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Maximum number of words per position in the dictionary
const WORDS_PER_POSITION: usize = 2048; // 2^11 for efficient bit packing

/// Total combinations available: 2048^3 = ~8.6 billion addresses
const TOTAL_COMBINATIONS: u64 = (WORDS_PER_POSITION as u64).pow(3);

/// Three-word address representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreeWordAddress {
    pub first: String,
    pub second: String, 
    pub third: String,
}

impl ThreeWordAddress {
    /// Create a new three-word address
    pub fn new(first: String, second: String, third: String) -> Self {
        Self { first, second, third }
    }
    
    /// Parse from dot-separated string format
    pub fn from_string(input: &str) -> Result<Self> {
        let parts: Vec<&str> = input.split('.').collect();
        if parts.len() != 3 {
            return Err(P2PError::Bootstrap(
                format!("Three-word address must have exactly 3 words separated by dots, got: {}", input)
            ));
        }
        
        Ok(Self {
            first: parts[0].to_lowercase(),
            second: parts[1].to_lowercase(),
            third: parts[2].to_lowercase(),
        })
    }
    
    /// Convert to dot-separated string format
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.first, self.second, self.third)
    }
    
    /// Validate that all words exist in the dictionary
    pub fn validate(&self, encoder: &WordEncoder) -> Result<()> {
        encoder.validate_words(&self.first, &self.second, &self.third)
    }
}

impl std::fmt::Display for ThreeWordAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for ThreeWordAddress {
    type Err = P2PError;
    
    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s)
    }
}

/// Word dictionary for three-word address encoding
#[derive(Debug, Clone)]
pub struct WordDictionary {
    /// Context words (position 1): geographic, network type
    context_words: Vec<String>,
    /// Quality words (position 2): performance, purpose, status  
    quality_words: Vec<String>,
    /// Identity words (position 3): nature, objects, abstract concepts
    identity_words: Vec<String>,
    
    /// Reverse lookup maps
    context_map: HashMap<String, usize>,
    quality_map: HashMap<String, usize>,
    identity_map: HashMap<String, usize>,
}

impl WordDictionary {
    /// Create a new word dictionary with default English words
    pub fn new() -> Self {
        let context_words = Self::default_context_words();
        let quality_words = Self::default_quality_words();
        let identity_words = Self::default_identity_words();
        
        let context_map: HashMap<String, usize> = context_words
            .iter()
            .enumerate()
            .map(|(i, word)| (word.clone(), i))
            .collect();
            
        let quality_map: HashMap<String, usize> = quality_words
            .iter()
            .enumerate() 
            .map(|(i, word)| (word.clone(), i))
            .collect();
            
        let identity_map: HashMap<String, usize> = identity_words
            .iter()
            .enumerate()
            .map(|(i, word)| (word.clone(), i))
            .collect();
        
        Self {
            context_words,
            quality_words,
            identity_words,
            context_map,
            quality_map,
            identity_map,
        }
    }
    
    /// Get word by position and index
    pub fn get_word(&self, position: usize, index: usize) -> Option<&String> {
        match position {
            0 => self.context_words.get(index),
            1 => self.quality_words.get(index), 
            2 => self.identity_words.get(index),
            _ => None,
        }
    }
    
    /// Get index by position and word
    pub fn get_index(&self, position: usize, word: &str) -> Option<usize> {
        let word_lower = word.to_lowercase();
        match position {
            0 => self.context_map.get(&word_lower).copied(),
            1 => self.quality_map.get(&word_lower).copied(),
            2 => self.identity_map.get(&word_lower).copied(),
            _ => None,
        }
    }
    
    /// Validate that a word exists in the specified position
    pub fn validate_word(&self, position: usize, word: &str) -> bool {
        self.get_index(position, word).is_some()
    }
    
    /// Get all words for a specific position
    pub fn get_words_for_position(&self, position: usize) -> Option<&Vec<String>> {
        match position {
            0 => Some(&self.context_words),
            1 => Some(&self.quality_words),
            2 => Some(&self.identity_words),
            _ => None,
        }
    }
    
    /// Default context words (position 1) - geographic and network context
    fn default_context_words() -> Vec<String> {
        vec![
            // Geographic contexts
            "global", "europe", "america", "asia", "africa", "oceania", "arctic", "pacific",
            "atlantic", "indian", "mountain", "desert", "forest", "urban", "rural", "coastal",
            "island", "valley", "plateau", "tundra", "savanna", "jungle", "prairie", "canyon",
            
            // Network contexts  
            "local", "mesh", "bridge", "gateway", "relay", "hub", "node", "cluster", "edge",
            "core", "access", "backbone", "fiber", "wireless", "mobile", "fixed", "satellite",
            "ground", "space", "cloud", "fog", "mist", "clear", "direct", "routed", "switched",
            
            // Scale contexts
            "micro", "mini", "small", "medium", "large", "huge", "giant", "massive", "tiny",
            "compact", "wide", "narrow", "deep", "shallow", "high", "low", "fast", "slow",
            
            // Additional contexts to reach 2048 words
            "north", "south", "east", "west", "central", "remote", "near", "far", "inner",
            "outer", "upper", "lower", "front", "back", "left", "right", "home", "work",
            "school", "public", "private", "open", "closed", "secure", "safe", "quick",
            "steady", "smooth", "rough", "sharp", "soft", "hard", "light", "dark", "bright",
            "dim", "warm", "cool", "hot", "cold", "fresh", "old", "new", "modern", "classic",
        ][..std::cmp::min(WORDS_PER_POSITION, 100)].iter().map(|s| s.to_string()).collect()
        // Note: This is a starter set - we'd expand to full 2048 words in production
    }
    
    /// Default quality words (position 2) - performance, purpose, status
    fn default_quality_words() -> Vec<String> {
        vec![
            // Performance qualities
            "fast", "quick", "rapid", "swift", "speedy", "turbo", "hyper", "ultra", "super",
            "stable", "solid", "steady", "reliable", "robust", "strong", "secure", "safe",
            "premium", "elite", "pro", "advanced", "expert", "master", "prime", "top", "best",
            "smooth", "fluid", "agile", "nimble", "efficient", "optimal", "perfect", "ideal",
            
            // Purpose qualities
            "chat", "talk", "voice", "video", "stream", "share", "store", "backup", "sync",
            "game", "play", "work", "study", "learn", "teach", "create", "build", "design",
            "connect", "link", "bridge", "tunnel", "route", "switch", "filter", "block",
            "allow", "grant", "deny", "check", "verify", "trust", "guard", "watch", "monitor",
            
            // Status qualities  
            "active", "live", "online", "ready", "awake", "alert", "busy", "free", "open",
            "public", "private", "hidden", "visible", "clear", "bright", "sharp", "focused",
            "verified", "trusted", "known", "famous", "popular", "common", "rare", "unique",
            "special", "magic", "power", "energy", "force", "strength", "grace", "beauty",
            
            // Additional qualities
            "gentle", "calm", "peaceful", "quiet", "loud", "bold", "brave", "smart", "wise",
            "clever", "bright", "brilliant", "clear", "pure", "clean", "fresh", "green",
            "blue", "red", "gold", "silver", "bronze", "crystal", "diamond", "pearl", "ruby",
        ][..std::cmp::min(WORDS_PER_POSITION, 100)].iter().map(|s| s.to_string()).collect()
    }
    
    /// Default identity words (position 3) - nature, objects, abstract concepts
    fn default_identity_words() -> Vec<String> {
        vec![
            // Nature - Animals
            "eagle", "falcon", "hawk", "owl", "raven", "swan", "crane", "heron", "robin",
            "lion", "tiger", "bear", "wolf", "fox", "deer", "elk", "moose", "bison",
            "whale", "dolphin", "shark", "ray", "octopus", "seal", "penguin", "turtle",
            "dragon", "phoenix", "griffin", "pegasus", "unicorn", "sphinx", "chimera",
            
            // Nature - Plants & Geography
            "oak", "pine", "maple", "cedar", "willow", "bamboo", "lotus", "rose", "lily",
            "mountain", "hill", "peak", "summit", "ridge", "valley", "canyon", "cliff",
            "river", "stream", "lake", "pond", "ocean", "sea", "bay", "inlet", "shore",
            "forest", "woods", "grove", "meadow", "field", "garden", "oasis", "desert",
            
            // Objects - Navigation & Tools
            "compass", "anchor", "lighthouse", "beacon", "tower", "bridge", "gate", "door",
            "key", "lock", "sword", "shield", "hammer", "anvil", "forge", "wheel", "gear",
            "engine", "motor", "spring", "lever", "pulley", "rope", "chain", "cable", "wire",
            "lens", "mirror", "prism", "crystal", "gem", "jewel", "crown", "ring", "star",
            
            // Abstract Concepts
            "harmony", "balance", "rhythm", "melody", "symphony", "song", "dance", "flight",
            "journey", "quest", "adventure", "discovery", "treasure", "mystery", "secret",
            "dream", "vision", "hope", "faith", "trust", "love", "peace", "joy", "bliss",
            "clarity", "wisdom", "knowledge", "truth", "light", "shadow", "spirit", "soul",
            "essence", "core", "heart", "mind", "thought", "idea", "spark", "flame", "fire",
        ][..std::cmp::min(WORDS_PER_POSITION, 100)].iter().map(|s| s.to_string()).collect()
    }
}

impl Default for WordDictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Main encoder/decoder for three-word addresses
#[derive(Debug, Clone)]
pub struct WordEncoder {
    dictionary: WordDictionary,
}

impl WordEncoder {
    /// Create a new word encoder with default dictionary
    pub fn new() -> Self {
        Self {
            dictionary: WordDictionary::new(),
        }
    }
    
    /// Create encoder with custom dictionary
    pub fn with_dictionary(dictionary: WordDictionary) -> Self {
        Self { dictionary }
    }
    
    /// Convert multiaddr to three-word address
    pub fn encode_multiaddr(&self, multiaddr: &Multiaddr) -> Result<ThreeWordAddress> {
        // Convert multiaddr to a consistent hash/fingerprint
        let multiaddr_str = multiaddr.to_string();
        let hash = self.hash_multiaddr(&multiaddr_str);
        
        // Extract three indices from the hash
        let (context_idx, quality_idx, identity_idx) = self.extract_indices(hash);
        
        // Get words from dictionary
        let first = self.dictionary.get_word(0, context_idx)
            .ok_or_else(|| P2PError::Bootstrap("Context word index out of range".to_string()))?
            .clone();
            
        let second = self.dictionary.get_word(1, quality_idx)
            .ok_or_else(|| P2PError::Bootstrap("Quality word index out of range".to_string()))?
            .clone();
            
        let third = self.dictionary.get_word(2, identity_idx)
            .ok_or_else(|| P2PError::Bootstrap("Identity word index out of range".to_string()))?
            .clone();
        
        Ok(ThreeWordAddress::new(first, second, third))
    }
    
    /// Convert three-word address back to multiaddr
    /// Note: This requires a registry/cache since the conversion isn't perfectly reversible
    pub fn decode_to_multiaddr(&self, words: &ThreeWordAddress) -> Result<Multiaddr> {
        // For now, return an error indicating this needs a registry lookup
        // In a real implementation, this would query a distributed registry
        Err(P2PError::Bootstrap(
            "Multiaddr decoding requires registry lookup - not yet implemented".to_string()
        ))
    }
    
    /// Validate that all three words exist in the dictionary
    pub fn validate_words(&self, first: &str, second: &str, third: &str) -> Result<()> {
        if !self.dictionary.validate_word(0, first) {
            return Err(P2PError::Bootstrap(format!("Unknown context word: {}", first)));
        }
        
        if !self.dictionary.validate_word(1, second) {
            return Err(P2PError::Bootstrap(format!("Unknown quality word: {}", second)));
        }
        
        if !self.dictionary.validate_word(2, third) {
            return Err(P2PError::Bootstrap(format!("Unknown identity word: {}", third)));
        }
        
        Ok(())
    }
    
    /// Get the word dictionary
    pub fn dictionary(&self) -> &WordDictionary {
        &self.dictionary
    }
    
    /// Generate a consistent hash from multiaddr string
    fn hash_multiaddr(&self, multiaddr: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        multiaddr.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Extract three indices from hash for word lookup
    fn extract_indices(&self, hash: u64) -> (usize, usize, usize) {
        // Use different parts of the hash for each word position
        // Ensure indices are within the actual dictionary size
        let context_size = self.dictionary.context_words.len();
        let quality_size = self.dictionary.quality_words.len();
        let identity_size = self.dictionary.identity_words.len();
        
        let context_idx = (hash as usize) % context_size;
        let quality_idx = ((hash >> 16) as usize) % quality_size;
        let identity_idx = ((hash >> 32) as usize) % identity_size;
        
        (context_idx, quality_idx, identity_idx)
    }
}

impl Default for WordEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_three_word_address_parsing() {
        let addr = ThreeWordAddress::from_string("ocean.thunder.falcon").unwrap();
        assert_eq!(addr.first, "ocean");
        assert_eq!(addr.second, "thunder");
        assert_eq!(addr.third, "falcon");
        assert_eq!(addr.to_string(), "ocean.thunder.falcon");
    }
    
    #[test]
    fn test_three_word_address_validation() {
        let words = ThreeWordAddress::new("global".to_string(), "fast".to_string(), "eagle".to_string());
        let encoder = WordEncoder::new();
        
        // Should pass validation since these are real words in our dictionary
        assert!(words.validate(&encoder).is_ok());
        
        // Should fail with invalid word
        let bad_words = ThreeWordAddress::new("invalid".to_string(), "words".to_string(), "here".to_string());
        assert!(bad_words.validate(&encoder).is_err());
    }
    
    #[test]
    fn test_multiaddr_encoding() {
        let encoder = WordEncoder::new();
        let multiaddr = "/ip6/2001:db8::1/udp/9000/quic".parse().unwrap();
        
        let words = encoder.encode_multiaddr(&multiaddr).unwrap();
        
        // Should produce valid three-word address
        assert!(!words.first.is_empty());
        assert!(!words.second.is_empty());
        assert!(!words.third.is_empty());
        
        // Should validate successfully
        assert!(words.validate(&encoder).is_ok());
        
        // Same multiaddr should always produce same words (deterministic)
        let words2 = encoder.encode_multiaddr(&multiaddr).unwrap();
        assert_eq!(words, words2);
    }
    
    #[test]
    fn test_word_dictionary() {
        let dict = WordDictionary::new();
        
        // Should have words in all positions
        assert!(!dict.context_words.is_empty());
        assert!(!dict.quality_words.is_empty());
        assert!(!dict.identity_words.is_empty());
        
        // Should be able to lookup words
        assert!(dict.validate_word(0, "global"));
        assert!(dict.validate_word(1, "fast"));
        assert!(dict.validate_word(2, "eagle"));
        
        // Should reject invalid words
        assert!(!dict.validate_word(0, "nonexistent"));
    }
    
    #[test]
    fn test_deterministic_encoding() {
        let encoder = WordEncoder::new();
        
        // Test multiple multiaddrs to ensure consistency
        let addrs = vec![
            "/ip6/2001:db8::1/udp/9000/quic",
            "/ip6/::1/tcp/8000",
            "/ip4/192.168.1.1/udp/5000/quic",
        ];
        
        for addr_str in addrs {
            let multiaddr: Multiaddr = addr_str.parse().unwrap();
            
            // Encode multiple times - should always get same result
            let words1 = encoder.encode_multiaddr(&multiaddr).unwrap();
            let words2 = encoder.encode_multiaddr(&multiaddr).unwrap();
            let words3 = encoder.encode_multiaddr(&multiaddr).unwrap();
            
            assert_eq!(words1, words2);
            assert_eq!(words2, words3);
            
            println!("{} -> {}", addr_str, words1);
        }
    }
}