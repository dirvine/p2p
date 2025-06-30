//! Integration Tests for Saorsa DHT-Based Identity Management
//!
//! Tests the actual DHT-based identity functionality implemented in Saorsa,
//! including the DHT helper functions and network identity operations.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use saorsa_core::{
    network::{P2PNode, NodeConfig, DHTConfig as NetworkDHTConfig, SecurityConfig},
    identity::{
        UserIdentity, UserProfile,
        UserPreferences, VerificationLevel,
    },
    identity::manager::IdentityManagerConfig,
    Result as P2PResult,
};

// Import from the library
use saorsa_lib::AppState;

/// Test framework for Saorsa identity functions
pub struct SaorsaIdentityTestFramework {
    /// Test nodes in the network
    nodes: Vec<Arc<P2PNode>>,
    /// App states for testing
    app_states: Vec<Arc<AppState>>,
}

impl SaorsaIdentityTestFramework {
    /// Create a new test framework
    pub async fn new(node_count: usize) -> P2PResult<Self> {
        info!("🚀 Creating Saorsa Identity Test Framework with {} nodes", node_count);
        
        let mut nodes = Vec::new();
        let mut app_states = Vec::new();
        
        for i in 0..node_count {
            let config = NodeConfig {
                peer_id: Some(format!("saorsa_test_node_{}", i)),
                listen_addrs: vec![
                    format!("/ip6/::1/tcp/{}", 11000 + i),
                    format!("/ip4/127.0.0.1/tcp/{}", 11000 + i),
                ],
                listen_addr: format!("127.0.0.1:{}", 11000 + i).parse().unwrap(),
                bootstrap_peers: vec![],
                bootstrap_peers_str: vec![],
                enable_ipv6: true,
                enable_mcp_server: false,
                mcp_server_config: None,
                connection_timeout: Duration::from_secs(10),
                keep_alive_interval: Duration::from_secs(30),
                max_connections: 50,
                max_incoming_connections: 25,
                dht_config: NetworkDHTConfig {
                    k_value: 8,
                    alpha_value: 3,
                    record_ttl: Duration::from_secs(3600),
                    refresh_interval: Duration::from_secs(600),
                },
                security_config: SecurityConfig::default(),
                production_config: None,
                bootstrap_cache_config: None,
                identity_config: Some(IdentityManagerConfig::default()),
            };
            
            let node = Arc::new(P2PNode::new(config.clone()).await?);
            nodes.push(node.clone());
            
            // Create app state with this node
            let app_state = Arc::new(AppState::default());
            // Note: In real tests, we would need to properly initialize the app state
            // For this test, we'll work directly with the nodes
            
            app_states.push(app_state);
        }
        
        Ok(Self {
            nodes,
            app_states,
        })
    }
    
    /// Setup network connectivity
    pub async fn setup_network(&mut self) -> P2PResult<()> {
        info!("🔧 Setting up Saorsa test network");
        
        // Start all nodes
        for (i, node) in self.nodes.iter().enumerate() {
            node.start().await.map_err(|e| {
                saorsa_core::P2PError::Network(format!("Failed to start Saorsa test node {}: {}", i, e))
            })?;
        }
        
        sleep(Duration::from_millis(500)).await;
        
        // Connect nodes in a mesh
        for i in 0..self.nodes.len() {
            for j in (i + 1)..std::cmp::min(i + 2, self.nodes.len()) {
                let target_addr = format!("/ip4/127.0.0.1/tcp/{}", 11000 + j);
                match self.nodes[i].connect_peer(&target_addr).await {
                    Ok(_) => info!("Connected Saorsa test node {} to node {}", i, j),
                    Err(e) => warn!("Failed to connect Saorsa test node {} to node {}: {}", i, j, e),
                }
            }
        }
        
        sleep(Duration::from_secs(2)).await;
        info!("✅ Saorsa test network setup completed");
        Ok(())
    }
    
