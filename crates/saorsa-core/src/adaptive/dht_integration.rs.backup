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

//! DHT Integration for Adaptive Network
//! 
//! This module bridges the existing S/Kademlia implementation with the adaptive
//! network, providing trust-weighted routing and integration with other adaptive
//! components.

use super::*;
use crate::dht::{DHT, DHTConfig, Key as DHTKey, DHTNode};
use crate::dht::skademlia::{SKademlia, SKademliaConfig};
use crate::{PeerId, Multiaddr};
use sha2::Digest;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::str::FromStr;

/// Adaptive DHT that integrates S/Kademlia with trust scoring
pub struct AdaptiveDHT {
    /// Underlying S/Kademlia implementation
    skademlia: Arc<RwLock<SKademlia>>,
    
    /// Base DHT for standard operations
    base_dht: Arc<RwLock<DHT>>,
    
    /// Trust provider for weighted routing
    trust_provider: Arc<dyn TrustProvider>,
    
    /// Adaptive router for strategy selection
    router: Arc<AdaptiveRouter>,
    
    /// Local node identity
    identity: Arc<NodeIdentity>,
    
    /// Performance metrics
    metrics: Arc<RwLock<DHTMetrics>>,
}

/// DHT performance metrics
#[derive(Debug, Default, Clone)]
pub struct DHTMetrics {
    pub lookups_total: u64,
    pub lookups_successful: u64,
    pub stores_total: u64,
    pub stores_successful: u64,
    pub average_lookup_hops: f64,
    pub trust_rejections: u64,
}

impl AdaptiveDHT {
    /// Create new adaptive DHT instance
    pub async fn new(
        config: DHTConfig,
        identity: Arc<NodeIdentity>,
        trust_provider: Arc<dyn TrustProvider>,
        router: Arc<AdaptiveRouter>,
    ) -> Result<Self> {
        let skademlia_config = SKademliaConfig {
            min_routing_reputation: 0.3,
            enable_distance_verification: true,
            enable_routing_validation: true,
            ..Default::default()
        };
        
        // Create DHT with local ID from identity
        let local_key = Self::node_id_to_key(identity.node_id());
        let base_dht = Arc::new(RwLock::new(DHT::new(local_key, config.clone())));
        // Create reputation manager for S/Kademlia
        let reputation_manager = crate::security::ReputationManager::new(0.99, 0.1);
        let skademlia = Arc::new(RwLock::new(SKademlia::new(skademlia_config)));
        
        Ok(Self {
            skademlia,
            base_dht,
            trust_provider,
            router,
            identity,
            metrics: Arc::new(RwLock::new(DHTMetrics::default())),
        })
    }
    
    /// Convert adaptive NodeId to DHT Key
    fn node_id_to_key(node_id: &NodeId) -> DHTKey {
        DHTKey::new(&node_id.hash)
    }
    
    /// Store value in the DHT with trust-based replication
    pub async fn store(&self, key: Vec<u8>, value: Vec<u8>) -> Result<ContentHash> {
        let mut metrics = self.metrics.write().await;
        metrics.stores_total += 1;
        
        // Create DHT key
        let dht_key = DHTKey::new(&key);
        
        // Store in base DHT using put method
        let dht = self.base_dht.read().await;
        dht.put(dht_key, value.clone()).await
            .map_err(|e| AdaptiveNetworkError::Other(e.to_string()))?;
        
        metrics.stores_successful += 1;
        
        // Return content hash
        let mut hasher = sha2::Sha256::new();
        hasher.update(&key);
        hasher.update(&value);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        
        Ok(ContentHash(hash))
    }
    
    /// Retrieve value from DHT using adaptive routing
    pub async fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        let mut metrics = self.metrics.write().await;
        metrics.lookups_total += 1;
        
        // Use adaptive router to select routing strategy
        let target_id = NodeId::from_bytes(hash.0);
        let path = self.router.route(&target_id, ContentType::DataRetrieval).await?;
        
        // Create DHT key from hash
        let dht_key = DHTKey::new(&hash.0);
        
