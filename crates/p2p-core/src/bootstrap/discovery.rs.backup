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


//! Bootstrap Discovery Module
//! 
//! Provides multiple mechanisms for discovering bootstrap nodes:
//! 1. Hardcoded well-known bootstrap nodes
//! 2. Four-word address resolution
//! 3. DNS-based discovery (future)
//! 4. Peer exchange from connected nodes

use crate::bootstrap::{WordEncoder, FourWordAddress};
use crate::address::NetworkAddress;
use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{info, warn, debug};

/// Well-known bootstrap nodes for the P2P Foundation network
#[derive(Debug, Clone)]
pub struct BootstrapDiscovery {
    word_encoder: WordEncoder,
    hardcoded_nodes: HashMap<String, NetworkAddress>,
    custom_nodes: Vec<NetworkAddress>,
}

impl BootstrapDiscovery {
    /// Create a new bootstrap discovery instance with default well-known nodes
    pub fn new() -> Self {
        let mut hardcoded_nodes = HashMap::new();
        
        // Digital Ocean bootstrap nodes (will be updated with real addresses)
        hardcoded_nodes.insert(
            "foundation.main.bootstrap".to_string(),
            NetworkAddress::from_ipv4(std::net::Ipv4Addr::new(147, 182, 203, 123), 9000)
        );
        
        hardcoded_nodes.insert(
            "foundation.backup.lighthouse".to_string(),
            NetworkAddress::from_ipv4(std::net::Ipv4Addr::new(147, 182, 203, 124), 9000)
        );
        
        // IPv6 primary bootstrap (Digital Ocean IPv6)
        hardcoded_nodes.insert(
            "global.fast.eagle".to_string(),
            NetworkAddress::from_ipv6("2604:a880:400:d1:0:2:40d7:9001".parse().unwrap(), 9000)
        );
        
        // IPv4 fallback bootstrap
        hardcoded_nodes.insert(
            "reliable.sturdy.anchor".to_string(),
            NetworkAddress::from_ipv4(std::net::Ipv4Addr::new(8, 8, 8, 8), 9000)
        );

        Self {
            word_encoder: WordEncoder::new(),
            hardcoded_nodes,
            custom_nodes: Vec::new(),
        }
    }

    /// Add a custom bootstrap node
    pub fn add_bootstrap(&mut self, addr: NetworkAddress) {
        self.custom_nodes.push(addr);
    }

    /// Resolve a four-word address to a socket address  
    pub fn resolve_four_words(&self, four_words: &str) -> Result<std::net::SocketAddr> {
        // First check if it's a hardcoded well-known address
        if let Some(addr) = self.hardcoded_nodes.get(four_words) {
            debug!("Resolved hardcoded four-word address: {} -> {}", four_words, addr);
            // Convert from NetworkAddress to SocketAddr
            return Ok(addr.socket_addr());
        }

        // Try to decode as a generated four-word address
        let word_address = FourWordAddress::from_string(four_words)
            .map_err(|e| anyhow::anyhow!("Invalid four-word address format: {}", e))?;
        
        // Decode four-word address to IP+port
        let socket_addr = self.word_encoder.decode_to_socket_addr(&word_address)
            .map_err(|e| anyhow::anyhow!("Failed to decode four-word address: {}", e))?;
        
        Ok(socket_addr)
    }

    /// Get all available bootstrap addresses
    pub fn get_bootstrap_addresses(&self) -> Vec<NetworkAddress> {
        let mut addresses = Vec::new();
        
        // Add hardcoded nodes
        addresses.extend(self.hardcoded_nodes.values().cloned());
        
        // Add custom nodes
        addresses.extend(self.custom_nodes.clone());
        
        addresses
    }

    /// Get well-known four-word addresses
    pub fn get_well_known_four_words(&self) -> Vec<String> {
        self.hardcoded_nodes.keys().cloned().collect()
    }

    /// Discover bootstrap nodes using multiple methods
    pub async fn discover_bootstraps(&self) -> Result<Vec<NetworkAddress>> {
        let mut discovered = Vec::new();
        
        info!("🔍 Discovering bootstrap nodes...");
        
        // Start with hardcoded nodes
        let hardcoded = self.get_bootstrap_addresses();
        info!("📍 Found {} hardcoded bootstrap nodes", hardcoded.len());
        discovered.extend(hardcoded);
        
        // TODO: Add DNS-based discovery
        // TODO: Add peer exchange discovery
        // TODO: Add DHT-based discovery
        
        if discovered.is_empty() {
            warn!("⚠️  No bootstrap nodes discovered, network may be unreachable");
        } else {
            info!("✅ Discovered {} total bootstrap nodes", discovered.len());
        }
        
        Ok(discovered)
    }

    /// Test connectivity to bootstrap nodes
    pub async fn test_bootstrap_connectivity(&self) -> Result<Vec<(NetworkAddress, bool)>> {
        let bootstraps = self.get_bootstrap_addresses();
        let mut results = Vec::new();
        
        info!("🧪 Testing connectivity to {} bootstrap nodes", bootstraps.len());
        
        for addr in bootstraps {
            let reachable = self.test_single_bootstrap(&addr).await;
            results.push((addr.clone(), reachable));
            
            if reachable {
                debug!("✅ Bootstrap node reachable: {}", addr);
            } else {
                warn!("❌ Bootstrap node unreachable: {}", addr);
            }
        }
        
        let reachable_count = results.iter().filter(|(_, reachable)| *reachable).count();
        info!("📊 Bootstrap connectivity: {}/{} nodes reachable", reachable_count, results.len());
        
        Ok(results)
    }

