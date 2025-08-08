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

use proptest::prelude::*;
use saorsa_core::adaptive::som::{GridSize, NodeFeatures, SelfOrganizingMap, SomConfig};
use saorsa_core::identity::NodeId;
// use std::collections::HashSet;
use std::time::Instant;

/// Create test features with predictable values
fn create_test_features(seed: u8) -> NodeFeatures {
    NodeFeatures {
        content_vector: vec![seed as f64 / 255.0; 128],
        compute_capability: (seed as f64 * 4.0) % 1000.0,
        network_latency: (seed as f64 * 0.5) % 200.0,
        storage_available: (seed as f64 * 10.0) % 5000.0,
    }
}

/// Create a test node ID
fn create_test_node_id(seed: u8) -> NodeId {
    let mut bytes = [0u8; 32];
    bytes[0] = seed;
    NodeId::from_bytes(&bytes).unwrap()
}

#[cfg(test)]
mod feature_tests {
    use super::*;

    #[test]
    fn test_feature_normalization() {
        let features = NodeFeatures {
            content_vector: vec![0.0, 0.5, 1.0, 2.0],
            compute_capability: 500.0,
            network_latency: 100.0,
            storage_available: 2500.0,
        };

        let normalized = features.normalize();

        // Check content vector normalization (should be unit vector)
        let magnitude: f64 = normalized
            .content_vector
            .iter()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt();
        assert!(
            (magnitude - 1.0).abs() < 1e-6,
            "Content vector should be unit length"
        );

        // Check other features are in [0, 1] range
        assert!(normalized.compute_capability >= 0.0 && normalized.compute_capability <= 1.0);
        assert!(normalized.network_latency >= 0.0 && normalized.network_latency <= 1.0);
        assert!(normalized.storage_available >= 0.0 && normalized.storage_available <= 1.0);
    }

    #[test]
    fn test_edge_case_normalization() {
        // Test with zero content vector
        let features = NodeFeatures {
            content_vector: vec![0.0; 128],
            compute_capability: 0.0,
            network_latency: 0.0,
            storage_available: 0.0,
        };

        let normalized = features.normalize();

        // Zero vector should remain zero
        assert!(normalized.content_vector.iter().all(|&x| x == 0.0));
        assert_eq!(normalized.compute_capability, 0.0);
        assert_eq!(normalized.network_latency, 0.0);
        assert_eq!(normalized.storage_available, 0.0);
    }

    #[test]
    fn test_feature_distance() {
        let features1 = NodeFeatures {
            content_vector: vec![1.0, 0.0, 0.0, 0.0],
            compute_capability: 100.0,
            network_latency: 50.0,
            storage_available: 1000.0,
        };

        let features2 = NodeFeatures {
            content_vector: vec![0.0, 1.0, 0.0, 0.0],
            compute_capability: 200.0,
            network_latency: 100.0,
            storage_available: 2000.0,
        };

        let distance = features1.euclidean_distance(&features2);
        assert!(
            distance > 0.0,
            "Different features should have positive distance"
        );

        let self_distance = features1.euclidean_distance(&features1);
        assert_eq!(self_distance, 0.0, "Distance to self should be zero");
    }

    proptest! {
        #[test]
        fn prop_normalization_preserves_ratios(
            v1 in 0.0..1000.0,
            v2 in 0.0..1000.0,
            v3 in 0.0..1000.0,
            v4 in 0.0..1000.0,
        ) {
            prop_assume!(v1 > 0.0 || v2 > 0.0 || v3 > 0.0 || v4 > 0.0);

            let features = NodeFeatures {
                content_vector: vec![v1, v2, v3, v4],
                compute_capability: 500.0,
                network_latency: 100.0,
                storage_available: 2000.0,
            };

            let normalized = features.normalize();

            // Check that normalization preserves relative magnitudes
            if v1 > 0.0 && v2 > 0.0 {
                let original_ratio = v1 / v2;
                let normalized_ratio = normalized.content_vector[0] / normalized.content_vector[1];
                prop_assert!((original_ratio - normalized_ratio).abs() < 1e-6);
            }
        }
    }
}

