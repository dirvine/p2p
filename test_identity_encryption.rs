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


// Test program to verify identity encryption implementation
use saorsa_core::identity_manager::{IdentityManager, IdentityCreationParams};
use saorsa_core::encrypted_key_storage::SecurityLevel;
use saorsa_core::secure_memory::SecureString;
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Identity Encryption Implementation...\n");
    
    // Create temp directory for testing
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path();
    
    // Create identity manager
    let manager = IdentityManager::new(storage_path, SecurityLevel::Fast).await?;
    
    // Initialize with password
    let password = SecureString::from_str("test_password_123!")?;
    manager.initialize(&password).await?;
    
    println!("✅ Identity manager initialized");
    
    // Create a test identity
    let params = IdentityCreationParams {
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        bio: Some("Test bio".to_string()),
        metadata: HashMap::new(),
        key_lifetime: None,
        derivation_path: None,
    };
    
    println!("Creating identity...");
    let start = std::time::Instant::now();
    let identity = manager.create_identity(&password, params).await?;
    let creation_time = start.elapsed();
    
    println!("✅ Identity created in {:?}", creation_time);
    println!("   ID: {}", identity.id);
    println!("   Display name: {:?}", identity.display_name);
    println!("   Four-word address: {}", identity.four_word_address);
    
    // Test loading the identity (this will decrypt it)
    println!("\nLoading identity from encrypted storage...");
    let start = std::time::Instant::now();
    let loaded_identity = manager.load_identity(&identity.id, &password).await?;
    let load_time = start.elapsed();
    
    println!("✅ Identity loaded and decrypted in {:?}", load_time);
    assert_eq!(identity.id, loaded_identity.id);
    assert_eq!(identity.display_name, loaded_identity.display_name);
    
    // Test wrong password
    println!("\nTesting wrong password handling...");
    let wrong_password = SecureString::from_str("wrong_password")?;
    match manager.load_identity(&identity.id, &wrong_password).await {
        Err(e) => println!("✅ Correctly rejected wrong password: {}", e),
        Ok(_) => panic!("❌ Should have failed with wrong password!"),
    }
    
    // Check encrypted file exists
    let enc_file = storage_path.join(format!("{}.enc", identity.id));
    assert!(enc_file.exists(), "Encrypted file should exist");
    println!("✅ Encrypted file exists at: {:?}", enc_file);
    
    // Check no plaintext file exists
    let json_file = storage_path.join(format!("{}.json", identity.id));
    assert!(!json_file.exists(), "Plaintext JSON file should NOT exist");
    println!("✅ No plaintext file found (good!)");
    
    // Performance check
    if creation_time.as_millis() < 10 && load_time.as_millis() < 10 {
        println!("\n✅ Performance requirement met: < 10ms overhead");
    } else {
        println!("\n⚠️  Performance may need optimization:");
        println!("   Creation: {:?}", creation_time);
        println!("   Load: {:?}", load_time);
    }
    
    println!("\n🎉 Identity encryption implementation is working correctly!");
    println!("   - Encryption/decryption functional");
    println!("   - Secure key storage integrated");
    println!("   - Wrong password properly rejected");
    println!("   - No plaintext storage");
    
    Ok(())
}