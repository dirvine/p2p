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

//! Configuration management for the test suite
//!
//! Handles loading and managing test configuration from files,
//! environment variables, and command-line arguments.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Remote node configurations
    pub remote_nodes: Vec<RemoteNodeConfig>,
    
    /// Data verification settings
    pub verification: VerificationConfig,
    
    /// Performance testing parameters
    pub performance: PerformanceConfig,
    
    /// Logging and reporting settings
    pub reporting: ReportingConfig,
}

/// Network configuration for test nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Default local port for test nodes
    pub default_local_port: u16,
    
    /// Bootstrap nodes for network discovery
    pub bootstrap_nodes: Vec<String>,
    
    /// Connection timeout
    pub connection_timeout_secs: u64,
    
    /// Keep-alive interval
    pub keep_alive_interval_secs: u64,
    
    /// Enable IPv6 support
    pub enable_ipv6: bool,
    
    /// Enable MCP server
    pub enable_mcp_server: bool,
}

/// Remote node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNodeConfig {
    /// Node identifier (e.g., "do" for Digital Ocean)
    pub id: String,
    
    /// SSH connection string
    pub ssh_host: String,
    
    /// SSH username
    pub ssh_user: String,
    
    /// SSH key path (optional, uses default if not specified)
    pub ssh_key_path: Option<PathBuf>,
    
    /// Remote port for the test node
    pub remote_port: u16,
    
    /// Working directory on remote host
    pub working_dir: String,
    
    /// Binary path on remote host
    pub binary_path: String,
}

/// Data verification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Enable mandatory round-trip verification
    pub enable_round_trip: bool,
    
    /// Enable cross-node consistency checks
    pub enable_cross_node: bool,
    
    /// Enable hash verification for stored data
    pub enable_hash_verification: bool,
    
    /// Enable signature verification
    pub enable_signature_verification: bool,
    
    /// Verification timeout in seconds
    pub verification_timeout_secs: u64,
    
    /// Number of verification retries
    pub verification_retries: u32,
    
    /// Acceptable error rate (percentage)
    pub acceptable_error_rate: f64,
}

/// Performance testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Default number of operations for stress tests
    pub default_operations: u32,
    
    /// Default concurrency level
    pub default_concurrency: u32,
    
    /// Maximum file size for testing (in bytes)
    pub max_file_size: u64,
    
    /// Performance thresholds
    pub thresholds: PerformanceThresholds,
}

/// Performance threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Maximum acceptable latency for local operations (milliseconds)
    pub max_local_latency_ms: u64,
    
    /// Maximum acceptable latency for remote operations (milliseconds)
    pub max_remote_latency_ms: u64,
    
    /// Minimum acceptable throughput (bytes per second)
    pub min_throughput_bps: u64,
    
    /// Maximum acceptable memory usage (bytes)
    pub max_memory_usage: u64,
    
    /// Maximum acceptable CPU usage (percentage)
    pub max_cpu_usage: f64,
}

