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

//! Component health checkers for various P2P subsystems

#![allow(dead_code)]

use super::HealthStatus;
use crate::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use tokio::time::{Duration, timeout};

/// Trait for component health checkers
#[async_trait]
pub trait ComponentChecker: Send + Sync {
    /// Check the health of the component
    async fn check(&self) -> Result<HealthStatus>;

    /// Get debug information about the component
    async fn debug_info(&self) -> Option<JsonValue> {
        None
    }
}

/// Network connectivity health checker
///
/// This is a placeholder implementation that will be connected to the actual
/// Network type once it's available
pub struct NetworkHealthChecker {
    get_peer_count: Box<
        dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize>> + Send>>
            + Send
            + Sync,
    >,
    min_peers: usize,
    timeout_duration: Duration,
}

impl NetworkHealthChecker {
    /// Create a new network health checker with a peer count function
    pub fn new<F, Fut>(get_peer_count: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<usize>> + Send + 'static,
    {
        Self {
            get_peer_count: Box::new(move || Box::pin(get_peer_count())),
            min_peers: 1,
            timeout_duration: Duration::from_millis(50),
        }
    }

    /// Set minimum number of peers for healthy status
    pub fn with_min_peers(mut self, min_peers: usize) -> Self {
        self.min_peers = min_peers;
        self
    }
}

#[async_trait]
impl ComponentChecker for NetworkHealthChecker {
    async fn check(&self) -> Result<HealthStatus> {
        // Check network connectivity with timeout
        let future = (self.get_peer_count)();
        match timeout(self.timeout_duration, future).await {
            Ok(Ok(count)) => {
                if count >= self.min_peers {
                    Ok(HealthStatus::Healthy)
                } else if count > 0 {
                    Ok(HealthStatus::Degraded)
                } else {
                    Ok(HealthStatus::Unhealthy)
                }
            }
            Ok(Err(_)) => Ok(HealthStatus::Unhealthy),
            Err(_) => Ok(HealthStatus::Unhealthy), // Timeout
        }
    }

    async fn debug_info(&self) -> Option<JsonValue> {
        let future = (self.get_peer_count)();
        if let Ok(Ok(count)) = timeout(self.timeout_duration, future).await {
            Some(serde_json::json!({
                "peer_count": count,
                "min_peers": self.min_peers,
            }))
        } else {
            None
        }
    }
}

/// DHT availability health checker
///
/// This is a placeholder implementation that will be connected to the actual
/// DHT type once it's available
pub struct DhtHealthChecker {
    get_routing_table_size: Box<
        dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize>> + Send>>
            + Send
            + Sync,
    >,
    min_nodes: usize,
    timeout_duration: Duration,
}

impl DhtHealthChecker {
    /// Create a new DHT health checker with a routing table size function
    pub fn new<F, Fut>(get_routing_table_size: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<usize>> + Send + 'static,
    {
        Self {
            get_routing_table_size: Box::new(move || Box::pin(get_routing_table_size())),
            min_nodes: 3,
            timeout_duration: Duration::from_millis(50),
        }
    }

    /// Set minimum number of DHT nodes for healthy status
    pub fn with_min_nodes(mut self, min_nodes: usize) -> Self {
        self.min_nodes = min_nodes;
        self
    }
}

#[async_trait]
impl ComponentChecker for DhtHealthChecker {
    async fn check(&self) -> Result<HealthStatus> {
        // Check DHT routing table
        let future = (self.get_routing_table_size)();
        match timeout(self.timeout_duration, future).await {
            Ok(Ok(size)) => {
                if size >= self.min_nodes {
                    Ok(HealthStatus::Healthy)
                } else if size > 0 {
                    Ok(HealthStatus::Degraded)
                } else {
                    Ok(HealthStatus::Unhealthy)
                }
            }
            Ok(Err(_)) => Ok(HealthStatus::Unhealthy),
            Err(_) => Ok(HealthStatus::Unhealthy), // Timeout
        }
    }

    async fn debug_info(&self) -> Option<JsonValue> {
        let future = (self.get_routing_table_size)();
        if let Ok(Ok(size)) = timeout(self.timeout_duration, future).await {
            Some(serde_json::json!({
                "routing_table_size": size,
                "min_nodes": self.min_nodes,
                "replication_factor": 8, // Default K value
            }))
        } else {
            None
        }
    }
}

/// Storage access health checker
pub struct StorageHealthChecker {
    storage_path: std::path::PathBuf,
    min_free_space: u64,
    timeout_duration: Duration,
}

impl StorageHealthChecker {
    /// Create a new storage health checker
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        Self {
            storage_path,
            min_free_space: 100 * 1024 * 1024, // 100MB default
            timeout_duration: Duration::from_millis(50),
        }
    }

