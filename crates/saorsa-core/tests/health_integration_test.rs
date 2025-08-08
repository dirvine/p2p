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

//! Integration tests for the health check system

use async_trait::async_trait;
use saorsa_core::Result;
use saorsa_core::health::{
    ComponentChecker, CompositeHealthChecker, DhtHealthChecker, HealthManager, HealthServer,
    HealthStatus, NetworkHealthChecker, PeerHealthChecker, ResourceHealthChecker,
    StorageHealthChecker, TransportHealthChecker,
};
use saorsa_core::production::{ProductionConfig, ResourceManager};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test implementation of a simple health checker
struct TestChecker {
    status: HealthStatus,
    latency_ms: u64,
}

#[async_trait]
impl ComponentChecker for TestChecker {
    async fn check(&self) -> Result<HealthStatus> {
        // Simulate latency
        sleep(Duration::from_millis(self.latency_ms)).await;
        Ok(self.status)
    }
}

#[tokio::test]
async fn test_health_manager_basic() {
    let manager = HealthManager::new("1.0.0-test".to_string());

    // Register a healthy component
    manager
        .register_checker(
            "test_healthy",
            Box::new(TestChecker {
                status: HealthStatus::Healthy,
                latency_ms: 10,
            }),
        )
        .await;

    // Register a degraded component
    manager
        .register_checker(
            "test_degraded",
            Box::new(TestChecker {
                status: HealthStatus::Degraded,
                latency_ms: 20,
            }),
        )
        .await;

    // Get health status
    let health = manager.get_health().await.unwrap();

    assert_eq!(health.version, "1.0.0-test");
    assert_eq!(health.status, "degraded"); // Should be degraded due to one degraded component
    assert!(health.is_ready());
    assert!(health.is_alive());
    assert_eq!(health.checks.len(), 2);

    // Check individual components
    let healthy_check = &health.checks["test_healthy"];
    assert_eq!(healthy_check.status, HealthStatus::Healthy);
    assert!(healthy_check.latency_ms >= 10);

    let degraded_check = &health.checks["test_degraded"];
    assert_eq!(degraded_check.status, HealthStatus::Degraded);
    assert!(degraded_check.latency_ms >= 20);
}

#[tokio::test]
async fn test_health_manager_unhealthy() {
    let manager = HealthManager::new("1.0.0-test".to_string());

    // Register an unhealthy component
    manager
        .register_checker(
            "test_unhealthy",
            Box::new(TestChecker {
                status: HealthStatus::Unhealthy,
                latency_ms: 5,
            }),
        )
        .await;

    let health = manager.get_health().await.unwrap();

    assert_eq!(health.status, "unhealthy");
    assert!(!health.is_ready()); // Should not be ready when unhealthy
    assert!(health.is_alive()); // But still alive
}

#[tokio::test]
async fn test_health_manager_caching() {
    let manager = HealthManager::new("1.0.0-test".to_string());

    // Register a component with variable latency
    let counter = Arc::new(tokio::sync::Mutex::new(0));
    let counter_clone = counter.clone();

    struct CountingChecker {
        counter: Arc<tokio::sync::Mutex<u32>>,
    }

    #[async_trait]
    impl ComponentChecker for CountingChecker {
        async fn check(&self) -> Result<HealthStatus> {
            let mut count = self.counter.lock().await;
            *count += 1;
            Ok(HealthStatus::Healthy)
        }
    }

    manager
        .register_checker(
            "counting",
            Box::new(CountingChecker {
                counter: counter_clone,
            }),
        )
        .await;

    // First call should execute the check
    let _health1 = manager.get_health().await.unwrap();
    assert_eq!(*counter.lock().await, 1);

    // Immediate second call should use cache
    let _health2 = manager.get_health().await.unwrap();
    assert_eq!(*counter.lock().await, 1); // Should still be 1 due to cache

    // Wait for cache to expire (>100ms)
    sleep(Duration::from_millis(150)).await;

    // This call should execute the check again
    let _health3 = manager.get_health().await.unwrap();
    assert_eq!(*counter.lock().await, 2);
}

#[tokio::test]
async fn test_network_health_checker() {
    // Test with healthy network
    let checker = NetworkHealthChecker::new(|| async { Ok(10) }).with_min_peers(5);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Healthy);

    // Test with degraded network
    let checker = NetworkHealthChecker::new(|| async { Ok(2) }).with_min_peers(5);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Degraded);

    // Test with unhealthy network
    let checker = NetworkHealthChecker::new(|| async { Ok(0) }).with_min_peers(1);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Unhealthy);
}

#[tokio::test]
async fn test_dht_health_checker() {
    // Test with healthy DHT
    let checker = DhtHealthChecker::new(|| async { Ok(20) }).with_min_nodes(10);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Healthy);

    // Test with degraded DHT
    let checker = DhtHealthChecker::new(|| async { Ok(5) }).with_min_nodes(10);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Degraded);
}

#[tokio::test]
async fn test_storage_health_checker() {
    let temp_dir = tempfile::tempdir().unwrap();
    let checker =
        StorageHealthChecker::new(temp_dir.path().to_path_buf()).with_min_free_space(1024); // 1KB minimum

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Healthy);

    // Test with non-existent path
    let checker = StorageHealthChecker::new(std::path::PathBuf::from("/non/existent/path"));
    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Unhealthy);
}

