// Copyright (c) 2025 Saorsa Labs Limited

// This file is part of the Saorsa P2P network.

// Licensed under the AGPL-3.0 license:
// <https://www.gnu.org/licenses/agpl-3.0.html>

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


// Copyright 2024 P2P Foundation
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Self-Organizing Map (SOM) Implementation
//!
//! This module provides a Self-Organizing Map for intelligent clustering and organization
//! of nodes in the P2P network based on multi-dimensional features such as content
//! specialization, compute capability, network latency, and storage availability.

use crate::identity::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use rand::Rng;
use super::*;
use async_trait::async_trait;

/// Configuration for Self-Organizing Map
#[derive(Debug, Clone)]
pub struct SomConfig {
    /// Initial learning rate (typically 0.1 - 0.5)
    pub initial_learning_rate: f64,
    /// Initial neighborhood radius
    pub initial_radius: f64,
    /// Number of training iterations
    pub iterations: usize,
    /// Grid size configuration
    pub grid_size: GridSize,
}

/// Grid size configuration
#[derive(Debug, Clone)]
pub enum GridSize {
    /// Fixed grid dimensions
    Fixed(usize, usize),
    /// Dynamic grid that grows with network size
    Dynamic { min: usize, max: usize },
}

/// Multi-dimensional features representing a node's characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFeatures {
    /// Content vector (128-dimensional semantic hash)
    pub content_vector: Vec<f64>,
    /// Compute capability (0-1000 benchmark score)
    pub compute_capability: f64,
    /// Average network latency in milliseconds
    pub network_latency: f64,
    /// Available storage in GB
    pub storage_available: f64,
}

impl NodeFeatures {
    /// Normalize features to ensure consistent scale
    pub fn normalize(&self) -> Self {
        // Normalize content vector to unit length
        let content_magnitude = self.content_vector.iter()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt();
        
        let normalized_content = if content_magnitude > 0.0 {
            self.content_vector.iter()
                .map(|x| x / content_magnitude)
                .collect()
        } else {
            vec![0.0; self.content_vector.len()]
        };
        
        // Normalize other features to [0, 1] range
        Self {
            content_vector: normalized_content,
            compute_capability: self.compute_capability / 1000.0, // Max 1000
            network_latency: (self.network_latency / 200.0).min(1.0), // Max 200ms
            storage_available: (self.storage_available / 5000.0).min(1.0), // Max 5TB
        }
    }
    
    /// Calculate Euclidean distance to another feature vector
    pub fn euclidean_distance(&self, other: &Self) -> f64 {
        let normalized_self = self.normalize();
        let normalized_other = other.normalize();
        
        // Combine all features into a single vector for distance calculation
        let self_vec = normalized_self.to_weight_vector();
        let other_vec = normalized_other.to_weight_vector();
        
        self_vec.iter()
            .zip(other_vec.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }
    
    /// Convert to weight vector for SOM operations
    pub fn to_weight_vector(&self) -> Vec<f64> {
        let normalized = self.normalize();
        let mut weights = normalized.content_vector.clone();
        weights.push(normalized.compute_capability);
        weights.push(normalized.network_latency);
        weights.push(normalized.storage_available);
        weights
    }
}

/// A single neuron in the SOM grid
#[derive(Debug, Clone)]
pub struct Neuron {
    /// Weight vector
    weights: Vec<f64>,
    /// Node IDs assigned to this neuron
    assigned_nodes: HashSet<NodeId>,
}

impl Neuron {
    /// Create a new neuron with random weights
    fn new(weight_dim: usize) -> Self {
        let mut rng = rand::thread_rng();
        let weights = (0..weight_dim)
            .map(|_| rng.gen::<f64>())
            .collect();
        
        Self {
            weights,
            assigned_nodes: HashSet::new(),
        }
    }
    
    /// Calculate distance to input vector
    fn distance(&self, input: &[f64]) -> f64 {
        self.weights.iter()
            .zip(input.iter())
            .map(|(w, i)| (w - i).powi(2))
            .sum::<f64>()
            .sqrt()
    }
    
    /// Update weights based on input and influence
    fn update_weights(&mut self, input: &[f64], learning_rate: f64, influence: f64) {
        for (weight, &input_val) in self.weights.iter_mut().zip(input.iter()) {
            *weight += learning_rate * influence * (input_val - *weight);
        }
    }
}

/// Self-Organizing Map for node clustering
pub struct SelfOrganizingMap {
    /// Grid of neurons
    grid: Arc<RwLock<Vec<Vec<Neuron>>>>,
    /// Current grid dimensions
    width: usize,
    height: usize,
    /// Configuration
    config: SomConfig,
    /// Weight dimension (features + metadata)
    weight_dim: usize,
    /// Node to grid position mapping for fast lookups
    node_positions: Arc<RwLock<HashMap<NodeId, (usize, usize)>>>,
    
