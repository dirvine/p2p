// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Network configuration management for P2P settings

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Network configuration for P2P connectivity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable P2P networking
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Listen address for P2P connections
    #[serde(default = "default_listen_address")]
    pub listen_address: String,

    /// Bootstrap nodes for initial network discovery
    #[serde(default)]
    pub bootstrap_nodes: Vec<String>,

    /// Enable IPv6 support
    #[serde(default)]
    pub enable_ipv6: bool,

    /// Maximum number of connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Enable DHT for peer discovery
    #[serde(default = "default_enable_dht")]
    pub enable_dht: bool,

    /// Enable mDNS for local peer discovery
    #[serde(default)]
    pub enable_mdns: bool,

    /// Node display name
    #[serde(default = "default_node_name")]
    pub node_name: String,

    /// Store path for network data
    #[serde(default)]
    pub storage_path: Option<PathBuf>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            listen_address: default_listen_address(),
            bootstrap_nodes: Vec::new(),
            enable_ipv6: false,
            max_connections: default_max_connections(),
            connection_timeout_secs: default_connection_timeout(),
            enable_dht: default_enable_dht(),
            enable_mdns: false,
            node_name: default_node_name(),
            storage_path: None,
        }
    }
}

// Default value functions for serde
fn default_enabled() -> bool {
    false
}

fn default_listen_address() -> String {
    "0.0.0.0:0".to_string()
}

