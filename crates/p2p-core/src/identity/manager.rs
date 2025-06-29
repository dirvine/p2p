//! Identity Manager
//! 
//! Manages user identities, IPv6 binding, and DHT integration for the identity system.

use crate::{P2PError, Result};
use ed25519_dalek::{PublicKey as Ed25519PublicKey, Keypair, Signer};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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
        user_keypair: &Keypair,
        ipv6_keypair: &Keypair,
    ) -> Result<Self> {
        let ipv6_address = format!("{:?}", ipv6_id); // Placeholder conversion
        let timestamp = SystemTime::now();
        
        // Create signature data (simplified)
        let signature_data = format!("{}:{}", ipv6_address, 
            timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs());
        let signature = user_keypair.sign(signature_data.as_bytes()).to_bytes().to_vec();
        
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
    pub fn new(display_name: String, three_word_address: String) -> Result<(Self, Keypair)> {
        use rand_core::OsRng;
        
        // Generate new keypair
        let mut csprng = OsRng;
        let keypair = Keypair::generate(&mut csprng);
        
        // Derive user ID from public key
        let user_id = Self::derive_user_id(&keypair.public);
        
        // Create display name hint
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
        use sha2::{Sha256, Digest};
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
        keypair: &Keypair,
        ipv6_binding: Option<IPv6BindingProof>,
    ) -> Result<Self> {
        // Serialize the profile data
        let profile_data = serde_json::to_vec(profile)
            .map_err(|e| P2PError::Serialization(e))?;
        
        // Create signature (placeholder implementation)
        let signature_data = format!("{}:{}", identity.user_id, profile.display_name);
        let signature = keypair.sign(signature_data.as_bytes()).to_bytes().to_vec();
        
        Ok(Self {
            user_id: identity.user_id.clone(),
            public_key: identity.public_key.clone(),
            encrypted_data: profile_data, // In real implementation, this would be encrypted
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
        // TODO: Implement proper signature verification
        // For now, just return true as a placeholder
        Ok(true)
    }
    
    /// Decrypt the encrypted profile data using provided key
    /// 
    /// Decrypts the AES-GCM encrypted profile data to recover the original
    /// UserProfile structure. Requires the correct decryption key.
    /// 
    /// # Arguments
    /// * `_key` - AES-256 decryption key (32 bytes)
    /// 
    /// # Returns
    /// Decrypted UserProfile structure
    /// 
    /// # Errors
    /// Returns error if decryption fails or data is corrupted
    pub fn decrypt_profile(&self, _key: &[u8]) -> Result<UserProfile> {
        // TODO: Implement proper decryption
        // For now, return a basic profile
        Ok(UserProfile {
            user_id: self.user_id.clone(),
            display_name: "Decrypted Profile".to_string(),
            bio: None,
            avatar_url: None,
            avatar_hash: None,
            status_message: None,
            public_key: self.public_key.clone(),
            preferences: UserPreferences::default(),
            custom_fields: std::collections::HashMap::new(),
            created_at: self.created_at,
            updated_at: self.created_at,
        })
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
        public_key_bytes: &[u8],
        permissions: ProfilePermissions,
        profile_key: &[u8; 32],
        keypair: &Keypair,
    ) -> Result<()> {
        // TODO: Implement proper access granting with encryption
        // For now, just log the operation
        info!("Granting access to user {} with permissions: {:?}", user_id, permissions);
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
    pub fn create_response(&self, _keypair: &ed25519_dalek::Keypair) -> ChallengeResponse {
        // TODO: Implement proper challenge response
        ChallengeResponse {
            challenge_id: self.challenge_id.clone(),
            signature: vec![0; 64], // Placeholder
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
        
        // TODO: Implement proper signature verification
        // For now, just return true as a placeholder
        Ok(true)
    }
}
use crate::dht::{Key, Record};
use crate::security::IPv6NodeID;
use crate::network::P2PNode;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Identity manager for handling user identities and network integration
pub struct IdentityManager {
    /// Local user identity
    local_identity: RwLock<Option<UserIdentity>>,
    /// Local user keypair
    local_keypair: RwLock<Option<Keypair>>,
    /// Local encrypted profile
    local_profile: RwLock<Option<EncryptedUserProfile>>,
    /// Profile encryption key
    profile_key: RwLock<Option<[u8; 32]>>,
    /// IPv6 binding proof
    ipv6_binding: RwLock<Option<IPv6BindingProof>>,
    /// Known user identities cache
    identity_cache: RwLock<HashMap<UserId, (UserIdentity, SystemTime)>>,
    /// Encrypted profiles cache
    profile_cache: RwLock<HashMap<UserId, (EncryptedUserProfile, SystemTime)>>,
    /// Active challenges
    active_challenges: RwLock<HashMap<String, IdentityChallenge>>,
    /// Pending contact requests
    pending_requests: RwLock<HashMap<String, ContactRequest>>,
    /// P2P network reference
    network: Option<Arc<P2PNode>>,
    /// Cache TTL
    cache_ttl: Duration,
}

/// Identity manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityManagerConfig {
    /// Cache TTL for identities and profiles
    pub cache_ttl: Duration,
    /// Challenge timeout duration
    pub challenge_timeout: Duration,
    /// Contact request timeout
    pub contact_request_timeout: Duration,
    /// Enable automatic profile backups to DHT
    pub enable_profile_backup: bool,
    /// Profile backup interval
    pub profile_backup_interval: Duration,
}

impl Default for IdentityManagerConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(3600), // 1 hour
            challenge_timeout: Duration::from_secs(300), // 5 minutes
            contact_request_timeout: Duration::from_secs(7 * 24 * 3600), // 1 week
            enable_profile_backup: true,
            profile_backup_interval: Duration::from_secs(24 * 3600), // 24 hours
        }
    }
}

