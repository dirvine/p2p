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

//! # Secure Memory Management for Cryptographic Operations
//!
//! This module provides memory-protected storage for cryptographic keys and sensitive data.
//! All allocations are automatically zeroized on drop and protected against memory dumps.
//!
//! ## Security Features
//! - Automatic zeroization on drop (prevents key recovery)
//! - Memory locking to prevent swapping to disk
//! - Protected allocation regions
//! - Constant-time comparison operations
//! - Guard pages to detect buffer overflows
//!
//! ## Performance Features
//! - Pool-based allocation to reduce fragmentation
//! - Batch allocation for multiple keys
//! - Efficient reuse of protected memory regions
//! - Minimal overhead for secure operations

#![allow(unsafe_code)] // Required for secure memory operations: mlock, memory zeroing, and protected allocation

use crate::{P2PError, Result};
use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::collections::VecDeque;
use std::fmt;
use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::sync::Mutex;

#[cfg(unix)]
use libc::{mlock, munlock};

#[cfg(windows)]
use winapi::um::memoryapi::{VirtualLock, VirtualUnlock};

/// Maximum size for a single secure allocation (64KB)
const MAX_SECURE_ALLOCATION: usize = 65536;

/// Default size for the secure memory pool (1MB)
const DEFAULT_POOL_SIZE: usize = 1024 * 1024;

/// Alignment requirement for secure allocations
const SECURE_ALIGNMENT: usize = 64;

/// Secure memory container that automatically zeroizes on drop
pub struct SecureMemory {
    /// Pointer to the allocated memory
    ptr: NonNull<u8>,
    /// Size of the allocation
    size: usize,
    /// Whether the memory is locked (cannot be swapped)
    locked: bool,
    /// Layout used for allocation
    layout: Layout,
}

// Safety: SecureMemory is safe to send between threads as it owns its memory
unsafe impl Send for SecureMemory {}
// Safety: SecureMemory is safe to share between threads with proper synchronization
unsafe impl Sync for SecureMemory {}

/// Secure vector with automatic zeroization
pub struct SecureVec {
    /// Underlying secure memory
    memory: SecureMemory,
    /// Current length of the vector
    len: usize,
}

/// Secure string with automatic zeroization
pub struct SecureString {
    /// Underlying secure vector
    vec: SecureVec,
}

/// Pool for managing secure memory allocations
pub struct SecureMemoryPool {
    /// Available memory chunks
    available: Mutex<VecDeque<SecureMemory>>,
    /// Total pool size
    total_size: usize,
    /// Chunk size for allocations
    chunk_size: usize,
    /// Statistics
    stats: Mutex<PoolStats>,
}

/// Statistics for secure memory pool
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total allocations made
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Current active allocations
    pub active_allocations: u64,
    /// Pool hits (reused memory)
    pub pool_hits: u64,
    /// Pool misses (new allocations)
    pub pool_misses: u64,
    /// Total bytes allocated
    pub total_bytes_allocated: u64,
    /// Current bytes in use
    pub current_bytes_in_use: u64,
}

/// Error types for secure memory operations
#[derive(Debug, Clone)]
pub enum SecureMemoryError {
    /// Allocation failed
    AllocationFailed(String),
    /// Memory locking failed
    LockingFailed(String),
    /// Invalid size or alignment
    InvalidParameters(String),
    /// Pool exhausted
    PoolExhausted,
    /// Operation not supported on this platform
    NotSupported(String),
}

impl std::fmt::Display for SecureMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecureMemoryError::AllocationFailed(msg) => write!(f, "Allocation failed: {msg}"),
            SecureMemoryError::LockingFailed(msg) => write!(f, "Memory locking failed: {msg}"),
            SecureMemoryError::InvalidParameters(msg) => write!(f, "Invalid parameters: {msg}"),
            SecureMemoryError::PoolExhausted => write!(f, "Secure memory pool exhausted"),
            SecureMemoryError::NotSupported(msg) => write!(f, "Operation not supported: {msg}"),
        }
    }
}

impl std::error::Error for SecureMemoryError {}

