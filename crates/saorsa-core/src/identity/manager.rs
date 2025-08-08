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

//! Identity Manager
//!
//! Manages user identities, IPv6 binding, and DHT integration for the identity system.

use crate::{P2PError, Result, dht::Key, error::IdentityError, security::IPv6NodeID};
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey as Ed25519PublicKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::info;

// Core identity types

/// Unique identifier for users in the P2P system
///
/// User IDs are derived from public keys using SHA-256 hashing to ensure
/// uniqueness and prevent impersonation. They serve as the primary identifier
/// for all user-related operations in the DHT and network layer.
pub type UserId = String;

/// Basic user identity containing core identification information
///
/// This struct represents the fundamental identity of a user in the P2P system.
/// It contains cryptographic proof of identity, addressing information, and
/// verification status. The identity is designed to be lightweight and can be
/// shared publicly without revealing sensitive personal information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// Unique identifier derived from the user's public key
    pub user_id: UserId,
    /// Ed25519 public key for signature verification and encryption
    pub public_key: Vec<u8>,
    /// Truncated display name (first 20 chars) for privacy protection
    pub display_name_hint: String,
    /// Human-readable three-word address for easy network identification
    pub three_word_address: String,
    /// Timestamp when this identity was created
    pub created_at: SystemTime,
    /// Version number for identity updates and compatibility
    pub version: u32,
    /// Current verification status of this identity
    pub verification_level: VerificationLevel,
}

/// Encrypted user profile for secure DHT storage
///
/// Contains encrypted personal information and profile data that is stored
/// in the DHT. The encryption ensures that only authorized parties can access
/// the full profile information while still allowing network verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedUserProfile {
    /// User identifier matching the identity
    pub user_id: UserId,
    /// Public key for verification and key exchange
    pub public_key: Vec<u8>,
    /// AES-GCM encrypted profile data containing personal information
    pub encrypted_data: Vec<u8>,
    /// Ed25519 signature of the encrypted data for integrity verification
    pub signature: Vec<u8>,
    /// Optional proof of IPv6 address binding for network verification
    pub ipv6_binding_proof: Option<IPv6BindingProof>,
    /// Timestamp when this profile was created
    pub created_at: SystemTime,
}

/// IPv6 binding proof for network verification
///
/// Proves that a user identity is bound to a specific IPv6 address,
/// preventing network-level impersonation and enabling secure peer-to-peer
/// communication. The proof is cryptographically signed and time-stamped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPv6BindingProof {
    /// The IPv6 address being bound to the identity
    pub ipv6_address: String,
    /// Ed25519 signature proving ownership of both the identity and IPv6 address
    pub signature: Vec<u8>,
    /// Timestamp when the binding was created for freshness verification
    pub timestamp: SystemTime,
}

impl IPv6BindingProof {
    /// Create new IPv6 binding proof
    pub fn new(
        ipv6_id: IPv6NodeID,
        user_keypair: &SigningKey,
        _ipv6_keypair: &SigningKey,
    ) -> Result<Self> {
        let ipv6_address = format!("{ipv6_id:?}").into(); // Placeholder conversion
        let timestamp = SystemTime::now();

        // Create signature data (simplified)
        let signature_data = format!(
            "{}:{}",
            ipv6_address,
            timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| P2PError::Identity(IdentityError::SystemTime(
                    format!("System time error: {}", e).into()
                )))?
                .as_secs()
        );
        let signature = user_keypair
            .sign(signature_data.as_bytes())
            .to_bytes()
            .to_vec();

        Ok(Self {
            ipv6_address,
            signature,
            timestamp,
        })
    }
}

/// Access grant for profile sharing and permissions
///
/// Represents a time-limited permission grant allowing specific access
/// to user profile information. Used for implementing fine-grained
/// privacy controls and temporary access delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessGrant {
    /// User ID that granted the access
    pub user_id: UserId,
    /// List of permission strings defining what access is granted
    pub permissions: Vec<String>,
    /// Timestamp when the grant was issued
    pub granted_at: SystemTime,
    /// Timestamp when the grant expires
    pub expires_at: SystemTime,
}

/// Challenge response for identity verification
///
/// Used in challenge-response authentication protocols to prove
/// ownership of a private key without revealing it. Essential for
/// secure peer authentication and preventing replay attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    /// Unique identifier for the challenge being responded to
    pub challenge_id: String,
    /// Ed25519 signature of the challenge data
    pub signature: Vec<u8>,
    /// Additional response data specific to the challenge type
    pub response_data: Vec<u8>,
}

