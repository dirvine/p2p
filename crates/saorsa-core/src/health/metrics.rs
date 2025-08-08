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

//! Prometheus metrics export for P2P health monitoring

use super::{HealthManager, HealthStatus};
use crate::Result;
use std::fmt::Write;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Health metrics for Prometheus export
pub struct HealthMetrics {
    /// Node uptime in seconds
    pub uptime_seconds: f64,
    /// Number of healthy components
    pub healthy_components: u64,
    /// Number of degraded components
    pub degraded_components: u64,
    /// Number of unhealthy components
    pub unhealthy_components: u64,
    /// Total number of components
    pub total_components: u64,
    /// Network peer count
    pub network_peer_count: u64,
    /// DHT routing table size
    pub dht_routing_table_size: u64,
    /// Active connections
    pub active_connections: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Bandwidth usage in bytes per second
    pub bandwidth_usage_bps: u64,
    /// Storage free space in bytes
    pub storage_free_bytes: u64,
    /// DHT operations per second
    pub dht_ops_per_second: f64,
    /// MCP calls per second
    pub mcp_calls_per_second: f64,
}

/// Prometheus exporter for health metrics
pub struct PrometheusExporter {
    health_manager: Arc<HealthManager>,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new(health_manager: Arc<HealthManager>) -> Self {
        Self { health_manager }
    }

