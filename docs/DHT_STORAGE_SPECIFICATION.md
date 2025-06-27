# DHT Storage Specification: Privacy-First Distributed Data Management

## Overview

The Ant Core DHT (Distributed Hash Table) provides privacy-first, encrypted storage with friend-based access control. Unlike traditional DHTs that store public data, our implementation prioritizes user privacy while enabling seamless data sharing among trusted contacts.

## Core Architecture

### Privacy-First Design Philosophy
```
Traditional DHT              vs              Ant Core DHT
┌─────────────────────┐                    ┌─────────────────────┐
│ Public Data Storage │                    │ Encrypted by Default│
│ Anyone can read     │                    │ Friend-based Access │
│ Global visibility   │                    │ Zero-knowledge Ops  │
│ Content-based keys  │                    │ Identity-based Keys │
└─────────────────────┘                    └─────────────────────┘
```

### Layered Security Model
```
┌─────────────────────────────────────────┐
│            Application Layer            │  ← Saorsa Chat App
├─────────────────────────────────────────┤
│         Friend Access Control          │  ← Who can see what
├─────────────────────────────────────────┤
│       AES-256-GCM Encryption          │  ← Data encryption
├─────────────────────────────────────────┤
│      Ed25519 Authentication           │  ← Identity verification
├─────────────────────────────────────────┤
│       Kademlia DHT Storage            │  ← Distributed storage
├─────────────────────────────────────────┤
│         QUIC Transport                 │  ← Network layer
└─────────────────────────────────────────┘
```

## Data Structures

### Encrypted Storage Record
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedStorageRecord {
    /// Blake3 hash of the original key
    pub key_hash: [u8; 32],
    
    /// AES-256-GCM encrypted data
    pub encrypted_data: Vec<u8>,
    
    /// Unique nonce for this encryption
    pub nonce: [u8; 12],
    
    /// Owner's user ID
    pub owner_id: UserId,
    
    /// List of users who can decrypt this data
    pub access_grants: Vec<AccessGrant>,
    
    /// Data size in bytes (for token economics)
    pub size: u64,
    
    /// When this record expires
    pub ttl: SystemTime,
    
    /// Ed25519 signature over all fields
    pub signature: Signature,
    
    /// Metadata for efficient querying
    pub metadata: StorageMetadata,
}
```

### Friend-Based Access Control
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessGrant {
    /// Friend's user ID
    pub friend_id: UserId,
    
    /// AES key encrypted with friend's public key
    pub encrypted_key: Vec<u8>,
    
    /// What level of access they have
    pub access_level: AccessLevel,
    
    /// When this access expires
    pub expires_at: Option<SystemTime>,
    
    /// Metadata this friend can see
    pub visible_metadata: Vec<MetadataField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessLevel {
    /// Can read the data
    Read,
    
    /// Can read and update metadata
    ReadWrite,
    
    /// Can read, write, and grant access to others
    Admin,
    
    /// Custom permissions
    Custom(Vec<Permission>),
}
```

### Storage Metadata
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    /// Data type for client filtering
    pub data_type: DataType,
    
    /// Tags for searching/organizing
    pub tags: Vec<String>,
    
    /// Creation timestamp
    pub created_at: SystemTime,
    
    /// Last modification timestamp
    pub updated_at: SystemTime,
    
    /// Version number for conflict resolution
    pub version: u64,
    
    /// Optional description
    pub description: Option<String>,
    
    /// Priority for replication
    pub priority: ReplicationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    /// User profile information
    UserProfile,
    
    /// Chat message
    Message,
    
    /// File attachment
    File,
    
    /// Contact/friend information  
    Contact,
    
    /// Application settings
    Settings,
    
    /// Custom type
    Custom(String),
}
```

## Key Generation Strategy

### Identity-Based Keys
```rust
impl KeyGenerator {
    /// Generate key for user's own data
    pub fn user_data_key(user_id: &UserId, data_type: &str, id: &str) -> DHTKey {
        let input = format!("user:{}:{}:{}", user_id, data_type, id);
        DHTKey::new(blake3::hash(input.as_bytes()).as_bytes())
    }
    