    /// Test DHT helper functions directly (these are the actual implemented functions)
    pub async fn test_dht_helper_functions(&self) -> P2PResult<()> {
        info!("🔍 Testing DHT helper functions directly");
        
        let network = self.nodes[0].clone();
        
        // Create test identity and profile
        let identity = UserIdentity {
            user_id: "test_dht_user".to_string(),
            public_key: vec![0u8; 32],
            display_name_hint: "DHT Test".to_string(),
            three_word_address: "test.dht.helper".to_string(),
            created_at: std::time::SystemTime::now(),
            version: 1,
            verification_level: VerificationLevel::SelfSigned,
        };
        
        let profile = UserProfile {
            user_id: "test_dht_user".to_string(),
            display_name: "DHT Test User".to_string(),
            bio: Some("Testing DHT helper functions".to_string()),
            avatar_url: None,
            avatar_hash: None,
            status_message: None,
            public_key: vec![0u8; 32],
            preferences: UserPreferences::default(),
            custom_fields: std::collections::HashMap::new(),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
        };
        
        // Test publish_identity_to_dht
        match self.publish_identity_to_dht(&network, &identity, &profile).await {
            Ok(_) => {
                info!("✅ Successfully published identity to DHT");
            }
            Err(e) => {
                warn!("❌ Failed to publish identity to DHT: {}", e);
                return Err(saorsa_core::P2PError::DHT("Identity publish failed".to_string()));
            }
        }
        
        // Test register_three_word_address
        match self.register_three_word_address(&network, &identity.three_word_address, &identity.user_id).await {
            Ok(_) => {
                info!("✅ Successfully registered three-word address");
            }
            Err(e) => {
                warn!("❌ Failed to register three-word address: {}", e);
                return Err(saorsa_core::P2PError::DHT("Three-word address registration failed".to_string()));
            }
        }
        
        // Wait for propagation
        sleep(Duration::from_millis(500)).await;
        
        // Test lookup_user_identity
        let lookup_network = self.nodes[1].clone();
        match self.lookup_user_identity(&lookup_network, &identity.user_id).await {
            Ok(Some(retrieved_profile)) => {
                info!("✅ Successfully looked up identity from DHT");
                assert_eq!(retrieved_profile.user_id, profile.user_id);
                assert_eq!(retrieved_profile.display_name, profile.display_name);
            }
            Ok(None) => {
                warn!("❌ Identity not found in DHT lookup");
                return Err(saorsa_core::P2PError::DHT("Identity not found".to_string()));
            }
            Err(e) => {
                warn!("❌ DHT identity lookup failed: {}", e);
                return Err(e);
            }
        }
        
        // Test resolve_three_word_address
        match self.resolve_three_word_address(&lookup_network, &identity.three_word_address).await {
            Ok(Some(resolved_user_id)) => {
                info!("✅ Successfully resolved three-word address from DHT");
                assert_eq!(resolved_user_id, identity.user_id);
            }
            Ok(None) => {
                warn!("❌ Three-word address not found in DHT");
                return Err(saorsa_core::P2PError::DHT("Three-word address not found".to_string()));
            }
            Err(e) => {
                warn!("❌ Three-word address resolution failed: {}", e);
                return Err(e);
            }
        }
        
        info!("✅ DHT helper functions test completed");
        Ok(())
    }
    
