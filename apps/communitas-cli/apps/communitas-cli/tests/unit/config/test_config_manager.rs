// Copyright 2025 Saorsa Labs Limited
// Tests for configuration management system

use anyhow::Result;
use communitas_cli::config::{ConfigManager, ConfigError};
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_default_config_creation() {
    let config = ConfigManager::default();
    
    // Test API defaults
    assert_eq!(config.get::<String>("api.default_model").unwrap(), "gpt-4");
    assert_eq!(config.get::<f64>("api.temperature").unwrap(), 0.7);
    
    // Test UI defaults
    assert_eq!(config.get::<String>("ui.theme").unwrap(), "dark");
    assert_eq!(config.get::<bool>("ui.auto_save").unwrap(), true);
    assert_eq!(config.get::<u32>("ui.history_limit").unwrap(), 1000);
    
    // Test network defaults
    assert_eq!(config.get::<bool>("network.p2p_enabled").unwrap(), true);
    assert_eq!(config.get::<u16>("network.listen_port").unwrap(), 9000);
    
    // Test voice defaults
    assert_eq!(config.get::<bool>("voice.enabled").unwrap(), true);
    assert_eq!(config.get::<String>("voice.language").unwrap(), "en");
    assert_eq!(config.get::<bool>("voice.voice_activation").unwrap(), false);
    assert_eq!(config.get::<String>("voice.wake_word").unwrap(), "communitas");
    
    // Test file defaults
    assert_eq!(config.get::<bool>("file.auto_process").unwrap(), false);
    assert_eq!(config.get::<String>("file.max_file_size").unwrap(), "100MB");
    
    // Test privacy defaults
    assert_eq!(config.get::<bool>("privacy.telemetry_enabled").unwrap(), false);
    assert_eq!(config.get::<bool>("privacy.crash_reporting").unwrap(), false);
    assert_eq!(config.get::<u32>("privacy.data_retention_days").unwrap(), 90);
}

#[test]
fn test_config_file_loading() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    let test_config = r#"
[api]
openai_key = "sk-test123456"
anthropic_key = "sk-ant-test123"
default_model = "claude-3"
temperature = 0.5

[ui]
theme = "light"
auto_save = false
history_limit = 500

[network]
p2p_enabled = false
listen_port = 8000
bootstrap_nodes = ["/ip4/1.2.3.4/tcp/9000"]

[voice]
enabled = false
language = "es"
voice_activation = true
wake_word = "asistente"

[file]
auto_process = true
max_file_size = "50MB"
output_directory = "/custom/output"
supported_formats = ["pdf", "txt"]

[privacy]
telemetry_enabled = true
crash_reporting = true
data_retention_days = 30
"#;
    
    std::fs::write(&config_path, test_config)?;
    let config = ConfigManager::load_from_path(&config_path)?;
    
    // Verify loaded values
    assert_eq!(config.get::<String>("api.openai_key")?, "sk-test123456");
    assert_eq!(config.get::<String>("api.anthropic_key")?, "sk-ant-test123");
    assert_eq!(config.get::<String>("api.default_model")?, "claude-3");
    assert_eq!(config.get::<f64>("api.temperature")?, 0.5);
    
    assert_eq!(config.get::<String>("ui.theme")?, "light");
    assert_eq!(config.get::<bool>("ui.auto_save")?, false);
    assert_eq!(config.get::<u32>("ui.history_limit")?, 500);
    
    assert_eq!(config.get::<bool>("network.p2p_enabled")?, false);
    assert_eq!(config.get::<u16>("network.listen_port")?, 8000);
    
    let bootstrap_nodes = config.get::<Vec<String>>("network.bootstrap_nodes")?;
    assert_eq!(bootstrap_nodes, vec!["/ip4/1.2.3.4/tcp/9000"]);
    
    assert_eq!(config.get::<bool>("voice.enabled")?, false);
    assert_eq!(config.get::<String>("voice.language")?, "es");
    assert_eq!(config.get::<bool>("voice.voice_activation")?, true);
    assert_eq!(config.get::<String>("voice.wake_word")?, "asistente");
    
    assert_eq!(config.get::<bool>("file.auto_process")?, true);
    assert_eq!(config.get::<String>("file.max_file_size")?, "50MB");
    assert_eq!(config.get::<String>("file.output_directory")?, "/custom/output");
    
    let supported_formats = config.get::<Vec<String>>("file.supported_formats")?;
    assert_eq!(supported_formats, vec!["pdf", "txt"]);
    
    assert_eq!(config.get::<bool>("privacy.telemetry_enabled")?, true);
    assert_eq!(config.get::<bool>("privacy.crash_reporting")?, true);
    assert_eq!(config.get::<u32>("privacy.data_retention_days")?, 30);
    
    Ok(())
}