    // Legacy fields for compatibility
    /// Feature dimensions
    feature_dim: usize,
    /// Current learning rate
    learning_rate: f64,
    /// Neighborhood radius
    neighborhood_radius: f64,
    /// Training iteration counter
    iteration: u64,
    /// Feature extractor
    extractor: Arc<tokio::sync::RwLock<FeatureExtractor>>,
    /// Node feature cache
    feature_cache: Arc<tokio::sync::RwLock<HashMap<NodeId, [f64; 4]>>>,
}

/// Feature extractor for SOM training
pub struct FeatureExtractor {
    /// Content type history for nodes
    content_history: HashMap<NodeId, HashMap<ContentType, u64>>,
    
    /// Maximum values for normalization
    max_storage: f64,
    max_compute: f64,
    max_bandwidth: f64,
}

impl Default for FeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureExtractor {
    /// Create a new feature extractor
    pub fn new() -> Self {
        Self {
            content_history: HashMap::new(),
            max_storage: 1000.0,    // 1TB default max
            max_compute: 1000.0,    // Arbitrary compute units
            max_bandwidth: 1000.0,  // 1Gbps default max
        }
    }
    
    /// Extract 4D features from a node descriptor
    /// Features: [content_affinity, storage_capacity, compute_capability, network_quality]
    pub fn extract_features(&self, node: &NodeDescriptor) -> [f64; 4] {
        // Feature 1: Content type affinity (based on historical content types)
        let content_affinity = if let Some(history) = self.content_history.get(&node.id) {
            let total: u64 = history.values().sum();
            if total > 0 {
                // Calculate dominant content type ratio
                let max_count = history.values().max().copied().unwrap_or(0);
                max_count as f64 / total as f64
            } else {
                0.5 // Neutral affinity if no history
            }
        } else {
            0.5
        };
        
        // Feature 2: Storage capacity (normalized)
        let storage_capacity = (node.capabilities.storage as f64 / self.max_storage).min(1.0);
        
        // Feature 3: Computational capability (normalized)
        let compute_capability = (node.capabilities.compute as f64 / self.max_compute).min(1.0);
        
        // Feature 4: Network quality (bandwidth * trust)
        let bandwidth_norm = (node.capabilities.bandwidth as f64 / self.max_bandwidth).min(1.0);
        let network_quality = bandwidth_norm * node.trust;
        
        [content_affinity, storage_capacity, compute_capability, network_quality]
    }
    
    /// Update content history for a node
    pub fn update_content_history(&mut self, node_id: &NodeId, content_type: ContentType) {
        let history = self.content_history.entry(node_id.clone()).or_default();
        *history.entry(content_type).or_insert(0) += 1;
    }
    
