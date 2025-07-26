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

//! Monitoring and metrics system for the adaptive P2P network
//!
//! This module provides comprehensive monitoring capabilities:
//! - Prometheus metrics export for external monitoring
//! - Real-time anomaly detection using statistical analysis
//! - Network health dashboards with key performance indicators
//! - Alert system for critical conditions
//! - Performance profiling for bottleneck detection
//! - Debug logging with configurable levels

use super::*;
use crate::adaptive::{
    ChurnHandler, AdaptiveRouter, AdaptiveGossipSub,
    ContentStore, ReplicationManager, learning::{ThompsonSampling, QLearnCacheManager},
};
use prometheus::{
    Encoder, TextEncoder, Counter, Gauge, Histogram,
    IntCounter, IntGauge, Registry, register_counter, register_gauge,
    register_histogram, register_int_counter, register_int_gauge,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{RwLock, mpsc};
use anyhow::Result;

/// Monitoring system for the adaptive P2P network
pub struct MonitoringSystem {
    /// Prometheus registry for metrics
    registry: Arc<Registry>,
    
    /// Core metrics
    metrics: Arc<NetworkMetrics>,
    
    /// Anomaly detector
    anomaly_detector: Arc<AnomalyDetector>,
    
    /// Alert manager
    alert_manager: Arc<AlertManager>,
    
    /// Performance profiler
    profiler: Arc<PerformanceProfiler>,
    
    /// Debug logger
    logger: Arc<DebugLogger>,
    
    /// Network components to monitor
    components: Arc<MonitoredComponents>,
    
    /// Configuration
    config: MonitoringConfig,
}

/// Configuration for monitoring system
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Metrics collection interval
    pub collection_interval: Duration,
    
    /// Anomaly detection window size
    pub anomaly_window_size: usize,
    
    /// Alert cooldown period
    pub alert_cooldown: Duration,
    
    /// Performance sampling rate (0.0-1.0)
    pub profiling_sample_rate: f64,
    
    /// Debug log level
    pub log_level: LogLevel,
    
    /// Dashboard update interval
    pub dashboard_interval: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(5),
            anomaly_window_size: 100,
            alert_cooldown: Duration::from_secs(300),
            profiling_sample_rate: 0.01,
            log_level: LogLevel::Info,
            dashboard_interval: Duration::from_secs(1),
        }
    }
}

/// Log levels for debug logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Core network metrics exposed via Prometheus
struct NetworkMetrics {
    // Node metrics
    connected_nodes: IntGauge,
    active_nodes: IntGauge,
    suspicious_nodes: IntGauge,
    failed_nodes: IntGauge,
    
    // Routing metrics
    routing_requests: Counter,
    routing_success: Counter,
    routing_latency: Histogram,
    
    // Storage metrics
    stored_items: IntGauge,
    storage_bytes: IntGauge,
    replication_factor: Gauge,
    
    // Network traffic metrics
    messages_sent: Counter,
    messages_received: Counter,
    bytes_sent: Counter,
    bytes_received: Counter,
    
    // Cache metrics
    cache_hits: Counter,
    cache_misses: Counter,
    cache_size: IntGauge,
    cache_evictions: Counter,
    
    // Learning metrics
    thompson_selections: IntCounter,
    qlearn_updates: Counter,
    churn_predictions: Counter,
    
    // Gossip metrics
    gossip_messages: Counter,
    mesh_size: IntGauge,
    topic_count: IntGauge,
    
    // Performance metrics
    cpu_usage: Gauge,
    memory_usage: IntGauge,
    thread_count: IntGauge,
}

/// Components being monitored
pub struct MonitoredComponents {
    pub router: Arc<AdaptiveRouter>,
    pub churn_handler: Arc<ChurnHandler>,
    pub gossip: Arc<AdaptiveGossipSub>,
    pub storage: Arc<ContentStore>,
    pub replication: Arc<ReplicationManager>,
    pub thompson: Arc<ThompsonSampling>,
    pub cache: Arc<QLearnCacheManager>,
}

/// Anomaly detection system
pub struct AnomalyDetector {
    /// Historical data for each metric
    history: Arc<RwLock<HashMap<String, MetricHistory>>>,
    
    /// Detected anomalies
    anomalies: Arc<RwLock<Vec<Anomaly>>>,
    
    /// Configuration
    window_size: usize,
}