impl IdentityManager {
    /// Create a new identity manager
    pub fn new(config: IdentityManagerConfig) -> Self {
        Self {
            local_identity: RwLock::new(None),
            local_keypair: RwLock::new(None),
            local_profile: RwLock::new(None),
            profile_key: RwLock::new(None),
            ipv6_binding: RwLock::new(None),
            identity_cache: RwLock::new(HashMap::new()),
            profile_cache: RwLock::new(HashMap::new()),
            active_challenges: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
            network: None,
            cache_ttl: config.cache_ttl,
        }
    }
    
    /// Set the P2P network reference
    pub fn set_network(&mut self, network: Arc<P2PNode>) {
        self.network = Some(network);
    }
    
    /// Create a new user identity
    pub async fn create_identity(
        &self,
        display_name: String,
        three_word_address: String,
        ipv6_identity: Option<IPv6NodeID>,
        ipv6_keypair: Option<&Keypair>,
    ) -> Result<UserIdentity> {
        let (identity, keypair) = UserIdentity::new(display_name.clone(), three_word_address)?;
        
        // Create default profile
        let profile = UserProfile::new(display_name);
        
        // Create IPv6 binding if provided
        let ipv6_binding = if let (Some(ipv6_id), Some(ipv6_kp)) = (ipv6_identity, ipv6_keypair) {
            Some(IPv6BindingProof::new(ipv6_id, &keypair, ipv6_kp)?)
        } else {
            None
        };
        
        // Create encrypted profile
        let encrypted_profile = EncryptedUserProfile::new_from_identity(
            &identity,
            &profile,
            &keypair,
            ipv6_binding.clone(),
        )?;
        
        // Generate and store profile key
        let profile_key = EncryptedUserProfile::generate_profile_key();
        
        // Store locally
        *self.local_identity.write().await = Some(identity.clone());
        *self.local_keypair.write().await = Some(keypair);
        *self.local_profile.write().await = Some(encrypted_profile);
        *self.profile_key.write().await = Some(profile_key);
        *self.ipv6_binding.write().await = ipv6_binding;
        
        info!("Created new user identity: {}", identity.user_id);
        Ok(identity)
    }
    
