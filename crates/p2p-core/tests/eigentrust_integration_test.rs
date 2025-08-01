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

//! Comprehensive integration tests for EigenTrust++ implementation
//!
//! Tests cover:
//! - Local trust calculation and normalization
//! - Global trust computation and convergence
//! - Pre-trusted node handling
//! - Trust decay over time
//! - Trust inheritance for new nodes
//! - Trust-based routing decisions
//! - Attack resistance (Sybil, collusion)
//! - Performance and scalability

#[cfg(test)]
mod eigentrust_tests {
    use p2p_core::adaptive::trust::*;
    use p2p_core::identity::NodeId;
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    use proptest::prelude::*;
    use rand::Rng;
    
    /// Helper to create test nodes
    fn create_test_nodes(count: usize) -> Vec<NodeId> {
        (0..count)
            .map(|i| {
                let mut hash = [0u8; 32];
                hash[0] = i as u8;
                NodeId::from_bytes(hash)
            })
            .collect()
    }
    
    /// Helper to create pre-trusted nodes
    fn create_pre_trusted_nodes(nodes: &[NodeId], count: usize) -> HashSet<NodeId> {
        nodes.iter().take(count).cloned().collect()
    }
    
    #[tokio::test]
    async fn test_local_trust_calculation() {
        let nodes = create_test_nodes(3);
        let engine = EigenTrustEngine::new(HashSet::new());
        
        // Update local trust with various interactions
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[0], &nodes[1], false).await;
        engine.update_local_trust(&nodes[0], &nodes[2], true).await;
        
        let local_trust = engine.local_trust.read().await;
        let trust_01 = local_trust.get(&(nodes[0].clone(), nodes[1].clone()))
            .expect("Should have trust data");
        let trust_02 = local_trust.get(&(nodes[0].clone(), nodes[2].clone()))
            .expect("Should have trust data");
        
        // Trust should be weighted average: 0.9 * 0.9 + 0.1 * 0 = 0.81
        assert!((trust_01.value - 0.81).abs() < 0.01);
        assert_eq!(trust_01.interactions, 3);
        
