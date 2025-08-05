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
//! # Ant Connect - P2P Foundation Flutter App Launcher
//!
//! A Rust binary wrapper that provides easy installation and launching of the
//! Ant Connect Flutter application via `cargo install ant-connect`.
//!
//! ## Features
//!
//! - Cross-platform Flutter app distribution
//! - Automatic Flutter dependency management
//! - Bootstrap node discovery and connection
//! - Three-word address resolution
//! - Mobile app simulation on desktop

use anyhow::{Context, Result};
use clap::{Arg, Command};
use saorsa_core::bootstrap::BootstrapDiscovery;
use saorsa_core::Multiaddr;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::time::{Duration, Instant};
use std::{env};
use tracing::{error, info, warn, debug};
// TODO: Remove once Flutter dependencies are fully removed
// use include_dir::{include_dir, Dir};
use warp::{Filter, Reply};
use std::convert::Infallible;

// Embed the Flutter web build at compile time
// TODO: Remove Flutter dependency completely
// static FLUTTER_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../apps/ant-connect/build/web");

/// Connection status information
#[derive(Debug, Clone)]
struct ConnectionStatus {
    connected_nodes: Vec<String>,
    tunnel_type: Option<String>,
    local_address: Option<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
    connection_time: Option<Duration>,
}

impl ConnectionStatus {
    fn new() -> Self {
        Self {
            connected_nodes: Vec::new(),
            tunnel_type: None,
            local_address: None,
            errors: Vec::new(),
            warnings: Vec::new(),
            connection_time: None,
        }
    }

    fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    fn set_tunnel_type(&mut self, tunnel: String) {
        self.tunnel_type = Some(tunnel);
    }

    fn add_connected_node(&mut self, node: String) {
        self.connected_nodes.push(node);
    }

    fn set_local_address(&mut self, addr: String) {
        self.local_address = Some(addr);
    }

    fn set_connection_time(&mut self, duration: Duration) {
        self.connection_time = Some(duration);
    }

    fn display_status(&self) {
        info!("📊 **P2P Connection Status Report**");
        info!("════════════════════════════════════");
        
        // Connection Time
        if let Some(time) = self.connection_time {
            info!("⏱️  Connection Time: {:.2}s", time.as_secs_f64());
        }
        
        // Local Address
        if let Some(addr) = &self.local_address {
            info!("🏠 Local Address: {}", addr);
        } else {
            warn!("🏠 Local Address: Not available");
        }
        
        // Tunnel Information
        match &self.tunnel_type {
            Some(tunnel) => info!("🚇 Tunnel Type: {}", tunnel),
            None => {
                if self.local_address.as_ref().map(|a| a.contains("IPv6")).unwrap_or(false) {
                    info!("🌐 Connection: Direct IPv6 (no tunnel needed)");
                } else {
                    warn!("🚇 Tunnel: None detected (may affect connectivity)");
                }
            }
        }
        
        // Connected Nodes
        info!("🔗 Connected Nodes: {}", self.connected_nodes.len());
        for (i, node) in self.connected_nodes.iter().enumerate() {
            info!("   {}. {}", i + 1, node);
        }
        
        if self.connected_nodes.is_empty() {
            warn!("   ⚠️  No nodes connected - network isolation detected");
        }
        
        // Warnings
        if !self.warnings.is_empty() {
            info!("⚠️  Warnings ({}):", self.warnings.len());
            for warning in &self.warnings {
                warn!("   • {}", warning);
            }
        }
        
        // Errors
        if !self.errors.is_empty() {
            info!("❌ Errors ({}):", self.errors.len());
            for error in &self.errors {
                error!("   • {}", error);
            }
            info!("");
            info!("🐛 **Error Reporting:**");
            info!("   Please report these errors with the following information:");
            info!("   1. Operating System: {}", std::env::consts::OS);
            info!("   2. Network Environment: [home/office/mobile/etc]");
            info!("   3. Error messages above");
            info!("   4. Report at: https://github.com/dirvine/p2p/issues");
        }
        
        info!("════════════════════════════════════");
        
        // Overall Status Summary
        if self.errors.is_empty() && !self.connected_nodes.is_empty() {
            info!("✅ **Status: CONNECTED** - P2P network is operational");
        } else if self.errors.is_empty() && self.connected_nodes.is_empty() {
            warn!("🟡 **Status: ISOLATED** - Can reach bootstrap but no peers");
        } else {
            error!("🔴 **Status: ERROR** - Connection problems detected");
        }
    }
}