    /// Set minimum free space for healthy status
    pub fn with_min_free_space(mut self, bytes: u64) -> Self {
        self.min_free_space = bytes;
        self
    }
}

#[async_trait]
impl ComponentChecker for StorageHealthChecker {
    async fn check(&self) -> Result<HealthStatus> {
        // Check storage accessibility and free space
        let path = self.storage_path.clone();
        let min_free = self.min_free_space;

        match timeout(
            self.timeout_duration,
            tokio::task::spawn_blocking(move || check_storage_health(&path, min_free)),
        )
        .await
        {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(_)) => Ok(HealthStatus::Unhealthy),
            Err(_) => Ok(HealthStatus::Unhealthy), // Timeout
        }
    }

    async fn debug_info(&self) -> Option<JsonValue> {
        if let Ok(metadata) = tokio::fs::metadata(&self.storage_path).await {
            // Get disk usage info (simplified)
            Some(serde_json::json!({
                "path": self.storage_path.display().to_string(),
                "exists": true,
                "is_dir": metadata.is_dir(),
                "min_free_space": self.min_free_space,
            }))
        } else {
            Some(serde_json::json!({
                "path": self.storage_path.display().to_string(),
                "exists": false,
            }))
        }
    }
}

/// Helper function to check storage health
fn check_storage_health(path: &std::path::Path, min_free_space: u64) -> HealthStatus {
    use std::fs;

    // Check if path exists and is writable
    if !path.exists() {
        return HealthStatus::Unhealthy;
    }

    // Try to create a test file to verify write access
    let test_file = path.join(".health_check");
    match fs::write(&test_file, b"health_check") {
        Ok(_) => {
            // Clean up test file
            let _ = fs::remove_file(&test_file);

            // Check free space (platform-specific, simplified here)
            // In production, use a crate like fs2 or sysinfo
            if get_free_space(path) >= min_free_space {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            }
        }
        Err(_) => HealthStatus::Unhealthy,
    }
}

/// Get free space for a path (stub implementation)
fn get_free_space(_path: &std::path::Path) -> u64 {
    // In a real implementation, use platform-specific APIs or a crate
    1024 * 1024 * 1024 // 1GB default
}

// TODO: Uncomment when ResourceManager is implemented in production module
// /// Resource usage health checker
// pub struct ResourceHealthChecker {
//     resource_manager: Arc<ResourceManager>,
//     max_memory_percent: f64,
//     max_cpu_percent: f64,
//     timeout_duration: Duration,
// }
//
// impl ResourceHealthChecker {
//     /// Create a new resource health checker
//     pub fn new(resource_manager: Arc<ResourceManager>) -> Self {
//         Self {
//             resource_manager,
//             max_memory_percent: 80.0,
//             max_cpu_percent: 90.0,
//             timeout_duration: Duration::from_millis(50),
//         }
//     }
// }
//
// #[async_trait]
// impl ComponentChecker for ResourceHealthChecker {
//     async fn check(&self) -> Result<HealthStatus> {
//         match timeout(self.timeout_duration, self.resource_manager.get_metrics()).await {
//             Ok(metrics) => {
//                 // Check CPU usage
//                 if metrics.cpu_usage > self.max_cpu_percent {
//                     return Ok(HealthStatus::Unhealthy);
//                 }
//
//                 // Check memory usage (simplified - compare against configured limit)
//                 let memory_percent = if self.resource_manager.config.max_memory_bytes > 0 {
//                     (metrics.memory_used as f64 / self.resource_manager.config.max_memory_bytes as f64) * 100.0
//                 } else {
//                     0.0
//                 };
//
//                 if memory_percent > self.max_memory_percent {
//                     Ok(HealthStatus::Degraded)
//                 } else {
//                     Ok(HealthStatus::Healthy)
//                 }
//             }
//             Err(_) => Ok(HealthStatus::Unhealthy), // Timeout
//         }
//     }
//
//     async fn debug_info(&self) -> Option<JsonValue> {
//         if let Ok(metrics) = self.resource_manager.get_metrics().await {
//             Some(serde_json::json!({
//                 "memory_used": metrics.memory_used,
//                 "active_connections": metrics.active_connections,
//                 "bandwidth_usage": metrics.bandwidth_usage,
//                 "cpu_usage": metrics.cpu_usage,
//                 "dht_ops_per_sec": metrics.dht_metrics.ops_per_sec,
//                 "mcp_calls_per_sec": metrics.mcp_metrics.calls_per_sec,
//             }))
//         } else {
//             None
//         }
//     }
// }