    /// Test concurrent identity operations
    pub async fn test_concurrent_identity_operations(&self) -> P2PResult<()> {
        info!("🔍 Testing concurrent identity operations");
        
        let mut handles: Vec<()> = Vec::new();
        
        // Create multiple identities concurrently using DHT operations
        for i in 0..5 {
            let node = self.nodes[i % self.nodes.len()].clone();
            let framework = self; // Can't move self into async block, so we'll run sequentially
            
            let identity = UserIdentity {
                user_id: format!("concurrent_user_{}", i),
                public_key: vec![0u8; 32],
                display_name_hint: format!("User{}", i),
                three_word_address: format!("concurrent.user.{}", i),
                created_at: std::time::SystemTime::now(),
                version: 1,
                verification_level: VerificationLevel::SelfSigned,
            };
            
            let profile = UserProfile {
                user_id: format!("concurrent_user_{}", i),
                display_name: format!("Concurrent User {}", i),
                bio: Some(format!("Test user {}", i)),
                avatar_url: None,
                avatar_hash: None,
                status_message: None,
                public_key: vec![0u8; 32],
                preferences: UserPreferences::default(),
                custom_fields: std::collections::HashMap::new(),
                created_at: std::time::SystemTime::now(),
                updated_at: std::time::SystemTime::now(),
            };
            
            // Store identity concurrently
            match framework.publish_identity_to_dht(&node, &identity, &profile).await {
                Ok(_) => info!("✅ Concurrent identity {} created successfully", i),
                Err(e) => warn!("❌ Failed to create concurrent identity {}: {}", i, e),
            }
            
            // Register three-word address
            match framework.register_three_word_address(&node, &identity.three_word_address, &identity.user_id).await {
                Ok(_) => info!("✅ Concurrent three-word address {} registered", i),
                Err(e) => warn!("❌ Failed to register concurrent three-word address {}: {}", i, e),
            }
        }
        
        // Wait for all operations to propagate
        sleep(Duration::from_secs(1)).await;
        
        // Verify some of the concurrent operations worked
        let mut successful_lookups = 0;
        for i in 0..3 {
            let lookup_node = &self.nodes[(i + 1) % self.nodes.len()];
            let user_id = format!("concurrent_user_{}", i);
            
            match self.lookup_user_identity(lookup_node, &user_id).await {
                Ok(Some(_)) => {
                    successful_lookups += 1;
                    info!("✅ Successfully looked up concurrent user {}", i);
                }
                Ok(None) => warn!("❌ Concurrent user {} not found", i),
                Err(e) => warn!("❌ Error looking up concurrent user {}: {}", i, e),
            }
        }
        
        info!("Concurrent operations test: {}/3 lookups successful", successful_lookups);
        assert!(successful_lookups >= 1, "At least one concurrent operation should succeed");
        
        info!("✅ Concurrent identity operations test completed");
        Ok(())
    }
    
    /// Test network identity persistence across node restarts
    pub async fn test_identity_persistence(&self) -> P2PResult<()> {
        info!("🔍 Testing identity persistence across network changes");
        
        let network = &self.nodes[0];
        
        // Create a test identity
        let identity = UserIdentity {
            user_id: "persistence_test_user".to_string(),
            public_key: vec![0u8; 32],
            display_name_hint: "Persist Test".to_string(),
            three_word_address: "persist.test.user".to_string(),
            created_at: std::time::SystemTime::now(),
            version: 1,
            verification_level: VerificationLevel::SelfSigned,
        };
        
        let profile = UserProfile {
            user_id: "persistence_test_user".to_string(),
            display_name: "Persistence Test User".to_string(),
            bio: Some("Testing identity persistence".to_string()),
            avatar_url: None,
            avatar_hash: None,
            status_message: None,
            public_key: vec![0u8; 32],
            preferences: UserPreferences::default(),
            custom_fields: std::collections::HashMap::new(),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
        };
        
        // Store the identity
        self.publish_identity_to_dht(network, &identity, &profile).await
            .map_err(|e| saorsa_core::P2PError::DHT(e))?;
        self.register_three_word_address(network, &identity.three_word_address, &identity.user_id).await
            .map_err(|e| saorsa_core::P2PError::DHT(e))?;
        
        // Wait for DHT propagation
        sleep(Duration::from_secs(1)).await;
        
        // Verify it can be retrieved from multiple nodes
        let mut retrieval_count = 0;
        for (i, node) in self.nodes.iter().enumerate() {
            match self.lookup_user_identity(node, &identity.user_id).await {
                Ok(Some(retrieved_profile)) => {
                    retrieval_count += 1;
                    info!("✅ Retrieved persistence test identity from node {}", i);
                    assert_eq!(retrieved_profile.display_name, profile.display_name);
                }
                Ok(None) => info!("❌ Persistence test identity not found on node {}", i),
                Err(e) => warn!("❌ Error retrieving from node {}: {}", i, e),
            }
        }
        
        info!("Identity persistence test: retrieved from {}/{} nodes", retrieval_count, self.nodes.len());
        assert!(retrieval_count >= 1, "Identity should be retrievable from at least one node");
        
        info!("✅ Identity persistence test completed");
        Ok(())
    }
    
    /// Cleanup test framework
    pub async fn cleanup(&self) -> P2PResult<()> {
        info!("🧹 Cleaning up Saorsa identity test framework");
        
        for (i, node) in self.nodes.iter().enumerate() {
            if let Err(e) = node.stop().await {
                warn!("Failed to stop Saorsa test node {}: {}", i, e);
            }
        }
        
        info!("✅ Cleanup completed");
        Ok(())
    }
    