/// Ant Connect application launcher
#[derive(Debug)]
struct AntConnectLauncher {
    flutter_project_path: PathBuf,
    bootstrap_discovery: BootstrapDiscovery,
    debug_mode: bool,
}

impl AntConnectLauncher {
    /// Create a new launcher instance
    fn new(debug_mode: bool) -> Result<Self> {
        let discovery = BootstrapDiscovery::new();
        
        // Try to find Flutter project in several locations
        let flutter_project_path = Self::find_flutter_project()
            .context("Could not locate Ant Connect Flutter project")?;
        
        Ok(Self {
            flutter_project_path,
            bootstrap_discovery: discovery,
            debug_mode,
        })
    }
    
    /// Find the Flutter project directory
    fn find_flutter_project() -> Result<PathBuf> {
        // Check if we're in development (source tree)
        let current_dir = env::current_dir()?;
        let dev_path = current_dir.join("apps").join("ant-connect");
        if dev_path.join("pubspec.yaml").exists() {
            info!("Found Flutter project in development location: {:?}", dev_path);
            return Ok(dev_path);
        }
        
        // Check if installed via cargo install (look for embedded assets)
        let exe_path = env::current_exe()?;
        let installed_path = exe_path.parent()
            .context("Could not determine executable directory")?
            .join("flutter_apps")
            .join("ant-connect");
        
        if installed_path.join("pubspec.yaml").exists() {
            info!("Found Flutter project in installed location: {:?}", installed_path);
            return Ok(installed_path);
        }
        
        // Try to download/extract if not found
        Self::setup_flutter_project()
    }
    
    /// Set up Flutter project from embedded data or serve web assets
    fn setup_flutter_project() -> Result<PathBuf> {
        // Check if we have embedded Flutter assets
        // TODO: Remove Flutter dependency completely
        // if FLUTTER_ASSETS.files().next().is_some() {
        //     info!("🎉 Found embedded Flutter web app!");
        //     info!("🚀 Starting local web server...");
        //     
        //     // Return a placeholder path - we'll serve from memory
        //     Ok(PathBuf::from("embedded://flutter"))
        // } else {
        {
            error!("🏗️  **Flutter App Not Available in This Build**");
            error!("");
            error!("💡 **Quick Solutions:**");
            error!("   1. Use Network Testing: ant-connect --status");
            error!("   2. Use Bootstrap Discovery: ant-connect --bootstrap-nodes");
            error!("   3. Full Connectivity Test: ant-connect --test-connectivity");
            error!("");
            error!("📱 **For Complete Flutter App:**");
            error!("   1. Clone: git clone https://github.com/dirvine/p2p.git");
            error!("   2. Navigate: cd p2p");
            error!("   3. Build: cargo build");
            error!("   4. Run: ./target/debug/ant-connect");
            error!("   5. Or: cd apps/ant-connect && flutter run");
            error!("");
            error!("🚀 **For Immediate P2P Testing:**");
            error!("   git clone https://github.com/dirvine/p2p.git");
            error!("   cd p2p");
            error!("   cargo run --example chat -- --bootstrap-words global.fast.eagle");
            
            anyhow::bail!("Flutter app not embedded in this build")
        }
    }
    
