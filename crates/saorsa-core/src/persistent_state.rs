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

//! # Persistent State Management with Crash Recovery
//!
//! This module provides durable state management with crash recovery capabilities,
//! ensuring data integrity and consistency across system restarts.
//!
//! ## Features
//! - Write-Ahead Logging (WAL) for durability
//! - Atomic state transitions with rollback capability
//! - State snapshots for faster recovery
//! - Corruption detection and recovery
//! - Multi-version storage with configurable retention
//!
//! ## Architecture
//! ```
//! State Changes → WAL → Apply → Snapshot → Cleanup
//!                  ↓                ↑
//!             Recovery ←────────────┘
//! ```

use crate::error::{SecurityError, StorageError};
use crate::secure_memory::SecureMemory;
use crate::{P2PError, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::task::JoinHandle;

/// WAL entry version for forward compatibility
const WAL_VERSION: u8 = 1;

/// Maximum WAL file size before rotation (10MB)
const MAX_WAL_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum number of WAL entries before checkpoint
const MAX_WAL_ENTRIES: usize = 1000;

/// Number of old snapshots to retain
const SNAPSHOT_RETENTION_COUNT: usize = 3;

/// Lock file name for crash detection
const LOCK_FILE_NAME: &str = ".state.lock";

/// WAL file extension
const WAL_EXTENSION: &str = "wal";

/// Snapshot file extension
const SNAPSHOT_EXTENSION: &str = "snap";

/// State file permissions (owner read/write only)
#[cfg(unix)]
#[allow(dead_code)]
const STATE_FILE_PERMISSIONS: u32 = 0o600;

/// Transaction type for WAL entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Insert or update operation
    Upsert,
    /// Delete operation
    Delete,
    /// Batch operation
    Batch,
    /// Checkpoint marker
    Checkpoint,
}

/// WAL entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Entry version
    pub version: u8,
    /// Unique transaction ID
    pub transaction_id: u64,
    /// Transaction timestamp
    pub timestamp: u64,
    /// Type of transaction
    pub transaction_type: TransactionType,
    /// Key affected by transaction
    pub key: String,
    /// Serialized value (None for deletes)
    pub value: Option<Vec<u8>>,
    /// HMAC for integrity verification
    pub hmac: [u8; 32],
}

/// State snapshot header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHeader {
    /// Snapshot version
    pub version: u8,
    /// Creation timestamp
    pub created_at: u64,
    /// Last transaction ID included
    pub last_transaction_id: u64,
    /// Number of entries
    pub entry_count: u64,
    /// Total size in bytes
    pub total_size: u64,
    /// Checksum of snapshot data
    pub checksum: [u8; 32],
}

/// Recovery mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    /// Fast recovery with minimal validation
    Fast,
    /// Standard recovery with consistency checks
    Standard,
    /// Full recovery with complete validation
    Full,
    /// Repair mode for corrupted states
    Repair,
}

/// State manager configuration
#[derive(Debug, Clone)]
pub struct StateConfig {
    /// Base directory for state files
    pub state_dir: PathBuf,
    /// WAL flush strategy
    pub flush_strategy: FlushStrategy,
    /// Checkpoint interval
    pub checkpoint_interval: Duration,
    /// Enable compression
    pub enable_compression: bool,
    /// Recovery mode
    pub recovery_mode: RecoveryMode,
    /// Maximum state size in bytes
    pub max_state_size: u64,
}

/// WAL flush strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushStrategy {
    /// Flush on every write (slowest, most durable)
    Always,
    /// Flush periodically
    Periodic(Duration),
    /// Flush when buffer reaches size
    BufferSize(usize),
    /// Adaptive based on load
    Adaptive,
}

/// Persistent state manager
pub struct PersistentStateManager<T: Serialize + for<'de> Deserialize<'de> + Clone + PartialEq> {
    /// Configuration
    config: StateConfig,
    /// Current state
    state: Arc<RwLock<HashMap<String, T>>>,
    /// WAL writer
    wal_writer: Arc<Mutex<WalWriter>>,
    /// Transaction counter
    transaction_counter: Arc<Mutex<u64>>,
    /// Checkpoint task handle
    checkpoint_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Recovery statistics
    recovery_stats: Arc<Mutex<RecoveryStats>>,
    /// State change listeners
    listeners: Arc<RwLock<Vec<Box<dyn Fn(&str, Option<&T>) + Send + Sync>>>>,
    /// HMAC key for integrity
    hmac_key: SecureMemory,
}

/// WAL writer implementation
struct WalWriter {
    /// Current WAL file
    file: File,
    /// WAL file path
    path: PathBuf,
    /// Current file size
    current_size: u64,
    /// Entry count in current file
    entry_count: usize,
    /// Flush strategy
    flush_strategy: FlushStrategy,
    /// Last flush time
    last_flush: Instant,
    /// Pending entries
    pending_entries: Vec<WalEntry>,
}

/// Recovery statistics
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    /// Recovery start time
    pub start_time: Option<Instant>,
    /// Recovery end time
    pub end_time: Option<Instant>,
    /// Entries recovered
    pub entries_recovered: u64,
    /// Entries failed
    pub entries_failed: u64,
    /// Snapshots processed
    pub snapshots_processed: u64,
    /// WAL files processed
    pub wal_files_processed: u64,
    /// Data loss detected
    pub data_loss_detected: bool,
    /// Corruption events
    pub corruption_events: Vec<CorruptionEvent>,
}

