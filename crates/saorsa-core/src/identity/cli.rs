// Copyright (c) 2025 Saorsa Labs Limited

// This file is part of the Saorsa P2P network.

// Licensed under the AGPL-3.0 license:
// <https://www.gnu.org/licenses/agpl-3.0.html>

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


// Copyright 2024 P2P Foundation
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CLI commands for identity management

use super::node_identity::{IdentityData, NodeIdentity};
use crate::Result;
use std::fs;
use std::path::Path;
use tracing::info;

/// Generate a new identity with proof of work
pub fn generate_identity(difficulty: u32) -> Result<()> {
    info!(
        "Generating new P2P identity with proof-of-work (difficulty: {})...",
        difficulty
    );
    info!("This may take a moment...");

    let start = std::time::Instant::now();
    let identity = NodeIdentity::generate(difficulty)?;
    let elapsed = start.elapsed();

    info!("✅ Identity generated successfully!");
    info!("⏱️  Generation time: {:?}", elapsed);
    info!("📋 Identity Details:");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Node ID:      {}", identity.node_id());
    info!("Word Address: {}", identity.word_address());
    info!(
        "Public Key:   {}",
        hex::encode(identity.public_key().as_bytes())
    );
    info!("PoW Nonce:    {}", identity.proof_of_work().nonce);
    info!(
        "PoW Time:     {:?}",
        identity.proof_of_work().computation_time
    );
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Save identity to file
pub fn save_identity(identity: &NodeIdentity, path: &Path) -> Result<()> {
    let data = identity.export();
    let json = serde_json::to_string_pretty(&data).map_err(|e| {
        crate::P2PError::Identity(crate::error::IdentityError::InvalidFormat(
            format!("Failed to serialize identity: {}", e).into(),
        ))
    })?;

    fs::write(path, json).map_err(|e| {
        crate::P2PError::Identity(crate::error::IdentityError::InvalidFormat(
            format!("Failed to write identity file: {}", e).into(),
        ))
    })?;

    info!("✅ Identity saved to: {}", path.display());
    Ok(())
}

/// Load identity from file
pub fn load_identity(path: &Path) -> Result<NodeIdentity> {
    let json = fs::read_to_string(path).map_err(|e| {
        crate::P2PError::Identity(crate::error::IdentityError::InvalidFormat(
            format!("Failed to read identity file: {}", e).into(),
        ))
    })?;

    let data: IdentityData = serde_json::from_str(&json).map_err(|e| {
        crate::P2PError::Identity(crate::error::IdentityError::InvalidFormat(
            format!("Failed to parse identity file: {}", e).into(),
        ))
    })?;

    let identity = NodeIdentity::import(&data)?;

    info!("✅ Identity loaded from: {}", path.display());
    info!("Node ID: {}", identity.node_id());
    info!("Word Address: {}", identity.word_address());

    Ok(identity)
}

/// Display identity information
pub fn show_identity(identity: &NodeIdentity) -> Result<()> {
    info!("🆔 P2P Identity Information");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Node ID:       {}", identity.node_id());
    info!("Word Address:  {}", identity.word_address());
    info!(
        "Public Key:    {}",
        hex::encode(identity.public_key().as_bytes())
    );
    info!("Proof of Work:");
    info!("  Difficulty:  {}", identity.proof_of_work().difficulty);
    info!("  Nonce:       {}", identity.proof_of_work().nonce);
    info!(
        "  Comp. Time:  {:?}",
        identity.proof_of_work().computation_time
    );
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_identity() {
        let temp_dir = TempDir::new().expect("Should create temp directory for test");
        let identity_path = temp_dir.path().join("test_identity.json");

        // Generate identity
        let identity = NodeIdentity::generate(8).expect("Should generate identity in test");
        let original_id = identity.node_id().clone();

        // Save
        save_identity(&identity, &identity_path).expect("Should save identity in test");

        // Load
        let loaded = load_identity(&identity_path).expect("Should load identity in test");

        // Verify
        assert_eq!(loaded.node_id(), &original_id);
        assert_eq!(loaded.word_address(), identity.word_address());
    }
}
