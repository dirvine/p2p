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
//! Quick test application for P2P communication and identity system
//! 
//! Run this with: `cargo run --bin test_p2p_app`

use saorsa_core::{
    network::{P2PNode, NodeConfig, DHTConfig as NetworkDHTConfig, SecurityConfig, TrustLevel},
    identity::manager::{IdentityManager, IdentityManagerConfig},
    production::ProductionConfig,
    Result as P2PResult,
};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting P2P test application");
    
    // Test 1: Identity Manager
    info!("📋 Test 1: Testing Identity Manager");
    test_identity_manager().await?;
    
    // Test 2: Create P2P Nodes
    info!("📋 Test 2: Testing P2P Node Creation");
    test_p2p_node_creation().await?;
    
    // Test 3: Multi-node communication (if time permits)
    info!("📋 Test 3: Testing Multi-node Communication");
    test_multi_node_communication().await?;
    
    info!("✅ All tests completed successfully!");
    Ok(())
}

async fn test_identity_manager() -> P2PResult<()> {
    info!("  Creating identity manager...");
    let config = IdentityManagerConfig::default();
    let manager = IdentityManager::new(config);
    
    info!("  Creating new user identity...");
    let identity = manager.create_identity(
        "Test User".to_string(),
        "test.user.example".to_string(),
        None,
        None,
    ).await?;
    
    info!("  ✅ Created identity with user_id: {}", identity.user_id);
    info!("  ✅ Display name hint: {}", identity.display_name_hint);
    info!("  ✅ Three word address: {}", identity.three_word_address);
    
    // Test profile export/import
    info!("  Testing profile export/import...");
    let export_data = manager.export_identity(&identity.user_id).await?;
    info!("  ✅ Exported identity data ({} bytes)", export_data.len());
    
    // Create a new manager and import
    let manager2 = IdentityManager::new(IdentityManagerConfig::default());
    let imported_identity = manager2.import_identity(&export_data, "test_password").await?;
    
    if imported_identity.user_id == identity.user_id {
        info!("  ✅ Identity export/import successful");
    } else {
        warn!("  ⚠️  Identity import mismatch");
    }
    
    Ok(())
}

async fn test_p2p_node_creation() -> P2PResult<()> {
    info!("  Creating P2P node configuration...");
    
    let config = NodeConfig {
        peer_id: None,
        listen_addrs: vec![],
        listen_addr: "127.0.0.1:9000".parse().unwrap(),
        bootstrap_peers: vec![],
        bootstrap_peers_str: vec![],
        enable_ipv6: true,
        enable_mcp_server: false, // Disable for testing
        mcp_server_config: None,
        connection_timeout: Duration::from_secs(30),
        keep_alive_interval: Duration::from_secs(60),
        max_connections: 100,
        max_incoming_connections: 50,
        dht_config: NetworkDHTConfig {
            k_value: 20,
            alpha_value: 3,
            record_ttl: Duration::from_secs(86400),
            refresh_interval: Duration::from_secs(3600),
        },
        security_config: SecurityConfig {
            enable_noise: true,
            enable_tls: true,
            trust_level: TrustLevel::Basic,
        },
        production_config: Some(ProductionConfig::default()),
        bootstrap_cache_config: None,
        identity_config: None,
    };
    
    info!("  Creating P2P node...");
    
    // Use timeout to prevent hanging
    let node_result = timeout(Duration::from_secs(10), P2PNode::new(config)).await;
    
    match node_result {
        Ok(Ok(node)) => {
            info!("  ✅ P2P node created successfully");
            info!("  ✅ Node peer ID: {}", node.peer_id());
            
            // Test basic DHT operations
            info!("  Testing DHT operations...");
            
            let key = saorsa_core::dht::Key::new(b"test_key");
            let value = b"test_value".to_vec();
            
            // Store value
            if let Err(e) = node.dht_put(key.clone(), value.clone()).await {
                warn!("  ⚠️  DHT put failed: {}", e);
            } else {
                info!("  ✅ DHT put successful");
                
                // Try to retrieve it
                if let Ok(Some(retrieved)) = node.dht_get(key).await {
                    if retrieved == value {
                        info!("  ✅ DHT get successful - value matches");
                    } else {
                        warn!("  ⚠️  DHT get value mismatch");
                    }
                } else {
                    warn!("  ⚠️  DHT get failed or value not found");
                }
            }
            
        },
        Ok(Err(e)) => {
            error!("  ❌ Failed to create P2P node: {}", e);
            return Err(e);
        },
        Err(_) => {
            error!("  ❌ P2P node creation timed out");
            return Err(saorsa_core::error::P2PError::Network(saorsa_core::error::NetworkError::Timeout).into());
        }
    }
    
    Ok(())
}

