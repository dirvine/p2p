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


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_encryption_roundtrip() {
        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let manager = IdentityManager::new(temp_dir.path(), SecurityLevel::High)
            .await
            .unwrap();
        
        // Test derive_encryption_key
        let password = SecureString::from_str("test_password").unwrap();
        let salt = b"test_salt_32_bytes_test_salt_32b";
        let key = manager.derive_encryption_key(&password, salt).unwrap();
        assert_eq!(key.len(), 32);
        
        // Test encrypt/decrypt
        let plaintext = b"Hello, World! This is a test message.";
        let nonce = [0u8; 12];
        
        let ciphertext = manager.encrypt_data(plaintext, &key, &nonce).unwrap();
        assert_ne!(ciphertext, plaintext);
        
        let decrypted = manager.decrypt_data(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}