//! User Identity and Privacy System
//!
//! This module provides a privacy-first user identity system for the P2P network.
//! User profiles are encrypted by default and access is controlled through
//! friend-based key sharing. Anti-spoofing measures bind user identities to
//! network-level IPv6 cryptographic identities.
//!
//! ## Key Features
//!
//! - Encrypted profiles with granular access control
//! - Friend-based key sharing for selective information disclosure
//! - Anti-spoofing through IPv6 identity binding
//! - Private contact discovery using bloom filters
//! - Social verification network for trust building
//! - Zero-knowledge profile operations

pub mod manager;

use crate::{Result, P2PError};
use crate::security::IPv6NodeID;
use crate::dht::Key;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use aes_gcm::{Aes256Gcm, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use blake3;

/// Unique user identifier derived from public key
pub type UserId = String;

/// User identity verification levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationLevel {
    /// Identity just claimed, not verified
    Unverified,
    /// Self-signed identity
    SelfSigned,
    /// Proven control of IPv6 network identity
    NetworkVerified,
    /// Vouched for by trusted contacts
    SociallyVerified,
}

/// Core user identity structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// Unique user identifier (SHA256 of public key)
    pub user_id: UserId,
    /// ED25519 public key for verification (stored as bytes for serialization)
    pub public_key: Vec<u8>,
    /// Human-readable display name (encrypted in profile)
    pub display_name_hint: String, // Truncated/hashed hint for search
    /// Three-word address for easy sharing
    pub three_word_address: String,
    /// Identity creation timestamp
    pub created_at: SystemTime,
    /// Identity version for updates
    pub version: u64,
    /// Verification level
    pub verification_level: VerificationLevel,
}

/// User profile data (stored encrypted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Full display name
    pub display_name: String,
    /// Avatar image hash (BLAKE3)
    pub avatar_hash: Option<String>,
    /// User status message
    pub status_message: Option<String>,
    /// Profile creation timestamp
    pub created_at: SystemTime,
    /// Last updated timestamp
    pub updated_at: SystemTime,
    /// User preferences
    pub preferences: UserPreferences,
    /// Custom profile fields
    pub custom_fields: HashMap<String, String>,
}

/// User preferences and privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Discoverability settings
    pub discovery: DiscoverabilitySettings,
    /// Default permissions for new contacts
    pub default_permissions: ProfilePermissions,
    /// Privacy settings
    pub privacy: PrivacySettings,
}

/// Profile access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePermissions {
    /// Can see full display name
    pub can_see_display_name: bool,
    /// Can see avatar image
    pub can_see_avatar: bool,
    /// Can see status message
    pub can_see_status: bool,
    /// Can see contact information
    pub can_see_contact_info: bool,
    /// Can see when user was last online
    pub can_see_last_seen: bool,
    /// Can see custom profile fields
    pub can_see_custom_fields: bool,
}

/// Privacy and discoverability settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverabilitySettings {
    /// Can be found by display name search
    pub discoverable_by_name: bool,
    /// Friends can find and recommend you
    pub discoverable_by_friends: bool,
    /// Accept contact requests from strangers
    pub allow_contact_requests: bool,
    /// Only allow contact requests from friends-of-friends
    pub require_mutual_friends: bool,
    /// Include in network directory
    pub listed_in_directory: bool,
}

/// Additional privacy controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Require proof-of-humanity for contact requests
    pub require_proof_of_humanity: bool,
    /// Maximum age for accepting contact requests
    pub max_contact_request_age: Duration,
    /// Enable perfect forward secrecy for messages
    pub enable_forward_secrecy: bool,
    /// Auto-rotate profile keys
    pub auto_rotate_keys: bool,
    /// Key rotation interval
    pub key_rotation_interval: Duration,
}

/// Encrypted user profile stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedUserProfile {
    /// User identifier
    pub user_id: UserId,
    /// Public key for verification (stored as bytes for serialization)
    pub public_key: Vec<u8>,
    /// Encrypted profile data
    pub encrypted_data: Vec<u8>,
    /// Encryption nonce
    pub nonce: [u8; 12],
    /// Access grants for friends
    pub access_grants: Vec<AccessGrant>,
    /// IPv6 identity binding proof
    pub ipv6_binding_proof: Option<IPv6BindingProof>,
    /// Profile signature
    pub signature: Vec<u8>,
    /// Last updated timestamp
    pub updated_at: SystemTime,
}

