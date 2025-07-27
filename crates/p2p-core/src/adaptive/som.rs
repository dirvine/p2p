// Copyright 2024 P2P Foundation
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Self-Organizing Map (SOM) Implementation
//!
//! This module provides a Self-Organizing Map for intelligent clustering and organization
//! of nodes in the P2P network based on multi-dimensional features such as content
//! specialization, compute capability, network latency, and storage availability.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use rand::Rng;

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
            .map(|_| rng.gen_range(0.0..1.0))
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
}

impl SelfOrganizingMap {
    /// Create a new Self-Organizing Map
    pub fn new(config: SomConfig) -> Self {
        let (width, height) = match &config.grid_size {
            GridSize::Fixed(w, h) => (*w, *h),
            GridSize::Dynamic { min, .. } => (*min, *min),
        };
        
        // Weight dimension = 128 (content) + 3 (other features)
        let weight_dim = 131;
        
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
        }
    }
    
    /// Find the Best Matching Unit (BMU) for given features
    pub fn find_best_matching_unit(&self, features: &NodeFeatures) -> (usize, usize) {
        let input = features.to_weight_vector();
        let grid = self.grid.read().unwrap();
        
        let mut best_x = 0;
        let mut best_y = 0;
        let mut best_distance = f64::MAX;
        
        for (y, row) in grid.iter().enumerate() {
            for (x, neuron) in row.iter().enumerate() {
                let distance = neuron.distance(&input);
                if distance < best_distance {
                    best_distance = distance;
                    best_x = x;
                    best_y = y;
                }
            }
        }
        
        (best_x, best_y)
    }
    
    /// Calculate Gaussian neighborhood function
    pub fn gaussian_neighborhood(distance: f64, radius: f64) -> f64 {
        (-distance.powi(2) / (2.0 * radius.powi(2))).exp()
    }
    
    /// Get current neighborhood radius based on iteration
    pub fn get_neighborhood_radius(&self, iteration: usize) -> f64 {
        self.config.initial_radius * (-(iteration as f64) / self.config.iterations as f64).exp()
    }
    
    /// Get current learning rate based on iteration
    pub fn get_learning_rate(&self, iteration: usize) -> f64 {
        self.config.initial_learning_rate * (-(iteration as f64) / self.config.iterations as f64).exp()
    }
    
    /// Train the SOM with a single sample
    pub fn train_single(&mut self, features: &NodeFeatures, iteration: usize) {
        let input = features.to_weight_vector();
        let (bmu_x, bmu_y) = self.find_best_matching_unit(features);
        
        let learning_rate = self.get_learning_rate(iteration);
        let radius = self.get_neighborhood_radius(iteration);
        
        let mut grid = self.grid.write().unwrap();
        
        // Update all neurons based on their distance to BMU
        for (y, row) in grid.iter_mut().enumerate() {
            for (x, neuron) in row.iter_mut().enumerate() {
                let distance = ((x as f64 - bmu_x as f64).powi(2) + 
                               (y as f64 - bmu_y as f64).powi(2)).sqrt();
                
                if distance <= radius * 3.0 { // Only update within 3 * radius
                    let influence = Self::gaussian_neighborhood(distance, radius);
                    neuron.update_weights(&input, learning_rate, influence);
                }
            }
        }
    }
    
    /// Train the SOM with a batch of samples
    pub fn train_batch(&mut self, features_batch: &[NodeFeatures]) {
        for iteration in 0..self.config.iterations {
            // Random sampling for each iteration
            let sample_idx = rand::thread_rng().gen_range(0..features_batch.len());
            self.train_single(&features_batch[sample_idx], iteration);
        }
    }
    
    /// Assign a node to its BMU
    pub fn assign_node(&mut self, node_id: NodeId, features: NodeFeatures) {
        let (x, y) = self.find_best_matching_unit(&features);
        
        // Remove from old position if exists
        let mut positions = self.node_positions.write().unwrap();
        if let Some((old_x, old_y)) = positions.get(&node_id) {
            let mut grid = self.grid.write().unwrap();
            grid[*old_y][*old_x].assigned_nodes.remove(&node_id);
        }
        
        // Assign to new position
        positions.insert(node_id.clone(), (x, y));
        let mut grid = self.grid.write().unwrap();
        grid[y][x].assigned_nodes.insert(node_id);
        
        // Check if we need to resize (for dynamic grids)
        drop(grid);
        drop(positions);
        self.check_and_resize();
    }
    
    /// Get nodes assigned to a specific neuron
    pub fn get_assigned_nodes(&self, x: usize, y: usize) -> HashSet<NodeId> {
        let grid = self.grid.read().unwrap();
        grid.get(y)
            .and_then(|row| row.get(x))
            .map(|neuron| neuron.assigned_nodes.clone())
            .unwrap_or_default()
    }
    
    /// Get all assigned nodes
    pub fn get_all_assigned_nodes(&self) -> HashSet<NodeId> {
        let grid = self.grid.read().unwrap();
        grid.iter()
            .flat_map(|row| row.iter())
            .flat_map(|neuron| neuron.assigned_nodes.iter())
            .cloned()
            .collect()
    }
    
    /// Find nodes similar to given features
    pub fn find_similar_nodes(&self, features: &NodeFeatures, radius: usize) -> Vec<NodeId> {
        let (bmu_x, bmu_y) = self.find_best_matching_unit(features);
        let grid = self.grid.read().unwrap();
        
        let mut similar_nodes = Vec::new();
        
        // Search in neighborhood around BMU
        let x_start = bmu_x.saturating_sub(radius);
        let x_end = (bmu_x + radius + 1).min(self.width);
        let y_start = bmu_y.saturating_sub(radius);
        let y_end = (bmu_y + radius + 1).min(self.height);
        
        for y in y_start..y_end {
            for x in x_start..x_end {
                if let Some(neuron) = grid.get(y).and_then(|row| row.get(x)) {
                    similar_nodes.extend(neuron.assigned_nodes.iter().cloned());
                }
            }
        }
        
        similar_nodes
    }
    
    /// Get current grid dimensions
    pub fn get_grid_dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
    
    /// Set neuron weights (for testing)
    pub fn set_neuron_weights(&mut self, x: usize, y: usize, weights: Vec<f64>) {
        let mut grid = self.grid.write().unwrap();
        if let Some(neuron) = grid.get_mut(y).and_then(|row| row.get_mut(x)) {
            neuron.weights = weights;
        }
    }
    
    /// Get neuron weights (for testing)
    pub fn get_neuron_weights(&self, x: usize, y: usize) -> Option<Vec<f64>> {
        let grid = self.grid.read().unwrap();
        grid.get(y)
            .and_then(|row| row.get(x))
            .map(|neuron| neuron.weights.clone())
    }
    
    /// Check if grid needs resizing and resize if necessary
    fn check_and_resize(&mut self) {
        if let GridSize::Dynamic { max, .. } = self.config.grid_size {
            let node_count = self.node_positions.read().unwrap().len();
            let current_capacity = self.width * self.height;
            
            // Resize if we're at 80% capacity and haven't hit max
            if node_count as f64 > current_capacity as f64 * 0.8 {
                let new_size = ((current_capacity as f64 * 1.5).sqrt() as usize).min(max);
                if new_size > self.width || new_size > self.height {
                    self.resize_grid(new_size, new_size);
                }
            }
        }
    }
    
    /// Resize the grid while preserving node assignments
    fn resize_grid(&mut self, new_width: usize, new_height: usize) {
        let mut old_grid = self.grid.write().unwrap();
        let mut new_grid = vec![vec![Neuron::new(self.weight_dim); new_width]; new_height];
        
        // Copy neurons that fit in new grid
        for (y, row) in old_grid.iter().enumerate() {
            if y >= new_height {
                break;
            }
            for (x, neuron) in row.iter().enumerate() {
                if x >= new_width {
                    break;
                }
                new_grid[y][x] = neuron.clone();
            }
        }
        
        *old_grid = new_grid;
        self.width = new_width;
        self.height = new_height;
        
        // Re-assign nodes that were outside the new grid
        let positions = self.node_positions.read().unwrap().clone();
        for (node_id, (x, y)) in positions {
            if x >= new_width || y >= new_height {
                // Find new position for this node
                // For simplicity, we'll just find a nearby valid position
                let new_x = x.min(new_width - 1);
                let new_y = y.min(new_height - 1);
                old_grid[new_y][new_x].assigned_nodes.insert(node_id.clone());
                self.node_positions.write().unwrap().insert(node_id, (new_x, new_y));
            }
        }
    }
    
    /// Get visualization data for the SOM
    pub fn get_visualization_data(&self) -> VisualizationData {
        let grid = self.grid.read().unwrap();
        let neurons = grid.iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter().enumerate().map(move |(x, neuron)| {
                    NeuronVisualization {
                        x,
                        y,
                        weights: neuron.weights.clone(),
                        assigned_nodes: neuron.assigned_nodes.iter().cloned().collect(),
                    }
                })
            })
            .collect();
        
        VisualizationData {
            grid_width: self.width,
            grid_height: self.height,
            neurons,
        }
    }
    
    /// Generate U-Matrix (unified distance matrix) for visualization
    pub fn generate_u_matrix(&self) -> Vec<Vec<f64>> {
        let grid = self.grid.read().unwrap();
        let mut u_matrix = vec![vec![0.0; self.width]; self.height];
        
        for y in 0..self.height {
            for x in 0..self.width {
                let mut distances = Vec::new();
                let current = &grid[y][x];
                
                // Calculate distances to neighbors
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        
                        let nx = (x as i32 + dx) as usize;
                        let ny = (y as i32 + dy) as usize;
                        
                        if nx < self.width && ny < self.height {
                            let neighbor = &grid[ny][nx];
                            let distance = current.weights.iter()
                                .zip(neighbor.weights.iter())
                                .map(|(a, b)| (a - b).powi(2))
                                .sum::<f64>()
                                .sqrt();
                            distances.push(distance);
                        }
                    }
                }
                
                // Average distance to neighbors
                if !distances.is_empty() {
                    u_matrix[y][x] = distances.iter().sum::<f64>() / distances.len() as f64;
                }
            }
        }
        
        u_matrix
    }
}

/// Visualization data for a single neuron
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuronVisualization {
    pub x: usize,
    pub y: usize,
    pub weights: Vec<f64>,
    pub assigned_nodes: Vec<NodeId>,
}

/// Complete visualization data for the SOM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationData {
    pub grid_width: usize,
    pub grid_height: usize,
    pub neurons: Vec<NeuronVisualization>,
}

// Legacy support types
use super::*;

/// SOM-based routing strategy
pub struct SOMRoutingStrategy;

/// Feature extractor for SOM
pub struct FeatureExtractor;