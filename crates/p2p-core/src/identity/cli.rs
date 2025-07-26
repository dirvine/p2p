// Copyright 2024 P2P Foundation
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CLI commands for identity management

use super::node_identity::{NodeIdentity, IdentityData};
use crate::Result;
use std::path::Path;
use std::fs;

/// Generate a new identity with proof of work
pub fn generate_identity(difficulty: u32) -> Result<()> {
    println!("Generating new P2P identity with proof-of-work (difficulty: {})...", difficulty);
    println!("This may take a moment...\n");
    
    let start = std::time::Instant::now();
    let identity = NodeIdentity::generate(difficulty)?;
    let elapsed = start.elapsed();
    
    println!("✅ Identity generated successfully!");
    println!("⏱️  Generation time: {:?}", elapsed);
    println!("\n📋 Identity Details:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Node ID:      {}", identity.node_id());
    println!("Word Address: {}", identity.word_address());
    println!("Public Key:   {}", hex::encode(identity.public_key().as_bytes()));
    println!("PoW Nonce:    {}", identity.proof_of_work().nonce);
    println!("PoW Time:     {:?}", identity.proof_of_work().computation_time);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(())
}

/// Save identity to file
pub fn save_identity(identity: &NodeIdentity, path: &Path) -> Result<()> {
    let data = identity.export();
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| crate::P2PError::Identity(format!("Failed to serialize identity: {}", e)))?;
    
    fs::write(path, json)
        .map_err(|e| crate::P2PError::Identity(format!("Failed to write identity file: {}", e)))?;
    
    println!("✅ Identity saved to: {}", path.display());
    Ok(())
}

/// Load identity from file
pub fn load_identity(path: &Path) -> Result<NodeIdentity> {
    let json = fs::read_to_string(path)
        .map_err(|e| crate::P2PError::Identity(format!("Failed to read identity file: {}", e)))?;
    
    let data: IdentityData = serde_json::from_str(&json)
        .map_err(|e| crate::P2PError::Identity(format!("Failed to parse identity file: {}", e)))?;
    
    let identity = NodeIdentity::import(&data)?;
    
    println!("✅ Identity loaded from: {}", path.display());
    println!("Node ID: {}", identity.node_id());
    println!("Word Address: {}", identity.word_address());
    
    Ok(identity)
}

/// Display identity information
pub fn show_identity(identity: &NodeIdentity) -> Result<()> {
    println!("\n🆔 P2P Identity Information");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Node ID:       {}", identity.node_id());
    println!("Word Address:  {}", identity.word_address());
    println!("Public Key:    {}", hex::encode(identity.public_key().as_bytes()));
    println!("\nProof of Work:");
    println!("  Difficulty:  {}", identity.proof_of_work().difficulty);
    println!("  Nonce:       {}", identity.proof_of_work().nonce);
    println!("  Comp. Time:  {:?}", identity.proof_of_work().computation_time);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_save_and_load_identity() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("test_identity.json");
        
        // Generate identity
        let identity = NodeIdentity::generate(8).unwrap();
        let original_id = identity.node_id().clone();
        
        // Save
        save_identity(&identity, &identity_path).unwrap();
        
        // Load
        let loaded = load_identity(&identity_path).unwrap();
        
        // Verify
        assert_eq!(loaded.node_id(), &original_id);
        assert_eq!(loaded.word_address(), identity.word_address());
    }
}