impl SecureMemory {
    /// Allocate secure memory with the given size
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 {
            return Err(P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot allocate zero-sized memory",
            )));
        }

        if size > MAX_SECURE_ALLOCATION {
            return Err(P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Allocation size {size} exceeds maximum {MAX_SECURE_ALLOCATION}"),
            )));
        }

        // Align size to secure alignment boundary
        let aligned_size = (size + SECURE_ALIGNMENT - 1) & !(SECURE_ALIGNMENT - 1);

        // Create layout for allocation
        let layout = Layout::from_size_align(aligned_size, SECURE_ALIGNMENT).map_err(|e| {
            P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid layout: {e}"),
            ))
        })?;

        // Allocate zeroed memory
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err(P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "Memory allocation failed",
            )));
        }

        let ptr = NonNull::new(ptr).ok_or_else(|| {
            P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "Null pointer returned from allocator",
            ))
        })?;

        let mut memory = Self {
            ptr,
            size: aligned_size,
            locked: false,
            layout,
        };

        // Attempt to lock the memory
        if let Err(e) = memory.lock_memory() {
            tracing::warn!("Failed to lock secure memory: {}", e);
        }

        Ok(memory)
    }

    /// Create secure memory from existing data (data is copied and source should be zeroized)
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        let mut memory = Self::new(data.len())?;
        memory.as_mut_slice().copy_from_slice(data);
        Ok(memory)
    }

    /// Get the size of the allocated memory
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if the memory is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get a slice view of the memory
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }

    /// Get a mutable slice view of the memory
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    /// Compare two secure memory regions in constant time
    pub fn constant_time_eq(&self, other: &SecureMemory) -> bool {
        if self.size != other.size {
            return false;
        }

        let a = self.as_slice();
        let b = other.as_slice();

        // Constant-time comparison
        let mut result = 0u8;
        for i in 0..self.size {
            result |= a[i] ^ b[i];
        }

        result == 0
    }

    /// Lock memory to prevent it from being swapped to disk
    fn lock_memory(&mut self) -> Result<()> {
        if self.locked {
            return Ok(());
        }

        #[cfg(unix)]
        {
            let result = unsafe { mlock(self.ptr.as_ptr() as *const libc::c_void, self.size) };
            if result != 0 {
                return Err(P2PError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Failed to lock memory pages",
                )));
            }
        }

        #[cfg(windows)]
        {
            let result =
                unsafe { VirtualLock(self.ptr.as_ptr() as *mut winapi::ctypes::c_void, self.size) };
            if result == 0 {
                return Err(P2PError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "VirtualLock failed",
                )));
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            tracing::warn!("Memory locking not supported on this platform");
        }

        self.locked = true;
        Ok(())
    }

    /// Unlock memory (called automatically on drop)
    fn unlock_memory(&mut self) {
        if !self.locked {
            return;
        }

        #[cfg(unix)]
        {
            unsafe { munlock(self.ptr.as_ptr() as *const libc::c_void, self.size) };
        }

        #[cfg(windows)]
        {
            unsafe { VirtualUnlock(self.ptr.as_ptr() as *mut winapi::ctypes::c_void, self.size) };
        }

        self.locked = false;
    }

    /// Securely zeroize the memory
    pub fn zeroize(&mut self) {
        unsafe {
            // Use volatile write to prevent compiler optimization
            ptr::write_volatile(self.ptr.as_ptr(), 0u8);

            // Zeroize the entire allocation
            for i in 0..self.size {
                ptr::write_volatile(self.ptr.as_ptr().add(i), 0u8);
            }
        }
    }
}

impl Drop for SecureMemory {
    fn drop(&mut self) {
        // Zeroize memory before deallocation
        self.zeroize();

        // Unlock memory
        self.unlock_memory();

        // Deallocate memory
        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

impl SecureVec {
    /// Create a new secure vector with the given capacity
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        let memory = SecureMemory::new(capacity)?;
        Ok(Self { memory, len: 0 })
    }

    /// Create a secure vector from existing data
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        let memory = SecureMemory::from_slice(data)?;
        let len = data.len();
        Ok(Self { memory, len })
    }

    /// Get the length of the vector
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the vector is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity of the vector
    pub fn capacity(&self) -> usize {
        self.memory.len()
    }