    /// Update normalization maximums
    pub fn update_max_values(&mut self, storage: f64, compute: f64, bandwidth: f64) {
        self.max_storage = self.max_storage.max(storage);
        self.max_compute = self.max_compute.max(compute);
        self.max_bandwidth = self.max_bandwidth.max(bandwidth);
    }
}

impl SelfOrganizingMap {
    /// Create a new Self-Organizing Map
    pub fn new(config: SomConfig) -> Self {
        let (width, height) = match &config.grid_size {
            GridSize::Fixed(w, h) => (*w, *h),
            GridSize::Dynamic { min, .. } => (min, min),
        };
        
        // Weight dimension = 128 (content) + 3 (other features)
        let weight_dim = 131;
        let feature_dim = 4; // For legacy compatibility
        
        // Initialize grid with random neurons
        let grid = (0..height)
            .map(|_| {
                (0..width)
                    .map(|_| Neuron::new(weight_dim))
                    .collect()
            })
            .collect();
        
        Self {
            grid: Arc::new(RwLock::new(grid)),
            width,
            height,
            config,
            weight_dim,
            node_positions: Arc::new(RwLock::new(HashMap::new())),
            // Legacy fields
            feature_dim,
            learning_rate: 0.1,
            neighborhood_radius: 3.0,
            iteration: 0,
            extractor: Arc::new(tokio::sync::RwLock::new(FeatureExtractor::new())),
            feature_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Create a new SOM with given dimensions (legacy)
    pub fn new_legacy(width: usize, height: usize) -> Self {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 3.0,
            iterations: 1000,
            grid_size: GridSize::Fixed(width, height),
        };
        Self::new(config)
    }
    
    /// Create a new SOM with dynamic sizing based on network size
    pub fn new_dynamic(expected_nodes: usize) -> Self {
        // Calculate grid dimensions based on expected network size
        // Rule: sqrt(5 * sqrt(N)) for optimal coverage
        let grid_size = ((5.0 * (expected_nodes as f64).sqrt()).sqrt() as usize).max(5);
        Self::new(grid_size, grid_size)
    }
    
    /// Update the SOM with a new node descriptor
    pub async fn update_node(&mut self, node: &NodeDescriptor) {
        // Extract features
        let features = {
            let extractor = self.extractor.read().await;
            extractor.extract_features(node)
        };
        
        // Cache the features
        {
            let mut cache = self.feature_cache.write().await;
            cache.insert(node.id.clone(), features);
        }
        
        // Update the SOM
        self.update(&node.id, &features);
        
        // Update extractor's max values
        {
            let mut extractor = self.extractor.write().await;
            extractor.update_max_values(
                node.capabilities.storage as f64,
                node.capabilities.compute as f64,
                node.capabilities.bandwidth as f64,
            );
        }
    }
    
    /// Update the SOM with a new node's features
    fn update(&mut self, node_id: &NodeId, features: &[f64]) {
        assert_eq!(features.len(), self.feature_dim);
        
        // Find best matching unit (BMU)
        let bmu = self.find_bmu(features);
        
        // Update BMU and neighbors
        let learning_rate = self.current_learning_rate();
        let neighborhood_radius = self.current_neighborhood_radius();
        
        for (i, row) in self.map.iter_mut().enumerate() {
            for (j, som_node) in row.iter_mut().enumerate() {
                let distance = ((i as f64 - bmu.0 as f64).powi(2) + 
                               (j as f64 - bmu.1 as f64).powi(2)).sqrt();
                
                if distance <= neighborhood_radius {
                    let influence = (-distance.powi(2) / 
                                    (2.0 * neighborhood_radius.powi(2))).exp();
                    
                    for (k, weight) in som_node.weights.iter_mut().enumerate() {
                        *weight += learning_rate * influence * 
                                  (features[k] - *weight);
                    }
                }
            }
        }
        
        // Update node assignment
        self.map[bmu.0][bmu.1].assigned_nodes.insert(node_id.clone());
        self.iteration += 1;
    }
    
    /// Find the best matching unit for given features
    fn find_bmu(&self, features: &[f64]) -> (usize, usize) {
        let mut min_distance = f64::MAX;
        let mut bmu = (0, 0);
        
        for (i, row) in self.map.iter().enumerate() {
            for (j, node) in row.iter().enumerate() {
                let distance = self.euclidean_distance(&node.weights, features);
                if distance < min_distance {
                    min_distance = distance;
                    bmu = (i, j);
                }
            }
        }
        
        bmu
    }
    
    /// Calculate Euclidean distance between two vectors
    fn euclidean_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }
    
    /// Get current learning rate (decreases over time)
    fn current_learning_rate(&self) -> f64 {
        self.learning_rate * (-(self.iteration as f64) / 1000.0).exp()
    }
    
    /// Get current neighborhood radius (decreases over time)
    fn current_neighborhood_radius(&self) -> f64 {
        self.neighborhood_radius * (-(self.iteration as f64) / 500.0).exp()
    }
    
    /// Find nodes in the same region as the given node
    pub fn find_similar_nodes(&self, node_id: &NodeId) -> Vec<NodeId> {
        for row in &self.map {
            for som_node in row {
                if som_node.assigned_nodes.contains(node_id) {
                    return som_node.assigned_nodes.iter().cloned().collect();
                }
            }
        }
        
        Vec::new()
    }
    
    /// Get the grid position of a node
    pub fn get_node_position(&self, node_id: &NodeId) -> Option<(usize, usize)> {
        for (i, row) in self.map.iter().enumerate() {
            for (j, som_node) in row.iter().enumerate() {
                if som_node.assigned_nodes.contains(node_id) {
                    return Some((i, j));
                }
            }
        }
        
        None
    }
    
    /// Get cached features for a node
    pub async fn get_node_features(&self, node_id: &NodeId) -> Option<[f64; 4]> {
        let cache = self.feature_cache.read().await;
        cache.get(node_id).copied()
    }
    
    /// Update content history for a node
    pub async fn update_content_history(&self, node_id: &NodeId, content_type: ContentType) {
        let mut extractor = self.extractor.write().await;
        extractor.update_content_history(node_id, content_type);
    }
    