#[tokio::test]
async fn test_transport_health_checker() {
    // Test listening transport
    let checker = TransportHealthChecker::new(|| async { Ok(true) });
    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Healthy);

    // Test non-listening transport
    let checker = TransportHealthChecker::new(|| async { Ok(false) });
    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Unhealthy);
}

#[tokio::test]
async fn test_resource_health_checker() {
    let config = ProductionConfig::default();
    let manager = Arc::new(ResourceManager::new(config));

    let checker = ResourceHealthChecker::new(manager);
    let status = checker.check().await.unwrap();

    // Should be healthy with default metrics
    assert_eq!(status, HealthStatus::Healthy);
}

#[tokio::test]
async fn test_peer_health_checker() {
    // Test with optimal peer count
    let checker = PeerHealthChecker::new(|| async { Ok(50) }).with_peer_limits(10, 100);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Healthy);

    // Test with too few peers
    let checker = PeerHealthChecker::new(|| async { Ok(5) }).with_peer_limits(10, 100);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Unhealthy);

    // Test with too many peers
    let checker = PeerHealthChecker::new(|| async { Ok(150) }).with_peer_limits(10, 100);

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Degraded);
}

#[tokio::test]
async fn test_composite_health_checker() {
    let checker = CompositeHealthChecker::new()
        .add_checker(
            "healthy",
            Box::new(TestChecker {
                status: HealthStatus::Healthy,
                latency_ms: 5,
            }),
        )
        .add_checker(
            "degraded",
            Box::new(TestChecker {
                status: HealthStatus::Degraded,
                latency_ms: 5,
            }),
        )
        .add_checker(
            "healthy2",
            Box::new(TestChecker {
                status: HealthStatus::Healthy,
                latency_ms: 5,
            }),
        );

    // Overall should be degraded
    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Degraded);

    // Test with unhealthy component
    let checker = CompositeHealthChecker::new().add_checker(
        "unhealthy",
        Box::new(TestChecker {
            status: HealthStatus::Unhealthy,
            latency_ms: 5,
        }),
    );

    let status = checker.check().await.unwrap();
    assert_eq!(status, HealthStatus::Unhealthy);
}

#[tokio::test]
async fn test_health_server_lifecycle() {
    let health_manager = Arc::new(HealthManager::new("1.0.0-test".to_string()));

    // Register a simple checker
    health_manager
        .register_checker(
            "test",
            Box::new(TestChecker {
                status: HealthStatus::Healthy,
                latency_ms: 5,
            }),
        )
        .await;

    // Use port 0 to let OS assign a port
    let addr = "127.0.0.1:0".parse().unwrap();
    let (server, shutdown_tx) = HealthServer::new(health_manager, addr);

    // Start server in background
    let server_handle = tokio::spawn(async move { server.run().await });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Shutdown server
    shutdown_tx.send(()).unwrap();

    // Wait for server to stop
    let result = server_handle.await.unwrap();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_health_response_time() {
    let manager = HealthManager::new("1.0.0-test".to_string());

    // Register multiple components
    for i in 0..5 {
        manager
            .register_checker(
                &format!("component_{}", i),
                Box::new(TestChecker {
                    status: HealthStatus::Healthy,
                    latency_ms: 10, // Each takes 10ms
                }),
            )
            .await;
    }

    // Measure total health check time
    let start = std::time::Instant::now();
    let _health = manager.get_health().await.unwrap();
    let elapsed = start.elapsed();

    // Should complete in under 100ms (checks run in parallel)
    assert!(
        elapsed.as_millis() < 100,
        "Health check took {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_health_check_timeout() {
    // Create a checker that times out
    struct TimeoutChecker;

    #[async_trait]
    impl ComponentChecker for TimeoutChecker {
        async fn check(&self) -> Result<HealthStatus> {
            sleep(Duration::from_secs(1)).await; // Sleep longer than timeout
            Ok(HealthStatus::Healthy)
        }
    }

    let manager = HealthManager::new("1.0.0-test".to_string());
    manager
        .register_checker("timeout", Box::new(TimeoutChecker))
        .await;

    let start = std::time::Instant::now();
    let health = manager.get_health().await.unwrap();
    let elapsed = start.elapsed();

    // Should timeout quickly
    assert!(elapsed.as_millis() < 200); // Allow some overhead

    // Component should be marked as unhealthy
    let check = &health.checks["timeout"];
    assert_eq!(check.status, HealthStatus::Unhealthy);
}

#[tokio::test]
async fn test_debug_info() {
    let manager = HealthManager::new("1.0.0-test".to_string());

    let debug_info = manager.get_debug_info().await.unwrap();

    // Verify system info
    assert!(!debug_info.system.os.is_empty());
    assert!(!debug_info.system.arch.is_empty());
    assert!(debug_info.system.cpu_count > 0);
    assert!(debug_info.system.total_memory > 0);

    // Verify runtime info
    assert!(!debug_info.runtime.rust_version.is_empty());
    assert!(debug_info.runtime.thread_count > 0);
}