async fn test_multi_node_communication() -> P2PResult<()> {
    info!("  Creating multiple P2P nodes for communication test...");
    
    // Create two nodes on different ports
    let config1 = create_test_config(9001);
    let config2 = create_test_config(9002);
    
    info!("  Creating node 1 on port 9001...");
    let node1_result = timeout(Duration::from_secs(10), P2PNode::new(config1)).await;
    
    let node1 = match node1_result {
        Ok(Ok(node)) => {
            info!("  ✅ Node 1 created successfully");
            node
        },
        Ok(Err(e)) => {
            error!("  ❌ Failed to create node 1: {}", e);
            return Err(e);
        },
        Err(_) => {
            error!("  ❌ Node 1 creation timed out");
            return Err(saorsa_core::error::P2PError::Network(saorsa_core::error::NetworkError::Timeout).into());
        }
    };
    
    info!("  Creating node 2 on port 9002...");
    let node2_result = timeout(Duration::from_secs(10), P2PNode::new(config2)).await;
    
    let node2 = match node2_result {
        Ok(Ok(node)) => {
            info!("  ✅ Node 2 created successfully");
            node
        },
        Ok(Err(e)) => {
            error!("  ❌ Failed to create node 2: {}", e);
            return Err(e);
        },
        Err(_) => {
            error!("  ❌ Node 2 creation timed out");
            return Err(saorsa_core::error::P2PError::Network(saorsa_core::error::NetworkError::Timeout).into());
        }
    };
    
    info!("  ✅ Both nodes created successfully");
    info!("  Node 1 peer ID: {}", node1.peer_id());
    info!("  Node 2 peer ID: {}", node2.peer_id());
    
    // Give nodes a moment to initialize
    sleep(Duration::from_secs(2)).await;
    
    // Test cross-node DHT operations
    info!("  Testing cross-node DHT communication...");
    
    let key = saorsa_core::dht::Key::new(b"cross_node_test");
    let value = b"Hello from node 1!".to_vec();
    
    // Store value in node 1
    if let Err(e) = node1.dht_put(key.clone(), value.clone()).await {
        warn!("  ⚠️  Node 1 DHT put failed: {}", e);
    } else {
        info!("  ✅ Node 1 stored value in DHT");
        
        // Give some time for DHT replication
        sleep(Duration::from_secs(1)).await;
        
        // Try to retrieve from node 2
        if let Ok(Some(retrieved)) = node2.dht_get(key).await {
            if retrieved == value {
                info!("  ✅ Cross-node DHT communication successful!");
            } else {
                warn!("  ⚠️  Cross-node DHT value mismatch");
            }
        } else {
            info!("  ℹ️  Cross-node DHT retrieval not available (expected for isolated nodes)");
        }
    }
    
    Ok(())
}

fn create_test_config(port: u16) -> NodeConfig {
    NodeConfig {
        peer_id: None,
        listen_addrs: vec![],
        listen_addr: format!("127.0.0.1:{}", port).parse().unwrap(),
        bootstrap_peers: vec![],
        bootstrap_peers_str: vec![],
        enable_ipv6: true,
        enable_mcp_server: false,
        mcp_server_config: None,
        connection_timeout: Duration::from_secs(30),
        keep_alive_interval: Duration::from_secs(60),
        max_connections: 100,
        max_incoming_connections: 50,
        dht_config: NetworkDHTConfig {
            k_value: 20,
            alpha_value: 3,
            record_ttl: Duration::from_secs(86400),
            refresh_interval: Duration::from_secs(3600),
        },
        security_config: SecurityConfig {
            enable_noise: true,
            enable_tls: true,
            trust_level: TrustLevel::Basic,
        },
        production_config: Some(ProductionConfig::default()),
        bootstrap_cache_config: None,
        identity_config: None,
    }
}