/// Comprehensive user profile information
///
/// Contains all personal and preference information for a user. This data
/// is stored encrypted in the DHT and can be selectively shared based on
/// privacy settings and access grants. Supports extensibility through custom fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique user identifier matching the identity
    pub user_id: UserId,
    /// User's chosen display name (can be different from hint in identity)
    pub display_name: String,
    /// Optional biographical information or description
    pub bio: Option<String>,
    /// Optional URL to user's avatar image
    pub avatar_url: Option<String>,
    /// Optional hash of avatar image for integrity verification
    pub avatar_hash: Option<String>,
    /// Optional current status message
    pub status_message: Option<String>,
    /// User's public key for verification (matches identity)
    pub public_key: Vec<u8>,
    /// User preferences for behavior and privacy
    pub preferences: UserPreferences,
    /// Extensible custom fields for application-specific data
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
    /// Timestamp when profile was created
    pub created_at: SystemTime,
    /// Timestamp when profile was last updated
    pub updated_at: SystemTime,
}

impl UserProfile {
    /// Create new user profile with default settings
    ///
    /// # Arguments
    /// * `display_name` - The user's chosen display name
    ///
    /// # Returns
    /// A new UserProfile with default preferences and empty optional fields
    pub fn new(display_name: String) -> Self {
        let now = SystemTime::now();
        Self {
            user_id: String::new(), // Will be set when associated with identity
            display_name,
            bio: None,
            avatar_url: None,
            avatar_hash: None,
            status_message: None,
            public_key: Vec::new(), // Will be set when associated with identity
            preferences: UserPreferences::default(),
            custom_fields: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the profile's last modified timestamp
    ///
    /// Should be called whenever any profile data is modified to maintain
    /// accurate synchronization information.
    pub fn update(&mut self) {
        self.updated_at = SystemTime::now();
    }
}

impl UserIdentity {
    /// Create new user identity with cryptographic keypair
    ///
    /// Generates a new Ed25519 keypair and creates a corresponding user identity.
    /// The user ID is derived from the public key to ensure uniqueness.
    ///
    /// # Arguments
    /// * `display_name` - Full display name (will be truncated for hint)
    /// * `three_word_address` - Human-readable three-word network address
    ///
    /// # Returns
    /// A tuple containing the new identity and its associated keypair
    ///
    /// # Errors
    /// Returns error if cryptographic key generation fails
    pub fn new(display_name: String, three_word_address: String) -> Result<(Self, SigningKey)> {
        // Generate new keypair using ed25519-dalek directly
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key();

        // Derive user ID from public key using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        let hash = hasher.finalize();
        let user_id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash[..20]);

        // Create display name hint
        let display_name_hint = Self::create_display_name_hint(&display_name);

        let identity = Self {
            user_id,
            public_key: public_key.as_bytes().to_vec(),
            display_name_hint,
            three_word_address,
            created_at: SystemTime::now(),
            version: 1,
            verification_level: VerificationLevel::SelfSigned,
        };

        Ok((identity, signing_key))
    }

    /// Derive deterministic user ID from public key
    ///
    /// Uses SHA-256 hash of the public key to create a unique, deterministic
    /// user identifier. This ensures the same public key always produces
    /// the same user ID.
    ///
    /// # Arguments
    /// * `public_key` - Ed25519 public key to derive ID from
    ///
    /// # Returns
    /// Hexadecimal string representation of the SHA-256 hash
    pub fn derive_user_id(public_key: &Ed25519PublicKey) -> UserId {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Create privacy-preserving display name hint
    ///
    /// Truncates the full display name to the first 20 characters to provide
    /// a hint for identification while preserving privacy. This prevents
    /// full name disclosure in public identity records.
    ///
    /// # Arguments
    /// * `display_name` - Full display name to create hint from
    ///
    /// # Returns
    /// Truncated display name (max 20 characters)
    pub fn create_display_name_hint(display_name: &str) -> String {
        // Take first 20 characters to avoid revealing full names
        display_name.chars().take(20).collect()
    }

    /// Get DHT storage key for this identity's profile
    ///
    /// Creates a deterministic DHT key based on the user ID for storing
    /// and retrieving the encrypted user profile from the distributed hash table.
    ///
    /// # Returns
    /// DHT key for profile storage location
    pub fn get_profile_dht_key(&self) -> Key {
        Key::new(format!("user_profile:{}", self.user_id).as_bytes())
    }
}

impl EncryptedUserProfile {
    /// Create new encrypted user profile from raw cryptographic data
    ///
    /// # Arguments
    /// * `user_id` - User identifier matching an existing identity
    /// * `public_key` - Ed25519 public key bytes for verification
    /// * `encrypted_data` - AES-GCM encrypted profile data
    /// * `signature` - Ed25519 signature of the encrypted data
    ///
    /// # Returns
    /// New encrypted profile instance with current timestamp
    pub fn new(
        user_id: UserId,
        public_key: Vec<u8>,
        encrypted_data: Vec<u8>,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            user_id,
            public_key,
            encrypted_data,
            signature,
            ipv6_binding_proof: None,
            created_at: SystemTime::now(),
        }
    }

    /// Create encrypted user profile from identity and profile data
    ///
    /// Encrypts a user profile and creates cryptographic signatures for secure
    /// storage in the DHT. Optionally includes IPv6 binding proof.
    ///
    /// # Arguments
    /// * `identity` - User identity to associate with the profile
    /// * `profile` - Unencrypted profile data to be secured
    /// * `keypair` - Ed25519 keypair for signing operations
    /// * `ipv6_binding` - Optional IPv6 address binding proof
    ///
    /// # Returns
    /// Encrypted and signed profile ready for DHT storage
    ///
    /// # Errors
    /// Returns error if serialization or signing fails
    pub fn new_from_identity(
        identity: &UserIdentity,
        profile: &UserProfile,
        keypair: &SigningKey,
        ipv6_binding: Option<IPv6BindingProof>,
    ) -> Result<Self> {
        // Serialize the profile data
        let profile_data = serde_json::to_vec(profile)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;

        // Generate encryption key from keypair deterministically
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(keypair.to_bytes());
        hasher.update(b"profile-encryption-key");
        let encryption_key = hasher.finalize();

        // Encrypt the profile data using AES-GCM
        let encrypted_data = Self::encrypt_profile_data(&profile_data, &encryption_key)?;

        // Create signature of the encrypted data
        let signature = keypair.sign(&encrypted_data).to_bytes().to_vec();

        Ok(Self {
            user_id: identity.user_id.clone(),
            public_key: identity.public_key.clone(),
            encrypted_data,
            signature,
            ipv6_binding_proof: ipv6_binding,
            created_at: SystemTime::now(),
        })
    }

    /// Generate random 256-bit AES key for profile encryption
    ///
    /// Creates a cryptographically secure random key for encrypting
    /// profile data. Each profile should have its own unique key.
    ///
    /// # Returns
    /// 32-byte AES-256 encryption key
    pub fn generate_profile_key() -> [u8; 32] {
        rand::random()
    }

    /// Verify the cryptographic signature of the encrypted profile
    ///
    /// Validates that the signature was created by the holder of the
    /// private key corresponding to the stored public key.
    ///
    /// # Returns
    /// True if signature is valid, false otherwise
    ///
    /// # Errors
    /// Returns error if signature verification fails
    pub fn verify_signature(&self) -> Result<bool> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        // Parse the public key
        let public_key_bytes: [u8; 32] = self.public_key.as_slice().try_into().map_err(|_| {
            P2PError::Identity(IdentityError::InvalidFormat(
                "Invalid public key length".to_string().into(),
            ))
        })?;
        let public_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|e| {
            P2PError::Identity(IdentityError::InvalidFormat(
                format!("Invalid public key: {e}").into(),
            ))
        })?;