/// Historical data for a metric
struct MetricHistory {
    /// Sliding window of values
    values: VecDeque<f64>,
    
    /// Running statistics
    mean: f64,
    std_dev: f64,
    
    /// Last update time
    last_update: Instant,
}

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Metric name
    pub metric: String,
    
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    
    /// Severity (0.0-1.0)
    pub severity: f64,
    
    /// Detection time
    pub detected_at: Instant,
    
    /// Current value
    pub value: f64,
    
    /// Expected range
    pub expected_range: (f64, f64),
}

/// Types of anomalies
#[derive(Debug, Clone, PartialEq)]
pub enum AnomalyType {
    /// Value outside statistical bounds
    Statistical,
    
    /// Sudden spike in value
    Spike,
    
    /// Gradual drift from normal
    Drift,
    
    /// Unusual pattern
    Pattern,
}

/// Alert management system
pub struct AlertManager {
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    
    /// Alert rules
    rules: Arc<RwLock<Vec<AlertRule>>>,
    
    /// Alert channels
    channels: Arc<RwLock<Vec<Box<dyn AlertChannel>>>>,
    
    /// Cooldown tracking
    cooldowns: Arc<RwLock<HashMap<String, Instant>>>,
    
    /// Configuration
    cooldown_period: Duration,
}

/// Alert definition
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    
    /// Alert name
    pub name: String,
    
    /// Severity level
    pub severity: AlertSeverity,
    
    /// Alert message
    pub message: String,
    
    /// Triggered at
    pub triggered_at: Instant,
    
    /// Associated metrics
    pub metrics: HashMap<String, f64>,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert rule definition
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    
    /// Condition to check
    pub condition: AlertCondition,
    
    /// Severity if triggered
    pub severity: AlertSeverity,
    
    /// Message template
    pub message_template: String,
}

/// Alert conditions
#[derive(Debug, Clone)]
pub enum AlertCondition {
    /// Metric above threshold
    Above { metric: String, threshold: f64 },
    
    /// Metric below threshold
    Below { metric: String, threshold: f64 },
    
    /// Metric rate of change
    RateOfChange { metric: String, threshold: f64 },
    
    /// Anomaly detected
    AnomalyDetected { metric: String },
}

/// Alert channel trait
#[async_trait]
pub trait AlertChannel: Send + Sync {
    /// Send an alert
    async fn send_alert(&self, alert: &Alert) -> Result<()>;
}

/// Performance profiler
pub struct PerformanceProfiler {
    /// Active profiles
    profiles: Arc<RwLock<HashMap<String, Profile>>>,
    
    /// Completed profiles
    completed: Arc<RwLock<VecDeque<CompletedProfile>>>,
    
    /// Sampling rate
    sample_rate: f64,
}

/// Active performance profile
struct Profile {
    /// Profile name
    name: String,
    
    /// Start time
    started_at: Instant,
    
    /// Samples collected
    samples: Vec<ProfileSample>,
}

/// Profile sample
#[derive(Debug, Clone)]
struct ProfileSample {
    /// Timestamp
    timestamp: Instant,
    
    /// CPU usage
    cpu_usage: f64,
    
    /// Memory usage
    memory_bytes: u64,
    
    /// Active operations
    operations: HashMap<String, u64>,
}

/// Completed profile
#[derive(Debug, Clone)]
pub struct CompletedProfile {
    /// Profile name
    pub name: String,
    
    /// Duration
    pub duration: Duration,
    
    /// Average CPU usage
    pub avg_cpu: f64,
    
    /// Peak memory usage
    pub peak_memory: u64,
    
    /// Operation counts
    pub operations: HashMap<String, u64>,
}

/// Debug logger
pub struct DebugLogger {
    /// Log level
    level: LogLevel,
    
    /// Log buffer
    buffer: Arc<RwLock<VecDeque<LogEntry>>>,
    
    /// Log channels
    channels: Arc<RwLock<Vec<mpsc::UnboundedSender<LogEntry>>>>,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp
    pub timestamp: Instant,
    
    /// Log level
    pub level: LogLevel,
    
    /// Component
    pub component: String,
    
    /// Message
    pub message: String,
    
