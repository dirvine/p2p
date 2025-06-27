# User ID Persistence Plan

## Overview

This document outlines the plan for implementing persistent user IDs on the P2P network, allowing users to maintain consistent identities across sessions and devices while ensuring security and privacy.

## Current State

- Users have temporary session-based identities
- Three-word addresses are generated but not persistent
- No cross-session identity continuity
- Contacts are local only (not shared across network)

## Goals

1. **Persistent Identity**: Users maintain the same identity across sessions
2. **Decentralized Storage**: No central authority for user data
3. **Privacy Protection**: User can control what information is shared
4. **Cross-Device Sync**: Access identity from multiple devices
5. **Recovery Mechanisms**: Restore identity if credentials are lost
6. **Network Discovery**: Find and connect to known contacts

## Technical Architecture

### 1. Cryptographic Identity

```rust
pub struct UserIdentity {
    /// Primary keypair for signing and identity
    pub signing_keypair: ed25519_dalek::Keypair,
    /// Encryption keypair for private messages
    pub encryption_keypair: x25519_dalek::StaticSecret,
    /// User's chosen display name
    pub display_name: String,
    /// Avatar image hash (optional)
    pub avatar_hash: Option<String>,
    /// Three-word address derived from public key
    pub three_word_address: String,
    /// Profile metadata
    pub profile: UserProfile,
    /// Creation timestamp
    pub created_at: SystemTime,
}

pub struct UserProfile {
    /// Public bio/description
    pub bio: Option<String>,
    /// Public contact preferences
    pub contact_preferences: ContactPreferences,
    /// Profile visibility settings
    pub visibility: ProfileVisibility,
}

pub enum ProfileVisibility {
    Public,       // Discoverable by anyone
    Contacts,     // Only visible to approved contacts
    Private,      // Not discoverable
}
```

### 2. Identity Storage Strategy

#### Local Storage
```rust
pub struct LocalIdentityStore {
    /// Encrypted identity file
    identity_file: PathBuf,
    /// Password-derived encryption key
    encryption_key: [u8; 32],
    /// Backup seed phrase
    seed_phrase: Option<SeedPhrase>,
}
```

#### DHT Storage
```rust
pub struct NetworkIdentityRecord {
    /// Public key (DHT key)
    pub_key: PublicKey,
    /// Signed profile data
    profile_data: SignedProfileData,
    /// Contact discovery info
    discovery_info: DiscoveryInfo,
    /// Revocation info (for key rotation)
    revocation_info: Option<RevocationInfo>,
}
```

### 3. Three-Word Address System

#### Address Generation
```rust
impl UserIdentity {
    /// Generate deterministic three-word address from public key
    pub fn generate_three_word_address(&self) -> String {
        let pub_key_hash = sha256(&self.signing_keypair.public.to_bytes());
        three_word_encoding::encode(pub_key_hash)
    }
    
    /// Resolve three-word address to public key
    pub fn resolve_address(address: &str) -> Result<PublicKey> {
        let hash = three_word_encoding::decode(address)?;
        // DHT lookup for identity record
        self.dht.get_identity_record(hash).await
    }
}
```

#### Address Registry
- Use DHT to store address → public key mappings
- Allow users to register custom vanity addresses
- Implement address collision resolution
- Support address aliases/redirects

### 4. Contact Management

#### Contact Discovery
```rust
pub struct ContactDiscovery {
    /// Search by three-word address
    pub fn find_by_address(address: &str) -> Result<UserProfile>,
    /// Search by display name (fuzzy matching)
    pub fn search_by_name(name: &str) -> Vec<UserProfile>,
    /// Discover mutual contacts
    pub fn find_mutual_contacts(user_id: &PublicKey) -> Vec<Contact>,
}
```

#### Contact Verification
```rust
pub struct ContactVerification {
    /// Verify contact authenticity via key signatures
    pub fn verify_contact(contact: &Contact) -> VerificationResult,
    /// Web of trust scoring
    pub fn trust_score(contact: &Contact, user: &UserIdentity) -> f64,
    /// Mutual contact verification
    pub fn verify_through_mutual(contact: &Contact) -> Vec<VerificationPath>,
}
```

### 5. Privacy and Security

#### Data Encryption
- All private data encrypted with user's key
- Profile data signed to prevent tampering
- Optional end-to-end encryption for messages
- Forward secrecy for message sessions

