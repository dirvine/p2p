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

//! Hyperbolic geometry routing implementation
//!
//! Implements greedy routing in hyperbolic space using the Poincaré disk model

use super::*;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// Hyperbolic space manager for coordinate-based routing
pub struct HyperbolicSpace {
    /// Our node's current coordinate
    my_coordinate: RwLock<HyperbolicCoordinate>,
    
    /// Neighbor coordinates
    neighbor_coordinates: Arc<RwLock<HashMap<NodeId, HyperbolicCoordinate>>>,
    
    /// Coordinate adjustment rate
    adjustment_rate: f64,
    
    /// Routing statistics
    routing_stats: Arc<RwLock<RoutingStats>>,
}

/// Statistics for hyperbolic routing performance
#[derive(Debug, Default, Clone)]
pub struct RoutingStats {
    pub attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub fallback_used: u64,
    pub average_hop_count: f64,
}

impl Default for HyperbolicSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl HyperbolicSpace {
    /// Create a new hyperbolic space instance
    pub fn new() -> Self {
        Self {
            my_coordinate: RwLock::new(HyperbolicCoordinate { 
                r: 0.5, 
                theta: rand::random::<f64>() * 2.0 * std::f64::consts::PI 
            }),
            neighbor_coordinates: Arc::new(RwLock::new(HashMap::new())),
            adjustment_rate: 0.01,
            routing_stats: Arc::new(RwLock::new(RoutingStats::default())),
        }
    }
    
    /// Calculate hyperbolic distance between two coordinates
    pub fn distance(a: &HyperbolicCoordinate, b: &HyperbolicCoordinate) -> f64 {
        let delta = 2.0 * ((a.r - b.r).powi(2) + 
                          (a.theta - b.theta).cos().acos().powi(2)).sqrt();
        let denominator = (1.0 - a.r.powi(2)) * (1.0 - b.r.powi(2));
        
        (1.0 + delta / denominator).acosh()
    }
    
    /// Perform greedy routing to find next hop
    pub async fn greedy_route(&self, target: &HyperbolicCoordinate) -> Option<NodeId> {
        let my_coord = self.my_coordinate.read().await;
        let my_distance = Self::distance(&my_coord, target);
        
        let neighbors = self.neighbor_coordinates.read().await;
        neighbors.iter()
            .filter(|(_, coord)| Self::distance(coord, target) < my_distance)
            .min_by(|(_, a), (_, b)| {
                let dist_a = Self::distance(a, target);
                let dist_b = Self::distance(b, target);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(id, _)| id.clone())
    }
    
    /// Adjust our coordinate based on neighbor positions
    pub async fn adjust_coordinate(&self, neighbor_coords: &[(NodeId, HyperbolicCoordinate)]) {
        let mut my_coord = self.my_coordinate.write().await;
        
        // Adjust radial coordinate based on degree
        let degree = neighbor_coords.len();
        let target_r = 1.0 - (2.0 / (degree as f64 + 2.0));
        my_coord.r += self.adjustment_rate * (target_r - my_coord.r);
        
        // Ensure r stays in bounds
        my_coord.r = my_coord.r.clamp(0.0, 0.999);
        
        // Adjust angular coordinate based on neighbor positions
        if !neighbor_coords.is_empty() {
            let avg_theta = neighbor_coords.iter()
                .map(|(_, coord)| coord.theta)
                .sum::<f64>() / neighbor_coords.len() as f64;
            
            let angle_diff = angle_difference(avg_theta, my_coord.theta);
            my_coord.theta += self.adjustment_rate * angle_diff;
            
            // Normalize theta to [0, 2π)
            while my_coord.theta < 0.0 {
                my_coord.theta += 2.0 * std::f64::consts::PI;
            }
            while my_coord.theta >= 2.0 * std::f64::consts::PI {
                my_coord.theta -= 2.0 * std::f64::consts::PI;
            }
        }
    }
    
    /// Get current coordinate
    pub async fn get_coordinate(&self) -> HyperbolicCoordinate {
        *self.my_coordinate.read().await
    }
    
    /// Update a neighbor's coordinate
    pub async fn update_neighbor(&self, node_id: NodeId, coord: HyperbolicCoordinate) {
        let mut neighbors = self.neighbor_coordinates.write().await;
        neighbors.insert(node_id, coord);
    }
    
    /// Remove a neighbor
    pub async fn remove_neighbor(&self, node_id: &NodeId) {
        let mut neighbors = self.neighbor_coordinates.write().await;
        neighbors.remove(node_id);
    }
    
    /// Get routing statistics
    pub async fn get_stats(&self) -> RoutingStats {
        self.routing_stats.read().await.clone()
    }
    
    /// Get routing success rate
    pub async fn get_success_rate(&self) -> f64 {
        let stats = self.routing_stats.read().await;
        if stats.attempts == 0 {
            0.0
        } else {
            stats.successes as f64 / stats.attempts as f64
        }
    }
    
    /// Record routing attempt result
    pub async fn record_routing_result(&self, success: bool, hop_count: usize, used_fallback: bool) {
        let mut stats = self.routing_stats.write().await;
        stats.attempts += 1;
        
        if success {
            stats.successes += 1;
        } else {
            stats.failures += 1;
        }
        
        if used_fallback {
            stats.fallback_used += 1;
        }
        
        // Update average hop count (exponential moving average)
        let alpha = 0.1;
        stats.average_hop_count = (1.0 - alpha) * stats.average_hop_count + alpha * hop_count as f64;
    }
}

/// Calculate the shortest angular difference between two angles
fn angle_difference(a: f64, b: f64) -> f64 {
    let diff = a - b;
    if diff > std::f64::consts::PI {
        diff - 2.0 * std::f64::consts::PI
    } else if diff < -std::f64::consts::PI {
        diff + 2.0 * std::f64::consts::PI
    } else {
        diff
    }
}

/// Hyperbolic routing strategy for integration with AdaptiveRouter
pub struct HyperbolicRoutingStrategy {
    /// The hyperbolic space manager
    space: Arc<HyperbolicSpace>,
    
