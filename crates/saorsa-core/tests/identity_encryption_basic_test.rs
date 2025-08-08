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


// SPDX-License-Identifier: AGPL-3.0-only
// (c) 2025 Saorsa P2P Foundation

use saorsa_core::{error::P2PError, identity_manager::*, secure_memory::SecureString};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio;

/// Basic test that identity is encrypted when saved to disk
#[tokio::test]
async fn test_basic_identity_encryption() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let password = SecureString::from_str("test_password_123!").unwrap();
    manager.initialize(&password).await?;

    // Create an identity
    let params = IdentityCreationParams {
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        bio: Some("Test bio".to_string()),
        metadata: HashMap::new(),
        key_lifetime: None,
        derivation_path: None,
    };

    let identity = manager.create_identity(&password, params).await?;
    let identity_id = identity.id.clone();

    // Read the raw file from disk
    let identity_path = temp_dir.path().join(format!("{}.enc", identity_id));
    assert!(identity_path.exists(), "Encrypted file should exist");

    let raw_data = tokio::fs::read(&identity_path).await?;
    let raw_str = String::from_utf8_lossy(&raw_data);

    // Verify that sensitive data is not visible in plaintext
    assert!(
        !raw_str.contains("Test User"),
        "Display name should be encrypted"
    );
    assert!(!raw_str.contains("Test bio"), "Bio should be encrypted");

    // Verify we can still load and decrypt the identity
    let loaded = manager.load_identity(&identity_id, &password).await?;
    assert_eq!(loaded.display_name, Some("Test User".to_string()));
    assert_eq!(loaded.bio, Some("Test bio".to_string()));

    Ok(())
}

/// Test migration of existing plaintext identities
#[tokio::test]
async fn test_basic_migration() -> Result<(), P2PError> {
    let temp_dir = TempDir::new().unwrap();

    // Create a mock plaintext identity file
    let identity_id = UserId::generate();
    let plaintext_identity = Identity {
        id: identity_id.clone(),
        four_word_address: "test.word.address.here".to_string(),
        state: IdentityState::Active,
        display_name: Some("Plaintext User".to_string()),
        avatar_url: None,
        bio: Some("Plaintext bio".to_string()),
        metadata: HashMap::new(),
        key_version: 1,
        created_at: 1234567890,
        updated_at: 1234567890,
        expires_at: 9999999999,
        previous_keys: vec![],
        revocation_cert: None,
    };

    // Write plaintext identity to disk
    let identity_path = temp_dir.path().join(format!("{}.json", identity_id));
    let plaintext_data = serde_json::to_vec_pretty(&plaintext_identity)?;
    tokio::fs::write(&identity_path, &plaintext_data).await?;

    // Initialize manager and migrate
    let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::Fast).await?;

    let password = SecureString::from_str("migration_password_123!").unwrap();
    manager.initialize(&password).await?;

    // Migrate existing identities
    manager.migrate_existing_identities(&password).await?;

    // Verify old file was removed
    assert!(
        !identity_path.exists(),
        "Old plaintext file should be removed"
    );

    // Verify new encrypted file exists
    let encrypted_path = temp_dir.path().join(format!("{}.enc", identity_id));
    assert!(encrypted_path.exists(), "New encrypted file should exist");

    Ok(())
}

use serde::{Deserialize, Serialize};
