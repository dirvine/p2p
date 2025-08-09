// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// P2P Node implementation and lifecycle management

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "network")]
use saorsa_core::P2PNode;

use super::config::NetworkConfig;

/// Node state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    Uninitialized,
    Initializing,
    Connected,
    Disconnected,
    Failed(String),
}

/// Manages P2P node lifecycle and operations
pub struct P2PNodeManager {
    #[cfg(feature = "network")]
    node: Option<Arc<RwLock<P2PNode>>>,
    config: NetworkConfig,
    state: Arc<RwLock<NodeState>>,
    retry_count: u32,
    max_retries: u32,
}

impl P2PNodeManager {
    /// Create a new P2P node manager
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            #[cfg(feature = "network")]
            node: None,
            config,
            state: Arc::new(RwLock::new(NodeState::Uninitialized)),
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// Initialize the P2P node
    #[cfg(feature = "network")]
    pub async fn initialize(&mut self) -> Result<()> {
        // Update state
        {
            let mut state = self.state.write().await;
            *state = NodeState::Initializing;
        }

        // Validate configuration
        self.config.validate()
            .context("Invalid network configuration")?;

        if !self.config.enabled {
            let mut state = self.state.write().await;
            *state = NodeState::Disconnected;
            return Ok(());
        }

        // Create P2P node with retry logic
        let mut last_error = None;
        
        for attempt in 0..=self.max_retries {
            match self.create_node().await {
                Ok(node) => {
                    self.node = Some(Arc::new(RwLock::new(node)));
                    let mut state = self.state.write().await;
                    *state = NodeState::Connected;
                    self.retry_count = 0;
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1 << attempt)).await;
                    }
                }
            }
        }

        // Failed after all retries
        let mut state = self.state.write().await;
        let error_msg = last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string());
        *state = NodeState::Failed(error_msg.clone());
        anyhow::bail!("Failed to initialize P2P node after {} retries: {}", self.max_retries, error_msg)
    }

    /// Create a P2P node instance
    #[cfg(feature = "network")]
    async fn create_node(&self) -> Result<P2PNode> {
        let mut builder = P2PNode::builder()
            .listen_on(&self.config.listen_address);
        
        // Add bootstrap nodes
        for bootstrap in &self.config.bootstrap_nodes {
            builder = builder.with_bootstrap_peer(bootstrap);
        }

        // Configure DHT if enabled
        if self.config.enable_dht {
            builder = builder.with_default_dht();
        }
        
        builder.build()
            .await
            .context("Failed to create P2P node")
    }

    /// Initialize (non-network mode)
    #[cfg(not(feature = "network"))]
    pub async fn initialize(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = NodeState::Disconnected;
        Ok(())
    }

    /// Get current node state
    pub async fn get_state(&self) -> NodeState {
        self.state.read().await.clone()
    }

    /// Check if node is connected
    pub async fn is_connected(&self) -> bool {
        matches!(*self.state.read().await, NodeState::Connected)
    }

    /// Shutdown the node gracefully
    #[cfg(feature = "network")]
    pub async fn shutdown(&mut self) -> Result<()> {
        if self.node.is_some() {
            // Update state
            let mut state = self.state.write().await;
            *state = NodeState::Disconnected;
            
            // Clear node reference
            self.node = None;
        }
        Ok(())
    }

    /// Shutdown (non-network mode)
    #[cfg(not(feature = "network"))]
    pub async fn shutdown(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = NodeState::Disconnected;
        Ok(())
    }

    /// Reconnect to the network
    pub async fn reconnect(&mut self) -> Result<()> {
        self.shutdown().await?;
        self.initialize().await
    }

    /// Get peer count
    #[cfg(feature = "network")]
    pub async fn get_peer_count(&self) -> usize {
        // TODO: Implement actual peer count from saorsa-core
        // For now, return 0 or 1 based on connection state
        if self.is_connected().await {
            1
        } else {
            0
        }
    }

    /// Get peer count (non-network mode)
    #[cfg(not(feature = "network"))]
    pub async fn get_peer_count(&self) -> usize {
        0
    }

    /// Discover peers
    #[cfg(feature = "network")]
    pub async fn discover_peers(&self) -> Result<Vec<String>> {
        if !self.is_connected().await {
            anyhow::bail!("Node is not connected");
        }

        // TODO: Implement actual peer discovery from saorsa-core
        // For now, return bootstrap nodes as discovered peers
        Ok(self.config.bootstrap_nodes.clone())
    }

    /// Discover peers (non-network mode)
    #[cfg(not(feature = "network"))]
    pub async fn discover_peers(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
}

// Manual Debug implementation
impl std::fmt::Debug for P2PNodeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("P2PNodeManager")
            .field("config", &self.config)
            .field("retry_count", &self.retry_count)
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_creation() {
        let config = NetworkConfig::test_config();
        let manager = P2PNodeManager::new(config);
        assert_eq!(manager.get_state().await, NodeState::Uninitialized);
    }

    #[tokio::test]
    async fn test_node_initialization() {
        let mut config = NetworkConfig::test_config();
        config.enabled = false; // Disable for testing without actual network
        
        let mut manager = P2PNodeManager::new(config);
        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert_eq!(manager.get_state().await, NodeState::Disconnected);
    }

    #[tokio::test]
    async fn test_node_shutdown() {
        let mut config = NetworkConfig::test_config();
        config.enabled = false;
        
        let mut manager = P2PNodeManager::new(config);
        manager.initialize().await.unwrap();
        
        let result = manager.shutdown().await;
        assert!(result.is_ok());
        assert_eq!(manager.get_state().await, NodeState::Disconnected);
    }

    #[tokio::test]
    async fn test_peer_discovery() {
        let mut config = NetworkConfig::test_config();
        config.enabled = false;
        config.bootstrap_nodes = vec!["127.0.0.1:9000".to_string()];
        
        let manager = P2PNodeManager::new(config);
        let peers = manager.discover_peers().await;
        
        #[cfg(feature = "network")]
        {
            // With network feature, should fail when not connected
            assert!(peers.is_err());
        }
        
        #[cfg(not(feature = "network"))]
        {
            // Without network feature, should return empty
            assert!(peers.is_ok());
            assert_eq!(peers.unwrap().len(), 0);
        }
    }

    #[tokio::test]
    async fn test_reconnect() {
        let mut config = NetworkConfig::test_config();
        config.enabled = false;
        
        let mut manager = P2PNodeManager::new(config);
        manager.initialize().await.unwrap();
        
        let result = manager.reconnect().await;
        assert!(result.is_ok());
        assert_eq!(manager.get_state().await, NodeState::Disconnected);
    }
}