/// Transport health checker
///
/// This is a placeholder implementation that will be connected to the actual
/// Transport type once it's available
pub struct TransportHealthChecker {
    is_listening: Box<
        dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send>>
            + Send
            + Sync,
    >,
    timeout_duration: Duration,
}

impl TransportHealthChecker {
    /// Create a new transport health checker with a listening check function
    pub fn new<F, Fut>(is_listening: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<bool>> + Send + 'static,
    {
        Self {
            is_listening: Box::new(move || Box::pin(is_listening())),
            timeout_duration: Duration::from_millis(50),
        }
    }
}

#[async_trait]
impl ComponentChecker for TransportHealthChecker {
    async fn check(&self) -> Result<HealthStatus> {
        let future = (self.is_listening)();
        match timeout(self.timeout_duration, future).await {
            Ok(Ok(true)) => Ok(HealthStatus::Healthy),
            Ok(Ok(false)) => Ok(HealthStatus::Unhealthy),
            Ok(Err(_)) => Ok(HealthStatus::Unhealthy),
            Err(_) => Ok(HealthStatus::Unhealthy), // Timeout
        }
    }

    async fn debug_info(&self) -> Option<JsonValue> {
        let future = (self.is_listening)();
        if let Ok(Ok(listening)) = timeout(self.timeout_duration, future).await {
            Some(serde_json::json!({
                "is_listening": listening,
                "transport_type": "p2p",
            }))
        } else {
            None
        }
    }
}

/// Peer connections health checker
///
/// This is a placeholder implementation that will be connected to the actual
/// Network type once it's available
pub struct PeerHealthChecker {
    get_peer_count: Box<
        dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize>> + Send>>
            + Send
            + Sync,
    >,
    min_peers: usize,
    max_peers: usize,
    timeout_duration: Duration,
}

impl PeerHealthChecker {
    /// Create a new peer health checker with a peer count function
    pub fn new<F, Fut>(get_peer_count: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<usize>> + Send + 'static,
    {
        Self {
            get_peer_count: Box::new(move || Box::pin(get_peer_count())),
            min_peers: 1,
            max_peers: 1000,
            timeout_duration: Duration::from_millis(50),
        }
    }

    /// Set peer count thresholds
    pub fn with_peer_limits(mut self, min: usize, max: usize) -> Self {
        self.min_peers = min;
        self.max_peers = max;
        self
    }
}

#[async_trait]
impl ComponentChecker for PeerHealthChecker {
    async fn check(&self) -> Result<HealthStatus> {
        let future = (self.get_peer_count)();
        match timeout(self.timeout_duration, future).await {
            Ok(Ok(count)) => {
                if count < self.min_peers {
                    Ok(HealthStatus::Unhealthy)
                } else if count > self.max_peers {
                    Ok(HealthStatus::Degraded) // Too many peers
                } else {
                    Ok(HealthStatus::Healthy)
                }
            }
            Ok(Err(_)) => Ok(HealthStatus::Unhealthy),
            Err(_) => Ok(HealthStatus::Unhealthy), // Timeout
        }
    }

    async fn debug_info(&self) -> Option<JsonValue> {
        let future = (self.get_peer_count)();
        if let Ok(Ok(count)) = timeout(self.timeout_duration, future).await {
            Some(serde_json::json!({
                "peer_count": count,
                "min_peers": self.min_peers,
                "max_peers": self.max_peers,
            }))
        } else {
            None
        }
    }
}

/// Composite health checker that runs multiple checks
pub struct CompositeHealthChecker {
    checkers: Vec<(&'static str, Box<dyn ComponentChecker>)>,
}

impl CompositeHealthChecker {
    /// Create a new composite health checker
    pub fn new() -> Self {
        Self {
            checkers: Vec::new(),
        }
    }

    /// Add a component checker
    pub fn add_checker(mut self, name: &'static str, checker: Box<dyn ComponentChecker>) -> Self {
        self.checkers.push((name, checker));
        self
    }
}

#[async_trait]
impl ComponentChecker for CompositeHealthChecker {
    async fn check(&self) -> Result<HealthStatus> {
        let mut overall_status = HealthStatus::Healthy;

        for (_, checker) in &self.checkers {
            match checker.check().await {
                Ok(HealthStatus::Unhealthy) => return Ok(HealthStatus::Unhealthy),
                Ok(HealthStatus::Degraded) => overall_status = HealthStatus::Degraded,
                Ok(HealthStatus::Healthy) => {}
                Err(_) => return Ok(HealthStatus::Unhealthy),
            }
        }

        Ok(overall_status)
    }