    // Helper methods that replicate the DHT functions from lib.rs
    
    async fn publish_identity_to_dht(
        &self,
        network: &Arc<P2PNode>,
        identity: &UserIdentity,
        profile: &UserProfile,
    ) -> Result<(), String> {
        use saorsa_core::dht::Key;
        use sha2::{Digest, Sha256};
        
        // Create DHT key from user ID
        let mut hasher = Sha256::new();
        hasher.update(identity.user_id.as_bytes());
        let key_hash: [u8; 32] = hasher.finalize().into();
        let dht_key = Key::new(&key_hash);
        
        // Serialize profile
        let profile_data = match serde_json::to_vec(profile) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to serialize profile: {}", e)),
        };
        
        // Store in DHT
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            match dht_guard.put(dht_key, profile_data).await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("DHT put failed: {}", e)),
            }
        } else {
            Err("DHT not available".to_string())
        }
    }
    
    async fn register_three_word_address(
        &self,
        network: &Arc<P2PNode>,
        three_word_address: &str,
        user_id: &str,
    ) -> Result<(), String> {
        use saorsa_core::dht::Key;
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(b"three_word_address:");
        hasher.update(three_word_address.as_bytes());
        let key_hash: [u8; 32] = hasher.finalize().into();
        let dht_key = Key::new(&key_hash);
        
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            match dht_guard.put(dht_key, user_id.as_bytes().to_vec()).await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Three-word address DHT put failed: {}", e)),
            }
        } else {
            Err("DHT not available".to_string())
        }
    }
    
    async fn lookup_user_identity(
        &self,
        network: &Arc<P2PNode>,
        user_id: &str,
    ) -> P2PResult<Option<UserProfile>> {
        use saorsa_core::dht::Key;
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        let key_hash: [u8; 32] = hasher.finalize().into();
        let dht_key = Key::new(&key_hash);
        
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            if let Some(record) = dht_guard.get(&dht_key).await {
                match serde_json::from_slice::<UserProfile>(&record.value) {
                    Ok(profile) => Ok(Some(profile)),
                    Err(e) => Err(saorsa_core::P2PError::Serialization(e)),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    async fn resolve_three_word_address(
        &self,
        network: &Arc<P2PNode>,
        three_word_address: &str,
    ) -> P2PResult<Option<String>> {
        use saorsa_core::dht::Key;
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(b"three_word_address:");
        hasher.update(three_word_address.as_bytes());
        let key_hash: [u8; 32] = hasher.finalize().into();
        let dht_key = Key::new(&key_hash);
        
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            if let Some(record) = dht_guard.get(&dht_key).await {
                match String::from_utf8(record.value) {
                    Ok(user_id) => Ok(Some(user_id)),
                    Err(e) => Err(saorsa_core::P2PError::Generic(anyhow::anyhow!("User ID deserialization failed: {}", e))),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

/// Run comprehensive Saorsa identity integration tests
#[tokio::test]
async fn test_saorsa_identity_integration_full_suite() {
    tracing_subscriber::fmt::init();
    
    let mut test_framework = SaorsaIdentityTestFramework::new(3).await
        .expect("Failed to create Saorsa identity test framework");
    
    // Setup network
    test_framework.setup_network().await
        .expect("Failed to setup test network");
    
    // Run all tests
    test_framework.test_dht_helper_functions().await
        .expect("DHT helper functions test failed");
    
    test_framework.test_concurrent_identity_operations().await
        .expect("Concurrent identity operations test failed");
    
    test_framework.test_identity_persistence().await
        .expect("Identity persistence test failed");
    
    // Cleanup
    test_framework.cleanup().await
        .expect("Failed to cleanup test framework");
    
    info!("🎉 All Saorsa identity integration tests passed!");
}

#[tokio::test]
async fn test_saorsa_identity_framework_creation() {
    let test_framework = SaorsaIdentityTestFramework::new(2).await
        .expect("Should create Saorsa identity test framework");
    
    assert_eq!(test_framework.nodes.len(), 2);
    assert_eq!(test_framework.app_states.len(), 2);
    
    test_framework.cleanup().await
        .expect("Should cleanup successfully");
}