    /// Generate key for shared friend data
    pub fn friend_data_key(user1: &UserId, user2: &UserId, data_type: &str) -> DHTKey {
        let mut users = vec![user1, user2];
        users.sort(); // Ensure consistent ordering
        let input = format!("shared:{}:{}:{}", users[0], users[1], data_type);
        DHTKey::new(blake3::hash(input.as_bytes()).as_bytes())
    }
    
    /// Generate key for group data
    pub fn group_data_key(group_id: &str, data_type: &str, id: &str) -> DHTKey {
        let input = format!("group:{}:{}:{}", group_id, data_type, id);
        DHTKey::new(blake3::hash(input.as_bytes()).as_bytes())
    }
}
```

### Key Examples
```
User Profile: user:alice_user_id:profile:main
Chat Message: shared:alice_id:bob_id:message:msg_123
File Share:   user:alice_user_id:file:document_456
Group Chat:   group:dev_team:message:msg_789
```

## Encryption Strategy

### Per-Record Encryption
```rust
impl EncryptionManager {
    /// Encrypt data with unique key per record
    pub async fn encrypt_for_storage(
        &self,
        data: &[u8],
        owner_id: &UserId,
        access_grants: &[AccessGrant],
    ) -> Result<EncryptedStorageRecord> {
        // Generate unique AES key for this record
        let record_key = self.generate_record_key();
        let nonce = self.generate_nonce();
        
        // Encrypt the actual data
        let encrypted_data = aes_gcm_encrypt(data, &record_key, &nonce)?;
        
        // Encrypt the record key for each authorized friend
        let mut grants = Vec::new();
        for grant in access_grants {
            let encrypted_key = self.encrypt_key_for_friend(&record_key, &grant.friend_id)?;
            grants.push(AccessGrant {
                friend_id: grant.friend_id,
                encrypted_key,
                access_level: grant.access_level.clone(),
                expires_at: grant.expires_at,
                visible_metadata: grant.visible_metadata.clone(),
            });
        }
        
        // Create signed record
        let record = EncryptedStorageRecord {
            key_hash: blake3::hash(data).into(),
            encrypted_data,
            nonce,
            owner_id: *owner_id,
            access_grants: grants,
            size: data.len() as u64,
            ttl: SystemTime::now() + Duration::from_secs(86400 * 30), // 30 days
            signature: [0; 64], // Will be filled below
            metadata: StorageMetadata::default(),
        };
        
        // Sign the record
        let signature = self.sign_record(&record)?;
        Ok(EncryptedStorageRecord { signature, ..record })
    }
}
```

### Friend-Based Decryption
```rust
impl DecryptionManager {
    /// Decrypt data if user has access
    pub async fn decrypt_if_authorized(
        &self,
        record: &EncryptedStorageRecord,
        user_id: &UserId,
    ) -> Result<Option<Vec<u8>>> {
        // Check if user is the owner
        if record.owner_id == *user_id {
            let record_key = self.derive_owner_key(&record.owner_id, &record.key_hash)?;
            return Ok(Some(aes_gcm_decrypt(&record.encrypted_data, &record_key, &record.nonce)?));
        }
        
        // Check if user has friend access
        for grant in &record.access_grants {
            if grant.friend_id == *user_id {
                // Decrypt the record key using our private key
                let record_key = self.decrypt_key_from_friend(&grant.encrypted_key)?;
                let data = aes_gcm_decrypt(&record.encrypted_data, &record_key, &record.nonce)?;
                return Ok(Some(data));
            }
        }
        
        // No access
        Ok(None)
    }
}
```

## Replication Strategy

### Intelligent Replication
```rust
#[derive(Debug, Clone)]
pub struct ReplicationStrategy {
    /// Base replication factor
    pub base_factor: u8,
    
    /// Additional replicas for important data
    pub priority_bonus: u8,
    
    /// Minimum replicas for any data
    pub min_replicas: u8,
    
