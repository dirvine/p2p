//! End-to-End Scenario Tests
//!
//! These tests simulate real-world usage patterns and complete workflows
//! for the P2P Foundation system, focusing on network formation, bootstrap
//! functionality, and basic infrastructure operations.

use anyhow::Result;
use p2p_foundation::{
    P2PNode, NodeConfig, PeerId, 
    bootstrap::CacheConfig,
    dht::Key,
};
use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use tracing::{info, debug, warn};
use futures::future::join_all;

/// Test configuration for E2E scenarios
const E2E_NETWORK_SIZE: usize = 8;
const DATA_REPLICATION_TIMEOUT: Duration = Duration::from_secs(15);
const NODE_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(30);

/// End-to-end test network simulation
struct E2ENetwork {
    nodes: Vec<P2PNode>,
    _cache_dirs: Vec<TempDir>, // Keep alive for cleanup
    node_configs: Vec<NodeConfig>,
    base_port: u16,
}

impl E2ENetwork {
    /// Create a realistic P2P network for E2E testing
    async fn new(size: usize) -> Result<Self> {
        let base_port = 25000 + (rand::random::<u16>() % 1000);
        let mut nodes = Vec::new();
        let mut cache_dirs = Vec::new();
        let mut node_configs = Vec::new();

        for i in 0..size {
            let cache_dir = TempDir::new()?;
            let cache_config = CacheConfig {
                cache_dir: cache_dir.path().to_path_buf(),
                max_contacts: 200,
                ..CacheConfig::default()
            };

            let listen_addr: SocketAddr = format!("127.0.0.1:{}", base_port + i as u16).parse()?;
            
            let mut config = NodeConfig::default();
            config.listen_addr = listen_addr;
            config.bootstrap_cache_config = Some(cache_config);
            config.enable_mcp_server = true; // Enable full functionality
            config.max_connections = 20;
            config.max_incoming_connections = 10;
            
            // Create realistic bootstrap topology
            if i > 0 {
                // Each node connects to 2-3 previous nodes for redundancy
                let bootstrap_count = (i.min(3)).max(1);
                let start_index = if i >= bootstrap_count { i - bootstrap_count } else { 0 };
                config.bootstrap_peers_str = (start_index..i)
                    .map(|j| format!("127.0.0.1:{}", base_port + j as u16))
                    .collect();
            }

            let node = P2PNode::new(config.clone()).await?;
            
            nodes.push(node);
            cache_dirs.push(cache_dir);
            node_configs.push(config);
        }

        Ok(Self {
            nodes,
            _cache_dirs: cache_dirs,
            node_configs,
            base_port,
        })
    }

    /// Start network with realistic timing
    async fn start_network(&mut self) -> Result<()> {
        info!("Starting E2E network with {} nodes", self.nodes.len());
        
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.start().await?;
            info!("Started E2E node {} on port {}", i, self.base_port + i as u16);
            
            // Realistic startup timing
            sleep(Duration::from_millis(200)).await;
        }

        // Wait for network formation
        sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    /// Get node by index
    fn get_node(&self, index: usize) -> &P2PNode {
        &self.nodes[index]
    }