    /// Associated data
    pub data: Option<serde_json::Value>,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new(
        components: MonitoredComponents,
        config: MonitoringConfig,
    ) -> Result<Self> {
        let registry = Registry::new();
        
        // Initialize metrics
        let metrics = NetworkMetrics {
            // Node metrics
            connected_nodes: register_int_gauge!("p2p_connected_nodes", "Number of connected nodes")?,
            active_nodes: register_int_gauge!("p2p_active_nodes", "Number of active nodes")?,
            suspicious_nodes: register_int_gauge!("p2p_suspicious_nodes", "Number of suspicious nodes")?,
            failed_nodes: register_int_gauge!("p2p_failed_nodes", "Number of failed nodes")?,
            
            // Routing metrics
            routing_requests: register_counter!("p2p_routing_requests_total", "Total routing requests")?,
            routing_success: register_counter!("p2p_routing_success_total", "Successful routing requests")?,
            routing_latency: register_histogram!(
                "p2p_routing_latency_seconds",
                "Routing request latency in seconds"
            )?,
            
            // Storage metrics
            stored_items: register_int_gauge!("p2p_stored_items", "Number of stored items")?,
            storage_bytes: register_int_gauge!("p2p_storage_bytes", "Total storage in bytes")?,
            replication_factor: register_gauge!("p2p_replication_factor", "Average replication factor")?,
            
            // Network traffic metrics
            messages_sent: register_counter!("p2p_messages_sent_total", "Total messages sent")?,
            messages_received: register_counter!("p2p_messages_received_total", "Total messages received")?,
            bytes_sent: register_counter!("p2p_bytes_sent_total", "Total bytes sent")?,
            bytes_received: register_counter!("p2p_bytes_received_total", "Total bytes received")?,
            
            // Cache metrics
            cache_hits: register_counter!("p2p_cache_hits_total", "Total cache hits")?,
            cache_misses: register_counter!("p2p_cache_misses_total", "Total cache misses")?,
            cache_size: register_int_gauge!("p2p_cache_size_bytes", "Cache size in bytes")?,
            cache_evictions: register_counter!("p2p_cache_evictions_total", "Total cache evictions")?,
            
            // Learning metrics
            thompson_selections: register_int_counter!(
                "p2p_thompson_selections_total",
                "Thompson sampling strategy selections"
            )?,
            qlearn_updates: register_counter!("p2p_qlearn_updates_total", "Q-learning updates")?,
            churn_predictions: register_counter!("p2p_churn_predictions_total", "Churn predictions made")?,
            
            // Gossip metrics
            gossip_messages: register_counter!("p2p_gossip_messages_total", "Total gossip messages")?,
            mesh_size: register_int_gauge!("p2p_gossip_mesh_size", "Gossip mesh size")?,
            topic_count: register_int_gauge!("p2p_gossip_topics", "Number of gossip topics")?,
            
            // Performance metrics
            cpu_usage: register_gauge!("p2p_cpu_usage_percent", "CPU usage percentage")?,
            memory_usage: register_int_gauge!("p2p_memory_usage_bytes", "Memory usage in bytes")?,
            thread_count: register_int_gauge!("p2p_thread_count", "Number of threads")?,
        };
        
        let anomaly_detector = Arc::new(AnomalyDetector::new(config.anomaly_window_size));
        let alert_manager = Arc::new(AlertManager::new(config.alert_cooldown));
        let profiler = Arc::new(PerformanceProfiler::new(config.profiling_sample_rate));
        let logger = Arc::new(DebugLogger::new(config.log_level));
        
        // Set up default alert rules
        let monitoring = Self {
            registry: Arc::new(registry),
            metrics: Arc::new(metrics),
            anomaly_detector,
            alert_manager,
            profiler,
            logger,
            components: Arc::new(components),
            config,
        };
        
        Ok(monitoring)
    }
    