#[cfg(test)]
mod bmu_tests {
    use super::*;

    #[test]
    fn test_bmu_consistency() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 100,
            grid_size: GridSize::Fixed(10, 10),
        };

        let som = SelfOrganizingMap::new(config);
        let features = create_test_features(42);

        let bmu1 = som.find_best_matching_unit(&features);
        let bmu2 = som.find_best_matching_unit(&features);

        assert_eq!(bmu1, bmu2, "Same input should always give same BMU");
    }

    #[test]
    fn test_bmu_finds_closest() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 100,
            grid_size: GridSize::Fixed(5, 5),
        };

        let mut som = SelfOrganizingMap::new(config);

        // Set one neuron to have specific weights
        let target_features = create_test_features(100);
        som.set_neuron_weights(2, 2, target_features.to_weight_vector());

        // Find BMU for similar features
        let similar_features = create_test_features(101);
        let (bmu_x, bmu_y) = som.find_best_matching_unit(&similar_features);

        // BMU should be the neuron we set
        assert_eq!((bmu_x, bmu_y), (2, 2), "BMU should be the closest neuron");
    }

    proptest! {
        #[test]
        fn prop_bmu_within_grid_bounds(seed in 0u8..255u8) {
            let config = SomConfig {
                initial_learning_rate: 0.1,
                initial_radius: 5.0,
                iterations: 100,
                grid_size: GridSize::Fixed(10, 10),
            };

            let som = SelfOrganizingMap::new(config);
            let features = create_test_features(seed);

            let (x, y) = som.find_best_matching_unit(&features);

            prop_assert!(x < 10, "BMU x coordinate should be within grid");
            prop_assert!(y < 10, "BMU y coordinate should be within grid");
        }
    }

    #[test]
    fn test_bmu_performance() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 100,
            grid_size: GridSize::Fixed(32, 32), // 1024 neurons
        };

        let som = SelfOrganizingMap::new(config);
        let features = create_test_features(42);

        let start = Instant::now();
        for _ in 0..100 {
            som.find_best_matching_unit(&features);
        }
        let elapsed = start.elapsed();

        let avg_time = elapsed.as_micros() as f64 / 100.0;
        println!("Average BMU search time: {:.2} μs", avg_time);

        assert!(
            avg_time < 1000.0,
            "BMU search should complete in < 1ms for 1000 neurons"
        );
    }
}

#[cfg(test)]
mod neighborhood_tests {
    use super::*;

    #[test]
    fn test_gaussian_neighborhood() {
        let radius = 5.0;

        // Test at different distances
        let influence_0 = SelfOrganizingMap::gaussian_neighborhood(0.0, radius);
        let influence_1 = SelfOrganizingMap::gaussian_neighborhood(1.0, radius);
        let influence_5 = SelfOrganizingMap::gaussian_neighborhood(5.0, radius);
        let influence_10 = SelfOrganizingMap::gaussian_neighborhood(10.0, radius);

        // Influence should be 1 at distance 0
        assert!((influence_0 - 1.0).abs() < 1e-6);

        // Influence should decrease with distance
        assert!(influence_1 < influence_0);
        assert!(influence_5 < influence_1);
        assert!(influence_10 < influence_5);

        // All influences should be positive
        assert!(influence_1 > 0.0);
        assert!(influence_5 > 0.0);
        assert!(influence_10 > 0.0);
    }