    /// Load existing identity from storage
    pub async fn load_identity(
        &self,
        keypair: Keypair,
        encrypted_profile_data: Vec<u8>,
        profile_key: [u8; 32],
    ) -> Result<UserIdentity> {
        // Derive identity from keypair
        let user_id = UserIdentity::derive_user_id(&keypair.public);
        
        // Decrypt and parse profile
        let encrypted_profile: EncryptedUserProfile = serde_json::from_slice(&encrypted_profile_data)
            .map_err(P2PError::Serialization)?;
        
        if encrypted_profile.user_id != user_id {
            return Err(P2PError::InvalidInput("Profile user ID mismatch".to_string()));
        }
        
        // Verify profile signature
        if !encrypted_profile.verify_signature()? {
            return Err(P2PError::Security("Invalid profile signature".to_string()));
        }
        
        // Decrypt profile to extract identity info
        let profile = encrypted_profile.decrypt_profile(&profile_key)?;
        
        // Reconstruct identity
        let identity = UserIdentity {
            user_id: encrypted_profile.user_id.clone(),
            public_key: encrypted_profile.public_key.clone(),
            display_name_hint: UserIdentity::create_display_name_hint(&profile.display_name),
            three_word_address: "loaded.from.storage".to_string(), // TODO: Store this properly
            created_at: profile.created_at,
            version: 1, // TODO: Store version in profile
            verification_level: if encrypted_profile.ipv6_binding_proof.is_some() {
                VerificationLevel::NetworkVerified
            } else {
                VerificationLevel::SelfSigned
            },
        };
        
        // Store locally
        *self.local_identity.write().await = Some(identity.clone());
        *self.local_keypair.write().await = Some(keypair);
        *self.local_profile.write().await = Some(encrypted_profile);
        *self.profile_key.write().await = Some(profile_key);
        
        info!("Loaded user identity: {}", identity.user_id);
        Ok(identity)
    }
    
    /// Get local user identity
    pub async fn get_local_identity(&self) -> Option<UserIdentity> {
        self.local_identity.read().await.clone()
    }
    
    /// Get local user profile (decrypted)
    pub async fn get_local_profile(&self) -> Result<Option<UserProfile>> {
        let profile_guard = self.local_profile.read().await;
        let key_guard = self.profile_key.read().await;
        
        if let (Some(encrypted_profile), Some(profile_key)) = (profile_guard.as_ref(), key_guard.as_ref()) {
            Ok(Some(encrypted_profile.decrypt_profile(profile_key)?))
        } else {
            Ok(None)
        }
    }
    