/// Corruption event details
#[derive(Debug, Clone)]
pub struct CorruptionEvent {
    /// File where corruption detected
    pub file_path: PathBuf,
    /// Type of corruption
    pub corruption_type: CorruptionType,
    /// Offset in file
    pub offset: u64,
    /// Recovery action taken
    pub recovery_action: RecoveryAction,
}

/// Types of corruption
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorruptionType {
    /// Invalid checksum
    ChecksumMismatch,
    /// Incomplete write
    IncompleteWrite,
    /// Invalid format
    InvalidFormat,
    /// Missing data
    MissingData,
}

/// Recovery actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Entry skipped
    Skipped,
    /// Entry repaired
    Repaired,
    /// Rolled back to snapshot
    RolledBack,
    /// Manual intervention required
    ManualRequired,
}

/// State change event
#[derive(Debug, Clone)]
pub struct StateChangeEvent<T> {
    /// Transaction ID
    pub transaction_id: u64,
    /// Affected key
    pub key: String,
    /// Old value
    pub old_value: Option<T>,
    /// New value
    pub new_value: Option<T>,
    /// Change timestamp
    pub timestamp: u64,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            state_dir: PathBuf::from("./state"),
            flush_strategy: FlushStrategy::Adaptive,
            checkpoint_interval: Duration::from_secs(300), // 5 minutes
            enable_compression: true,
            recovery_mode: RecoveryMode::Standard,
            max_state_size: 1024 * 1024 * 1024, // 1GB
        }
    }
}

impl WalWriter {
    /// Create new WAL writer
    fn new(wal_path: PathBuf, flush_strategy: FlushStrategy) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to open WAL file: {e}").into(),
                ))
            })?;

        let metadata = file.metadata().map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to get WAL metadata: {e}").into(),
            ))
        })?;

        Ok(Self {
            file,
            path: wal_path,
            current_size: metadata.len(),
            entry_count: 0,
            flush_strategy,
            last_flush: Instant::now(),
            pending_entries: Vec::new(),
        })
    }

    /// Write entry to WAL
    fn write_entry(&mut self, entry: &WalEntry) -> Result<()> {
        let serialized = bincode::serialize(entry).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to serialize WAL entry: {e}").into(),
            ))
        })?;

        // Write entry size first (for recovery)
        let size_bytes = (serialized.len() as u32).to_le_bytes();
        self.file.write_all(&size_bytes).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to write entry size: {e}").into(),
            ))
        })?;

        // Write entry data
        self.file.write_all(&serialized).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to write WAL entry: {e}").into(),
            ))
        })?;

        self.current_size += 4 + serialized.len() as u64;
        self.entry_count += 1;

        // Handle flush strategy
        match self.flush_strategy {
            FlushStrategy::Always => {
                self.file.flush().map_err(|e| {
                    P2PError::Storage(StorageError::Database(
                        format!("Failed to flush WAL: {e}").into(),
                    ))
                })?;
            }
            FlushStrategy::Periodic(duration) => {
                if self.last_flush.elapsed() >= duration {
                    self.file.flush().map_err(|e| {
                        P2PError::Storage(StorageError::Database(
                            format!("Failed to flush WAL: {e}").into(),
                        ))
                    })?;
                    self.last_flush = Instant::now();
                }
            }
            FlushStrategy::BufferSize(size) => {
                if self.pending_entries.len() >= size {
                    self.file.flush().map_err(|e| {
                        P2PError::Storage(StorageError::Database(
                            format!("Failed to flush WAL: {e}").into(),
                        ))
                    })?;
                    self.pending_entries.clear();
                }
            }
            FlushStrategy::Adaptive => {
                // Adaptive strategy: flush based on time, size, and entry count
                let should_flush = self.last_flush.elapsed() >= Duration::from_secs(1)
                    || self.pending_entries.len() >= 100
                    || self.current_size >= MAX_WAL_SIZE / 10;

                if should_flush {
                    self.file.flush().map_err(|e| {
                        P2PError::Storage(StorageError::Database(
                            format!("Failed to flush WAL: {e}").into(),
                        ))
                    })?;
                    self.last_flush = Instant::now();
                    self.pending_entries.clear();
                }
            }
        }

        Ok(())
    }

    /// Check if rotation needed
    fn needs_rotation(&self) -> bool {
        self.current_size >= MAX_WAL_SIZE || self.entry_count >= MAX_WAL_ENTRIES
    }

    /// Rotate WAL file
    fn rotate(&mut self) -> Result<()> {
        // Close current file
        self.file.sync_all().map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to sync WAL: {e}").into(),
            ))
        })?;

        // Rename to timestamped file
        let timestamp = current_timestamp();
        let rotated_path = self
            .path
            .with_file_name(format!("wal.{timestamp}.{WAL_EXTENSION}"));
        std::fs::rename(&self.path, &rotated_path).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to rotate WAL: {e}").into(),
            ))
        })?;

        // Create new WAL file
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to create new WAL: {e}").into(),
                ))
            })?;

        self.current_size = 0;
        self.entry_count = 0;

        Ok(())
    }
}