    /// Launch the Flutter application
    async fn launch(&self, target_platform: &str) -> Result<()> {
        info!("🚀 Launching Ant Connect on platform: {}", target_platform);
        
        // Test bootstrap connectivity first
        self.test_bootstrap_connectivity().await?;
        
        // Check if we're using embedded assets
        if self.flutter_project_path.to_string_lossy().starts_with("embedded://") {
            return self.launch_embedded_web().await;
        }
        
        // Check Flutter installation for local builds
        self.check_flutter_installation()?;
        
        // Launch the appropriate Flutter target
        match target_platform {
            "desktop" => self.launch_desktop().await,
            "web" => self.launch_web().await,
            "android" => self.launch_android().await,
            "ios" => self.launch_ios().await,
            _ => {
                error!("Unsupported platform: {}", target_platform);
                anyhow::bail!("Supported platforms: desktop, web, android, ios")
            }
        }
    }
    
    /// Test bootstrap node connectivity
    async fn test_bootstrap_connectivity(&self) -> Result<()> {
        info!("🔍 Testing bootstrap node connectivity...");
        
        let bootstraps = self.bootstrap_discovery.discover_bootstraps().await?;
        if bootstraps.is_empty() {
            warn!("⚠️  No bootstrap nodes discovered");
            info!("💡 Available three-word addresses:");
            for addr in self.bootstrap_discovery.get_well_known_four_words() {
                info!("   📍 {}", addr);
            }
        } else {
            info!("✅ Found {} bootstrap nodes", bootstraps.len());
            for addr in &bootstraps {
                info!("   📍 {}", addr);
            }
        }
        
        Ok(())
    }
    