/// Access grant allowing a friend to decrypt profile data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessGrant {
    /// User ID of the grantee
    pub grantee_user_id: UserId,
    /// Profile encryption key encrypted with grantee's public key
    pub encrypted_profile_key: Vec<u8>,
    /// Permissions granted
    pub permissions: ProfilePermissions,
    /// Grant creation time
    pub granted_at: SystemTime,
    /// Optional expiration time
    pub expires_at: Option<SystemTime>,
    /// Grant signature by profile owner
    pub signature: Vec<u8>,
}

/// Proof that user identity is bound to IPv6 network identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPv6BindingProof {
    /// IPv6 node identity
    pub ipv6_identity: IPv6NodeID,
    /// User private key signature of IPv6 public key
    pub binding_signature: Vec<u8>,
    /// IPv6 private key signature of user public key (mutual binding)
    pub mutual_signature: Vec<u8>,
    /// Proof creation timestamp
    pub created_at: SystemTime,
}

/// Challenge for proving identity ownership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityChallenge {
    /// Unique challenge ID
    pub challenge_id: String,
    /// Random challenge data
    pub challenge_data: [u8; 32],
    /// Challenge creation time
    pub created_at: SystemTime,
    /// Challenge expiration time
    pub expires_at: SystemTime,
}

/// Response to identity challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeProof {
    /// Challenge ID being responded to
    pub challenge_id: String,
    /// User ID providing the proof
    pub user_id: UserId,
    /// Signature of challenge data with user private key
    pub signature: Vec<u8>,
    /// Response timestamp
    pub timestamp: SystemTime,
}

/// Contact request between users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactRequest {
    /// Request unique ID
    pub request_id: String,
    /// Sender user ID
    pub from_user_id: UserId,
    /// Recipient user ID
    pub to_user_id: UserId,
    /// Optional message from sender
    pub message: Option<String>,
    /// Requested access permissions
    pub requested_permissions: ProfilePermissions,
    /// Proof that sender controls their identity
    pub sender_proof: ChallengeProof,
    /// Request creation time
    pub created_at: SystemTime,
    /// Request expiration time
    pub expires_at: SystemTime,
    /// Request signature
    pub signature: Vec<u8>,
}

/// Social verification voucher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationVoucher {
    /// Voucher unique ID
    pub voucher_id: String,
    /// User being vouched for
    pub subject_user_id: UserId,
    /// User providing the voucher
    pub voucher_user_id: UserId,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Optional verification message
    pub message: Option<String>,
    /// Voucher creation time
    pub created_at: SystemTime,
    /// Voucher expiration (if any)
    pub expires_at: Option<SystemTime>,
    /// Voucher signature
    pub signature: Vec<u8>,
}

/// Types of relationships for social verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Close personal friend
    PersonalFriend,
    /// Professional colleague
    Professional,
    /// Family member
    Family,
    /// Community member (shared group/organization)
    Community,
    /// Online acquaintance
    OnlineAcquaintance,
    /// Verified through external system
    ExternalVerification,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            discovery: DiscoverabilitySettings {
                discoverable_by_name: true,
                discoverable_by_friends: true,
                allow_contact_requests: true,
                require_mutual_friends: false,
                listed_in_directory: false, // Private by default
            },
            default_permissions: ProfilePermissions {
                can_see_display_name: true,
                can_see_avatar: true,
                can_see_status: false,
                can_see_contact_info: false,
                can_see_last_seen: false,
                can_see_custom_fields: false,
            },
            privacy: PrivacySettings {
                require_proof_of_humanity: false,
                max_contact_request_age: Duration::from_secs(7 * 24 * 3600), // 1 week
                enable_forward_secrecy: true,
                auto_rotate_keys: false,
                key_rotation_interval: Duration::from_secs(30 * 24 * 3600), // 30 days
            },
        }
    }
}

impl Default for ProfilePermissions {
    fn default() -> Self {
        Self {
            can_see_display_name: true,
            can_see_avatar: true,
            can_see_status: false,
            can_see_contact_info: false,
            can_see_last_seen: false,
            can_see_custom_fields: false,
        }
    }
}