    #[test]
    fn test_neighborhood_radius_decay() {
        let config = SomConfig {
            initial_learning_rate: 0.5,
            initial_radius: 10.0,
            iterations: 1000,
            grid_size: GridSize::Fixed(10, 10),
        };

        let som = SelfOrganizingMap::new(config);

        let radius_0 = som.get_neighborhood_radius(0);
        let radius_500 = som.get_neighborhood_radius(500);
        let radius_999 = som.get_neighborhood_radius(999);

        // Radius should decay over iterations
        assert_eq!(radius_0, 10.0, "Initial radius should be as configured");
        assert!(radius_500 < radius_0, "Radius should decay");
        assert!(radius_999 < radius_500, "Radius should continue decaying");
        assert!(radius_999 > 0.0, "Radius should remain positive");
    }

    proptest! {
        #[test]
        fn prop_neighborhood_function_properties(
            distance in 0.0..20.0,
            radius in 1.0..10.0,
        ) {
            let influence = SelfOrganizingMap::gaussian_neighborhood(distance, radius);

            // Influence should be in [0, 1]
            prop_assert!(influence >= 0.0);
            prop_assert!(influence <= 1.0);

            // Influence should be inversely related to distance
            if distance < radius {
                let smaller_distance = distance * 0.5;
                let smaller_influence = SelfOrganizingMap::gaussian_neighborhood(smaller_distance, radius);
                prop_assert!(smaller_influence >= influence);
            }
        }
    }
}

#[cfg(test)]
mod learning_tests {
    use super::*;

    #[test]
    fn test_weight_update() {
        let config = SomConfig {
            initial_learning_rate: 0.5,
            initial_radius: 5.0,
            iterations: 100,
            grid_size: GridSize::Fixed(5, 5),
        };

        let mut som = SelfOrganizingMap::new(config);
        let features = create_test_features(42);

        // Get initial weights of BMU
        let (bmu_x, bmu_y) = som.find_best_matching_unit(&features);
        let initial_weights = som.get_neuron_weights(bmu_x, bmu_y).unwrap();

        // Train with one sample
        som.train_single(&features, 0);

        // Weights should have moved towards the input
        let updated_weights = som.get_neuron_weights(bmu_x, bmu_y).unwrap();

        // Calculate if weights moved closer to input
        let initial_distance = calculate_distance(&initial_weights, &features.to_weight_vector());
        let updated_distance = calculate_distance(&updated_weights, &features.to_weight_vector());

        assert!(
            updated_distance < initial_distance,
            "Weights should move towards input"
        );
    }

    #[test]
    fn test_learning_rate_decay() {
        let config = SomConfig {
            initial_learning_rate: 0.5,
            initial_radius: 10.0,
            iterations: 1000,
            grid_size: GridSize::Fixed(10, 10),
        };

        let som = SelfOrganizingMap::new(config);

        let lr_0 = som.get_learning_rate(0);
        let lr_500 = som.get_learning_rate(500);
        let lr_999 = som.get_learning_rate(999);

        // Learning rate should decay
        assert_eq!(lr_0, 0.5, "Initial learning rate should be as configured");
        assert!(lr_500 < lr_0, "Learning rate should decay");
        assert!(lr_999 < lr_500, "Learning rate should continue decaying");
        assert!(lr_999 > 0.0, "Learning rate should remain positive");
    }

    #[test]
    fn test_batch_training() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 50,
            grid_size: GridSize::Fixed(10, 10),
        };

        let mut som = SelfOrganizingMap::new(config);

        // Create training data with clusters
        let mut training_data = Vec::new();
        for i in 0..20 {
            training_data.push(create_test_features(i));
            training_data.push(create_test_features(100 + i));
            training_data.push(create_test_features(200 + i));
        }

        som.train_batch(&training_data);

        // Test that similar features map to nearby neurons
        let cluster1_bmu = som.find_best_matching_unit(&create_test_features(10));
        let cluster1_bmu2 = som.find_best_matching_unit(&create_test_features(11));

        let distance = ((cluster1_bmu.0 as i32 - cluster1_bmu2.0 as i32).abs()
            + (cluster1_bmu.1 as i32 - cluster1_bmu2.1 as i32).abs()) as u32;

        assert!(
            distance <= 2,
            "Similar features should map to nearby neurons"
        );
    }

    fn calculate_distance(v1: &[f64], v2: &[f64]) -> f64 {
        v1.iter()
            .zip(v2.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }
}

