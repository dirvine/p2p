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

//! Real-time test reporter with terminal UI

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{stdout, Write},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{Mutex, RwLock},
    time::interval,
};

use super::test_network::{ConnectivityReport, NetworkStatus, NetworkTopology};

/// Test reporter for real-time progress tracking
pub struct TestReporter {
    terminal: Arc<Mutex<Terminal>>,
    log_file: Arc<Mutex<File>>,
    metrics: Arc<RwLock<TestMetrics>>,
    start_time: Instant,
    verbose: bool,
}

impl TestReporter {
    /// Create a new test reporter
    pub async fn new(verbose: bool) -> Result<Self> {
        // Create log file
        let log_path = format!("test_run_{}.log", 
            chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let log_file = File::create(&log_path).await
            .context("Failed to create log file")?;
        
        // Initialize terminal
        let terminal = Terminal::new()?;
        
        Ok(Self {
            terminal: Arc::new(Mutex::new(terminal)),
            log_file: Arc::new(Mutex::new(log_file)),
            metrics: Arc::new(RwLock::new(TestMetrics::default())),
            start_time: Instant::now(),
            verbose,
        })
    }
    
    /// Report test progress in real-time
    pub async fn report_progress(&self, event: TestEvent) {
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_events += 1;
            
            match &event.event_type {
                TestEventType::NodeStarted => metrics.nodes_started += 1,
                TestEventType::ConnectionEstablished => metrics.connections_established += 1,
                TestEventType::TunnelCreated => metrics.tunnels_created += 1,
                TestEventType::DHTOperation => metrics.dht_operations += 1,
                TestEventType::ChatMessage => metrics.chat_messages += 1,
                TestEventType::ProjectCreated => metrics.projects_created += 1,
                TestEventType::ThresholdSigning => metrics.threshold_signings += 1,
                TestEventType::MCPToolCall => metrics.mcp_tool_calls += 1,
                TestEventType::Error(_) => metrics.errors += 1,
            }
            
            if event.success {
                metrics.successful_events += 1;
            } else {
                metrics.failed_events += 1;
            }
        }
        
        // Log to file
        self.log_event(&event).await;
        
        // Update terminal display
        if self.verbose || !event.success || matches!(event.event_type, TestEventType::Error(_)) {
            self.display_event(&event).await;
        }
    }
    
    /// Log event to file
    async fn log_event(&self, event: &TestEvent) {
        let mut log_file = self.log_file.lock().await;
        let log_entry = format!("[{}] {} - {} - {:?} - Success: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            event.node_id,
            event.event_type.to_string(),
            event.details,
            event.success
        );
        
        let _ = log_file.write_all(log_entry.as_bytes()).await;
        let _ = log_file.flush().await;
    }
    
    /// Display event in terminal
    async fn display_event(&self, event: &TestEvent) {
        let mut terminal = self.terminal.lock().await;
        
        let icon = if event.success { "✅" } else { "❌" };
        let color = if event.success { Color::Green } else { Color::Red };
        
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        let message = format!("[{}] {} {} - {}",
            timestamp,
            icon,
            event.event_type.to_string(),
            event.node_id
        );
        
        terminal.add_event(message, color);
        terminal.refresh().await;
    }
    
    /// Display live network topology
    pub async fn display_topology(&self, topology: &NetworkTopology) {
        let mut terminal = self.terminal.lock().await;
        terminal.update_topology(topology);
        terminal.refresh().await;
    }
    
    /// Show tunnel usage statistics
    pub async fn show_tunnel_stats(&self, stats: &TunnelStats) {
        let mut terminal = self.terminal.lock().await;
        terminal.update_tunnel_stats(stats);
        terminal.refresh().await;
    }
    
    /// Display connectivity report
    pub async fn display_connectivity_report(&self, report: &ConnectivityReport) {
        let mut terminal = self.terminal.lock().await;
        
        let success_rate = if report.total_nodes > 0 {
            (report.connected_pairs as f64 / report.total_nodes as f64) * 100.0
        } else {
            0.0
        };
        
        terminal.add_event(
            format!("🔗 Connectivity: {}/{} pairs connected ({:.1}%)",
                report.connected_pairs,
                report.total_nodes,
                success_rate
            ),
            Color::Cyan
        );
        
        if !report.failed_pairs.is_empty() {
            terminal.add_event(
                format!("⚠️  Failed connections: {:?}", report.failed_pairs),
                Color::Yellow
            );
        }
        
        terminal.refresh().await;
    }
    
    /// Show network summary
    pub async fn show_network_summary(&self, network: &super::test_network::DistributedTestNetwork) {
        let mut terminal = self.terminal.lock().await;
        let status = network.get_network_status();
        
        terminal.update_network_status(&status);
        terminal.refresh().await;
    }
    