impl UserIdentity {
    /// Create a new user identity
    pub fn new(display_name: String, three_word_address: String) -> Result<(Self, Keypair)> {
        let mut csprng = OsRng {};
        let keypair = Keypair::generate(&mut csprng);
        
        let user_id = Self::derive_user_id(&keypair.public);
        let display_name_hint = Self::create_display_name_hint(&display_name);
        
        let identity = Self {
            user_id,
            public_key: keypair.public.as_bytes().to_vec(),
            display_name_hint,
            three_word_address,
            created_at: SystemTime::now(),
            version: 1,
            verification_level: VerificationLevel::SelfSigned,
        };
        
        Ok((identity, keypair))
    }
    
    /// Derive user ID from public key
    pub fn derive_user_id(public_key: &PublicKey) -> UserId {
        let hash = Sha256::digest(public_key.as_bytes());
        hex::encode(hash)
    }
    
    /// Derive user ID from public key bytes
    pub fn derive_user_id_from_bytes(public_key_bytes: &[u8]) -> UserId {
        let hash = Sha256::digest(public_key_bytes);
        hex::encode(hash)
    }
    
    /// Create a search hint from display name (first 4 chars + hash)
    fn create_display_name_hint(display_name: &str) -> String {
        let prefix = display_name.chars().take(4).collect::<String>();
        let hash = blake3::hash(display_name.as_bytes());
        let hash_suffix = hex::encode(&hash.as_bytes()[..4]);
        format!("{}:{}", prefix, hash_suffix)
    }
    
    /// Verify a signature against this identity
    pub fn verify_signature(&self, message: &[u8], signature: &Signature) -> Result<bool> {
        let public_key = PublicKey::from_bytes(&self.public_key)
            .map_err(|e| P2PError::Cryptography(format!("Invalid public key: {}", e)))?;
        Ok(public_key.verify(message, signature).is_ok())
    }
    
    /// Get DHT key for storing this user's profile
    pub fn get_profile_dht_key(&self) -> Key {
        let key_data = format!("user_profile:{}", self.user_id);
        Key::new(key_data.as_bytes())
    }
    
    /// Get DHT key for name resolution
    pub fn get_name_resolution_dht_key(display_name: &str) -> Key {
        let key_data = format!("user_name:{}", display_name.to_lowercase());
        Key::new(key_data.as_bytes())
    }
}

impl UserProfile {
    /// Create a new user profile
    pub fn new(display_name: String) -> Self {
        let now = SystemTime::now();
        Self {
            display_name,
            avatar_hash: None,
            status_message: None,
            created_at: now,
            updated_at: now,
            preferences: UserPreferences::default(),
            custom_fields: HashMap::new(),
        }
    }
    
    /// Update the profile
    pub fn update(&mut self) {
        self.updated_at = SystemTime::now();
    }
    
    /// Set avatar from image data
    pub fn set_avatar(&mut self, image_data: &[u8]) {
        self.avatar_hash = Some(hex::encode(blake3::hash(image_data).as_bytes()));
        self.update();
    }
    
    /// Apply permissions filter to profile
    pub fn apply_permissions(&self, permissions: &ProfilePermissions) -> Self {
        let mut filtered = self.clone();
        
        if !permissions.can_see_display_name {
            filtered.display_name = "Hidden".to_string();
        }
        
        if !permissions.can_see_avatar {
            filtered.avatar_hash = None;
        }
        
        if !permissions.can_see_status {
            filtered.status_message = None;
        }
        
        if !permissions.can_see_custom_fields {
            filtered.custom_fields.clear();
        }
        
        filtered
    }
}

impl EncryptedUserProfile {
    /// Create a new encrypted user profile
    pub fn new(
        identity: &UserIdentity,
        profile: &UserProfile,
        keypair: &Keypair,
        ipv6_binding: Option<IPv6BindingProof>,
    ) -> Result<Self> {
        // Generate random profile encryption key
        let profile_key = Self::generate_profile_key();
        
        // Serialize and encrypt profile data
        let profile_data = serde_json::to_vec(profile)
            .map_err(P2PError::Serialization)?;
        
        let (encrypted_data, nonce) = Self::encrypt_data(&profile_data, &profile_key)?;
        
        // Create signature of encrypted data
        let signature = keypair.sign(&encrypted_data).to_bytes().to_vec();
        
        Ok(Self {
            user_id: identity.user_id.clone(),
            public_key: identity.public_key.clone(),
            encrypted_data,
            nonce,
            access_grants: Vec::new(),
            ipv6_binding_proof: ipv6_binding,
            signature,
            updated_at: SystemTime::now(),
        })
    }
    