    /// Start monitoring
    pub async fn start(&self) {
        let interval = self.config.collection_interval;
        let monitoring = self.clone_for_task();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                
                if let Err(e) = monitoring.collect_metrics().await {
                    // Log error but continue monitoring
                    monitoring.logger.error("monitoring", &format!("Metric collection error: {e}")).await;
                }
            }
        });
        
        // Start anomaly detection
        self.start_anomaly_detection().await;
        
        // Start alert processing
        self.start_alert_processing().await;
    }
    
    /// Collect metrics from all components
    async fn collect_metrics(&self) -> Result<()> {
        // Collect churn statistics
        let churn_stats = self.components.churn_handler.get_stats().await;
        self.metrics.active_nodes.set(churn_stats.active_nodes as i64);
        self.metrics.suspicious_nodes.set(churn_stats.suspicious_nodes as i64);
        self.metrics.failed_nodes.set(churn_stats.failed_nodes as i64);
        
        // Collect routing statistics
        let routing_stats = self.components.router.get_stats().await;
        self.metrics.routing_requests.inc_by(routing_stats.total_requests as f64);
        self.metrics.routing_success.inc_by(routing_stats.successful_requests as f64);
        
        // Collect storage statistics
        let storage_stats = self.components.storage.get_stats().await;
        self.metrics.stored_items.set(storage_stats.total_items as i64);
        self.metrics.storage_bytes.set(storage_stats.total_bytes as i64);
        
        // Collect gossip statistics
        let gossip_stats = self.components.gossip.get_stats().await;
        self.metrics.gossip_messages.inc_by(gossip_stats.messages_sent as f64);
        self.metrics.mesh_size.set(gossip_stats.mesh_size as i64);
        self.metrics.topic_count.set(gossip_stats.topic_count as i64);
        
        // Collect cache statistics
        let cache_stats = self.components.cache.get_stats();
        self.metrics.cache_hits.inc_by(cache_stats.hits as f64);
        self.metrics.cache_misses.inc_by(cache_stats.misses as f64);
        self.metrics.cache_size.set(cache_stats.size_bytes as i64);
        
        // Update anomaly detector
        self.update_anomaly_detector().await?;
        
        Ok(())
    }
    
    /// Export metrics in Prometheus format
    pub fn export_metrics(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| anyhow::anyhow!("UTF-8 error: {}", e))
    }
    
    /// Get current network health
    pub async fn get_health(&self) -> NetworkHealth {
        let churn_stats = self.components.churn_handler.get_stats().await;
        let routing_stats = self.components.router.get_stats().await;
        let storage_stats = self.components.storage.get_stats().await;
        
        let health_score = self.calculate_health_score(&churn_stats, &routing_stats, &storage_stats);
        
        NetworkHealth {
            score: health_score,
            status: if health_score > 0.8 {
                HealthStatus::Healthy
            } else if health_score > 0.5 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Critical
            },
            active_nodes: churn_stats.active_nodes,
            churn_rate: churn_stats.churn_rate,
            routing_success_rate: routing_stats.success_rate(),
            storage_utilization: storage_stats.utilization(),
            active_alerts: self.alert_manager.get_active_alerts().await.len(),
        }
    }
    
    /// Calculate overall health score
    fn calculate_health_score(
        &self,
        churn_stats: &crate::adaptive::churn::ChurnStats,
        routing_stats: &crate::adaptive::routing::RoutingStats,
        storage_stats: &crate::adaptive::storage::StorageStats,
    ) -> f64 {
        let mut score = 1.0;
        
        // Penalize high churn
        if churn_stats.churn_rate > 0.3 {
            score *= 0.7;
        } else if churn_stats.churn_rate > 0.1 {
            score *= 0.9;
        }
        
        // Penalize low routing success
        let routing_success = routing_stats.success_rate();
        if routing_success < 0.9 {
            score *= routing_success;
        }
        
        // Penalize high storage utilization
        let storage_util = storage_stats.utilization();
        if storage_util > 0.9 {
            score *= 0.8;
        }
        
        score
    }
    
    /// Start anomaly detection
    async fn start_anomaly_detection(&self) {
        let detector = self.anomaly_detector.clone();
        let alert_manager = self.alert_manager.clone();
        let logger = self.logger.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                let anomalies = detector.get_recent_anomalies().await;
                for anomaly in anomalies {
                    // Log anomaly
                    logger.warn("anomaly_detector", &format!("Anomaly detected: {anomaly:?}")).await;
                    
                    // Create alert if severe
                    if anomaly.severity > 0.7 {
                        let alert = Alert {
                            id: format!("anomaly_{}", anomaly.metric),
                            name: format!("{} Anomaly", anomaly.metric),
                            severity: AlertSeverity::Warning,
                            message: format!(
                                "Anomaly detected in {}: value {} outside expected range {:?}",
                                anomaly.metric, anomaly.value, anomaly.expected_range
                            ),
                            triggered_at: Instant::now(),
                            metrics: HashMap::from([(anomaly.metric.clone(), anomaly.value)]),
                        };
                        
                        let _ = alert_manager.trigger_alert(alert).await;
                    }
                }
            }
        });
    }
    
    /// Start alert processing
    async fn start_alert_processing(&self) {
        let alert_manager = self.alert_manager.clone();
        
        // Add default alert rules
        let rules = vec![
            AlertRule {
                name: "High Churn Rate".to_string(),
                condition: AlertCondition::Above {
                    metric: "churn_rate".to_string(),
                    threshold: 0.5,
                },
                severity: AlertSeverity::Critical,
                message_template: "Churn rate is critically high: {value}".to_string(),
            },
            AlertRule {
                name: "Low Routing Success".to_string(),
                condition: AlertCondition::Below {
                    metric: "routing_success_rate".to_string(),
                    threshold: 0.8,
                },
                severity: AlertSeverity::Warning,
                message_template: "Routing success rate is low: {value}".to_string(),
            },
            AlertRule {
                name: "Storage Near Capacity".to_string(),
                condition: AlertCondition::Above {
                    metric: "storage_utilization".to_string(),
                    threshold: 0.9,
                },
                severity: AlertSeverity::Warning,
                message_template: "Storage utilization is high: {value}".to_string(),
            },
        ];
        
        for rule in rules {
            alert_manager.add_rule(rule).await;
        }
        
        // Start rule evaluation
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                let _ = alert_manager.evaluate_rules().await;
            }
        });
    }
    
    /// Update anomaly detector with current metrics
    async fn update_anomaly_detector(&self) -> Result<()> {
        // Update with key metrics
        let churn_stats = self.components.churn_handler.get_stats().await;
        self.anomaly_detector.update_metric("churn_rate", churn_stats.churn_rate).await;
        
        let routing_stats = self.components.router.get_stats().await;
        self.anomaly_detector.update_metric("routing_success_rate", routing_stats.success_rate()).await;
        
        let storage_stats = self.components.storage.get_stats().await;
        self.anomaly_detector.update_metric("storage_utilization", storage_stats.utilization()).await;
        
        Ok(())
    }
    
    /// Get dashboard data
    pub async fn get_dashboard_data(&self) -> DashboardData {
        DashboardData {
            health: self.get_health().await,
            metrics: self.get_current_metrics().await,
            recent_alerts: self.alert_manager.get_recent_alerts(10).await,
            anomalies: self.anomaly_detector.get_recent_anomalies().await,
            performance: self.profiler.get_current_profile().await,
        }
    }
    
    /// Get current metric values
    async fn get_current_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        
        // Collect current values
        let churn_stats = self.components.churn_handler.get_stats().await;
        metrics.insert("active_nodes".to_string(), churn_stats.active_nodes as f64);
        metrics.insert("churn_rate".to_string(), churn_stats.churn_rate);
        
        let routing_stats = self.components.router.get_stats().await;
        metrics.insert("routing_success_rate".to_string(), routing_stats.success_rate());
        
        let storage_stats = self.components.storage.get_stats().await;
        metrics.insert("storage_items".to_string(), storage_stats.total_items as f64);
        metrics.insert("storage_bytes".to_string(), storage_stats.total_bytes as f64);
        
        metrics
    }
    
    /// Clone for spawning tasks
    fn clone_for_task(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            metrics: self.metrics.clone(),
            anomaly_detector: self.anomaly_detector.clone(),
            alert_manager: self.alert_manager.clone(),
            profiler: self.profiler.clone(),
            logger: self.logger.clone(),
            components: self.components.clone(),
            config: self.config.clone(),
        }
    }
}