    /// Generate final test report
    pub async fn generate_final_report(&self) -> Result<TestReport> {
        let metrics = self.metrics.read().await;
        let duration = self.start_time.elapsed();
        
        let report = TestReport {
            test_run_id: format!("e2e_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")),
            duration,
            environment: TestEnvironment {
                local_nodes: metrics.nodes_started,
                remote_nodes: 0, // Will be updated by network
                total_nodes: metrics.nodes_started,
                ipv6_only: true,
            },
            test_results: self.compile_test_results(&metrics).await,
            network_metrics: self.compile_network_metrics(&metrics).await,
            recommendations: self.generate_recommendations(&metrics).await,
        };
        
        Ok(report)
    }
    
    /// Compile test results
    async fn compile_test_results(&self, metrics: &TestMetrics) -> HashMap<String, TestSuiteResult> {
        let mut results = HashMap::new();
        
        // These will be populated by actual test execution
        results.insert("identity".to_string(), TestSuiteResult {
            passed: 0,
            failed: 0,
            duration: Duration::default(),
            details: HashMap::new(),
        });
        
        results
    }
    
    /// Compile network metrics
    async fn compile_network_metrics(&self, metrics: &TestMetrics) -> NetworkMetrics {
        NetworkMetrics {
            total_connections: metrics.connections_established,
            avg_peer_count: 0.0, // Will be calculated
            total_bandwidth_gb: 0.0, // Will be calculated
            dht_operations: metrics.dht_operations,
            tunnel_statistics: HashMap::new(), // Will be populated
        }
    }
    
    /// Generate recommendations based on test results
    async fn generate_recommendations(&self, metrics: &TestMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if metrics.errors > 0 {
            recommendations.push(format!(
                "{} errors detected - investigate error logs",
                metrics.errors
            ));
        }
        
        if metrics.failed_events > metrics.successful_events / 10 {
            recommendations.push(
                "High failure rate detected - review system stability".to_string()
            );
        }
        
        if recommendations.is_empty() {
            recommendations.push("All systems operating within normal parameters".to_string());
        }
        
        recommendations
    }
}

/// Terminal UI manager
struct Terminal {
    events: Vec<(String, Color)>,
    topology: Option<NetworkTopology>,
    tunnel_stats: Option<TunnelStats>,
    network_status: Option<NetworkStatus>,
    max_events: usize,
}

impl Terminal {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), terminal::Clear(ClearType::All), cursor::Hide)?;
        
        Ok(Self {
            events: Vec::new(),
            topology: None,
            tunnel_stats: None,
            network_status: None,
            max_events: 20,
        })
    }
    
    fn add_event(&mut self, message: String, color: Color) {
        self.events.push((message, color));
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }
    
    fn update_topology(&mut self, topology: &NetworkTopology) {
        self.topology = Some(NetworkTopology {
            nodes: topology.nodes.clone(),
            edges: topology.edges.clone(),
        });
    }
    
    fn update_tunnel_stats(&mut self, stats: &TunnelStats) {
        self.tunnel_stats = Some(stats.clone());
    }
    
    fn update_network_status(&mut self, status: &NetworkStatus) {
        self.network_status = Some(NetworkStatus {
            local_nodes: status.local_nodes,
            remote_nodes: status.remote_nodes,
            total_connections: status.total_connections,
            uptime: status.uptime,
            test_phase: status.test_phase.clone(),
        });
    }
    
    async fn refresh(&self) {
        let _ = execute!(stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0));
        
        // Header
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║ Saorsa Core E2E Test Suite - Real-Time Monitor          ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();
        
        // Network status
        if let Some(status) = &self.network_status {
            println!("📡 Network Status:");
            println!("├─ Local Nodes: {} ✓", status.local_nodes);
            println!("├─ Remote Nodes: {} ✓", status.remote_nodes);
            println!("├─ Total Connections: {}", status.total_connections);
            println!("├─ Uptime: {:?}", status.uptime);
            println!("└─ Test Phase: {}", status.test_phase);
            println!();
        }
        
        // Tunnel statistics
        if let Some(stats) = &self.tunnel_stats {
            println!("🌐 Tunnel Usage:");
            println!("┌─────────────┬──────────┬───────────┬──────────┬─────────┐");
            println!("│ Protocol    │ State    │ Bytes Sent│ Bytes Rcv│ RTT(ms) │");
            println!("├─────────────┼──────────┼───────────┼──────────┼─────────┤");
            
            for (protocol, tunnel_info) in &stats.tunnels {
                println!("│ {:11} │ {:8} │ {:9} │ {:8} │ {:7} │",
                    protocol,
                    tunnel_info.state,
                    format_bytes(tunnel_info.bytes_sent),
                    format_bytes(tunnel_info.bytes_received),
                    tunnel_info.avg_rtt_ms.map_or("-".to_string(), |r| r.to_string())
                );
            }
            
            println!("└─────────────┴──────────┴───────────┴──────────┴─────────┘");
            println!();
        }
        
        // Recent events
        println!("📝 Recent Events:");
        for (event, color) in &self.events {
            execute!(stdout(), SetForegroundColor(*color), Print(event), Print("\n"), ResetColor)?;
        }
        
        let _ = stdout().flush();
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}

/// Test event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEvent {
    pub timestamp: SystemTime,
    pub node_id: String,
    pub event_type: TestEventType,
    pub details: HashMap<String, serde_json::Value>,
    pub success: bool,
}