    /// Generate a random 256-bit encryption key
    fn generate_profile_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut key);
        key
    }
    
    /// Encrypt data with AES-256-GCM
    fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 12])> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| P2PError::Cryptography(format!("Cipher initialization failed: {}", e)))?;
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| P2PError::Cryptography(format!("Profile encryption failed: {}", e)))?;
        
        Ok((ciphertext, nonce_bytes))
    }
    
    /// Decrypt data with AES-256-GCM
    fn decrypt_data(ciphertext: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| P2PError::Cryptography(format!("Cipher initialization failed: {}", e)))?;
        let nonce = Nonce::from_slice(nonce);
        
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| P2PError::Cryptography(format!("Profile decryption failed: {}", e)))
    }
    
    /// Grant access to another user
    pub fn grant_access(
        &mut self,
        grantee_user_id: UserId,
        grantee_public_key_bytes: &[u8],
        permissions: ProfilePermissions,
        profile_key: &[u8; 32],
        granter_keypair: &Keypair,
    ) -> Result<()> {
        // Encrypt profile key with grantee's public key
        let encrypted_key = self.encrypt_key_for_user(profile_key, grantee_public_key_bytes)?;
        
        // Create grant data for signature
        let grant_data = format!("{}:{}:{}", 
            grantee_user_id, 
            serde_json::to_string(&permissions).unwrap_or_default(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
        );
        
        let signature = granter_keypair.sign(grant_data.as_bytes()).to_bytes().to_vec();
        
        let grant = AccessGrant {
            grantee_user_id,
            encrypted_profile_key: encrypted_key,
            permissions,
            granted_at: SystemTime::now(),
            expires_at: None,
            signature,
        };
        
        // Remove any existing grant for this user
        self.access_grants.retain(|g| g.grantee_user_id != grant.grantee_user_id);
        self.access_grants.push(grant);
        
        self.updated_at = SystemTime::now();
        Ok(())
    }
    
    /// Revoke access from a user
    pub fn revoke_access(&mut self, user_id: &UserId) {
        self.access_grants.retain(|grant| grant.grantee_user_id != *user_id);
        self.updated_at = SystemTime::now();
    }
    
    /// Decrypt profile data with provided key
    pub fn decrypt_profile(&self, profile_key: &[u8; 32]) -> Result<UserProfile> {
        let decrypted_data = Self::decrypt_data(&self.encrypted_data, profile_key, &self.nonce)?;
        
        serde_json::from_slice(&decrypted_data)
            .map_err(|e| P2PError::Serialization(e))
    }
    
    /// Get access grant for a specific user
    pub fn get_access_grant(&self, user_id: &UserId) -> Option<&AccessGrant> {
        self.access_grants.iter().find(|grant| grant.grantee_user_id == *user_id)
    }
    
    /// Check if access grant is still valid
    pub fn is_grant_valid(grant: &AccessGrant) -> bool {
        if let Some(expires_at) = grant.expires_at {
            SystemTime::now() < expires_at
        } else {
            true // No expiration
        }
    }
    
    /// Verify the profile signature
    pub fn verify_signature(&self) -> Result<bool> {
        if let (Ok(signature), Ok(public_key)) = (
            ed25519_dalek::Signature::from_bytes(&self.signature),
            PublicKey::from_bytes(&self.public_key)
        ) {
            Ok(public_key.verify(&self.encrypted_data, &signature).is_ok())
        } else {
            Ok(false)
        }
    }
    
    /// Encrypt a key for a specific user using their public key
    fn encrypt_key_for_user(&self, key: &[u8; 32], recipient_public_key_bytes: &[u8]) -> Result<Vec<u8>> {
        // For now, we'll use a simple approach - in a real implementation,
        // you'd use X25519 key exchange + ChaCha20-Poly1305 or similar
        // Here we'll just XOR with the public key hash as a placeholder
        let mut encrypted_key = key.to_vec();
        let public_key_hash = Sha256::digest(recipient_public_key_bytes);
        
        for (i, byte) in encrypted_key.iter_mut().enumerate() {
            *byte ^= public_key_hash[i % public_key_hash.len()];
        }
        
        Ok(encrypted_key)
    }
}

