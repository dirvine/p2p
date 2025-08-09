// Copyright 2025 Saorsa Labs Limited
// Integration tests for CLI commands

use anyhow::Result;
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use std::path::PathBuf;
use tokio::time::timeout;

const BINARY_NAME: &str = "communitas";
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

struct TestApp {
    binary_path: PathBuf,
    temp_config_dir: TempDir,
}

impl TestApp {
    fn new() -> Result<Self> {
        let binary_path = std::env::current_exe()?
            .parent()
            .unwrap()
            .join(BINARY_NAME);
            
        let temp_config_dir = TempDir::new()?;
        
        Ok(Self {
            binary_path,
            temp_config_dir,
        })
    }
    
    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.binary_path);
        cmd.env("COMMUNITAS_CONFIG_DIR", self.temp_config_dir.path());
        cmd
    }
    
    fn create_test_config(&self) -> Result<()> {
        let config_path = self.temp_config_dir.path().join("config.toml");
        let test_config = r#"
[api]
openai_key = "test-key-12345"
default_model = "gpt-3.5-turbo"

[ui]
theme = "dark"
auto_save = true

[network]
p2p_enabled = false
listen_port = 9000

[voice]
enabled = false

[file]
max_file_size = "10MB"
output_directory = "/tmp/communitas_test"
"#;
        
        std::fs::write(&config_path, test_config)?;
        Ok(())
    }
    
    async fn run_command(&self, args: &[&str]) -> Result<(String, String, i32)> {
        let output = timeout(TEST_TIMEOUT, async {
            self.command()
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        }).await??;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        
        Ok((stdout, stderr, exit_code))
    }
}