        // Trust should be 1.0 for single positive interaction
        assert_eq!(trust_02.value, 1.0);
        assert_eq!(trust_02.interactions, 1);
    }
    
    #[tokio::test]
    async fn test_trust_normalization() {
        let nodes = create_test_nodes(4);
        let engine = EigenTrustEngine::new(HashSet::new());
        
        // Node 0 trusts three other nodes
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[0], &nodes[2], true).await;
        engine.update_local_trust(&nodes[0], &nodes[3], true).await;
        
        let local_trust = engine.local_trust.read().await;
        
        // All outgoing trust should normalize to 1/3
        let norm_01 = engine.get_normalized_trust(&local_trust, &nodes[0], &nodes[1])
            .expect("Should have normalized trust");
        let norm_02 = engine.get_normalized_trust(&local_trust, &nodes[0], &nodes[2])
            .expect("Should have normalized trust");
        let norm_03 = engine.get_normalized_trust(&local_trust, &nodes[0], &nodes[3])
            .expect("Should have normalized trust");
        
        assert!((norm_01 - 1.0/3.0).abs() < 0.001);
        assert!((norm_02 - 1.0/3.0).abs() < 0.001);
        assert!((norm_03 - 1.0/3.0).abs() < 0.001);
        
        // Sum should be 1.0
        assert!((norm_01 + norm_02 + norm_03 - 1.0).abs() < 0.001);
    }
    
    #[tokio::test]
    async fn test_global_trust_computation() {
        let nodes = create_test_nodes(5);
        let pre_trusted = create_pre_trusted_nodes(&nodes, 1);
        let engine = EigenTrustEngine::new(pre_trusted.clone());
        
        // Build trust network
        // Pre-trusted node trusts node 1
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        // Node 1 trusts nodes 2 and 3
        engine.update_local_trust(&nodes[1], &nodes[2], true).await;
        engine.update_local_trust(&nodes[1], &nodes[3], true).await;
        // Node 2 trusts node 4
        engine.update_local_trust(&nodes[2], &nodes[4], true).await;
        // Create a cycle: node 3 trusts node 1
        engine.update_local_trust(&nodes[3], &nodes[1], true).await;
        
        let global_trust = engine.compute_global_trust().await;
        
        // Pre-trusted node should have highest trust
        let pre_trust = global_trust.get(&nodes[0]).expect("Should have trust");
        assert!(pre_trust > &0.1, "Pre-trusted node should have high trust");
        
        // Trust should propagate through network
        let trust_1 = global_trust.get(&nodes[1]).expect("Should have trust");
        let trust_2 = global_trust.get(&nodes[2]).expect("Should have trust");
        let trust_3 = global_trust.get(&nodes[3]).expect("Should have trust");
        let trust_4 = global_trust.get(&nodes[4]).expect("Should have trust");
        
        // Node 1 should have higher trust than nodes 2,3 (directly trusted by pre-trusted)
        assert!(trust_1 > trust_2);
        assert!(trust_1 > trust_3);
        
        // Nodes 2,3 should have higher trust than node 4 (further from pre-trusted)
        assert!(trust_2 > trust_4);
        assert!(trust_3 > trust_4);
        
        // All trust values should sum to 1.0
        let total_trust: f64 = global_trust.values().sum();
        assert!((total_trust - 1.0).abs() < 0.001);
    }
    
    #[tokio::test]
    async fn test_trust_convergence() {
        let nodes = create_test_nodes(10);
        let engine = EigenTrustEngine::new(HashSet::new());
        
        // Create random trust relationships
        let mut rng = rand::thread_rng();
        for _ in 0..30 {
            let from = rng.gen_range(0..10);
            let to = rng.gen_range(0..10);
            if from != to {
                let success = rng.gen_bool(0.7);
                engine.update_local_trust(&nodes[from], &nodes[to], success).await;
            }
        }
        
        // Compute trust multiple times to check convergence
        let trust1 = engine.compute_global_trust().await;
        let trust2 = engine.compute_global_trust().await;
        
        // Trust values should be identical (converged)
        for node in &nodes {
            let t1 = trust1.get(node).copied().unwrap_or(0.0);
            let t2 = trust2.get(node).copied().unwrap_or(0.0);
            assert!((t1 - t2).abs() < 0.0001, "Trust should converge");
        }
    }
    
    #[tokio::test]
    async fn test_pre_trusted_nodes() {
        let nodes = create_test_nodes(5);
        let pre_trusted = create_pre_trusted_nodes(&nodes, 2);
        let engine = Arc::new(EigenTrustEngine::new(pre_trusted.clone()));
        
        // Initial cache should have high trust for pre-trusted nodes
        for node in &pre_trusted {
            let trust = engine.get_trust_async(node).await;
            assert_eq!(trust, 0.9, "Pre-trusted nodes should start with high trust");
        }
        
        // Add and remove pre-trusted nodes dynamically
        engine.add_pre_trusted(nodes[2].clone()).await;
        let trust = engine.get_trust_async(&nodes[2]).await;
        assert_eq!(trust, 0.9, "Newly added pre-trusted node should have high trust");
        
        engine.remove_pre_trusted(&nodes[0]).await;
        let pre_trusted_after = engine.pre_trusted_nodes.read().await;
        assert!(!pre_trusted_after.contains(&nodes[0]));
    }
    
    #[tokio::test]
    async fn test_trust_decay() {
        let nodes = create_test_nodes(3);
        let mut engine = EigenTrustEngine::new(HashSet::new());
        engine.decay_rate = 0.5; // Fast decay for testing
        
        // Build trust relationships
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[1], &nodes[2], true).await;
        
        // First computation
        let trust1 = engine.compute_global_trust().await;
        let initial_trust_2 = trust1.get(&nodes[2]).copied().unwrap_or(0.0);
        
        // Simulate 1 hour passing
        *engine.last_update.write().await = 
            tokio::time::Instant::now() - Duration::from_secs(3600);
        
        // Second computation should show decay
        let trust2 = engine.compute_global_trust().await;
        let decayed_trust_2 = trust2.get(&nodes[2]).copied().unwrap_or(0.0);
        
        // Trust should decay by factor of 0.5
        assert!(decayed_trust_2 < initial_trust_2);
        assert!((decayed_trust_2 - initial_trust_2 * 0.5).abs() < 0.1);
    }
    
    #[tokio::test]
    async fn test_trust_inheritance() {
        let nodes = create_test_nodes(5);
        let pre_trusted = create_pre_trusted_nodes(&nodes, 1);
        let engine = EigenTrustEngine::new(pre_trusted);
        
        // Established trust network
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[1], &nodes[2], true).await;
        
        // New node joins and is trusted by established node
        engine.update_local_trust(&nodes[2], &nodes[3], true).await;
        
        let global_trust = engine.compute_global_trust().await;
        
        // New node should inherit some trust
        let new_node_trust = global_trust.get(&nodes[3]).expect("Should have trust");
        assert!(new_node_trust > &0.0, "New node should inherit trust");
        
        // But less than the node that trusts it
        let parent_trust = global_trust.get(&nodes[2]).expect("Should have trust");
        assert!(new_node_trust < parent_trust, "New node should have less trust than parent");
    }
    
    #[tokio::test]
    async fn test_multi_factor_trust() {
        let nodes = create_test_nodes(3);
        let engine = Arc::new(EigenTrustEngine::new(HashSet::new()));
        
        // Node with good statistics
        engine.update_node_stats(&nodes[0], NodeStatisticsUpdate::Uptime(86400)).await;
        engine.update_node_stats(&nodes[0], NodeStatisticsUpdate::CorrectResponse).await;
        engine.update_node_stats(&nodes[0], NodeStatisticsUpdate::CorrectResponse).await;
        engine.update_node_stats(&nodes[0], NodeStatisticsUpdate::CorrectResponse).await;
        engine.update_node_stats(&nodes[0], NodeStatisticsUpdate::StorageContributed(1000)).await;
        engine.update_node_stats(&nodes[0], NodeStatisticsUpdate::BandwidthContributed(500)).await;
        
        // Node with poor statistics
        engine.update_node_stats(&nodes[1], NodeStatisticsUpdate::Uptime(3600)).await;
        engine.update_node_stats(&nodes[1], NodeStatisticsUpdate::FailedResponse).await;
        engine.update_node_stats(&nodes[1], NodeStatisticsUpdate::FailedResponse).await;
        engine.update_node_stats(&nodes[1], NodeStatisticsUpdate::CorrectResponse).await;
        
        // Create trust relationships
        engine.update_local_trust(&nodes[2], &nodes[0], true).await;
        engine.update_local_trust(&nodes[2], &nodes[1], true).await;
        
        let global_trust = engine.compute_global_trust().await;
        
        let trust_0 = global_trust.get(&nodes[0]).expect("Should have trust");
        let trust_1 = global_trust.get(&nodes[1]).expect("Should have trust");
        
        // Node with better statistics should have higher trust
        assert!(trust_0 > trust_1, "Node with better stats should have higher trust");
    }
    
    #[tokio::test]
    async fn test_trust_based_routing() {
        let nodes = create_test_nodes(6);
        let pre_trusted = create_pre_trusted_nodes(&nodes, 1);
        let engine = Arc::new(EigenTrustEngine::new(pre_trusted));
        
        // Build trust network
        // High trust path: 0 -> 1 -> 2 -> target
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[1], &nodes[2], true).await;
        engine.update_local_trust(&nodes[2], &nodes[5], true).await;
        
        // Low trust path: 0 -> 3 -> 4 -> target
        engine.update_local_trust(&nodes[0], &nodes[3], false).await;
        engine.update_local_trust(&nodes[3], &nodes[4], false).await;
        engine.update_local_trust(&nodes[4], &nodes[5], true).await;
        
        // Compute global trust
        engine.compute_global_trust().await;
        
        // Create routing strategy
        let strategy = TrustBasedRoutingStrategy::new(engine.clone(), nodes[0].clone());
        
        // Find path to target
        let path = strategy.find_path(&nodes[5]).await
            .expect("Should find a path");
        
        // Should prefer high-trust nodes
        assert!(path.contains(&nodes[1]) || path.contains(&nodes[2]),
                "Path should include high-trust nodes");
        assert!(!path.contains(&nodes[3]) && !path.contains(&nodes[4]),
                "Path should avoid low-trust nodes");
        
        // Test route scoring
        let score_high = strategy.route_score(&nodes[1], &nodes[5]);
        let score_low = strategy.route_score(&nodes[3], &nodes[5]);
        assert!(score_high > score_low, "High-trust route should score better");
    }
    
    #[tokio::test]
    async fn test_sybil_attack_resistance() {
        let honest_nodes = create_test_nodes(5);
        let sybil_nodes = create_test_nodes(20); // Many Sybil nodes
        let pre_trusted = create_pre_trusted_nodes(&honest_nodes, 2);
        let engine = Arc::new(EigenTrustEngine::new(pre_trusted));
        
        // Honest nodes trust each other
        for i in 0..5 {
            for j in 0..5 {
                if i != j {
                    engine.update_local_trust(&honest_nodes[i], &honest_nodes[j], true).await;
                }
            }
        }
        
        // Sybil nodes collude - trust each other
        for i in 0..20 {
            for j in 0..20 {
                if i != j {
                    engine.update_local_trust(&sybil_nodes[i], &sybil_nodes[j], true).await;
                }
            }
        }
        
        // Sybil nodes try to gain trust from one honest node
        for sybil in &sybil_nodes {
            engine.update_local_trust(&honest_nodes[4], sybil, true).await;
        }
        
        let global_trust = engine.compute_global_trust().await;
        
        // Calculate average trust for honest vs Sybil nodes
        let honest_avg: f64 = honest_nodes.iter()
            .map(|n| global_trust.get(n).copied().unwrap_or(0.0))
            .sum::<f64>() / honest_nodes.len() as f64;
            
        let sybil_avg: f64 = sybil_nodes.iter()
            .map(|n| global_trust.get(n).copied().unwrap_or(0.0))
            .sum::<f64>() / sybil_nodes.len() as f64;
        
        // Honest nodes should maintain higher average trust
        assert!(honest_avg > sybil_avg * 2.0,
                "Honest nodes should have much higher trust than Sybils");
        
        // Pre-trusted nodes should maintain highest trust
        for pre_trusted_node in engine.pre_trusted_nodes.read().await.iter() {
            let trust = global_trust.get(pre_trusted_node).expect("Should have trust");
            assert!(trust > &sybil_avg * 5.0,
                    "Pre-trusted nodes should maintain very high trust");
        }
    }
    
    #[tokio::test]
    async fn test_collusion_attack_resistance() {
        let nodes = create_test_nodes(10);
        let pre_trusted = create_pre_trusted_nodes(&nodes, 1);
        let engine = Arc::new(EigenTrustEngine::new(pre_trusted));
        
        // Honest behavior from pre-trusted and some nodes
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[0], &nodes[2], true).await;
        engine.update_local_trust(&nodes[1], &nodes[3], true).await;
        engine.update_local_trust(&nodes[2], &nodes[3], true).await;
        
        // Colluding group (nodes 4-7) boost each other
        for i in 4..8 {
            for j in 4..8 {
                if i != j {
                    engine.update_local_trust(&nodes[i], &nodes[j], true).await;
                }
            }
        }
        
        // Colluding nodes provide false negative feedback about honest nodes
        for i in 4..8 {
            for j in 1..4 {
                engine.update_local_trust(&nodes[i], &nodes[j], false).await;
            }
        }
        
        let global_trust = engine.compute_global_trust().await;
        
        // Honest nodes connected to pre-trusted should maintain good trust
        let trust_1 = global_trust.get(&nodes[1]).expect("Should have trust");
        let trust_2 = global_trust.get(&nodes[2]).expect("Should have trust");
        
        // Colluding nodes should have lower trust
        let trust_5 = global_trust.get(&nodes[5]).expect("Should have trust");
        let trust_6 = global_trust.get(&nodes[6]).expect("Should have trust");
        
        assert!(trust_1 > trust_5, "Honest nodes should have higher trust");
        assert!(trust_2 > trust_6, "Honest nodes should have higher trust");
    }
    
    #[tokio::test]
    async fn test_concurrent_updates() {
        let nodes = create_test_nodes(10);
        let engine = Arc::new(EigenTrustEngine::new(HashSet::new()));
        
        // Spawn multiple tasks updating trust concurrently
        let mut handles = vec![];
        
        for i in 0..10 {
            let engine_clone = engine.clone();
            let nodes_clone = nodes.clone();
            
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    if i != j {
                        let success = (i + j) % 2 == 0;
                        engine_clone.update_local_trust(
                            &nodes_clone[i],
                            &nodes_clone[j],
                            success
                        ).await;
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all updates to complete
        for handle in handles {
            handle.await.expect("Task should complete");
        }
        
        // Verify all updates were recorded
        let local_trust = engine.local_trust.read().await;
        let update_count = local_trust.len();
        
        // Should have recorded all unique pairs
        assert_eq!(update_count, 90, "Should have 90 trust relationships");
        
        // Compute global trust should work correctly
        let global_trust = engine.compute_global_trust().await;
        assert!(!global_trust.is_empty());
    }
    
    #[tokio::test]
    async fn test_background_updates() {
        let nodes = create_test_nodes(3);
        let engine = Arc::new(EigenTrustEngine::new(HashSet::new()));
        let mut test_engine = engine.clone();
        
        // Set very short update interval for testing
        Arc::get_mut(&mut test_engine).unwrap().update_interval = Duration::from_millis(100);
        
        // Start background updates
        test_engine.clone().start_background_updates();
        
        // Add some trust relationships
        engine.update_local_trust(&nodes[0], &nodes[1], true).await;
        engine.update_local_trust(&nodes[1], &nodes[2], true).await;
        
        // Wait for background update to run
        sleep(Duration::from_millis(200)).await;
        
        // Cache should be updated
        let cached_trust = engine.trust_cache.read().await;
        assert!(!cached_trust.is_empty(), "Background update should populate cache");
        
        // Synchronous access should work
        let trust = engine.get_trust(&nodes[1]);
        assert!(trust > 0.0, "Should have trust value from cache");
    }
    
    #[tokio::test]
    async fn test_trust_provider_trait() {
        let nodes = create_test_nodes(4);
        let engine = Arc::new(EigenTrustEngine::new(HashSet::new()));
        let provider: Arc<dyn TrustProvider> = engine.clone();
        
        // Update trust through trait
        provider.update_trust(&nodes[0], &nodes[1], true);
        provider.update_trust(&nodes[1], &nodes[2], true);
        provider.update_trust(&nodes[2], &nodes[3], false);
        
        // Wait for async updates to complete
        sleep(Duration::from_millis(100)).await;
        
        // Compute global trust
        engine.compute_global_trust().await;
        
        // Access trust through trait
        let trust = provider.get_trust(&nodes[1]);
        assert!(trust >= 0.0 && trust <= 1.0, "Trust should be in valid range");
        
        let global = provider.get_global_trust();
        assert!(!global.is_empty(), "Should have global trust scores");
        
        // Remove node
        provider.remove_node(&nodes[3]);
        sleep(Duration::from_millis(100)).await;
        
        // Node should be removed from cache
        let cached = engine.trust_cache.read().await;
        assert!(!cached.contains_key(&nodes[3]), "Node should be removed");
    }
    
    // Property-based tests
    proptest! {
        #[test]
        fn prop_trust_always_normalized(
            interactions: Vec<(u8, u8, bool)>
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let nodes = create_test_nodes(10);
                let engine = EigenTrustEngine::new(HashSet::new());
                
                // Apply random interactions
                for (from, to, success) in interactions {
                    let from_idx = (from % 10) as usize;
                    let to_idx = (to % 10) as usize;
                    if from_idx != to_idx {
                        engine.update_local_trust(
                            &nodes[from_idx],
                            &nodes[to_idx],
                            success
                        ).await;
                    }
                }
                
                let global_trust = engine.compute_global_trust().await;
                
                // Total trust should always sum to 1.0 (or 0 if no nodes)
                let total: f64 = global_trust.values().sum();
                if !global_trust.is_empty() {
                    prop_assert!((total - 1.0).abs() < 0.001,
                                "Total trust should be 1.0, got {}", total);
                }
                
                // All individual trust values should be in [0, 1]
                for trust in global_trust.values() {
                    prop_assert!(*trust >= 0.0 && *trust <= 1.0,
                                "Trust should be in [0,1], got {}", trust);
                }
            });
        }
        
        #[test]
        fn prop_trust_monotonic_with_positive_feedback(
            positive_count: u8
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let nodes = create_test_nodes(3);
                let engine = Arc::new(EigenTrustEngine::new(HashSet::new()));
                
                // Record initial trust
                engine.update_local_trust(&nodes[0], &nodes[1], true).await;
                let trust1 = engine.compute_global_trust().await;
                let initial = trust1.get(&nodes[1]).copied().unwrap_or(0.0);
                
                // Add more positive feedback
                for _ in 0..positive_count.min(10) {
                    engine.update_local_trust(&nodes[0], &nodes[1], true).await;
                    engine.update_local_trust(&nodes[2], &nodes[1], true).await;
                }
                
                let trust2 = engine.compute_global_trust().await;
                let final_trust = trust2.get(&nodes[1]).copied().unwrap_or(0.0);
                
                // Trust should not decrease with positive feedback
                prop_assert!(final_trust >= initial * 0.99, // Allow small numerical error
                            "Trust decreased with positive feedback");
            });
        }
        
        #[test]
        fn prop_pre_trusted_maintain_high_trust(
            interactions: Vec<(u8, u8, bool)>
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let nodes = create_test_nodes(10);
                let pre_trusted = create_pre_trusted_nodes(&nodes, 2);
                let engine = EigenTrustEngine::new(pre_trusted.clone());
                
                // Apply random interactions
                for (from, to, success) in interactions {
                    let from_idx = (from % 10) as usize;
                    let to_idx = (to % 10) as usize;
                    if from_idx != to_idx {
                        engine.update_local_trust(
                            &nodes[from_idx],
                            &nodes[to_idx],
                            success
                        ).await;
                    }
                }
                
                let global_trust = engine.compute_global_trust().await;
                
                // Pre-trusted nodes should maintain relatively high trust
                for pre_node in &pre_trusted {
                    let trust = global_trust.get(pre_node).copied().unwrap_or(0.0);
                    let avg_trust = global_trust.values().sum::<f64>() / 
                                   global_trust.len().max(1) as f64;
                    
                    prop_assert!(trust >= avg_trust,
                                "Pre-trusted node should have above-average trust");
                }
            });
        }
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn bench_trust_computation_scaling() {
        println!("\nTrust Computation Scaling Benchmark:");
        println!("Nodes\tTime (ms)\tTime/Node (μs)");
        
        for size in [10, 50, 100, 200, 500] {
            let nodes = create_test_nodes(size);
            let engine = EigenTrustEngine::new(HashSet::new());
            
            // Create random trust network
            let mut rng = rand::thread_rng();
            let interactions = size * 5; // 5x interactions as nodes
            
            for _ in 0..interactions {
                let from = rng.gen_range(0..size);
                let to = rng.gen_range(0..size);
                if from != to {
                    let success = rng.gen_bool(0.7);
                    engine.update_local_trust(&nodes[from], &nodes[to], success).await;
                }
            }
            
            // Benchmark computation
            let start = Instant::now();
            let _ = engine.compute_global_trust().await;
            let duration = start.elapsed();
            
            let ms = duration.as_secs_f64() * 1000.0;
            let us_per_node = (duration.as_secs_f64() * 1_000_000.0) / size as f64;
            
            println!("{:5}\t{:8.2}\t{:10.2}", size, ms, us_per_node);
        }
    }
    
    #[tokio::test]
    async fn bench_concurrent_updates() {
        let nodes = create_test_nodes(100);
        let engine = Arc::new(EigenTrustEngine::new(HashSet::new()));
        
        println!("\nConcurrent Update Benchmark:");
        println!("Threads\tUpdates\tTime (ms)\tUpdates/sec");
        
        for threads in [1, 2, 4, 8, 16] {
            let updates_per_thread = 1000;
            let total_updates = threads * updates_per_thread;
            
            let start = Instant::now();
            
            let mut handles = vec![];
            for t in 0..threads {
                let engine_clone = engine.clone();
                let nodes_clone = nodes.clone();
                
                let handle = tokio::spawn(async move {
                    let mut rng = rand::thread_rng();
                    for _ in 0..updates_per_thread {
                        let from = rng.gen_range(0..100);
                        let to = rng.gen_range(0..100);
                        if from != to {
                            let success = rng.gen_bool(0.7);
                            engine_clone.update_local_trust(
                                &nodes_clone[from],
                                &nodes_clone[to],
                                success
                            ).await;
                        }
                    }
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.await.unwrap();
            }
            
            let duration = start.elapsed();
            let ms = duration.as_secs_f64() * 1000.0;
            let updates_per_sec = total_updates as f64 / duration.as_secs_f64();
            
            println!("{:7}\t{:7}\t{:8.2}\t{:11.0}", 
                     threads, total_updates, ms, updates_per_sec);
        }
    }
}
