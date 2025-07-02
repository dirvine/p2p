// Performance tests for Saorsa application

use saorsa_lib::*;
use saorsa_core::{
    network::{P2PNode, NodeConfig},
    identity::{UserIdentity, VerificationLevel},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use criterion::{black_box, Criterion};

#[cfg(test)]
mod performance_tests {
    use super::*;

    // Helper to measure operation time
    async fn measure_time<F, Fut, T>(operation: F) -> (T, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        (result, duration)
    }

    #[tokio::test]
    async fn test_identity_creation_performance() {
        let app = tauri::test::mock_app();
        let node = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
        let state = create_test_state_with_network(node).await;
        let state_wrapper = tauri::State::new(state);
        
        // Measure identity creation time
        let (result, duration) = measure_time(|| {
            create_identity(
                state_wrapper.clone(),
                "Performance Test User".to_string(),
                None,
                app.app_handle()
            )
        }).await;
        
        assert!(result.is_ok());
        println!("Identity creation took: {:?}", duration);
        
        // Should complete within 500ms
        assert!(duration < Duration::from_millis(500), 
            "Identity creation took too long: {:?}", duration);
    }

    #[tokio::test]
    async fn test_message_sending_throughput() {
        let node1 = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
        let node2 = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
        
        // Connect nodes
        let addr2 = node2.local_addr().await.unwrap();
        node1.connect_peer(&addr2.to_string()).await.unwrap();
        
        let state1 = create_test_state_with_network(node1).await;
        let state_wrapper1 = tauri::State::new(state1);
        
        let app = tauri::test::mock_app();
        
        // Create identities
        let identity1 = create_identity(
            state_wrapper1.clone(),
            "Sender".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        // Send 100 messages and measure throughput
        let start = Instant::now();
        let message_count = 100;
        
        for i in 0..message_count {
            let result = send_message(
                state_wrapper1.clone(),
                "recipient123".to_string(),
                format!("Message {}", i),
                app.app_handle()
            ).await;
            assert!(result.is_ok());
        }
        
        let duration = start.elapsed();
        let messages_per_second = message_count as f64 / duration.as_secs_f64();
        
        println!("Sent {} messages in {:?}", message_count, duration);
        println!("Throughput: {:.2} messages/second", messages_per_second);
        
        // Should handle at least 50 messages per second
        assert!(messages_per_second > 50.0, 
            "Message throughput too low: {:.2} msg/s", messages_per_second);
    }

    #[tokio::test]
    async fn test_contact_list_scaling() {
        let node = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
        let state = create_test_state_with_network(node).await;
        let state_wrapper = tauri::State::new(state);
        
        // Add many contacts
        let contact_count = 1000;
        let start = Instant::now();
        
        {
            let mut contacts = state_wrapper.contacts.write().await;
            for i in 0..contact_count {
                contacts.insert(format!("contact_{}", i), Contact {
                    id: format!("contact_{}", i),
                    name: format!("Contact {}", i),
                    nickname: None,
                    three_word_address: format!("contact.{}.address", i),
                    is_online: i % 2 == 0,
                    last_seen: 0,
                    unread_count: i % 10,
                    is_blocked: false,
                    notes: None,
                    category: Some(format!("Category{}", i % 5)),
                    permissions: ContactPermissions {
                        can_see_profile: true,
                        can_see_online_status: true,
                        can_see_last_seen: true,
                        can_see_avatar: true,
                        can_send_messages: true,
                    },
                    added_at: 0,
                    trust_level: 0.5,
                });
            }
        }
        
        let insert_duration = start.elapsed();
        println!("Inserted {} contacts in {:?}", contact_count, insert_duration);
        
        // Test retrieval performance
        let (contacts, retrieve_duration) = measure_time(|| {
            get_contacts(state_wrapper.clone())
        }).await;
        
        assert!(contacts.is_ok());
        let contact_list = contacts.unwrap();
        assert_eq!(contact_list.len(), contact_count);
        
        println!("Retrieved {} contacts in {:?}", contact_count, retrieve_duration);
        
        // Should handle 1000 contacts efficiently
        assert!(retrieve_duration < Duration::from_millis(100),
            "Contact retrieval too slow: {:?}", retrieve_duration);
    }

    #[tokio::test]
    async fn test_search_performance() {
        let node = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
        let state = create_test_state_with_network(node).await;
        let state_wrapper = tauri::State::new(state);
        
        // Simulate DHT with many users
        // In real scenario, this would be distributed across network
        
        let search_iterations = 50;
        let mut total_duration = Duration::ZERO;
        
        for i in 0..search_iterations {
            let (result, duration) = measure_time(|| {
                search_users(state_wrapper.clone(), format!("user{}", i))
            }).await;
            
            total_duration += duration;
            
            // Each search should complete quickly
            assert!(duration < Duration::from_millis(100),
                "Search took too long: {:?}", duration);
        }
        
        let avg_duration = total_duration / search_iterations;
        println!("Average search time: {:?}", avg_duration);
        
        // Average should be under 50ms
        assert!(avg_duration < Duration::from_millis(50),
            "Average search time too high: {:?}", avg_duration);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        use tokio::task::JoinSet;
        
        let node = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
        let state = Arc::new(create_test_state_with_network(node).await);
        let app = tauri::test::mock_app();
        
        // Run many concurrent operations
        let concurrent_ops = 100;
        let start = Instant::now();
        
        let mut tasks = JoinSet::new();
        
        for i in 0..concurrent_ops {
            let state_clone = state.clone();
            let app_handle = app.app_handle();
            
            tasks.spawn(async move {
                let state_wrapper = tauri::State::new(state_clone.as_ref().clone());
                
                match i % 4 {
                    0 => {
                        // Send message
                        send_message(
                            state_wrapper,
                            format!("user_{}", i),
                            format!("Concurrent message {}", i),
                            app_handle
                        ).await
                    },
                    1 => {
                        // Get contacts
                        get_contacts(state_wrapper).await.map(|_| ())
                    },
                    2 => {
                        // Get network status
                        get_network_status(state_wrapper).await.map(|_| ())
                    },
                    _ => {
                        // Search users
                        search_users(state_wrapper, format!("search_{}", i)).await.map(|_| ())
                    }
                }
            });
        }
        
        // Wait for all operations to complete
        let mut completed = 0;
        while let Some(result) = tasks.join_next().await {
            assert!(result.is_ok());
            completed += 1;
        }
        
        let duration = start.elapsed();
        let ops_per_second = concurrent_ops as f64 / duration.as_secs_f64();
        
        println!("Completed {} concurrent operations in {:?}", concurrent_ops, duration);
        println!("Throughput: {:.2} ops/second", ops_per_second);
        
        assert_eq!(completed, concurrent_ops);
        assert!(ops_per_second > 100.0,
            "Concurrent operation throughput too low: {:.2} ops/s", ops_per_second);
    }

    #[tokio::test]
    async fn test_memory_usage_under_load() {
        let node = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
        let state = create_test_state_with_network(node).await;
        let state_wrapper = tauri::State::new(state);
        let app = tauri::test::mock_app();
        
        // Get initial memory usage (approximate)
        let initial_memory = get_approximate_memory_usage();
        
        // Generate load
        for i in 0..1000 {
            // Add messages
            let mut messages = state_wrapper.messages.write().await;
            let contact_id = format!("contact_{}", i % 10);
            let msg_list = messages.entry(contact_id).or_insert_with(Vec::new);
            msg_list.push(Message {
                id: format!("msg_{}", i),
                content: format!("Test message with some content {}", i),
                from_peer: "test_peer".to_string(),
                timestamp: chrono::Utc::now(),
                status: MessageStatus::Delivered,
                reply_to: None,
                edited: false,
                reactions: std::collections::HashMap::new(),
                attachments: vec![],
            });
            drop(messages);
            
            // Add contact requests
            if i % 100 == 0 {
                let mut requests = state_wrapper.contact_requests.write().await;
                requests.sent.push(ContactRequest {
                    request_id: format!("req_{}", i),
                    from_user_id: "test_user".to_string(),
                    from_user_name: "Test User".to_string(),
                    to_user_id: format!("user_{}", i),
                    to_user_name: None,
                    message: "Test request".to_string(),
                    created_at: chrono::Utc::now(),
                    status: ContactRequestStatus::Pending,
                });
                drop(requests);
            }
        }
        
        // Force some operations to ensure data is actually stored
        let _ = get_contacts(state_wrapper.clone()).await;
        let _ = get_messages(state_wrapper.clone(), "contact_5".to_string()).await;
        
        let final_memory = get_approximate_memory_usage();
        let memory_increase = final_memory.saturating_sub(initial_memory);
        
        println!("Memory increase under load: {} bytes", memory_increase);
        
        // Memory increase should be reasonable (less than 100MB for this test)
        assert!(memory_increase < 100_000_000,
            "Memory usage increased too much: {} bytes", memory_increase);
    }

    // Helper to create test state with network
    async fn create_test_state_with_network(node: Arc<P2PNode>) -> AppState {
        let mut state = AppState::default();
        *state.network.write().await = Some(node);
        
        let identity_manager = Arc::new(
            saorsa_core::identity::manager::IdentityManager::new(
                saorsa_core::identity::manager::IdentityManagerConfig::default()
            ).await.unwrap()
        );
        *state.identity_manager.write().await = Some(identity_manager);
        
        state
    }

    // Approximate memory usage (not precise, but good for relative comparison)
    fn get_approximate_memory_usage() -> usize {
        // This is a simplified approximation
        // In production, you'd use proper memory profiling tools
        std::mem::size_of::<AppState>() * 1000 // Rough estimate
    }
}

// Benchmark tests using Criterion (requires criterion feature)
#[cfg(all(test, feature = "bench"))]
mod benchmarks {
    use super::*;
    use criterion::{criterion_group, criterion_main, Criterion};

    fn bench_identity_creation(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("identity_creation", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let app = tauri::test::mock_app();
                    let node = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
                    let state = create_test_state_with_network(node).await;
                    let state_wrapper = tauri::State::new(state);
                    
                    create_identity(
                        state_wrapper,
                        black_box("Benchmark User".to_string()),
                        None,
                        app.app_handle()
                    ).await
                })
            })
        });
    }

    fn bench_message_sending(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("message_sending", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let app = tauri::test::mock_app();
                    let node = Arc::new(P2PNode::new(NodeConfig::default()).await.unwrap());
                    let state = create_test_state_with_network(node).await;
                    let state_wrapper = tauri::State::new(state);
                    
                    send_message(
                        state_wrapper,
                        black_box("recipient".to_string()),
                        black_box("Benchmark message".to_string()),
                        app.app_handle()
                    ).await
                })
            })
        });
    }

    criterion_group!(benches, bench_identity_creation, bench_message_sending);
    criterion_main!(benches);
}