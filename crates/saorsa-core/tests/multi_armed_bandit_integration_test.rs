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

//! Integration tests for Multi-Armed Bandit routing optimization

use saorsa_core::adaptive::{
    ContentType, MABConfig, MultiArmedBandit, NodeId, Outcome, RouteId, StrategyChoice,
};
use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;
use tokio;

/// Simulate a network environment with different route success rates
struct NetworkSimulator {
    /// Success rates for each (strategy, content_type) pair
    success_rates: HashMap<(StrategyChoice, ContentType), f64>,
    /// Latency ranges for each strategy
    latencies: HashMap<StrategyChoice, (u64, u64)>,
}

impl NetworkSimulator {
    fn new() -> Self {
        let mut success_rates = HashMap::new();
        let mut latencies = HashMap::new();

        // Kademlia: Good for DHT lookups, poor for real-time
        success_rates.insert((StrategyChoice::Kademlia, ContentType::DHTLookup), 0.95);
        success_rates.insert((StrategyChoice::Kademlia, ContentType::DataRetrieval), 0.80);
        success_rates.insert(
            (StrategyChoice::Kademlia, ContentType::RealtimeMessage),
            0.40,
        );
        success_rates.insert(
            (StrategyChoice::Kademlia, ContentType::ComputeRequest),
            0.70,
        );

        // Hyperbolic: Good for data retrieval, excellent for real-time
        success_rates.insert((StrategyChoice::Hyperbolic, ContentType::DHTLookup), 0.70);
        success_rates.insert(
            (StrategyChoice::Hyperbolic, ContentType::DataRetrieval),
            0.90,
        );
        success_rates.insert(
            (StrategyChoice::Hyperbolic, ContentType::RealtimeMessage),
            0.95,
        );
        success_rates.insert(
            (StrategyChoice::Hyperbolic, ContentType::ComputeRequest),
            0.75,
        );

        // TrustPath: Good for compute requests, poor for DHT
        success_rates.insert((StrategyChoice::TrustPath, ContentType::DHTLookup), 0.50);
        success_rates.insert(
            (StrategyChoice::TrustPath, ContentType::DataRetrieval),
            0.75,
        );
        success_rates.insert(
            (StrategyChoice::TrustPath, ContentType::RealtimeMessage),
            0.80,
        );
        success_rates.insert(
            (StrategyChoice::TrustPath, ContentType::ComputeRequest),
            0.95,
        );

        // SOMRegion: Specialized for compute requests
        success_rates.insert((StrategyChoice::SOMRegion, ContentType::DHTLookup), 0.60);
        success_rates.insert(
            (StrategyChoice::SOMRegion, ContentType::DataRetrieval),
            0.70,
        );
        success_rates.insert(
            (StrategyChoice::SOMRegion, ContentType::RealtimeMessage),
            0.65,
        );
        success_rates.insert(
            (StrategyChoice::SOMRegion, ContentType::ComputeRequest),
            0.98,
        );

        // Latency ranges (min, max) in milliseconds
        latencies.insert(StrategyChoice::Kademlia, (20, 100));
        latencies.insert(StrategyChoice::Hyperbolic, (10, 50));
        latencies.insert(StrategyChoice::TrustPath, (30, 150));
        latencies.insert(StrategyChoice::SOMRegion, (15, 80));

        Self {
            success_rates,
            latencies,
        }
    }

    fn simulate_request(&self, strategy: StrategyChoice, content_type: ContentType) -> Outcome {
        let success_rate = self
            .success_rates
            .get(&(strategy, content_type))
            .unwrap_or(&0.5);

        let success = rand::random::<f64>() < *success_rate;

        let (min_latency, max_latency) = self.latencies.get(&strategy).unwrap_or(&(50, 200));

        let latency_ms = min_latency + (rand::random::<u64>() % (max_latency - min_latency));

        let hops = match strategy {
            StrategyChoice::Hyperbolic => 2 + (rand::random::<usize>() % 3),
            StrategyChoice::Kademlia => 3 + (rand::random::<usize>() % 5),
            StrategyChoice::TrustPath => 2 + (rand::random::<usize>() % 4),
            StrategyChoice::SOMRegion => 1 + (rand::random::<usize>() % 3),
        };

        Outcome {
            success,
            latency_ms,
            hops,
        }
    }
}

