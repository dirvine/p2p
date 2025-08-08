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

//! Health check system for P2P Foundation
//!
//! This module provides comprehensive health monitoring with:
//! - HTTP endpoints for liveness and readiness checks
//! - Prometheus-compatible metrics export
//! - Component-level health status
//! - Debug information endpoints
//! - Sub-100ms response times

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

mod checks;
mod endpoints;
mod metrics;

pub use checks::ComponentChecker;
pub use endpoints::{HealthEndpoints, HealthServer};
pub use metrics::{HealthMetrics, PrometheusExporter};

/// Health status for a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Component is healthy and functioning normally
    Healthy,
    /// Component is degraded but still functional
    Degraded,
    /// Component is unhealthy and not functioning
    Unhealthy,
}

impl HealthStatus {
    /// Check if the status indicates the component is operational
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }

    /// Get string representation for serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        }
    }
}

/// Health information for a single component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Current status of the component
    pub status: HealthStatus,
    /// Response time in milliseconds
    pub latency_ms: u64,
    /// Optional error message if unhealthy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional metadata about the component
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ComponentHealth {
    /// Create a healthy component status
    pub fn healthy(latency_ms: u64) -> Self {
        Self {
            status: HealthStatus::Healthy,
            latency_ms,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an unhealthy component status
    pub fn unhealthy(latency_ms: u64, error: String) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            latency_ms,
            error: Some(error),
            metadata: HashMap::new(),
        }
    }

    /// Create a degraded component status
    pub fn degraded(latency_ms: u64, error: Option<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            latency_ms,
            error,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the component health
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Overall health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status (healthy, degraded, or unhealthy)
    pub status: String,
    /// Version of the P2P node
    pub version: String,
    /// Uptime duration
    pub uptime: Duration,
    /// Individual component health checks
    pub checks: HashMap<String, ComponentHealth>,
    /// Timestamp of the health check
    pub timestamp: SystemTime,
}

impl HealthResponse {
    /// Create a new health response
    pub fn new(version: String, uptime: Duration) -> Self {
        Self {
            status: "healthy".to_string(),
            version,
            uptime,
            checks: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }

    /// Add a component health check result
    pub fn add_check(&mut self, name: impl Into<String>, health: ComponentHealth) {
        // Update overall status based on component health
        if health.status == HealthStatus::Unhealthy {
            self.status = "unhealthy".to_string();
        } else if health.status == HealthStatus::Degraded && self.status != "unhealthy" {
            self.status = "degraded".to_string();
        }

        self.checks.insert(name.into(), health);
    }

    /// Check if the system is ready to serve traffic
    pub fn is_ready(&self) -> bool {
        self.status != "unhealthy"
    }

    /// Check if the system is alive (basic liveness)
    pub fn is_alive(&self) -> bool {
        // System is alive if we can generate a response
        true
    }
}

/// Debug information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    /// System information
    pub system: SystemInfo,
    /// Runtime information
    pub runtime: RuntimeInfo,
    /// Component details
    pub components: HashMap<String, serde_json::Value>,
}

/// System information for debug endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Number of CPUs
    pub cpu_count: usize,
    /// Total memory in bytes
    pub total_memory: u64,
    /// Available memory in bytes
    pub available_memory: u64,
}

/// Runtime information for debug endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    /// Rust version
    pub rust_version: String,
    /// Number of active threads
    pub thread_count: usize,
    /// Current memory usage in bytes
    pub memory_usage: u64,
    /// Uptime duration
    pub uptime: Duration,
}

/// Health check manager
pub struct HealthManager {
    /// Start time for uptime calculation
    start_time: Instant,
    /// Version string
    version: String,
    /// Component checkers
    checkers: Arc<RwLock<HashMap<String, Box<dyn ComponentChecker>>>>,
    /// Cached health response
    cached_response: Arc<RwLock<Option<(Instant, HealthResponse)>>>,
    /// Cache duration
    cache_duration: Duration,
}

impl HealthManager {
    /// Create a new health manager
    pub fn new(version: String) -> Self {
        Self {
            start_time: Instant::now(),
            version,
            checkers: Arc::new(RwLock::new(HashMap::new())),
            cached_response: Arc::new(RwLock::new(None)),
            cache_duration: Duration::from_millis(100), // Cache for 100ms
        }
    }

    /// Register a component health checker
    pub async fn register_checker(
        &self,
        name: impl Into<String>,
        checker: Box<dyn ComponentChecker>,
    ) {
        let mut checkers = self.checkers.write().await;
        checkers.insert(name.into(), checker);
    }