/// Reporting and logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfig {
    /// Default output directory for reports
    pub output_dir: PathBuf,
    
    /// Enable real-time monitoring
    pub enable_real_time_monitoring: bool,
    
    /// Monitoring check interval in seconds
    pub monitoring_interval_secs: u64,
    
    /// Enable alert notifications
    pub enable_alerts: bool,
    
    /// Alert webhook URL (optional)
    pub alert_webhook_url: Option<String>,
    
    /// Maximum log file size (bytes)
    pub max_log_file_size: u64,
    
    /// Number of log files to retain
    pub log_file_retention: u32,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                default_local_port: 9000,
                bootstrap_nodes: vec![
                    "/ip4/127.0.0.1/tcp/9001".to_string(),
                    "/ip6/::1/tcp/9001".to_string(),
                ],
                connection_timeout_secs: 30,
                keep_alive_interval_secs: 60,
                enable_ipv6: true,
                enable_mcp_server: true,
            },
            remote_nodes: vec![
                RemoteNodeConfig {
                    id: "do".to_string(),
                    ssh_host: "do".to_string(),
                    ssh_user: "root".to_string(),
                    ssh_key_path: None,
                    remote_port: 9001,
                    working_dir: "/tmp/ant-test".to_string(),
                    binary_path: "/tmp/ant-test/ant-test-node".to_string(),
                },
            ],
            verification: VerificationConfig {
                enable_round_trip: true,
                enable_cross_node: true,
                enable_hash_verification: true,
                enable_signature_verification: true,
                verification_timeout_secs: 30,
                verification_retries: 3,
                acceptable_error_rate: 0.0, // Zero tolerance for data corruption
            },
            performance: PerformanceConfig {
                default_operations: 1000,
                default_concurrency: 10,
                max_file_size: 100 * 1024 * 1024, // 100MB
                thresholds: PerformanceThresholds {
                    max_local_latency_ms: 100,
                    max_remote_latency_ms: 1000,
                    min_throughput_bps: 1024 * 1024, // 1MB/s
                    max_memory_usage: 1024 * 1024 * 1024, // 1GB
                    max_cpu_usage: 80.0, // 80%
                },
            },
            reporting: ReportingConfig {
                output_dir: PathBuf::from("./test-results"),
                enable_real_time_monitoring: true,
                monitoring_interval_secs: 30,
                enable_alerts: false,
                alert_webhook_url: None,
                max_log_file_size: 100 * 1024 * 1024, // 100MB
                log_file_retention: 10,
            },
        }
    }
}

impl TestConfig {
    /// Load configuration from file or create default
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        if let Some(path) = config_path {
            Self::load_from_file(path)
        } else {
            // Try to load from default locations
            let default_paths = [
                "./ant-test-config.toml",
                "~/.config/ant-test/config.toml",
                "/etc/ant-test/config.toml",
            ];

            for path in &default_paths {
                if let Ok(config) = Self::load_from_file(path) {
                    return Ok(config);
                }
            }

            // Return default configuration if no file found
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let expanded_path = shellexpand::tilde(path);
        let content = std::fs::read_to_string(expanded_path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path))?;
        
        let config: TestConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;
        
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let expanded_path = shellexpand::tilde(path);
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(expanded_path.as_ref()).parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }
        
        std::fs::write(expanded_path.as_ref(), content)
            .with_context(|| format!("Failed to write config file: {}", path))?;
        
        Ok(())
    }

    /// Get remote node configuration by ID
    pub fn get_remote_node(&self, id: &str) -> Option<&RemoteNodeConfig> {
        self.remote_nodes.iter().find(|node| node.id == id)
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.network.connection_timeout_secs)
    }

    /// Get keep-alive interval as Duration
    pub fn keep_alive_interval(&self) -> Duration {
        Duration::from_secs(self.network.keep_alive_interval_secs)
    }

    /// Get verification timeout as Duration
    pub fn verification_timeout(&self) -> Duration {
        Duration::from_secs(self.verification.verification_timeout_secs)
    }

    /// Get monitoring interval as Duration
    pub fn monitoring_interval(&self) -> Duration {
        Duration::from_secs(self.reporting.monitoring_interval_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = TestConfig::default();
        assert_eq!(config.network.default_local_port, 9000);
        assert!(config.verification.enable_round_trip);
        assert_eq!(config.verification.acceptable_error_rate, 0.0);
    }

    #[test]
    fn test_config_serialization() {
        let config = TestConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: TestConfig = toml::from_str(&serialized).unwrap();
        
        assert_eq!(config.network.default_local_port, deserialized.network.default_local_port);
    }

    #[test]
    fn test_config_file_operations() {
        let config = TestConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();
        
        // Save config
        config.save_to_file(temp_path).unwrap();
        
        // Load config
        let loaded_config = TestConfig::load_from_file(temp_path).unwrap();
        
        assert_eq!(config.network.default_local_port, loaded_config.network.default_local_port);
    }
}