    /// Local node ID
    local_id: NodeId,
    
    /// Maximum hops before declaring failure
    max_hops: usize,
}

impl HyperbolicRoutingStrategy {
    /// Create a new hyperbolic routing strategy
    pub fn new(local_id: NodeId, space: Arc<HyperbolicSpace>) -> Self {
        Self {
            space,
            local_id,
            max_hops: 10,
        }
    }
    
    /// Find path using greedy hyperbolic routing
    async fn find_hyperbolic_path(&self, target: &NodeId) -> Result<Vec<NodeId>> {
        // Check if we have the target's coordinate
        let target_coord = {
            let neighbors = self.space.neighbor_coordinates.read().await;
            neighbors.get(target).cloned()
        };
        
        let target_coord = match target_coord {
            Some(coord) => coord,
            None => {
                // We don't know the target's coordinate, can't route
                return Err(AdaptiveNetworkError::Routing(
                    "Target coordinate unknown".to_string()
                ));
            }
        };
        
        let mut path = Vec::new();
        let mut current = self.local_id.clone();
        let mut visited = std::collections::HashSet::<NodeId>::new();
        visited.insert(current.clone());
        
        // Greedy routing with loop detection
        for _ in 0..self.max_hops {
            // Find next hop
            let next_hop = self.space.greedy_route(&target_coord).await;
            
            match next_hop {
                Some(next) => {
                    if next == *target {
                        // Reached target
                        path.push(next);
                        return Ok(path);
                    }
                    
                    if visited.contains(&next) {
                        // Loop detected, routing failed
                        return Err(AdaptiveNetworkError::Routing(
                            "Routing loop detected".to_string()
                        ));
                    }
                    
                    path.push(next.clone());
                    visited.insert(next.clone());
                    current = next;
                }
                None => {
                    // No closer neighbor found, greedy routing failed
                    return Err(AdaptiveNetworkError::Routing(
                        "No closer neighbor found".to_string()
                    ));
                }
            }
        }
        
        // Max hops exceeded
        Err(AdaptiveNetworkError::Routing(
            "Maximum hop count exceeded".to_string()
        ))
    }
}

#[async_trait]
impl RoutingStrategy for HyperbolicRoutingStrategy {
    async fn find_path(&self, target: &NodeId) -> Result<Vec<NodeId>> {
        // Try hyperbolic routing
        let result = self.find_hyperbolic_path(target).await;
        
        // Record the result
        let (success, hop_count, used_fallback) = match &result {
            Ok(path) => (true, path.len(), false),
            Err(_) => (false, 0, true), // Will use fallback
        };
        
        self.space.record_routing_result(success, hop_count, used_fallback).await;
        
        result
    }
    
    fn route_score(&self, _neighbor: &NodeId, _target: &NodeId) -> f64 {
        // This is synchronous, so we can't access async coordinates
        // Return a default score - the actual routing logic is in find_path
        0.5
    }
    