#### Access Control
```rust
pub struct AccessControl {
    /// Contact approval system
    contact_requests: Vec<ContactRequest>,
    /// Blocked users list
    blocked_users: HashSet<PublicKey>,
    /// Privacy settings per contact
    contact_permissions: HashMap<PublicKey, ContactPermissions>,
}

pub struct ContactPermissions {
    can_see_profile: bool,
    can_see_online_status: bool,
    can_send_messages: bool,
    can_see_mutual_contacts: bool,
}
```

### 6. Data Synchronization

#### Multi-Device Support
```rust
pub struct DeviceSync {
    /// Sync identity across devices
    pub fn sync_identity(device_id: &str) -> Result<()>,
    /// Sync contact list
    pub fn sync_contacts() -> Result<Vec<Contact>>,
    /// Sync message history (optional)
    pub fn sync_messages(contact_id: &PublicKey) -> Result<Vec<Message>>,
}
```

#### Conflict Resolution
- Last-write-wins for profile updates
- Merge contact lists intelligently
- Version vectors for complex sync scenarios

## Implementation Phases

### Phase 1: Basic Identity (Week 1-2)
- [ ] Generate cryptographic identity
- [ ] Local identity storage with encryption
- [ ] Three-word address generation
- [ ] Basic profile management

### Phase 2: Network Storage (Week 3-4)
- [ ] DHT integration for identity records
- [ ] Address → public key resolution
- [ ] Profile publishing and discovery
- [ ] Basic contact search

### Phase 3: Contact Management (Week 5-6)
- [ ] Contact request system
- [ ] Contact verification mechanisms
- [ ] Privacy controls and permissions
- [ ] Mutual contact discovery

### Phase 4: Advanced Features (Week 7-8)
- [ ] Multi-device synchronization
- [ ] Key rotation and recovery
- [ ] Web of trust implementation
- [ ] Performance optimizations

## User Experience Flow

### 1. First Time Setup
```
1. User opens app for first time
2. Generate or import identity
3. Choose display name and three-word address
4. Set privacy preferences
5. Publish identity to network
6. Import/add initial contacts
```

### 2. Daily Usage
```
1. App loads saved identity
2. Sync contacts and profile updates
3. Discover new contacts via search
4. Send/receive contact requests
5. Chat with approved contacts
```

### 3. Device Migration
```
1. Export identity with seed phrase
2. Install app on new device
3. Import identity from seed phrase
4. Sync contacts and message history
5. Revoke old device access (optional)
```

## Security Considerations

### Threat Model
- **Malicious nodes**: DHT pollution, identity spoofing
- **Network surveillance**: Traffic analysis, metadata leakage
- **Device compromise**: Local data extraction
- **Social engineering**: Impersonation, contact manipulation

### Mitigations
- **Cryptographic verification**: All data cryptographically signed
- **Distributed storage**: No single point of failure
- **Forward secrecy**: Compromise doesn't affect past communications
- **User education**: Clear security indicators and warnings

## Open Questions

1. **Seed phrase vs. password**: Which recovery method is more user-friendly?
2. **Contact request spam**: How to prevent unwanted contact requests?
3. **Name squatting**: How to handle disputes over three-word addresses?
4. **DHT persistence**: How long should identity records be cached?
5. **Mobile considerations**: How to handle background sync and battery usage?

## Success Metrics

- **User adoption**: % of users who set up persistent identity
- **Contact discovery**: Average number of contacts found per user
- **Session continuity**: % of users who successfully restore identity
- **Network effect**: Growth in connected user pairs over time
- **Security incidents**: Number of reported identity/contact issues

## Dependencies

### Existing Components
- DHT implementation (Kademlia)
- Three-word address system
- Cryptographic primitives (ed25519, x25519)
- Contact management UI

### New Components Needed
- Identity management module
- Secure local storage
- Network discovery protocols
- Contact verification system
- Multi-device sync logic

## Testing Strategy

### Unit Tests
- Cryptographic key generation and operations
- Three-word address encoding/decoding
- Contact verification algorithms
- Privacy permission enforcement

### Integration Tests
- DHT storage and retrieval
- Cross-device identity sync
- Contact discovery workflows
- Message encryption/decryption

### User Testing
- First-time setup experience
- Contact discovery and verification
- Multi-device setup process
- Recovery from lost credentials

## Conclusion

This plan provides a comprehensive approach to implementing persistent user IDs while maintaining the decentralized, privacy-focused nature of the P2P foundation. The phased implementation allows for iterative development and user feedback incorporation.

The system balances user convenience with security and privacy, providing a foundation for building trusted communication networks without central authorities.