#[tokio::test]
async fn test_mab_learns_optimal_strategies() {
    let temp_dir = TempDir::new().unwrap();
    let config = MABConfig {
        epsilon: 0.1, // 10% exploration
        min_samples: 20,
        decay_factor: 0.99,
        storage_path: Some(temp_dir.path().to_path_buf()),
        persist_interval: Duration::from_secs(10),
        max_stats_age: Duration::from_secs(3600),
    };

    let mab = MultiArmedBandit::new(config).await.unwrap();
    let simulator = NetworkSimulator::new();
    let destination = NodeId::from_bytes([42u8; 32]);
    let all_strategies = vec![
        StrategyChoice::Kademlia,
        StrategyChoice::Hyperbolic,
        StrategyChoice::TrustPath,
        StrategyChoice::SOMRegion,
    ];

    // Training phase: Let MAB learn the network characteristics
    for _ in 0..1000 {
        for content_type in &[
            ContentType::DHTLookup,
            ContentType::DataRetrieval,
            ContentType::RealtimeMessage,
            ContentType::ComputeRequest,
        ] {
            let decision = mab
                .select_route(&destination, *content_type, &all_strategies)
                .await
                .unwrap();

            let outcome = simulator.simulate_request(decision.route_id.strategy, *content_type);

            mab.update_route(&decision.route_id, *content_type, &outcome)
                .await
                .unwrap();
        }
    }

    // Evaluation phase: Check if MAB learned the optimal strategies
    let mut strategy_selections: HashMap<(ContentType, StrategyChoice), u32> = HashMap::new();

    for _ in 0..1000 {
        for content_type in &[
            ContentType::DHTLookup,
            ContentType::DataRetrieval,
            ContentType::RealtimeMessage,
            ContentType::ComputeRequest,
        ] {
            let decision = mab
                .select_route(&destination, *content_type, &all_strategies)
                .await
                .unwrap();

            if !decision.exploration {
                *strategy_selections
                    .entry((*content_type, decision.route_id.strategy))
                    .or_insert(0) += 1;
            }
        }
    }

    // Verify that MAB learned the optimal strategies
    // Kademlia should be preferred for DHT lookups
    let kademlia_dht = strategy_selections
        .get(&(ContentType::DHTLookup, StrategyChoice::Kademlia))
        .unwrap_or(&0);
    assert!(
        *kademlia_dht > 600,
        "Kademlia should be preferred for DHT lookups"
    );

    // Hyperbolic should be preferred for real-time messages
    let hyperbolic_realtime = strategy_selections
        .get(&(ContentType::RealtimeMessage, StrategyChoice::Hyperbolic))
        .unwrap_or(&0);
    assert!(
        *hyperbolic_realtime > 600,
        "Hyperbolic should be preferred for real-time messages"
    );

    // SOMRegion should be preferred for compute requests
    let som_compute = strategy_selections
        .get(&(ContentType::ComputeRequest, StrategyChoice::SOMRegion))
        .unwrap_or(&0);
    assert!(
        *som_compute > 600,
        "SOMRegion should be preferred for compute requests"
    );

    // Check metrics
    let metrics = mab.get_metrics().await;
    assert!(metrics.total_decisions > 4000);
    assert!(metrics.overall_success_rate > 0.7);
    assert_eq!(metrics.unique_routes, 16); // 4 strategies × 4 content types
}

#[tokio::test]
async fn test_mab_exploration_vs_exploitation() {
    let config = MABConfig {
        epsilon: 0.2, // 20% exploration
        min_samples: 10,
        decay_factor: 0.99,
        storage_path: None,
        persist_interval: Duration::from_secs(60),
        max_stats_age: Duration::from_secs(3600),
    };

    let mab = MultiArmedBandit::new(config).await.unwrap();
    let destination = NodeId::from_bytes([1u8; 32]);
    let strategies = vec![StrategyChoice::Kademlia, StrategyChoice::Hyperbolic];

    let mut exploration_count = 0;
    let mut exploitation_count = 0;

    for _ in 0..1000 {
        let decision = mab
            .select_route(&destination, ContentType::DHTLookup, &strategies)
            .await
            .unwrap();

        if decision.exploration {
            exploration_count += 1;
        } else {
            exploitation_count += 1;
        }

        // Update with random outcome
        let outcome = Outcome {
            success: rand::random(),
            latency_ms: 50,
            hops: 3,
        };
        mab.update_route(&decision.route_id, ContentType::DHTLookup, &outcome)
            .await
            .unwrap();
    }

    // Should have roughly 20% exploration
    let exploration_ratio = exploration_count as f64 / 1000.0;
    assert!(
        (exploration_ratio - 0.2).abs() < 0.05,
        "Exploration ratio {} should be close to 0.2",
        exploration_ratio
    );
}

#[tokio::test]
async fn test_mab_persistence_and_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let destination = NodeId::from_bytes([1u8; 32]);

    // Phase 1: Create MAB and add statistics
    {
        let config = MABConfig {
            epsilon: 0.1,
            min_samples: 5,
            decay_factor: 0.99,
            storage_path: Some(temp_dir.path().to_path_buf()),
            persist_interval: Duration::from_secs(1),
            max_stats_age: Duration::from_secs(3600),
        };

        let mab = MultiArmedBandit::new(config).await.unwrap();

        // Add statistics for different routes
        for i in 0..100 {
            let route_id = RouteId::new(destination.clone(), StrategyChoice::Kademlia);
            let outcome = Outcome {
                success: i % 2 == 0,
                latency_ms: 50 + i,
                hops: 3,
            };
            mab.update_route(&route_id, ContentType::DHTLookup, &outcome)
                .await
                .unwrap();
        }

        // Wait for automatic persistence
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Phase 2: Create new MAB instance and verify persistence
    {
        let config = MABConfig {
            epsilon: 0.1,
            min_samples: 5,
            decay_factor: 0.99,
            storage_path: Some(temp_dir.path().to_path_buf()),
            persist_interval: Duration::from_secs(60),
            max_stats_age: Duration::from_secs(3600),
        };

        let mab = MultiArmedBandit::new(config).await.unwrap();
        let stats = mab.get_all_statistics().await;

        assert!(!stats.is_empty(), "Statistics should be loaded from disk");

        let key = (
            RouteId::new(destination.clone(), StrategyChoice::Kademlia),
            ContentType::DHTLookup,
        );
        assert!(stats.contains_key(&key));
        assert_eq!(stats[&key].attempts, 100);
        assert_eq!(stats[&key].successes, 50);
    }
}

