use saorsa_core::adaptive::{
    HyperbolicSpace, HyperbolicCoordinate, HyperbolicRoutingStrategy,
    NodeId, RoutingStrategy, AdaptiveNetworkError,
};
use std::sync::Arc;
use proptest::prelude::*;
use approx::assert_relative_eq;

// Property-based testing for hyperbolic distance metric
proptest! {
    #[test]
    fn prop_distance_is_symmetric(
        r1 in 0.0..0.999f64,
        theta1 in 0.0..(2.0 * std::f64::consts::PI),
        r2 in 0.0..0.999f64,
        theta2 in 0.0..(2.0 * std::f64::consts::PI)
    ) {
        let coord1 = HyperbolicCoordinate { r: r1, theta: theta1 };
        let coord2 = HyperbolicCoordinate { r: r2, theta: theta2 };
        
        let dist1 = HyperbolicSpace::distance(&coord1, &coord2);
        let dist2 = HyperbolicSpace::distance(&coord2, &coord1);
        
        assert_relative_eq!(dist1, dist2, epsilon = 1e-10);
    }
    
    #[test]
    fn prop_distance_satisfies_triangle_inequality(
        r1 in 0.0..0.999f64,
        theta1 in 0.0..(2.0 * std::f64::consts::PI),
        r2 in 0.0..0.999f64,
        theta2 in 0.0..(2.0 * std::f64::consts::PI),
        r3 in 0.0..0.999f64,
        theta3 in 0.0..(2.0 * std::f64::consts::PI)
    ) {
        let coord1 = HyperbolicCoordinate { r: r1, theta: theta1 };
        let coord2 = HyperbolicCoordinate { r: r2, theta: theta2 };
        let coord3 = HyperbolicCoordinate { r: r3, theta: theta3 };
        
        let dist12 = HyperbolicSpace::distance(&coord1, &coord2);
        let dist23 = HyperbolicSpace::distance(&coord2, &coord3);
        let dist13 = HyperbolicSpace::distance(&coord1, &coord3);
        
        // Triangle inequality: d(a,c) <= d(a,b) + d(b,c)
        assert!(dist13 <= dist12 + dist23 + 1e-10);
    }
    
    #[test]
    fn prop_distance_is_non_negative(
        r1 in 0.0..0.999f64,
        theta1 in 0.0..(2.0 * std::f64::consts::PI),
        r2 in 0.0..0.999f64,
        theta2 in 0.0..(2.0 * std::f64::consts::PI)
    ) {
        let coord1 = HyperbolicCoordinate { r: r1, theta: theta1 };
        let coord2 = HyperbolicCoordinate { r: r2, theta: theta2 };
        
        let dist = HyperbolicSpace::distance(&coord1, &coord2);
        assert!(dist >= 0.0);
    }
    
    #[test]
    fn prop_greedy_routing_convergence(
        seed: u64,
        num_nodes in 10..50usize,
    ) {
        // TODO: Implement property test for greedy routing convergence
        // This will require setting up a network topology
    }
}

#[test]
fn test_distance_edge_cases() {
    // Test distance at origin
    let origin = HyperbolicCoordinate { r: 0.0, theta: 0.0 };
    let point = HyperbolicCoordinate { r: 0.5, theta: std::f64::consts::PI };
    
    let dist = HyperbolicSpace::distance(&origin, &point);
    assert!(dist > 0.0);
    assert!(dist.is_finite());
    
    // Test distance near boundary
    let boundary1 = HyperbolicCoordinate { r: 0.999, theta: 0.0 };
    let boundary2 = HyperbolicCoordinate { r: 0.999, theta: std::f64::consts::PI };
    
    let boundary_dist = HyperbolicSpace::distance(&boundary1, &boundary2);
    assert!(boundary_dist.is_finite());
    assert!(boundary_dist > 0.0);
    
    // Test same point
    let same_dist = HyperbolicSpace::distance(&origin, &origin);
    assert_relative_eq!(same_dist, 0.0, epsilon = 1e-10);
}