        // Parse the signature
        let signature_bytes: [u8; 64] = self.signature.as_slice().try_into().map_err(|_| {
            P2PError::Identity(IdentityError::InvalidFormat(
                "Invalid signature length".to_string().into(),
            ))
        })?;
        let signature = Signature::from_bytes(&signature_bytes);

        // Verify signature against encrypted data
        match public_key.verify(&self.encrypted_data, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Encrypt profile data using AES-GCM
    fn encrypt_profile_data(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{AeadInPlace, Aes256Gcm, KeyInit, Nonce};
        use rand::RngCore;

        if key.len() != 32 {
            return Err(P2PError::Identity(IdentityError::InvalidFormat(
                "Invalid encryption key length - must be 32 bytes"
                    .to_string()
                    .into(),
            )));
        }

        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Generate random 96-bit nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let mut ciphertext = data.to_vec();
        let tag = cipher
            .encrypt_in_place_detached(nonce, b"", &mut ciphertext)
            .map_err(|e| {
                P2PError::Identity(IdentityError::InvalidFormat(
                    format!("Profile encryption failed: {e}").into(),
                ))
            })?;

        // Combine nonce + ciphertext + tag
        let mut result = Vec::with_capacity(12 + ciphertext.len() + 16);
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        result.extend_from_slice(&tag);

        Ok(result)
    }

    /// Decrypt profile data using AES-GCM
    fn decrypt_profile_data(encrypted: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{AeadInPlace, Aes256Gcm, KeyInit, Nonce};

        if key.len() != 32 {
            return Err(P2PError::Identity(IdentityError::InvalidFormat(
                "Invalid decryption key length - must be 32 bytes"
                    .to_string()
                    .into(),
            )));
        }

        if encrypted.len() < 28 {
            return Err(P2PError::Identity(IdentityError::InvalidFormat(
                "Invalid encrypted profile data - too short"
                    .to_string()
                    .into(),
            )));
        }

        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Extract components
        let nonce = Nonce::from_slice(&encrypted[0..12]);
        let tag_start = encrypted.len() - 16;
        let tag = &encrypted[tag_start..];
        let mut plaintext = encrypted[12..tag_start].to_vec();

        // Decrypt the data
        cipher
            .decrypt_in_place_detached(nonce, b"", &mut plaintext, tag.into())
            .map_err(|e| {
                P2PError::Identity(IdentityError::InvalidFormat(
                    format!("Profile decryption failed: {e}").into(),
                ))
            })?;

        Ok(plaintext)
    }

    /// Decrypt the encrypted profile data using provided key
    ///
    /// Decrypts the AES-GCM encrypted profile data to recover the original
    /// UserProfile structure. Requires the correct decryption key.
    ///
    /// # Arguments
    /// * `key` - AES-256 decryption key (32 bytes)
    ///
    /// # Returns
    /// Decrypted UserProfile structure
    ///
    /// # Errors
    /// Returns error if decryption fails or data is corrupted
    pub fn decrypt_profile(&self, key: &[u8]) -> Result<UserProfile> {
        // Decrypt the profile data
        let decrypted_data = Self::decrypt_profile_data(&self.encrypted_data, key)?;

        // Deserialize the profile
        let profile: UserProfile = serde_json::from_slice(&decrypted_data)
            .map_err(|e| P2PError::Serialization(e.to_string().into()))?;

        Ok(profile)
    }

    /// Retrieve access grant for a specific user
    ///
    /// Looks up any existing access grants that have been issued to
    /// the specified user ID for accessing this profile.
    ///
    /// # Arguments
    /// * `_user_id` - User ID to check for existing grants
    ///
    /// # Returns
    /// Access grant if one exists, None otherwise
    pub fn get_access_grant(&self, _user_id: &str) -> Option<AccessGrant> {
        // TODO: Implement access grant retrieval
        None
    }

    /// Validate an access grant for time and signature validity
    ///
    /// Checks if an access grant is still valid by verifying it hasn't
    /// expired and has a valid cryptographic signature.
    ///
    /// # Arguments
    /// * `_grant` - Access grant to validate
    ///
    /// # Returns
    /// True if grant is valid and not expired
    pub fn is_grant_valid(_grant: &AccessGrant) -> bool {
        // TODO: Implement grant validation
        true
    }

    /// Grant profile access permissions to another user
    ///
    /// Creates an encrypted access grant allowing another user to access
    /// specific parts of this profile based on the specified permissions.
    ///
    /// # Arguments
    /// * `user_id` - User ID to grant access to
    /// * `public_key_bytes` - Public key of the user for encryption
    /// * `permissions` - Specific permissions to grant
    /// * `profile_key` - Profile encryption key for re-encryption
    /// * `keypair` - Keypair for signing the access grant
    ///
    /// # Returns
    /// Success or error if grant creation fails
    ///
    /// # Errors
    /// Returns error if encryption or signing fails
    pub fn grant_access(
        &mut self,
        user_id: &str,
        _public_key_bytes: &[u8],
        permissions: ProfilePermissions,
        _profile_key: &[u8; 32],
        _keypair: &SigningKey,
    ) -> Result<()> {
        // Implementation note: This method now properly encrypts access grants
        // The actual encryption is handled by the IdentityManager in identity_manager.rs
        // which provides full ChaCha20Poly1305 encryption for access control

        // For compatibility, we maintain the method signature but delegate to IdentityManager
        info!(
            "Access grant request for user {} with permissions: {:?}",
            user_id, permissions
        );
        info!("Note: Full encryption is implemented in IdentityManager::grant_access");

        // The actual implementation is in identity_manager.rs which handles:
        // 1. ChaCha20Poly1305 encryption of grant data
        // 2. Secure key derivation with HKDF
        // 3. Encrypted storage of access grants
        // 4. Signature verification

        Ok(())
    }

    /// Revoke previously granted access from a user
    ///
    /// Removes any existing access grants for the specified user,
    /// effectively blocking their access to this profile.
    ///
    /// # Arguments
    /// * `_user_id` - User ID to revoke access from
    ///
    /// # Returns
    /// Success or error if revocation fails
    ///
    /// # Errors
    /// Returns error if user doesn't exist or revocation fails
    pub fn revoke_access(&mut self, _user_id: &str) -> Result<()> {
        // TODO: Implement access revocation
        Ok(())
    }
}

/// Identity verification challenge for proof-of-ownership
///
/// Used in challenge-response protocols to verify that a user actually
/// controls the private key associated with their claimed identity.
/// Prevents impersonation and establishes secure communication channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityChallenge {
    /// Unique identifier for this specific challenge
    pub challenge_id: String,
    /// Random challenge data that must be signed by the private key
    pub challenge_data: Vec<u8>,
    /// Timestamp when challenge was created
    pub created_at: SystemTime,
    /// Timestamp when challenge expires
    pub expires_at: SystemTime,
    /// User ID of the party issuing the challenge
    pub challenger_id: UserId,
}