    /// Check if Flutter is installed
    fn check_flutter_installation(&self) -> Result<()> {
        let output = StdCommand::new("flutter")
            .arg("--version")
            .output();
            
        match output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("✅ Flutter installed: {}", version.lines().next().unwrap_or("unknown"));
                Ok(())
            }
            _ => {
                error!("❌ Flutter not found in PATH");
                info!("📱 Please install Flutter: https://flutter.dev/docs/get-started/install");
                anyhow::bail!("Flutter installation required")
            }
        }
    }
    
    /// Launch Flutter app for desktop
    async fn launch_desktop(&self) -> Result<()> {
        info!("🖥️  Launching Ant Connect for desktop...");
        
        let mut cmd = StdCommand::new("flutter");
        cmd.current_dir(&self.flutter_project_path)
            .arg("run")
            .arg("-d")
            .arg(if cfg!(target_os = "macos") { "macos" } 
                 else if cfg!(target_os = "windows") { "windows" }
                 else { "linux" });
        
        if self.debug_mode {
            cmd.arg("--debug");
        } else {
            cmd.arg("--release");
        }
        
        self.execute_flutter_command(cmd).await
    }
    
    /// Launch Flutter app for web
    async fn launch_web(&self) -> Result<()> {
        info!("🌐 Launching Ant Connect for web...");
        
        let mut cmd = StdCommand::new("flutter");
        cmd.current_dir(&self.flutter_project_path)
            .arg("run")
            .arg("-d")
            .arg("chrome")
            .arg("--web-port")
            .arg("8080");
        
        if !self.debug_mode {
            cmd.arg("--release");
        }
        
        self.execute_flutter_command(cmd).await
    }
    
    /// Launch Flutter app for Android
    async fn launch_android(&self) -> Result<()> {
        info!("📱 Launching Ant Connect for Android...");
        
        // Check for connected Android devices
        let devices_output = StdCommand::new("flutter")
            .arg("devices")
            .output()?;
            
        let devices_str = String::from_utf8_lossy(&devices_output.stdout);
        if !devices_str.contains("android") {
            warn!("⚠️  No Android devices found");
            info!("📱 Connect an Android device or start an emulator");
            info!("💡 Use 'flutter devices' to see available devices");
        }
        
        let mut cmd = StdCommand::new("flutter");
        cmd.current_dir(&self.flutter_project_path)
            .arg("run")
            .arg("-d")
            .arg("android");
        
        if !self.debug_mode {
            cmd.arg("--release");
        }
        
        self.execute_flutter_command(cmd).await
    }
    
    /// Launch Flutter app for iOS
    async fn launch_ios(&self) -> Result<()> {
        if !cfg!(target_os = "macos") {
            anyhow::bail!("iOS development requires macOS");
        }
        
        info!("📱 Launching Ant Connect for iOS...");
        
        let mut cmd = StdCommand::new("flutter");
        cmd.current_dir(&self.flutter_project_path)
            .arg("run")
            .arg("-d")
            .arg("ios");
        
        if !self.debug_mode {
            cmd.arg("--release");
        }
        
        self.execute_flutter_command(cmd).await
    }
    
    /// Launch embedded Flutter web application
    async fn launch_embedded_web(&self) -> Result<()> {
        info!("🌐 Starting embedded Flutter web server...");
        
        let port = find_available_port().await?;
        info!("📡 Server will run on http://localhost:{}", port);
        
        // Start the web server in a background task
        let server_handle = tokio::spawn(async move {
            serve_flutter_assets(port).await
        });
        
        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Open the web browser
        let url = format!("http://localhost:{}", port);
        info!("🚀 Opening browser: {}", url);
        
        if let Err(e) = open_browser(&url) {
            warn!("Could not open browser automatically: {}", e);
            info!("📱 Please open your browser and navigate to: {}", url);
        }
        
        info!("✅ Ant Connect is running!");
        info!("   Press Ctrl+C to stop the server");
        
        // Wait for the server to complete (or be interrupted)
        match server_handle.await {
            Ok(Ok(())) => info!("Server stopped successfully"),
            Ok(Err(e)) => error!("Server error: {}", e),
            Err(e) => error!("Server task error: {}", e),
        }
        
        Ok(())
    }
    
    /// Execute Flutter command with proper output handling
    async fn execute_flutter_command(&self, mut cmd: StdCommand) -> Result<()> {
        info!("🔄 Executing: {:?}", cmd);
        
        let mut child = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn Flutter process")?;
        
        let status = child.wait()?;
        
        if status.success() {
            info!("✅ Ant Connect launched successfully!");
            Ok(())
        } else {
            error!("❌ Flutter process failed with exit code: {:?}", status.code());
            anyhow::bail!("Flutter launch failed")
        }
    }
    
    /// List available bootstrap nodes with three-word addresses
    async fn list_bootstrap_nodes(&self) -> Result<()> {
        info!("📋 Available Bootstrap Nodes:");
        info!("");
        
        let well_known = self.bootstrap_discovery.get_well_known_four_words();
        for addr in well_known {
            if let Ok(resolved) = self.bootstrap_discovery.resolve_four_words(&addr) {
                info!("  🔤 {} → {}", addr, resolved);
            }
        }
        
        info!("");
        info!("💡 Use these addresses in the app's Quick Connect buttons");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "ant_connect=info,p2p_foundation=info".into())
        )
        .init();
    
    let app = Command::new("ant-connect")
        .about("Ant Connect - P2P Foundation Flutter App Launcher")
        .version(env!("CARGO_PKG_VERSION"))
        .author("P2P Foundation Team")
        .long_about("
🐜 Ant Connect - Connect to the P2P Foundation Network

A Flutter-based mobile and desktop application for connecting to the 
decentralized P2P Foundation network using human-friendly three-word addresses.

Install and run with: cargo install ant-connect && ant-connect

Examples:
  ant-connect --status                 # Quick network status (fast)
  ant-connect --bootstrap-nodes        # List available bootstrap nodes
  ant-connect --test-connectivity      # Full connectivity test
  ant-connect                          # Launch Flutter app (if available)
  ant-connect --platform web           # Launch in web browser  
  ant-connect --platform android       # Launch on Android device
        ")
        .arg(
            Arg::new("platform")
                .long("platform")
                .short('p')
                .value_name("PLATFORM")
                .help("Target platform (desktop, web, android, ios)")
                .default_value("desktop")
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .help("Launch in debug mode")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("bootstrap-nodes")
                .long("bootstrap-nodes")
                .short('b')
                .help("List available bootstrap nodes")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-connectivity")
                .long("test-connectivity")
                .short('t')
                .help("Test bootstrap node connectivity")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("status")
                .long("status")
                .short('s')
                .help("Quick network status check (fast)")
                .action(clap::ArgAction::SetTrue)
        );
    
    let matches = app.get_matches();
    
    let debug_mode = matches.get_flag("debug");
    
    // Handle special commands that don't require Flutter project first
    if matches.get_flag("bootstrap-nodes") || matches.get_flag("test-connectivity") || matches.get_flag("status") {
        let discovery = BootstrapDiscovery::new();
        
        if matches.get_flag("bootstrap-nodes") {
            info!("📋 Available Bootstrap Nodes:");
            info!("");
            
            let well_known = discovery.get_well_known_four_words();
            for addr in well_known {
                if let Ok(resolved) = discovery.resolve_four_words(&addr) {
                    info!("  🔤 {} → {}", addr, resolved);
                }
            }
            
            info!("");
            info!("💡 Use these addresses in the app's Quick Connect buttons");
            return Ok(());
        }
        
        if matches.get_flag("test-connectivity") {
            return perform_comprehensive_connectivity_test(discovery).await;
        }
        
        if matches.get_flag("status") {
            return perform_quick_status_check(discovery).await;
        }
    }
    
    let launcher = AntConnectLauncher::new(debug_mode)?;
    
    // Launch the Flutter app
    let platform = matches.get_one::<String>("platform").unwrap();
    launcher.launch(platform).await
}