    /// Find nodes best suited for a content type
    pub async fn find_nodes_for_content(&self, content_type: ContentType, count: usize) -> Vec<NodeId> {
        let mut candidates = Vec::new();
        
        // Get content type weights (example mapping)
        let target_features = match content_type {
            ContentType::DHTLookup => [0.8, 0.2, 0.5, 0.9],     // High affinity, high network
            ContentType::DataRetrieval => [0.7, 0.9, 0.3, 0.8], // High storage, high network
            ContentType::ComputeRequest => [0.5, 0.3, 0.9, 0.7], // High compute
            ContentType::RealtimeMessage => [0.6, 0.1, 0.4, 1.0], // Highest network quality
        };
        
        // Find BMU for target features
        let bmu = self.find_bmu(&target_features);
        
        // Collect nodes from BMU and neighbors
        let radius = 2;
        for di in -radius..=radius {
            for dj in -radius..=radius {
                let i = (bmu.0 as i32 + di) as usize;
                let j = (bmu.1 as i32 + dj) as usize;
                
                if i < self.map.len() && j < self.map[0].len() {
                    for node_id in &self.map[i][j].assigned_nodes {
                        candidates.push(node_id.clone());
                    }
                }
            }
        }
        
        // Return up to count nodes
        candidates.into_iter().take(count).collect()
    }
}

/// SOM-based routing strategy
pub struct SOMRoutingStrategy {
    /// Reference to the SOM
    som: Arc<RwLock<SelfOrganizingMap>>,
    
    /// Local node ID
    local_id: NodeId,
}

impl SOMRoutingStrategy {
    /// Create a new SOM routing strategy
    pub fn new(som: Arc<RwLock<SelfOrganizingMap>>, local_id: NodeId) -> Self {
        Self { som, local_id }
    }
    
    /// Calculate similarity between two feature vectors
    fn feature_similarity(a: &[f64], b: &[f64]) -> f64 {
        let distance: f64 = a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt();
        
        // Convert distance to similarity score [0, 1]
        1.0 / (1.0 + distance)
    }
}

#[async_trait]
impl RoutingStrategy for SOMRoutingStrategy {
    async fn find_path(&self, target: &NodeId) -> Result<Vec<NodeId>> {
        let som = self.som.read().await;
        
        // Get target node features
        let target_features = match som.get_node_features(target).await {
            Some(features) => features,
            None => {
                // Target not in SOM, can't route
                return Err(AdaptiveNetworkError::Routing(
                    "Target node not found in SOM".to_string()
                ));
            }
        };
        
        // Get our features
        let _our_features = match som.get_node_features(&self.local_id).await {
            Some(features) => features,
            None => {
                return Err(AdaptiveNetworkError::Routing(
                    "Local node not found in SOM".to_string()
                ));
            }
        };
        
        // Find nodes in similar regions
        let similar_nodes = som.find_similar_nodes(target);
        
        if similar_nodes.is_empty() {
            return Err(AdaptiveNetworkError::Routing(
                "No similar nodes found".to_string()
            ));
        }
        
        // Sort by feature similarity and create path
        let mut scored_nodes: Vec<(NodeId, f64)> = Vec::new();
        
        for node_id in similar_nodes {
            if node_id != self.local_id && node_id != *target {
                if let Some(features) = som.get_node_features(&node_id).await {
                    let similarity = Self::feature_similarity(&features, &target_features);
                    scored_nodes.push((node_id, similarity));
                }
            }
        }
        
        scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return top nodes as path
        Ok(scored_nodes.into_iter()
            .take(3)
            .map(|(id, _)| id)
            .chain(std::iter::once(target.clone()))
            .collect())
    }
    
    fn route_score(&self, neighbor: &NodeId, target: &NodeId) -> f64 {
        // This is synchronous, so we can't access async SOM data
        // Return a default score - actual routing logic is in find_path
        0.5
    }
    
    fn update_metrics(&mut self, _path: &[NodeId], _success: bool) {
        // Metrics handled by SOM update_node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Create a test verifying key
    fn create_test_verifying_key() -> ed25519_dalek::VerifyingKey {
        // For tests, we can use a dummy key with all zeros
        ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32])
            .expect("Test key creation should not fail")
    }
    
    #[test]
    fn test_som_creation() {
        let som = SelfOrganizingMap::new(10, 10);
        assert_eq!(som.map.len(), 10);
        assert_eq!(som.map[0].len(), 10);
        assert_eq!(som.feature_dim, 4);
    }
    