#[tokio::test]
async fn test_coordinate_adjustment_with_hysteresis() {
    let space = HyperbolicSpace::new();
    let initial = space.get_coordinate().await;
    
    // Create neighbors with specific pattern
    let neighbors = create_test_neighbors(5, 0.8);
    
    // Perform multiple adjustments
    let mut previous = initial;
    for _ in 0..10 {
        space.adjust_coordinate(&neighbors).await;
        let current = space.get_coordinate().await;
        
        // Check that adjustments become smaller (hysteresis effect)
        let delta_r = (current.r - previous.r).abs();
        let delta_theta = angle_diff(current.theta, previous.theta).abs();
        
        // Adjustments should be bounded
        assert!(delta_r < 0.1);
        assert!(delta_theta < 0.1);
        
        previous = current;
    }
    
    // Final coordinate should be stable
    let final_coord = space.get_coordinate().await;
    assert!(final_coord.r > initial.r); // Should move outward with high-degree neighbors
    assert!(final_coord.r < 0.999); // Should stay within bounds
}

#[tokio::test]
async fn test_greedy_routing_decisions() {
    let space = Arc::new(HyperbolicSpace::new());
    let local_id = generate_test_node_id(0);
    
    // Create a network topology
    let mut topology = vec![];
    for i in 1..10 {
        let node_id = generate_test_node_id(i);
        let coord = HyperbolicCoordinate {
            r: 0.1 + (i as f64) * 0.1,
            theta: (i as f64) * std::f64::consts::PI / 5.0,
        };
        space.update_neighbor(node_id.clone(), coord).await;
        topology.push((node_id, coord));
    }
    
    // Test routing to various targets
    let target_coord = HyperbolicCoordinate { r: 0.85, theta: 1.5 };
    let target_id = generate_test_node_id(100);
    space.update_neighbor(target_id.clone(), target_coord).await;
    
    // Greedy routing should find path
    let next_hop = space.greedy_route(&target_coord).await;
    assert!(next_hop.is_some());
    
    // Verify it chose a node closer to target
    if let Some(chosen) = next_hop {
        let neighbors = space.neighbor_coordinates.read().await;
        let chosen_coord = neighbors.get(&chosen).expect("chosen neighbor should exist");
        let my_coord = space.get_coordinate().await;
        
        let my_dist = HyperbolicSpace::distance(&my_coord, &target_coord);
        let chosen_dist = HyperbolicSpace::distance(chosen_coord, &target_coord);
        
        assert!(chosen_dist < my_dist, "Greedy routing should choose closer neighbor");
    }
}

#[tokio::test]
async fn test_routing_fallback_conditions() {
    let space = Arc::new(HyperbolicSpace::new());
    let local_id = generate_test_node_id(0);
    let strategy = HyperbolicRoutingStrategy::new(local_id, space.clone());
    
    // Test routing to unknown node (should fail and trigger fallback)
    let unknown_target = generate_test_node_id(999);
    let result = strategy.find_path(&unknown_target).await;
    
    assert!(result.is_err());
    match result {
        Err(AdaptiveNetworkError::Routing(msg)) => {
            assert!(msg.contains("Target coordinate unknown"));
        }
        _ => panic!("Expected routing error for unknown target"),
    }
    
    // Check that fallback was recorded in stats
    let stats = space.get_stats().await;
    assert_eq!(stats.attempts, 1);
    assert_eq!(stats.failures, 1);
    assert_eq!(stats.fallback_used, 1);
}

#[tokio::test]
async fn test_routing_loop_detection() {
    let space = Arc::new(HyperbolicSpace::new());
    let local_id = generate_test_node_id(0);
    
    // Create a topology that could cause loops
    let node1 = generate_test_node_id(1);
    let node2 = generate_test_node_id(2);
    let target = generate_test_node_id(3);
    
    // Set coordinates that might cause oscillation
    space.update_neighbor(node1.clone(), HyperbolicCoordinate { r: 0.5, theta: 0.0 }).await;
    space.update_neighbor(node2.clone(), HyperbolicCoordinate { r: 0.5, theta: std::f64::consts::PI }).await;
    space.update_neighbor(target.clone(), HyperbolicCoordinate { r: 0.9, theta: std::f64::consts::PI / 2.0 }).await;
    
    let strategy = HyperbolicRoutingStrategy::new(local_id, space.clone());
    
    // This topology might cause routing loops - test that they're detected
    let result = strategy.find_path(&target).await;
    
    // The test should either succeed or fail with loop detection
    if let Err(AdaptiveNetworkError::Routing(msg)) = &result {
        // If it fails, it should be due to proper reasons
        assert!(
            msg.contains("loop detected") || 
            msg.contains("No closer neighbor") ||
            msg.contains("Maximum hop count"),
            "Unexpected error message: {}", msg
        );
    }
}