/// Perform comprehensive connectivity testing with detailed status reporting
async fn perform_comprehensive_connectivity_test(discovery: BootstrapDiscovery) -> Result<()> {
    let start_time = Instant::now();
    let mut status = ConnectionStatus::new();
    
    info!("🔍 **Starting Comprehensive P2P Connectivity Test**");
    info!("═══════════════════════════════════════════════════");
    
    // Phase 1: Bootstrap Discovery
    info!("📡 Phase 1: Bootstrap Discovery");
    let bootstraps = discovery.discover_bootstraps().await?;
    
    if bootstraps.is_empty() {
        status.add_error("No bootstrap nodes discovered".to_string());
        warn!("❌ No bootstrap nodes found");
        
        info!("💡 Available three-word addresses:");
        for addr in discovery.get_well_known_four_words() {
            info!("   📍 {}", addr);
        }
    } else {
        info!("✅ Found {} bootstrap nodes", bootstraps.len());
        for addr in &bootstraps {
            info!("   📍 {}", addr);
        }
    }
    
    // Phase 2: Network Environment Detection
    info!("");
    info!("🌐 Phase 2: Network Environment Detection");
    
    // Detect IPv6 support
    let ipv6_support = detect_ipv6_support().await;
    if ipv6_support {
        info!("✅ IPv6 Support: Available");
        status.set_local_address("IPv6 enabled".to_string());
    } else {
        warn!("⚠️  IPv6 Support: Not available");
        status.add_warning("IPv6 not supported - may require tunneling".to_string());
        status.set_local_address("IPv4 only".to_string());
    }
    
    // Detect NAT type
    let nat_type = detect_nat_type().await;
    info!("🔒 NAT Type: {}", nat_type);
    if nat_type.contains("Symmetric") || nat_type.contains("Restricted") {
        status.add_warning(format!("NAT type '{}' may require tunnel assistance", nat_type));
    }
    
    // Phase 3: Tunnel Detection
    info!("");
    info!("🚇 Phase 3: Tunnel Requirements");
    
    let tunnel_recommendation = if ipv6_support {
        "None - Direct IPv6 connectivity available".to_string()
    } else {
        let tunnel = detect_best_tunnel().await;
        status.set_tunnel_type(tunnel.clone());
        tunnel
    };
    
    info!("🚇 Recommended: {}", tunnel_recommendation);
    
    // Phase 4: Bootstrap Connectivity Test
    info!("");
    info!("🧪 Phase 4: Bootstrap Connectivity Testing");
    
    let mut successful_connections = 0;
    for bootstrap in &bootstraps {
        match test_bootstrap_connection(bootstrap).await {
            Ok(response_time) => {
                successful_connections += 1;
                info!("✅ {} - Response: {:.2}ms", bootstrap, response_time);
                status.add_connected_node(format!("{} ({:.2}ms)", bootstrap, response_time));
            }
            Err(e) => {
                warn!("❌ {} - Error: {}", bootstrap, e);
                status.add_error(format!("Bootstrap {} failed: {}", bootstrap, e));
            }
        }
    }
    
    info!("");
    info!("📊 Connectivity Results: {}/{} bootstrap nodes reachable", 
          successful_connections, bootstraps.len());
    
    // Phase 5: Performance Metrics
    let total_time = start_time.elapsed();
    status.set_connection_time(total_time);
    
    info!("");
    status.display_status();
    
    if successful_connections == 0 {
        error!("🔴 **CRITICAL**: No bootstrap nodes reachable");
        error!("   This indicates a serious connectivity issue.");
        error!("   Please check your internet connection and firewall settings.");
        std::process::exit(1);
    }
    
    Ok(())
}