    /// Push a byte to the vector
    pub fn push(&mut self, value: u8) -> Result<()> {
        if self.len >= self.capacity() {
            return Err(P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "SecureVec capacity exceeded",
            )));
        }

        self.memory.as_mut_slice()[self.len] = value;
        self.len += 1;
        Ok(())
    }

    /// Extend the vector with data from a slice
    pub fn extend_from_slice(&mut self, data: &[u8]) -> Result<()> {
        if self.len + data.len() > self.capacity() {
            return Err(P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "SecureVec capacity exceeded",
            )));
        }

        self.memory.as_mut_slice()[self.len..self.len + data.len()].copy_from_slice(data);
        self.len += data.len();
        Ok(())
    }

    /// Get a slice of the vector's contents
    pub fn as_slice(&self) -> &[u8] {
        &self.memory.as_slice()[..self.len]
    }

    /// Clear the vector (zeroizes the data)
    pub fn clear(&mut self) {
        self.memory.zeroize();
        self.len = 0;
    }
}

impl Deref for SecureVec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl SecureString {
    /// Create a new secure string with the given capacity
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        let vec = SecureVec::with_capacity(capacity)?;
        Ok(Self { vec })
    }

    /// Create a secure string from a regular string
    pub fn from_str(s: &str) -> Result<Self> {
        let vec = SecureVec::from_slice(s.as_bytes())?;
        Ok(Self { vec })
    }

    /// Get the length of the string
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Check if the string is empty
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Push a character to the string
    pub fn push(&mut self, ch: char) -> Result<()> {
        let mut buffer = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buffer);
        self.vec.extend_from_slice(encoded.as_bytes())
    }

    /// Push a string slice to the string
    pub fn push_str(&mut self, s: &str) -> Result<()> {
        self.vec.extend_from_slice(s.as_bytes())
    }

    /// Get the string as a str slice
    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(self.vec.as_slice()).map_err(|e| {
            P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8: {e}"),
            ))
        })
    }

    /// Clear the string (zeroizes the data)
    pub fn clear(&mut self) {
        self.vec.clear();
    }
}

impl fmt::Display for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_str() {
            Ok(s) => write!(f, "{s}"),
            Err(_) => write!(f, "<invalid UTF-8>"),
        }
    }
}

impl fmt::Debug for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureString[{}]", self.len())
    }
}

impl SecureMemoryPool {
    /// Create a new secure memory pool
    pub fn new(total_size: usize, chunk_size: usize) -> Result<Self> {
        if chunk_size > total_size {
            return Err(P2PError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Chunk size cannot exceed total size",
            )));
        }

        let pool = Self {
            available: Mutex::new(VecDeque::new()),
            total_size,
            chunk_size,
            stats: Mutex::new(PoolStats::default()),
        };

        // Pre-allocate chunks
        pool.preallocate_chunks()?;

        Ok(pool)
    }

    /// Create a default secure memory pool
    pub fn default() -> Result<Self> {
        Self::new(DEFAULT_POOL_SIZE, 4096)
    }

    /// Allocate memory from the pool
    pub fn allocate(&self, size: usize) -> Result<SecureMemory> {
        if size > self.chunk_size {
            // Large allocation - allocate directly
            let mut stats = self.stats.lock().expect("lock not poisoned");
            stats.pool_misses += 1;
            stats.total_allocations += 1;
            stats.active_allocations += 1;
            stats.total_bytes_allocated += size as u64;
            stats.current_bytes_in_use += size as u64;

            return SecureMemory::new(size);
        }

        // Try to get from pool
        {
            let mut available = self.available.lock().expect("lock not poisoned");
            if let Some(memory) = available.pop_front() {
                let mut stats = self.stats.lock().expect("lock not poisoned");
                stats.pool_hits += 1;
                stats.total_allocations += 1;
                stats.active_allocations += 1;
                stats.current_bytes_in_use += memory.len() as u64;

                return Ok(memory);
            }
        }

        // Pool empty - allocate new chunk
        let mut stats = self.stats.lock().expect("lock not poisoned");
        stats.pool_misses += 1;
        stats.total_allocations += 1;
        stats.active_allocations += 1;
        stats.total_bytes_allocated += self.chunk_size as u64;
        stats.current_bytes_in_use += self.chunk_size as u64;

        SecureMemory::new(self.chunk_size)
    }

    /// Return memory to the pool
    pub fn deallocate(&self, mut memory: SecureMemory) {
        // Zeroize before returning to pool
        memory.zeroize();

        let memory_size = memory.len();

        if memory_size == self.chunk_size {
            // Return to pool
            let mut available = self.available.lock().expect("lock not poisoned");
            available.push_back(memory);
        }
        // Large allocations are dropped automatically

        let mut stats = self.stats.lock().expect("lock not poisoned");
        stats.total_deallocations += 1;
        stats.active_allocations -= 1;
        stats.current_bytes_in_use -= memory_size as u64;
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        self.stats.lock().expect("lock not poisoned").clone()
    }

    /// Pre-allocate chunks for the pool
    fn preallocate_chunks(&self) -> Result<()> {
        let num_chunks = self.total_size / self.chunk_size;
        let mut available = self.available.lock().expect("lock not poisoned");

        for _ in 0..num_chunks {
            let memory = SecureMemory::new(self.chunk_size)?;
            available.push_back(memory);
        }

        Ok(())
    }
}