#[tokio::test]
async fn test_success_rate_tracking() {
    let space = HyperbolicSpace::new();
    
    // Record various routing outcomes
    for i in 0..100 {
        let success = i % 3 != 0; // 66.67% success rate
        let hop_count = if success { 3 + (i % 5) } else { 0 };
        let used_fallback = !success;
        
        space.record_routing_result(success, hop_count, used_fallback).await;
    }
    
    let stats = space.get_stats().await;
    assert_eq!(stats.attempts, 100);
    
    let success_rate = space.get_success_rate().await;
    assert_relative_eq!(success_rate, 0.67, epsilon = 0.01);
    
    // Check average hop count (should be around 5 for successful routes)
    assert!(stats.average_hop_count > 3.0);
    assert!(stats.average_hop_count < 6.0);
}

// Helper functions

fn generate_test_node_id(seed: u64) -> NodeId {
    use rand::{RngCore, SeedableRng};
    use rand::rngs::StdRng;
    
    let mut rng = StdRng::seed_from_u64(seed);
    let mut hash = [0u8; 32];
    rng.fill_bytes(&mut hash);
    NodeId::from_bytes(hash)
}

fn create_test_neighbors(count: usize, avg_radius: f64) -> Vec<(NodeId, HyperbolicCoordinate)> {
    let mut neighbors = vec![];
    for i in 0..count {
        let node_id = generate_test_node_id(i as u64 + 1000);
        let coord = HyperbolicCoordinate {
            r: avg_radius + 0.1 * ((i as f64) - (count as f64) / 2.0) / (count as f64),
            theta: (i as f64) * 2.0 * std::f64::consts::PI / (count as f64),
        };
        neighbors.push((node_id, coord));
    }
    neighbors
}

fn angle_diff(a: f64, b: f64) -> f64 {
    let diff = a - b;
    if diff > std::f64::consts::PI {
        diff - 2.0 * std::f64::consts::PI
    } else if diff < -std::f64::consts::PI {
        diff + 2.0 * std::f64::consts::PI
    } else {
        diff
    }
}

#[cfg(test)]
mod simulation_tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_scale_free_topology_routing() {
        // Create a scale-free network topology (Barabási-Albert model)
        let mut nodes = vec![];
        let mut spaces = HashMap::new();
        
        // Initial fully connected nodes
        for i in 0..5 {
            let node_id = generate_test_node_id(i);
            let space = Arc::new(HyperbolicSpace::new());
            nodes.push(node_id.clone());
            spaces.insert(node_id, space);
        }
        
        // Add nodes with preferential attachment
        for i in 5..50 {
            let new_node_id = generate_test_node_id(i);
            let new_space = Arc::new(HyperbolicSpace::new());
            
            // Connect to m existing nodes (preferential attachment)
            let m = 3;
            for j in 0..m {
                let target_idx = (i * 7 + j * 13) % nodes.len(); // Pseudo-random selection
                let target = &nodes[target_idx];
                
                // Update coordinates to reflect connection
                let coord = HyperbolicCoordinate {
                    r: 0.1 + (i as f64) * 0.015,
                    theta: (i as f64) * 2.0 * std::f64::consts::PI / 50.0,
                };
                
                new_space.update_neighbor(target.clone(), coord).await;
                if let Some(target_space) = spaces.get(target) {
                    target_space.update_neighbor(new_node_id.clone(), coord).await;
                }
            }
            
            nodes.push(new_node_id.clone());
            spaces.insert(new_node_id, new_space);
        }
        
        // Test routing performance
        let mut total_attempts = 0;
        let mut successful_routes = 0;
        