    /// Test connectivity to a single bootstrap node
    async fn test_single_bootstrap(&self, _addr: &NetworkAddress) -> bool {
        // TODO: Implement actual connectivity test
        // This would attempt to establish a connection to the bootstrap node
        // For now, return true as a placeholder
        true
    }

    /// Update the hardcoded bootstrap list (for dynamic updates)
    pub fn update_hardcoded_bootstraps(&mut self, new_bootstraps: HashMap<String, NetworkAddress>) {
        info!("🔄 Updating hardcoded bootstrap list with {} entries", new_bootstraps.len());
        self.hardcoded_nodes = new_bootstraps;
    }
}

impl Default for BootstrapDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Bootstrap configuration for different deployment scenarios
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Enable hardcoded bootstrap discovery
    pub enable_hardcoded: bool,
    /// Enable four-word address resolution
    pub enable_four_words: bool,
    /// Enable DNS-based discovery
    pub enable_dns: bool,
    /// Custom bootstrap addresses
    pub custom_bootstraps: Vec<NetworkAddress>,
    /// Fallback behavior when no bootstraps are available
    pub fallback_behavior: FallbackBehavior,
}

#[derive(Debug, Clone)]
pub enum FallbackBehavior {
    /// Continue without bootstrap (may have limited connectivity)
    ContinueWithoutBootstrap,
    /// Retry discovery after a delay
    RetryAfterDelay(std::time::Duration),
    /// Fail if no bootstraps available
    FailIfUnavailable,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            enable_hardcoded: true,
            enable_four_words: true,
            enable_dns: true,
            custom_bootstraps: Vec::new(),
            fallback_behavior: FallbackBehavior::RetryAfterDelay(
                std::time::Duration::from_secs(30)
            ),
        }
    }
}

/// Enhanced bootstrap discovery with configuration
pub struct ConfigurableBootstrapDiscovery {
    discovery: BootstrapDiscovery,
    config: BootstrapConfig,
}

impl ConfigurableBootstrapDiscovery {
    /// Create a new configurable bootstrap discovery
    pub fn new(config: BootstrapConfig) -> Self {
        let mut discovery = BootstrapDiscovery::new();
        
        // Add custom bootstrap nodes
        for addr in &config.custom_bootstraps {
            discovery.add_bootstrap(addr.clone());
        }
        
        Self { discovery, config }
    }

    /// Discover bootstrap nodes with configuration options
    pub async fn discover(&self) -> Result<Vec<NetworkAddress>> {
        self.discover_internal(0).await
    }

    /// Internal discovery with retry limit to prevent infinite recursion
    async fn discover_internal(&self, retry_count: u32) -> Result<Vec<NetworkAddress>> {
        let mut addresses = Vec::new();
        
        if self.config.enable_hardcoded {
            let hardcoded = self.discovery.get_bootstrap_addresses();
            addresses.extend(hardcoded);
        }
        
        // Add custom bootstraps
        addresses.extend(self.config.custom_bootstraps.clone());
        
        if addresses.is_empty() && retry_count < 3 {
            match &self.config.fallback_behavior {
                FallbackBehavior::ContinueWithoutBootstrap => {
                    warn!("⚠️  No bootstrap nodes available, continuing without bootstrap");
                }
                FallbackBehavior::RetryAfterDelay(duration) => {
                    warn!("⚠️  No bootstrap nodes available, retrying after {:?} (attempt {})", duration, retry_count + 1);
                    tokio::time::sleep(*duration).await;
                    return Box::pin(self.discover_internal(retry_count + 1)).await;
                }
                FallbackBehavior::FailIfUnavailable => {
                    return Err(anyhow::anyhow!("No bootstrap nodes available and fallback disabled"));
                }
            }
        }
        
        Ok(addresses)
    }

    /// Resolve four-word address if enabled
    pub fn resolve_four_words(&self, four_words: &str) -> Result<SocketAddr> {
        if !self.config.enable_four_words {
            return Err(anyhow::anyhow!("Four-word address resolution disabled"));
        }
        
        self.discovery.resolve_four_words(four_words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_discovery_creation() {
        let discovery = BootstrapDiscovery::new();
        let addresses = discovery.get_bootstrap_addresses();
        assert!(!addresses.is_empty(), "Should have hardcoded bootstrap addresses");
    }

    #[test]
    fn test_four_word_resolution() {
        let discovery = BootstrapDiscovery::new();
        
        // Test hardcoded four-word addresses
        let result = discovery.resolve_four_words("foundation.main.bootstrap");
        assert!(result.is_ok(), "Should resolve hardcoded four-word address");
    }

    #[test]
    fn test_custom_bootstrap_addition() {
        let mut discovery = BootstrapDiscovery::new();
        let custom_addr = NetworkAddress::from_ipv4(std::net::Ipv4Addr::new(192, 168, 1, 100), 9000);
        
        let initial_count = discovery.get_bootstrap_addresses().len();
        discovery.add_bootstrap(custom_addr.clone());
        let final_count = discovery.get_bootstrap_addresses().len();
        
        assert_eq!(final_count, initial_count + 1, "Should add custom bootstrap");
        assert!(discovery.get_bootstrap_addresses().contains(&custom_addr));
    }

    #[tokio::test]
    async fn test_configurable_discovery() {
        let config = BootstrapConfig::default();
        let discovery = ConfigurableBootstrapDiscovery::new(config);
        
        let addresses = discovery.discover().await.unwrap();
        assert!(!addresses.is_empty(), "Should discover bootstrap addresses");
    }

    #[test]
    fn test_well_known_addresses() {
        let discovery = BootstrapDiscovery::new();
        let four_words = discovery.get_well_known_four_words();
        
        assert!(four_words.contains(&"foundation.main.bootstrap".to_string()));
        assert!(four_words.contains(&"global.fast.eagle".to_string()));
    }
}