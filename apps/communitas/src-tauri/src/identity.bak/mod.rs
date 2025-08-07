//! Identity management module for Communitas
//!
//! This module provides secure cross-platform identity management with:
//! - Platform-specific secure storage (Keychain, Credential Manager, Secret Service)
//! - Encrypted file storage as fallback
//! - Integration with saorsa-core identity system
//! - 4-word address generation and management

pub mod secure_storage;
pub mod encrypted_file_storage;
pub mod macos_keychain;
pub mod windows_credential_manager;
pub mod linux_secret_service;
pub mod identity_manager;

// Re-export main types
pub use secure_storage::{
    SecureStorage, 
    SecureStorageFactory, 
    KeyEntry, 
    KeyMetadata, 
    StorageInfo
};

pub use identity_manager::CommunidentityManager;
