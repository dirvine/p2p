#!/usr/bin/env rust
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
use p2p_foundation::bootstrap::{BootstrapDiscovery, BootstrapManager};
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::{env, fs};
use tracing::{error, info, warn};

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
    
    /// Set up Flutter project from embedded data or download
    fn setup_flutter_project() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .context("Could not determine home directory")?;
        
        let app_dir = home_dir.join(".p2p-foundation").join("ant-connect");
        
        if !app_dir.exists() {
            info!("Setting up Ant Connect Flutter app in: {:?}", app_dir);
            fs::create_dir_all(&app_dir)?;
            
            // For now, provide instructions to user
            // In a full implementation, we'd embed the Flutter project or download it
            warn!("🏗️  Ant Connect Flutter app not found!");
            warn!("📱 To use the Flutter app:");
            warn!("   1. Clone: git clone https://github.com/dirvine/p2p.git");
            warn!("   2. Navigate: cd p2p/apps/ant-connect");
            warn!("   3. Run: flutter run");
            warn!("");
            warn!("🚀 Or use the P2P chat example instead:");
            warn!("   cargo run --example chat -- --bootstrap-words global.fast.eagle");
            
            anyhow::bail!("Flutter project setup required - see instructions above");
        }
        
        Ok(app_dir)
    }
    
    /// Launch the Flutter application
    async fn launch(&self, target_platform: &str) -> Result<()> {
        info!("🚀 Launching Ant Connect on platform: {}", target_platform);
        
        // Test bootstrap connectivity first
        self.test_bootstrap_connectivity().await?;
        
        // Check Flutter installation
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
            for addr in self.bootstrap_discovery.get_well_known_three_words() {
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
        
        let well_known = self.bootstrap_discovery.get_well_known_three_words();
        for addr in well_known {
            if let Ok(resolved) = self.bootstrap_discovery.resolve_three_words(&addr) {
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
  ant-connect                          # Launch on default platform (desktop)
  ant-connect --platform web           # Launch in web browser  
  ant-connect --platform android       # Launch on Android device
  ant-connect --bootstrap-nodes        # List available bootstrap nodes
  ant-connect --test-connectivity      # Test bootstrap connectivity
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
        );
    
    let matches = app.get_matches();
    
    let debug_mode = matches.get_flag("debug");
    let launcher = AntConnectLauncher::new(debug_mode)?;
    
    // Handle special commands
    if matches.get_flag("bootstrap-nodes") {
        return launcher.list_bootstrap_nodes().await;
    }
    
    if matches.get_flag("test-connectivity") {
        launcher.test_bootstrap_connectivity().await?;
        info!("✅ Bootstrap connectivity test completed");
        return Ok(());
    }
    
    // Launch the Flutter app
    let platform = matches.get_one::<String>("platform").unwrap();
    launcher.launch(platform).await
}

// dirs crate is included in Cargo.toml dependencies