#[test]
fn test_environment_variable_override() -> Result<()> {
    // Set test environment variables
    std::env::set_var("COMMUNITAS_API_OPENAI_KEY", "env-openai-key");
    std::env::set_var("COMMUNITAS_API_DEFAULT_MODEL", "env-model");
    std::env::set_var("COMMUNITAS_UI_THEME", "env-theme");
    std::env::set_var("COMMUNITAS_NETWORK_LISTEN_PORT", "7777");
    std::env::set_var("COMMUNITAS_VOICE_ENABLED", "false");
    
    let config = ConfigManager::load()?;
    
    // Environment variables should override defaults
    assert_eq!(config.get::<String>("api.openai_key")?, "env-openai-key");
    assert_eq!(config.get::<String>("api.default_model")?, "env-model");
    assert_eq!(config.get::<String>("ui.theme")?, "env-theme");
    assert_eq!(config.get::<u16>("network.listen_port")?, 7777);
    assert_eq!(config.get::<bool>("voice.enabled")?, false);
    
    // Cleanup
    std::env::remove_var("COMMUNITAS_API_OPENAI_KEY");
    std::env::remove_var("COMMUNITAS_API_DEFAULT_MODEL");
    std::env::remove_var("COMMUNITAS_UI_THEME");
    std::env::remove_var("COMMUNITAS_NETWORK_LISTEN_PORT");
    std::env::remove_var("COMMUNITAS_VOICE_ENABLED");
    
    Ok(())
}

#[test]
fn test_config_validation() {
    let mut config = ConfigManager::default();
    
    // Test valid configurations
    assert!(config.set("api.temperature", 0.5).is_ok());
    assert!(config.set("ui.theme", "dark").is_ok());
    assert!(config.set("network.listen_port", 8080).is_ok());
    assert!(config.set("file.max_file_size", "100MB").is_ok());
    assert!(config.validate().is_ok());
    
    // Test invalid configurations
    assert!(config.set("api.temperature", -1.0).is_err()); // Temperature out of range
    assert!(config.set("ui.theme", "invalid_theme").is_err()); // Invalid theme
    assert!(config.set("network.listen_port", 70000).is_err()); // Port out of range
    assert!(config.set("file.max_file_size", "invalid").is_err()); // Invalid size format
}

#[test]
fn test_config_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    // Create and modify config
    let mut config = ConfigManager::default();
    config.set("api.openai_key", "test-key-123")?;
    config.set("ui.theme", "light")?;
    config.set("voice.enabled", false)?;
    
    // Save config
    config.save_to_path(&config_path)?;
    
    // Load config from saved file
    let loaded_config = ConfigManager::load_from_path(&config_path)?;
    
    // Verify persistence
    assert_eq!(loaded_config.get::<String>("api.openai_key")?, "test-key-123");
    assert_eq!(loaded_config.get::<String>("ui.theme")?, "light");
    assert_eq!(loaded_config.get::<bool>("voice.enabled")?, false);
    
    // Other defaults should be preserved
    assert_eq!(loaded_config.get::<String>("api.default_model")?, "gpt-4");
    assert_eq!(loaded_config.get::<bool>("ui.auto_save")?, true);
    
    Ok(())
}