#[cfg(test)]
mod node_assignment_tests {
    use super::*;

    #[test]
    fn test_node_assignment() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 10,
            grid_size: GridSize::Fixed(5, 5),
        };

        let mut som = SelfOrganizingMap::new(config);

        let node_id = create_test_node_id(42);
        let features = create_test_features(42);

        // Assign node to SOM
        som.assign_node(node_id.clone(), features);

        // Retrieve nodes for the BMU
        let (bmu_x, bmu_y) = som.find_best_matching_unit(&create_test_features(42));
        let assigned_nodes = som.get_assigned_nodes(bmu_x, bmu_y);

        assert!(
            assigned_nodes.contains(&node_id),
            "Node should be assigned to its BMU"
        );
    }

    #[test]
    fn test_node_reassignment() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 10,
            grid_size: GridSize::Fixed(5, 5),
        };

        let mut som = SelfOrganizingMap::new(config);

        let node_id = create_test_node_id(42);

        // First assignment
        som.assign_node(node_id.clone(), create_test_features(10));
        let (old_x, old_y) = som.find_best_matching_unit(&create_test_features(10));

        // Reassign to different features
        som.assign_node(node_id.clone(), create_test_features(200));
        let (new_x, new_y) = som.find_best_matching_unit(&create_test_features(200));

        // Check old neuron no longer has the node
        let old_nodes = som.get_assigned_nodes(old_x, old_y);
        assert!(
            !old_nodes.contains(&node_id),
            "Node should be removed from old neuron"
        );

        // Check new neuron has the node
        let new_nodes = som.get_assigned_nodes(new_x, new_y);
        assert!(
            new_nodes.contains(&node_id),
            "Node should be assigned to new neuron"
        );
    }

    #[test]
    fn test_find_similar_nodes() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 50,
            grid_size: GridSize::Fixed(10, 10),
        };

        let mut som = SelfOrganizingMap::new(config);

        // Train SOM first
        let mut training_data = Vec::new();
        for i in 0..50 {
            training_data.push(create_test_features(i));
        }
        som.train_batch(&training_data);

        // Assign nodes with similar features
        for i in 0..10 {
            let node_id = create_test_node_id(i);
            let features = create_test_features(i);
            som.assign_node(node_id, features);
        }

        // Find similar nodes
        let query_features = create_test_features(5);
        let similar_nodes = som.find_similar_nodes(&query_features, 2);

        assert!(!similar_nodes.is_empty(), "Should find similar nodes");
        assert!(
            similar_nodes.len() <= 10,
            "Should not return more nodes than assigned"
        );
    }
}

#[cfg(test)]
mod grid_sizing_tests {
    use super::*;

    #[test]
    fn test_dynamic_grid_sizing() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 10,
            grid_size: GridSize::Dynamic { min: 5, max: 20 },
        };

        let mut som = SelfOrganizingMap::new(config);

        // Add many nodes
        for i in 0..100 {
            let node_id = create_test_node_id(i);
            let features = create_test_features(i);
            som.assign_node(node_id, features);
        }

        // Grid should have grown
        let (width, height) = som.get_grid_dimensions();
        assert!(width > 5 || height > 5, "Grid should grow with more nodes");
        assert!(
            width <= 20 && height <= 20,
            "Grid should respect maximum size"
        );
    }

    #[test]
    fn test_grid_resize_preserves_assignments() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 10,
            grid_size: GridSize::Dynamic { min: 5, max: 20 },
        };

        let mut som = SelfOrganizingMap::new(config);

        // Assign some nodes
        let mut node_ids = Vec::new();
        for i in 0..20 {
            let node_id = create_test_node_id(i);
            let features = create_test_features(i);
            som.assign_node(node_id.clone(), features);
            node_ids.push(node_id);
        }

        // Trigger resize by adding more nodes
        for i in 20..50 {
            let node_id = create_test_node_id(i);
            let features = create_test_features(i);
            som.assign_node(node_id, features);
        }

        // Check all original nodes are still assigned
        let all_assigned = som.get_all_assigned_nodes();
        for node_id in &node_ids {
            assert!(
                all_assigned.contains(node_id),
                "Original nodes should remain assigned after resize"
            );
        }
    }
}