    #[test]
    fn test_dynamic_sizing() {
        let som = SelfOrganizingMap::new_dynamic(100);
        let size = som.map.len();
        assert!(size >= 5);
        assert_eq!(som.map[0].len(), size);
    }
    
    #[tokio::test]
    async fn test_som_update() {
        use rand::RngCore;
        
        let mut som = SelfOrganizingMap::new(5, 5);
        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let node_id = NodeId::from_bytes(hash);
        
        let node = NodeDescriptor {
            id: node_id.clone(),
            public_key: create_test_verifying_key(),
            addresses: vec![], // No hardcoded addresses in tests
            hyperbolic: None,
            som_position: None,
            trust: 0.8,
            capabilities: NodeCapabilities {
                storage: 500,
                compute: 300,
                bandwidth: 100,
            },
        };
        
        som.update_node(&node).await;
        
        // Check that the node was assigned
        let similar = som.find_similar_nodes(&node_id);
        assert!(similar.contains(&node_id));
        
        // Check features were cached
        let features = som.get_node_features(&node_id).await;
        assert!(features.is_some());
    }
    
    #[test]
    fn test_learning_rate_decay() {
        let mut som = SelfOrganizingMap::new(5, 5);
        let initial_rate = som.current_learning_rate();
        
        som.iteration = 1000;
        let later_rate = som.current_learning_rate();
        
        assert!(later_rate < initial_rate);
    }
    
    #[test]
    fn test_feature_extraction() {
        use rand::RngCore;
        
        let extractor = FeatureExtractor::new();
        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let node_id = NodeId::from_bytes(hash);
        
        let node = NodeDescriptor {
            id: node_id,
            public_key: create_test_verifying_key(),
            addresses: vec![],
            hyperbolic: None,
            som_position: None,
            trust: 0.9,
            capabilities: NodeCapabilities {
                storage: 750,
                compute: 500,
                bandwidth: 250,
            },
        };
        
        let features = extractor.extract_features(&node);
        
        // Check all features are in range [0, 1]
        for &f in &features {
            assert!(f >= 0.0 && f <= 1.0);
        }
        
        // Check specific feature values
        assert_eq!(features[0], 0.5); // No content history
        assert!(features[1] > 0.7);    // High storage
        assert_eq!(features[2], 0.5);  // Medium compute
        assert!(features[3] > 0.2);    // Good network quality
    }
    
    #[tokio::test]
    async fn test_find_nodes_for_content() {
        use rand::RngCore;
        
        let mut som = SelfOrganizingMap::new(5, 5);
        
        // Add some test nodes
        for i in 0..10 {
            let mut hash = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut hash);
            let node_id = NodeId::from_bytes(hash);
            
            let node = NodeDescriptor {
                id: node_id,
                public_key: create_test_verifying_key(),
                addresses: vec![],
                hyperbolic: None,
                som_position: None,
                trust: 0.5 + (i as f64) * 0.05,
                capabilities: NodeCapabilities {
                    storage: 100 * i,
                    compute: 50 * i,
                    bandwidth: 25 * i,
                },
            };
            
            som.update_node(&node).await;
        }
        
        // Find nodes for different content types
        let dht_nodes = som.find_nodes_for_content(ContentType::DHTLookup, 3).await;
        assert!(!dht_nodes.is_empty());
        
        let storage_nodes = som.find_nodes_for_content(ContentType::DataRetrieval, 3).await;
        assert!(!storage_nodes.is_empty());
    }
    
    #[tokio::test]
    async fn test_som_routing_strategy() {
        use rand::RngCore;
        
        let som = Arc::new(RwLock::new(SelfOrganizingMap::new(5, 5)));
        
        // Create some test nodes
        let mut hash1 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash1);
        let local_id = NodeId::from_bytes(hash1);
        
        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);
        let target_id = NodeId::from_bytes(hash2);
        
        // Add nodes to SOM
        for (id, storage) in [(local_id.clone(), 100), (target_id.clone(), 900)] {
            let node = NodeDescriptor {
                id: id.clone(),
                public_key: create_test_verifying_key(),
                addresses: vec![],
                hyperbolic: None,
                som_position: None,
                trust: 0.8,
                capabilities: NodeCapabilities {
                    storage,
                    compute: 100,
                    bandwidth: 100,
                },
            };
            
            som.write().await.update_node(&node).await;
        }
        
        // Create routing strategy
        let strategy = SOMRoutingStrategy::new(som.clone(), local_id);
        
        // Try to find path (will fail without more nodes in similar regions)
        let result = strategy.find_path(&target_id).await;
        
        // Should fail because no intermediate nodes in similar regions
        assert!(result.is_err());
    }
}