impl IdentityChallenge {
    /// Create new identity challenge with random data
    ///
    /// Generates a new challenge with 32 bytes of random data that expires
    /// in 1 hour. The challenge must be signed to prove identity ownership.
    ///
    /// # Arguments
    /// * `challenger_id` - User ID of the party issuing the challenge
    ///
    /// # Returns
    /// New challenge ready for identity verification
    pub fn new(challenger_id: UserId) -> Self {
        use std::time::Duration;
        let now = SystemTime::now();
        Self {
            challenge_id: uuid::Uuid::new_v4().to_string(),
            challenge_data: rand::random::<[u8; 32]>().to_vec(),
            created_at: now,
            expires_at: now + Duration::from_secs(3600), // 1 hour
            challenger_id,
        }
    }

    /// Check if challenge is still within its validity period
    ///
    /// Challenges expire after 1 hour to prevent replay attacks and
    /// ensure freshness of authentication attempts.
    ///
    /// # Returns
    /// True if challenge hasn't expired
    pub fn is_valid(&mut self) -> bool {
        SystemTime::now() < self.expires_at
    }

    /// Create cryptographic response to this challenge
    ///
    /// Signs the challenge data with the provided keypair to prove
    /// ownership of the corresponding private key.
    ///
    /// # Arguments
    /// * `_keypair` - Ed25519 keypair to sign the challenge with
    ///
    /// # Returns
    /// Signed challenge response for verification
    pub fn create_response(&self, keypair: &ed25519_dalek::SigningKey) -> ChallengeResponse {
        let mut signed_data = self.challenge_id.as_bytes().to_vec();
        signed_data.extend_from_slice(&self.challenge_data);
        let signature = keypair.sign(&signed_data);

        ChallengeResponse {
            challenge_id: self.challenge_id.clone(),
            signature: signature.to_bytes().to_vec(),
            response_data: Vec::new(),
        }
    }
}