#[cfg(test)]
mod visualization_tests {
    use super::*;

    #[test]
    fn test_grid_visualization() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 50,
            grid_size: GridSize::Fixed(10, 10),
        };

        let mut som = SelfOrganizingMap::new(config);

        // Train and assign nodes
        let mut training_data = Vec::new();
        for i in 0..30 {
            let features = create_test_features(i);
            training_data.push(features.clone());

            let node_id = create_test_node_id(i);
            som.assign_node(node_id, features);
        }
        som.train_batch(&training_data);

        // Get visualization data
        let viz_data = som.get_visualization_data();

        assert_eq!(viz_data.grid_width, 10);
        assert_eq!(viz_data.grid_height, 10);
        assert_eq!(viz_data.neurons.len(), 100);

        // Check that neurons with assigned nodes are marked
        let assigned_count = viz_data
            .neurons
            .iter()
            .filter(|n| !n.assigned_nodes.is_empty())
            .count();
        assert!(
            assigned_count > 0,
            "Some neurons should have assigned nodes"
        );
    }

    #[test]
    fn test_u_matrix_generation() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 5.0,
            iterations: 50,
            grid_size: GridSize::Fixed(5, 5),
        };

        let mut som = SelfOrganizingMap::new(config);

        // Train SOM
        let mut training_data = Vec::new();
        for i in 0..25 {
            training_data.push(create_test_features(i));
        }
        som.train_batch(&training_data);

        // Generate U-Matrix
        let u_matrix = som.generate_u_matrix();

        assert_eq!(u_matrix.len(), 5);
        assert_eq!(u_matrix[0].len(), 5);

        // All values should be non-negative
        for row in &u_matrix {
            for &value in row {
                assert!(value >= 0.0, "U-Matrix values should be non-negative");
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_som_workflow() {
        let config = SomConfig {
            initial_learning_rate: 0.2,
            initial_radius: 8.0,
            iterations: 100,
            grid_size: GridSize::Fixed(15, 15),
        };

        let mut som = SelfOrganizingMap::new(config);

        // Generate synthetic network data with 3 distinct clusters
        let mut all_features = Vec::new();
        let mut all_nodes = Vec::new();

        // Cluster 1: High compute, low latency (compute nodes)
        for i in 0..20 {
            let mut features = create_test_features(i);
            features.compute_capability = 800.0 + (i as f64 * 10.0);
            features.network_latency = 10.0 + (i as f64 * 0.5);
            all_features.push(features.clone());

            let node_id = create_test_node_id(i);
            all_nodes.push((node_id.clone(), features));
        }

        // Cluster 2: High storage, moderate latency (storage nodes)
        for i in 20..40 {
            let mut features = create_test_features(i);
            features.storage_available = 4000.0 + (i as f64 * 50.0);
            features.network_latency = 50.0 + (i as f64 * 2.0);
            all_features.push(features.clone());

            let node_id = create_test_node_id(i);
            all_nodes.push((node_id.clone(), features));
        }

        // Cluster 3: Balanced features (general nodes)
        for i in 40..60 {
            let features = create_test_features(i);
            all_features.push(features.clone());

            let node_id = create_test_node_id(i);
            all_nodes.push((node_id.clone(), features));
        }

        // Train the SOM
        som.train_batch(&all_features);

        // Assign all nodes
        for (node_id, features) in all_nodes {
            som.assign_node(node_id, features);
        }

        // Verify clustering quality
        // Nodes from same cluster should map to nearby neurons
        let compute_node_bmus: Vec<_> = (0..5)
            .map(|i| som.find_best_matching_unit(&create_test_features(i)))
            .collect();

        let storage_node_bmus: Vec<_> = (20..25)
            .map(|i| som.find_best_matching_unit(&create_test_features(i)))
            .collect();

        // Calculate average intra-cluster distance
        let compute_cluster_spread = calculate_cluster_spread(&compute_node_bmus);
        let storage_cluster_spread = calculate_cluster_spread(&storage_node_bmus);

        // Clusters should be relatively compact
        assert!(
            compute_cluster_spread < 5.0,
            "Compute nodes should cluster together"
        );
        assert!(
            storage_cluster_spread < 5.0,
            "Storage nodes should cluster together"
        );

        // Different clusters should be separated
        let inter_cluster_distance = calculate_min_distance(&compute_node_bmus, &storage_node_bmus);
        assert!(
            inter_cluster_distance > 2.0,
            "Different clusters should be separated"
        );
    }

    fn calculate_cluster_spread(positions: &[(usize, usize)]) -> f64 {
        if positions.is_empty() {
            return 0.0;
        }

        let center_x =
            positions.iter().map(|(x, _)| *x as f64).sum::<f64>() / positions.len() as f64;
        let center_y =
            positions.iter().map(|(_, y)| *y as f64).sum::<f64>() / positions.len() as f64;

        positions
            .iter()
            .map(|(x, y)| {
                let dx = *x as f64 - center_x;
                let dy = *y as f64 - center_y;
                (dx * dx + dy * dy).sqrt()
            })
            .sum::<f64>()
            / positions.len() as f64
    }

    fn calculate_min_distance(cluster1: &[(usize, usize)], cluster2: &[(usize, usize)]) -> f64 {
        let mut min_distance = f64::MAX;

        for (x1, y1) in cluster1 {
            for (x2, y2) in cluster2 {
                let dx = *x1 as f64 - *x2 as f64;
                let dy = *y1 as f64 - *y2 as f64;
                let distance = (dx * dx + dy * dy).sqrt();
                min_distance = min_distance.min(distance);
            }
        }

        min_distance
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_training_performance() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 10.0,
            iterations: 100,
            grid_size: GridSize::Fixed(20, 20), // 400 neurons
        };

        let mut som = SelfOrganizingMap::new(config);

        // Generate 1000 training samples
        let mut training_data = Vec::new();
        for i in 0..1000 {
            training_data.push(create_test_features((i % 256) as u8));
        }

        let start = Instant::now();
        som.train_batch(&training_data);
        let elapsed = start.elapsed();

        println!("Training 1000 samples on 20x20 SOM took: {:?}", elapsed);
        assert!(
            elapsed.as_secs() < 10,
            "Training should complete within 10 seconds"
        );
    }

    #[test]
    fn benchmark_query_performance() {
        let config = SomConfig {
            initial_learning_rate: 0.1,
            initial_radius: 10.0,
            iterations: 50,
            grid_size: GridSize::Fixed(30, 30), // 900 neurons
        };

        let mut som = SelfOrganizingMap::new(config);

        // Train and populate SOM
        let mut training_data = Vec::new();
        for i in 0..500 {
            let features = create_test_features((i % 256) as u8);
            training_data.push(features.clone());

            let node_id = create_test_node_id((i % 256) as u8);
            som.assign_node(node_id, features);
        }
        som.train_batch(&training_data);

        // Benchmark similarity queries
        let query_features = create_test_features(128);
        let start = Instant::now();
        for _ in 0..1000 {
            som.find_similar_nodes(&query_features, 5);
        }
        let elapsed = start.elapsed();

        let avg_query_time = elapsed.as_micros() as f64 / 1000.0;
        println!("Average similarity query time: {:.2} μs", avg_query_time);
        assert!(avg_query_time < 1000.0, "Queries should complete in < 1ms");
    }
}