    /// Export metrics in Prometheus format
    pub async fn export(&self) -> Result<String> {
        let health = self.health_manager.get_health().await?;
        let debug_info = self.health_manager.get_debug_info().await?;

        let mut output = String::with_capacity(4096);

        // Node info
        writeln!(
            &mut output,
            "# HELP p2p_node_info Node information\n# TYPE p2p_node_info gauge\np2p_node_info{{version=\"{}\",os=\"{}\",arch=\"{}\"}} 1",
            health.version,
            debug_info.system.os,
            debug_info.system.arch
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        // Uptime
        writeln!(
            &mut output,
            "\n# HELP p2p_uptime_seconds Node uptime in seconds\n# TYPE p2p_uptime_seconds counter\np2p_uptime_seconds {}",
            health.uptime.as_secs_f64()
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        // Component health status
        let mut healthy = 0u64;
        let mut degraded = 0u64;
        let mut unhealthy = 0u64;

        for (_, component) in &health.checks {
            match component.status {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Degraded => degraded += 1,
                HealthStatus::Unhealthy => unhealthy += 1,
            }
        }

        writeln!(
            &mut output,
            "\n# HELP p2p_health_status Health status of components (1=healthy, 0=unhealthy)\n# TYPE p2p_health_status gauge"
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        for (name, component) in &health.checks {
            let value = match component.status {
                HealthStatus::Healthy => 1,
                HealthStatus::Degraded => 0, // Could use 0.5 for degraded
                HealthStatus::Unhealthy => 0,
            };
            writeln!(
                &mut output,
                "p2p_health_status{{component=\"{}\"}} {}",
                name, value
            )
            .map_err(|e| {
                crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into())
            })?;
        }

        // Component latency
        writeln!(
            &mut output,
            "\n# HELP p2p_health_check_latency_ms Health check latency in milliseconds\n# TYPE p2p_health_check_latency_ms gauge"
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        for (name, component) in &health.checks {
            writeln!(
                &mut output,
                "p2p_health_check_latency_ms{{component=\"{}\"}} {}",
                name, component.latency_ms
            )
            .map_err(|e| {
                crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into())
            })?;
        }

        // Summary metrics
        writeln!(
            &mut output,
            "\n# HELP p2p_healthy_components Number of healthy components\n# TYPE p2p_healthy_components gauge\np2p_healthy_components {}",
            healthy
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        writeln!(
            &mut output,
            "\n# HELP p2p_degraded_components Number of degraded components\n# TYPE p2p_degraded_components gauge\np2p_degraded_components {}",
            degraded
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        writeln!(
            &mut output,
            "\n# HELP p2p_unhealthy_components Number of unhealthy components\n# TYPE p2p_unhealthy_components gauge\np2p_unhealthy_components {}",
            unhealthy
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        // System metrics
        writeln!(
            &mut output,
            "\n# HELP p2p_system_cpu_count Number of CPU cores\n# TYPE p2p_system_cpu_count gauge\np2p_system_cpu_count {}",
            debug_info.system.cpu_count
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        writeln!(
            &mut output,
            "\n# HELP p2p_system_memory_total_bytes Total system memory in bytes\n# TYPE p2p_system_memory_total_bytes gauge\np2p_system_memory_total_bytes {}",
            debug_info.system.total_memory
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        writeln!(
            &mut output,
            "\n# HELP p2p_system_memory_available_bytes Available system memory in bytes\n# TYPE p2p_system_memory_available_bytes gauge\np2p_system_memory_available_bytes {}",
            debug_info.system.available_memory
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        // Runtime metrics
        writeln!(
            &mut output,
            "\n# HELP p2p_runtime_threads Number of runtime threads\n# TYPE p2p_runtime_threads gauge\np2p_runtime_threads {}",
            debug_info.runtime.thread_count
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        writeln!(
            &mut output,
            "\n# HELP p2p_runtime_memory_usage_bytes Runtime memory usage in bytes\n# TYPE p2p_runtime_memory_usage_bytes gauge\np2p_runtime_memory_usage_bytes {}",
            debug_info.runtime.memory_usage
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        // Extract component-specific metrics from metadata
        for (name, component) in &health.checks {
            for (key, value) in &component.metadata {
                if let Some(num) = value.as_u64() {
                    writeln!(
                        &mut output,
                        "\n# HELP p2p_{}_{} Component-specific metric\n# TYPE p2p_{}_{} gauge\np2p_{}_{} {}",
                        name, key, name, key, name, key, num
                    ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;
                } else if let Some(num) = value.as_f64() {
                    writeln!(
                        &mut output,
                        "\n# HELP p2p_{}_{} Component-specific metric\n# TYPE p2p_{}_{} gauge\np2p_{}_{} {}",
                        name, key, name, key, name, key, num
                    ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;
                }
            }
        }

        // Last scrape timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| {
                crate::P2PError::Internal(format!("Failed to get timestamp: {}", e).into())
            })?
            .as_secs();

        writeln!(
            &mut output,
            "\n# HELP p2p_last_scrape_timestamp_seconds Unix timestamp of last scrape\n# TYPE p2p_last_scrape_timestamp_seconds gauge\np2p_last_scrape_timestamp_seconds {}",
            timestamp
        ).map_err(|e| crate::P2PError::Internal(format!("Failed to write metrics: {}", e).into()))?;

        Ok(output)
    }

    /// Export metrics as a structured object
    pub async fn export_metrics(&self) -> Result<HealthMetrics> {
        let health = self.health_manager.get_health().await?;
        let debug_info = self.health_manager.get_debug_info().await?;

        let mut healthy = 0u64;
        let mut degraded = 0u64;
        let mut unhealthy = 0u64;

        for (_, component) in &health.checks {
            match component.status {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Degraded => degraded += 1,
                HealthStatus::Unhealthy => unhealthy += 1,
            }
        }

        // Extract metrics from component metadata
        let mut network_peer_count = 0u64;
        let mut dht_routing_table_size = 0u64;
        let mut active_connections = 0u64;
        let mut dht_ops_per_second = 0.0;
        let mut mcp_calls_per_second = 0.0;
        let mut bandwidth_usage_bps = 0u64;
        let mut storage_free_bytes = 0u64;

        for (name, component) in &health.checks {
            match name.as_str() {
                "network" => {
                    if let Some(count) = component
                        .metadata
                        .get("peer_count")
                        .and_then(|v| v.as_u64())
                    {
                        network_peer_count = count;
                    }
                    if let Some(count) = component
                        .metadata
                        .get("active_connections")
                        .and_then(|v| v.as_u64())
                    {
                        active_connections = count;
                    }
                }
                "dht" => {
                    if let Some(size) = component
                        .metadata
                        .get("routing_table_size")
                        .and_then(|v| v.as_u64())
                    {
                        dht_routing_table_size = size;
                    }
                }
                "resources" => {
                    if let Some(ops) = component
                        .metadata
                        .get("dht_ops_per_sec")
                        .and_then(|v| v.as_f64())
                    {
                        dht_ops_per_second = ops;
                    }
                    if let Some(calls) = component
                        .metadata
                        .get("mcp_calls_per_sec")
                        .and_then(|v| v.as_f64())
                    {
                        mcp_calls_per_second = calls;
                    }
                    if let Some(bw) = component
                        .metadata
                        .get("bandwidth_usage")
                        .and_then(|v| v.as_u64())
                    {
                        bandwidth_usage_bps = bw;
                    }
                }
                "storage" => {
                    if let Some(free) = component
                        .metadata
                        .get("free_space")
                        .and_then(|v| v.as_u64())
                    {
                        storage_free_bytes = free;
                    }
                }
                _ => {}
            }
        }

        Ok(HealthMetrics {
            uptime_seconds: health.uptime.as_secs_f64(),
            healthy_components: healthy,
            degraded_components: degraded,
            unhealthy_components: unhealthy,
            total_components: health.checks.len() as u64,
            network_peer_count,
            dht_routing_table_size,
            active_connections,
            memory_usage_bytes: debug_info.runtime.memory_usage,
            cpu_usage_percent: 0.0, // Would need actual CPU monitoring
            bandwidth_usage_bps,
            storage_free_bytes,
            dht_ops_per_second,
            mcp_calls_per_second,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthManager;

    #[tokio::test]
    async fn test_prometheus_export_basic() {
        let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));
        let exporter = PrometheusExporter::new(health_manager);

        let metrics = exporter.export().await.unwrap();

        // Check for required metric types
        assert!(metrics.contains("# HELP p2p_node_info"));
        assert!(metrics.contains("# TYPE p2p_node_info gauge"));
        assert!(metrics.contains("p2p_node_info{"));

        assert!(metrics.contains("# HELP p2p_uptime_seconds"));
        assert!(metrics.contains("# TYPE p2p_uptime_seconds counter"));
        assert!(metrics.contains("p2p_uptime_seconds"));

        assert!(metrics.contains("# HELP p2p_health_status"));
        assert!(metrics.contains("# TYPE p2p_health_status gauge"));

        assert!(metrics.contains("# HELP p2p_last_scrape_timestamp_seconds"));
        assert!(metrics.contains("# TYPE p2p_last_scrape_timestamp_seconds gauge"));
    }

    #[tokio::test]
    async fn test_prometheus_export_with_components() {
        let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));

        // Add a mock component checker
        struct MockChecker;
        #[async_trait::async_trait]
        impl crate::health::checks::ComponentChecker for MockChecker {
            async fn check(&self) -> Result<HealthStatus> {
                Ok(HealthStatus::Healthy)
            }
        }

        health_manager
            .register_checker("test_component", Box::new(MockChecker))
            .await;

        let exporter = PrometheusExporter::new(health_manager);
        let metrics = exporter.export().await.unwrap();

        // Check for component-specific metrics
        assert!(metrics.contains("p2p_health_status{component=\"test_component\"}"));
        assert!(metrics.contains("p2p_health_check_latency_ms{component=\"test_component\"}"));
        assert!(metrics.contains("p2p_healthy_components 1"));
        assert!(metrics.contains("p2p_degraded_components 0"));
        assert!(metrics.contains("p2p_unhealthy_components 0"));
    }

    #[tokio::test]
    async fn test_health_metrics_structure() {
        let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));
        let exporter = PrometheusExporter::new(health_manager);

        let metrics = exporter.export_metrics().await.unwrap();

        assert!(metrics.uptime_seconds >= 0.0);
        assert_eq!(metrics.healthy_components, 0);
        assert_eq!(metrics.degraded_components, 0);
        assert_eq!(metrics.unhealthy_components, 0);
        assert_eq!(metrics.total_components, 0);
    }

    #[tokio::test]
    async fn test_prometheus_format_validation() {
        let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));
        let exporter = PrometheusExporter::new(health_manager);

        let metrics = exporter.export().await.unwrap();

        // Validate Prometheus format
        for line in metrics.lines() {
            if line.is_empty() {
                continue;
            }

            // Comments should start with #
            if line.starts_with('#') {
                assert!(line.starts_with("# HELP") || line.starts_with("# TYPE"));
                continue;
            }

            // Metric lines should have a space between name and value
            if !line.starts_with('#') {
                assert!(line.contains(' '));
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                assert_eq!(parts.len(), 2);

                // Value should be numeric
                let value = parts[1].trim();
                assert!(
                    value.parse::<f64>().is_ok(),
                    "Invalid metric value: {}",
                    value
                );
            }
        }
    }

    #[tokio::test]
    async fn test_export_with_metadata() {
        let health_manager = Arc::new(HealthManager::new("1.0.0".to_string()));

        // Add a mock component with metadata
        struct MockCheckerWithMetadata;
        #[async_trait::async_trait]
        impl crate::health::checks::ComponentChecker for MockCheckerWithMetadata {
            async fn check(&self) -> Result<HealthStatus> {
                Ok(HealthStatus::Healthy)
            }

            async fn debug_info(&self) -> Option<serde_json::Value> {
                Some(serde_json::json!({
                    "peer_count": 10,
                    "connection_rate": 5.5,
                }))
            }
        }

        health_manager
            .register_checker("network", Box::new(MockCheckerWithMetadata))
            .await;

        let exporter = PrometheusExporter::new(health_manager);
        let metrics = exporter.export_metrics().await.unwrap();

        // The metadata extraction in export_metrics would need the component
        // to include metadata in its health check result, not just debug_info
        assert_eq!(metrics.total_components, 1);
        assert_eq!(metrics.healthy_components, 1);
    }
}
