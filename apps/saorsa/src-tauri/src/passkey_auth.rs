// Copyright 2024 MaidSafe Limited
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

//! # Passkey Authentication Module
//! 
//! Provides platform-specific biometric authentication using:
//! - TouchID on macOS
//! - Windows Hello on Windows
//! - System authentication on Linux
//! 
//! Stores credentials securely in platform keychains and provides
//! a unified interface for passkey creation and authentication.
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, error, info};

#[cfg(target_os = "windows")]
use crate::platform::windows::WindowsHelloAuth;
#[cfg(target_os = "macos")]
use crate::platform::macos::TouchIdAuth;
#[cfg(target_os = "linux")]
use crate::platform::linux::LinuxAuth;

/// Passkey credential for secure storage
/// 
/// Contains all necessary information to authenticate a user
/// using platform biometric authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPasskeyCredential {
    pub credential_id: String,
    pub public_key: Vec<u8>,
    pub counter: u32,
    pub created_at: u64,
    pub three_word_address: String,
    pub user_id: String,
}

/// Platform-specific authenticator implementations
/// 
/// Provides abstraction over different OS authentication methods
#[derive(Debug)]
pub enum PlatformAuthenticator {
    #[cfg(target_os = "windows")]
    WindowsHello(WindowsHelloAuth),
    #[cfg(target_os = "macos")]
    TouchId(TouchIdAuth),
    #[cfg(target_os = "linux")]
    Linux(LinuxAuth),
    Mock(MockAuthenticator), // For testing
}

/// Main passkey authentication manager
/// 
/// Handles creation, storage, and verification of passkey credentials
/// using platform-specific biometric authentication methods.
pub struct PasskeyAuthManager {
    data_dir: PathBuf,
    pub authenticator: PlatformAuthenticator,
    keychain_service: String,
}

impl PasskeyAuthManager {
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        let authenticator = Self::create_platform_authenticator()?;
        
        Ok(Self {
            data_dir,
            authenticator,
            keychain_service: "saorsa_p2p".to_string(),
        })
    }
    
    /// Create platform-specific authenticator based on OS
    /// 
    /// # Returns
    /// * Platform-specific authenticator implementation
    /// * Falls back to mock authenticator on unsupported platforms
    fn create_platform_authenticator() -> Result<PlatformAuthenticator> {
        #[cfg(target_os = "macos")]
        {
            Ok(PlatformAuthenticator::TouchId(TouchIdAuth::new()?))
        }
        
        #[cfg(target_os = "windows")]
        {
            Ok(PlatformAuthenticator::WindowsHello(WindowsHelloAuth::new()?))
        }
        
        #[cfg(target_os = "linux")]
        {
            Ok(PlatformAuthenticator::Linux(LinuxAuth::new()?))
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            // Fallback to mock for unsupported platforms
            Ok(PlatformAuthenticator::Mock(MockAuthenticator::new(true)))
        }
    }
    
    /// Check if platform authenticator is available and ready
    /// 
    /// # Returns
    /// * `true` if biometric authentication is available
    /// * `false` if unavailable or not configured
    pub async fn is_available(&self) -> bool {
        match &self.authenticator {
            #[cfg(target_os = "macos")]
            PlatformAuthenticator::TouchId(auth) => auth.is_available().await,
            #[cfg(target_os = "windows")]
            PlatformAuthenticator::WindowsHello(auth) => auth.is_available().await,
            #[cfg(target_os = "linux")]
            PlatformAuthenticator::Linux(auth) => auth.is_available().await,
            PlatformAuthenticator::Mock(_) => true,
        }
    }
    
    /// Create a new passkey credential with biometric protection
    /// 
    /// # Arguments
    /// * `user_id` - Unique user identifier
    /// * `three_word_address` - User's three-word address
    /// 
    /// # Returns
    /// * `StoredPasskeyCredential` - Created credential
    /// * `Error` if biometric authentication fails or storage fails
    /// 
    /// # Security
    /// - Requires user biometric authentication
    /// - Stores credential in platform keychain
    /// - Generates cryptographically secure keys
    pub async fn create_passkey(
        &self,
        user_id: &str,
        three_word_address: &str,
    ) -> Result<StoredPasskeyCredential> {
        info!("Creating passkey for user: {}", user_id);
        
        // Request biometric authentication
        self.authenticate_user("Create passkey for Saorsa").await?;
        
        // Generate credential ID and keypair
        let credential_id = self.generate_credential_id();
        let (public_key, private_key) = self.generate_keypair()?;
        
        // Store private key in OS keychain
        self.store_in_keychain(&credential_id, &private_key).await?;
        
        let credential = StoredPasskeyCredential {
            credential_id: credential_id.clone(),
            public_key,
            counter: 0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            three_word_address: three_word_address.to_string(),
            user_id: user_id.to_string(),
        };
        
        info!("Passkey created successfully: {}", credential_id);
        Ok(credential)
    }
    
    /// Authenticate with passkey
    pub async fn authenticate_with_passkey(
        &self,
        credential_id: &str,
    ) -> Result<Vec<u8>> {
        info!("Authenticating with passkey: {}", credential_id);
        
        // Request biometric authentication
        self.authenticate_user("Unlock Saorsa data").await?;
        
        // Retrieve private key from keychain
        let private_key = self.retrieve_from_keychain(credential_id).await?;
        
        // Generate authentication signature
        let challenge = self.generate_challenge();
        let signature = self.sign_challenge(&private_key, &challenge)?;
        
        Ok(signature)
    }
    
    /// Delete a passkey credential
    pub async fn delete_passkey(&self, credential_id: &str) -> Result<()> {
        info!("Deleting passkey: {}", credential_id);
        
        // Request authentication before deletion
        self.authenticate_user("Delete passkey credential").await?;
        
        // Remove from keychain
        let entry = Entry::new(&self.keychain_service, credential_id)?;
        entry.delete_password()?;
        
        debug!("Deleted credential from keychain: {}", credential_id);
        Ok(())
    }
    
    /// Platform-specific user authentication
    async fn authenticate_user(&self, reason: &str) -> Result<()> {
        match &self.authenticator {
            #[cfg(target_os = "macos")]
            PlatformAuthenticator::TouchId(auth) => auth.authenticate(reason).await,
            #[cfg(target_os = "windows")]
            PlatformAuthenticator::WindowsHello(auth) => auth.verify_user(reason).await,
            #[cfg(target_os = "linux")]
            PlatformAuthenticator::Linux(auth) => auth.authenticate(reason).await,
            PlatformAuthenticator::Mock(auth) => auth.authenticate(reason).await,
        }
    }
    
    /// Store credential in OS keychain
    async fn store_in_keychain(&self, credential_id: &str, private_key: &[u8]) -> Result<()> {
        let entry = Entry::new(&self.keychain_service, credential_id)?;
        let encoded_key = general_purpose::STANDARD.encode(private_key);
        entry.set_password(&encoded_key)?;
        debug!("Stored credential in keychain: {}", credential_id);
        Ok(())
    }
    
    /// Retrieve credential from OS keychain
    async fn retrieve_from_keychain(&self, credential_id: &str) -> Result<Vec<u8>> {
        let entry = Entry::new(&self.keychain_service, credential_id)?;
        let encoded_key = entry.get_password()?;
        let private_key = general_purpose::STANDARD.decode(&encoded_key)?;
        debug!("Retrieved credential from keychain: {}", credential_id);
        Ok(private_key)
    }
    
    /// Generate unique credential ID
    fn generate_credential_id(&self) -> String {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let bytes: [u8; 32] = rng.gen();
        general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
    }
    
    /// Generate keypair
    fn generate_keypair(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        use ed25519_dalek::{Keypair, PublicKey, SecretKey};
        use rand::rngs::OsRng;
        
        let keypair = Keypair::generate(&mut OsRng);
        let public_key = keypair.public.to_bytes().to_vec();
        let private_key = keypair.secret.to_bytes().to_vec();
        
        Ok((public_key, private_key))
    }
    
    /// Generate authentication challenge
    fn generate_challenge(&self) -> Vec<u8> {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let mut challenge = vec![0u8; 32];
        rng.fill(&mut challenge[..]);
        challenge
    }
    
    /// Sign challenge with private key
    fn sign_challenge(&self, private_key: &[u8], challenge: &[u8]) -> Result<Vec<u8>> {
        use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
        
        let secret = SecretKey::from_bytes(private_key)?;
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };
        
        let signature = keypair.sign(challenge);
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Get platform information
    pub fn get_platform_info(&self) -> String {
        match &self.authenticator {
            #[cfg(target_os = "macos")]
            PlatformAuthenticator::TouchId(_) => "macOS TouchID".to_string(),
            #[cfg(target_os = "windows")]
            PlatformAuthenticator::WindowsHello(_) => "Windows Hello".to_string(),
            #[cfg(target_os = "linux")]
            PlatformAuthenticator::Linux(_) => "Linux System Auth".to_string(),
            PlatformAuthenticator::Mock(_) => "Mock (Testing)".to_string(),
        }
    }
}

