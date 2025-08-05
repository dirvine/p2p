//\! End-to-End Scenario Integration Tests
//\! 
//\! Tests complete user workflows, cross-component interactions,
//\! and performance benchmarks for real-world usage scenarios.

use tokio::time::{sleep, Duration, timeout};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use anyhow::{Result, Context};

use saorsa_core::{
    Config, Node, Identity, StorageKey, StorageValue,
    network::NetworkEvent,
    ApplicationMessage, UserProfile, ChatMessage,
};

/// Complete application scenario test framework
struct EndToEndTestFramework {
    users: Vec<TestUser>,
    network_events: Arc<RwLock<Vec<NetworkEvent>>>,
    message_log: Arc<RwLock<Vec<(String, String, String)>>>, // (sender, receiver, message)
}

#[derive(Clone)]
struct TestUser {
    node: Arc<Node>,
    identity: Identity,
    profile: UserProfile,
    username: String,
    contacts: Arc<RwLock<HashSet<String>>>,
    received_messages: Arc<RwLock<Vec<ChatMessage>>>,
}

impl TestUser {
    async fn new(username: String, port: u16) -> Result<Self> {
        let mut config = Config::default();
        config.network.listen_port = port;
        config.network.enable_mdns = true;
        config.security.enable_encryption = true;
        config.storage.replication_factor = 2;
        
        let identity = Identity::generate()?;
        let profile = UserProfile {
            username: username.clone(),
            display_name: format\!("Test User {}", username),
            avatar_hash: None,
            public_key: identity.public_key().clone(),
            created_at: chrono::Utc::now(),
        };
        
        let node = Node::new_with_identity(config, identity.clone()).await
            .context(format\!("Failed to create user node for {}", username))?;
        
        Ok(Self {
            node: Arc::new(node),
            identity,
            profile,
            username,
            contacts: Arc::new(RwLock::new(HashSet::new())),
            received_messages: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    async fn start(&self) -> Result<()> {
        self.node.start().await?;
        
        // Register profile in DHT
        let profile_key = StorageKey::from_str(&format\!("user_profile_{}", self.username))?;
        let profile_data = serde_json::to_vec(&self.profile)?;
        let profile_value = StorageValue::from_bytes(profile_data)?;
        
        self.node.store(profile_key, profile_value).await?;
        
        sleep(Duration::from_millis(500)).await;
        Ok(())
    }
    
    async fn connect_to_user(&self, other_user: &TestUser) -> Result<()> {
        let peer_addr = format\!("/ip4/127.0.0.1/tcp/{}", 
                               other_user.node.get_listen_port().await?);
        
        self.node.connect_to_peer_secure(&peer_addr, &other_user.identity).await?;
        
        // Add to contacts
        self.contacts.write().await.insert(other_user.username.clone());
        
        sleep(Duration::from_millis(200)).await;
        Ok(())
    }
    
    async fn send_message(&self, recipient: &str, content: &str) -> Result<()> {
        let message = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: self.username.clone(),
            recipient: recipient.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            encrypted: true,
        };
        
        self.node.send_application_message(recipient, &message).await?;
        Ok(())
    }
    
    async fn discover_user(&self, username: &str) -> Result<Option<UserProfile>> {
        let profile_key = StorageKey::from_str(&format\!("user_profile_{}", username))?;
        
        if let Some(profile_data) = self.node.retrieve(&profile_key).await? {
            let profile: UserProfile = serde_json::from_slice(profile_data.as_bytes())?;
            Ok(Some(profile))
        } else {
            Ok(None)
        }
    }
    
    async fn share_file(&self, recipient: &str, filename: &str, data: Vec<u8>) -> Result<String> {
        // Store file in DHT
        let file_hash = blake3::hash(&data).to_string();
        let file_key = StorageKey::from_str(&format\!("file_{}", file_hash))?;
        let file_value = StorageValue::from_bytes(data)?;
        
        self.node.store(file_key, file_value).await?;
        
        // Send file notification
        let file_message = format\!("FILE_SHARE:{}:{}", filename, file_hash);
        self.send_message(recipient, &file_message).await?;
        
        Ok(file_hash)
    }
    
    async fn download_file(&self, file_hash: &str) -> Result<Option<Vec<u8>>> {
        let file_key = StorageKey::from_str(&format\!("file_{}", file_hash))?;
        
        if let Some(file_data) = self.node.retrieve(&file_key).await? {
            Ok(Some(file_data.into_bytes()))
        } else {
            Ok(None)
        }
    }
    
    async fn get_network_stats(&self) -> Result<(usize, usize, u64)> {
        let peers = self.node.get_connected_peers().await?.len();
        let storage_info = self.node.get_storage_info().await?;
        Ok((peers, storage_info.record_count, storage_info.total_size))
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.node.shutdown().await
    }
}

impl EndToEndTestFramework {
    async fn new(user_count: usize) -> Result<Self> {
        let mut users = Vec::new();
        
        for i in 0..user_count {
            let username = format\!("user_{}", i + 1);
            let port = 5000 + i as u16;
            let user = TestUser::new(username, port).await?;
            users.push(user);
        }
        
        Ok(Self {
            users,
            network_events: Arc::new(RwLock::new(Vec::new())),
            message_log: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    async fn start_all_users(&self) -> Result<()> {
        for user in &self.users {
            user.start().await?;
        }
        
        // Wait for all users to be ready
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }
    
    async fn create_social_network(&self) -> Result<()> {
        // Create connections between users (small world network)
        for i in 0..self.users.len() {
            // Connect to next user (ring)
            let next_idx = (i + 1) % self.users.len();
            self.users[i].connect_to_user(&self.users[next_idx]).await?;
            
            // Connect to user two positions ahead (small world shortcuts)
            if self.users.len() > 3 {
                let shortcut_idx = (i + 2) % self.users.len();
                self.users[i].connect_to_user(&self.users[shortcut_idx]).await?;
            }
        }
        
        sleep(Duration::from_secs(3)).await;
        Ok(())
    }
    
    async fn simulate_chat_session(&self) -> Result<usize> {
        let mut message_count = 0;
        
        // Simulate various conversation patterns
        for round in 0..5 {
            for i in 0..self.users.len() {
                let sender = &self.users[i];
                let recipient_idx = (i + 1) % self.users.len();
                let recipient = &self.users[recipient_idx];
                
                let message = format\!("Hello from {} - message {}", sender.username, round);
                sender.send_message(&recipient.username, &message).await?;
                
                message_count += 1;
                
                // Log message
                self.message_log.write().await.push((
                    sender.username.clone(),
                    recipient.username.clone(),
                    message,
                ));
                
                sleep(Duration::from_millis(100)).await;
            }
        }
        
        // Wait for message delivery
        sleep(Duration::from_secs(2)).await;
        Ok(message_count)
    }
    
    async fn simulate_file_sharing(&self) -> Result<Vec<String>> {
        let mut shared_files = Vec::new();
        
        // Create test files of various sizes
        let test_files = vec\![
            ("small.txt", vec\![b'A'; 1024]),         // 1KB
            ("medium.jpg", vec\![b'B'; 100 * 1024]), // 100KB
            ("large.video", vec\![b'C'; 1024 * 1024]), // 1MB
        ];
        
        for (filename, data) in test_files {
            let sender = &self.users[0];
            let recipient = &self.users[1];
            
            let file_hash = sender.share_file(&recipient.username, filename, data.clone()).await?;
            shared_files.push(file_hash.clone());
            
            sleep(Duration::from_millis(500)).await;
            
            // Recipient downloads the file
            let downloaded = recipient.download_file(&file_hash).await?;
            assert_eq\!(downloaded, Some(data), "File {} should be downloaded correctly", filename);
        }
        
        Ok(shared_files)
    }
    
    async fn simulate_user_discovery(&self) -> Result<usize> {
        let mut discoveries = 0;
        
        // Each user tries to discover all other users
        for i in 0..self.users.len() {
            let searcher = &self.users[i];
            
            for j in 0..self.users.len() {
                if i \!= j {
                    let target_username = &self.users[j].username;
                    
                    if let Some(profile) = searcher.discover_user(target_username).await? {
                        assert_eq\!(profile.username, *target_username);
                        discoveries += 1;
                    }
                }
            }
        }
        
        Ok(discoveries)
    }
    
    async fn stress_test_network(&self, duration_secs: u64) -> Result<HashMap<String, u64>> {
        let mut metrics = HashMap::new();
        let start_time = std::time::Instant::now();
        
        let stress_tasks = self.users.iter().enumerate().map(|(i, user)| {
            let user = user.clone();
            let message_log = self.message_log.clone();
            
            async move {
                let mut local_message_count = 0u64;
                
                while start_time.elapsed().as_secs() < duration_secs {
                    // Send messages to random users
                    let recipient_idx = (i + 1 + (local_message_count as usize)) % self.users.len();
                    let recipient_name = format\!("user_{}", recipient_idx + 1);
                    
                    let message = format\!("Stress test message {} from {}", 
                                        local_message_count, user.username);
                    
                    if user.send_message(&recipient_name, &message).await.is_ok() {
                        local_message_count += 1;
                        
                        message_log.write().await.push((
                            user.username.clone(),
                            recipient_name,
                            message,
                        ));
                    }
                    
                    sleep(Duration::from_millis(50)).await;
                }
                
                (user.username.clone(), local_message_count)
            }
        });
        
        let results = futures::future::join_all(stress_tasks).await;
        
        for (username, count) in results {
            metrics.insert(username, count);
        }
        
        Ok(metrics)
    }
    
    async fn measure_network_performance(&self) -> Result<(f64, f64, f64)> {
        // Measure message latency
        let start = std::time::Instant::now();
        
        let sender = &self.users[0];
        let recipient = &self.users[1];
        
        // Send test message and measure round trip
        sender.send_message(&recipient.username, "PING").await?;
        
        // Wait for response (simulated)
        sleep(Duration::from_millis(100)).await;
        
        let latency_ms = start.elapsed().as_millis() as f64;
        
        // Measure throughput
        let throughput_start = std::time::Instant::now();
        let message_count = 50;
        
        for i in 0..message_count {
            let message = format\!("throughput_test_{}", i);
            sender.send_message(&recipient.username, &message).await?;
        }
        
        let throughput_duration = throughput_start.elapsed();
        let messages_per_second = message_count as f64 / throughput_duration.as_secs_f64();
        
        // Measure storage performance
        let storage_start = std::time::Instant::now();
        let storage_ops = 20;
        
        for i in 0..storage_ops {
            let key = StorageKey::from_str(&format\!("perf_test_{}", i))?;
            let value = StorageValue::from_bytes(vec\![i as u8; 1024])?; // 1KB each
            sender.node.store(key, value).await?;
        }
        
        let storage_duration = storage_start.elapsed();
        let storage_ops_per_second = storage_ops as f64 / storage_duration.as_secs_f64();
        
        Ok((latency_ms, messages_per_second, storage_ops_per_second))
    }
    
    async fn get_overall_stats(&self) -> Result<HashMap<String, (usize, usize, u64)>> {
        let mut stats = HashMap::new();
        
        for user in &self.users {
            let user_stats = user.get_network_stats().await?;
            stats.insert(user.username.clone(), user_stats);
        }
        
        Ok(stats)
    }
    
    async fn shutdown_all(&self) -> Result<()> {
        for user in &self.users {
            let _ = user.shutdown().await;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_complete_social_network_scenario() -> Result<()> {
    let framework = EndToEndTestFramework::new(5).await?;
    
    // Start all users
    framework.start_all_users().await?;
    
    // Create social network connections
    framework.create_social_network().await?;
    
    // Verify network connectivity
    let initial_stats = framework.get_overall_stats().await?;
    for (username, (peer_count, _, _)) in &initial_stats {
        assert\!(peer_count >= &1, "User {} should have at least 1 connection", username);
    }
    
    // Simulate chat session
    let message_count = framework.simulate_chat_session().await?;
    assert\!(message_count > 0, "Should have sent messages");
    
    // Test user discovery
    let discoveries = framework.simulate_user_discovery().await?;
    assert\!(discoveries > 0, "Should have discovered users");
    
    // Verify final network state
    let final_stats = framework.get_overall_stats().await?;
    for (username, (peer_count, record_count, storage_size)) in &final_stats {
        println\!("User {}: {} peers, {} records, {} bytes", 
                username, peer_count, record_count, storage_size);
        assert\!(peer_count >= &1, "User should maintain connections");
    }
    
    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_file_sharing_workflow() -> Result<()> {
    let framework = EndToEndTestFramework::new(3).await?;
    
    framework.start_all_users().await?;
    framework.create_social_network().await?;
    
    // Test file sharing between users
    let shared_files = framework.simulate_file_sharing().await?;
    assert_eq\!(shared_files.len(), 3, "Should have shared 3 files");
    
    // Verify files are accessible from other users too
    let downloader = &framework.users[2];
    for file_hash in &shared_files {
        let downloaded = downloader.download_file(file_hash).await?;
        assert\!(downloaded.is_some(), "File {} should be downloadable by third user", file_hash);
    }
    
    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_network_performance_benchmarks() -> Result<()> {
    let framework = EndToEndTestFramework::new(4).await?;
    
    framework.start_all_users().await?;
    framework.create_social_network().await?;
    
    // Measure performance metrics
    let (latency_ms, msg_per_sec, storage_ops_per_sec) = framework.measure_network_performance().await?;
    
    println\!("Performance Results:");
    println\!("  Latency: {:.2}ms", latency_ms);
    println\!("  Message Throughput: {:.2} msg/sec", msg_per_sec);
    println\!("  Storage Throughput: {:.2} ops/sec", storage_ops_per_sec);
    
    // Performance assertions (adjust based on requirements)
    assert\!(latency_ms < 1000.0, "Latency should be < 1000ms");
    assert\!(msg_per_sec > 10.0, "Should handle > 10 messages/second");
    assert\!(storage_ops_per_sec > 5.0, "Should handle > 5 storage ops/second");
    
    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_high_load_stress_scenario() -> Result<()> {
    let framework = EndToEndTestFramework::new(6).await?;
    
    framework.start_all_users().await?;
    framework.create_social_network().await?;
    
    // Run stress test for 10 seconds
    let stress_metrics = framework.stress_test_network(10).await?;
    
    let total_messages: u64 = stress_metrics.values().sum();
    println\!("Stress test results: {} total messages", total_messages);
    
    // Verify all users participated
    assert_eq\!(stress_metrics.len(), framework.users.len(), 
              "All users should participate in stress test");
    
    // Verify reasonable message counts
    for (username, count) in &stress_metrics {
        assert\!(count > &0, "User {} should send messages during stress test", username);
        println\!("  {}: {} messages", username, count);
    }
    
    // Total should indicate good throughput
    assert\!(total_messages > 100, "Should handle significant message load");
    
    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_network_resilience_scenario() -> Result<()> {
    let framework = EndToEndTestFramework::new(6).await?;
    
    framework.start_all_users().await?;
    framework.create_social_network().await?;
    
    // Get initial connectivity
    let initial_stats = framework.get_overall_stats().await?;
    let initial_total_connections: usize = initial_stats.values().map(|(peers, _, _)| peers).sum();
    
    // Simulate user leaving (shutdown user 2)
    framework.users[2].shutdown().await?;
    sleep(Duration::from_secs(3)).await;
    
    // Test network adaptation
    let mut remaining_stats = HashMap::new();
    for (i, user) in framework.users.iter().enumerate() {
        if i \!= 2 { // Skip shut down user
            let stats = user.get_network_stats().await?;
            remaining_stats.insert(user.username.clone(), stats);
        }
    }
    
    // Network should adapt to loss
    let remaining_connections: usize = remaining_stats.values().map(|(peers, _, _)| peers).sum();
    println\!("Connections: {} -> {}", initial_total_connections, remaining_connections);
    
    // Should maintain some connectivity
    assert\!(remaining_connections > 0, "Network should maintain some connectivity");
    
    // Test continued functionality
    let test_message_count = framework.simulate_chat_session().await?;
    assert\!(test_message_count > 0, "Network should continue functioning after node loss");
    
    // Simulate user rejoining
    let rejoining_user = TestUser::new("user_3".to_string(), 5002).await?;
    rejoining_user.start().await?;
    
    // Reconnect to network
    rejoining_user.connect_to_user(&framework.users[0]).await?;
    rejoining_user.connect_to_user(&framework.users[1]).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    // Verify rejoin successful
    let rejoin_stats = rejoining_user.get_network_stats().await?;
    assert\!(rejoin_stats.0 >= 2, "Rejoining user should reconnect");
    
    rejoining_user.shutdown().await?;
    framework.shutdown_all().await?;
    Ok(())
}

#[tokio::test]
async fn test_real_world_usage_simulation() -> Result<()> {
    let framework = EndToEndTestFramework::new(8).await?;
    
    framework.start_all_users().await?;
    framework.create_social_network().await?;
    
    // Simulate real-world usage patterns over time
    let simulation_start = std::time::Instant::now();
    
    // Phase 1: User discovery and initial connections (0-30s)
    let discoveries = timeout(Duration::from_secs(5), 
                             framework.simulate_user_discovery()).await??;
    assert\!(discoveries > 0, "User discovery should work");
    
    // Phase 2: Active messaging period (30s-60s)
    let chat_messages = timeout(Duration::from_secs(10),
                               framework.simulate_chat_session()).await??;
    assert\!(chat_messages > 0, "Chat messaging should work");
    
    // Phase 3: File sharing activity (60s-90s)
    let shared_files = timeout(Duration::from_secs(10),
                              framework.simulate_file_sharing()).await??;
    assert\!(\!shared_files.is_empty(), "File sharing should work");
    
    // Phase 4: Mixed activity under load (90s-120s)
    let stress_results = timeout(Duration::from_secs(15),
                                framework.stress_test_network(10)).await??;
    let total_stress_messages: u64 = stress_results.values().sum();
    assert\!(total_stress_messages > 50, "Should handle stress testing");
    
    let total_simulation_time = simulation_start.elapsed();
    println\!("Complete simulation completed in {:?}", total_simulation_time);
    
    // Final verification
    let final_stats = framework.get_overall_stats().await?;
    let total_final_connections: usize = final_stats.values().map(|(peers, _, _)| peers).sum();
    let total_storage_records: usize = final_stats.values().map(|(_, records, _)| records).sum();
    let total_storage_size: u64 = final_stats.values().map(|(_, _, size)| size).sum();
    
    println\!("Final network state:");
    println\!("  Total connections: {}", total_final_connections);
    println\!("  Total storage records: {}", total_storage_records);
    println\!("  Total storage size: {} bytes", total_storage_size);
    
    // Network should be healthy after complete simulation
    assert\!(total_final_connections > 0, "Network should maintain connectivity");
    assert\!(total_storage_records > 0, "Should have stored data");
    assert\!(total_storage_size > 0, "Storage should be utilized");
    
    framework.shutdown_all().await?;
    Ok(())
}