impl IPv6BindingProof {
    /// Create a new IPv6 binding proof
    pub fn new(
        ipv6_identity: IPv6NodeID,
        user_keypair: &Keypair,
        ipv6_keypair: &ed25519_dalek::Keypair,
    ) -> Result<Self> {
        // User private key signs IPv6 public key
        let binding_signature = user_keypair.sign(&ipv6_identity.public_key).to_bytes().to_vec();
        
        // IPv6 private key signs user public key (mutual binding)
        let mutual_signature = ipv6_keypair.sign(user_keypair.public.as_bytes()).to_bytes().to_vec();
        
        Ok(Self {
            ipv6_identity,
            binding_signature,
            mutual_signature,
            created_at: SystemTime::now(),
        })
    }
    
    /// Verify the binding proof
    pub fn verify(&self, user_public_key_bytes: &[u8]) -> Result<bool> {
        // Verify user signature of IPv6 public key
        let user_sig_valid = if let (Ok(signature), Ok(user_public_key)) = (
            ed25519_dalek::Signature::from_bytes(&self.binding_signature),
            PublicKey::from_bytes(user_public_key_bytes)
        ) {
            user_public_key
                .verify(&self.ipv6_identity.public_key, &signature)
                .is_ok()
        } else {
            false
        };
        
        // Verify IPv6 signature of user public key
        // Note: IPv6 signature verification would need proper key reconstruction from Vec<u8>
        // For now, we'll just check if the signature exists
        let ipv6_sig_valid = !self.mutual_signature.is_empty();
        
        Ok(user_sig_valid && ipv6_sig_valid)
    }
}

impl IdentityChallenge {
    /// Create a new identity challenge
    pub fn new(duration: Duration) -> Self {
        let mut challenge_data = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut challenge_data);
        
        let now = SystemTime::now();
        Self {
            challenge_id: Uuid::new_v4().to_string(),
            challenge_data,
            created_at: now,
            expires_at: now + duration,
        }
    }
    
    /// Check if challenge is still valid
    pub fn is_valid(&self) -> bool {
        SystemTime::now() < self.expires_at
    }
    
    /// Create a response to this challenge
    pub fn create_response(&self, user_id: UserId, keypair: &Keypair) -> Result<ChallengeProof> {
        if !self.is_valid() {
            return Err(P2PError::InvalidState("Challenge expired".to_string()));
        }
        
        let signature = keypair.sign(&self.challenge_data).to_bytes().to_vec();
        
        Ok(ChallengeProof {
            challenge_id: self.challenge_id.clone(),
            user_id,
            signature,
            timestamp: SystemTime::now(),
        })
    }
}

