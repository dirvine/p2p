//! Bootstrap Discovery Module
//! 
//! Provides multiple mechanisms for discovering bootstrap nodes:
//! 1. Hardcoded well-known bootstrap nodes
//! 2. Three-word address resolution
//! 3. DNS-based discovery (future)
//! 4. Peer exchange from connected nodes

use crate::bootstrap::{WordEncoder, ThreeWordAddress};
use crate::Multiaddr;
use anyhow::{Result, Context};
use std::collections::HashMap;
use tracing::{info, warn, debug};

/// Well-known bootstrap nodes for the P2P Foundation network
#[derive(Debug, Clone)]
pub struct BootstrapDiscovery {
    word_encoder: WordEncoder,
    hardcoded_nodes: HashMap<String, Multiaddr>,
    custom_nodes: Vec<Multiaddr>,
}

impl BootstrapDiscovery {
    /// Create a new bootstrap discovery instance with default well-known nodes
    pub fn new() -> Self {
        let mut hardcoded_nodes = HashMap::new();
        
        // Digital Ocean bootstrap nodes (will be updated with real addresses)
        hardcoded_nodes.insert(
            "foundation.main.bootstrap".to_string(),
            "/dns4/bootstrap.p2pfoundation.org/udp/9000/quic".parse().unwrap()
        );
        
        hardcoded_nodes.insert(
            "foundation.backup.lighthouse".to_string(),
            "/dns4/bootstrap2.p2pfoundation.org/udp/9000/quic".parse().unwrap()
        );
        
        // IPv6 primary bootstrap (Digital Ocean IPv6)
        hardcoded_nodes.insert(
            "global.fast.eagle".to_string(),
            "/ip6/2604:a880:400:d1:0:2:40d7:9001/udp/9000/quic".parse().unwrap()
        );
        
        // IPv4 fallback bootstrap
        hardcoded_nodes.insert(
            "reliable.sturdy.anchor".to_string(),
            "/ip4/147.182.203.123/udp/9000/quic".parse().unwrap()
        );

        Self {
            word_encoder: WordEncoder::new(),
            hardcoded_nodes,
            custom_nodes: Vec::new(),
        }
    }

    /// Add a custom bootstrap node
    pub fn add_bootstrap(&mut self, addr: Multiaddr) {
        self.custom_nodes.push(addr);
    }

    /// Resolve a three-word address to a multiaddr
    pub fn resolve_three_words(&self, three_words: &str) -> Result<Multiaddr> {
        // First check if it's a hardcoded well-known address
        if let Some(addr) = self.hardcoded_nodes.get(three_words) {
            debug!("Resolved hardcoded three-word address: {} -> {}", three_words, addr);
            return Ok(addr.clone());
        }

        // Try to decode as a generated three-word address
        // For now, parse as ThreeWordAddress and attempt resolution
        let word_address = ThreeWordAddress::from_string(three_words)?;
        self.word_encoder.decode_to_multiaddr(&word_address)
            .with_context(|| format!("Failed to resolve three-word address: {}", three_words))
    }

    /// Get all available bootstrap addresses
    pub fn get_bootstrap_addresses(&self) -> Vec<Multiaddr> {
        let mut addresses = Vec::new();
        
        // Add hardcoded nodes
        addresses.extend(self.hardcoded_nodes.values().cloned());
        
        // Add custom nodes
        addresses.extend(self.custom_nodes.clone());
        
        addresses
    }

    /// Get well-known three-word addresses
    pub fn get_well_known_three_words(&self) -> Vec<String> {
        self.hardcoded_nodes.keys().cloned().collect()
    }

    /// Discover bootstrap nodes using multiple methods
    pub async fn discover_bootstraps(&self) -> Result<Vec<Multiaddr>> {
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
    pub async fn test_bootstrap_connectivity(&self) -> Result<Vec<(Multiaddr, bool)>> {
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
    async fn test_single_bootstrap(&self, _addr: &Multiaddr) -> bool {
        // TODO: Implement actual connectivity test
        // This would attempt to establish a connection to the bootstrap node
        // For now, return true as a placeholder
        true
    }

    /// Update the hardcoded bootstrap list (for dynamic updates)
    pub fn update_hardcoded_bootstraps(&mut self, new_bootstraps: HashMap<String, Multiaddr>) {
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
    /// Enable three-word address resolution
    pub enable_three_words: bool,
    /// Enable DNS-based discovery
    pub enable_dns: bool,
    /// Custom bootstrap addresses
    pub custom_bootstraps: Vec<Multiaddr>,
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
            enable_three_words: true,
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
    pub async fn discover(&self) -> Result<Vec<Multiaddr>> {
        self.discover_internal(0).await
    }

    /// Internal discovery with retry limit to prevent infinite recursion
    async fn discover_internal(&self, retry_count: u32) -> Result<Vec<Multiaddr>> {
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

    /// Resolve three-word address if enabled
    pub fn resolve_three_words(&self, three_words: &str) -> Result<Multiaddr> {
        if !self.config.enable_three_words {
            return Err(anyhow::anyhow!("Three-word address resolution disabled"));
        }
        
        self.discovery.resolve_three_words(three_words)
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
    fn test_three_word_resolution() {
        let discovery = BootstrapDiscovery::new();
        
        // Test hardcoded three-word addresses
        let result = discovery.resolve_three_words("foundation.main.bootstrap");
        assert!(result.is_ok(), "Should resolve hardcoded three-word address");
    }

    #[test]
    fn test_custom_bootstrap_addition() {
        let mut discovery = BootstrapDiscovery::new();
        let custom_addr: Multiaddr = "/ip4/192.168.1.100/udp/9000/quic".parse().unwrap();
        
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
        let three_words = discovery.get_well_known_three_words();
        
        assert!(three_words.contains(&"foundation.main.bootstrap".to_string()));
        assert!(three_words.contains(&"global.fast.eagle".to_string()));
    }
}