#[test]
fn test_secure_api_key_storage() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    let mut config = ConfigManager::default();
    config.set("api.openai_key", "sk-secret123456")?;
    config.set("api.anthropic_key", "sk-ant-secret789")?;
    
    // Save config (keys should be encrypted)
    config.save_to_path(&config_path)?;
    
    // Read raw file content
    let file_content = std::fs::read_to_string(&config_path)?;
    
    // Raw API keys should NOT appear in file
    assert!(!file_content.contains("sk-secret123456"));
    assert!(!file_content.contains("sk-ant-secret789"));
    
    // But should contain encrypted markers
    assert!(file_content.contains("[api]"));
    assert!(file_content.contains("openai_key"));
    assert!(file_content.contains("anthropic_key"));
    
    // Loading should decrypt correctly
    let loaded_config = ConfigManager::load_from_path(&config_path)?;
    assert_eq!(loaded_config.get::<String>("api.openai_key")?, "sk-secret123456");
    assert_eq!(loaded_config.get::<String>("api.anthropic_key")?, "sk-ant-secret789");
    
    Ok(())
}

#[test]
fn test_config_migration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    // Create old format config (version 1)
    let old_config = r#"
version = 1

[api]
openai_key = "old-key"
model = "gpt-3.5-turbo"

[settings]
theme = "dark"
"#;
    
    std::fs::write(&config_path, old_config)?;
    
    // Loading should trigger migration
    let config = ConfigManager::load_from_path(&config_path)?;
    
    // Values should be migrated to new format
    assert_eq!(config.get::<String>("api.openai_key")?, "old-key");
    assert_eq!(config.get::<String>("api.default_model")?, "gpt-3.5-turbo"); // migrated from "model"
    assert_eq!(config.get::<String>("ui.theme")?, "dark"); // migrated from "settings.theme"
    
    // New defaults should be present
    assert_eq!(config.get::<f64>("api.temperature")?, 0.7);
    assert_eq!(config.get::<bool>("voice.enabled")?, true);
    
    Ok(())
}

#[test]
fn test_missing_config_file_creates_default() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let non_existent_path = temp_dir.path().join("missing.toml");
    
    // Loading non-existent file should create default config
    let config = ConfigManager::load_from_path(&non_existent_path)?;
    
    // Should have all default values
    assert_eq!(config.get::<String>("api.default_model")?, "gpt-4");
    assert_eq!(config.get::<String>("ui.theme")?, "dark");
    assert_eq!(config.get::<bool>("voice.enabled")?, true);
    
    // File should be created with defaults
    assert!(non_existent_path.exists());
    
    Ok(())
}

#[test]
fn test_partial_config_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    // Create partial config with only some values
    let partial_config = r#"
[api]
openai_key = "partial-key"

[ui]
theme = "light"
"#;
    
    std::fs::write(&config_path, partial_config)?;
    let config = ConfigManager::load_from_path(&config_path)?;
    
    // Specified values should be loaded
    assert_eq!(config.get::<String>("api.openai_key")?, "partial-key");
    assert_eq!(config.get::<String>("ui.theme")?, "light");
    
    // Missing values should use defaults
    assert_eq!(config.get::<String>("api.default_model")?, "gpt-4");
    assert_eq!(config.get::<f64>("api.temperature")?, 0.7);
    assert_eq!(config.get::<bool>("ui.auto_save")?, true);
    assert_eq!(config.get::<bool>("voice.enabled")?, true);
    
    Ok(())
}

#[test]
fn test_config_change_notifications() -> Result<()> {
    let mut config = ConfigManager::default();
    let mut notification_count = 0;
    
    // Register change listener
    config.on_change(|key, old_value, new_value| {
        notification_count += 1;
        println!("Config changed: {} = {} -> {}", key, old_value, new_value);
    });
    
    // Make changes
    config.set("ui.theme", "light")?;
    config.set("voice.enabled", false)?;
    config.set("api.temperature", 0.9)?;
    
    // Should have received notifications
    assert_eq!(notification_count, 3);
    
    Ok(())
}

// This test will fail initially because the ConfigManager doesn't exist yet
#[test]
#[should_panic(expected = "ConfigManager not implemented")]
fn test_config_manager_not_implemented() {
    let _config = ConfigManager::default();
}