    /// Maximum replicas to avoid waste
    pub max_replicas: u8,
}

impl ReplicationManager {
    /// Calculate optimal replication for data
    pub fn calculate_replication(&self, metadata: &StorageMetadata) -> u8 {
        let base = self.strategy.base_factor;
        
        let priority_bonus = match metadata.priority {
            ReplicationPriority::Critical => 3,
            ReplicationPriority::High => 2,
            ReplicationPriority::Normal => 0,
            ReplicationPriority::Low => 0,
        };
        
        let data_type_bonus = match metadata.data_type {
            DataType::UserProfile => 2, // Profiles need high availability
            DataType::Message => 1,     // Messages are important
            DataType::File => 0,        // Files can be re-uploaded
            DataType::Contact => 2,     // Contact info is critical
            DataType::Settings => 1,    // Settings should be available
            DataType::Custom(_) => 0,
        };
        
        (base + priority_bonus + data_type_bonus)
            .min(self.strategy.max_replicas)
            .max(self.strategy.min_replicas)
    }
}
```

### Geographic Distribution
```rust
impl GeographicReplication {
    /// Select diverse peers for replication
    pub async fn select_replica_peers(
        &self,
        key: &DHTKey,
        required_replicas: u8,
    ) -> Result<Vec<PeerId>> {
        let mut candidates = self.dht.find_closest_peers(key).await?;
        
        // Sort by distance (standard Kademlia)
        candidates.sort_by_key(|peer| xor_distance(key, &peer.id));
        
        // Apply geographic diversity
        let mut selected = Vec::new();
        let mut regions_used = HashSet::new();
        
        for peer in candidates {
            if selected.len() >= required_replicas as usize {
                break;
            }
            
            let region = self.get_peer_region(&peer.id).await?;
            
            // Prefer peers in new regions, but don't require it
            if !regions_used.contains(&region) || selected.len() < (required_replicas / 2) as usize {
                selected.push(peer.id);
                regions_used.insert(region);
            }
        }
        
        Ok(selected)
    }
}
```

## Token Economics Integration

### Storage Cost Calculation
```rust
impl TokenEconomics {
    /// Calculate ANT tokens required for storage
    pub fn calculate_storage_cost(
        &self,
        data_size: u64,
        replication_factor: u8,
        duration_days: u32,
    ) -> u64 {
        let base_cost_per_gb_day = 1000; // 1000 ANT tokens per GB per day
        let gb_size = (data_size as f64) / (1024.0 * 1024.0 * 1024.0);
        let total_gb_days = gb_size * (replication_factor as f64) * (duration_days as f64);
        (total_gb_days * (base_cost_per_gb_day as f64)) as u64
    }
    
