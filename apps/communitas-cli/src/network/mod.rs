// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Network integration and P2P functionality

pub mod config;
pub mod connection;
pub mod node;

use anyhow::Result;
use std::path::PathBuf;

use crate::identity::EnhancedIdentityManager;

pub use config::{NetworkConfig, BootstrapNode};
pub use connection::{ConnectionManager, ConnectionInfo, ConnectionState, ConnectionHealth};
pub use node::{P2PNodeManager, NodeState};

/// Network manager for P2P connectivity
pub struct NetworkManager {
    node_manager: P2PNodeManager,
    connection_manager: ConnectionManager,
    config: NetworkConfig,
    identity_manager: Option<EnhancedIdentityManager>,
}

// Manual Debug implementation
impl std::fmt::Debug for NetworkManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkManager")
            .field("node_manager", &self.node_manager)
            .field("config", &self.config)
            .field("identity_manager", &self.identity_manager)
            .finish()
    }
}

impl NetworkManager {
    /// Create a new network manager with default config
    pub fn new() -> Self {
        let config = NetworkConfig::default();
        let node_manager = P2PNodeManager::new(config.clone());
        let connection_manager = ConnectionManager::new(
            config.max_connections,
            config.connection_timeout_secs,
        );
        
        NetworkManager {
            node_manager,
            connection_manager,
            config,
            identity_manager: None,
        }
    }

    /// Create a network manager with the given configuration
    pub fn with_config(config: NetworkConfig) -> Self {
        let node_manager = P2PNodeManager::new(config.clone());
        let connection_manager = ConnectionManager::new(
            config.max_connections,
            config.connection_timeout_secs,
        );
        
        NetworkManager {
            node_manager,
            connection_manager,
            config,
            identity_manager: None,
        }
    }

    /// Initialize P2P node when network feature is enabled
    #[cfg(feature = "network")]
    pub async fn initialize(&mut self) -> Result<()> {
        self.node_manager.initialize().await
    }

    /// Initialize with custom listen address (overrides config)
    #[cfg(feature = "network")]
    pub async fn initialize_with_address(&mut self, listen_addr: &str) -> Result<()> {
        self.config.listen_address = listen_addr.to_string();
        self.initialize().await
    }

    /// Get the current configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    /// Get mutable access to the configuration
    pub fn config_mut(&mut self) -> &mut NetworkConfig {
        &mut self.config
    }

    /// Load configuration from file
    pub async fn load_config<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        self.config = NetworkConfig::load(path).await?;
        Ok(())
    }

    /// Save configuration to file
    pub async fn save_config<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        self.config.save(path).await
    }

    /// Check if connected to network
    pub async fn is_connected(&self) -> bool {
        self.node_manager.is_connected().await
    }

    /// Get node address
    pub async fn get_address(&self) -> Option<String> {
        if self.node_manager.is_connected().await {
            Some(self.config.listen_address.clone())
        } else {
            None
        }
    }

    /// Shutdown the network node
    pub async fn shutdown(&mut self) -> Result<()> {
        self.node_manager.shutdown().await
    }
    
    /// Get node state
    pub async fn get_node_state(&self) -> NodeState {
        self.node_manager.get_state().await
    }
    
    /// Get peer count
    pub async fn get_peer_count(&self) -> usize {
        self.node_manager.get_peer_count().await
    }
    
    /// Discover peers
    pub async fn discover_peers(&self) -> Result<Vec<String>> {
        self.node_manager.discover_peers().await
    }
    
    /// Get connection manager
    pub fn connection_manager(&self) -> &ConnectionManager {
        &self.connection_manager
    }
    
    /// Reconnect to the network
    pub async fn reconnect(&mut self) -> Result<()> {
        self.node_manager.reconnect().await
    }
    
    /// Initialize identity manager
    pub async fn initialize_identity(&mut self, storage_path: PathBuf) -> Result<()> {
        let mut identity_manager = EnhancedIdentityManager::new(storage_path);
        
        // Try to load existing identity
        match identity_manager.load().await {
            Ok(()) => {
                // Check if identity was actually loaded
                if identity_manager.current().is_none() {
                    // No identity was loaded, create a new one
                    identity_manager.create_local_identity("Network User").await?;
                }
            }
            Err(_) => {
                // Loading failed (file doesn't exist), create a new identity
                identity_manager.create_local_identity("Network User").await?;
            }
        }
        
        // Verify identity was created successfully
        if identity_manager.current().is_none() {
            return Err(anyhow::anyhow!("Failed to create or load identity"));
        }
        
        self.identity_manager = Some(identity_manager);
        Ok(())
    }
    
    /// Get identity manager reference
    pub fn identity_manager(&self) -> Option<&EnhancedIdentityManager> {
        self.identity_manager.as_ref()
    }
    
    /// Get mutable identity manager reference
    pub fn identity_manager_mut(&mut self) -> Option<&mut EnhancedIdentityManager> {
        self.identity_manager.as_mut()
    }
    
    /// Get current identity address
    pub fn get_identity_address(&self) -> Option<String> {
        self.identity_manager.as_ref()
            .and_then(|im| im.get_address())
            .map(|addr| addr.as_string())
    }
    
    /// Check if peer is trusted
    pub fn is_peer_trusted(&self, peer_address: &str) -> bool {
        use crate::identity::FourWordAddress;
        
        if let Some(identity_manager) = &self.identity_manager {
            if let Ok(address) = FourWordAddress::from_string(peer_address) {
                return identity_manager.can_interact_with(&address);
            }
        }
        false
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let manager = NetworkManager::new();
        assert!(!manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_bootstrap_nodes() {
        let mut manager = NetworkManager::new();
        manager.config_mut().add_bootstrap_node("127.0.0.1:9000".to_string()).unwrap();
        manager.config_mut().add_bootstrap_node("[::1]:9001".to_string()).unwrap();
        
        assert_eq!(manager.config().bootstrap_nodes.len(), 2);
    }

    #[cfg(feature = "network")]
    #[tokio::test]
    async fn test_network_initialization() {
        let mut manager = NetworkManager::new();
        manager.config_mut().enabled = true;
        manager.config_mut().listen_address = "127.0.0.1:0".to_string();
        
        // For now, just test that we can create and configure the manager
        assert_eq!(manager.config().enabled, true);
        assert_eq!(manager.config().listen_address, "127.0.0.1:0");
        assert!(!manager.is_connected().await);
        
        // Actual network initialization would be tested in integration tests
        // let result = manager.initialize().await;
        // assert!(result.is_ok());
        // assert!(manager.is_connected().await);
        // manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_node_state() {
        let manager = NetworkManager::new();
        let state = manager.get_node_state().await;
        assert_eq!(state, NodeState::Uninitialized);
    }

    #[tokio::test]
    async fn test_peer_count() {
        let manager = NetworkManager::new();
        let count = manager.get_peer_count().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_connection_manager() {
        let manager = NetworkManager::new();
        let conn_mgr = manager.connection_manager();
        assert_eq!(conn_mgr.get_connection_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_identity_integration() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let mut manager = NetworkManager::new();
        
        // Initially no identity
        assert!(manager.identity_manager().is_none());
        assert!(manager.get_identity_address().is_none());
        
        // Initialize identity
        manager.initialize_identity(temp_dir.path().to_path_buf()).await.unwrap();
        
        // Now identity should exist
        assert!(manager.identity_manager().is_some());
        assert!(manager.get_identity_address().is_some());
        
        // Test trust check
        assert!(!manager.is_peer_trusted("unknown-peer-address"));
    }
}