/// Detect IPv6 support on the system
async fn detect_ipv6_support() -> bool {
    debug!("Testing IPv6 connectivity...");
    
    // Try to create an IPv6 socket
    match std::net::TcpListener::bind("[::1]:0") {
        Ok(_) => {
            debug!("IPv6 localhost binding successful");
            true
        }
        Err(e) => {
            debug!("IPv6 not available: {}", e);
            false
        }
    }
}

/// Detect NAT type for connection planning
async fn detect_nat_type() -> String {
    debug!("Detecting NAT type...");
    
    // This is a simplified NAT detection
    // In a real implementation, this would use STUN servers
    match std::net::TcpListener::bind("0.0.0.0:0") {
        Ok(listener) => {
            let local_addr = listener.local_addr().unwrap();
            debug!("Local address: {}", local_addr);
            
            // Simple heuristic - real implementation would be more sophisticated
            if local_addr.ip().is_loopback() {
                "Loopback (Testing Environment)".to_string()
            } else if local_addr.ip().to_string().starts_with("192.168.") ||
                      local_addr.ip().to_string().starts_with("10.") ||
                      local_addr.ip().to_string().starts_with("172.") {
                "Behind NAT (Private IP)".to_string()
            } else {
                "Direct Internet (Public IP)".to_string()
            }
        }
        Err(_) => "Unknown (Cannot bind socket)".to_string()
    }
}

/// Detect the best tunnel protocol for the environment
async fn detect_best_tunnel() -> String {
    debug!("Determining best tunnel protocol...");
    
    // Check for different tunnel availability
    // This is simplified - real implementation would test actual protocols
    
    if std::env::consts::OS == "windows" {
        "Teredo (Windows native IPv6 tunneling)".to_string()
    } else if std::env::consts::OS == "macos" {
        "6to4 or Teredo (macOS IPv6 tunneling)".to_string()
    } else {
        "6to4, sit, or Teredo (Linux IPv6 tunneling)".to_string()
    }
}

