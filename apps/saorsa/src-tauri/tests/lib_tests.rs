
// Unit tests for lib.rs functions

use saorsa_lib::*;
use saorsa_core::{
    network::{P2PNode, NodeConfig},
    identity::{UserIdentity, VerificationLevel},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tauri::Manager;

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
    async fn test_network_state_management() {
        let state = create_test_state();
        
        // Test initial state
        let network_guard = state.network.read().await;
        assert!(network_guard.is_none());
        drop(network_guard);
        
        // Test setting network
        let network = create_mock_network().await;
        *state.network.write().await = Some(network);
        
        let network_guard = state.network.read().await;
        assert!(network_guard.is_some());
    }

    #[tokio::test]
    async fn test_contacts_management() {
        let state = create_test_state();
        
        // Test initial empty contacts
        let contacts = state.contacts.read().await;
        assert_eq!(contacts.len(), 0);
        drop(contacts);
        
        // Test adding contact
        let mut contacts = state.contacts.write().await;
        let test_contact = Contact {
            id: "test_contact".to_string(),
            name: "Test Contact".to_string(),
            nickname: None,
            three_word_address: "test.contact.address".to_string(),
            is_online: false,
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
        };
        contacts.insert("test_contact".to_string(), test_contact);
        assert_eq!(contacts.len(), 1);
    }

    #[tokio::test]
    async fn test_messages_management() {
        let state = create_test_state();
        
        // Test initial empty messages
        let messages = state.messages.read().await;
        assert_eq!(messages.len(), 0);
        drop(messages);
        
        // Test adding message
        let mut messages = state.messages.write().await;
        let test_message = Message {
            id: "test_msg".to_string(),
            content: "Test message".to_string(),
            from_peer: "test_peer".to_string(),
            to_peer: "test_contact".to_string(),
            timestamp: chrono::Utc::now(),
            is_from_me: false,
            status: MessageStatus::Delivered,
            reply_to: None,
            edited: false,
            reactions: HashMap::new(),
            attachments: vec![],
        };
        messages.insert("test_contact".to_string(), vec![test_message]);
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_identity_manager_initialization() {
        let state = create_test_state();
        
        // Test initial identity manager
        let identity_manager = state.identity_manager.read().await;
        assert!(identity_manager.is_none());
        drop(identity_manager);
        
        // Test setting identity manager
        let manager = Arc::new(
            saorsa_core::identity::manager::IdentityManager::new(
                saorsa_core::identity::manager::IdentityManagerConfig::default()
            )
        );
        *state.identity_manager.write().await = Some(manager);
        
        let identity_manager = state.identity_manager.read().await;
        assert!(identity_manager.is_some());
    }

    #[tokio::test]
    async fn test_contact_requests_management() {
        let state = create_test_state();
        
        // Test initial contact requests
        let requests = state.contact_requests.read().await;
        assert_eq!(requests.sent.len(), 0);
        assert_eq!(requests.received.len(), 0);
        drop(requests);
        
        // Test adding contact request
        let mut requests = state.contact_requests.write().await;
        let test_request = ContactRequest {
            request_id: "test_req".to_string(),
            from_user_id: "test_from".to_string(),
            from_user_name: "Test From".to_string(),
            to_user_id: "test_to".to_string(),
            to_user_name: Some("Test To".to_string()),
            message: "Test request".to_string(),
            created_at: chrono::Utc::now(),
            status: ContactRequestStatus::Pending,
        };
        requests.sent.push(test_request);
        assert_eq!(requests.sent.len(), 1);
    }

    #[tokio::test]
    async fn test_blocked_users_management() {
        let state = create_test_state();
        
        // Test initial blocked users
        let blocked = state.blocked_users.read().await;
        assert_eq!(blocked.len(), 0);
        drop(blocked);
        
        // Test blocking user
        let mut blocked = state.blocked_users.write().await;
        let timestamp = chrono::Utc::now().timestamp();
        blocked.insert("blocked_user".to_string(), timestamp);
        assert_eq!(blocked.len(), 1);
    }

    #[tokio::test]
    async fn test_contact_categories_management() {
        let state = create_test_state();
        
        // Test default categories
        let categories = state.contact_categories.read().await;
        assert!(categories.len() > 0);
        assert!(categories.contains(&"Friends".to_string()));
        assert!(categories.contains(&"Family".to_string()));
        assert!(categories.contains(&"Work".to_string()));
    }
}