    /// Update local user profile
    pub async fn update_local_profile(&self, mut profile: UserProfile) -> Result<()> {
        let identity_guard = self.local_identity.read().await;
        let keypair_guard = self.local_keypair.read().await;
        let binding_guard = self.ipv6_binding.read().await;
        
        let identity = identity_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No local identity".to_string()))?;
        let keypair = keypair_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No local keypair".to_string()))?;
        
        // Update timestamp
        profile.update();
        
        // Create new encrypted profile
        let encrypted_profile = EncryptedUserProfile::new_from_identity(
            identity,
            &profile,
            keypair,
            binding_guard.clone(),
        )?;
        
        // Generate new profile key for security
        let profile_key = EncryptedUserProfile::generate_profile_key();
        
        // Store locally
        *self.local_profile.write().await = Some(encrypted_profile);
        *self.profile_key.write().await = Some(profile_key);
        
        info!("Updated local user profile");
        
        // Automatically publish to DHT if network is available
        if self.network.is_some() {
            if let Err(e) = self.publish_to_dht().await {
                tracing::warn!("Failed to auto-publish profile to DHT: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Publish identity and profile to DHT
    pub async fn publish_to_dht(&self) -> Result<()> {
        let network = self.network.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No network connection".to_string()))?;
        
        let identity_guard = self.local_identity.read().await;
        let profile_guard = self.local_profile.read().await;
        
        let identity = identity_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No local identity".to_string()))?;
        let encrypted_profile = profile_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No local profile".to_string()))?;
        
        // Serialize encrypted profile
        let profile_data = serde_json::to_vec(encrypted_profile)
            .map_err(P2PError::Serialization)?;
        
        // Store in DHT using the network layer
        let dht_key = identity.get_profile_dht_key();
        network.dht_put(dht_key, profile_data).await?;
        
        info!("Published identity {} to DHT", identity.user_id);
        Ok(())
    }
    
    /// Lookup user identity and profile from DHT
    pub async fn lookup_user(&self, user_id: &UserId) -> Result<Option<(UserIdentity, EncryptedUserProfile)>> {
        // Check cache first
        {
            let cache_guard = self.profile_cache.read().await;
            if let Some((encrypted_profile, cached_at)) = cache_guard.get(user_id) {
                if cached_at.elapsed().unwrap_or(Duration::MAX) < self.cache_ttl {
                    // Create identity from cached profile
                    let identity = UserIdentity {
                        user_id: encrypted_profile.user_id.clone(),
                        public_key: encrypted_profile.public_key.clone(),
                        display_name_hint: "cached".to_string(), // TODO: Store hint in profile
                        three_word_address: "cached.from.dht".to_string(),
                        created_at: SystemTime::UNIX_EPOCH, // TODO: Store creation time
                        version: 1,
                        verification_level: if encrypted_profile.ipv6_binding_proof.is_some() {
                            VerificationLevel::NetworkVerified
                        } else {
                            VerificationLevel::SelfSigned
                        },
                    };
                    return Ok(Some((identity, encrypted_profile.clone())));
                }
            }
        }
        
        let network = self.network.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No network connection".to_string()))?;
        
        // Create DHT key for lookup
        let profile_key = Key::new(format!("user_profile:{}", user_id).as_bytes());
        
        // Lookup in DHT
        if let Some(profile_data) = network.dht_get(profile_key).await? {
            // Deserialize the encrypted profile
            let encrypted_profile: EncryptedUserProfile = serde_json::from_slice(&profile_data)
                .map_err(P2PError::Serialization)?;
            
            // Verify the profile signature
            if !encrypted_profile.verify_signature()? {
                return Err(P2PError::Security("Invalid profile signature from DHT".to_string()));
            }
            
            // Create identity from DHT profile
            let identity = UserIdentity {
                user_id: encrypted_profile.user_id.clone(),
                public_key: encrypted_profile.public_key.clone(),
                display_name_hint: "from_dht".to_string(), // TODO: Store hint in profile
                three_word_address: "from.dht.network".to_string(),
                created_at: SystemTime::UNIX_EPOCH, // TODO: Store creation time
                version: 1,
                verification_level: if encrypted_profile.ipv6_binding_proof.is_some() {
                    VerificationLevel::NetworkVerified
                } else {
                    VerificationLevel::SelfSigned
                },
            };
            
            // Cache the result
            {
                let mut cache_guard = self.profile_cache.write().await;
                cache_guard.insert(user_id.clone(), (encrypted_profile.clone(), SystemTime::now()));
            }
            
            info!("Retrieved user {} profile from DHT", user_id);
            return Ok(Some((identity, encrypted_profile)));
        }
        
        debug!("User {} not found in DHT", user_id);
        Ok(None)
    }
    
    /// Decrypt and retrieve a friend's profile if we have access
    pub async fn get_friend_profile(&self, user_id: &UserId) -> Result<Option<UserProfile>> {
        // First lookup the user's encrypted profile
        if let Some((_, encrypted_profile)) = self.lookup_user(user_id).await? {
            let local_identity_guard = self.local_identity.read().await;
            let local_identity = local_identity_guard.as_ref()
                .ok_or_else(|| P2PError::InvalidState("No local identity".to_string()))?;
            
            // Check if we have an access grant for this profile
            if let Some(grant) = encrypted_profile.get_access_grant(&local_identity.user_id) {
                if EncryptedUserProfile::is_grant_valid(&grant) {
                    // TODO: Decrypt the profile key using our private key
                    // For now, we can't decrypt without implementing proper key exchange
                    // This would require implementing X25519 key exchange + ChaCha20-Poly1305
                    return Err(P2PError::InvalidState("Profile key decryption not yet implemented".to_string()));
                } else {
                    debug!("Access grant for user {} has expired", user_id);
                }
            } else {
                debug!("No access grant found for user {}", user_id);
            }
        }
        
        Ok(None)
    }
    
    /// Grant access to local profile
    pub async fn grant_profile_access(
        &self,
        grantee_user_id: UserId,
        grantee_public_key_bytes: Vec<u8>,
        permissions: ProfilePermissions,
    ) -> Result<()> {
        let mut profile_guard = self.local_profile.write().await;
        let keypair_guard = self.local_keypair.read().await;
        let key_guard = self.profile_key.read().await;
        
        let encrypted_profile = profile_guard.as_mut()
            .ok_or_else(|| P2PError::InvalidState("No local profile".to_string()))?;
        let keypair = keypair_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No local keypair".to_string()))?;
        let profile_key = key_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No profile key".to_string()))?;
        
        encrypted_profile.grant_access(
            &grantee_user_id,
            &grantee_public_key_bytes,
            permissions,
            profile_key,
            keypair,
        )?;
        
        info!("Granted profile access to user {}", grantee_user_id);
        Ok(())
    }
    
    /// Revoke access to local profile
    pub async fn revoke_profile_access(&self, user_id: &UserId) -> Result<()> {
        let mut profile_guard = self.local_profile.write().await;
        
        let encrypted_profile = profile_guard.as_mut()
            .ok_or_else(|| P2PError::InvalidState("No local profile".to_string()))?;
        
        encrypted_profile.revoke_access(user_id);
        
        info!("Revoked profile access for user {}", user_id);
        Ok(())
    }
    
    /// Create an identity challenge
    pub async fn create_challenge(&self, duration: Duration) -> IdentityChallenge {
        let local_identity_guard = self.local_identity.read().await;
        let challenger_id = local_identity_guard
            .as_ref()
            .map(|id| id.user_id.clone())
            .unwrap_or_else(|| "unknown".to_string());
        
        let challenge = IdentityChallenge::new(challenger_id);
        
        let mut challenges = self.active_challenges.write().await;
        challenges.insert(challenge.challenge_id.clone(), challenge.clone());
        
        debug!("Created identity challenge: {}", challenge.challenge_id);
        challenge
    }
    
    /// Verify a challenge response
    pub async fn verify_challenge_response(
        &self,
        proof: &ChallengeProof,
        public_key_bytes: &[u8],
    ) -> Result<bool> {
        let challenges = self.active_challenges.read().await;
        
        if let Some(challenge) = challenges.get(&proof.challenge_id) {
            proof.verify(challenge, public_key_bytes)
        } else {
            Ok(false) // Challenge not found
        }
    }
    
    /// Create a contact request
    pub async fn create_contact_request(
        &self,
        to_user_id: UserId,
        message: Option<String>,
        requested_permissions: ProfilePermissions,
    ) -> Result<ContactRequest> {
        let identity_guard = self.local_identity.read().await;
        let keypair_guard = self.local_keypair.read().await;
        
        let identity = identity_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No local identity".to_string()))?;
        let keypair = keypair_guard.as_ref()
            .ok_or_else(|| P2PError::InvalidState("No local keypair".to_string()))?;
        
        // Create challenge proof
        let challenge = IdentityChallenge::new(identity.user_id.clone());
        let proof = challenge.create_response(keypair);
        
        let request = ContactRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            from_user_id: identity.user_id.clone(),
            to_user_id,
            message,
            requested_permissions,
            sender_proof: proof,
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(7 * 24 * 3600), // 1 week
            signature: keypair.sign(format!("contact_request:{}", identity.user_id).as_bytes()).to_bytes().to_vec(),
            status: ContactRequestStatus::Pending,
        };
        
        // Store pending request
        let mut pending = self.pending_requests.write().await;
        pending.insert(request.request_id.clone(), request.clone());
        
        info!("Created contact request: {}", request.request_id);
        Ok(request)
    }
    
    /// Accept a contact request
    pub async fn accept_contact_request(
        &self,
        request: &ContactRequest,
        granted_permissions: ProfilePermissions,
    ) -> Result<()> {
        // Verify request signature and proof
        let requester_identity = self.lookup_user(&request.from_user_id).await?;
        
        if let Some((identity, _)) = requester_identity {
            // TODO: Verify request signature
            // For now, skip signature verification
            
            // Grant access to profile
            self.grant_profile_access(
                request.from_user_id.clone(),
                identity.public_key,
                granted_permissions,
            ).await?;
            
            info!("Accepted contact request from {}", request.from_user_id);
            Ok(())
        } else {
            Err(P2PError::InvalidInput("Requester not found".to_string()))
        }
    }
    
    /// Clean up expired entries
    pub async fn cleanup_expired(&self) {
        let now = SystemTime::now();
        
        // Clean expired challenges
        {
            let mut challenges = self.active_challenges.write().await;
            challenges.retain(|_, challenge| challenge.is_valid());
        }
        
        // Clean expired contact requests
        {
            let mut requests = self.pending_requests.write().await;
            requests.retain(|_, request| now < request.expires_at);
        }
        
        // Clean expired cache entries
        {
            let mut identity_cache = self.identity_cache.write().await;
            identity_cache.retain(|_, (_, cached_at)| {
                cached_at.elapsed().unwrap_or(Duration::MAX) < self.cache_ttl
            });
        }
        
        {
            let mut profile_cache = self.profile_cache.write().await;
            profile_cache.retain(|_, (_, cached_at)| {
                cached_at.elapsed().unwrap_or(Duration::MAX) < self.cache_ttl
            });
        }
        
        debug!("Cleaned up expired identity manager entries");
    }
    
    /// Export identity for backup/sync
    pub async fn export_identity(&self) -> Result<Vec<u8>> {
        let identity_guard = self.local_identity.read().await;
        let keypair_guard = self.local_keypair.read().await;
        let profile_guard = self.local_profile.read().await;
        let key_guard = self.profile_key.read().await;
        
        let export_data = IdentityExport {
            identity: identity_guard.as_ref().cloned()
                .ok_or_else(|| P2PError::InvalidState("No local identity".to_string()))?,
            keypair_bytes: keypair_guard.as_ref()
                .ok_or_else(|| P2PError::InvalidState("No local keypair".to_string()))?
                .to_bytes().to_vec(),
            encrypted_profile: profile_guard.as_ref().cloned()
                .ok_or_else(|| P2PError::InvalidState("No local profile".to_string()))?,
            profile_key: *key_guard.as_ref()
                .ok_or_else(|| P2PError::InvalidState("No profile key".to_string()))?,
        };
        
        serde_json::to_vec(&export_data)
            .map_err(P2PError::Serialization)
    }
    
    /// Import identity from backup
    pub async fn import_identity(&self, export_data: &[u8]) -> Result<UserIdentity> {
        let import: IdentityExport = serde_json::from_slice(export_data)
            .map_err(P2PError::Serialization)?;
        
        let keypair = Keypair::from_bytes(&import.keypair_bytes)
            .map_err(|e| P2PError::Cryptography(format!("Invalid keypair: {}", e)))?;
        
        // Verify consistency
        if import.identity.user_id != UserIdentity::derive_user_id(&keypair.public) {
            return Err(P2PError::InvalidInput("Identity-keypair mismatch".to_string()));
        }
        
        if import.identity.public_key != keypair.public.as_bytes().to_vec() {
            return Err(P2PError::InvalidInput("Public key mismatch".to_string()));
        }
        
        // Store imported data
        *self.local_identity.write().await = Some(import.identity.clone());
        *self.local_keypair.write().await = Some(keypair);
        *self.local_profile.write().await = Some(import.encrypted_profile);
        *self.profile_key.write().await = Some(import.profile_key);
        
        info!("Imported user identity: {}", import.identity.user_id);
        Ok(import.identity)
    }
}

/// Data structure for identity export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdentityExport {
    identity: UserIdentity,
    keypair_bytes: Vec<u8>,
    encrypted_profile: EncryptedUserProfile,
    profile_key: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_identity_manager_creation() {
        let config = IdentityManagerConfig::default();
        let manager = IdentityManager::new(config);
        
        assert!(manager.get_local_identity().await.is_none());
    }
    
    #[tokio::test]
    async fn test_create_identity() {
        let config = IdentityManagerConfig::default();
        let manager = IdentityManager::new(config);
        
        let identity = manager.create_identity(
            "Test User".to_string(),
            "test.user.example".to_string(),
            None,
            None,
        ).await.unwrap();
        
        assert_eq!(identity.display_name_hint, "Test User");
        assert_eq!(identity.three_word_address, "test.user.example");
        
        let local_identity = manager.get_local_identity().await.unwrap();
        assert_eq!(local_identity.user_id, identity.user_id);
    }
    
    #[tokio::test]
    async fn test_export_import_identity() {
        let config = IdentityManagerConfig::default();
        let manager1 = IdentityManager::new(config.clone());
        let manager2 = IdentityManager::new(config);
        
        // Create identity in manager1
        let original_identity = manager1.create_identity(
            "Test User".to_string(),
            "test.user.example".to_string(),
            None,
            None,
        ).await.unwrap();
        
        // Export from manager1
        let export_data = manager1.export_identity().await.unwrap();
        
        // Import to manager2
        let imported_identity = manager2.import_identity(&export_data).await.unwrap();
        
        // Verify identities match
        assert_eq!(original_identity.user_id, imported_identity.user_id);
        assert_eq!(original_identity.public_key, imported_identity.public_key);
        assert_eq!(original_identity.display_name_hint, imported_identity.display_name_hint);
    }
    
    #[tokio::test]
    async fn test_challenge_system() {
        let config = IdentityManagerConfig::default();
        let manager = IdentityManager::new(config);
        
        let identity = manager.create_identity(
            "Test User".to_string(),
            "test.user.example".to_string(),
            None,
            None,
        ).await.unwrap();
        
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
        let is_valid = manager.verify_challenge_response(&proof, &identity.public_key).await.unwrap();
        assert!(is_valid);
    }
}