    /// Calculate ANT tokens earned for providing storage
    pub fn calculate_storage_earnings(
        &self,
        bytes_stored: u64,
        uptime_percentage: f32,
        duration_hours: u32,
    ) -> u64 {
        let base_rate_per_gb_hour = 10; // 10 ANT tokens per GB per hour
        let gb_stored = (bytes_stored as f64) / (1024.0 * 1024.0 * 1024.0);
        let adjusted_rate = (base_rate_per_gb_hour as f64) * (uptime_percentage as f64);
        (gb_stored * adjusted_rate * (duration_hours as f64)) as u64
    }
}
```

### Automatic Economic Management
```rust
impl AIWalletIntegration {
    /// Automatically manage storage economics
    pub async fn manage_storage_economics(
        &mut self,
        operation: StorageOperation,
    ) -> Result<()> {
        match operation {
            StorageOperation::Store { data_size, duration, priority } => {
                let replication = self.calculate_replication(priority);
                let cost = self.token_economics.calculate_storage_cost(
                    data_size, 
                    replication, 
                    duration
                );
                
                if self.wallet.balance() >= cost {
                    self.wallet.spend_tokens(cost).await?;
                    self.metrics.record_spending(cost, "storage").await;
                } else {
                    // AI decides whether to purchase more tokens or optimize usage
                    self.handle_insufficient_tokens(cost).await?;
                }
            }
            
            StorageOperation::Provide { bytes_contributed, uptime } => {
                let earnings = self.token_economics.calculate_storage_earnings(
                    bytes_contributed,
                    uptime,
                    1 // per hour
                );
                
                self.wallet.earn_tokens(earnings).await?;
                self.metrics.record_earnings(earnings, "storage_provision").await;
            }
        }
        
        Ok(())
    }
}
```

## Privacy Features

### Zero-Knowledge Queries
```rust
impl PrivacyPreservingQueries {
    /// Search without revealing search terms
    pub async fn private_search(
        &self,
        search_terms: &[String],
        user_id: &UserId,
    ) -> Result<Vec<DHTKey>> {
        // Use Bloom filters to search without revealing exact terms
        let bloom_filter = self.create_search_bloom_filter(search_terms);
        
        // Query nodes with bloom filter
        let candidates = self.dht.query_with_bloom_filter(&bloom_filter).await?;
        
        // Client-side filtering of results we can decrypt
        let mut accessible_keys = Vec::new();
        for key in candidates {
            if let Some(record) = self.dht.get_record(&key).await? {
                if self.can_decrypt_record(&record, user_id).await? {
                    accessible_keys.push(key);
                }
            }
        }
        
        Ok(accessible_keys)
    }
}
```

### Friend Discovery
```rust
impl FriendDiscovery {
    /// Find friends without revealing your contact list
    pub async fn discover_mutual_friends(
        &self,
        my_contacts: &[UserId],
        friend_id: &UserId,
    ) -> Result<Vec<UserId>> {
        // Create bloom filter of your contacts
        let my_bloom = self.create_contact_bloom_filter(my_contacts);
        
        // Exchange bloom filters with friend
        let friend_bloom = self.exchange_bloom_filters(friend_id, &my_bloom).await?;
        
        // Check for possible matches (with false positive rate)
        let mut possible_matches = Vec::new();
        for contact in my_contacts {
            if friend_bloom.might_contain(&contact.to_bytes()) {
                possible_matches.push(*contact);
            }
        }
        
        // Confirm matches with zero-knowledge proof
        let confirmed_matches = self.confirm_mutual_contacts(
            &possible_matches,
            friend_id
        ).await?;
        
        Ok(confirmed_matches)
    }
}
```

## Performance Optimizations

### Caching Strategy
```rust
impl CacheManager {
    /// Multi-level caching for performance
    pub async fn get_with_cache(&self, key: &DHTKey) -> Result<Option<Vec<u8>>> {
        // L1: Local memory cache
        if let Some(data) = self.memory_cache.get(key) {
            self.metrics.record_cache_hit("memory").await;
            return Ok(Some(data));
        }
        
        // L2: Local disk cache
        if let Some(data) = self.disk_cache.get(key).await? {
            self.memory_cache.insert(key.clone(), data.clone());
            self.metrics.record_cache_hit("disk").await;
            return Ok(Some(data));
        }
        
        // L3: DHT network lookup
        if let Some(record) = self.dht.get_record(key).await? {
            if let Some(data) = self.decrypt_if_authorized(&record).await? {
                self.disk_cache.insert(key, &data).await?;
                self.memory_cache.insert(key.clone(), data.clone());
                self.metrics.record_cache_miss().await;
                return Ok(Some(data));
            }
        }
        
        Ok(None)
    }
}
```

### Batch Operations
```rust
impl BatchOperations {
    /// Store multiple records efficiently
    pub async fn batch_store(
        &self,
        records: Vec<(DHTKey, Vec<u8>, Vec<AccessGrant>)>,
    ) -> Result<Vec<OperationResult>> {
        // Group by target peers to minimize network round-trips
        let grouped = self.group_by_target_peers(&records).await?;
        
        let mut results = Vec::new();
        for (peer_id, peer_records) in grouped {
            let batch_result = self.send_batch_to_peer(&peer_id, peer_records).await?;
            results.extend(batch_result);
        }
        
        Ok(results)
    }
    