impl<T: Serialize + for<'de> Deserialize<'de> + Clone + PartialEq + Send + Sync + 'static>
    PersistentStateManager<T>
{
    /// Create new persistent state manager
    pub async fn new(config: StateConfig) -> Result<Self> {
        // Ensure state directory exists
        std::fs::create_dir_all(&config.state_dir).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to create state directory: {e}").into(),
            ))
        })?;

        // Generate HMAC key
        let mut hmac_key_bytes = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut hmac_key_bytes);
        let hmac_key = SecureMemory::from_slice(&hmac_key_bytes)?;

        // Create WAL writer
        let wal_path = config.state_dir.join(format!("state.{WAL_EXTENSION}"));
        let wal_writer = Arc::new(Mutex::new(WalWriter::new(wal_path, config.flush_strategy)?));

        // Create state manager
        let manager = Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(HashMap::new())),
            wal_writer,
            transaction_counter: Arc::new(Mutex::new(0)),
            checkpoint_task: Arc::new(Mutex::new(None)),
            recovery_stats: Arc::new(Mutex::new(RecoveryStats::default())),
            listeners: Arc::new(RwLock::new(Vec::new())),
            hmac_key,
        };

        // Perform recovery
        manager.recover().await?;

        // Start checkpoint task
        manager.start_checkpoint_task()?;

        Ok(manager)
    }

    /// Insert or update state entry
    pub async fn upsert(&self, key: String, value: T) -> Result<Option<T>> {
        // Generate transaction ID
        let transaction_id = {
            let mut counter = self.transaction_counter.lock().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "mutex lock failed".to_string().into(),
                ))
            })?;
            *counter += 1;
            *counter
        };

        // Serialize value
        let serialized_value = bincode::serialize(&value).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to serialize value: {e}").into(),
            ))
        })?;

        // Create WAL entry
        let wal_entry = self.create_wal_entry(
            transaction_id,
            TransactionType::Upsert,
            key.clone(),
            Some(serialized_value),
        )?;

        // Write to WAL first
        {
            let mut writer = self.wal_writer.lock().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "mutex lock failed".to_string().into(),
                ))
            })?;
            writer.write_entry(&wal_entry)?;

            if writer.needs_rotation() {
                writer.rotate()?;
            }
        }

        // Update in-memory state
        let old_value = {
            let mut state = self.state.write().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "write lock failed".to_string().into(),
                ))
            })?;
            state.insert(key.clone(), value.clone())
        };

        // Notify listeners
        self.notify_listeners(&key, Some(&value)).await;

        Ok(old_value)
    }

    /// Delete state entry
    pub async fn delete(&self, key: &str) -> Result<Option<T>> {
        // Generate transaction ID
        let transaction_id = {
            let mut counter = self.transaction_counter.lock().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "mutex lock failed".to_string().into(),
                ))
            })?;
            *counter += 1;
            *counter
        };

        // Create WAL entry
        let wal_entry = self.create_wal_entry(
            transaction_id,
            TransactionType::Delete,
            key.to_string(),
            None,
        )?;

        // Write to WAL first
        {
            let mut writer = self.wal_writer.lock().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "mutex lock failed".to_string().into(),
                ))
            })?;
            writer.write_entry(&wal_entry)?;

            if writer.needs_rotation() {
                writer.rotate()?;
            }
        }

        // Update in-memory state
        let old_value = {
            let mut state = self.state.write().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "write lock failed".to_string().into(),
                ))
            })?;
            state.remove(key)
        };

        // Notify listeners
        self.notify_listeners(key, None).await;

        Ok(old_value)
    }

    /// Get state entry
    pub fn get(&self, key: &str) -> Result<Option<T>> {
        let state = self.state.read().map_err(|_| {
            P2PError::Storage(StorageError::LockPoisoned(
                "read lock failed".to_string().into(),
            ))
        })?;
        Ok(state.get(key).cloned())
    }

    /// Get all state entries
    pub fn get_all(&self) -> Result<HashMap<String, T>> {
        let state = self.state.read().map_err(|_| {
            P2PError::Storage(StorageError::LockPoisoned(
                "read lock failed".to_string().into(),
            ))
        })?;
        Ok(state.clone())
    }

    /// Perform batch update
    pub async fn batch_update<F>(&self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut HashMap<String, T>) -> Result<()>,
    {
        // Generate transaction ID
        let transaction_id = {
            let mut counter = self.transaction_counter.lock().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "mutex lock failed".to_string().into(),
                ))
            })?;
            *counter += 1;
            *counter
        };

        // Clone current state for rollback
        let backup_state = {
            let state = self.state.read().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "read lock failed".to_string().into(),
                ))
            })?;
            state.clone()
        };

        // Apply updates
        let changes = {
            let mut state = self.state.write().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "write lock failed".to_string().into(),
                ))
            })?;
            let initial_state = state.clone();

            // Apply update function
            match update_fn(&mut state) {
                Ok(()) => {
                    // Calculate changes
                    let mut changes = Vec::new();

                    // Find updates and inserts
                    for (key, value) in state.iter() {
                        if !initial_state.contains_key(key) || initial_state[key] != *value {
                            changes.push((key.clone(), Some(value.clone())));
                        }
                    }

                    // Find deletes
                    for key in initial_state.keys() {
                        if !state.contains_key(key) {
                            changes.push((key.clone(), None));
                        }
                    }

                    changes
                }
                Err(e) => {
                    // Rollback on error
                    *state = backup_state;
                    return Err(e);
                }
            }
        };

        // Write batch to WAL
        for (key, value) in changes {
            let serialized_value = value
                .as_ref()
                .map(|v| bincode::serialize(v))
                .transpose()
                .map_err(|e| {
                    P2PError::Storage(StorageError::Database(
                        format!("Failed to serialize value: {e}").into(),
                    ))
                })?;

            let wal_entry = self.create_wal_entry(
                transaction_id,
                TransactionType::Batch,
                key.clone(),
                serialized_value,
            )?;

            {
                let mut writer = self.wal_writer.lock().map_err(|_| {
                    P2PError::Storage(StorageError::LockPoisoned(
                        "mutex lock failed".to_string().into(),
                    ))
                })?;
                writer.write_entry(&wal_entry)?;
            }

            // Notify listeners
            self.notify_listeners(&key, value.as_ref()).await;
        }

        Ok(())
    }

    /// Create checkpoint (snapshot)
    pub async fn checkpoint(&self) -> Result<()> {
        let snapshot_path = self.generate_snapshot_path();
        let temp_path = snapshot_path.with_extension("tmp");

        // Get current state and transaction ID
        let (current_state, last_transaction_id) = {
            let state = self.state.read().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "read lock failed".to_string().into(),
                ))
            })?;
            let counter = self.transaction_counter.lock().map_err(|_| {
                P2PError::Storage(StorageError::LockPoisoned(
                    "mutex lock failed".to_string().into(),
                ))
            })?;
            (state.clone(), *counter)
        };

        // Create snapshot
        let snapshot_data = bincode::serialize(&current_state).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to serialize snapshot: {e}").into(),
            ))
        })?;

        // Calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(&snapshot_data);
        let checksum: [u8; 32] = hasher.finalize().into();

        // Create snapshot header
        let header = SnapshotHeader {
            version: WAL_VERSION,
            created_at: current_timestamp(),
            last_transaction_id,
            entry_count: current_state.len() as u64,
            total_size: snapshot_data.len() as u64,
            checksum,
        };

        // Write snapshot to temp file
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)
                .map_err(|e| {
                    P2PError::Storage(StorageError::Database(
                        format!("Failed to create snapshot file: {e}").into(),
                    ))
                })?;

            // Write header
            let header_data = bincode::serialize(&header).map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to serialize header: {e}").into(),
                ))
            })?;
            let header_size = (header_data.len() as u32).to_le_bytes();
            file.write_all(&header_size)?;
            file.write_all(&header_data)?;

            // Write snapshot data
            file.write_all(&snapshot_data)?;

            file.sync_all().map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to sync snapshot: {e}").into(),
                ))
            })?;
        }

        // Atomic rename
        std::fs::rename(&temp_path, &snapshot_path).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to rename snapshot: {e}").into(),
            ))
        })?;

        // Clean up old WAL files
        self.cleanup_old_wal_files(last_transaction_id).await?;

        // Clean up old snapshots
        self.cleanup_old_snapshots().await?;

        Ok(())
    }

    /// Recover state from persistent storage
    async fn recover(&self) -> Result<()> {
        let mut stats = RecoveryStats {
            start_time: Some(Instant::now()),
            ..Default::default()
        };

        // Check for lock file (crash detection)
        let lock_path = self.config.state_dir.join(LOCK_FILE_NAME);
        let crashed = lock_path.exists();

        if crashed {
            tracing::error!("Detected unclean shutdown, performing recovery...");
        }

        // Create lock file
        File::create(&lock_path).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to create lock file: {e}").into(),
            ))
        })?;

        // Find latest snapshot
        let _snapshot_result = self.recover_from_snapshot(&mut stats).await;

        // Recover from WAL files
        self.recover_from_wal(&mut stats).await?;

        // Remove lock file
        std::fs::remove_file(&lock_path).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to remove lock file: {e}").into(),
            ))
        })?;

        stats.end_time = Some(Instant::now());

        // Store recovery stats
        *self.recovery_stats.lock().map_err(|_| {
            P2PError::Storage(StorageError::LockPoisoned(
                "mutex lock failed".to_string().into(),
            ))
        })? = stats;

        Ok(())
    }

    /// Recover from snapshot
    async fn recover_from_snapshot(&self, stats: &mut RecoveryStats) -> Result<()> {
        let snapshots = self.find_snapshots()?;

        for snapshot_path in snapshots.iter().rev() {
            match self.load_snapshot(snapshot_path).await {
                Ok((header, state)) => {
                    // Verify checksum
                    let data = bincode::serialize(&state).map_err(|e| {
                        P2PError::Storage(StorageError::Database(
                            format!("Failed to serialize for checksum: {e}").into(),
                        ))
                    })?;

                    let mut hasher = Sha256::new();
                    hasher.update(&data);
                    let checksum: [u8; 32] = hasher.finalize().into();

                    if checksum != header.checksum {
                        stats.corruption_events.push(CorruptionEvent {
                            file_path: snapshot_path.clone(),
                            corruption_type: CorruptionType::ChecksumMismatch,
                            offset: 0,
                            recovery_action: RecoveryAction::Skipped,
                        });
                        continue;
                    }

                    // Load state
                    {
                        let mut current_state = self.state.write().map_err(|_| {
                            P2PError::Storage(StorageError::LockPoisoned(
                                "write lock failed".to_string().into(),
                            ))
                        })?;
                        *current_state = state;
                    }

                    // Update transaction counter
                    {
                        let mut counter = self.transaction_counter.lock().map_err(|_| {
                            P2PError::Storage(StorageError::LockPoisoned(
                                "mutex lock failed".to_string().into(),
                            ))
                        })?;
                        *counter = header.last_transaction_id;
                    }

                    stats.snapshots_processed += 1;
                    stats.entries_recovered += header.entry_count;

                    return Ok(());
                }
                Err(_) => {
                    stats.corruption_events.push(CorruptionEvent {
                        file_path: snapshot_path.clone(),
                        corruption_type: CorruptionType::InvalidFormat,
                        offset: 0,
                        recovery_action: RecoveryAction::Skipped,
                    });
                }
            }
        }

        Ok(())
    }

    /// Recover from WAL files
    async fn recover_from_wal(&self, stats: &mut RecoveryStats) -> Result<()> {
        let wal_files = self.find_wal_files()?;

        for wal_path in wal_files {
            match self.replay_wal_file(&wal_path, stats).await {
                Ok(entries) => {
                    stats.wal_files_processed += 1;
                    stats.entries_recovered += entries;
                }
                Err(e) => {
                    tracing::error!("Failed to replay WAL file {:?}: {}", wal_path, e);
                    stats.data_loss_detected = true;
                }
            }
        }

        Ok(())
    }

    /// Replay single WAL file
    async fn replay_wal_file(&self, path: &Path, stats: &mut RecoveryStats) -> Result<u64> {
        let mut file = File::open(path).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to open WAL file: {e}").into(),
            ))
        })?;

        let mut entries_recovered = 0u64;
        let mut buffer = Vec::new();

        loop {
            // Read entry size
            let mut size_bytes = [0u8; 4];
            match file.read_exact(&mut size_bytes) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(P2PError::Io(e)),
            }

            let entry_size = u32::from_le_bytes(size_bytes) as usize;

            // Read entry data
            buffer.resize(entry_size, 0);
            match file.read_exact(&mut buffer) {
                Ok(()) => {}
                Err(_e) => {
                    stats.corruption_events.push(CorruptionEvent {
                        file_path: path.to_path_buf(),
                        corruption_type: CorruptionType::IncompleteWrite,
                        offset: file.stream_position().unwrap_or(0),
                        recovery_action: RecoveryAction::Skipped,
                    });
                    stats.entries_failed += 1;
                    continue;
                }
            }

            // Deserialize entry
            let entry: WalEntry = match bincode::deserialize(&buffer) {
                Ok(e) => e,
                Err(_) => {
                    stats.corruption_events.push(CorruptionEvent {
                        file_path: path.to_path_buf(),
                        corruption_type: CorruptionType::InvalidFormat,
                        offset: file.stream_position().unwrap_or(0) - entry_size as u64,
                        recovery_action: RecoveryAction::Skipped,
                    });
                    stats.entries_failed += 1;
                    continue;
                }
            };

            // Verify HMAC
            if !self.verify_wal_entry(&entry) {
                stats.corruption_events.push(CorruptionEvent {
                    file_path: path.to_path_buf(),
                    corruption_type: CorruptionType::ChecksumMismatch,
                    offset: file.stream_position().unwrap_or(0) - entry_size as u64,
                    recovery_action: RecoveryAction::Skipped,
                });
                stats.entries_failed += 1;
                continue;
            }

            // Apply entry to state
            match entry.transaction_type {
                TransactionType::Upsert | TransactionType::Batch => {
                    if let Some(value_data) = entry.value {
                        match bincode::deserialize::<T>(&value_data) {
                            Ok(value) => {
                                let mut state = self.state.write().map_err(|_| {
                                    P2PError::Storage(StorageError::LockPoisoned(
                                        "write lock failed".to_string().into(),
                                    ))
                                })?;
                                state.insert(entry.key, value);
                                entries_recovered += 1;
                            }
                            Err(_) => {
                                stats.entries_failed += 1;
                            }
                        }
                    }
                }
                TransactionType::Delete => {
                    let mut state = self.state.write().map_err(|_| {
                        P2PError::Storage(StorageError::LockPoisoned(
                            "write lock failed".to_string().into(),
                        ))
                    })?;
                    state.remove(&entry.key);
                    entries_recovered += 1;
                }
                TransactionType::Checkpoint => {
                    // Checkpoint marker, no action needed
                }
            }

            // Update transaction counter
            {
                let mut counter = self.transaction_counter.lock().map_err(|_| {
                    P2PError::Storage(StorageError::LockPoisoned(
                        "mutex lock failed".to_string().into(),
                    ))
                })?;
                if entry.transaction_id > *counter {
                    *counter = entry.transaction_id;
                }
            }
        }

        Ok(entries_recovered)
    }

    /// Create WAL entry with HMAC
    fn create_wal_entry(
        &self,
        transaction_id: u64,
        transaction_type: TransactionType,
        key: String,
        value: Option<Vec<u8>>,
    ) -> Result<WalEntry> {
        let mut entry = WalEntry {
            version: WAL_VERSION,
            transaction_id,
            timestamp: current_timestamp(),
            transaction_type,
            key,
            value,
            hmac: [0u8; 32],
        };

        // Calculate HMAC
        entry.hmac = self.calculate_entry_hmac(&entry)?;

        Ok(entry)
    }

    /// Calculate HMAC for WAL entry
    fn calculate_entry_hmac(&self, entry: &WalEntry) -> Result<[u8; 32]> {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.hmac_key.as_slice()).map_err(|e| {
            P2PError::Security(SecurityError::InvalidKey(
                format!("Failed to create HMAC: {e}").into(),
            ))
        })?;

        mac.update(&entry.version.to_le_bytes());
        mac.update(&entry.transaction_id.to_le_bytes());
        mac.update(&entry.timestamp.to_le_bytes());
        mac.update(&[entry.transaction_type as u8]);
        mac.update(entry.key.as_bytes());

        if let Some(ref value) = entry.value {
            mac.update(value);
        }

        Ok(mac.finalize().into_bytes().into())
    }

    /// Verify WAL entry HMAC
    fn verify_wal_entry(&self, entry: &WalEntry) -> bool {
        let mut temp_entry = entry.clone();
        temp_entry.hmac = [0u8; 32];

        match self.calculate_entry_hmac(&temp_entry) {
            Ok(calculated_hmac) => calculated_hmac == entry.hmac,
            Err(_) => false,
        }
    }

    /// Generate snapshot path
    fn generate_snapshot_path(&self) -> PathBuf {
        let timestamp = current_timestamp();
        self.config
            .state_dir
            .join(format!("snapshot.{timestamp}.{SNAPSHOT_EXTENSION}"))
    }

    /// Find all snapshot files
    fn find_snapshots(&self) -> Result<Vec<PathBuf>> {
        let mut snapshots = Vec::new();

        let entries = std::fs::read_dir(&self.config.state_dir).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to read state directory: {e}").into(),
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to read directory entry: {e}").into(),
                ))
            })?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some(SNAPSHOT_EXTENSION) {
                snapshots.push(path);
            }
        }

        // Sort by timestamp (newest first)
        snapshots.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        Ok(snapshots)
    }

    /// Find all WAL files
    fn find_wal_files(&self) -> Result<Vec<PathBuf>> {
        let mut wal_files = Vec::new();

        let entries = std::fs::read_dir(&self.config.state_dir).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to read state directory: {e}").into(),
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to read directory entry: {e}").into(),
                ))
            })?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some(WAL_EXTENSION) {
                wal_files.push(path);
            }
        }

        // Sort by timestamp (oldest first for replay)
        wal_files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        Ok(wal_files)
    }

    /// Load snapshot from file
    async fn load_snapshot(&self, path: &Path) -> Result<(SnapshotHeader, HashMap<String, T>)> {
        let mut file = File::open(path).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to open snapshot: {e}").into(),
            ))
        })?;

        // Read header size
        let mut size_bytes = [0u8; 4];
        file.read_exact(&mut size_bytes).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to read header size: {e}").into(),
            ))
        })?;

        let header_size = u32::from_le_bytes(size_bytes) as usize;

        // Read header
        let mut header_data = vec![0u8; header_size];
        file.read_exact(&mut header_data).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to read header: {e}").into(),
            ))
        })?;

        let header: SnapshotHeader = bincode::deserialize(&header_data).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to deserialize header: {e}").into(),
            ))
        })?;

        // Read snapshot data
        let mut snapshot_data = Vec::new();
        file.read_to_end(&mut snapshot_data).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to read snapshot data: {e}").into(),
            ))
        })?;

        // Deserialize state
        let state: HashMap<String, T> = bincode::deserialize(&snapshot_data).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to deserialize snapshot: {e}").into(),
            ))
        })?;

        Ok((header, state))
    }

    /// Clean up old WAL files
    async fn cleanup_old_wal_files(&self, last_checkpoint_id: u64) -> Result<()> {
        let wal_files = self.find_wal_files()?;

        for wal_path in wal_files {
            // Skip current WAL file
            if wal_path.file_name() == Some(std::ffi::OsStr::new(&format!("state.{WAL_EXTENSION}")))
            {
                continue;
            }

            // Check if all entries are before checkpoint
            let can_delete = match self.check_wal_file_transactions(&wal_path).await {
                Ok(max_transaction_id) => max_transaction_id <= last_checkpoint_id,
                Err(_) => false,
            };

            if can_delete {
                std::fs::remove_file(&wal_path).map_err(|e| {
                    P2PError::Storage(StorageError::Database(
                        format!("Failed to remove old WAL: {e}").into(),
                    ))
                })?;
            }
        }

        Ok(())
    }

    /// Check maximum transaction ID in WAL file
    async fn check_wal_file_transactions(&self, path: &Path) -> Result<u64> {
        let mut file = File::open(path).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to open WAL file: {e}").into(),
            ))
        })?;

        let mut max_transaction_id = 0u64;
        let mut buffer = Vec::new();

        loop {
            // Read entry size
            let mut size_bytes = [0u8; 4];
            match file.read_exact(&mut size_bytes) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(P2PError::Io(e)),
            }

            let entry_size = u32::from_le_bytes(size_bytes) as usize;

            // Read entry data
            buffer.resize(entry_size, 0);
            file.read_exact(&mut buffer).map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to read entry: {e}").into(),
                ))
            })?;

            // Deserialize entry
            if let Ok(entry) = bincode::deserialize::<WalEntry>(&buffer) {
                max_transaction_id = max_transaction_id.max(entry.transaction_id);
            }
        }

        Ok(max_transaction_id)
    }

    /// Clean up old snapshots
    async fn cleanup_old_snapshots(&self) -> Result<()> {
        let snapshots = self.find_snapshots()?;

        // Keep only the most recent snapshots
        if snapshots.len() > SNAPSHOT_RETENTION_COUNT {
            for snapshot_path in &snapshots[SNAPSHOT_RETENTION_COUNT..] {
                std::fs::remove_file(snapshot_path).map_err(|e| {
                    P2PError::Storage(StorageError::Database(
                        format!("Failed to remove old snapshot: {e}").into(),
                    ))
                })?;
            }
        }

        Ok(())
    }

    /// Start background checkpoint task
    fn start_checkpoint_task(&self) -> Result<()> {
        let _state = Arc::clone(&self.state);
        let _wal_writer = Arc::clone(&self.wal_writer);
        let _transaction_counter = Arc::clone(&self.transaction_counter);
        let config = self.config.clone();

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.checkpoint_interval);

            loop {
                interval.tick().await;

                // Note: We can't actually checkpoint from here without the full manager
                // This would need to be redesigned to work without cloning the manager
                tracing::debug!("Checkpoint interval reached");
            }
        });

        let mut checkpoint_task = self.checkpoint_task.lock().map_err(|_| {
            P2PError::Storage(StorageError::LockPoisoned(
                "mutex lock failed".to_string().into(),
            ))
        })?;
        *checkpoint_task = Some(task);
        Ok(())
    }

    /// Notify state change listeners
    async fn notify_listeners(&self, key: &str, value: Option<&T>) {
        let listeners = match self.listeners.read() {
            Ok(guard) => guard,
            Err(_) => {
                tracing::error!("Failed to acquire read lock for listeners");
                return;
            }
        };
        for listener in listeners.iter() {
            listener(key, value);
        }
    }

    /// Register state change listener
    pub fn add_listener<F>(&self, listener: F) -> Result<()>
    where
        F: Fn(&str, Option<&T>) + Send + Sync + 'static,
    {
        let mut listeners = self.listeners.write().map_err(|_| {
            P2PError::Storage(StorageError::LockPoisoned(
                "write lock failed".to_string().into(),
            ))
        })?;
        listeners.push(Box::new(listener));
        Ok(())
    }

    /// Get recovery statistics
    pub fn recovery_stats(&self) -> Result<RecoveryStats> {
        let stats = self.recovery_stats.lock().map_err(|_| {
            P2PError::Storage(StorageError::LockPoisoned(
                "mutex lock failed".to_string().into(),
            ))
        })?;
        Ok(stats.clone())
    }

    /// Perform integrity check
    pub async fn verify_integrity(&self) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::default();

        // Check snapshots
        let snapshots = self.find_snapshots()?;
        for snapshot_path in snapshots {
            match self.verify_snapshot_integrity(&snapshot_path).await {
                Ok(()) => report.valid_snapshots += 1,
                Err(_) => report.corrupted_snapshots += 1,
            }
        }

        // Check WAL files
        let wal_files = self.find_wal_files()?;
        for wal_path in wal_files {
            match self.verify_wal_integrity(&wal_path).await {
                Ok(entries) => {
                    report.valid_wal_entries += entries;
                }
                Err(_) => {
                    report.corrupted_wal_files += 1;
                }
            }
        }

        // Calculate state size
        let state = self.state.read().map_err(|_| {
            P2PError::Storage(StorageError::LockPoisoned(
                "read lock failed".to_string().into(),
            ))
        })?;
        report.total_entries = state.len();

        for (key, value) in state.iter() {
            let serialized = bincode::serialize(value).map_err(|e| {
                P2PError::Storage(StorageError::Database(
                    format!("Failed to serialize for size: {e}").into(),
                ))
            })?;
            report.total_size += key.len() + serialized.len();
        }

        Ok(report)
    }

    /// Verify snapshot integrity
    async fn verify_snapshot_integrity(&self, path: &Path) -> Result<()> {
        let (header, state) = self.load_snapshot(path).await?;

        // Verify checksum
        let data = bincode::serialize(&state).map_err(|e| {
            P2PError::Storage(StorageError::Database(
                format!("Failed to serialize for checksum: {e}").into(),
            ))
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum: [u8; 32] = hasher.finalize().into();

        if checksum != header.checksum {
            return Err(P2PError::Storage(
                crate::error::StorageError::CorruptionDetected(
                    "Snapshot checksum mismatch".to_string().into(),
                ),
            ));
        }

        Ok(())
    }

    /// Verify WAL file integrity
    async fn verify_wal_integrity(&self, path: &Path) -> Result<u64> {
        let stats = &mut RecoveryStats::default();
        self.replay_wal_file(path, stats).await
    }
}

/// Integrity check report
#[derive(Debug, Clone, Default)]
pub struct IntegrityReport {
    /// Valid snapshots
    pub valid_snapshots: usize,
    /// Corrupted snapshots
    pub corrupted_snapshots: usize,
    /// Valid WAL entries
    pub valid_wal_entries: u64,
    /// Corrupted WAL files
    pub corrupted_wal_files: usize,
    /// Total state entries
    pub total_entries: usize,
    /// Total state size in bytes
    pub total_size: usize,
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestState {
        id: u64,
        data: String,
    }

    #[tokio::test]
    async fn test_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = StateConfig {
            state_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = PersistentStateManager::<TestState>::new(config)
            .await
            .unwrap();

        // Insert
        let state = TestState {
            id: 1,
            data: "test".to_string(),
        };
        let old = manager
            .upsert("key1".to_string(), state.clone())
            .await
            .unwrap();
        assert!(old.is_none());

        // Get
        let retrieved = manager.get("key1").unwrap().unwrap();
        assert_eq!(retrieved, state);

        // Update
        let updated = TestState {
            id: 2,
            data: "updated".to_string(),
        };
        let old = manager
            .upsert("key1".to_string(), updated.clone())
            .await
            .unwrap();
        assert_eq!(old.unwrap(), state);

        // Delete
        let deleted = manager.delete("key1").await.unwrap();
        assert_eq!(deleted.unwrap(), updated);

        // Verify deletion
        assert!(manager.get("key1").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_crash_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let config = StateConfig {
            state_dir: temp_dir.path().to_path_buf(),
            flush_strategy: FlushStrategy::Always,
            ..Default::default()
        };

        // Create and populate manager
        {
            let manager = PersistentStateManager::<TestState>::new(config.clone())
                .await
                .unwrap();

            for i in 0..10 {
                let state = TestState {
                    id: i,
                    data: format!("test_{}", i).into(),
                };
                manager
                    .upsert(format!("key_{}", i).into(), state)
                    .await
                    .unwrap();
            }
        }

        // Simulate crash by not calling checkpoint

        // Recover
        let manager = PersistentStateManager::<TestState>::new(config)
            .await
            .unwrap();

        // Verify all data recovered
        for i in 0..10 {
            let state = manager.get(&format!("key_{}", i)).unwrap().unwrap();
            assert_eq!(state.id, i);
            assert_eq!(state.data, format!("test_{}", i));
        }

        let stats = manager.recovery_stats().unwrap();
        assert!(stats.entries_recovered >= 10);
    }

    #[tokio::test]
    async fn test_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let config = StateConfig {
            state_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = PersistentStateManager::<TestState>::new(config)
            .await
            .unwrap();

        // Add data
        for i in 0..5 {
            let state = TestState {
                id: i,
                data: format!("test_{}", i).into(),
            };
            manager
                .upsert(format!("key_{}", i).into(), state)
                .await
                .unwrap();
        }

        // Create checkpoint
        manager.checkpoint().await.unwrap();

        // Verify snapshot exists
        let snapshots = manager.find_snapshots().unwrap();
        assert!(!snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_batch_update() {
        let temp_dir = TempDir::new().unwrap();
        let config = StateConfig {
            state_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = PersistentStateManager::<TestState>::new(config)
            .await
            .unwrap();

        // Batch insert
        manager
            .batch_update(|state| {
                for i in 0..5 {
                    state.insert(
                        format!("key_{}", i).into(),
                        TestState {
                            id: i,
                            data: format!("batch_{}", i).into(),
                        },
                    );
                }
                Ok(())
            })
            .await
            .unwrap();

        // Verify all inserted
        for i in 0..5 {
            let state = manager.get(&format!("key_{}", i)).unwrap().unwrap();
            assert_eq!(state.id, i);
            assert_eq!(state.data, format!("batch_{}", i));
        }
    }

    #[tokio::test]
    async fn test_integrity_verification() {
        let temp_dir = TempDir::new().unwrap();
        let config = StateConfig {
            state_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = PersistentStateManager::<TestState>::new(config)
            .await
            .unwrap();

        // Add data
        for i in 0..10 {
            let state = TestState {
                id: i,
                data: format!("test_{}", i).into(),
            };
            manager
                .upsert(format!("key_{}", i).into(), state)
                .await
                .unwrap();
        }

        // Create checkpoint
        manager.checkpoint().await.unwrap();

        // Verify integrity
        let report = manager.verify_integrity().await.unwrap();
        assert_eq!(report.total_entries, 10);
        assert!(report.valid_snapshots > 0);
        assert_eq!(report.corrupted_snapshots, 0);
    }
}