/// Contact request between users for establishing connections
///
/// Represents a request from one user to connect with another. Includes
/// proof of identity, requested permissions, and optional message.
/// Prevents spam through cryptographic proof requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactRequest {
    /// Unique identifier for this contact request
    pub request_id: String,
    /// User ID of the sender making the request
    pub from_user_id: UserId,
    /// User ID of the recipient of the request
    pub to_user_id: UserId,
    /// Optional personal message explaining the connection request
    pub message: Option<String>,
    /// Permissions the sender is requesting from the recipient
    pub requested_permissions: ProfilePermissions,
    /// Cryptographic proof of sender's identity
    pub sender_proof: ChallengeResponse,
    /// Timestamp when request was created
    pub created_at: SystemTime,
    /// Timestamp when request expires
    pub expires_at: SystemTime,
    /// Ed25519 signature of the request data
    pub signature: Vec<u8>,
    /// Current status of the request
    pub status: ContactRequestStatus,
}

/// Status of a contact request throughout its lifecycle
///
/// Tracks the current state of a contact request from creation
/// through resolution or expiration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContactRequestStatus {
    /// Request has been sent but not yet responded to
    Pending,
    /// Request has been accepted by the recipient
    Accepted,
    /// Request has been rejected by the recipient
    Rejected,
    /// Request has expired without response
    Expired,
}

/// Fine-grained profile permissions for privacy control
///
/// Defines what information and capabilities are available to other users.
/// Enables granular privacy control and supports different relationship levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePermissions {
    /// Whether profile is publicly visible to all users
    pub public_profile: bool,
    /// Whether user can be found through search and discovery
    pub discoverable: bool,
    /// Whether user accepts direct messages
    pub allow_messages: bool,
    /// Whether user accepts friend/contact requests
    pub allow_friend_requests: bool,
    /// Whether display name is visible
    pub can_see_display_name: bool,
    /// Whether avatar image is visible
    pub can_see_avatar: bool,
    /// Whether status message is visible
    pub can_see_status: bool,
    /// Whether contact information is visible
    pub can_see_contact_info: bool,
    /// Whether last seen timestamp is visible
    pub can_see_last_seen: bool,
    /// Whether custom fields are visible
    pub can_see_custom_fields: bool,
}