    /// Retrieve multiple records efficiently
    pub async fn batch_get(
        &self,
        keys: Vec<DHTKey>,
    ) -> Result<Vec<Option<Vec<u8>>>> {
        // Use parallel requests with connection pooling
        let futures = keys.into_iter().map(|key| {
            self.get_with_cache(&key)
        });
        
        let results = futures::future::join_all(futures).await;
        results.into_iter().collect()
    }
}
```

## Monitoring and Analytics

### Privacy-Preserving Metrics
```rust
impl PrivacyPreservingMetrics {
    /// Collect metrics without revealing user data
    pub async fn record_operation_metrics(&self, operation: &str, success: bool) {
        // Only record aggregate statistics
        self.operation_counter.increment(&[
            ("operation", operation),
            ("success", &success.to_string()),
        ]);
        
        // Use differential privacy for detailed metrics
        if self.should_report_detailed_metrics() {
            let noisy_value = self.add_differential_privacy_noise(1.0);
            self.detailed_metrics.record(operation, noisy_value);
        }
    }
    
    /// Health metrics for network monitoring
    pub async fn get_network_health(&self) -> NetworkHealth {
        NetworkHealth {
            total_records: self.approximate_record_count(),
            replication_health: self.calculate_replication_health(),
            storage_utilization: self.calculate_storage_utilization(),
            average_lookup_time: self.get_average_lookup_time(),
            // No individual user data exposed
        }
    }
}
```

## API Examples

### Saorsa Integration
```rust
// High-level API for Saorsa app
impl SaorsaStorageAPI {
    /// Store user profile
    pub async fn store_profile(
        &self,
        profile: &UserProfile,
        visible_to: &[UserId],
    ) -> Result<()> {
        let key = KeyGenerator::user_data_key(&profile.user_id, "profile", "main");
        let access_grants = self.create_friend_access_grants(visible_to, AccessLevel::Read);
        
        self.storage.store_encrypted(key, &profile.serialize()?, access_grants).await?;
        
        // AI wallet automatically handles token cost
        self.ai_wallet.handle_storage_payment(profile.size(), 30).await?;
        
        Ok(())
    }
    
    /// Store chat message
    pub async fn store_message(
        &self,
        message: &ChatMessage,
        friend_id: &UserId,
    ) -> Result<()> {
        let key = KeyGenerator::friend_data_key(&message.sender, friend_id, "message");
        let access_grants = vec![
            AccessGrant::new(*friend_id, AccessLevel::Read),
        ];
        
        self.storage.store_encrypted(key, &message.serialize()?, access_grants).await?;
        
        Ok(())
    }
    
    /// Get friend's profile (if shared with us)
    pub async fn get_friend_profile(&self, friend_id: &UserId) -> Result<Option<UserProfile>> {
        let key = KeyGenerator::user_data_key(friend_id, "profile", "main");
        
        if let Some(data) = self.storage.get_if_authorized(key, &self.current_user_id).await? {
            Ok(Some(UserProfile::deserialize(&data)?))
        } else {
            Ok(None)
        }
    }
}
```

## Future Enhancements

### Advanced Privacy Features
1. **Zero-Knowledge Proofs**: Prove data properties without revealing data
2. **Homomorphic Encryption**: Compute on encrypted data
3. **Secure Multi-Party Computation**: Collaborative computation on private data
4. **Anonymous Credentials**: Identity verification without revealing identity

### Scalability Improvements
1. **Sharding**: Distribute DHT across multiple shards
2. **Hierarchical Storage**: Hot/warm/cold data tiers
3. **Compression**: Automatic data compression
4. **Deduplication**: Eliminate duplicate data across users

### Integration Enhancements
1. **IPFS Compatibility**: Bridge with IPFS network
2. **Blockchain Integration**: Optional on-chain verification
3. **Web3 Standards**: Support for decentralized identity standards
4. **Cross-Network Bridges**: Connect with other P2P networks

---

**Privacy-first distributed storage for the decentralized future.** 🔒🌐