impl AnomalyDetector {
    /// Create a new anomaly detector
    pub fn new(window_size: usize) -> Self {
        Self {
            history: Arc::new(RwLock::new(HashMap::new())),
            anomalies: Arc::new(RwLock::new(Vec::new())),
            window_size,
        }
    }
    
    /// Update a metric value
    pub async fn update_metric(&self, metric: &str, value: f64) {
        let mut history = self.history.write().await;
        
        let metric_history = history.entry(metric.to_string()).or_insert_with(|| {
            MetricHistory {
                values: VecDeque::new(),
                mean: 0.0,
                std_dev: 0.0,
                last_update: Instant::now(),
            }
        });
        
        // Add value to sliding window
        metric_history.values.push_back(value);
        if metric_history.values.len() > self.window_size {
            metric_history.values.pop_front();
        }
        
        // Update statistics
        if metric_history.values.len() >= 10 {
            let sum: f64 = metric_history.values.iter().sum();
            metric_history.mean = sum / metric_history.values.len() as f64;
            
            let variance: f64 = metric_history.values.iter()
                .map(|v| (v - metric_history.mean).powi(2))
                .sum::<f64>() / metric_history.values.len() as f64;
            metric_history.std_dev = variance.sqrt();
            
            // Check for anomalies
            if let Some(anomaly) = self.detect_anomaly(metric, value, metric_history) {
                let mut anomalies = self.anomalies.write().await;
                anomalies.push(anomaly);
                
                // Keep only recent anomalies
                if anomalies.len() > 1000 {
                    anomalies.drain(0..100);
                }
            }
        }
        
        metric_history.last_update = Instant::now();
    }
    