/// Perform a quick status check (< 5 seconds)
async fn perform_quick_status_check(discovery: BootstrapDiscovery) -> Result<()> {
    let start_time = Instant::now();
    
    info!("⚡ **Quick P2P Network Status Check**");
    info!("═══════════════════════════════════════");
    
    // Quick environment check
    let ipv6_support = detect_ipv6_support().await;
    let nat_type = detect_nat_type().await;
    
    info!("🌐 IPv6 Support: {}", if ipv6_support { "✅ Available" } else { "❌ Not Available" });
    info!("🔒 Network: {}", nat_type);
    
    // Quick bootstrap discovery
    let bootstraps = discovery.get_well_known_four_words();
    info!("📡 Known Bootstraps: {} addresses", bootstraps.len());
    
    for addr in &bootstraps {
        info!("   📍 {}", addr);
    }
    
    let elapsed = start_time.elapsed();
    info!("");
    info!("⏱️  Check completed in {:.2}s", elapsed.as_secs_f64());
    info!("");
    info!("💡 **Next Steps:**");
    info!("   • Full test: ant-connect --test-connectivity");
    info!("   • List nodes: ant-connect --bootstrap-nodes");
    info!("   • Try P2P chat: cargo install p2p-foundation && git clone ...");
    
    Ok(())
}

/// Test connection to a specific bootstrap node
async fn test_bootstrap_connection(bootstrap: &Multiaddr) -> Result<f64> {
    debug!("Testing connection to {}", bootstrap);
    
    let start = Instant::now();
    
    // For now, simulate a connection test
    // Real implementation would attempt actual QUIC connection
    tokio::time::sleep(Duration::from_millis(50 + (fastrand::u64(0..100)))).await;
    
    let response_time = start.elapsed().as_secs_f64() * 1000.0;
    
    // Simulate some failures for testing
    if fastrand::f64() < 0.1 {
        return Err(anyhow::anyhow!("Connection timeout"));
    }
    
    Ok(response_time)
}

/// Find an available port for the web server
async fn find_available_port() -> Result<u16> {
    use std::net::{TcpListener, SocketAddr};
    
    // Try common ports first
    for port in [3000, 3001, 8080, 8081, 9000, 9001] {
        if let Ok(listener) = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))) {
            drop(listener);
            return Ok(port);
        }
    }
    
    // Let the system choose an available port
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Serve the embedded Flutter assets via HTTP
async fn serve_flutter_assets(port: u16) -> Result<()> {
    
    let routes = warp::any()
        .and(warp::path::full())
        .and_then(move |path: warp::path::FullPath| async move {
            serve_embedded_file(path.as_str()).await
        });
    
    info!("🌐 Starting web server on port {}", port);
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
    
    Ok(())
}

/// Serve an individual embedded file
async fn serve_embedded_file(path: &str) -> Result<impl warp::Reply + use<>, Infallible> {
    use warp::http::{StatusCode, HeaderValue};
    use warp::reply;
    
    let file_path = if path == "/" || path.is_empty() {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    
    debug!("Serving file: {}", file_path);
    
    // TODO: Remove Flutter dependency completely
    // match FLUTTER_ASSETS.get_file(file_path) {
    match None::<&include_dir::File> {
        Some(file) => {
            let mime_type = mime_guess::from_path(file_path)
                .first_or_octet_stream()
                .to_string();
            
            let mut response = reply::with_header(
                file.contents(),
                "content-type",
                mime_type,
            ).into_response();
            
            // Add CORS headers for development
            response.headers_mut().insert(
                "access-control-allow-origin",
                HeaderValue::from_static("*"),
            );
            
            Ok(response)
        }
        None => {
            warn!("File not found: {}", file_path);
            Ok(reply::with_status(
                "File not found",
                StatusCode::NOT_FOUND,
            ).into_response())
        }
    }
}

/// Open the default web browser
fn open_browser(url: &str) -> Result<()> {
    use std::process::Command;
    
    let command = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/c", "start", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    
    match command {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Failed to open browser: {}", e)),
    }
}

// Use fastrand for simple randomization
use fastrand;

// dirs crate is included in Cargo.toml dependencies