        // Lookup in base DHT
        let dht = self.base_dht.read().await;
        match dht.get(&dht_key).await {
            Some(record) => {
                metrics.lookups_successful += 1;
                Ok(record.value)
            }
            None => {
                Err(AdaptiveNetworkError::Other("Record not found".to_string()))
            }
        }
    }
    
    /// Find nodes close to a key using trust-weighted selection
    pub async fn find_closest_nodes(&self, target: &NodeId, count: usize) -> Result<Vec<NodeDescriptor>> {
        let dht_key = Self::node_id_to_key(target);
        let dht = self.base_dht.read().await;
        
        // Get closest nodes from DHT using find_node
        let nodes = dht.find_node(&dht_key).await;
        
        // Sort by trust score
        let sorted_nodes: Vec<_> = nodes.into_iter()
            .filter_map(|node| {
                // Extract node ID from peer_id string
                // PeerId is a string, so we need to hash it to get 32 bytes
                let mut hash = [0u8; 32];
                let peer_bytes = node.peer_id.as_bytes();
                if peer_bytes.len() >= 32 {
                    hash.copy_from_slice(&peer_bytes[..32]);
                } else {
                    // If peer_id is shorter, hash it
                    let hashed = blake3::hash(peer_bytes);
                    hash.copy_from_slice(hashed.as_bytes());
                }
                let node_id = NodeId::from_bytes(hash);
                let trust = self.trust_provider.get_trust(&node_id);
                
                // Filter out low-trust nodes
                if trust < 0.3 {
                    return None;
                }
                
                Some((node, trust))
            })
            .collect();
        
        // Sort by trust descending
        let mut sorted_nodes = sorted_nodes;
        sorted_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Take top nodes and convert to NodeDescriptors
        Ok(sorted_nodes.into_iter()
            .take(count)
            .map(|(node, trust)| {
                // Convert peer_id to node_id
                let mut hash = [0u8; 32];
                let peer_bytes = node.peer_id.as_bytes();
                if peer_bytes.len() >= 32 {
                    hash.copy_from_slice(&peer_bytes[..32]);
                } else {
                    let hashed = blake3::hash(peer_bytes);
                    hash.copy_from_slice(hashed.as_bytes());
                }
                let node_id = NodeId::from_bytes(hash);
                
                NodeDescriptor {
                    id: node_id,
                    public_key: ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32]).unwrap(), // TODO: Get real key
                    addresses: node.addresses.iter().map(|a| a.to_string()).collect(),
                    hyperbolic: None,
                    som_position: None,
                    trust,
                    capabilities: NodeCapabilities {
                        storage: 0,
                        compute: 0,
                        bandwidth: 0,
                    },
                }
            }).collect())
    }
    
    /// Update routing table with new node information
    pub async fn update_routing(&self, node: NodeDescriptor) -> Result<()> {
        // Convert NodeId to PeerId (using the hash as peer ID string)
        let peer_id = PeerId::from_str(&node.id.to_string())
            .map_err(|e| AdaptiveNetworkError::Other(format!("Invalid peer ID: {e}")))?;
        
        // Parse addresses to Multiaddr
        let addresses: Vec<Multiaddr> = node.addresses.iter()
            .filter_map(|a| Multiaddr::from_str(a).ok())
            .collect();
        
        if addresses.is_empty() {
            return Err(AdaptiveNetworkError::Other("No valid addresses".to_string()));
        }
        
        let dht = self.base_dht.read().await;
        dht.add_node(DHTNode::new(peer_id, addresses, &Self::node_id_to_key(&node.id))).await
            .map_err(|e| AdaptiveNetworkError::Other(e.to_string()))
    }
    
    /// Get current DHT metrics
    pub async fn get_metrics(&self) -> DHTMetrics {
        self.metrics.read().await.clone()
    }
}

/// Implement Kademlia routing strategy for adaptive router
pub struct KademliaRoutingStrategy {
    dht: Arc<AdaptiveDHT>,
}

impl KademliaRoutingStrategy {
    pub fn new(dht: Arc<AdaptiveDHT>) -> Self {
        Self { dht }
    }
}

#[async_trait]
impl RoutingStrategy for KademliaRoutingStrategy {
    async fn find_path(&self, target: &NodeId) -> Result<Vec<NodeId>> {
        let nodes = self.dht.find_closest_nodes(target, 3).await?;
        Ok(nodes.into_iter().map(|n| n.id).collect())
    }
    
    fn route_score(&self, neighbor: &NodeId, target: &NodeId) -> f64 {
        // XOR distance metric
        let neighbor_bytes = &neighbor.hash;
        let target_bytes = &target.hash;
        let mut distance = 0u32;
        
        for i in 0..32 {
            distance += (neighbor_bytes[i] ^ target_bytes[i]).count_ones();
        }
        
        // Convert to score (closer = higher score)
        1.0 / (1.0 + distance as f64)
    }
    
    fn update_metrics(&mut self, _path: &[NodeId], _success: bool) {
        // Metrics updated in AdaptiveDHT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_adaptive_dht_creation() {
        struct MockTrustProvider;
        impl TrustProvider for MockTrustProvider {
            fn get_trust(&self, _node: &NodeId) -> f64 { 0.5 }
            fn update_trust(&self, _from: &NodeId, _to: &NodeId, _success: bool) {}
            fn get_global_trust(&self) -> HashMap<NodeId, f64> {
                HashMap::new()
            }
        }
        
        let config = DHTConfig::default();
        let identity = Arc::new(NodeIdentity::generate().unwrap());
        let trust_provider = Arc::new(MockTrustProvider);
        let router = Arc::new(AdaptiveRouter::new(
            identity.node_id().clone(),
            trust_provider.clone(),
        ));
        
        let dht = AdaptiveDHT::new(config, identity, trust_provider, router).await.unwrap();
        let metrics = dht.get_metrics().await;
        
        assert_eq!(metrics.lookups_total, 0);
        assert_eq!(metrics.stores_total, 0);
    }
    
    #[tokio::test]
    async fn test_node_id_to_key_conversion() {
        use rand::RngCore;
        
        // Create a random UserId
        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let node_id = NodeId::from_bytes(hash);
        
        let key = AdaptiveDHT::node_id_to_key(&node_id);
        
        // Should create valid key from node ID
        assert_eq!(key.as_bytes().len(), 32);
    }
}