    /// Detect anomaly in metric
    fn detect_anomaly(
        &self,
        metric: &str,
        value: f64,
        history: &MetricHistory,
    ) -> Option<Anomaly> {
        // Statistical anomaly detection (3-sigma rule)
        let z_score = (value - history.mean).abs() / history.std_dev;
        if z_score > 3.0 {
            return Some(Anomaly {
                metric: metric.to_string(),
                anomaly_type: AnomalyType::Statistical,
                severity: (z_score - 3.0).min(1.0),
                detected_at: Instant::now(),
                value,
                expected_range: (
                    history.mean - 3.0 * history.std_dev,
                    history.mean + 3.0 * history.std_dev,
                ),
            });
        }
        
        // Spike detection
        if history.values.len() >= 2 {
            let prev_value = history.values[history.values.len() - 2];
            let change_rate = (value - prev_value).abs() / prev_value.abs().max(1.0);
            
            if change_rate > 0.5 {
                return Some(Anomaly {
                    metric: metric.to_string(),
                    anomaly_type: AnomalyType::Spike,
                    severity: change_rate.min(1.0),
                    detected_at: Instant::now(),
                    value,
                    expected_range: (prev_value * 0.5, prev_value * 1.5),
                });
            }
        }
        
        None
    }
    
    /// Get recent anomalies
    pub async fn get_recent_anomalies(&self) -> Vec<Anomaly> {
        let anomalies = self.anomalies.read().await;
        let cutoff = Instant::now() - Duration::from_secs(300);
        
        anomalies.iter()
            .filter(|a| a.detected_at > cutoff)
            .cloned()
            .collect()
    }
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(cooldown_period: Duration) -> Self {
        Self {
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            rules: Arc::new(RwLock::new(Vec::new())),
            channels: Arc::new(RwLock::new(Vec::new())),
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
            cooldown_period,
        }
    }
    
    /// Add an alert rule
    pub async fn add_rule(&self, rule: AlertRule) -> Result<()> {
        let mut rules = self.rules.write().await;
        rules.push(rule);
        Ok(())
    }
    
    /// Add an alert channel
    pub async fn add_channel(&self, channel: Box<dyn AlertChannel>) {
        let mut channels = self.channels.write().await;
        channels.push(channel);
    }
    
    /// Trigger an alert
    pub async fn trigger_alert(&self, alert: Alert) -> Result<()> {
        // Check cooldown
        let mut cooldowns = self.cooldowns.write().await;
        if let Some(last_trigger) = cooldowns.get(&alert.id) {
            if last_trigger.elapsed() < self.cooldown_period {
                return Ok(()); // Skip due to cooldown
            }
        }
        
        // Record alert
        let mut active_alerts = self.active_alerts.write().await;
        active_alerts.insert(alert.id.clone(), alert.clone());
        cooldowns.insert(alert.id.clone(), Instant::now());
        
        // Send to all channels
        let channels = self.channels.read().await;
        for channel in channels.iter() {
            let _ = channel.send_alert(&alert).await;
        }
        
        Ok(())
    }
    
