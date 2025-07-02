// Unit tests for lib.rs functions

use saorsa_lib::*;
use saorsa_core::{
    network::{P2PNode, NodeConfig},
    identity::{UserIdentity, VerificationLevel},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test app state
    fn create_test_state() -> AppState {
        AppState::default()
    }

    // Helper function to create a mock network
    async fn create_mock_network() -> Arc<P2PNode> {
        let config = NodeConfig::default();
        Arc::new(P2PNode::new(config).await.unwrap())
    }

    #[tokio::test]
    async fn test_get_network_status_disconnected() {
        let state = create_test_state();
        let state_wrapper = tauri::State::new(state);
        
        let result = get_network_status(state_wrapper).await;
        assert!(result.is_ok());
        
        let status = result.unwrap();
        assert_eq!(status.is_connected, false);
        assert_eq!(status.local_address, "Not connected");
        assert_eq!(status.peer_count, 0);
        assert_eq!(status.bootstrap_nodes, 0);
    }

    #[tokio::test]
    async fn test_get_network_status_connected() {
        let mut state = create_test_state();
        let network = create_mock_network().await;
        *state.network.write().await = Some(network);
        
        let state_wrapper = tauri::State::new(state);
        let result = get_network_status(state_wrapper).await;
        assert!(result.is_ok());
        
        let status = result.unwrap();
        // Should show as connected with network initialized
        assert!(status.local_address != "Not connected");
    }

    #[tokio::test]
    async fn test_connect_peer_invalid_address() {
        let state = create_test_state();
        let state_wrapper = tauri::State::new(state);
        
        // Create a mock app handle
        let app = tauri::test::mock_app();
        
        let result = connect_peer(
            state_wrapper,
            "invalid-address".to_string(),
            app.app_handle()
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid address format"));
    }

    #[tokio::test]
    async fn test_send_message_without_network() {
        let state = create_test_state();
        let state_wrapper = tauri::State::new(state);
        let app = tauri::test::mock_app();
        
        let result = send_message(
            state_wrapper,
            "contact123".to_string(),
            "Hello, world!".to_string(),
            app.app_handle()
        ).await;
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Network not initialized");
    }

    #[tokio::test]
    async fn test_send_system_message() {
        let mut state = create_test_state();
        let network = create_mock_network().await;
        *state.network.write().await = Some(network);
        
        let state_wrapper = tauri::State::new(state);
        let app = tauri::test::mock_app();
        
        let result = send_message(
            state_wrapper.clone(),
            "system".to_string(),
            "?".to_string(),
            app.app_handle()
        ).await;
        
        assert!(result.is_ok());
        
        // Check that help response was added
        let messages = state_wrapper.messages.read().await;
        let system_messages = messages.get("system").unwrap();
        assert!(system_messages.len() >= 2); // Original + response
        
        let help_msg = system_messages.last().unwrap();
        assert!(help_msg.content.contains("Available options"));
        assert_eq!(help_msg.from_peer, "system");
    }

    #[tokio::test]
    async fn test_get_contacts_empty() {
        let state = create_test_state();
        let state_wrapper = tauri::State::new(state);
        
        let result = get_contacts(state_wrapper).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_get_contacts_with_data() {
        let mut state = create_test_state();
        
        // Add test contacts
        let mut contacts = state.contacts.write().await;
        contacts.insert("alice".to_string(), Contact {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            nickname: None,
            three_word_address: "alice.test.address".to_string(),
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
        drop(contacts);
        
        let state_wrapper = tauri::State::new(state);
        let result = get_contacts(state_wrapper).await;
        assert!(result.is_ok());
        
        let contacts = result.unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "Alice");
    }

    #[tokio::test]
    async fn test_delete_contact() {
        let mut state = create_test_state();
        
        // Add a contact
        let mut contacts = state.contacts.write().await;
        contacts.insert("alice".to_string(), Contact {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            nickname: None,
            three_word_address: "alice.test.address".to_string(),
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
        drop(contacts);
        
        let state_wrapper = tauri::State::new(state);
        
        // Delete the contact
        let result = delete_contact(state_wrapper.clone(), "alice".to_string()).await;
        assert!(result.is_ok());
        
        // Verify contact was deleted
        let contacts = state_wrapper.contacts.read().await;
        assert!(!contacts.contains_key("alice"));
    }

    #[tokio::test]
    async fn test_delete_system_contact_fails() {
        let state = create_test_state();
        let state_wrapper = tauri::State::new(state);
        
        let result = delete_contact(state_wrapper, "system".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot delete system contact");
    }

    #[tokio::test]
    async fn test_block_unblock_user() {
        let state = create_test_state();
        let state_wrapper = tauri::State::new(state);
        
        // Block user
        let result = block_user(state_wrapper.clone(), "user123".to_string()).await;
        assert!(result.is_ok());
        
        // Verify user is blocked
        let blocked = state_wrapper.blocked_users.read().await;
        assert!(blocked.contains_key("user123"));
        drop(blocked);
        
        // Unblock user
        let result = unblock_user(state_wrapper.clone(), "user123".to_string()).await;
        assert!(result.is_ok());
        
        // Verify user is unblocked
        let blocked = state_wrapper.blocked_users.read().await;
        assert!(!blocked.contains_key("user123"));
    }

    #[tokio::test]
    async fn test_update_contact() {
        let mut state = create_test_state();
        
        // Add a contact
        let mut contacts = state.contacts.write().await;
        contacts.insert("alice".to_string(), Contact {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            nickname: None,
            three_word_address: "alice.test.address".to_string(),
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
        drop(contacts);
        
        let state_wrapper = tauri::State::new(state);
        
        // Update contact
        let result = update_contact(
            state_wrapper.clone(),
            "alice".to_string(),
            Some("Ali".to_string()),
            Some("Friend from work".to_string()),
            Some("Work".to_string())
        ).await;
        assert!(result.is_ok());
        
        // Verify updates
        let contacts = state_wrapper.contacts.read().await;
        let alice = contacts.get("alice").unwrap();
        assert_eq!(alice.nickname, Some("Ali".to_string()));
        assert_eq!(alice.notes, Some("Friend from work".to_string()));
        assert_eq!(alice.category, Some("Work".to_string()));
    }

    #[tokio::test]
    async fn test_search_users_empty_query() {
        let state = create_test_state();
        let state_wrapper = tauri::State::new(state);
        
        let result = search_users(state_wrapper, "".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Network not initialized");
    }

    #[tokio::test]
    async fn test_contact_request_workflow() {
        let mut state = create_test_state();
        let network = create_mock_network().await;
        *state.network.write().await = Some(network);
        
        // Initialize identity manager
        let identity_manager = Arc::new(
            saorsa_core::identity::manager::IdentityManager::new(
                saorsa_core::identity::manager::IdentityManagerConfig::default()
            ).await.unwrap()
        );
        
        // Create test identity
        let test_identity = identity_manager.create_identity("Test User").await.unwrap();
        *state.identity_manager.write().await = Some(identity_manager);
        
        let state_wrapper = tauri::State::new(state);
        
        // Send contact request
        let result = send_contact_request(
            state_wrapper.clone(),
            "recipient123".to_string(),
            "Hello, let's connect!".to_string()
        ).await;
        assert!(result.is_ok());
        
        // Check that request was stored
        let requests = state_wrapper.contact_requests.read().await;
        assert_eq!(requests.sent.len(), 1);
        assert_eq!(requests.sent[0].to_user_id, "recipient123");
        assert_eq!(requests.sent[0].message, "Hello, let's connect!");
    }

    #[tokio::test]
    async fn test_accept_contact_request() {
        let mut state = create_test_state();
        let app = tauri::test::mock_app();
        
        // Add a pending contact request
        let mut requests = state.contact_requests.write().await;
        requests.received.push(ContactRequest {
            request_id: "req123".to_string(),
            from_user_id: "sender123".to_string(),
            from_user_name: "Sender Name".to_string(),
            to_user_id: "me".to_string(),
            to_user_name: None,
            message: "Let's connect!".to_string(),
            created_at: chrono::Utc::now(),
            status: ContactRequestStatus::Pending,
        });
        drop(requests);
        
        let state_wrapper = tauri::State::new(state);
        
        // Accept the request
        let result = accept_contact_request(
            state_wrapper.clone(),
            "req123".to_string(),
            app.app_handle()
        ).await;
        assert!(result.is_ok());
        
        // Verify contact was created
        let contacts = state_wrapper.contacts.read().await;
        assert!(contacts.contains_key("sender123"));
        
        // Verify request was removed from pending
        let requests = state_wrapper.contact_requests.read().await;
        assert_eq!(requests.received.len(), 0);
    }

    #[tokio::test]
    async fn test_reject_contact_request() {
        let mut state = create_test_state();
        
        // Add a pending contact request
        let mut requests = state.contact_requests.write().await;
        requests.received.push(ContactRequest {
            request_id: "req123".to_string(),
            from_user_id: "sender123".to_string(),
            from_user_name: "Sender Name".to_string(),
            to_user_id: "me".to_string(),
            to_user_name: None,
            message: "Let's connect!".to_string(),
            created_at: chrono::Utc::now(),
            status: ContactRequestStatus::Pending,
        });
        drop(requests);
        
        let state_wrapper = tauri::State::new(state);
        
        // Reject the request
        let result = reject_contact_request(
            state_wrapper.clone(),
            "req123".to_string()
        ).await;
        assert!(result.is_ok());
        
        // Verify request status was updated
        let requests = state_wrapper.contact_requests.read().await;
        assert_eq!(requests.received[0].status, ContactRequestStatus::Rejected);
    }
}