fn default_max_connections() -> usize {
    100
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_enable_dht() -> bool {
    true
}

fn default_node_name() -> String {
    format!("communitas-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("node"))
}

impl NetworkConfig {
    /// Create a new network configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a file
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read network config file")?;

        let config = if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content)
                .context("Failed to parse JSON network config")?
        } else {
            toml::from_str(&content)
                .context("Failed to parse TOML network config")?
        };

        Ok(config)
    }

    /// Save configuration to a file
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }

        let content = if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::to_string_pretty(self)
                .context("Failed to serialize network config to JSON")?
        } else {
            toml::to_string_pretty(self)
                .context("Failed to serialize network config to TOML")?
        };

        tokio::fs::write(path, content)
            .await
            .context("Failed to write network config file")?;

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate listen address
        if self.enabled && !self.listen_address.is_empty() {
            if self.listen_address != "0.0.0.0:0" && self.listen_address != "[::]:0" {
                self.listen_address.parse::<SocketAddr>()
                    .context("Invalid listen address")?;
            }
        }

        // Validate bootstrap nodes
        for node in &self.bootstrap_nodes {
            if !node.is_empty() {
                // Try parsing as socket address or as multiaddr
                if !node.contains('/') {
                    node.parse::<SocketAddr>()
                        .context(format!("Invalid bootstrap node address: {}", node))?;
                }
            }
        }

        // Validate connection settings
        if self.max_connections == 0 {
            anyhow::bail!("Maximum connections must be greater than 0");
        }

        if self.connection_timeout_secs == 0 {
            anyhow::bail!("Connection timeout must be greater than 0");
        }

        Ok(())
    }

    /// Add a bootstrap node
    pub fn add_bootstrap_node(&mut self, node: String) -> Result<()> {
        // Validate the node address before adding
        if !node.is_empty() {
            if !node.contains('/') {
                node.parse::<SocketAddr>()
                    .context(format!("Invalid bootstrap node address: {}", node))?;
            }
            
            if !self.bootstrap_nodes.contains(&node) {
                self.bootstrap_nodes.push(node);
            }
        }
        Ok(())
    }

    /// Remove a bootstrap node
    pub fn remove_bootstrap_node(&mut self, node: &str) -> bool {
        if let Some(pos) = self.bootstrap_nodes.iter().position(|n| n == node) {
            self.bootstrap_nodes.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clear all bootstrap nodes
    pub fn clear_bootstrap_nodes(&mut self) {
        self.bootstrap_nodes.clear();
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// Get the storage path, creating a default if not set
    pub fn get_storage_path(&self) -> Result<PathBuf> {
        if let Some(ref path) = self.storage_path {
            Ok(path.clone())
        } else {
            let dirs = directories::ProjectDirs::from("com", "saorsa", "communitas")
                .context("Failed to get project directories")?;
            Ok(dirs.data_dir().join("network"))
        }
    }

    /// Create a config suitable for testing
    #[cfg(test)]
    pub fn test_config() -> Self {
        Self {
            enabled: true,
            listen_address: "127.0.0.1:0".to_string(),
            bootstrap_nodes: vec![],
            enable_ipv6: false,
            max_connections: 10,
            connection_timeout_secs: 5,
            enable_dht: false,
            enable_mdns: false,
            node_name: "test-node".to_string(),
            storage_path: None,
        }
    }
}

/// Bootstrap node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapNode {
    /// Node address (socket address or multiaddr)
    pub address: String,
    
    /// Optional node name/description
    pub name: Option<String>,
    
    /// Whether this node is currently active
    #[serde(default = "default_active")]
    pub active: bool,
    
    /// Priority for connection attempts (lower = higher priority)
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_active() -> bool {
    true
}

fn default_priority() -> u32 {
    100
}

impl BootstrapNode {
    /// Create a new bootstrap node
    pub fn new(address: String) -> Self {
        Self {
            address,
            name: None,
            active: true,
            priority: default_priority(),
        }
    }

    /// Create a bootstrap node with a name
    pub fn with_name(address: String, name: String) -> Self {
        Self {
            address,
            name: Some(name),
            active: true,
            priority: default_priority(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.listen_address, "0.0.0.0:0");
        assert!(config.bootstrap_nodes.is_empty());
        assert_eq!(config.max_connections, 100);
    }

    #[test]
    fn test_config_validation() {
        let mut config = NetworkConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid listen address
        config.enabled = true;
        config.listen_address = "invalid:address:format".to_string();
        assert!(config.validate().is_err());

        // Test valid listen address
        config.listen_address = "127.0.0.1:8080".to_string();
        assert!(config.validate().is_ok());

        // Test invalid max connections
        config.max_connections = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bootstrap_node_management() {
        let mut config = NetworkConfig::default();
        
        // Add valid bootstrap node
        assert!(config.add_bootstrap_node("127.0.0.1:9000".to_string()).is_ok());
        assert_eq!(config.bootstrap_nodes.len(), 1);
        
        // Try to add duplicate
        assert!(config.add_bootstrap_node("127.0.0.1:9000".to_string()).is_ok());
        assert_eq!(config.bootstrap_nodes.len(), 1);
        
        // Add another node
        assert!(config.add_bootstrap_node("[::1]:9001".to_string()).is_ok());
        assert_eq!(config.bootstrap_nodes.len(), 2);
        
        // Remove a node
        assert!(config.remove_bootstrap_node("127.0.0.1:9000"));
        assert_eq!(config.bootstrap_nodes.len(), 1);
        
        // Clear all nodes
        config.clear_bootstrap_nodes();
        assert!(config.bootstrap_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_config_save_load_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("network.toml");
        
        let mut config = NetworkConfig::default();
        config.enabled = true;
        config.node_name = "test-node".to_string();
        config.add_bootstrap_node("127.0.0.1:9000".to_string()).unwrap();
        
        // Save config
        config.save(&config_path).await.unwrap();
        assert!(config_path.exists());
        
        // Load config
        let loaded = NetworkConfig::load(&config_path).await.unwrap();
        assert_eq!(loaded.enabled, config.enabled);
        assert_eq!(loaded.node_name, config.node_name);
        assert_eq!(loaded.bootstrap_nodes, config.bootstrap_nodes);
    }

    #[tokio::test]
    async fn test_config_save_load_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("network.json");
        
        let mut config = NetworkConfig::default();
        config.enabled = true;
        config.enable_ipv6 = true;
        config.max_connections = 50;
        
        // Save config
        config.save(&config_path).await.unwrap();
        assert!(config_path.exists());
        
        // Load config
        let loaded = NetworkConfig::load(&config_path).await.unwrap();
        assert_eq!(loaded.enabled, config.enabled);
        assert_eq!(loaded.enable_ipv6, config.enable_ipv6);
        assert_eq!(loaded.max_connections, config.max_connections);
    }

    #[test]
    fn test_bootstrap_node_creation() {
        let node = BootstrapNode::new("127.0.0.1:9000".to_string());
        assert_eq!(node.address, "127.0.0.1:9000");
        assert!(node.name.is_none());
        assert!(node.active);
        assert_eq!(node.priority, 100);
        
        let named_node = BootstrapNode::with_name(
            "192.168.1.1:9000".to_string(),
            "Primary Bootstrap".to_string()
        );
        assert_eq!(named_node.name, Some("Primary Bootstrap".to_string()));
    }
}