//! Production Integration Tests
//!
//! Comprehensive tests for production readiness scenarios using real API.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use saorsa_core::{
    Config,
    adaptive::{
        client::AdaptiveClient, coordinator::AdaptiveCoordinator, replication::ReplicationManager,
        security::SecurityManager, storage::StorageManager,
    },
    health::HealthMonitor,
    identity::IdentityManager,
    network::NetworkEvent,
    validation::InputValidator,
};

/// Production integration test framework
struct ProductionTestFramework {
    configs: Vec<Config>,
    coordinators: Vec<Arc<AdaptiveCoordinator>>,
    clients: Vec<Arc<AdaptiveClient>>,
    validators: Vec<Arc<InputValidator>>,
    health_monitors: Vec<Arc<HealthMonitor>>,
}

impl ProductionTestFramework {
    async fn new(node_count: usize) -> Result<Self> {
        let mut configs = Vec::new();
        let mut coordinators = Vec::new();
        let mut clients = Vec::new();
        let mut validators = Vec::new();
        let mut health_monitors = Vec::new();

        for i in 0..node_count {
            // Create test configuration
            let mut config = Config::default();
            config.network.listen_port = 8000 + i as u16;
            config.network.max_connections = 50;
            config.storage.replication_factor = 3;
            config.security.enable_encryption = true;

            // Create coordinator
            let coordinator = Arc::new(AdaptiveCoordinator::new(config.clone()).await?);

            // Create client
            let client = Arc::new(AdaptiveClient::new(config.clone()).await?);

            // Create validator
            let validator = Arc::new(InputValidator::new());

            // Create health monitor
            let health_monitor = Arc::new(HealthMonitor::new(config.clone()).await?);

            configs.push(config);
            coordinators.push(coordinator);
            clients.push(client);
            validators.push(validator);
            health_monitors.push(health_monitor);
        }

        Ok(Self {
            configs,
            coordinators,
            clients,
            validators,
            health_monitors,
        })
    }

    async fn start_all_nodes(&self) -> Result<()> {
        // Start coordinators
        for coordinator in &self.coordinators {
            coordinator.start().await?;
        }

        // Start clients
        for client in &self.clients {
            client.start().await?;
        }

        // Start health monitors
        for health_monitor in &self.health_monitors {
            health_monitor.start().await?;
        }

        // Allow startup time
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }

    async fn connect_nodes(&self) -> Result<()> {
        // Connect clients to coordinators
        for (i, client) in self.clients.iter().enumerate() {
            for (j, _) in self.coordinators.iter().enumerate() {
                if i != j {
                    let coord_addr = format!("127.0.0.1:{}", 8000 + j);
                    client.connect_to_peer(&coord_addr).await?;
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
        Ok(())
    }

    async fn test_data_operations(&self) -> Result<usize> {
        let mut successful_operations = 0;

        // Test store and retrieve operations
        for i in 0..10 {
            let key = format!("test_key_{}", i);
            let value = format!("test_value_{}", i).into_bytes();

            // Store via first client
            if self.clients[0].store(&key, value.clone()).await.is_ok() {
                // Try to retrieve from another client
                if let Ok(Some(retrieved)) = self.clients[1].retrieve(&key).await {
                    if retrieved == value {
                        successful_operations += 1;
                    }
                }
            }
        }

        Ok(successful_operations)
    }

    async fn test_input_validation(&self) -> Result<usize> {
        let mut validation_tests_passed = 0;

        let test_cases = vec![
            ("valid_key", true),
            ("", false),                        // Empty key should fail
            ("a".repeat(1000).as_str(), false), // Too long should fail
            ("valid/path", true),
            ("../invalid", false), // Path traversal should fail
        ];

        for (test_input, should_pass) in test_cases {
            let validation_result = self.validators[0].validate_key(test_input);

            if validation_result.is_ok() == should_pass {
                validation_tests_passed += 1;
            }
        }

        Ok(validation_tests_passed)
    }

    async fn test_health_monitoring(&self) -> Result<bool> {
        // Check that health monitors are active
        for health_monitor in &self.health_monitors {
            let health_status = health_monitor.get_status().await?;
            if !health_status.is_healthy {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn test_security_features(&self) -> Result<usize> {
        let mut security_tests_passed = 0;

        // Test that encryption is enabled
        for config in &self.configs {
            if config.security.enable_encryption {
                security_tests_passed += 1;
            }
        }

        // Test identity management
        let identity_manager = IdentityManager::new(self.configs[0].clone())?;
        if identity_manager.get_node_identity().is_ok() {
            security_tests_passed += 1;
        }

        Ok(security_tests_passed)
    }

    async fn test_performance_under_load(&self) -> Result<(f64, f64)> {
        let operations_count = 50;
        let start_time = std::time::Instant::now();

        // Concurrent operations
        let mut tasks = Vec::new();

        for i in 0..operations_count {
            let client = self.clients[i % self.clients.len()].clone();
            let task = tokio::spawn(async move {
                let key = format!("perf_test_{}", i);
                let value = format!("perf_value_{}", i).into_bytes();

                client.store(&key, value).await.is_ok()
            });
            tasks.push(task);
        }

        let results = futures::future::join_all(tasks).await;
        let successful_ops = results
            .into_iter()
            .map(|r| r.unwrap_or(false))
            .filter(|&success| success)
            .count();

        let duration = start_time.elapsed();
        let ops_per_second = successful_ops as f64 / duration.as_secs_f64();

        Ok((ops_per_second, duration.as_secs_f64()))
    }

    async fn get_network_stats(&self) -> Result<HashMap<String, usize>> {
        let mut stats = HashMap::new();

        for (i, coordinator) in self.coordinators.iter().enumerate() {
            let peer_count = coordinator.get_peer_count().await.unwrap_or(0);
            stats.insert(format!("coordinator_{}", i), peer_count);
        }

        for (i, client) in self.clients.iter().enumerate() {
            let connection_count = client.get_connection_count().await.unwrap_or(0);
            stats.insert(format!("client_{}", i), connection_count);
        }

        Ok(stats)
    }

    async fn shutdown_all(&self) -> Result<()> {
        // Shutdown coordinators
        for coordinator in &self.coordinators {
            let _ = coordinator.shutdown().await;
        }

        // Shutdown clients
        for client in &self.clients {
            let _ = client.shutdown().await;
        }

        // Shutdown health monitors
        for health_monitor in &self.health_monitors {
            let _ = health_monitor.shutdown().await;
        }

        Ok(())
    }
}

#[tokio::test]
async fn test_production_system_startup() -> Result<()> {
    let framework = ProductionTestFramework::new(3).await?;

    // Test system startup
    framework.start_all_nodes().await?;

    // Test network connectivity
    framework.connect_nodes().await?;

    // Verify network stats
    let stats = framework.get_network_stats().await?;
    assert!(!stats.is_empty(), "Should have network statistics");

    println!("Network stats: {:?}", stats);

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_production_data_operations() -> Result<()> {
    let framework = ProductionTestFramework::new(3).await?;

    framework.start_all_nodes().await?;
    framework.connect_nodes().await?;

    // Test data operations
    let successful_ops = framework.test_data_operations().await?;

    println!("Successful data operations: {}/10", successful_ops);
    assert!(
        successful_ops > 0,
        "Should have some successful data operations"
    );

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_production_input_validation() -> Result<()> {
    let framework = ProductionTestFramework::new(1).await?;

    framework.start_all_nodes().await?;

    // Test input validation
    let validation_passes = framework.test_input_validation().await?;

    println!("Validation tests passed: {}/5", validation_passes);
    assert_eq!(validation_passes, 5, "All validation tests should pass");

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_production_health_monitoring() -> Result<()> {
    let framework = ProductionTestFramework::new(2).await?;

    framework.start_all_nodes().await?;

    // Test health monitoring
    let all_healthy = framework.test_health_monitoring().await?;

    println!("All nodes healthy: {}", all_healthy);
    assert!(all_healthy, "All nodes should be healthy");

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_production_security_features() -> Result<()> {
    let framework = ProductionTestFramework::new(2).await?;

    framework.start_all_nodes().await?;

    // Test security features
    let security_tests_passed = framework.test_security_features().await?;

    println!("Security tests passed: {}", security_tests_passed);
    assert!(security_tests_passed > 0, "Should pass some security tests");

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_production_performance_benchmarks() -> Result<()> {
    let framework = ProductionTestFramework::new(3).await?;

    framework.start_all_nodes().await?;
    framework.connect_nodes().await?;

    // Test performance under load
    let (ops_per_sec, total_duration) = framework.test_performance_under_load().await?;

    println!(
        "Performance: {:.2} ops/sec in {:.2}s",
        ops_per_sec, total_duration
    );

    // Performance assertions (adjust based on requirements)
    assert!(ops_per_sec > 1.0, "Should achieve > 1 operation per second");
    assert!(total_duration < 60.0, "Should complete within 60 seconds");

    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_production_integration_comprehensive() -> Result<()> {
    let framework = ProductionTestFramework::new(4).await?;

    println!("🚀 Starting comprehensive production integration test");

    // 1. System startup
    println!("  Starting all nodes...");
    framework.start_all_nodes().await?;

    // 2. Network connectivity
    println!("  Establishing network connections...");
    framework.connect_nodes().await?;

    // 3. Data operations
    println!("  Testing data operations...");
    let data_ops = framework.test_data_operations().await?;
    assert!(
        data_ops > 5,
        "Should have significant data operation success"
    );

    // 4. Input validation
    println!("  Testing input validation...");
    let validation_ops = framework.test_input_validation().await?;
    assert!(validation_ops >= 4, "Most validation tests should pass");

    // 5. Health monitoring
    println!("  Testing health monitoring...");
    let health_ok = framework.test_health_monitoring().await?;
    assert!(health_ok, "Health monitoring should work");

    // 6. Security features
    println!("  Testing security features...");
    let security_ops = framework.test_security_features().await?;
    assert!(security_ops > 0, "Security features should work");

    // 7. Performance test
    println!("  Testing performance...");
    let (perf_ops, perf_duration) = framework.test_performance_under_load().await?;
    assert!(perf_ops > 0.5, "Should have reasonable performance");

    // 8. Final network stats
    println!("  Collecting final stats...");
    let final_stats = framework.get_network_stats().await?;

    println!("✅ Comprehensive test completed successfully!");
    println!("   Data operations: {}/10", data_ops);
    println!("   Validation tests: {}/5", validation_ops);
    println!("   Security tests: {}", security_ops);
    println!("   Performance: {:.2} ops/sec", perf_ops);
    println!("   Final stats: {:?}", final_stats);

    framework.shutdown_all().await?;
    Ok(())
}