    /// Stop all nodes
    async fn stop_all(&mut self) -> Result<()> {
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if let Err(e) = node.stop().await {
                warn!("Failed to stop E2E node {}: {}", i, e);
            }
        }
        Ok(())
    }

    /// Wait for network convergence with connectivity checks
    async fn wait_for_convergence(&self) -> Result<()> {
        let start_time = tokio::time::Instant::now();
        
        while start_time.elapsed() < NODE_DISCOVERY_TIMEOUT {
            let mut total_peers = 0;
            let mut connected_nodes = 0;
            
            for (i, node) in self.nodes.iter().enumerate() {
                let peer_count = node.peer_count().await;
                if peer_count > 0 {
                    total_peers += peer_count;
                    connected_nodes += 1;
                }
                debug!("E2E node {} has {} peers", i, peer_count);
            }

            let connectivity_ratio = connected_nodes as f64 / self.nodes.len() as f64;
            
            if connectivity_ratio >= 0.75 && total_peers >= self.nodes.len() * 2 {
                info!("E2E network converged: {}/{} nodes connected with {} total peer connections",
                      connected_nodes, self.nodes.len(), total_peers);
                return Ok(());
            }

            sleep(Duration::from_millis(500)).await;
        }

        anyhow::bail!("E2E network failed to converge within timeout");
    }

    /// Verify data replication across the network
    async fn verify_data_replication(&self, data_map: &HashMap<Key, Vec<u8>>) -> Result<f64> {
        let mut successful_retrievals = 0;
        let total_operations = data_map.len() * self.nodes.len();

        for (key, expected_value) in data_map {
            for (i, node) in self.nodes.iter().enumerate() {
                match timeout(Duration::from_secs(10), node.dht_get(key.clone())).await {
                    Ok(Ok(Some(value))) if value == *expected_value => {
                        successful_retrievals += 1;
                        debug!("Node {} successfully retrieved key", i);
                    }
                    Ok(Ok(Some(_))) => {
                        debug!("Node {} retrieved incorrect value for key", i);
                    }
                    Ok(Ok(None)) => {
                        debug!("Node {} could not find key", i);
                    }
                    Ok(Err(e)) => {
                        debug!("Node {} DHT get failed: {}", i, e);
                    }
                    Err(_) => {
                        debug!("Node {} DHT get timed out", i);
                    }
                }
            }
        }

        let success_rate = successful_retrievals as f64 / total_operations as f64;
        Ok(success_rate)
    }

    /// Get network statistics
    async fn get_network_stats(&self) -> Result<NetworkStats> {
        let mut total_peers = 0;
        let mut total_cached_contacts = 0;
        let mut nodes_with_cache_stats = 0;
        
        for node in &self.nodes {
            total_peers += node.peer_count().await;
            total_cached_contacts += node.cached_peer_count().await;
            
            if let Ok(Some(_)) = node.get_bootstrap_cache_stats().await {
                nodes_with_cache_stats += 1;
            }
        }

        Ok(NetworkStats {
            total_nodes: self.nodes.len(),
            total_peers,
            total_cached_contacts,
            nodes_with_cache_stats,
            avg_peers_per_node: total_peers as f64 / self.nodes.len() as f64,
        })
    }
}

/// Network statistics for analysis
#[derive(Debug)]
struct NetworkStats {
    total_nodes: usize,
    total_peers: usize,
    total_cached_contacts: usize,
    nodes_with_cache_stats: usize,
    avg_peers_per_node: f64,
}

/// Test distributed file storage and retrieval scenario
#[tokio::test]
async fn test_distributed_file_storage_scenario() -> Result<()> {
    let mut network = E2ENetwork::new(E2E_NETWORK_SIZE).await?;
    
    info!("Starting distributed file storage scenario");
    
    network.start_network().await?;
    network.wait_for_convergence().await?;

    // Simulate storing multiple files across the network
    let files = vec![
        ("document.txt", "This is a text document stored in the P2P network."),
        ("config.json", "{\"setting1\": \"value1\", \"setting2\": \"value2\"}"),
        ("data.csv", "name,age,city\nAlice,30,NYC\nBob,25,SF\nCharlie,35,LA"),
        ("image.bin", "binary_data_placeholder_1024_bytes"),
        ("large_file.dat", "large_file_data_placeholder_10240_bytes"),
    ];

    let mut stored_files = HashMap::new();
    
    // Store files using different nodes (distributed upload)
    for (i, (filename, content)) in files.iter().enumerate() {
        let key = Key::new(filename.as_bytes());
        let node_index = i % network.nodes.len();
        let node = network.get_node(node_index);
        
        match timeout(Duration::from_secs(15), node.dht_put(key.clone(), content.as_bytes().to_vec())).await {
            Ok(Ok(_)) => {
                stored_files.insert(key, content.as_bytes().to_vec());
                info!("Stored file '{}' ({} bytes) via node {}", filename, content.len(), node_index);
            }
            Ok(Err(e)) => {
                warn!("Failed to store file '{}' via node {}: {}", filename, node_index, e);
            }
            Err(_) => {
                warn!("Timeout storing file '{}' via node {}", filename, node_index);
            }
        }
        
        // Small delay between uploads
        sleep(Duration::from_millis(100)).await;
    }

    // Wait for replication
    sleep(DATA_REPLICATION_TIMEOUT).await;

    // Verify files can be retrieved from any node
    let replication_rate = network.verify_data_replication(&stored_files).await?;
    info!("File replication rate: {:.1}%", replication_rate * 100.0);

    // Test specific file retrieval scenarios
    let document_key = Key::new(b"document.txt");
    let mut retrieval_success_count = 0;
    
    for (i, node) in network.nodes.iter().enumerate() {
        if let Ok(Some(content)) = node.dht_get(document_key.clone()).await {
            if content == files[0].1.as_bytes() {
                retrieval_success_count += 1;
                debug!("Node {} successfully retrieved document.txt", i);
            }
        }
    }

    assert!(retrieval_success_count >= 1, 
           "No nodes can retrieve the document");
    assert!(replication_rate >= 0.1, 
           "File replication rate too low: {:.1}%", replication_rate * 100.0);

    let stats = network.get_network_stats().await?;
    info!("Network stats: {:?}", stats);
    
    network.stop_all().await?;
    
    info!("✅ Distributed file storage scenario passed");
    Ok(())
}

