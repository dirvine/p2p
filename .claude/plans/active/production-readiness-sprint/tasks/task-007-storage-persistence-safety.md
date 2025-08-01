# Task 007: Storage and Persistence Error Handling

## Overview
Implement comprehensive error handling for all storage operations including encrypted key storage, persistent state, and the monotonic counter. These components are critical for data integrity and must handle failures gracefully.

## Acceptance Criteria
- [ ] Zero panics in storage operations
- [ ] Corruption detection and recovery
- [ ] Atomic operations with rollback
- [ ] Proper error propagation
- [ ] Data integrity maintained under failures

## Technical Details

### 1. Files to Update
- `encrypted_key_storage.rs` - Encrypted key management
- `persistent_state.rs` - Application state persistence
- `monotonic_counter.rs` - Atomic counter operations
- `secure_memory.rs` - Secure memory handling
- `dht/storage.rs` - DHT storage backend

### 2. Encrypted Key Storage Safety

#### Safe Key Operations
```rust
// Before
let encrypted = cipher.encrypt(&key.to_bytes()).unwrap();
fs::write(&key_path, encrypted).unwrap();

// After
pub async fn store_encrypted_key(&self, key: &SecretKey) -> Result<()> {
    // Create temporary file for atomic write
    let temp_path = format!("{}.tmp", self.key_path);
    
    // Encrypt with verification
    let encrypted = self.cipher
        .encrypt(&key.to_bytes())
        .map_err(|e| StorageError::Encryption(e.to_string()))?;
    
    // Write to temporary file
    tokio::fs::write(&temp_path, &encrypted)
        .await
        .map_err(|e| StorageError::Io(e))?;
    
    // Verify write succeeded
    let verification = tokio::fs::read(&temp_path)
        .await
        .map_err(|e| StorageError::Io(e))?;
    
    if verification != encrypted {
        tokio::fs::remove_file(&temp_path).await.ok();
        return Err(StorageError::VerificationFailed);
    }
    
    // Atomic rename
    tokio::fs::rename(&temp_path, &self.key_path)
        .await
        .map_err(|e| StorageError::Io(e))?;
    
    Ok(())
}
```

### 3. Persistent State Management

#### Transactional Updates
```rust
pub struct PersistentState {
    state: Arc<RwLock<State>>,
    wal: WriteAheadLog,
}

impl PersistentState {
    pub async fn update<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut State) -> Result<()>,
    {
        // Record intent in WAL
        let transaction_id = self.wal.begin_transaction().await?;
        
        // Update in memory
        let mut state = self.state.write().await;
        let backup = state.clone();
        
        match updater(&mut state) {
            Ok(()) => {
                // Persist to disk
                match self.persist_state(&state).await {
                    Ok(()) => {
                        self.wal.commit(transaction_id).await?;
                        Ok(())
                    }
                    Err(e) => {
                        // Rollback memory state
                        *state = backup;
                        self.wal.rollback(transaction_id).await?;
                        Err(e)
                    }
                }
            }
            Err(e) => {
                // Rollback
                *state = backup;
                self.wal.rollback(transaction_id).await?;
                Err(e)
            }
        }
    }
}
```

### 4. Monotonic Counter Safety
```rust
// Before
let current = fs::read_to_string(&counter_file).unwrap().parse().unwrap();

// After
impl MonotonicCounter {
    pub async fn increment(&self) -> Result<u64> {
        let mut attempts = 0;
        loop {
            match self.try_increment().await {
                Ok(value) => return Ok(value),
                Err(e) if attempts < 3 => {
                    attempts += 1;
                    log::warn!("Counter increment attempt {} failed: {}", attempts, e);
                    tokio::time::sleep(Duration::from_millis(10 * attempts)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    async fn try_increment(&self) -> Result<u64> {
        // Use file locking for atomicity
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.path)
            .await
            .map_err(|e| StorageError::Io(e))?;
        
        // Exclusive lock
        file.lock_exclusive()
            .map_err(|e| StorageError::LockFailed(e.to_string()))?;
        
        // Read current value
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| StorageError::Io(e))?;
        
        let current = contents.trim()
            .parse::<u64>()
            .unwrap_or(0);
        
        let next = current.checked_add(1)
            .ok_or(StorageError::CounterOverflow)?;
        
        // Write new value
        file.set_len(0).await?;
        file.seek(SeekFrom::Start(0)).await?;
        file.write_all(next.to_string().as_bytes()).await?;
        file.sync_all().await?;
        
        Ok(next)
    }
}
```

### 5. Corruption Detection
- Add checksums to stored data
- Implement backup/restore mechanisms
- Validate data on read
- Add recovery procedures

## Testing Requirements
- Simulate disk failures during writes
- Test with corrupted files
- Concurrent access testing
- Power failure simulation
- Verify data integrity

## Dependencies
- Previous: Task 001 (Error Framework)
- Related: Task 004 (Identity key storage)

## Time Estimate
- Implementation: 10 hours
- Testing: 4 hours
- Total: 14 hours

## Definition of Done
- [ ] All storage operations handle errors
- [ ] Atomic operations implemented
- [ ] Corruption detection working
- [ ] No data loss under failures
- [ ] Performance acceptable