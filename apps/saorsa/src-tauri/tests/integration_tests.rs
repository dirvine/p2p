// Integration tests for complete Saorsa workflows

use saorsa_lib::*;
use saorsa_core::{
    network::{P2PNode, NodeConfig},
    identity::{UserIdentity, VerificationLevel, EncryptedUserProfile},
    dht::{DHTStore, DHTConfig},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tempfile::TempDir;
use std::time::Duration;

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Helper to create test nodes
    async fn create_test_node(port: u16) -> Arc<P2PNode> {
        let mut config = NodeConfig::default();
        config.port = port;
        config.bootstrap_nodes = vec![];
        Arc::new(P2PNode::new(config).await.unwrap())
    }

    // Helper to create test app state with network
    async fn create_test_state_with_network(node: Arc<P2PNode>) -> AppState {
        let mut state = AppState::default();
        *state.network.write().await = Some(node);
        
        // Initialize identity manager
        let identity_manager = Arc::new(
            saorsa_core::identity::manager::IdentityManager::new(
                saorsa_core::identity::manager::IdentityManagerConfig::default()
            ).await.unwrap()
        );
        *state.identity_manager.write().await = Some(identity_manager);
        
        state
    }

    #[tokio::test]
    async fn test_full_identity_creation_and_registration_flow() {
        let temp_dir = TempDir::new().unwrap();
        let app = tauri::test::mock_app();
        
        // Create network node
        let node = create_test_node(9001).await;
        let state = create_test_state_with_network(node.clone()).await;
        let state_wrapper = tauri::State::new(state);
        
        // Step 1: Create identity
        let identity_result = create_identity(
            state_wrapper.clone(),
            "Test User".to_string(),
            None,
            app.app_handle()
        ).await;
        assert!(identity_result.is_ok());
        
        let identity_data = identity_result.unwrap();
        assert_eq!(identity_data.display_name, "Test User");
        assert!(!identity_data.three_word_address.is_empty());
        
        // Step 2: Create inbox for the identity
        let inbox_result = create_inbox(
            state_wrapper.clone(),
            app.app_handle()
        ).await;
        assert!(inbox_result.is_ok());
        
        // Step 3: Verify identity is stored in DHT
        let dht = node.dht();
        tokio::time::sleep(Duration::from_millis(100)).await; // Wait for DHT propagation
        
        let stored_identity = dht.get(&format!("identity:{}", identity_data.user_id)).await;
        assert!(stored_identity.is_ok());
        
        // Step 4: Export identity
        let export_result = export_identity(state_wrapper.clone()).await;
        assert!(export_result.is_ok());
        
        let export_data = export_result.unwrap();
        assert!(!export_data.is_empty());
        
        // Step 5: Import identity in a new state
        let node2 = create_test_node(9002).await;
        let state2 = create_test_state_with_network(node2).await;
        let state_wrapper2 = tauri::State::new(state2);
        
        let import_result = import_identity(
            state_wrapper2.clone(),
            export_data,
            app.app_handle()
        ).await;
        assert!(import_result.is_ok());
        
        // Verify imported identity matches
        let current_identity2 = state_wrapper2.current_identity.read().await;
        assert!(current_identity2.is_some());
        let imported = current_identity2.as_ref().unwrap();
        assert_eq!(imported.user_id, identity_data.user_id);
        assert_eq!(imported.display_name_hint, identity_data.display_name);
    }

    #[tokio::test]
    async fn test_end_to_end_messaging_between_nodes() {
        // Create two test nodes
        let node1 = create_test_node(9101).await;
        let node2 = create_test_node(9102).await;
        
        // Connect nodes
        let addr2 = node2.local_addr().await.unwrap();
        node1.connect_peer(&addr2.to_string()).await.unwrap();
        
        // Create app states
        let state1 = create_test_state_with_network(node1.clone()).await;
        let state2 = create_test_state_with_network(node2.clone()).await;
        
        let state_wrapper1 = tauri::State::new(state1);
        let state_wrapper2 = tauri::State::new(state2);
        
        let app = tauri::test::mock_app();
        
        // Create identities for both users
        let identity1 = create_identity(
            state_wrapper1.clone(),
            "Alice".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        let identity2 = create_identity(
            state_wrapper2.clone(),
            "Bob".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        // Create inboxes
        create_inbox(state_wrapper1.clone(), app.app_handle()).await.unwrap();
        create_inbox(state_wrapper2.clone(), app.app_handle()).await.unwrap();
        
        // Add each other as contacts
        {
            let mut contacts1 = state_wrapper1.contacts.write().await;
            contacts1.insert(identity2.user_id.clone(), Contact {
                id: identity2.user_id.clone(),
                name: "Bob".to_string(),
                nickname: None,
                three_word_address: identity2.three_word_address.clone(),
                is_online: true,
                last_seen: 0,
                unread_count: 0,
                is_blocked: false,
                notes: None,
                category: None,
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
        
        // Send message from Alice to Bob
        let msg_result = send_message(
            state_wrapper1.clone(),
            identity2.user_id.clone(),
            "Hello Bob!".to_string(),
            app.app_handle()
        ).await;
        assert!(msg_result.is_ok());
        
        // Wait for message propagation
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Check Bob received the message via DHT
        let dht2 = node2.dht();
        let inbox_messages = dht2.get(&format!("inbox:{}", identity2.three_word_address)).await;
        assert!(inbox_messages.is_ok());
        
        // Verify message content
        let messages1 = state_wrapper1.messages.read().await;
        let alice_msgs = messages1.get(&identity2.user_id).unwrap();
        assert_eq!(alice_msgs.len(), 1);
        assert_eq!(alice_msgs[0].content, "Hello Bob!");
        assert_eq!(alice_msgs[0].from_peer, identity1.user_id);
    }

    #[tokio::test]
    async fn test_contact_request_workflow_complete() {
        // Setup two nodes
        let node1 = create_test_node(9201).await;
        let node2 = create_test_node(9202).await;
        
        // Connect nodes
        let addr2 = node2.local_addr().await.unwrap();
        node1.connect_peer(&addr2.to_string()).await.unwrap();
        
        let state1 = create_test_state_with_network(node1.clone()).await;
        let state2 = create_test_state_with_network(node2.clone()).await;
        
        let state_wrapper1 = tauri::State::new(state1);
        let state_wrapper2 = tauri::State::new(state2);
        
        let app = tauri::test::mock_app();
        
        // Create identities
        let identity1 = create_identity(
            state_wrapper1.clone(),
            "Alice".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        let identity2 = create_identity(
            state_wrapper2.clone(),
            "Bob".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        // Alice sends contact request to Bob
        let request_result = send_contact_request(
            state_wrapper1.clone(),
            identity2.user_id.clone(),
            "Hi Bob, let's connect!".to_string()
        ).await;
        assert!(request_result.is_ok());
        
        // Wait for DHT propagation
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Check request was stored in DHT
        let dht1 = node1.dht();
        let stored_request = dht1.get(&format!("contact_request:{}:{}", 
            identity2.user_id, 
            identity1.user_id
        )).await;
        assert!(stored_request.is_ok());
        
        // Simulate Bob checking for contact requests
        // In real app, this would be handled by background task
        let requests1 = state_wrapper1.contact_requests.read().await;
        assert_eq!(requests1.sent.len(), 1);
        assert_eq!(requests1.sent[0].to_user_id, identity2.user_id);
        
        // Bob accepts the request
        // First, manually add the request to Bob's received list (simulating DHT sync)
        {
            let mut requests2 = state_wrapper2.contact_requests.write().await;
            requests2.received.push(ContactRequest {
                request_id: requests1.sent[0].request_id.clone(),
                from_user_id: identity1.user_id.clone(),
                from_user_name: identity1.display_name.clone(),
                to_user_id: identity2.user_id.clone(),
                to_user_name: Some(identity2.display_name.clone()),
                message: "Hi Bob, let's connect!".to_string(),
                created_at: chrono::Utc::now(),
                status: ContactRequestStatus::Pending,
            });
        }
        
        let accept_result = accept_contact_request(
            state_wrapper2.clone(),
            requests1.sent[0].request_id.clone(),
            app.app_handle()
        ).await;
        assert!(accept_result.is_ok());
        
        // Verify Bob now has Alice as a contact
        let contacts2 = state_wrapper2.contacts.read().await;
        assert!(contacts2.contains_key(&identity1.user_id));
        assert_eq!(contacts2.get(&identity1.user_id).unwrap().name, "Alice");
        
        // Verify request was removed from pending
        let requests2 = state_wrapper2.contact_requests.read().await;
        assert_eq!(requests2.received.len(), 0);
    }

    #[tokio::test]
    async fn test_webrtc_call_establishment() {
        // This test verifies WebRTC signaling through P2P network
        let node1 = create_test_node(9301).await;
        let node2 = create_test_node(9302).await;
        
        // Connect nodes
        let addr2 = node2.local_addr().await.unwrap();
        node1.connect_peer(&addr2.to_string()).await.unwrap();
        
        let state1 = create_test_state_with_network(node1).await;
        let state2 = create_test_state_with_network(node2).await;
        
        let state_wrapper1 = tauri::State::new(state1);
        let state_wrapper2 = tauri::State::new(state2);
        
        let app = tauri::test::mock_app();
        
        // Create identities
        let identity1 = create_identity(
            state_wrapper1.clone(),
            "Alice".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        let identity2 = create_identity(
            state_wrapper2.clone(),
            "Bob".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        // Alice initiates call to Bob
        let offer_result = send_call_offer(
            state_wrapper1.clone(),
            identity2.user_id.clone(),
            "channel123".to_string(),
            "offer_sdp_data".to_string(),
            false,
            app.app_handle()
        ).await;
        assert!(offer_result.is_ok());
        
        // Wait for signaling
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Bob sends answer
        let answer_result = send_call_answer(
            state_wrapper2.clone(),
            identity1.user_id.clone(),
            "channel123".to_string(),
            "answer_sdp_data".to_string(),
            app.app_handle()
        ).await;
        assert!(answer_result.is_ok());
        
        // Exchange ICE candidates
        let ice_result1 = send_ice_candidate(
            state_wrapper1.clone(),
            identity2.user_id.clone(),
            serde_json::json!({
                "candidate": "candidate:123",
                "sdpMLineIndex": 0,
                "sdpMid": "0"
            }),
            app.app_handle()
        ).await;
        assert!(ice_result1.is_ok());
        
        let ice_result2 = send_ice_candidate(
            state_wrapper2.clone(),
            identity1.user_id.clone(),
            serde_json::json!({
                "candidate": "candidate:456",
                "sdpMLineIndex": 0,
                "sdpMid": "0"
            }),
            app.app_handle()
        ).await;
        assert!(ice_result2.is_ok());
        
        // End call
        let end_result = end_call(
            state_wrapper1.clone(),
            identity2.user_id.clone(),
            "channel123".to_string(),
            "user-ended".to_string(),
            app.app_handle()
        ).await;
        assert!(end_result.is_ok());
    }

    #[tokio::test]
    async fn test_identity_import_export_with_verification() {
        let temp_dir = TempDir::new().unwrap();
        let app = tauri::test::mock_app();
        
        // Create first node and identity
        let node1 = create_test_node(9401).await;
        let state1 = create_test_state_with_network(node1).await;
        let state_wrapper1 = tauri::State::new(state1);
        
        // Create identity with profile
        let identity_result = create_identity(
            state_wrapper1.clone(),
            "Alice Exporter".to_string(),
            Some("I love P2P networking!".to_string()),
            app.app_handle()
        ).await;
        assert!(identity_result.is_ok());
        
        let original_identity = identity_result.unwrap();
        
        // Set avatar
        let avatar_data = vec![1, 2, 3, 4, 5]; // Mock avatar data
        let avatar_result = update_avatar(
            state_wrapper1.clone(),
            avatar_data.clone(),
            app.app_handle()
        ).await;
        assert!(avatar_result.is_ok());
        
        // Export identity
        let export_result = export_identity(state_wrapper1.clone()).await;
        assert!(export_result.is_ok());
        let export_data = export_result.unwrap();
        
        // Create second node and import
        let node2 = create_test_node(9402).await;
        let state2 = create_test_state_with_network(node2).await;
        let state_wrapper2 = tauri::State::new(state2);
        
        let import_result = import_identity(
            state_wrapper2.clone(),
            export_data,
            app.app_handle()
        ).await;
        assert!(import_result.is_ok());
        
        // Verify imported identity
        let current_identity = state_wrapper2.current_identity.read().await;
        assert!(current_identity.is_some());
        
        let imported = current_identity.as_ref().unwrap();
        assert_eq!(imported.user_id, original_identity.user_id);
        assert_eq!(imported.display_name_hint, original_identity.display_name);
        assert_eq!(imported.three_word_address, original_identity.three_word_address);
        
        // Verify profile was imported
        let profile = state_wrapper2.current_profile.read().await;
        assert!(profile.is_some());
        
        // Note: We can't directly check encrypted profile data, but we verify it exists
        let encrypted_profile = profile.as_ref().unwrap();
        assert_eq!(encrypted_profile.user_id, original_identity.user_id);
        assert!(!encrypted_profile.encrypted_data.is_empty());
        assert!(!encrypted_profile.signature.is_empty());
    }

    #[tokio::test]
    async fn test_concurrent_operations_stress_test() {
        use tokio::task::JoinSet;
        
        // Create a network with multiple nodes
        let mut nodes = vec![];
        let mut states = vec![];
        
        // Create 5 nodes
        for i in 0..5 {
            let node = create_test_node(9500 + i).await;
            nodes.push(node.clone());
            
            let state = create_test_state_with_network(node).await;
            states.push(Arc::new(state));
        }
        
        // Connect all nodes to first node (bootstrap)
        let bootstrap_addr = nodes[0].local_addr().await.unwrap();
        for i in 1..5 {
            nodes[i].connect_peer(&bootstrap_addr.to_string()).await.unwrap();
        }
        
        let app = tauri::test::mock_app();
        
        // Create identities for all nodes
        let mut identities = vec![];
        for (i, state) in states.iter().enumerate() {
            let state_wrapper = tauri::State::new(state.as_ref().clone());
            let identity = create_identity(
                state_wrapper,
                format!("User{}", i),
                None,
                app.app_handle()
            ).await.unwrap();
            identities.push(identity);
        }
        
        // Perform concurrent operations
        let mut tasks = JoinSet::new();
        
        // Each node sends messages to all others
        for i in 0..5 {
            for j in 0..5 {
                if i != j {
                    let state = states[i].clone();
                    let to_id = identities[j].user_id.clone();
                    let from_name = identities[i].display_name.clone();
                    let app_handle = app.app_handle();
                    
                    tasks.spawn(async move {
                        let state_wrapper = tauri::State::new(state.as_ref().clone());
                        let msg = format!("Hello from {} to user {}", from_name, j);
                        send_message(
                            state_wrapper,
                            to_id,
                            msg,
                            app_handle
                        ).await
                    });
                }
            }
        }
        
        // Wait for all messages to be sent
        while let Some(result) = tasks.join_next().await {
            assert!(result.is_ok());
            let msg_result = result.unwrap();
            assert!(msg_result.is_ok());
        }
        
        // Allow time for message propagation
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Verify message counts
        for (i, state) in states.iter().enumerate() {
            let messages = state.messages.read().await;
            // Each node should have sent 4 messages (to 4 other nodes)
            let total_messages: usize = messages.values().map(|v| v.len()).sum();
            assert!(total_messages >= 4, "Node {} has {} messages", i, total_messages);
        }
    }

    #[tokio::test]
    async fn test_offline_message_delivery_via_dht() {
        // Create two nodes
        let node1 = create_test_node(9601).await;
        let node2 = create_test_node(9602).await;
        
        // Initially connect them
        let addr2 = node2.local_addr().await.unwrap();
        node1.connect_peer(&addr2.to_string()).await.unwrap();
        
        let state1 = create_test_state_with_network(node1.clone()).await;
        let state2 = create_test_state_with_network(node2.clone()).await;
        
        let state_wrapper1 = tauri::State::new(state1);
        let state_wrapper2 = tauri::State::new(state2);
        
        let app = tauri::test::mock_app();
        
        // Create identities
        let identity1 = create_identity(
            state_wrapper1.clone(),
            "Alice".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        let identity2 = create_identity(
            state_wrapper2.clone(),
            "Bob".to_string(),
            None,
            app.app_handle()
        ).await.unwrap();
        
        // Create inboxes
        create_inbox(state_wrapper1.clone(), app.app_handle()).await.unwrap();
        create_inbox(state_wrapper2.clone(), app.app_handle()).await.unwrap();
        
        // Add contacts
        {
            let mut contacts1 = state_wrapper1.contacts.write().await;
            contacts1.insert(identity2.user_id.clone(), Contact {
                id: identity2.user_id.clone(),
                name: "Bob".to_string(),
                nickname: None,
                three_word_address: identity2.three_word_address.clone(),
                is_online: false, // Mark as offline
                last_seen: 0,
                unread_count: 0,
                is_blocked: false,
                notes: None,
                category: None,
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
        
        // Alice sends message to offline Bob
        let msg_result = send_message(
            state_wrapper1.clone(),
            identity2.user_id.clone(),
            "Message while you were offline!".to_string(),
            app.app_handle()
        ).await;
        assert!(msg_result.is_ok());
        
        // Verify message was stored in DHT
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let dht1 = node1.dht();
        let inbox_key = format!("inbox:{}", identity2.three_word_address);
        let stored_msgs = dht1.get(&inbox_key).await;
        assert!(stored_msgs.is_ok());
        
        // Simulate Bob coming online and checking inbox
        // In real app, this would be handled by background task
        let dht2 = node2.dht();
        let bob_inbox = dht2.get(&inbox_key).await;
        assert!(bob_inbox.is_ok());
        
        // The message should be available in DHT for Bob to retrieve
        let messages1 = state_wrapper1.messages.read().await;
        let alice_msgs = messages1.get(&identity2.user_id).unwrap();
        assert_eq!(alice_msgs.len(), 1);
        assert_eq!(alice_msgs[0].content, "Message while you were offline!");
    }
}