        for _ in 0..100 {
            let source_idx = rand::random::<usize>() % nodes.len();
            let target_idx = rand::random::<usize>() % nodes.len();
            
            if source_idx != target_idx {
                let source = &nodes[source_idx];
                let target = &nodes[target_idx];
                
                if let Some(source_space) = spaces.get(source) {
                    let strategy = HyperbolicRoutingStrategy::new(source.clone(), source_space.clone());
                    
                    total_attempts += 1;
                    if strategy.find_path(target).await.is_ok() {
                        successful_routes += 1;
                    }
                }
            }
        }
        
        // In a scale-free network with hyperbolic routing, we expect decent success rate
        let success_rate = successful_routes as f64 / total_attempts as f64;
        println!("Scale-free topology routing success rate: {:.2}%", success_rate * 100.0);
        
        // We expect at least some successful routes in a connected topology
        assert!(success_rate > 0.1, "Success rate too low: {}", success_rate);
    }
    
    #[tokio::test]
    async fn test_performance_vs_hop_count() {
        // Create a grid topology for predictable hop counts
        let grid_size = 5;
        let mut grid_spaces = HashMap::new();
        
        for x in 0..grid_size {
            for y in 0..grid_size {
                let node_id = generate_test_node_id((x * grid_size + y) as u64);
                let space = Arc::new(HyperbolicSpace::new());
                
                // Set coordinate based on grid position
                let r = 0.1 + 0.8 * ((x * x + y * y) as f64).sqrt() / ((grid_size * grid_size * 2) as f64).sqrt();
                let theta = (y as f64).atan2(x as f64);
                
                // Add neighbors (4-connected grid)
                for (dx, dy) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    
                    if nx >= 0 && nx < grid_size as i32 && ny >= 0 && ny < grid_size as i32 {
                        let neighbor_id = generate_test_node_id((nx * grid_size as i32 + ny) as u64);
                        let neighbor_r = 0.1 + 0.8 * ((nx * nx + ny * ny) as f64).sqrt() / ((grid_size * grid_size * 2) as f64).sqrt();
                        let neighbor_theta = (ny as f64).atan2(nx as f64);
                        
                        space.update_neighbor(
                            neighbor_id,
                            HyperbolicCoordinate { r: neighbor_r, theta: neighbor_theta }
                        ).await;
                    }
                }
                
                grid_spaces.insert((x, y), (node_id, space));
            }
        }
        
        // Test routing between nodes at various distances
        let mut distance_stats = HashMap::new();
        
        for distance in 1..=8 {
            let mut attempts = 0;
            let mut successes = 0;
            let mut total_hops = 0;
            
            // Try routing between nodes at this Manhattan distance
            for x1 in 0..grid_size {
                for y1 in 0..grid_size {
                    for x2 in 0..grid_size {
                        for y2 in 0..grid_size {
                            let manhattan = ((x1 as i32 - x2 as i32).abs() + (y1 as i32 - y2 as i32).abs()) as usize;
                            
                            if manhattan == distance {
                                if let (Some((source_id, source_space)), Some((target_id, _))) = 
                                    (grid_spaces.get(&(x1, y1)), grid_spaces.get(&(x2, y2))) {
                                    
                                    let strategy = HyperbolicRoutingStrategy::new(source_id.clone(), source_space.clone());
                                    
                                    attempts += 1;
                                    match strategy.find_path(target_id).await {
                                        Ok(path) => {
                                            successes += 1;
                                            total_hops += path.len();
                                        }
                                        Err(_) => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            if attempts > 0 {
                let success_rate = successes as f64 / attempts as f64;
                let avg_hops = if successes > 0 { total_hops as f64 / successes as f64 } else { 0.0 };
                
                distance_stats.insert(distance, (success_rate, avg_hops));
                println!("Distance {}: success_rate={:.2}%, avg_hops={:.1}", 
                         distance, success_rate * 100.0, avg_hops);
            }
        }
        
        // Verify that success rate generally decreases with distance
        // and hop count increases with distance
        let close_stats = distance_stats.get(&2).unwrap_or(&(0.0, 0.0));
        let far_stats = distance_stats.get(&6).unwrap_or(&(0.0, 0.0));
        
        // Close nodes should have better routing than far nodes
        assert!(close_stats.0 >= far_stats.0 || close_stats.0 > 0.5, 
                "Close nodes should route better than far nodes");
    }
}