    /// Evaluate all rules
    pub async fn evaluate_rules(&self) -> Result<()> {
        let rules = self.rules.read().await.clone();
        
        for rule in rules {
            // Evaluate condition
            // This would check actual metric values
            // For now, this is a placeholder
        }
        
        Ok(())
    }
    
    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().await.values().cloned().collect()
    }
    
    /// Get recent alerts
    pub async fn get_recent_alerts(&self, count: usize) -> Vec<Alert> {
        let mut alerts: Vec<_> = self.active_alerts.read().await.values().cloned().collect();
        alerts.sort_by_key(|a| std::cmp::Reverse(a.triggered_at));
        alerts.truncate(count);
        alerts
    }
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new(sample_rate: f64) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            completed: Arc::new(RwLock::new(VecDeque::new())),
            sample_rate,
        }
    }
    
    /// Start a profile
    pub async fn start_profile(&self, name: String) {
        if rand::random::<f64>() > self.sample_rate {
            return; // Skip based on sampling rate
        }
        
        let mut profiles = self.profiles.write().await;
        profiles.insert(name.clone(), Profile {
            name,
            started_at: Instant::now(),
            samples: Vec::new(),
        });
    }
    
    /// Record a sample
    pub async fn record_sample(&self, profile_name: &str) {
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(profile_name) {
            profile.samples.push(ProfileSample {
                timestamp: Instant::now(),
                cpu_usage: Self::get_cpu_usage(),
                memory_bytes: Self::get_memory_usage(),
                operations: HashMap::new(), // Would track actual operations
            });
        }
    }
    
    /// End a profile
    pub async fn end_profile(&self, name: &str) {
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.remove(name) {
            let duration = profile.started_at.elapsed();
            
            let avg_cpu = profile.samples.iter()
                .map(|s| s.cpu_usage)
                .sum::<f64>() / profile.samples.len().max(1) as f64;
            
            let peak_memory = profile.samples.iter()
                .map(|s| s.memory_bytes)
                .max()
                .unwrap_or(0);
            
            let completed_profile = CompletedProfile {
                name: profile.name,
                duration,
                avg_cpu,
                peak_memory,
                operations: HashMap::new(), // Aggregate operations
            };
            
            let mut completed = self.completed.write().await;
            completed.push_back(completed_profile);
            
            // Keep only recent profiles
            if completed.len() > 100 {
                completed.pop_front();
            }
        }
    }
    
    /// Get current profile data
    pub async fn get_current_profile(&self) -> Option<ProfileData> {
        Some(ProfileData {
            cpu_usage: Self::get_cpu_usage(),
            memory_bytes: Self::get_memory_usage(),
            thread_count: Self::get_thread_count(),
            active_profiles: self.profiles.read().await.len(),
        })
    }
    
    /// Get CPU usage (placeholder implementation)
    fn get_cpu_usage() -> f64 {
        // In real implementation, would use platform-specific APIs
        rand::random::<f64>() * 100.0
    }
    
    /// Get memory usage (placeholder implementation)
    fn get_memory_usage() -> u64 {
        // In real implementation, would use platform-specific APIs
        1024 * 1024 * 512 // 512MB placeholder
    }
    
    /// Get thread count (placeholder implementation)
    fn get_thread_count() -> usize {
        // In real implementation, would use platform-specific APIs
        8
    }
}

impl DebugLogger {
    /// Create a new debug logger
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            channels: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Log a message
    pub async fn log(&self, level: LogLevel, component: &str, message: &str) {
        if level > self.level {
            return; // Skip lower priority logs
        }
        
        let entry = LogEntry {
            timestamp: Instant::now(),
            level,
            component: component.to_string(),
            message: message.to_string(),
            data: None,
        };
        
        // Add to buffer
        let mut buffer = self.buffer.write().await;
        buffer.push_back(entry.clone());
        
        // Keep buffer size limited
        if buffer.len() > 10000 {
            buffer.pop_front();
        }
        
        // Send to channels
        let channels = self.channels.read().await;
        for channel in channels.iter() {
            let _ = channel.send(entry.clone());
        }
    }
    
    /// Log error
    pub async fn error(&self, component: &str, message: &str) {
        self.log(LogLevel::Error, component, message).await;
    }
    
    /// Log warning
    pub async fn warn(&self, component: &str, message: &str) {
        self.log(LogLevel::Warn, component, message).await;
    }
    
    /// Log info
    pub async fn info(&self, component: &str, message: &str) {
        self.log(LogLevel::Info, component, message).await;
    }
    
    /// Log debug
    pub async fn debug(&self, component: &str, message: &str) {
        self.log(LogLevel::Debug, component, message).await;
    }
    
    /// Log trace
    pub async fn trace(&self, component: &str, message: &str) {
        self.log(LogLevel::Trace, component, message).await;
    }
    
