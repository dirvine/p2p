//! Identity Manager
//! 
//! Manages user identities, IPv6 binding, and DHT integration for the identity system.

use super::*;
// Placeholder types (identity.rs was removed)
pub type UserIdentity = String;
pub type EncryptedUserProfile = String;
pub type IPv6BindingProof = String;
pub type UserId = String;
use crate::dht::{Key, Record};
use crate::security::IPv6NodeID;
use crate::network::P2PNode;
use ed25519_dalek::Keypair;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
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
        let encrypted_profile = EncryptedUserProfile::new(
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
        let encrypted_profile = EncryptedUserProfile::new(
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
                if EncryptedUserProfile::is_grant_valid(grant) {
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
            grantee_user_id.clone(),
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
        let challenge = IdentityChallenge::new(duration);
        
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
        let challenge = IdentityChallenge::new(Duration::from_secs(300));
        let proof = challenge.create_response(identity.user_id.clone(), keypair)?;
        
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
        
        assert_eq!(identity.display_name_hint, "Test:a665a45920");
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
        
        // Create response with local keypair
        let keypair_guard = manager.local_keypair.read().await;
        let keypair = keypair_guard.as_ref().unwrap();
        let proof = challenge.create_response(identity.user_id.clone(), keypair).unwrap();
        
        // Verify response
        let is_valid = manager.verify_challenge_response(&proof, &identity.public_key).await.unwrap();
        assert!(is_valid);
    }
}