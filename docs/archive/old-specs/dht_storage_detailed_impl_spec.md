# Secure DHT Storage Design for Multi-User P2P Applications - Implementation Specification

## Table of Contents

1. [Enhanced Encryption Tiers](#1-enhanced-encryption-tiers)
2. [DHT Record Structure](#2-enhanced-dht-record-structure)
3. [Serialization Strategy](#3-serialization-strategy)
4. [K=8 Replication Implementation](#4-k8-replication-implementation)
5. [Storage Manager Integration](#5-storage-manager-integration)
6. [Application-Specific Patterns](#6-application-specific-patterns)
7. [Synchronization Mechanisms](#7-synchronization-mechanisms)
8. [Access Control & Permissions](#8-access-control--permissions)
9. [DHT Key Strategy](#9-dht-key-strategy)
10. [Local Storage & Synchronization](#10-local-storage--synchronization)
11. [Multi-User Real-time Features](#11-multi-user-real-time-features)
12. [Implementation Architecture](#12-implementation-architecture)
13. [Security Guarantees](#13-security-guarantees)

---

## 1. Enhanced Encryption Tiers

Building on the existing `EncryptedData` structure from `ant-core`, we extend the encryption system to support multiple access levels with quantum-resistant cryptography.

### 1.1 Data Access Levels

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataAccessLevel {
    /// Public data - signed but readable by anyone
    Public {
        signature: MlDsaSignature,
        content_hash: [u8; 32],
    },
    
    /// Existing user-private level (extends current EncryptedData)
    UserPrivate {
        encrypted_data: EncryptedData,
        ml_kem_session_key: Vec<u8>,    // Session key encrypted with ML-KEM
        user_key_id: String,
    },
    
    /// Group-shared using threshold encryption
    GroupShared {
        encrypted_data: EncryptedData,
        threshold_metadata: ThresholdEncryptionMeta,
        group_id: GroupId,
        required_shares: u16,
    },
    
    /// Organization-level access control
    OrganizationLevel {
        encrypted_data: EncryptedData,
        org_id: OrgId,
        access_policy: AccessPolicy,
        permission_tokens: Vec<CapabilityToken>,
    },
}
```

### 1.2 Threshold Encryption Metadata

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdEncryptionMeta {
    pub shares: Vec<EncryptedShare>,     // Encrypted shares of the symmetric key
    pub share_polynomial: Vec<u8>,       // Polynomial coefficients for reconstruction
    pub verification_data: Vec<u8>,      // For verifying share authenticity
    pub threshold: u16,                  // Minimum shares needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedShare {
    pub participant_id: ParticipantId,
    pub encrypted_share: Vec<u8>,        // Share encrypted with participant's ML-KEM key
    pub share_commitment: Vec<u8>,       // Commitment for verification
}
```

### 1.3 Type Definitions

```rust
// Core identity types
pub type GroupId = String;
pub type OrgId = String;
pub type ParticipantId = String;
pub type ContentHash = [u8; 32];

// Quantum-resistant signature types
pub use ml_dsa::MlDsa65 as MlDsaSignature;
pub use ml_kem::MlKem768 as MlKemKeyPair;

// FROST threshold signature types  
pub use frost_ed25519::Signature as FrostSignature;
pub use frost_ed25519::Identifier as FrostParticipant;
```

---

## 2. Enhanced DHT Record Structure

Extends the existing `Record` struct from `ant-core` to support multi-user applications with enhanced security and versioning.

### 2.1 Core Record Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedDhtRecord {
    // Core DHT fields (from existing Record struct)
    pub key: Key,
    pub value: Vec<u8>,               // Serialized SecureRecordPayload
    pub publisher: PeerId,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    
    // Enhanced fields for multi-user apps
    pub access_level: DataAccessLevel,
    pub content_type: ContentType,
    pub version_vector: VersionVector,
    pub parent_hash: Option<ContentHash>,
    pub application_metadata: ApplicationMetadata,
    
    // Security and verification
    pub integrity_proof: IntegrityProof,
    pub threshold_signatures: Vec<FrostSignature>,  // For group-owned records
}
```

### 2.2 Secure Record Payload

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureRecordPayload {
    pub content: Vec<u8>,             // The actual application data
    pub compression: CompressionType,
    pub checksum: [u8; 32],          // Blake3 hash for integrity
    pub audit_trail: Vec<AuditEntry>,// Operation history
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub operation: String,
    pub timestamp: SystemTime,
    pub actor: PeerId,
    pub metadata: HashMap<String, String>,
}
```

### 2.3 Content Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContentType {
    // Public content types
    PublicChannel,
    PublicPost,
    PublicAnnouncement,
    UserDirectory,
    
    // Private content types  
    PrivateMessage,
    UserProfile,
    PrivateDocument,
    
    // Group content types
    GroupChannel,
    GroupDocument,
    GroupSettings,
    
    // System content types
    OrganizationConfig,
    PermissionGrant,
    ThresholdKeyShare,
    
    // File content types
    FileChunk,
    FileMetadata,
    MediaThumbnail,
    
    // Custom application types
    Custom(String),
}
```

### 2.4 Version Vector for Conflict Resolution

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionVector {
    pub peer_versions: HashMap<PeerId, u64>,
    pub last_modified: SystemTime,
}

impl VersionVector {
    pub fn increment(&mut self, peer_id: &PeerId) {
        let version = self.peer_versions.entry(peer_id.clone()).or_insert(0);
        *version += 1;
        self.last_modified = SystemTime::now();
    }
    
    pub fn merge(&mut self, other: &VersionVector) {
        for (peer_id, version) in &other.peer_versions {
            let current = self.peer_versions.entry(peer_id.clone()).or_insert(0);
            *current = (*current).max(*version);
        }
        self.last_modified = self.last_modified.max(other.last_modified);
    }
    
    pub fn compare(&self, other: &VersionVector) -> VectorComparison {
        let mut self_newer = false;
        let mut other_newer = false;
        
        // Collect all peer IDs from both vectors
        let all_peers: HashSet<_> = self.peer_versions.keys()
            .chain(other.peer_versions.keys())
            .collect();
        
        for peer_id in all_peers {
            let self_version = self.peer_versions.get(peer_id).unwrap_or(&0);
            let other_version = other.peer_versions.get(peer_id).unwrap_or(&0);
            
            if self_version > other_version {
                self_newer = true;
            } else if other_version > self_version {
                other_newer = true;
            }
        }
        
        match (self_newer, other_newer) {
            (true, false) => VectorComparison::NewerThan,
            (false, true) => VectorComparison::OlderThan,
            (false, false) => VectorComparison::Equal,
            (true, true) => VectorComparison::Concurrent,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VectorComparison {
    NewerThan,
    OlderThan,
    Equal,
    Concurrent,  // Conflict - requires resolution
}
```

### 2.5 Application Metadata

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicationMetadata {
    pub app_name: String,
    pub app_version: String,
    pub schema_version: u32,
    pub custom_fields: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal, 
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}
```

### 2.6 Integrity Proof

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityProof {
    pub content_hash: [u8; 32],       // Blake3 hash of content
    pub metadata_hash: [u8; 32],      // Hash of metadata
    pub signature: MlDsaSignature,    // Publisher's signature
    pub timestamp_proof: Option<TimestampProof>, // Optional timestamping
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampProof {
    pub timestamp: SystemTime,
    pub authority: String,             // Timestamping authority
    pub proof_data: Vec<u8>,          // Authority-specific proof
}
```

## 3. Serialization Strategy

Extends the existing bincode usage in `ant-core` with multiple serialization formats optimized for different use cases.

### 3.1 Serialization Format Selection

```rust
use bincode;
use postcard;  // For no_std compatibility and smaller size
use serde_cbor; // For structured data with schema evolution

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializationFormat {
    Bincode,      // Existing format for compatibility
    Postcard,     // Compact, deterministic for DHT keys
    Cbor,         // Schema evolution for application data
    MessagePack,  // Cross-language compatibility
}

#[derive(Debug, Clone)]
pub struct SerializationManager {
    default_format: SerializationFormat,
    compression_enabled: bool,
    compression_threshold: usize, // Only compress if data > threshold
}
```

### 3.2 Serialization Implementation

```rust
impl SerializationManager {
    pub fn new(default_format: SerializationFormat, compression_enabled: bool) -> Self {
        Self {
            default_format,
            compression_enabled,
            compression_threshold: 1024, // 1KB threshold
        }
    }
    
    pub fn serialize<T: Serialize>(
        &self, 
        data: &T, 
        format: Option<SerializationFormat>
    ) -> Result<Vec<u8>, SerializationError> {
        let format = format.unwrap_or(self.default_format.clone());
        
        let bytes = match format {
            SerializationFormat::Bincode => bincode::serialize(data)
                .map_err(|e| SerializationError::BincodeError(e))?,
            
            SerializationFormat::Postcard => postcard::to_allocvec(data)
                .map_err(|e| SerializationError::PostcardError(e.to_string()))?,
            
            SerializationFormat::Cbor => serde_cbor::to_vec(data)
                .map_err(|e| SerializationError::CborError(e))?,
            
            SerializationFormat::MessagePack => rmp_serde::to_vec(data)
                .map_err(|e| SerializationError::MessagePackError(e))?,
        };
        
        if self.compression_enabled && bytes.len() > self.compression_threshold {
            self.compress(&bytes)
        } else {
            Ok(bytes)
        }
    }
    
    pub fn deserialize<T: for<'de> Deserialize<'de>>(
        &self, 
        data: &[u8], 
        format: SerializationFormat
    ) -> Result<T, SerializationError> {
        let decompressed = if self.compression_enabled {
            self.decompress(data)?
        } else {
            data.to_vec()
        };
        
        match format {
            SerializationFormat::Bincode => bincode::deserialize(&decompressed)
                .map_err(|e| SerializationError::BincodeError(e)),
            
            SerializationFormat::Postcard => postcard::from_bytes(&decompressed)
                .map_err(|e| SerializationError::PostcardError(e.to_string())),
            
            SerializationFormat::Cbor => serde_cbor::from_slice(&decompressed)
                .map_err(|e| SerializationError::CborError(e)),
            
            SerializationFormat::MessagePack => rmp_serde::from_slice(&decompressed)
                .map_err(|e| SerializationError::MessagePackError(e)),
        }
    }
    
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, SerializationError> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)
            .map_err(|e| SerializationError::CompressionError(e.to_string()))?;
        encoder.finish()
            .map_err(|e| SerializationError::CompressionError(e.to_string()))
    }
    
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, SerializationError> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        
        let mut decoder = GzDecoder::new(data);
        let mut result = Vec::new();
        decoder.read_to_end(&mut result)
            .map_err(|e| SerializationError::CompressionError(e.to_string()))?;
        Ok(result)
    }
    
    /// Estimate the best format for given data size and type
    pub fn recommend_format(&self, data_size: usize, content_type: &ContentType) -> SerializationFormat {
        match content_type {
            // For DHT keys and small structured data, use postcard
            ContentType::UserDirectory | ContentType::GroupSettings => SerializationFormat::Postcard,
            
            // For large files and media, use bincode (fastest)
            ContentType::FileChunk | ContentType::MediaThumbnail if data_size > 10_000 => {
                SerializationFormat::Bincode
            }
            
            // For cross-language compatibility, use MessagePack
            ContentType::PublicPost | ContentType::PublicAnnouncement => {
                SerializationFormat::MessagePack
            }
            
            // For schema evolution, use CBOR
            ContentType::GroupDocument | ContentType::PrivateDocument => {
                SerializationFormat::Cbor
            }
            
            // Default to bincode for compatibility
            _ => SerializationFormat::Bincode,
        }
    }
}
```

### 3.3 Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    #[error("Bincode serialization error: {0}")]
    BincodeError(#[from] bincode::Error),
    
    #[error("Postcard serialization error: {0}")]
    PostcardError(String),
    
    #[error("CBOR serialization error: {0}")]
    CborError(#[from] serde_cbor::Error),
    
    #[error("MessagePack serialization error: {0}")]
    MessagePackError(#[from] rmp_serde::encode::Error),
    
    #[error("Compression error: {0}")]
    CompressionError(String),
    
    #[error("Custom serialization error: {0}")]
    Custom(String),
}
```

### 3.4 Format-Specific Optimizations

```rust
impl SerializationManager {
    /// Serialize with format-specific optimizations
    pub fn serialize_optimized<T: Serialize>(
        &self,
        data: &T,
        content_type: &ContentType,
    ) -> Result<(Vec<u8>, SerializationFormat), SerializationError> {
        // First, serialize to estimate size
        let temp_data = bincode::serialize(data)?;
        let recommended_format = self.recommend_format(temp_data.len(), content_type);
        
        // Re-serialize with optimal format if different
        let final_data = if matches!(recommended_format, SerializationFormat::Bincode) {
            temp_data
        } else {
            self.serialize(data, Some(recommended_format.clone()))?
        };
        
        Ok((final_data, recommended_format))
    }
    
    /// Batch serialize multiple records efficiently
    pub fn batch_serialize<T: Serialize>(
        &self,
        records: &[(T, ContentType)],
    ) -> Result<Vec<(Vec<u8>, SerializationFormat)>, SerializationError> {
        let mut results = Vec::with_capacity(records.len());
        
        for (data, content_type) in records {
            let (serialized, format) = self.serialize_optimized(data, content_type)?;
            results.push((serialized, format));
        }
        
        Ok(results)
    }
}
```

## 4. K=8 Replication Implementation

Building on the existing DHT infrastructure in `ant-core`, we implement a K=8 replication strategy for enhanced data availability and fault tolerance in multi-user applications.

### 4.1 Replication Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub replication_factor: usize,           // K=8 for production
    pub min_replication_factor: usize,       // Minimum acceptable replicas (K=3)
    pub preferred_distance_factor: f64,      // XOR distance preference (0.3)
    pub geographic_awareness: bool,          // Consider geographic distribution
    pub repair_threshold: usize,             // Trigger repair when replicas < this
    pub repair_interval: Duration,           // How often to check for repairs
    pub max_repair_concurrent: usize,        // Max concurrent repair operations
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            replication_factor: 8,
            min_replication_factor: 3,
            preferred_distance_factor: 0.3,
            geographic_awareness: true,
            repair_threshold: 5,
            repair_interval: Duration::from_secs(300), // 5 minutes
            max_repair_concurrent: 3,
        }
    }
}
```

### 4.2 Enhanced Record Manager

```rust
use ant_core::dht::{DHT, Key, Record};
use ant_core::network::P2PNode;

#[derive(Debug)]
pub struct EnhancedRecordManager {
    dht: Arc<DHT>,
    network: Arc<P2PNode>,
    config: ReplicationConfig,
    replication_tracker: Arc<RwLock<ReplicationTracker>>,
    repair_scheduler: Arc<Mutex<RepairScheduler>>,
}

impl EnhancedRecordManager {
    pub fn new(
        dht: Arc<DHT>, 
        network: Arc<P2PNode>, 
        config: ReplicationConfig
    ) -> Self {
        Self {
            dht,
            network,
            config,
            replication_tracker: Arc::new(RwLock::new(ReplicationTracker::new())),
            repair_scheduler: Arc::new(Mutex::new(RepairScheduler::new())),
        }
    }
    
    /// Store a record with K=8 replication
    pub async fn store_with_replication(
        &self,
        record: EnhancedDhtRecord,
    ) -> Result<ReplicationResult, ReplicationError> {
        let key = record.key.clone();
        let target_peers = self.select_optimal_peers(&key, self.config.replication_factor).await?;
        
        if target_peers.len() < self.config.min_replication_factor {
            return Err(ReplicationError::InsufficientPeers {
                required: self.config.min_replication_factor,
                available: target_peers.len(),
            });
        }
        
        let mut successful_stores = Vec::new();
        let mut failed_stores = Vec::new();
        
        // Serialize the record with optimal format
        let serialized_record = self.serialize_record(&record).await?;
        
        // Store to target peers concurrently
        let store_futures = target_peers.iter().map(|peer_id| {
            let record_data = serialized_record.clone();
            let key_clone = key.clone();
            let dht_clone = self.dht.clone();
            let peer_clone = peer_id.clone();
            
            async move {
                let store_result = dht_clone.put_record_to_peer(
                    key_clone,
                    record_data,
                    peer_clone.clone(),
                    Some(Duration::from_secs(30)) // Store timeout
                ).await;
                
                (peer_clone, store_result)
            }
        });
        
        let store_results = futures::future::join_all(store_futures).await;
        
        for (peer_id, result) in store_results {
            match result {
                Ok(_) => {
                    successful_stores.push(peer_id.clone());
                    // Track successful replication
                    self.replication_tracker.write().await.record_successful_store(
                        &key, &peer_id
                    );
                }
                Err(e) => {
                    failed_stores.push((peer_id, e));
                }
            }
        }
        
        let replication_result = ReplicationResult {
            key: key.clone(),
            successful_replicas: successful_stores.len(),
            failed_replicas: failed_stores.len(),
            target_replicas: target_peers.len(),
            successful_peers: successful_stores,
            failed_peers: failed_stores,
            is_sufficient: successful_stores.len() >= self.config.min_replication_factor,
        };
        
        // Schedule repair if needed
        if replication_result.successful_replicas < self.config.repair_threshold {
            self.repair_scheduler.lock().await.schedule_repair(&key, &replication_result);
        }
        
        Ok(replication_result)
    }
}
```

### 4.3 Peer Selection Strategy

```rust
impl EnhancedRecordManager {
    /// Select optimal peers for replication based on XOR distance and network topology
    async fn select_optimal_peers(
        &self,
        key: &Key,
        target_count: usize,
    ) -> Result<Vec<PeerId>, ReplicationError> {
        let all_peers = self.network.connected_peers().await;
        
        if all_peers.is_empty() {
            return Err(ReplicationError::NoPeersAvailable);
        }
        
        // Calculate XOR distances for all peers
        let mut peer_distances: Vec<(PeerId, u64)> = all_peers
            .into_iter()
            .map(|peer_id| {
                let distance = self.calculate_xor_distance(key, &peer_id);
                (peer_id, distance)
            })
            .collect();
        
        // Sort by XOR distance (closest first)
        peer_distances.sort_by_key(|(_, distance)| *distance);
        
        // Apply geographic distribution if enabled
        if self.config.geographic_awareness {
            peer_distances = self.apply_geographic_distribution(peer_distances).await?;
        }
        
        // Select top K peers, considering diversity
        let selected_peers = peer_distances
            .into_iter()
            .take(target_count)
            .map(|(peer_id, _)| peer_id)
            .collect();
        
        Ok(selected_peers)
    }
    
    /// Calculate XOR distance between key and peer ID
    fn calculate_xor_distance(&self, key: &Key, peer_id: &PeerId) -> u64 {
        let key_bytes = key.as_bytes();
        let peer_bytes = peer_id.as_bytes();
        
        // XOR the first 8 bytes and convert to u64 for comparison
        let mut xor_result = [0u8; 8];
        for i in 0..8.min(key_bytes.len()).min(peer_bytes.len()) {
            xor_result[i] = key_bytes[i] ^ peer_bytes[i];
        }
        
        u64::from_be_bytes(xor_result)
    }
    
    /// Apply geographic distribution to improve fault tolerance
    async fn apply_geographic_distribution(
        &self,
        mut peer_distances: Vec<(PeerId, u64)>,
    ) -> Result<Vec<(PeerId, u64)>, ReplicationError> {
        // Get geographic information for peers (from network metadata)
        let peer_geo_info = self.network.get_peer_geographic_info().await?;
        
        // Group peers by geographic region
        let mut regions: HashMap<String, Vec<(PeerId, u64)>> = HashMap::new();
        
        for (peer_id, distance) in peer_distances {
            let region = peer_geo_info
                .get(&peer_id)
                .map(|info| info.region.clone())
                .unwrap_or_else(|| "unknown".to_string());
            
            regions.entry(region).or_default().push((peer_id, distance));
        }
        
        // Select peers from different regions when possible
        let mut result = Vec::new();
        let mut region_iter = regions.values().cycle();
        
        while result.len() < self.config.replication_factor && !regions.is_empty() {
            let mut added_any = false;
            
            for region_peers in region_iter.by_ref().take(regions.len()) {
                if let Some((peer_id, distance)) = region_peers.first() {
                    result.push((peer_id.clone(), *distance));
                    // Remove the peer we just added
                    regions.values_mut().for_each(|peers| {
                        peers.retain(|(p, _)| p != peer_id);
                    });
                    added_any = true;
                    break;
                }
            }
            
            if !added_any {
                break; // No more peers available
            }
            
            // Remove empty regions
            regions.retain(|_, peers| !peers.is_empty());
        }
        
        Ok(result)
    }
}
```

### 4.4 Replication Tracking

```rust
#[derive(Debug, Default)]
pub struct ReplicationTracker {
    replica_locations: HashMap<Key, HashSet<PeerId>>,
    peer_health: HashMap<PeerId, PeerHealthInfo>,
    repair_queue: VecDeque<RepairTask>,
}

#[derive(Debug, Clone)]
pub struct PeerHealthInfo {
    pub success_rate: f64,
    pub last_successful_store: SystemTime,
    pub last_failed_store: Option<SystemTime>,
    pub total_attempts: u64,
    pub successful_attempts: u64,
}

#[derive(Debug, Clone)]
pub struct RepairTask {
    pub key: Key,
    pub current_replicas: Vec<PeerId>,
    pub required_replicas: usize,
    pub priority: RepairPriority,
    pub scheduled_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepairPriority {
    Low,      // Replicas above threshold but below target
    Medium,   // Replicas at threshold
    High,     // Replicas below threshold
    Critical, // Very few replicas remaining
}

impl ReplicationTracker {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_successful_store(&mut self, key: &Key, peer_id: &PeerId) {
        // Track replica location
        self.replica_locations
            .entry(key.clone())
            .or_default()
            .insert(peer_id.clone());
        
        // Update peer health
        let health = self.peer_health.entry(peer_id.clone()).or_insert_with(|| {
            PeerHealthInfo {
                success_rate: 1.0,
                last_successful_store: SystemTime::now(),
                last_failed_store: None,
                total_attempts: 0,
                successful_attempts: 0,
            }
        });
        
        health.total_attempts += 1;
        health.successful_attempts += 1;
        health.success_rate = health.successful_attempts as f64 / health.total_attempts as f64;
        health.last_successful_store = SystemTime::now();
    }
    
    pub fn record_failed_store(&mut self, key: &Key, peer_id: &PeerId, error: &dyn std::error::Error) {
        // Update peer health
        let health = self.peer_health.entry(peer_id.clone()).or_insert_with(|| {
            PeerHealthInfo {
                success_rate: 0.0,
                last_successful_store: SystemTime::UNIX_EPOCH,
                last_failed_store: Some(SystemTime::now()),
                total_attempts: 0,
                successful_attempts: 0,
            }
        });
        
        health.total_attempts += 1;
        health.success_rate = health.successful_attempts as f64 / health.total_attempts as f64;
        health.last_failed_store = Some(SystemTime::now());
        
        // Log the failure for debugging
        tracing::warn!(
            "Failed to store key {:?} to peer {:?}: {}",
            key, peer_id, error
        );
    }
    
    pub fn get_replica_count(&self, key: &Key) -> usize {
        self.replica_locations
            .get(key)
            .map(|peers| peers.len())
            .unwrap_or(0)
    }
    
    pub fn get_replica_peers(&self, key: &Key) -> Option<&HashSet<PeerId>> {
        self.replica_locations.get(key)
    }
    
    pub fn get_peer_health(&self, peer_id: &PeerId) -> Option<&PeerHealthInfo> {
        self.peer_health.get(peer_id)
    }
    
    /// Get peers with good health scores for replication
    pub fn get_healthy_peers(&self, min_success_rate: f64) -> Vec<PeerId> {
        self.peer_health
            .iter()
            .filter(|(_, health)| health.success_rate >= min_success_rate)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }
}
```

### 4.5 Replication Result Types

```rust
#[derive(Debug, Clone)]
pub struct ReplicationResult {
    pub key: Key,
    pub successful_replicas: usize,
    pub failed_replicas: usize,
    pub target_replicas: usize,
    pub successful_peers: Vec<PeerId>,
    pub failed_peers: Vec<(PeerId, Box<dyn std::error::Error + Send + Sync>)>,
    pub is_sufficient: bool,
}

impl ReplicationResult {
    /// Check if replication meets minimum requirements
    pub fn is_healthy(&self, min_replicas: usize) -> bool {
        self.successful_replicas >= min_replicas
    }
    
    /// Calculate replication success rate
    pub fn success_rate(&self) -> f64 {
        if self.target_replicas == 0 {
            0.0
        } else {
            self.successful_replicas as f64 / self.target_replicas as f64
        }
    }
    
    /// Get a summary of the replication attempt
    pub fn summary(&self) -> String {
        format!(
            "Replication: {}/{} successful ({}% success rate), {} failed",
            self.successful_replicas,
            self.target_replicas,
            (self.success_rate() * 100.0) as u32,
            self.failed_replicas
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("Insufficient peers available: required {required}, available {available}")]
    InsufficientPeers { required: usize, available: usize },
    
    #[error("No peers available for replication")]
    NoPeersAvailable,
    
    #[error("Serialization failed: {0}")]
    SerializationFailed(#[from] SerializationError),
    
    #[error("Network error during replication: {0}")]
    NetworkError(String),
    
    #[error("DHT operation failed: {0}")]
    DhtError(String),
    
    #[error("Geographic information unavailable")]
    GeographicInfoUnavailable,
    
    #[error("Timeout during replication operation")]
    Timeout,
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Repair operation failed: {0}")]
    RepairFailed(String),
}

/// Geographic information for peer distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerGeographicInfo {
    pub peer_id: PeerId,
    pub region: String,           // Geographic region (e.g., "us-east", "eu-west")
    pub country_code: String,     // ISO country code
    pub latitude: Option<f64>,    // Approximate coordinates
    pub longitude: Option<f64>,
    pub network_provider: Option<String>, // ISP or cloud provider
    pub estimated_rtt: Option<Duration>,  // Round-trip time estimate
}
```

### 4.6 Repair Scheduler

```rust
#[derive(Debug)]
pub struct RepairScheduler {
    repair_queue: BinaryHeap<PrioritizedRepairTask>,
    active_repairs: HashMap<Key, RepairTask>,
    repair_history: VecDeque<CompletedRepair>,
    max_history: usize,
}

#[derive(Debug, Clone)]
struct PrioritizedRepairTask {
    task: RepairTask,
    priority_score: u64,
}

#[derive(Debug, Clone)]
struct CompletedRepair {
    key: Key,
    started_at: SystemTime,
    completed_at: SystemTime,
    success: bool,
    replicas_added: usize,
    error: Option<String>,
}

impl Ord for PrioritizedRepairTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority_score.cmp(&other.priority_score)
    }
}

impl PartialOrd for PrioritizedRepairTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PrioritizedRepairTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl Eq for PrioritizedRepairTask {}

impl RepairScheduler {
    pub fn new() -> Self {
        Self {
            repair_queue: BinaryHeap::new(),
            active_repairs: HashMap::new(),
            repair_history: VecDeque::new(),
            max_history: 1000,
        }
    }
    
    pub fn schedule_repair(&mut self, key: &Key, replication_result: &ReplicationResult) {
        // Don't schedule if already active
        if self.active_repairs.contains_key(key) {
            return;
        }
        
        let priority = self.calculate_repair_priority(replication_result);
        let priority_score = self.priority_to_score(&priority);
        
        let repair_task = RepairTask {
            key: key.clone(),
            current_replicas: replication_result.successful_peers.clone(),
            required_replicas: replication_result.target_replicas - replication_result.successful_replicas,
            priority,
            scheduled_at: SystemTime::now(),
        };
        
        let prioritized_task = PrioritizedRepairTask {
            task: repair_task,
            priority_score,
        };
        
        self.repair_queue.push(prioritized_task);
    }
    
    pub fn next_repair(&mut self) -> Option<RepairTask> {
        while let Some(prioritized_task) = self.repair_queue.pop() {
            let task = prioritized_task.task;
            
            // Check if this repair is still needed
            if !self.active_repairs.contains_key(&task.key) {
                self.active_repairs.insert(task.key.clone(), task.clone());
                return Some(task);
            }
        }
        None
    }
    
    pub fn complete_repair(&mut self, key: &Key, success: bool, replicas_added: usize, error: Option<String>) {
        if let Some(task) = self.active_repairs.remove(key) {
            let completed = CompletedRepair {
                key: key.clone(),
                started_at: task.scheduled_at,
                completed_at: SystemTime::now(),
                success,
                replicas_added,
                error,
            };
            
            self.repair_history.push_back(completed);
            
            // Maintain history size limit
            while self.repair_history.len() > self.max_history {
                self.repair_history.pop_front();
            }
        }
    }
    
    fn calculate_repair_priority(&self, replication_result: &ReplicationResult) -> RepairPriority {
        let replica_ratio = replication_result.successful_replicas as f64 / replication_result.target_replicas as f64;
        
        match replica_ratio {
            r if r < 0.25 => RepairPriority::Critical,
            r if r < 0.5 => RepairPriority::High,
            r if r < 0.75 => RepairPriority::Medium,
            _ => RepairPriority::Low,
        }
    }
    
    fn priority_to_score(&self, priority: &RepairPriority) -> u64 {
        match priority {
            RepairPriority::Critical => 1000,
            RepairPriority::High => 750,
            RepairPriority::Medium => 500,
            RepairPriority::Low => 250,
        }
    }
    
    pub fn pending_repairs(&self) -> usize {
        self.repair_queue.len()
    }
    
    pub fn active_repairs(&self) -> usize {
        self.active_repairs.len()
    }
    
    pub fn repair_history(&self) -> &VecDeque<CompletedRepair> {
        &self.repair_history
    }
}
```

---

## 5. Storage Manager Integration

The storage manager provides a unified interface for handling DHT operations with enhanced encryption, serialization, and replication capabilities. It integrates all the components we've built into a cohesive system.

### 5.1 Enhanced Storage Manager

```rust
use std::time::{Duration, SystemTime};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main storage manager that coordinates DHT operations
#[derive(Debug)]
pub struct EnhancedDhtStorageManager {
    /// K=8 replication manager
    replication_manager: Arc<RwLock<EnhancedRecordManager>>,
    /// Encryption service for data protection
    encryption_service: Arc<EncryptionService>,
    /// Serialization service for data formatting
    serialization_service: Arc<SerializationService>,
    /// Local storage for caching and metadata
    local_storage: Arc<RwLock<LocalStorageManager>>,
    /// Configuration for the storage system
    config: StorageManagerConfig,
    /// Event publisher for notifications
    event_publisher: Arc<EventPublisher>,
}

#[derive(Debug, Clone)]
pub struct StorageManagerConfig {
    /// K=8 replication configuration
    pub replication_config: ReplicationConfig,
    /// Local cache configuration
    pub cache_config: CacheConfig,
    /// Encryption preferences
    pub encryption_config: EncryptionConfig,
    /// Serialization preferences
    pub serialization_config: SerializationConfig,
    /// Performance tuning
    pub performance_config: PerformanceConfig,
}

impl Default for StorageManagerConfig {
    fn default() -> Self {
        Self {
            replication_config: ReplicationConfig::default(),
            cache_config: CacheConfig::default(),
            encryption_config: EncryptionConfig::default(),
            serialization_config: SerializationConfig::default(),
            performance_config: PerformanceConfig::default(),
        }
    }
}
```

### 5.2 High-Level Storage Operations

```rust
impl EnhancedDhtStorageManager {
    /// Create a new storage manager with the given configuration
    pub async fn new(config: StorageManagerConfig) -> Result<Self, StorageError> {
        let replication_manager = Arc::new(RwLock::new(
            EnhancedRecordManager::new(config.replication_config.clone())
        ));
        
        let encryption_service = Arc::new(
            EncryptionService::new(config.encryption_config.clone())?
        );
        
        let serialization_service = Arc::new(
            SerializationService::new(config.serialization_config.clone())
        );
        
        let local_storage = Arc::new(RwLock::new(
            LocalStorageManager::new(config.cache_config.clone()).await?
        ));
        
        let event_publisher = Arc::new(EventPublisher::new());
        
        Ok(Self {
            replication_manager,
            encryption_service,
            serialization_service,
            local_storage,
            config,
            event_publisher,
        })
    }
    
    /// Store data with specified access level and encryption
    pub async fn store<T>(&self, request: StoreRequest<T>) -> Result<StoreResponse, StorageError>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let operation_id = OperationId::new();
        self.publish_event(StorageEvent::OperationStarted {
            operation_id: operation_id.clone(),
            operation_type: OperationType::Store,
            key: request.key.clone(),
        }).await;
        
        // Step 1: Validate the request
        self.validate_store_request(&request)?;
        
        // Step 2: Serialize the data with the specified format
        let serialized_data = self.serialization_service
            .serialize(&request.data, request.serialization_format)
            .await?;
        
        // Step 3: Apply encryption based on access level
        let encrypted_data = self.encryption_service
            .encrypt(serialized_data, &request.access_level)
            .await?;
        
        // Step 4: Create enhanced DHT record
        let dht_record = self.create_dht_record(
            request.key.clone(),
            encrypted_data,
            request.metadata.clone(),
            request.access_level.clone(),
        ).await?;
        
        // Step 5: Store locally first for immediate availability
        if request.cache_locally {
            self.store_locally(&dht_record).await?;
        }
        
        // Step 6: Replicate across the network with K=8
        let replication_result = {
            let mut manager = self.replication_manager.write().await;
            manager.store_with_replication(dht_record.clone()).await?
        };
        
        // Step 7: Create response
        let response = StoreResponse {
            operation_id: operation_id.clone(),
            key: request.key.clone(),
            replication_result,
            storage_metadata: StorageMetadata {
                stored_at: SystemTime::now(),
                size_bytes: dht_record.serialized_size(),
                access_level: request.access_level,
                serialization_format: request.serialization_format,
                local_cached: request.cache_locally,
            },
        };
        
        // Step 8: Publish completion event
        self.publish_event(StorageEvent::OperationCompleted {
            operation_id,
            success: replication_result.is_sufficient,
            details: response.clone().into(),
        }).await;
        
        Ok(response)
    }
    
    /// Retrieve data with automatic decryption and deserialization
    pub async fn retrieve<T>(&self, request: RetrieveRequest) -> Result<RetrieveResponse<T>, StorageError>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let operation_id = OperationId::new();
        self.publish_event(StorageEvent::OperationStarted {
            operation_id: operation_id.clone(),
            operation_type: OperationType::Retrieve,
            key: request.key.clone(),
        }).await;
        
        // Step 1: Try local storage first if enabled
        if request.prefer_local {
            if let Some(local_record) = self.retrieve_locally(&request.key).await? {
                let response = self.process_retrieved_record(
                    local_record,
                    request.access_credentials.clone(),
                    operation_id.clone(),
                ).await?;
                
                self.publish_event(StorageEvent::OperationCompleted {
                    operation_id,
                    success: true,
                    details: response.clone().into(),
                }).await;
                
                return Ok(response);
            }
        }
        
        // Step 2: Retrieve from DHT network
        let replica_info = {
            let manager = self.replication_manager.read().await;
            manager.get_replica_info(&request.key)
        };
        
        let dht_record = if let Some(info) = replica_info {
            // We know where replicas are, try them in order of health
            self.retrieve_from_replicas(&request.key, &info).await?
        } else {
            // Fallback to general DHT lookup
            self.retrieve_from_dht(&request.key).await?
        };
        
        // Step 3: Process the retrieved record
        let response = self.process_retrieved_record(
            dht_record,
            request.access_credentials,
            operation_id.clone(),
        ).await?;
        
        // Step 4: Cache locally if requested
        if request.cache_locally && response.record.is_some() {
            self.store_locally(response.record.as_ref().unwrap()).await.ok();
        }
        
        // Step 5: Publish completion event
        self.publish_event(StorageEvent::OperationCompleted {
            operation_id,
            success: response.record.is_some(),
            details: response.clone().into(),
        }).await;
        
        Ok(response)
    }
    
    /// Update existing data with conflict resolution
    pub async fn update<T>(&self, request: UpdateRequest<T>) -> Result<UpdateResponse, StorageError>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let operation_id = OperationId::new();
        self.publish_event(StorageEvent::OperationStarted {
            operation_id: operation_id.clone(),
            operation_type: OperationType::Update,
            key: request.key.clone(),
        }).await;
        
        // Step 1: Retrieve current version
        let current_record = self.retrieve_from_dht(&request.key).await?;
        
        // Step 2: Apply conflict resolution strategy
        let resolved_data = match request.conflict_resolution {
            ConflictResolution::LastWriterWins => {
                // Simply use the new data
                request.new_data
            },
            ConflictResolution::MergeStrategy(merger) => {
                // Decrypt and deserialize current data for merging
                let current_data = self.decrypt_and_deserialize::<T>(
                    &current_record,
                    &request.access_credentials,
                ).await?;
                
                // Apply custom merge function
                merger.merge(current_data, request.new_data)?
            },
            ConflictResolution::VersionVector => {
                // Use version vector for conflict resolution
                self.resolve_with_version_vector(
                    &current_record,
                    request.new_data,
                    &request.version_vector,
                ).await?
            },
        };
        
        // Step 3: Store the resolved data
        let store_request = StoreRequest {
            key: request.key.clone(),
            data: resolved_data,
            access_level: request.access_level,
            metadata: request.metadata,
            serialization_format: request.serialization_format,
            cache_locally: request.cache_locally,
            ttl: request.ttl,
        };
        
        let store_response = self.store(store_request).await?;
        
        // Step 4: Create update response
        let response = UpdateResponse {
            operation_id: operation_id.clone(),
            key: request.key,
            old_version: current_record.version_vector.clone(),
            new_version: store_response.storage_metadata,
            conflict_detected: matches!(request.conflict_resolution, ConflictResolution::MergeStrategy(_)),
            replication_result: store_response.replication_result,
        };
        
        // Step 5: Publish completion event
        self.publish_event(StorageEvent::OperationCompleted {
            operation_id,
            success: response.replication_result.is_sufficient,
            details: response.clone().into(),
        }).await;
        
        Ok(response)
    }
    
    /// Delete data from the DHT with proper cleanup
    pub async fn delete(&self, request: DeleteRequest) -> Result<DeleteResponse, StorageError> {
        let operation_id = OperationId::new();
        self.publish_event(StorageEvent::OperationStarted {
            operation_id: operation_id.clone(),
            operation_type: OperationType::Delete,
            key: request.key.clone(),
        }).await;
        
        // Step 1: Verify deletion permissions
        self.verify_delete_permissions(&request).await?;
        
        // Step 2: Create tombstone record
        let tombstone = self.create_tombstone_record(&request).await?;
        
        // Step 3: Store tombstone across replicas
        let replication_result = {
            let mut manager = self.replication_manager.write().await;
            manager.store_with_replication(tombstone.clone()).await?
        };
        
        // Step 4: Remove from local storage
        if request.remove_local {
            self.remove_locally(&request.key).await?;
        }
        
        // Step 5: Schedule cleanup for later
        self.schedule_cleanup(&request.key, request.cleanup_delay).await?;
        
        let response = DeleteResponse {
            operation_id: operation_id.clone(),
            key: request.key,
            deleted_at: SystemTime::now(),
            replication_result,
            cleanup_scheduled: request.cleanup_delay.is_some(),
        };
        
        // Step 6: Publish completion event
        self.publish_event(StorageEvent::OperationCompleted {
            operation_id,
            success: replication_result.is_sufficient,
            details: response.clone().into(),
        }).await;
        
        Ok(response)
    }
}
```

### 5.3 Request and Response Types

```rust
#[derive(Debug, Clone)]
pub struct StoreRequest<T> {
    pub key: Key,
    pub data: T,
    pub access_level: DataAccessLevel,
    pub metadata: HashMap<String, String>,
    pub serialization_format: SerializationFormat,
    pub cache_locally: bool,
    pub ttl: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct StoreResponse {
    pub operation_id: OperationId,
    pub key: Key,
    pub replication_result: ReplicationResult,
    pub storage_metadata: StorageMetadata,
}

#[derive(Debug, Clone)]
pub struct RetrieveRequest {
    pub key: Key,
    pub access_credentials: AccessCredentials,
    pub prefer_local: bool,
    pub cache_locally: bool,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct RetrieveResponse<T> {
    pub operation_id: OperationId,
    pub key: Key,
    pub record: Option<T>,
    pub metadata: Option<StorageMetadata>,
    pub retrieved_from: RetrievalSource,
    pub retrieval_stats: RetrievalStatistics,
}

#[derive(Debug, Clone)]
pub struct UpdateRequest<T> {
    pub key: Key,
    pub new_data: T,
    pub access_level: DataAccessLevel,
    pub access_credentials: AccessCredentials,
    pub conflict_resolution: ConflictResolution<T>,
    pub version_vector: Option<VersionVector>,
    pub metadata: HashMap<String, String>,
    pub serialization_format: SerializationFormat,
    pub cache_locally: bool,
    pub ttl: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct UpdateResponse {
    pub operation_id: OperationId,
    pub key: Key,
    pub old_version: VersionVector,
    pub new_version: StorageMetadata,
    pub conflict_detected: bool,
    pub replication_result: ReplicationResult,
}

#[derive(Debug, Clone)]
pub struct DeleteRequest {
    pub key: Key,
    pub access_credentials: AccessCredentials,
    pub reason: String,
    pub remove_local: bool,
    pub cleanup_delay: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct DeleteResponse {
    pub operation_id: OperationId,
    pub key: Key,
    pub deleted_at: SystemTime,
    pub replication_result: ReplicationResult,
    pub cleanup_scheduled: bool,
}
```

### 5.4 Supporting Types and Enums

```rust
#[derive(Debug, Clone)]
pub struct AccessCredentials {
    pub user_id: String,
    pub access_tokens: Vec<AccessToken>,
    pub group_memberships: Vec<GroupMembership>,
    pub capability_tokens: Vec<CapabilityToken>,
}

#[derive(Debug, Clone)]
pub enum ConflictResolution<T> {
    LastWriterWins,
    MergeStrategy(Box<dyn MergeStrategy<T>>),
    VersionVector,
}

#[derive(Debug, Clone)]
pub enum RetrievalSource {
    LocalCache,
    DhtNetwork { replica_count: usize },
    BootstrapNode,
}

#[derive(Debug, Clone)]
pub struct RetrievalStatistics {
    pub total_time: Duration,
    pub network_time: Option<Duration>,
    pub decryption_time: Duration,
    pub deserialization_time: Duration,
    pub replicas_contacted: usize,
    pub replicas_responded: usize,
}

#[derive(Debug, Clone)]
pub struct StorageMetadata {
    pub stored_at: SystemTime,
    pub size_bytes: usize,
    pub access_level: DataAccessLevel,
    pub serialization_format: SerializationFormat,
    pub local_cached: bool,
}

#[derive(Debug, Clone)]
pub enum OperationType {
    Store,
    Retrieve,
    Update,
    Delete,
}

#[derive(Debug, Clone)]
pub struct OperationId(String);

impl OperationId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
```

### 5.5 Event System for Monitoring

```rust
#[derive(Debug, Clone)]
pub enum StorageEvent {
    OperationStarted {
        operation_id: OperationId,
        operation_type: OperationType,
        key: Key,
    },
    OperationCompleted {
        operation_id: OperationId,
        success: bool,
        details: EventDetails,
    },
    ReplicationEvent {
        key: Key,
        event_type: ReplicationEventType,
        peer_id: PeerId,
    },
    EncryptionEvent {
        operation_id: OperationId,
        event_type: EncryptionEventType,
    },
    ErrorEvent {
        operation_id: OperationId,
        error: StorageError,
        context: String,
    },
}

#[derive(Debug, Clone)]
pub enum ReplicationEventType {
    ReplicaCreated,
    ReplicaVerified,
    ReplicaFailed,
    RepairScheduled,
    RepairCompleted,
}

#[derive(Debug, Clone)]
pub enum EncryptionEventType {
    EncryptionStarted,
    EncryptionCompleted,
    DecryptionStarted,
    DecryptionCompleted,
    KeyRotationStarted,
    KeyRotationCompleted,
}

#[derive(Debug)]
pub struct EventPublisher {
    subscribers: Arc<RwLock<Vec<Box<dyn EventSubscriber>>>>,
}

#[async_trait::async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn handle_event(&self, event: StorageEvent);
}

impl EventPublisher {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn subscribe(&self, subscriber: Box<dyn EventSubscriber>) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(subscriber);
    }
    
    pub async fn publish(&self, event: StorageEvent) {
        let subscribers = self.subscribers.read().await;
        for subscriber in subscribers.iter() {
            subscriber.handle_event(event.clone()).await;
        }
    }
}
```

### 5.6 Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializationError),
    
    #[error("Encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),
    
    #[error("Replication error: {0}")]
    ReplicationError(#[from] ReplicationError),
    
    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },
    
    #[error("Key not found: {key:?}")]
    KeyNotFound { key: Key },
    
    #[error("Conflict resolution failed: {reason}")]
    ConflictResolutionFailed { reason: String },
    
    #[error("Network timeout after {timeout:?}")]
    NetworkTimeout { timeout: Duration },
    
    #[error("Local storage error: {0}")]
    LocalStorageError(String),
    
    #[error("Validation error: {field}: {reason}")]
    ValidationError { field: String, reason: String },
    
    #[error("System error: {0}")]
    SystemError(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
```

---