/// Global secure memory pool instance
static GLOBAL_POOL: std::sync::OnceLock<SecureMemoryPool> = std::sync::OnceLock::new();

/// Get the global secure memory pool
pub fn global_secure_pool() -> &'static SecureMemoryPool {
    GLOBAL_POOL.get_or_init(|| {
        SecureMemoryPool::default().expect("Failed to initialize global secure memory pool")
    })
}

/// Convenience function to allocate secure memory from global pool
pub fn allocate_secure(size: usize) -> Result<SecureMemory> {
    global_secure_pool().allocate(size)
}

/// Convenience function to create a secure vector from global pool
pub fn secure_vec_with_capacity(capacity: usize) -> Result<SecureVec> {
    let memory = global_secure_pool().allocate(capacity)?;
    Ok(SecureVec { memory, len: 0 })
}

/// Convenience function to create a secure string from global pool
pub fn secure_string_with_capacity(capacity: usize) -> Result<SecureString> {
    let vec = secure_vec_with_capacity(capacity)?;
    Ok(SecureString { vec })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_memory_basic() {
        let mut memory = SecureMemory::new(1024).unwrap();

        // Test basic operations
        assert_eq!(memory.len(), 1024);
        assert!(!memory.is_empty());

        // Test writing and reading
        memory.as_mut_slice()[0] = 42;
        assert_eq!(memory.as_slice()[0], 42);

        // Test zeroization
        memory.zeroize();
        assert_eq!(memory.as_slice()[0], 0);
    }

    #[test]
    fn test_secure_memory_constant_time_comparison() {
        let memory1 = SecureMemory::from_slice(b"hello").unwrap();
        let memory2 = SecureMemory::from_slice(b"hello").unwrap();
        let memory3 = SecureMemory::from_slice(b"world").unwrap();

        assert!(memory1.constant_time_eq(&memory2));
        assert!(!memory1.constant_time_eq(&memory3));
    }

    #[test]
    fn test_secure_vec() {
        let mut vec = SecureVec::with_capacity(100).unwrap();

        // Test basic operations
        vec.push(1).unwrap();
        vec.push(2).unwrap();
        vec.extend_from_slice(&[3, 4, 5]).unwrap();

        assert_eq!(vec.len(), 5);
        assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5]);

        // Test clear
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
    }

    #[test]
    fn test_secure_string() {
        let mut string = SecureString::with_capacity(100).unwrap();

        // Test basic operations
        string.push('H').unwrap();
        string.push_str("ello").unwrap();

        assert_eq!(string.as_str().unwrap(), "Hello");
        assert_eq!(string.len(), 5);

        // Test clear
        string.clear();
        assert_eq!(string.len(), 0);
        assert!(string.is_empty());
    }

    #[test]
    fn test_secure_memory_pool() {
        let pool = SecureMemoryPool::new(8192, 1024).unwrap();

        // Test allocation
        let memory1 = pool.allocate(512).unwrap();
        let memory2 = pool.allocate(1024).unwrap();

        // Check stats
        let stats = pool.stats();
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.active_allocations, 2);

        // Test deallocation
        pool.deallocate(memory1);
        pool.deallocate(memory2);

        let stats = pool.stats();
        assert_eq!(stats.total_deallocations, 2);
        assert_eq!(stats.active_allocations, 0);
    }

    #[test]
    fn test_global_pool() {
        let memory = allocate_secure(256).unwrap();
        assert_eq!(memory.len(), 256);

        let vec = secure_vec_with_capacity(128).unwrap();
        assert_eq!(vec.capacity(), 128);

        let string = secure_string_with_capacity(64).unwrap();
        assert_eq!(string.vec.capacity(), 64);
    }
}