/// Mock authenticator for testing
#[derive(Debug)]
pub struct MockAuthenticator {
    should_succeed: bool,
}

impl MockAuthenticator {
    pub fn new(should_succeed: bool) -> Self {
        Self { should_succeed }
    }
    
    pub async fn authenticate(&self, reason: &str) -> Result<()> {
        info!("Mock authentication requested: {}", reason);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        if self.should_succeed {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Mock authentication failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_passkey_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        // Use mock authenticator for testing
        manager.authenticator = PlatformAuthenticator::Mock(MockAuthenticator::new(true));
        
        let credential = manager.create_passkey("test_user", "test.word.address")
            .await
            .unwrap();
        
        assert_eq!(credential.user_id, "test_user");
        assert_eq!(credential.three_word_address, "test.word.address");
        assert!(!credential.credential_id.is_empty());
        assert!(!credential.public_key.is_empty());
    }
    
    #[tokio::test]
    async fn test_authentication_flow() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        // Use mock authenticator for testing
        manager.authenticator = PlatformAuthenticator::Mock(MockAuthenticator::new(true));
        
        // Create passkey
        let credential = manager.create_passkey("test_user", "test.word.address")
            .await
            .unwrap();
        
        // Authenticate with passkey
        let signature = manager.authenticate_with_passkey(&credential.credential_id)
            .await
            .unwrap();
        
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64); // Ed25519 signature length
    }
    
    #[tokio::test]
    async fn test_authentication_failure() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        // Use mock authenticator that fails
        manager.authenticator = PlatformAuthenticator::Mock(MockAuthenticator::new(false));
        
        // Should fail to create passkey
        let result = manager.create_passkey("test_user", "test.word.address").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_platform_info() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PasskeyAuthManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        let info = manager.get_platform_info();
        assert!(!info.is_empty());
        
        // Should contain platform-specific information
        #[cfg(target_os = "macos")]
        assert!(info.contains("macOS"));
        
        #[cfg(target_os = "windows")]
        assert!(info.contains("Windows"));
        
        #[cfg(target_os = "linux")]
        assert!(info.contains("Linux"));
    }
}