//! Configuration tests
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

use saorsa_core::config::Config;
use saorsa_core::network::NodeConfig;
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_config_defaults() {
    let config = Config::default();

    // Network defaults
    assert_eq!(config.network.listen_address, "0.0.0.0:9000");
    assert!(config.network.ipv6_enabled);
    assert_eq!(config.network.max_connections, 10000);

    // Security defaults
    assert_eq!(config.security.rate_limit, 1000);
    assert!(config.security.encryption_enabled);
    assert_eq!(config.security.min_tls_version, "1.3");

    // Storage defaults
    assert_eq!(config.storage.path.to_str().unwrap(), "./data");
    assert_eq!(config.storage.max_size, "10GB");
}

#[test]
fn test_development_config() {
    let config = Config::development();

    assert_eq!(config.network.listen_address, "127.0.0.1:9000");
    assert_eq!(config.security.rate_limit, 10000);
    assert_eq!(config.storage.path.to_str().unwrap(), "./dev-data");
}

#[test]
fn test_production_config() {
    let config = Config::production();

    assert_eq!(config.network.listen_address, "0.0.0.0:9000");
    assert_eq!(config.transport.buffer_size, 131072);
    assert!(config.storage.compression_enabled);
}

#[test]
fn test_config_from_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
[network]
listen_address = "192.168.1.100:8000"
bootstrap_nodes = ["node1:9000", "node2:9000"]
max_connections = 5000

[security]
rate_limit = 500
encryption_enabled = false
    "#
    )
    .unwrap();

    let config = Config::load_from_file(file.path()).unwrap();

    assert_eq!(config.network.listen_address, "192.168.1.100:8000");
    assert_eq!(config.network.bootstrap_nodes.len(), 2);
    assert_eq!(config.network.max_connections, 5000);
    assert_eq!(config.security.rate_limit, 500);
    assert!(!config.security.encryption_enabled);
}

#[test]
fn test_env_overrides() {
    // Set environment variables
    #[allow(unsafe_code)]
    unsafe {
        env::set_var("SAORSA_LISTEN_ADDRESS", "10.0.0.1:7000");
        env::set_var("SAORSA_RATE_LIMIT", "2000");
        env::set_var("SAORSA_BOOTSTRAP_NODES", "boot1:9000,boot2:9000,boot3:9000");
        env::set_var("SAORSA_MCP_ENABLED", "false");
    }

    let config = Config::load().unwrap();

    assert_eq!(config.network.listen_address, "10.0.0.1:7000");
    assert_eq!(config.security.rate_limit, 2000);
    assert_eq!(config.network.bootstrap_nodes.len(), 3);
    assert!(!config.mcp.enabled);

    // Clean up
    #[allow(unsafe_code)]
    unsafe {
        env::remove_var("SAORSA_LISTEN_ADDRESS");
        env::remove_var("SAORSA_RATE_LIMIT");
        env::remove_var("SAORSA_BOOTSTRAP_NODES");
        env::remove_var("SAORSA_MCP_ENABLED");
    }
}

#[test]
fn test_config_validation() {
    let mut config = Config::default();

    // Valid configuration
    assert!(config.validate().is_ok());

    // Invalid listen address
    config.network.listen_address = "invalid_address".to_string();
    assert!(config.validate().is_err());

    // Fix address but invalid storage size
    config.network.listen_address = "127.0.0.1:9000".to_string();
    config.storage.max_size = "10XB".to_string();
    assert!(config.validate().is_err());

    // Fix storage size but invalid transport
    config.storage.max_size = "10GB".to_string();
    config.transport.protocol = "invalid".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn test_node_config_from_config() {
    let config = Config::development();
    let node_config = NodeConfig::from_config(&config).unwrap();

    assert_eq!(node_config.listen_addr.to_string(), "127.0.0.1:9000");
    assert_eq!(node_config.max_connections, config.network.max_connections);
    assert_eq!(node_config.enable_ipv6, config.network.ipv6_enabled);
    assert_eq!(node_config.enable_mcp_server, config.mcp.enabled);

    // Check MCP server config
    let mcp_config = node_config.mcp_server_config.unwrap();
    assert_eq!(mcp_config.server_name, "P2P Node MCP Server");
    assert_eq!(mcp_config.enable_dht_discovery, true);
}

#[test]
fn test_bootstrap_address_parsing() {
    let mut config = Config::default();
    config.network.bootstrap_nodes = vec![
        "192.168.1.1:9000".to_string(),
        "example.com:9001".to_string(),
        "/ip4/10.0.0.1/tcp/9002".to_string(),
    ];

    assert!(config.validate().is_ok());

    let addrs = config.bootstrap_addrs().unwrap();
    assert_eq!(addrs.len(), 3);
}

// Size format validation test removed - validate_size_format is a private method

#[test]
fn test_config_save_and_load() {
    let mut config = Config::development();
    config.network.bootstrap_nodes = vec!["test1:9000".to_string(), "test2:9001".to_string()];
    config.security.rate_limit = 12345;

    let file = NamedTempFile::new().unwrap();
    config.save_to_file(file.path()).unwrap();

    let loaded = Config::load_from_file(file.path()).unwrap();
    assert_eq!(
        loaded.network.bootstrap_nodes,
        config.network.bootstrap_nodes
    );
    assert_eq!(loaded.security.rate_limit, config.security.rate_limit);
}

#[test]
fn test_multiaddr_validation() {
    let mut config = Config::default();

    // Valid multiaddr
    config.network.listen_address = "/ip4/127.0.0.1/tcp/9000".to_string();
    assert!(config.validate().is_ok());

    // Valid IPv6 multiaddr
    config.network.listen_address = "/ip6/::1/tcp/9000".to_string();
    assert!(config.validate().is_ok());

    // Invalid multiaddr
    config.network.listen_address = "/invalid/format".to_string();
    assert!(config.validate().is_err());
}