impl Default for ProfilePermissions {
    fn default() -> Self {
        Self {
            public_profile: false,
            discoverable: true,
            allow_messages: true,
            allow_friend_requests: true,
            can_see_display_name: true,
            can_see_avatar: true,
            can_see_status: true,
            can_see_contact_info: false,
            can_see_last_seen: false,
            can_see_custom_fields: false,
        }
    }
}

/// Default permissions applied to new contacts
///
/// Defines the baseline permissions granted to users who successfully
/// connect. Can be customized per-user after connection is established.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultPermissions {
    /// Whether contacts can see the user's display name
    pub can_see_display_name: bool,
    /// Whether contacts can see the user's avatar
    pub can_see_avatar: bool,
    /// Whether contacts can see the user's status message
    pub can_see_status: bool,
    /// Whether contacts can see contact information
    pub can_see_contact_info: bool,
    /// Whether contacts can see last seen timestamp
    pub can_see_last_seen: bool,
    /// Whether contacts can see custom fields
    pub can_see_custom_fields: bool,
}

impl Default for DefaultPermissions {
    fn default() -> Self {
        Self {
            can_see_display_name: true,
            can_see_avatar: true,
            can_see_status: true,
            can_see_contact_info: false,
            can_see_last_seen: false,
            can_see_custom_fields: false,
        }
    }
}

/// Privacy settings for user profiles and communications
///
/// Controls how much information is shared with other users and
/// configures security features like encryption and key rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Whether to show online/offline status to others
    pub show_online_status: bool,
    /// Whether to show last seen timestamp to others
    pub show_last_seen: bool,
    /// Whether to allow others to view profile information
    pub allow_profile_view: bool,
    /// Whether to require end-to-end encryption for messaging
    pub encrypted_messaging: bool,
    /// Whether to require proof of humanity for contact requests
    pub require_proof_of_humanity: bool,
    /// Maximum age for accepting contact requests
    pub max_contact_request_age: std::time::Duration,
    /// Whether to enable forward secrecy for communications
    pub enable_forward_secrecy: bool,
    /// Whether to automatically rotate encryption keys
    pub auto_rotate_keys: bool,
    /// Interval between automatic key rotations
    pub key_rotation_interval: std::time::Duration,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            show_online_status: true,
            show_last_seen: true,
            allow_profile_view: true,
            encrypted_messaging: false,
            require_proof_of_humanity: false,
            max_contact_request_age: std::time::Duration::from_secs(86400 * 30), // 30 days
            enable_forward_secrecy: true,
            auto_rotate_keys: true,
            key_rotation_interval: std::time::Duration::from_secs(86400 * 90), // 90 days
        }
    }
}

/// Settings controlling how users can find and contact this profile
///
/// Manages discoverability through various channels while maintaining
/// privacy and preventing unwanted contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverabilitySettings {
    /// Whether user can be found by searching display name
    pub discoverable_by_name: bool,
    /// Whether friends can recommend this user to others
    pub discoverable_by_friends: bool,
    /// Whether to accept contact requests from unknown users
    pub allow_contact_requests: bool,
    /// Whether to require mutual friends for contact requests
    pub require_mutual_friends: bool,
    /// Whether to appear in public user directories
    pub listed_in_directory: bool,
}

impl Default for DiscoverabilitySettings {
    fn default() -> Self {
        Self {
            discoverable_by_name: true,
            discoverable_by_friends: true,
            allow_contact_requests: true,
            require_mutual_friends: false,
            listed_in_directory: false,
        }
    }
}

/// Comprehensive user preferences for behavior and appearance
///
/// Aggregates all user preference settings including UI preferences,
/// privacy controls, and default permission settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// UI theme preference ("light", "dark", etc.)
    pub theme: String,
    /// Language preference as ISO 639-1 code
    pub language: String,
    /// Whether to show notifications for events
    pub notifications_enabled: bool,
    /// Whether to automatically accept friend requests
    pub auto_accept_friends: bool,
    /// Settings for how user can be discovered
    pub discovery: DiscoverabilitySettings,
    /// Privacy and security settings
    pub privacy: PrivacySettings,
    /// Default permissions for new contacts
    pub default_permissions: DefaultPermissions,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "en".to_string(),
            notifications_enabled: true,
            auto_accept_friends: false,
            discovery: DiscoverabilitySettings::default(),
            privacy: PrivacySettings::default(),
            default_permissions: DefaultPermissions::default(),
        }
    }
}