#[tokio::test]
async fn test_mab_confidence_intervals() {
    let config = MABConfig::default();
    let mab = MultiArmedBandit::new(config).await.unwrap();
    let destination = NodeId::from_bytes([1u8; 32]);
    let route_id = RouteId::new(destination.clone(), StrategyChoice::Kademlia);

    // Initially, confidence interval should be maximum uncertainty
    let (lower, upper) = mab
        .get_route_confidence(&route_id, ContentType::DHTLookup)
        .await
        .unwrap();
    assert_eq!((lower, upper), (0.0, 1.0));

    // Add some successful outcomes
    for _ in 0..20 {
        let outcome = Outcome {
            success: true,
            latency_ms: 50,
            hops: 3,
        };
        mab.update_route(&route_id, ContentType::DHTLookup, &outcome)
            .await
            .unwrap();
    }

    // Add some failures
    for _ in 0..5 {
        let outcome = Outcome {
            success: false,
            latency_ms: 100,
            hops: 5,
        };
        mab.update_route(&route_id, ContentType::DHTLookup, &outcome)
            .await
            .unwrap();
    }

    // Confidence interval should be narrower and reflect ~80% success rate
    let (lower, upper) = mab
        .get_route_confidence(&route_id, ContentType::DHTLookup)
        .await
        .unwrap();

    assert!(
        lower > 0.6 && lower < 0.8,
        "Lower bound {} should be around 0.7",
        lower
    );
    assert!(
        upper > 0.8 && upper < 0.95,
        "Upper bound {} should be around 0.9",
        upper
    );
    assert!(
        upper > lower,
        "Upper bound should be greater than lower bound"
    );
}

#[tokio::test]
async fn test_mab_adaptive_to_network_changes() {
    let config = MABConfig {
        epsilon: 0.15,
        min_samples: 10,
        decay_factor: 0.95, // Faster decay for this test
        storage_path: None,
        persist_interval: Duration::from_secs(60),
        max_stats_age: Duration::from_secs(3600),
    };

    let mab = MultiArmedBandit::new(config).await.unwrap();
    let destination = NodeId::from_bytes([1u8; 32]);
    let strategies = vec![StrategyChoice::Kademlia, StrategyChoice::Hyperbolic];

    // Phase 1: Kademlia is better (90% success)
    for _ in 0..200 {
        for strategy in &strategies {
            let route_id = RouteId::new(destination.clone(), *strategy);
            let success = match strategy {
                StrategyChoice::Kademlia => rand::random::<f64>() < 0.9,
                _ => rand::random::<f64>() < 0.3,
            };
            let outcome = Outcome {
                success,
                latency_ms: 50,
                hops: 3,
            };
            mab.update_route(&route_id, ContentType::DHTLookup, &outcome)
                .await
                .unwrap();
        }
    }

    // Check that Kademlia is preferred
    let mut kademlia_count = 0;
    for _ in 0..100 {
        let decision = mab
            .select_route(&destination, ContentType::DHTLookup, &strategies)
            .await
            .unwrap();

        if decision.route_id.strategy == StrategyChoice::Kademlia && !decision.exploration {
            kademlia_count += 1;
        }
    }
    assert!(
        kademlia_count > 70,
        "Kademlia should be preferred initially"
    );

    // Phase 2: Network changes - Hyperbolic becomes better
    for _ in 0..200 {
        for strategy in &strategies {
            let route_id = RouteId::new(destination.clone(), *strategy);
            let success = match strategy {
                StrategyChoice::Hyperbolic => rand::random::<f64>() < 0.95,
                _ => rand::random::<f64>() < 0.2,
            };
            let outcome = Outcome {
                success,
                latency_ms: 30,
                hops: 2,
            };
            mab.update_route(&route_id, ContentType::DHTLookup, &outcome)
                .await
                .unwrap();
        }
    }

    // Check that MAB adapts to prefer Hyperbolic
    let mut hyperbolic_count = 0;
    for _ in 0..100 {
        let decision = mab
            .select_route(&destination, ContentType::DHTLookup, &strategies)
            .await
            .unwrap();

        if decision.route_id.strategy == StrategyChoice::Hyperbolic && !decision.exploration {
            hyperbolic_count += 1;
        }
    }
    assert!(
        hyperbolic_count > 70,
        "Should adapt to prefer Hyperbolic after network change"
    );
}