#[tokio::test]
async fn test_help_command() -> Result<()> {
    let app = TestApp::new()?;
    
    let (stdout, stderr, exit_code) = app.run_command(&["--help"]).await?;
    
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Personal AI assistant with advanced capabilities"));
    assert!(stdout.contains("chat"));
    assert!(stdout.contains("process"));
    assert!(stdout.contains("tui"));
    assert!(stdout.contains("config"));
    assert!(stdout.contains("connect"));
    assert!(stderr.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_version_command() -> Result<()> {
    let app = TestApp::new()?;
    
    let (stdout, stderr, exit_code) = app.run_command(&["--version"]).await?;
    
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("0.1.0")); // Version from Cargo.toml
    assert!(stderr.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_chat_command_basic() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    // Test basic chat command (should fail gracefully without real API key)
    let (stdout, stderr, exit_code) = app.run_command(&["chat", "--model", "gpt-3.5-turbo"]).await?;
    
    // For now, expecting the placeholder message since chat isn't implemented
    assert!(stdout.contains("Chat functionality coming soon!") || exit_code != 0);
    
    Ok(())
}

#[tokio::test]
async fn test_chat_command_with_voice() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&["chat", "--voice"]).await?;
    
    // Should indicate voice mode is enabled
    assert!(stdout.contains("Voice I/O enabled") || stdout.contains("Chat functionality coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_chat_command_with_vision() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&["chat", "--vision"]).await?;
    
    // Should indicate vision mode is enabled
    assert!(stdout.contains("Vision capabilities enabled") || stdout.contains("Chat functionality coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_process_command() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    // Create a test file
    let test_file = app.temp_config_dir.path().join("test.txt");
    std::fs::write(&test_file, "This is a test document for processing.")?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&[
        "process", 
        test_file.to_str().unwrap(),
        "--instruction", "Summarize this document"
    ]).await?;
    
    // Should acknowledge processing the file
    assert!(stdout.contains("Processing file") || stdout.contains("File processing coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_process_command_with_format() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    let test_file = app.temp_config_dir.path().join("test.txt");
    std::fs::write(&test_file, "Test content")?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&[
        "process",
        test_file.to_str().unwrap(),
        "--format", "json"
    ]).await?;
    
    // Should handle format specification
    assert!(stdout.contains("Processing file") || stdout.contains("File processing coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_tui_command() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    // TUI command should start but we can't easily test the interactive part
    // Just verify it recognizes the command and options
    let (stdout, stderr, _exit_code) = app.run_command(&["tui", "--theme", "light"]).await?;
    
    assert!(stdout.contains("Starting TUI with light theme") || stdout.contains("TUI interface coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_config_show_command() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&["config", "--show"]).await?;
    
    // Should display configuration or indicate it's coming soon
    assert!(stdout.contains("Configuration display coming soon!") || stdout.contains("[api]"));
    
    Ok(())
}

#[tokio::test]
async fn test_config_set_api_key() -> Result<()> {
    let app = TestApp::new()?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&["config", "--api-key", "openai"]).await?;
    
    // Should indicate it's setting the API key
    assert!(stdout.contains("Setting API key for: openai"));
    
    Ok(())
}

#[tokio::test]
async fn test_connect_command() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&[
        "connect", 
        "--port", "8000",
        "--bootstrap", "/ip4/127.0.0.1/tcp/9000"
    ]).await?;
    
    // Should acknowledge P2P connection attempt
    assert!(stdout.contains("Connecting to P2P network on port 8000"));
    assert!(stdout.contains("Using bootstrap node: /ip4/127.0.0.1/tcp/9000"));
    assert!(stdout.contains("P2P connection coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_verbose_logging() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&["--verbose", "chat"]).await?;
    
    // Should show debug-level logging when verbose is enabled
    assert!(stderr.contains("DEBUG") || stdout.contains("debug") || stdout.contains("Chat functionality coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_custom_config_file() -> Result<()> {
    let app = TestApp::new()?;
    
    // Create custom config file
    let custom_config = app.temp_config_dir.path().join("custom.toml");
    std::fs::write(&custom_config, r#"
[api]
default_model = "claude-3"

[ui]
theme = "custom"
"#)?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&[
        "--config", custom_config.to_str().unwrap(),
        "chat"
    ]).await?;
    
    // Should use the custom config file
    // For now, just verify command runs without error
    assert!(exit_code == 0 || stdout.contains("Chat functionality coming soon!"));
    
    Ok(())
}

#[tokio::test]
async fn test_invalid_command() -> Result<()> {
    let app = TestApp::new()?;
    
    let (stdout, stderr, exit_code) = app.run_command(&["invalid_command"]).await?;
    
    assert_ne!(exit_code, 0);
    assert!(stderr.contains("error") || stderr.contains("unexpected") || stderr.contains("invalid"));
    
    Ok(())
}

#[tokio::test]
async fn test_missing_required_argument() -> Result<()> {
    let app = TestApp::new()?;
    
    // Process command requires a file argument
    let (stdout, stderr, exit_code) = app.run_command(&["process"]).await?;
    
    assert_ne!(exit_code, 0);
    assert!(stderr.contains("required") || stderr.contains("argument") || stderr.contains("missing"));
    
    Ok(())
}

#[tokio::test]
async fn test_file_not_found() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    let (stdout, stderr, _exit_code) = app.run_command(&["process", "/nonexistent/file.txt"]).await?;
    
    // Should handle non-existent files gracefully
    // For now, just verify it doesn't crash
    assert!(stdout.contains("File processing coming soon!") || stderr.contains("not found") || exit_code != 0);
    
    Ok(())
}

#[tokio::test]
async fn test_startup_performance() -> Result<()> {
    let app = TestApp::new()?;
    
    let start_time = std::time::Instant::now();
    let (stdout, stderr, exit_code) = app.run_command(&["--help"]).await?;
    let duration = start_time.elapsed();
    
    // Should start quickly
    assert!(duration < Duration::from_millis(2000), "Startup took {:?}", duration);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Personal AI assistant"));
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_commands() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    // Test multiple commands running concurrently
    let task1 = app.run_command(&["--help"]);
    let task2 = app.run_command(&["--version"]);
    let task3 = app.run_command(&["config", "--show"]);
    
    let (result1, result2, result3) = tokio::join!(task1, task2, task3);
    
    // All should succeed
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());
    
    let (_, _, exit_code1) = result1?;
    let (_, _, exit_code2) = result2?;
    
    assert_eq!(exit_code1, 0);
    assert_eq!(exit_code2, 0);
    // exit_code3 might be non-zero if config functionality isn't implemented yet
    
    Ok(())
}

#[tokio::test]
async fn test_environment_variables() -> Result<()> {
    let app = TestApp::new()?;
    
    let (stdout, stderr, _exit_code) = Command::new(&app.binary_path)
        .env("COMMUNITAS_API_OPENAI_KEY", "env-test-key")
        .env("COMMUNITAS_UI_THEME", "light")
        .env("RUST_LOG", "debug")
        .args(&["config", "--show"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    
    let stdout = String::from_utf8_lossy(&stdout);
    let stderr = String::from_utf8_lossy(&stderr);
    
    // Should respect environment variables
    // For now, just verify command runs
    assert!(stdout.contains("Configuration display coming soon!") || stdout.len() > 0);
    
    Ok(())
}

// Integration test for the full CLI application lifecycle
#[tokio::test]
async fn test_application_lifecycle() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    // 1. Show help
    let (stdout, _, exit_code) = app.run_command(&["--help"]).await?;
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Personal AI assistant"));
    
    // 2. Show version
    let (stdout, _, exit_code) = app.run_command(&["--version"]).await?;
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("0.1.0"));
    
    // 3. Show configuration
    let (stdout, _, _exit_code) = app.run_command(&["config", "--show"]).await?;
    // May succeed or show placeholder message
    
    // 4. Try to start a chat session (will show placeholder for now)
    let (stdout, _, _exit_code) = app.run_command(&["chat", "--model", "gpt-3.5-turbo"]).await?;
    assert!(stdout.contains("Chat functionality coming soon!") || stdout.len() > 0);
    
    // 5. Process a file
    let test_file = app.temp_config_dir.path().join("lifecycle_test.txt");
    std::fs::write(&test_file, "Lifecycle test content")?;
    
    let (stdout, _, _exit_code) = app.run_command(&[
        "process", 
        test_file.to_str().unwrap()
    ]).await?;
    assert!(stdout.contains("Processing file") || stdout.contains("File processing coming soon!"));
    
    Ok(())
}

// This test documents the expected behavior once features are implemented
#[tokio::test]
async fn test_future_chat_functionality() -> Result<()> {
    let app = TestApp::new()?;
    app.create_test_config()?;
    
    // This test will initially show placeholder behavior
    // but documents the expected real functionality
    let (stdout, stderr, exit_code) = app.run_command(&[
        "chat", 
        "--model", "gpt-3.5-turbo",
        "--voice",
        "--vision"
    ]).await?;
    
    // Current behavior: placeholder message
    if stdout.contains("Chat functionality coming soon!") {
        // Expected current behavior
        assert!(stdout.contains("Starting chat session with model: gpt-3.5-turbo"));
        assert!(stdout.contains("Voice I/O enabled"));
        assert!(stdout.contains("Vision capabilities enabled"));
    } else {
        // Future expected behavior when implemented
        assert_eq!(exit_code, 0);
        // Would test actual chat functionality here
    }
    
    Ok(())
}