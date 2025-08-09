// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Trust relationship management for peer identities

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

use super::address::FourWordAddress;

/// Trust level for peer relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Unknown peer - no trust established
    Unknown = 0,
    /// Basic trust - peer has been seen
    Basic = 1,
    /// Verified - peer identity has been verified
    Verified = 2,
    /// Trusted - actively trusted peer
    Trusted = 3,
    /// FullyTrusted - highest level of trust
    FullyTrusted = 4,
}

impl TrustLevel {
    /// Check if trust level allows interaction
    pub fn allows_interaction(&self) -> bool {
        *self >= TrustLevel::Basic
    }
    
    /// Check if trust level allows file sharing
    pub fn allows_file_sharing(&self) -> bool {
        *self >= TrustLevel::Verified
    }
    
    /// Check if trust level allows sensitive operations
    pub fn allows_sensitive_ops(&self) -> bool {
        *self >= TrustLevel::Trusted
    }
}

/// Trust relationship with a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRelationship {
    pub peer_address: FourWordAddress,
    pub trust_level: TrustLevel,
    pub established_at: SystemTime,
    pub last_interaction: SystemTime,
    pub notes: Option<String>,
    pub verification_method: Option<String>,
}

impl TrustRelationship {
    /// Create a new trust relationship
    pub fn new(peer_address: FourWordAddress, trust_level: TrustLevel) -> Self {
        let now = SystemTime::now();
        Self {
            peer_address,
            trust_level,
            established_at: now,
            last_interaction: now,
            notes: None,
            verification_method: None,
        }
    }
    
    /// Update the last interaction time
    pub fn update_interaction(&mut self) {
        self.last_interaction = SystemTime::now();
    }
    
    /// Update trust level
    pub fn update_trust_level(&mut self, new_level: TrustLevel) {
        self.trust_level = new_level;
        self.update_interaction();
    }
    
    /// Add verification method
    pub fn add_verification(&mut self, method: String) {
        self.verification_method = Some(method);
        if self.trust_level < TrustLevel::Verified {
            self.trust_level = TrustLevel::Verified;
        }
        self.update_interaction();
    }
}

/// Manages trust relationships
#[derive(Debug)]
pub struct TrustManager {
    relationships: HashMap<String, TrustRelationship>,
}

impl TrustManager {
    /// Create a new trust manager
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
        }
    }
    
    /// Add or update a trust relationship
    pub fn add_relationship(&mut self, relationship: TrustRelationship) {
        let key = relationship.peer_address.as_string().to_string();
        self.relationships.insert(key, relationship);
    }
    
    /// Get trust level for a peer
    pub fn get_trust_level(&self, peer_address: &FourWordAddress) -> TrustLevel {
        self.relationships
            .get(&peer_address.as_string())
            .map(|r| r.trust_level)
            .unwrap_or(TrustLevel::Unknown)
    }
    
    /// Get a trust relationship
    pub fn get_relationship(&self, peer_address: &FourWordAddress) -> Option<&TrustRelationship> {
        self.relationships.get(&peer_address.as_string())
    }
    
    /// Get mutable trust relationship
    pub fn get_relationship_mut(&mut self, peer_address: &FourWordAddress) -> Option<&mut TrustRelationship> {
        self.relationships.get_mut(&peer_address.as_string())
    }
    
    /// Update trust level for a peer
    pub fn update_trust_level(&mut self, peer_address: &FourWordAddress, level: TrustLevel) -> Result<()> {
        if let Some(relationship) = self.get_relationship_mut(peer_address) {
            relationship.update_trust_level(level);
            Ok(())
        } else {
            // Create new relationship if it doesn't exist
            let relationship = TrustRelationship::new(peer_address.clone(), level);
            self.add_relationship(relationship);
            Ok(())
        }
    }
    
    /// Verify a peer
    pub fn verify_peer(&mut self, peer_address: &FourWordAddress, method: String) -> Result<()> {
        if let Some(relationship) = self.get_relationship_mut(peer_address) {
            relationship.add_verification(method);
        } else {
            let mut relationship = TrustRelationship::new(peer_address.clone(), TrustLevel::Verified);
            relationship.verification_method = Some(method);
            self.add_relationship(relationship);
        }
        Ok(())
    }
    
    /// Get all trusted peers (Trusted or FullyTrusted)
    pub fn get_trusted_peers(&self) -> Vec<&TrustRelationship> {
        self.relationships
            .values()
            .filter(|r| r.trust_level >= TrustLevel::Trusted)
            .collect()
    }
    
    /// Get all verified peers
    pub fn get_verified_peers(&self) -> Vec<&TrustRelationship> {
        self.relationships
            .values()
            .filter(|r| r.trust_level >= TrustLevel::Verified)
            .collect()
    }
    
    /// Remove a trust relationship
    pub fn remove_relationship(&mut self, peer_address: &FourWordAddress) -> bool {
        self.relationships.remove(&peer_address.as_string()).is_some()
    }
    
    /// Get all relationships
    pub fn get_all_relationships(&self) -> Vec<&TrustRelationship> {
        self.relationships.values().collect()
    }
    
    /// Load trust relationships from storage
    pub fn load_from_vec(&mut self, relationships: Vec<TrustRelationship>) {
        for relationship in relationships {
            self.add_relationship(relationship);
        }
    }
    
    /// Export trust relationships for storage
    pub fn export_to_vec(&self) -> Vec<TrustRelationship> {
        self.relationships.values().cloned().collect()
    }
}