/// Test event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestEventType {
    NodeStarted,
    ConnectionEstablished,
    TunnelCreated,
    DHTOperation,
    ChatMessage,
    ProjectCreated,
    ThresholdSigning,
    MCPToolCall,
    Error(String),
}

impl ToString for TestEventType {
    fn to_string(&self) -> String {
        match self {
            Self::NodeStarted => "Node Started",
            Self::ConnectionEstablished => "Connection Established",
            Self::TunnelCreated => "Tunnel Created",
            Self::DHTOperation => "DHT Operation",
            Self::ChatMessage => "Chat Message",
            Self::ProjectCreated => "Project Created",
            Self::ThresholdSigning => "Threshold Signing",
            Self::MCPToolCall => "MCP Tool Call",
            Self::Error(e) => format!("Error: {}", e),
        }.to_string()
    }
}

/// Test metrics
#[derive(Default)]
struct TestMetrics {
    total_events: u64,
    successful_events: u64,
    failed_events: u64,
    nodes_started: u64,
    connections_established: u64,
    tunnels_created: u64,
    dht_operations: u64,
    chat_messages: u64,
    projects_created: u64,
    threshold_signings: u64,
    mcp_tool_calls: u64,
    errors: u64,
}

/// Tunnel statistics
#[derive(Clone)]
pub struct TunnelStats {
    pub tunnels: HashMap<String, TunnelInfo>,
}

#[derive(Clone)]
pub struct TunnelInfo {
    pub state: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub avg_rtt_ms: Option<u64>,
}

/// Test report structure
#[derive(Serialize, Deserialize)]
pub struct TestReport {
    pub test_run_id: String,
    pub duration: Duration,
    pub environment: TestEnvironment,
    pub test_results: HashMap<String, TestSuiteResult>,
    pub network_metrics: NetworkMetrics,
    pub recommendations: Vec<String>,
}

impl TestReport {
    /// Convert to Markdown format
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# Saorsa Core E2E Test Report\n\n"));
        md.push_str(&format!("**Test Run ID**: {}\n", self.test_run_id));
        md.push_str(&format!("**Duration**: {:?}\n\n", self.duration));
        
        md.push_str("## Environment\n\n");
        md.push_str(&format!("- Local Nodes: {}\n", self.environment.local_nodes));
        md.push_str(&format!("- Remote Nodes: {}\n", self.environment.remote_nodes));
        md.push_str(&format!("- Total Nodes: {}\n", self.environment.total_nodes));
        md.push_str(&format!("- IPv6 Only: {}\n\n", self.environment.ipv6_only));
        
        md.push_str("## Test Results\n\n");
        for (suite, result) in &self.test_results {
            md.push_str(&format!("### {}\n", suite));
            md.push_str(&format!("- Passed: {}\n", result.passed));
            md.push_str(&format!("- Failed: {}\n", result.failed));
            md.push_str(&format!("- Duration: {:?}\n\n", result.duration));
        }
        
        md.push_str("## Recommendations\n\n");
        for rec in &self.recommendations {
            md.push_str(&format!("- {}\n", rec));
        }
        
        md
    }
    
    /// Convert to HTML format
    pub fn to_html(&self) -> String {
        format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>Saorsa Core E2E Test Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        h1, h2, h3 {{ color: #333; }}
        .metric {{ background: #f0f0f0; padding: 10px; margin: 10px 0; }}
        .success {{ color: green; }}
        .failure {{ color: red; }}
    </style>
</head>
<body>
    <h1>Saorsa Core E2E Test Report</h1>
    <div class="metric">
        <strong>Test Run ID:</strong> {}<br>
        <strong>Duration:</strong> {:?}
    </div>
    <h2>Test Results Summary</h2>
    {}
</body>
</html>"#, self.test_run_id, self.duration, self.generate_html_results())
    }
    
    fn generate_html_results(&self) -> String {
        let mut html = String::new();
        
        for (suite, result) in &self.test_results {
            let status_class = if result.failed == 0 { "success" } else { "failure" };
            html.push_str(&format!(
                r#"<div class="metric">
                    <h3>{}</h3>
                    <span class="{}">Passed: {} / Failed: {}</span><br>
                    Duration: {:?}
                </div>"#,
                suite, status_class, result.passed, result.failed, result.duration
            ));
        }
        
        html
    }
}

#[derive(Serialize, Deserialize)]
pub struct TestEnvironment {
    pub local_nodes: u64,
    pub remote_nodes: u64,
    pub total_nodes: u64,
    pub ipv6_only: bool,
}

#[derive(Serialize, Deserialize)]
pub struct TestSuiteResult {
    pub passed: u32,
    pub failed: u32,
    pub duration: Duration,
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub total_connections: u64,
    pub avg_peer_count: f64,
    pub total_bandwidth_gb: f64,
    pub dht_operations: u64,
    pub tunnel_statistics: HashMap<String, TunnelStatistic>,
}

#[derive(Serialize, Deserialize)]
pub struct TunnelStatistic {
    pub count: u32,
    pub success_rate: f64,
}

/// Format bytes for display
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}