/// Test real-world P2P application scenario
#[tokio::test]
async fn test_p2p_application_scenario() -> Result<()> {
    let mut network = E2ENetwork::new(E2E_NETWORK_SIZE).await?;
    
    info!("Starting P2P application scenario");
    
    network.start_network().await?;
    network.wait_for_convergence().await?;

    // Simulate a distributed application workflow
    
    // Phase 1: User registration and profile storage
    let users = vec![
        ("alice", r#"{"name": "Alice", "role": "admin", "joined": "2024-01-01"}"#),
        ("bob", r#"{"name": "Bob", "role": "user", "joined": "2024-02-15"}"#),
        ("charlie", r#"{"name": "Charlie", "role": "moderator", "joined": "2024-03-10"}"#),
    ];

    let mut user_profiles = HashMap::new();
    
    for (username, profile_json) in &users {
        let profile_key = Key::new(format!("user:{}", username).as_bytes());
        let node_index = rand::random::<usize>() % network.nodes.len();
        let node = network.get_node(node_index);
        
        node.dht_put(profile_key.clone(), profile_json.as_bytes().to_vec()).await?;
        user_profiles.insert(profile_key, profile_json.as_bytes().to_vec());
        
        info!("Registered user '{}' via node {}", username, node_index);
        sleep(Duration::from_millis(50)).await;
    }

    // Phase 2: Content creation and sharing
    let content_items = vec![
        ("post:1", r#"{"author": "alice", "title": "Welcome!", "content": "Hello P2P network!"}"#),
        ("post:2", r#"{"author": "bob", "title": "Technical Update", "content": "New features coming soon"}"#),
        ("comment:1:1", r#"{"author": "charlie", "post": "post:1", "content": "Great to be here!"}"#),
    ];

    let mut content_data = HashMap::new();
    
    for (content_id, content_json) in &content_items {
        let content_key = Key::new(content_id.as_bytes());
        let node_index = rand::random::<usize>() % network.nodes.len();
        let node = network.get_node(node_index);
        
        node.dht_put(content_key.clone(), content_json.as_bytes().to_vec()).await?;
        content_data.insert(content_key, content_json.as_bytes().to_vec());
        
        info!("Created content '{}' via node {}", content_id, node_index);
        sleep(Duration::from_millis(50)).await;
    }

    // Phase 3: Distributed query and data aggregation
    sleep(Duration::from_secs(3)).await;
    
    // Simulate user lookup from different nodes
    let alice_key = Key::new(b"user:alice");
    let mut alice_lookup_success = 0;
    
    for i in 0..network.nodes.len() {
        let node = network.get_node(i);
        if let Ok(Some(profile_data)) = node.dht_get(alice_key.clone()).await {
            if String::from_utf8_lossy(&profile_data).contains("Alice") {
                alice_lookup_success += 1;
                debug!("Node {} successfully looked up Alice's profile", i);
            }
        }
    }

    // Simulate content feed generation
    let post1_key = Key::new(b"post:1");
    let comment_key = Key::new(b"comment:1:1");
    
    let mut content_accessibility = 0;
    let total_content_checks = 2 * network.nodes.len();
    
    for i in 0..network.nodes.len() {
        let node = network.get_node(i);
        
        // Check post accessibility
        if let Ok(Some(_)) = node.dht_get(post1_key.clone()).await {
            content_accessibility += 1;
        }
        
        // Check comment accessibility
        if let Ok(Some(_)) = node.dht_get(comment_key.clone()).await {
            content_accessibility += 1;
        }
    }

    let content_accessibility_rate = content_accessibility as f64 / total_content_checks as f64;

    // Phase 4: Network resilience testing
    info!("Testing application resilience...");
    
    // Add new content while network is active
    let late_content_key = Key::new(b"post:3");
    let late_content = br#"{"author": "bob", "title": "Late Post", "content": "Posted after network formation"}"#;
    
    let node = network.get_node(0);
    node.dht_put(late_content_key.clone(), late_content.to_vec()).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    // Verify late content is accessible
    let mut late_content_found = false;
    for i in 1..network.nodes.len() {
        let node = network.get_node(i);
        if let Ok(Some(content)) = node.dht_get(late_content_key.clone()).await {
            if content == late_content {
                late_content_found = true;
                break;
            }
        }
    }

    // Verify application scenario success
    assert!(alice_lookup_success >= network.nodes.len() / 2, 
           "User profile lookup failed on too many nodes");
    assert!(content_accessibility_rate >= 0.4, 
           "Content accessibility too low: {:.1}%", content_accessibility_rate * 100.0);
    assert!(late_content_found, "Late content not replicated properly");

    let stats = network.get_network_stats().await?;
    info!("Application scenario stats: {:?}", stats);
    
    info!("User profile lookups: {}/{}", alice_lookup_success, network.nodes.len());
    info!("Content accessibility: {:.1}%", content_accessibility_rate * 100.0);
    info!("Late content replication: {}", late_content_found);

    network.stop_all().await?;
    
    info!("✅ P2P application scenario passed");
    Ok(())
}

/// Test AI service coordination scenario
#[tokio::test]
async fn test_ai_service_coordination_scenario() -> Result<()> {
    let mut network = E2ENetwork::new(E2E_NETWORK_SIZE).await?;
    
    info!("Starting AI service coordination scenario");
    
    network.start_network().await?;
    network.wait_for_convergence().await?;

    // Simulate AI service discovery and capability advertisement
    let ai_services = vec![
        ("nlp-service", r#"{"type": "nlp", "capabilities": ["sentiment", "translation"], "load": 0.2}"#),
        ("vision-service", r#"{"type": "vision", "capabilities": ["object-detection", "ocr"], "load": 0.5}"#),
        ("ml-service", r#"{"type": "ml", "capabilities": ["classification", "regression"], "load": 0.1}"#),
    ];

    let mut service_registry = HashMap::new();
    
    // Register AI services across different nodes
    for (i, (service_name, service_info)) in ai_services.iter().enumerate() {
        let service_key = Key::new(format!("service:{}", service_name).as_bytes());
        let node_index = i % network.nodes.len();
        let node = network.get_node(node_index);
        
        node.dht_put(service_key.clone(), service_info.as_bytes().to_vec()).await?;
        service_registry.insert(service_key, service_info.as_bytes().to_vec());
        
        info!("Registered AI service '{}' on node {}", service_name, node_index);
        sleep(Duration::from_millis(100)).await;
    }

    // Simulate task distribution and coordination
    let ai_tasks = vec![
        ("task:sentiment:1", r#"{"type": "sentiment", "input": "I love this product!", "priority": "high"}"#),
        ("task:translate:1", r#"{"type": "translation", "input": "Hello world", "from": "en", "to": "es"}"#),
        ("task:detect:1", r#"{"type": "object-detection", "image_url": "http://example.com/image.jpg"}"#),
        ("task:classify:1", r#"{"type": "classification", "features": [1.2, 3.4, 5.6], "model": "default"}"#),
    ];

    let mut task_queue = HashMap::new();
    
    // Distribute tasks across the network
    for (task_id, task_data) in &ai_tasks {
        let task_key = Key::new(task_id.as_bytes());
        let node_index = rand::random::<usize>() % network.nodes.len();
        let node = network.get_node(node_index);
        
        node.dht_put(task_key.clone(), task_data.as_bytes().to_vec()).await?;
        task_queue.insert(task_key, task_data.as_bytes().to_vec());
        
        info!("Queued AI task '{}' on node {}", task_id, node_index);
        sleep(Duration::from_millis(50)).await;
    }

    // Wait for task distribution
    sleep(Duration::from_secs(3)).await;

    // Simulate service discovery by worker nodes
    let nlp_service_key = Key::new(b"service:nlp-service");
    let mut service_discovery_success = 0;
    
    for i in 0..network.nodes.len() {
        let node = network.get_node(i);
        if let Ok(Some(service_info)) = node.dht_get(nlp_service_key.clone()).await {
            if String::from_utf8_lossy(&service_info).contains("nlp") {
                service_discovery_success += 1;
                debug!("Node {} discovered NLP service", i);
            }
        }
    }

    // Simulate task execution results
    let task_results = vec![
        ("result:sentiment:1", r#"{"task_id": "task:sentiment:1", "result": "positive", "confidence": 0.95}"#),
        ("result:translate:1", r#"{"task_id": "task:translate:1", "result": "Hola mundo", "confidence": 0.99}"#),
    ];

    let mut result_storage = HashMap::new();
    
    for (result_id, result_data) in &task_results {
        let result_key = Key::new(result_id.as_bytes());
        let node_index = rand::random::<usize>() % network.nodes.len();
        let node = network.get_node(node_index);
        
        node.dht_put(result_key.clone(), result_data.as_bytes().to_vec()).await?;
        result_storage.insert(result_key, result_data.as_bytes().to_vec());
        
        info!("Stored AI result '{}' on node {}", result_id, node_index);
        sleep(Duration::from_millis(50)).await;
    }

    // Wait for result replication
    sleep(Duration::from_secs(2)).await;

    // Verify AI coordination scenario
    let service_replication = network.verify_data_replication(&service_registry).await?;
    let task_replication = network.verify_data_replication(&task_queue).await?;
    let result_replication = network.verify_data_replication(&result_storage).await?;

    // Test result retrieval from different nodes
    let sentiment_result_key = Key::new(b"result:sentiment:1");
    let mut result_retrieval_success = 0;
    
    for i in 0..network.nodes.len() {
        let node = network.get_node(i);
        if let Ok(Some(result_data)) = node.dht_get(sentiment_result_key.clone()).await {
            if String::from_utf8_lossy(&result_data).contains("positive") {
                result_retrieval_success += 1;
            }
        }
    }

    // Verify AI service coordination success
    assert!(service_discovery_success >= network.nodes.len() / 2, 
           "AI service discovery failed on too many nodes");
    assert!(service_replication >= 0.3, 
           "AI service replication too low: {:.1}%", service_replication * 100.0);
    assert!(task_replication >= 0.3, 
           "AI task replication too low: {:.1}%", task_replication * 100.0);
    assert!(result_replication >= 0.3, 
           "AI result replication too low: {:.1}%", result_replication * 100.0);
    assert!(result_retrieval_success >= network.nodes.len() / 2, 
           "AI result retrieval failed on too many nodes");

    let stats = network.get_network_stats().await?;
    info!("AI coordination stats: {:?}", stats);
    
    info!("Service discovery: {}/{}", service_discovery_success, network.nodes.len());
    info!("Service replication: {:.1}%", service_replication * 100.0);
    info!("Task replication: {:.1}%", task_replication * 100.0);
    info!("Result replication: {:.1}%", result_replication * 100.0);
    info!("Result retrieval: {}/{}", result_retrieval_success, network.nodes.len());

    network.stop_all().await?;
    
    info!("✅ AI service coordination scenario passed");
    Ok(())
}

/// Test network bootstrap and discovery scenario
#[tokio::test]
async fn test_network_bootstrap_discovery_scenario() -> Result<()> {
    info!("Starting network bootstrap and discovery scenario");
    
    // Phase 1: Start initial network seed nodes
    let mut seed_network = E2ENetwork::new(3).await?;
    seed_network.start_network().await?;
    seed_network.wait_for_convergence().await?;
    
    info!("Initial seed network established");
    
    // Populate seed network with bootstrap data
    let bootstrap_data = vec![
        ("node-list", r#"["seed1", "seed2", "seed3"]"#),
        ("network-info", r#"{"version": "1.0", "protocol": "p2p-foundation"}"#),
        ("capabilities", r#"{"dht": true, "mcp": true, "tunneling": true}"#),
    ];

    for (key_str, data) in &bootstrap_data {
        let key = Key::new(key_str.as_bytes());
        let node = seed_network.get_node(0);
        node.dht_put(key, data.as_bytes().to_vec()).await?;
        info!("Stored bootstrap data: {}", key_str);
    }

    sleep(Duration::from_secs(2)).await;

    // Phase 2: New nodes join and discover the network
    let mut new_nodes = Vec::new();
    let mut new_cache_dirs = Vec::new();
    
    for i in 0..3 {
        let cache_dir = TempDir::new()?;
        let cache_config = CacheConfig {
            cache_dir: cache_dir.path().to_path_buf(),
            max_contacts: 200,
            ..CacheConfig::default()
        };

        let listen_addr: SocketAddr = format!("127.0.0.1:{}", seed_network.base_port + 100 + i as u16).parse()?;
        
        let mut config = NodeConfig::default();
        config.listen_addr = listen_addr;
        config.bootstrap_cache_config = Some(cache_config);
        config.enable_mcp_server = true;
        
        // Bootstrap from seed network
        config.bootstrap_peers_str = vec![
            format!("127.0.0.1:{}", seed_network.base_port),
            format!("127.0.0.1:{}", seed_network.base_port + 1),
        ];

        let node = P2PNode::new(config).await?;
        node.start().await?;
        
        new_nodes.push(node);
        new_cache_dirs.push(cache_dir);
        
        info!("Started new node {} attempting to join network", i);
        sleep(Duration::from_millis(500)).await;
    }

    // Wait for new nodes to discover and join
    sleep(Duration::from_secs(8)).await;

    // Phase 3: Verify new nodes can access bootstrap data
    let mut successful_discoveries = 0;
    let bootstrap_key = Key::new(b"network-info");
    
    for (i, node) in new_nodes.iter().enumerate() {
        match timeout(Duration::from_secs(10), node.dht_get(bootstrap_key.clone())).await {
            Ok(Ok(Some(data))) => {
                if String::from_utf8_lossy(&data).contains("p2p-foundation") {
                    successful_discoveries += 1;
                    info!("New node {} successfully discovered bootstrap data", i);
                } else {
                    warn!("New node {} got incorrect bootstrap data", i);
                }
            }
            Ok(Ok(None)) => {
                warn!("New node {} could not find bootstrap data", i);
            }
            Ok(Err(e)) => {
                warn!("New node {} bootstrap discovery failed: {}", i, e);
            }
            Err(_) => {
                warn!("New node {} bootstrap discovery timed out", i);
            }
        }
    }

    // Phase 4: Verify network health after expansion
    let mut total_connections = 0;
    let all_nodes = seed_network.nodes.iter().chain(new_nodes.iter());
    
    for (i, node) in all_nodes.enumerate() {
        let peer_count = node.peer_count().await;
        total_connections += peer_count;
        debug!("Combined network node {} has {} peers", i, peer_count);
    }

    let total_nodes = seed_network.nodes.len() + new_nodes.len();
    let avg_connections = total_connections as f64 / total_nodes as f64;

    // Phase 5: Test data propagation across expanded network
    let test_key = Key::new(b"expansion-test");
    let test_data = b"Data added after network expansion";
    
    new_nodes[0].dht_put(test_key.clone(), test_data.to_vec()).await?;
    sleep(Duration::from_secs(3)).await;
    
    let mut propagation_success = 0;
    for node in seed_network.nodes.iter() {
        if let Ok(Some(data)) = node.dht_get(test_key.clone()).await {
            if data == test_data {
                propagation_success += 1;
            }
        }
    }

    // Cleanup new nodes
    for node in new_nodes.iter_mut() {
        let _ = node.stop().await;
    }

    seed_network.stop_all().await?;

    // Verify bootstrap and discovery scenario success
    assert!(successful_discoveries >= 2, 
           "Too few new nodes successfully discovered bootstrap data: {}/3", successful_discoveries);
    assert!(avg_connections >= 1.5, 
           "Average connections too low after network expansion: {:.1}", avg_connections);
    assert!(propagation_success >= 1, 
           "Data propagation failed from new node to seed network");

    info!("Bootstrap discovery: {}/3 new nodes successful", successful_discoveries);
    info!("Network connections: {:.1} average per node", avg_connections);
    info!("Data propagation: {}/{} seed nodes received new data", propagation_success, seed_network.nodes.len());

    info!("✅ Network bootstrap and discovery scenario passed");
    Ok(())
}

/// Test multi-application coexistence scenario
#[tokio::test]
async fn test_multi_application_coexistence_scenario() -> Result<()> {
    let mut network = E2ENetwork::new(E2E_NETWORK_SIZE).await?;
    
    info!("Starting multi-application coexistence scenario");
    
    network.start_network().await?;
    network.wait_for_convergence().await?;

    // Simulate multiple applications using the same P2P network
    
    // Application 1: Chat system
    let chat_data = vec![
        ("chat:room:general", r#"{"name": "General", "participants": 15, "created": "2024-01-01"}"#),
        ("chat:msg:1", r#"{"room": "general", "author": "alice", "text": "Hello everyone!", "timestamp": "2024-01-15T10:00:00Z"}"#),
        ("chat:msg:2", r#"{"room": "general", "author": "bob", "text": "Hi Alice!", "timestamp": "2024-01-15T10:01:00Z"}"#),
    ];

    // Application 2: File sharing system
    let file_data = vec![
        ("files:index", r#"["doc1.pdf", "image.jpg", "data.csv"]"#),
        ("files:doc1.pdf", "PDF_BINARY_CONTENT_PLACEHOLDER_2048_BYTES"),
        ("files:metadata:doc1.pdf", r#"{"size": 2048, "type": "pdf", "uploaded": "2024-01-10", "owner": "alice"}"#),
    ];

    // Application 3: IoT sensor network
    let iot_data = vec![
        ("iot:sensor:temp1", r#"{"value": 23.5, "unit": "celsius", "timestamp": "2024-01-15T10:05:00Z", "location": "room1"}"#),
        ("iot:sensor:humidity1", r#"{"value": 65.2, "unit": "percent", "timestamp": "2024-01-15T10:05:00Z", "location": "room1"}"#),
        ("iot:config:alerts", r#"{"temp_threshold": 25.0, "humidity_threshold": 70.0, "notifications": true}"#),
    ];

    let mut all_app_data = HashMap::new();

    // Store data from all applications concurrently
    let store_tasks = vec![
        ("Chat", chat_data),
        ("FileShare", file_data),
        ("IoT", iot_data),
    ];

    for (app_name, app_data) in store_tasks {
        for (i, (key_str, data)) in app_data.iter().enumerate() {
            let key = Key::new(key_str.as_bytes());
            let node_index = (i + app_name.len()) % network.nodes.len(); // Distribute across nodes
            let node = network.get_node(node_index);
            
            let data_vec = data.as_bytes().to_vec();
            
            match timeout(Duration::from_secs(10), node.dht_put(key.clone(), data_vec.clone())).await {
                Ok(Ok(_)) => {
                    all_app_data.insert(key, data_vec);
                    debug!("Stored {} data '{}' via node {}", app_name, key_str, node_index);
                }
                Ok(Err(e)) => {
                    warn!("Failed to store {} data '{}': {}", app_name, key_str, e);
                }
                Err(_) => {
                    warn!("Timeout storing {} data '{}'", app_name, key_str);
                }
            }
            
            sleep(Duration::from_millis(25)).await;
        }
        info!("Completed storing {} application data", app_name);
    }

    // Wait for cross-application data replication
    sleep(Duration::from_secs(5)).await;

    // Test cross-application data access
    let chat_room_key = Key::new(b"chat:room:general");
    let file_index_key = Key::new(b"files:index");
    let iot_config_key = Key::new(b"iot:config:alerts");

    let test_keys = vec![
        ("Chat room", chat_room_key),
        ("File index", file_index_key),  
        ("IoT config", iot_config_key),
    ];

    let mut cross_app_access_success = 0;
    let total_cross_tests = test_keys.len() * network.nodes.len();

    for (app_name, key) in &test_keys {
        for (i, node) in network.nodes.iter().enumerate() {
            if let Ok(Some(_)) = timeout(Duration::from_secs(5), node.dht_get(key.clone())).await? {
                cross_app_access_success += 1;
                debug!("Node {} can access {} data", i, app_name);
            }
        }
    }

    // Test application isolation and namespace verification
    let mut app_namespaces = HashMap::new();
    app_namespaces.insert("chat:", 0);
    app_namespaces.insert("files:", 0);
    app_namespaces.insert("iot:", 0);

    for key in all_app_data.keys() {
        let key_str = String::from_utf8_lossy(key.as_bytes());
        for (namespace, count) in app_namespaces.iter_mut() {
            if key_str.starts_with(namespace) {
                *count += 1;
                break;
            }
        }
    }

    // Test concurrent application operations
    let concurrent_operations = vec![
        ("chat:msg:3", r#"{"room": "general", "author": "charlie", "text": "How is everyone?", "timestamp": "2024-01-15T10:02:00Z"}"#),
        ("files:image.jpg", "IMAGE_BINARY_CONTENT_PLACEHOLDER_1024_BYTES"),
        ("iot:sensor:temp2", r#"{"value": 24.1, "unit": "celsius", "timestamp": "2024-01-15T10:06:00Z", "location": "room2"}"#),
    ];

    let concurrent_tasks: Vec<_> = concurrent_operations.iter().enumerate().map(|(i, (key_str, data))| {
        let key = Key::new(key_str.as_bytes());
        let node_index = i % network.nodes.len();
        let node = network.get_node(node_index);
        let data_vec = data.as_bytes().to_vec();
        
        async move {
            timeout(Duration::from_secs(10), node.dht_put(key, data_vec)).await
        }
    }).collect();

    let concurrent_results = join_all(concurrent_tasks).await;
    let concurrent_success = concurrent_results.iter().filter(|r| matches!(r, Ok(Ok(_)))).count();

    let cross_app_access_rate = cross_app_access_success as f64 / total_cross_tests as f64;

    // Verify multi-application coexistence
    assert!(cross_app_access_rate >= 0.4, 
           "Cross-application data access rate too low: {:.1}%", cross_app_access_rate * 100.0);
    assert!(app_namespaces.values().all(|&count| count > 0), 
           "Not all application namespaces have data");
    assert!(concurrent_success >= 2, 
           "Concurrent application operations failed: {}/3", concurrent_success);

    let stats = network.get_network_stats().await?;
    info!("Multi-application stats: {:?}", stats);
    
    info!("Application namespaces: {:?}", app_namespaces);
    info!("Cross-application access: {:.1}%", cross_app_access_rate * 100.0);
    info!("Concurrent operations: {}/3 successful", concurrent_success);

    network.stop_all().await?;
    
    info!("✅ Multi-application coexistence scenario passed");
    Ok(())
}