impl Default for TrustManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trust_level_ordering() {
        assert!(TrustLevel::Unknown < TrustLevel::Basic);
        assert!(TrustLevel::Basic < TrustLevel::Verified);
        assert!(TrustLevel::Verified < TrustLevel::Trusted);
        assert!(TrustLevel::Trusted < TrustLevel::FullyTrusted);
    }
    
    #[test]
    fn test_trust_level_permissions() {
        assert!(!TrustLevel::Unknown.allows_interaction());
        assert!(TrustLevel::Basic.allows_interaction());
        assert!(!TrustLevel::Basic.allows_file_sharing());
        assert!(TrustLevel::Verified.allows_file_sharing());
        assert!(!TrustLevel::Verified.allows_sensitive_ops());
        assert!(TrustLevel::Trusted.allows_sensitive_ops());
    }
    
    #[test]
    fn test_trust_relationship_creation() {
        let address = FourWordAddress::generate().unwrap();
        let relationship = TrustRelationship::new(address.clone(), TrustLevel::Basic);
        
        assert_eq!(relationship.peer_address, address);
        assert_eq!(relationship.trust_level, TrustLevel::Basic);
        assert!(relationship.notes.is_none());
        assert!(relationship.verification_method.is_none());
    }
    
    #[test]
    fn test_trust_manager() {
        let mut manager = TrustManager::new();
        let address1 = FourWordAddress::generate().unwrap();
        let address2 = FourWordAddress::generate().unwrap();
        
        // Add relationships
        manager.update_trust_level(&address1, TrustLevel::Trusted).unwrap();
        manager.update_trust_level(&address2, TrustLevel::Basic).unwrap();
        
        // Check trust levels
        assert_eq!(manager.get_trust_level(&address1), TrustLevel::Trusted);
        assert_eq!(manager.get_trust_level(&address2), TrustLevel::Basic);
        
        // Verify peer
        manager.verify_peer(&address2, "manual".to_string()).unwrap();
        assert_eq!(manager.get_trust_level(&address2), TrustLevel::Verified);
        
        // Get trusted peers
        let trusted = manager.get_trusted_peers();
        assert_eq!(trusted.len(), 1);
        assert_eq!(trusted[0].peer_address, address1);
        
        // Remove relationship
        assert!(manager.remove_relationship(&address1));
        assert_eq!(manager.get_trust_level(&address1), TrustLevel::Unknown);
    }
    
    #[test]
    fn test_trust_manager_export_import() {
        let mut manager1 = TrustManager::new();
        let address = FourWordAddress::generate().unwrap();
        
        manager1.update_trust_level(&address, TrustLevel::Trusted).unwrap();
        manager1.verify_peer(&address, "test".to_string()).unwrap();
        
        // Export
        let exported = manager1.export_to_vec();
        assert_eq!(exported.len(), 1);
        
        // Import into new manager
        let mut manager2 = TrustManager::new();
        manager2.load_from_vec(exported);
        
        assert_eq!(manager2.get_trust_level(&address), TrustLevel::Trusted);
        let rel = manager2.get_relationship(&address).unwrap();
        assert_eq!(rel.verification_method, Some("test".to_string()));
    }
}