    fn update_metrics(&mut self, _path: &[NodeId], _success: bool) {
        // Metrics are updated in find_path via record_routing_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hyperbolic_distance() {
        let origin = HyperbolicCoordinate { r: 0.0, theta: 0.0 };
        let point = HyperbolicCoordinate { r: 0.5, theta: std::f64::consts::PI };
        
        let distance = HyperbolicSpace::distance(&origin, &point);
        assert!(distance > 0.0);
        
        // Distance to self should be 0
        let self_distance = HyperbolicSpace::distance(&origin, &origin);
        assert!((self_distance - 0.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_angle_difference() {
        assert!((angle_difference(0.0, 0.0) - 0.0).abs() < 1e-10);
        assert!((angle_difference(std::f64::consts::PI, 0.0) - std::f64::consts::PI).abs() < 1e-10);
        assert!((angle_difference(0.0, std::f64::consts::PI) - (-std::f64::consts::PI)).abs() < 1e-10);
        assert!((angle_difference(1.9 * std::f64::consts::PI, 0.1 * std::f64::consts::PI) - (-0.2 * std::f64::consts::PI)).abs() < 1e-10);
    }
    
    #[tokio::test]
    async fn test_coordinate_adjustment() {
        let space = HyperbolicSpace::new();
        let initial = space.get_coordinate().await;
        
        // Simulate neighbors at the edge
        use rand::RngCore;
        
        let mut hash1 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash1);
        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);
        
        let neighbors = vec![
            (NodeId::from_bytes(hash1), HyperbolicCoordinate { r: 0.9, theta: 0.0 }),
            (NodeId::from_bytes(hash2), HyperbolicCoordinate { r: 0.9, theta: std::f64::consts::PI }),
        ];
        
        space.adjust_coordinate(&neighbors).await;
        let adjusted = space.get_coordinate().await;
        
        // Should move towards edge with high-degree neighbors
        assert!(adjusted.r > initial.r);
    }
    
    #[tokio::test]
    async fn test_routing_stats() {
        let space = HyperbolicSpace::new();
        
        // Record some routing results
        space.record_routing_result(true, 3, false).await;
        space.record_routing_result(true, 4, false).await;
        space.record_routing_result(false, 0, true).await;
        
        let stats = space.get_stats().await;
        assert_eq!(stats.attempts, 3);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.fallback_used, 1);
        
        let success_rate = space.get_success_rate().await;
        assert!((success_rate - 0.666).abs() < 0.01);
    }
    
    #[tokio::test]
    async fn test_hyperbolic_routing_strategy() {
        use rand::RngCore;
        
        // Create space and strategy
        let space = Arc::new(HyperbolicSpace::new());
        
        let mut hash = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash);
        let local_id = NodeId::from_bytes(hash);
        
        let strategy = HyperbolicRoutingStrategy::new(local_id.clone(), space.clone());
        
        // Add some neighbors with coordinates
        let mut hash1 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash1);
        let neighbor1 = NodeId::from_bytes(hash1);
        
        let mut hash2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash2);
        let neighbor2 = NodeId::from_bytes(hash2);
        
        let mut hash_target = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hash_target);
        let target = NodeId::from_bytes(hash_target);
        
        // Set up coordinates
        space.update_neighbor(neighbor1.clone(), HyperbolicCoordinate { r: 0.3, theta: 0.0 }).await;
        space.update_neighbor(neighbor2.clone(), HyperbolicCoordinate { r: 0.7, theta: 1.0 }).await;
        space.update_neighbor(target.clone(), HyperbolicCoordinate { r: 0.8, theta: 1.5 }).await;
        
        // Try routing to target
        let _result = strategy.find_path(&target).await;
        
        // Without proper network setup, routing will fail, but we can check stats
        let stats = space.get_stats().await;
        assert_eq!(stats.attempts, 1);
    }
    
    #[tokio::test]
    async fn test_greedy_routing() {
        let space = HyperbolicSpace::new();
        
        use rand::RngCore;
        
        // Add neighbors at various positions
        let mut neighbors = vec![];
        for i in 0..5 {
            let mut hash = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut hash);
            let node_id = NodeId::from_bytes(hash);
            
            let coord = HyperbolicCoordinate {
                r: 0.1 + (i as f64) * 0.2,
                theta: (i as f64) * std::f64::consts::PI / 3.0,
            };
            
            space.update_neighbor(node_id.clone(), coord).await;
            neighbors.push((node_id, coord));
        }
        
        // Test greedy routing to a target
        let target_coord = HyperbolicCoordinate { r: 0.9, theta: 1.0 };
        let next_hop = space.greedy_route(&target_coord).await;
        
        // Should find a neighbor closer to target
        assert!(next_hop.is_some());
        
        // Verify it chose the closest neighbor
        if let Some(chosen) = next_hop {
            let neighbors_map = space.neighbor_coordinates.read().await;
            let chosen_coord = neighbors_map.get(&chosen).unwrap();
            let chosen_dist = HyperbolicSpace::distance(chosen_coord, &target_coord);
            
            // Check that no other neighbor is closer
            for (id, coord) in &neighbors {
                if id != &chosen {
                    let dist = HyperbolicSpace::distance(coord, &target_coord);
                    assert!(dist >= chosen_dist);
                }
            }
        }
    }
}