    /// Get current health status
    pub async fn get_health(&self) -> Result<HealthResponse> {
        // Check cache first
        {
            let cache = self.cached_response.read().await;
            if let Some((cached_at, ref response)) = *cache {
                if cached_at.elapsed() < self.cache_duration {
                    return Ok(response.clone());
                }
            }
        }

        // Perform health checks
        let uptime = self.start_time.elapsed();
        let mut response = HealthResponse::new(self.version.clone(), uptime);

        let checkers = self.checkers.read().await;
        for (name, checker) in checkers.iter() {
            let start = Instant::now();
            let health = match checker.check().await {
                Ok(status) => {
                    let latency_ms = start.elapsed().as_millis() as u64;
                    match status {
                        HealthStatus::Healthy => ComponentHealth::healthy(latency_ms),
                        HealthStatus::Degraded => ComponentHealth::degraded(latency_ms, None),
                        HealthStatus::Unhealthy => {
                            ComponentHealth::unhealthy(latency_ms, "Check failed".to_string())
                        }
                    }
                }
                Err(e) => {
                    let latency_ms = start.elapsed().as_millis() as u64;
                    ComponentHealth::unhealthy(latency_ms, e.to_string())
                }
            };
            response.add_check(name, health);
        }

        // Update cache
        {
            let mut cache = self.cached_response.write().await;
            *cache = Some((Instant::now(), response.clone()));
        }

        Ok(response)
    }

    /// Get debug information
    pub async fn get_debug_info(&self) -> Result<DebugInfo> {
        let system = SystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_count: num_cpus::get(),
            total_memory: Self::get_total_memory(),
            available_memory: Self::get_available_memory(),
        };

        let runtime = RuntimeInfo {
            rust_version: env!("CARGO_PKG_VERSION").to_string(), // Use package version instead
            thread_count: Self::get_thread_count(),
            memory_usage: Self::get_memory_usage(),
            uptime: self.start_time.elapsed(),
        };

        let mut components = HashMap::new();
        let checkers = self.checkers.read().await;
        for (name, checker) in checkers.iter() {
            if let Some(debug_info) = checker.debug_info().await {
                components.insert(name.clone(), debug_info);
            }
        }

        Ok(DebugInfo {
            system,
            runtime,
            components,
        })
    }

    /// Get total system memory (stub implementation)
    fn get_total_memory() -> u64 {
        // In a real implementation, use sysinfo crate
        8 * 1024 * 1024 * 1024 // 8GB default
    }

    /// Get available system memory (stub implementation)
    fn get_available_memory() -> u64 {
        // In a real implementation, use sysinfo crate
        4 * 1024 * 1024 * 1024 // 4GB default
    }

    /// Get current thread count (stub implementation)
    fn get_thread_count() -> usize {
        // In a real implementation, use std::thread or sysinfo
        4
    }

    /// Get current memory usage (stub implementation)
    fn get_memory_usage() -> u64 {
        // In a real implementation, use jemalloc stats or similar
        100 * 1024 * 1024 // 100MB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded.is_operational());
        assert!(!HealthStatus::Unhealthy.is_operational());

        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Degraded.as_str(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "unhealthy");
    }

    #[test]
    fn test_component_health() {
        let health = ComponentHealth::healthy(10);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.latency_ms, 10);
        assert!(health.error.is_none());

        let health = ComponentHealth::unhealthy(20, "Connection failed".to_string());
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.latency_ms, 20);
        assert_eq!(health.error.as_deref(), Some("Connection failed"));

        let health = ComponentHealth::degraded(15, Some("High latency".to_string()))
            .with_metadata("connections", serde_json::json!(95));
        assert_eq!(health.status, HealthStatus::Degraded);
        assert_eq!(
            health.metadata.get("connections"),
            Some(&serde_json::json!(95))
        );
    }

    #[test]
    fn test_health_response() {
        let mut response = HealthResponse::new("1.0.0".to_string(), Duration::from_secs(3600));
        assert_eq!(response.status, "healthy");
        assert!(response.is_ready());
        assert!(response.is_alive());

        // Add healthy component
        response.add_check("network", ComponentHealth::healthy(5));
        assert_eq!(response.status, "healthy");

        // Add degraded component
        response.add_check("dht", ComponentHealth::degraded(50, None));
        assert_eq!(response.status, "degraded");
        assert!(response.is_ready());

        // Add unhealthy component
        response.add_check(
            "storage",
            ComponentHealth::unhealthy(100, "Disk full".to_string()),
        );
        assert_eq!(response.status, "unhealthy");
        assert!(!response.is_ready());
        assert!(response.is_alive()); // Still alive even if unhealthy
    }

    #[tokio::test]
    async fn test_health_manager() {
        let manager = HealthManager::new("1.0.0".to_string());

        // Get health without any checkers
        let health = manager.get_health().await.unwrap();
        assert_eq!(health.status, "healthy");
        assert!(health.checks.is_empty());

        // Test caching
        let health2 = manager.get_health().await.unwrap();
        assert_eq!(health.timestamp, health2.timestamp); // Should be cached
    }

    #[test]
    fn test_debug_info_structure() {
        let system = SystemInfo {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            cpu_count: 8,
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 8 * 1024 * 1024 * 1024,
        };

        let runtime = RuntimeInfo {
            rust_version: "1.75.0".to_string(),
            thread_count: 10,
            memory_usage: 500 * 1024 * 1024,
            uptime: Duration::from_secs(7200),
        };

        let debug_info = DebugInfo {
            system,
            runtime,
            components: HashMap::new(),
        };

        // Verify serialization works
        let json = serde_json::to_string(&debug_info).unwrap();
        assert!(json.contains("linux"));
        assert!(json.contains("x86_64"));
    }
}