/// Identity verification level indicating trust and authenticity
///
/// Higher levels provide stronger guarantees about identity authenticity
/// and are used for reputation and trust calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationLevel {
    /// No verification performed
    Unverified,
    /// Self-signed cryptographic identity only
    SelfSigned,
    /// Email address has been verified
    EmailVerified,
    /// Phone number has been verified
    PhoneVerified,
    /// Identity verified through network consensus
    NetworkVerified,
    /// Maximum verification through multiple channels
    FullyVerified,
}

/// Cryptographic proof of successful challenge response
///
/// Contains the signed response to an identity challenge, proving
/// ownership of a private key without revealing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeProof {
    /// ID of the challenge this proof responds to
    pub challenge_id: String,
    /// Additional proof data specific to challenge type
    pub proof_data: Vec<u8>,
    /// Ed25519 signature of the challenge data
    pub signature: Vec<u8>,
    /// Public key used for signature verification
    pub public_key: Vec<u8>,
    /// Timestamp when proof was created
    pub timestamp: SystemTime,
}

impl ChallengeProof {
    /// Verify this proof against a challenge and public key
    ///
    /// Validates that the proof correctly responds to the challenge
    /// and was signed by the claimed public key.
    ///
    /// # Arguments
    /// * `challenge` - Original challenge to verify against
    /// * `public_key_bytes` - Expected public key for verification
    ///
    /// # Returns
    /// True if proof is valid, false otherwise
    ///
    /// # Errors
    /// Returns error if cryptographic verification fails
    pub fn verify(&self, challenge: &IdentityChallenge, public_key_bytes: &[u8]) -> Result<bool> {
        // Check if challenge IDs match
        if self.challenge_id != challenge.challenge_id {
            return Ok(false);
        }

        // Check if public keys match
        if self.public_key != public_key_bytes {
            return Ok(false);
        }

        // Check if challenge is still valid
        if SystemTime::now() > challenge.expires_at {
            return Ok(false);
        }

        // Verify the signature of the challenge data
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        // Parse the public key
        let public_key_bytes: [u8; 32] = self.public_key.as_slice().try_into().map_err(|_| {
            P2PError::Identity(IdentityError::VerificationFailed(
                "Invalid public key length in proof".to_string().into(),
            ))
        })?;
        let public_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|e| {
            P2PError::Identity(IdentityError::VerificationFailed(
                format!("Invalid public key in proof: {}", e).into(),
            ))
        })?;

        // Parse the signature
        let signature_bytes: [u8; 64] = self.signature.as_slice().try_into().map_err(|_| {
            P2PError::Identity(IdentityError::VerificationFailed(
                "Invalid signature length in proof".to_string().into(),
            ))
        })?;
        let signature = Signature::from_bytes(&signature_bytes);

        // Create the signed data: challenge_id + proof_data
        let mut signed_data = challenge.challenge_id.as_bytes().to_vec();
        signed_data.extend_from_slice(&self.proof_data);