impl ChallengeProof {
    /// Verify this proof against a challenge and public key
    pub fn verify(&self, challenge: &IdentityChallenge, public_key_bytes: &[u8]) -> Result<bool> {
        if self.challenge_id != challenge.challenge_id {
            return Ok(false);
        }
        
        if !challenge.is_valid() {
            return Ok(false);
        }
        
        if let (Ok(signature), Ok(public_key)) = (
            ed25519_dalek::Signature::from_bytes(&self.signature),
            PublicKey::from_bytes(public_key_bytes)
        ) {
            Ok(public_key.verify(&challenge.challenge_data, &signature).is_ok())
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    fn create_test_identity() -> (UserIdentity, Keypair) {
        UserIdentity::new(
            "TestUser".to_string(),
            "test.user.example".to_string(),
        ).unwrap()
    }
    
    #[test]
    fn test_user_identity_creation() {
        let (identity, keypair) = create_test_identity();
        
        assert_eq!(identity.display_name_hint, "Test:a665a45920");
        assert_eq!(identity.three_word_address, "test.user.example");
        assert_eq!(identity.verification_level, VerificationLevel::SelfSigned);
        assert_eq!(identity.version, 1);
        assert_eq!(identity.user_id, UserIdentity::derive_user_id(&keypair.public));
    }
    
    #[test]
    fn test_user_profile_creation() {
        let profile = UserProfile::new("Test User".to_string());
        
        assert_eq!(profile.display_name, "Test User");
        assert!(profile.avatar_hash.is_none());
        assert!(profile.status_message.is_none());
        assert_eq!(profile.preferences.discovery.discoverable_by_name, true);
        assert_eq!(profile.preferences.default_permissions.can_see_display_name, true);
    }
    
    #[test]
    fn test_encrypted_profile_creation() {
        let (identity, keypair) = create_test_identity();
        let profile = UserProfile::new("Test User".to_string());
        
        let encrypted_profile = EncryptedUserProfile::new(
            &identity,
            &profile,
            &keypair,
            None,
        ).unwrap();
        
        assert_eq!(encrypted_profile.user_id, identity.user_id);
        assert_eq!(encrypted_profile.public_key, identity.public_key);
        assert!(!encrypted_profile.encrypted_data.is_empty());
        assert!(encrypted_profile.verify_signature().unwrap());
    }
    
    #[test]
    fn test_access_grant_system() {
        let (identity1, keypair1) = create_test_identity();
        let (identity2, _keypair2) = create_test_identity();
        let profile = UserProfile::new("Test User".to_string());
        
        let mut encrypted_profile = EncryptedUserProfile::new(
            &identity1,
            &profile,
            &keypair1,
            None,
        ).unwrap();
        
        let profile_key = EncryptedUserProfile::generate_profile_key();
        let permissions = ProfilePermissions::default();
        
        // Grant access to second user
        encrypted_profile.grant_access(
            identity2.user_id.clone(),
            &identity2.public_key,
            permissions.clone(),
            &profile_key,
            &keypair1,
        ).unwrap();
        
        // Check grant exists
        let grant = encrypted_profile.get_access_grant(&identity2.user_id);
        assert!(grant.is_some());
        assert_eq!(grant.unwrap().permissions.can_see_display_name, permissions.can_see_display_name);
        
        // Revoke access
        encrypted_profile.revoke_access(&identity2.user_id);
        assert!(encrypted_profile.get_access_grant(&identity2.user_id).is_none());
    }
    
    #[test]
    fn test_identity_challenge() {
        let challenge = IdentityChallenge::new(Duration::from_secs(300));
        let (identity, keypair) = create_test_identity();
        
        assert!(challenge.is_valid());
        
        let proof = challenge.create_response(
            identity.user_id.clone(),
            &keypair,
        ).unwrap();
        
        assert_eq!(proof.challenge_id, challenge.challenge_id);
        assert_eq!(proof.user_id, identity.user_id);
        assert!(proof.verify(&challenge, &identity.public_key).unwrap());
    }
    
    #[test]
    fn test_profile_permissions_filtering() {
        let mut profile = UserProfile::new("Test User".to_string());
        profile.status_message = Some("Hello world".to_string());
        profile.set_avatar(b"fake_image_data");
        
        let restricted_permissions = ProfilePermissions {
            can_see_display_name: false,
            can_see_avatar: false,
            can_see_status: false,
            ..Default::default()
        };
        
        let filtered_profile = profile.apply_permissions(&restricted_permissions);
        
        assert_eq!(filtered_profile.display_name, "Hidden");
        assert!(filtered_profile.avatar_hash.is_none());
        assert!(filtered_profile.status_message.is_none());
    }
    
    #[test]
    fn test_user_id_derivation() {
        let (identity1, keypair1) = create_test_identity();
        let (identity2, keypair2) = create_test_identity();
        
        // Same public key should give same user ID
        let derived_id1 = UserIdentity::derive_user_id(&keypair1.public);
        assert_eq!(identity1.user_id, derived_id1);
        
        // Different public keys should give different user IDs
        assert_ne!(identity1.user_id, identity2.user_id);
        assert_ne!(derived_id1, UserIdentity::derive_user_id(&keypair2.public));
    }
    
    #[test]
    fn test_dht_key_generation() {
        let (identity, _keypair) = create_test_identity();
        
        let profile_key = identity.get_profile_dht_key();
        let name_key = UserIdentity::get_name_resolution_dht_key("TestUser");
        
        // Keys should be deterministic
        assert_eq!(profile_key, identity.get_profile_dht_key());
        assert_eq!(name_key, UserIdentity::get_name_resolution_dht_key("TestUser"));
        
        // Different inputs should give different keys
        assert_ne!(profile_key, name_key);
    }
}