    async fn debug_info(&self) -> Option<JsonValue> {
        let mut info = serde_json::json!({});

        for (name, checker) in &self.checkers {
            if let Some(debug) = checker.debug_info().await {
                info[name] = debug;
            }
        }

        Some(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test implementations
    struct TestNetwork {
        peer_count: usize,
    }

    impl TestNetwork {
        fn new(peer_count: usize) -> Self {
            Self { peer_count }
        }

        async fn peer_count(&self) -> Result<usize> {
            Ok(self.peer_count)
        }

        async fn debug_info(&self) -> Result<NetworkDebugInfo> {
            Ok(NetworkDebugInfo {
                peer_count: self.peer_count,
                active_connections: self.peer_count,
                listening_addresses: vec![],
                protocols: vec![],
            })
        }
    }

    struct NetworkDebugInfo {
        peer_count: usize,
        active_connections: usize,
        listening_addresses: Vec<String>,
        protocols: Vec<String>,
    }

    struct TestDHT {
        routing_table_size: usize,
    }

    impl TestDHT {
        fn new(size: usize) -> Self {
            Self {
                routing_table_size: size,
            }
        }

        async fn routing_table_size(&self) -> Result<usize> {
            Ok(self.routing_table_size)
        }

        async fn debug_info(&self) -> Result<DhtDebugInfo> {
            Ok(DhtDebugInfo {
                routing_table_size: self.routing_table_size,
                stored_values: 0,
                pending_queries: 0,
                replication_factor: 8,
            })
        }
    }

    struct DhtDebugInfo {
        routing_table_size: usize,
        stored_values: usize,
        pending_queries: usize,
        replication_factor: usize,
    }

    struct TestTransport {
        listening: bool,
    }

    impl TestTransport {
        fn new(listening: bool) -> Self {
            Self { listening }
        }

        async fn is_listening(&self) -> Result<bool> {
            Ok(self.listening)
        }

        async fn debug_info(&self) -> Result<TransportDebugInfo> {
            Ok(TransportDebugInfo {
                transport_type: "test".to_string(),
                listening_addresses: vec![],
                active_connections: 0,
                bytes_sent: 0,
                bytes_received: 0,
            })
        }
    }

    struct TransportDebugInfo {
        transport_type: String,
        listening_addresses: Vec<String>,
        active_connections: usize,
        bytes_sent: u64,
        bytes_received: u64,
    }

    #[tokio::test]
    async fn test_storage_health_checker() {
        let temp_dir = tempfile::tempdir().unwrap();
        let checker = StorageHealthChecker::new(temp_dir.path().to_path_buf());

        let status = checker.check().await.unwrap();
        assert_eq!(status, HealthStatus::Healthy);
    }

    // TODO: Uncomment when ResourceHealthChecker is implemented
    // #[tokio::test]
    // async fn test_resource_health_checker() {
    //     let config = ProductionConfig::default();
    //     let manager = Arc::new(ResourceManager::new(config));
    //
    //     let checker = ResourceHealthChecker::new(manager);
    //     let status = checker.check().await.unwrap();
    //     assert_eq!(status, HealthStatus::Healthy);
    // }

    #[tokio::test]
    async fn test_composite_health_checker() {
        // Test composite with simple checkers
        struct AlwaysHealthy;
        #[async_trait]
        impl ComponentChecker for AlwaysHealthy {
            async fn check(&self) -> Result<HealthStatus> {
                Ok(HealthStatus::Healthy)
            }
        }

        struct AlwaysDegraded;
        #[async_trait]
        impl ComponentChecker for AlwaysDegraded {
            async fn check(&self) -> Result<HealthStatus> {
                Ok(HealthStatus::Degraded)
            }
        }

        let checker = CompositeHealthChecker::new()
            .add_checker("healthy", Box::new(AlwaysHealthy))
            .add_checker("degraded", Box::new(AlwaysDegraded));

        let status = checker.check().await.unwrap();
        assert_eq!(status, HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_composite_health_checker_unhealthy() {
        struct AlwaysUnhealthy;
        #[async_trait]
        impl ComponentChecker for AlwaysUnhealthy {
            async fn check(&self) -> Result<HealthStatus> {
                Ok(HealthStatus::Unhealthy)
            }
        }

        let checker =
            CompositeHealthChecker::new().add_checker("unhealthy", Box::new(AlwaysUnhealthy));

        let status = checker.check().await.unwrap();
        assert_eq!(status, HealthStatus::Unhealthy);
    }
}