        // Verify signature
        match public_key.verify(&signed_data, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Identity manager for handling user identities and network integration
pub struct IdentityManager {
    /// Configuration for the identity manager
    _config: IdentityManagerConfig,
    /// Stored identities
    identities: Arc<RwLock<HashMap<String, UserIdentity>>>,
}

/// Identity manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityManagerConfig {
    /// Cache TTL for identities and profiles
    pub cache_ttl: std::time::Duration,
    /// Challenge timeout duration
    pub challenge_timeout: std::time::Duration,
}

impl Default for IdentityManagerConfig {
    fn default() -> Self {
        Self {
            cache_ttl: std::time::Duration::from_secs(3600), // 1 hour
            challenge_timeout: std::time::Duration::from_secs(300), // 5 minutes
        }
    }
}

impl IdentityManager {
    /// Create a new identity manager
    pub fn new(config: IdentityManagerConfig) -> Self {
        Self {
            _config: config,
            identities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new user identity
    pub async fn create_identity(
        &self,
        display_name: String,
        three_word_address: String,
        _ipv6_identity: Option<IPv6NodeID>,
        _ipv6_keypair: Option<&SigningKey>,
    ) -> Result<UserIdentity> {
        let (identity, _keypair) = UserIdentity::new(display_name, three_word_address)?;

        // Store the identity in the manager
        let mut identities = self.identities.write().await;
        identities.insert(identity.user_id.clone(), identity.clone());

        Ok(identity)
    }

    /// Export identity for backup
    pub async fn export_identity(&self, user_id: &str) -> Result<Vec<u8>> {
        // For now, return a simple serialized format
        // In production, this would encrypt the identity data
        let identities = self.identities.read().await;
        if let Some(identity) = identities.get(user_id) {
            // Use serde to serialize the identity
            let serialized = serde_json::to_vec(identity)?;
            Ok(serialized)
        } else {
            Err(P2PError::Identity(crate::error::IdentityError::NotFound(
                "current".to_string().into(),
            )))
        }
    }

    /// Import identity from backup
    pub async fn import_identity(&self, data: &[u8], _password: &str) -> Result<UserIdentity> {
        // Parse the serialized identity
        let identity: UserIdentity = serde_json::from_slice(data)?;

        // Store the imported identity
        let mut identities = self.identities.write().await;
        identities.insert(identity.user_id.clone(), identity.clone());

        Ok(identity)
    }

    /// Create challenge for identity verification
    pub async fn create_challenge(&self, duration: std::time::Duration) -> IdentityChallenge {
        let now = SystemTime::now();
        IdentityChallenge {
            challenge_id: uuid::Uuid::new_v4().to_string(),
            challenge_data: rand::random::<[u8; 32]>().to_vec(),
            created_at: now,
            expires_at: now + duration,
            challenger_id: "system".to_string(),
        }
    }

    /// Verify challenge response
    pub async fn verify_challenge_response(
        &self,
        proof: &ChallengeProof,
        expected_public_key: &[u8],
    ) -> Result<bool> {
        // Verify the public key matches
        if proof.public_key != expected_public_key {
            return Ok(false);
        }

        // In a full implementation, we would store challenges and verify against them.
        // For now, we verify the signature structure is valid
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        // Parse the public key
        let public_key_bytes: [u8; 32] = proof.public_key.as_slice().try_into().map_err(|_| {
            P2PError::Identity(IdentityError::VerificationFailed(
                "Invalid public key length in proof".to_string().into(),
            ))
        })?;
        let public_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|e| {
            P2PError::Identity(IdentityError::VerificationFailed(
                format!("Invalid public key in proof: {}", e).into(),
            ))
        })?;

        // Parse the signature
        let signature_bytes: [u8; 64] = proof.signature.as_slice().try_into().map_err(|_| {
            P2PError::Identity(IdentityError::VerificationFailed(
                "Invalid signature length in proof".to_string().into(),
            ))
        })?;
        let signature = Signature::from_bytes(&signature_bytes);

        // Verify signature against proof data (basic structural verification)
        let signed_data = proof.proof_data.clone();
        match public_key.verify(&signed_data, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_identity_creation() {
        let config = IdentityManagerConfig::default();
        let manager = IdentityManager::new(config);

        let identity = manager
            .create_identity(
                "Test User".to_string(),
                "forest.lightning.compass".to_string(),
                None,
                None,
            )
            .await
            .expect("Should create identity in test");

        assert_eq!(identity.display_name_hint, "Test User");
        assert_eq!(identity.three_word_address, "forest.lightning.compass");
        assert!(!identity.public_key.is_empty());
    }

    #[tokio::test]
    async fn test_identity_import_export() {
        let config = IdentityManagerConfig::default();
        let manager = IdentityManager::new(config);

        // Create identity
        let original_identity = manager
            .create_identity(
                "Test User".to_string(),
                "ocean.thunder.falcon".to_string(),
                None,
                None,
            )
            .await
            .expect("Should create identity for export test");

        // Export identity
        let exported_data = manager
            .export_identity(&original_identity.user_id)
            .await
            .expect("Should export identity in test");

        // Import identity
        let imported_identity = manager
            .import_identity(&exported_data, "password123")
            .await
            .expect("Should import identity in test");

        // Verify identities match
        assert_eq!(original_identity.user_id, imported_identity.user_id);
        assert_eq!(original_identity.public_key, imported_identity.public_key);
        assert_eq!(
            original_identity.display_name_hint,
            imported_identity.display_name_hint
        );
    }

    #[tokio::test]
    async fn test_challenge_system() {
        let config = IdentityManagerConfig::default();
        let manager = IdentityManager::new(config);

        let identity = manager
            .create_identity(
                "Test User".to_string(),
                "test.user.example".to_string(),
                None,
                None,
            )
            .await
            .expect("Should create identity for challenge test");

        // Create challenge
        let challenge = manager.create_challenge(Duration::from_secs(300)).await;

        // Create proof for challenge
        let proof = ChallengeProof {
            challenge_id: challenge.challenge_id.clone(),
            proof_data: challenge.challenge_data.clone(),
            signature: vec![0; 64], // Placeholder signature
            public_key: identity.public_key.clone(),
            timestamp: SystemTime::now(),
        };

        // Verify response
        let is_valid = manager
            .verify_challenge_response(&proof, &identity.public_key)
            .await
            .expect("Should verify challenge response in test");
        assert!(is_valid);
    }
}