    /// Subscribe to log stream
    pub async fn subscribe(&self) -> mpsc::UnboundedReceiver<LogEntry> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut channels = self.channels.write().await;
        channels.push(tx);
        rx
    }
    
    /// Get recent logs
    pub async fn get_recent_logs(&self, count: usize) -> Vec<LogEntry> {
        let buffer = self.buffer.read().await;
        buffer.iter().rev().take(count).cloned().collect()
    }
}

/// Network health status
#[derive(Debug, Clone)]
pub struct NetworkHealth {
    /// Overall health score (0.0-1.0)
    pub score: f64,
    
    /// Health status
    pub status: HealthStatus,
    
    /// Number of active nodes
    pub active_nodes: u64,
    
    /// Current churn rate
    pub churn_rate: f64,
    
    /// Routing success rate
    pub routing_success_rate: f64,
    
    /// Storage utilization
    pub storage_utilization: f64,
    
    /// Number of active alerts
    pub active_alerts: usize,
}

/// Health status levels
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

/// Dashboard data structure
#[derive(Debug, Clone)]
pub struct DashboardData {
    /// Current health
    pub health: NetworkHealth,
    
    /// Current metric values
    pub metrics: HashMap<String, f64>,
    
    /// Recent alerts
    pub recent_alerts: Vec<Alert>,
    
    /// Recent anomalies
    pub anomalies: Vec<Anomaly>,
    
    /// Performance data
    pub performance: Option<ProfileData>,
}

/// Current profile data
#[derive(Debug, Clone)]
pub struct ProfileData {
    /// CPU usage percentage
    pub cpu_usage: f64,
    
    /// Memory usage in bytes
    pub memory_bytes: u64,
    
    /// Number of threads
    pub thread_count: usize,
    
    /// Active profiles
    pub active_profiles: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_anomaly_detection() {
        let detector = AnomalyDetector::new(100);
        
        // Add normal values
        for i in 0..50 {
            detector.update_metric("test_metric", 50.0 + (i as f64 % 10.0)).await;
        }
        
        // Add anomalous value
        detector.update_metric("test_metric", 200.0).await;
        
        // Check anomalies detected
        let anomalies = detector.get_recent_anomalies().await;
        assert!(!anomalies.is_empty());
        assert_eq!(anomalies[0].anomaly_type, AnomalyType::Statistical);
    }
    
    #[tokio::test]
    async fn test_alert_cooldown() {
        let alert_manager = AlertManager::new(Duration::from_secs(60));
        
        let alert = Alert {
            id: "test_alert".to_string(),
            name: "Test Alert".to_string(),
            severity: AlertSeverity::Warning,
            message: "Test message".to_string(),
            triggered_at: Instant::now(),
            metrics: HashMap::new(),
        };
        
        // First alert should trigger
        alert_manager.trigger_alert(alert.clone()).await.unwrap();
        assert_eq!(alert_manager.get_active_alerts().await.len(), 1);
        
        // Second alert should be skipped due to cooldown
        alert_manager.trigger_alert(alert).await.unwrap();
        assert_eq!(alert_manager.get_active_alerts().await.len(), 1);
    }
    
    #[tokio::test]
    async fn test_performance_profiling() {
        let profiler = PerformanceProfiler::new(1.0); // 100% sampling
        
        profiler.start_profile("test_operation".to_string()).await;
        
        // Record some samples
        for _ in 0..5 {
            profiler.record_sample("test_operation").await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        profiler.end_profile("test_operation").await;
        
        // Check profile was completed
        let completed = profiler.completed.read().await;
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].name, "test_operation");
    }
    
    #[tokio::test]
    async fn test_debug_logging() {
        let logger = DebugLogger::new(LogLevel::Debug);
        
        // Subscribe to logs
        let mut rx = logger.subscribe().await;
        
        // Log messages
        logger.error("test", "Error message").await;
        logger.warn("test", "Warning message").await;
        logger.info("test", "Info message").await;
        logger.debug("test", "Debug message").await;
        logger.trace("test", "Trace message").await; // Should be filtered
        
        // Check received logs
        let mut count = 0;
        while let Ok(Some(entry)) = rx.try_recv() {
            count += 1;
            assert!(entry.level <= LogLevel::Debug);
        }
        assert_eq!(count, 4); // Trace should be filtered out
    }
}