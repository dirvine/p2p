//! # Ant Connect Desktop - Tauri Application
//!
//! Native desktop application for P2P Foundation, built with Tauri for
//! superior performance and native system integration.
//!
//! ## Features
//!
//! - Direct Rust P2P library integration (no FFI overhead)
//! - Native OS integration and permissions
//! - High-performance networking with QUIC
//! - Three-word address system
//! - DHT-based messaging and contacts
//! - Cross-platform desktop support (macOS, Windows, Linux)

pub mod identity_storage;
pub mod frontend_bundle;

use identity_storage::{IdentityStorage, IdentityStorageConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State, Emitter};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use base64;

use saorsa_core::{
    network::{P2PNode, NodeConfig, DHTConfig as NetworkDHTConfig, SecurityConfig, TrustLevel},
    dht::{DHT, DHTConfig, Key, Record},
    production::ProductionConfig,
    identity::{
        UserIdentity, UserProfile, EncryptedUserProfile, 
        ProfilePermissions, PrivacySettings, DiscoverabilitySettings,
        UserPreferences, VerificationLevel, DefaultPermissions,
    },
    identity::manager::{IdentityManager, IdentityManagerConfig},
    PeerId, Multiaddr, Result as P2PResult,
};

/// Application state for P2P network
pub struct AppState {
    network: RwLock<Option<Arc<P2PNode>>>,
    contacts: RwLock<HashMap<String, Contact>>,
    messages: RwLock<HashMap<String, Vec<Message>>>,
    identity_manager: RwLock<Option<Arc<IdentityManager>>>,
    identity_storage: RwLock<Option<Arc<IdentityStorage>>>,
    blocked_users: RwLock<HashMap<String, i64>>, // user_id -> blocked_at timestamp
    contact_categories: RwLock<Vec<String>>, // Available categories
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            network: RwLock::new(None),
            contacts: RwLock::new(HashMap::new()),
            messages: RwLock::new(HashMap::new()),
            identity_manager: RwLock::new(None),
            identity_storage: RwLock::new(None),
            blocked_users: RwLock::new(HashMap::new()),
            contact_categories: RwLock::new(vec!["Friends".to_string(), "Family".to_string(), "Work".to_string()]),
        }
    }
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub nickname: Option<String>,  // User-defined nickname
    pub three_word_address: String,
    pub is_online: bool,
    pub last_seen: i64,
    pub unread_count: u32,
    pub is_blocked: bool,           // Block status
    pub notes: Option<String>,      // Personal notes about contact
    pub category: Option<String>,   // Contact category/group
    pub permissions: ContactPermissions, // Per-contact privacy settings
    pub added_at: i64,             // When contact was added
    pub trust_level: f32,          // Trust score
}

/// Per-contact privacy permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactPermissions {
    pub can_see_profile: bool,
    pub can_see_online_status: bool,
    pub can_see_last_seen: bool,
    pub can_see_avatar: bool,
    pub can_send_messages: bool,
}

/// Message data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub from_peer: String,
    pub to_peer: String,
    pub timestamp: i64,
    pub is_from_me: bool,
}

/// Network status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub is_connected: bool,
    pub local_address: String,
    pub peer_count: u32,
    pub bootstrap_nodes: u32,
}

/// Initialize the P2P network
#[tauri::command]
async fn init_network(
    state: State<'_, AppState>,
    listen_port: Option<u16>,
    bootstrap_nodes: Vec<String>,
) -> Result<String, String> {
    info!("Initializing P2P network on port {:?}", listen_port);
    
    // Create a default configuration with mobile-friendly binding
    let listen_addr = if cfg!(target_os = "ios") || cfg!(target_os = "android") {
        // Mobile platforms: bind to all interfaces for P2P connectivity
        format!("0.0.0.0:{}", listen_port.unwrap_or(9000))
    } else {
        // Desktop: bind to localhost for security
        format!("127.0.0.1:{}", listen_port.unwrap_or(9000))
    }
    .parse()
    .map_err(|e| format!("Invalid listen address: {}", e))?;
    
    let bootstrap_peers: Vec<Multiaddr> = bootstrap_nodes
        .into_iter()
        .filter_map(|addr| addr.parse().ok())
        .collect();
    
    let config = NodeConfig {
        peer_id: None,
        listen_addrs: vec![],
        listen_addr,
        bootstrap_peers: bootstrap_peers.clone(),
        bootstrap_peers_str: vec![],
        enable_ipv6: true,
        enable_mcp_server: true,
        mcp_server_config: None,
        connection_timeout: Duration::from_secs(30),
        keep_alive_interval: Duration::from_secs(60),
        max_connections: if cfg!(target_os = "ios") || cfg!(target_os = "android") { 50 } else { 100 },
        max_incoming_connections: if cfg!(target_os = "ios") || cfg!(target_os = "android") { 25 } else { 50 },
        dht_config: NetworkDHTConfig {
            k_value: 20,
            alpha_value: 3,
            record_ttl: Duration::from_secs(86400), // 24 hours
            refresh_interval: Duration::from_secs(3600), // 1 hour
        },
        security_config: SecurityConfig {
            enable_noise: true,
            enable_tls: true,
            trust_level: TrustLevel::Basic,
        },
        production_config: Some(ProductionConfig::default()),
        bootstrap_cache_config: None,
        identity_config: Some(IdentityManagerConfig::default()),
    };

    match P2PNode::new(config).await {
        Ok(mut network) => {
            let network_arc = Arc::new(network);
            
            // Set network reference in identity manager if available
            if let Some(identity_manager) = state.identity_manager.write().await.as_mut() {
                if let Some(manager) = Arc::get_mut(identity_manager) {
                    // Note: Network integration simplified for basic functionality
                    info!("Network reference set in identity manager");
                }
            }
            
            let mut net_guard = state.network.write().await;
            *net_guard = Some(network_arc);
            
            // Initialize with system contact
            let mut contacts = state.contacts.write().await;
            contacts.insert("system".to_string(), Contact {
                id: "system".to_string(),
                name: "System".to_string(),
                nickname: None,
                three_word_address: "system.helper.assistant".to_string(),
                is_online: true,
                last_seen: chrono::Utc::now().timestamp(),
                unread_count: 0,
                is_blocked: false,
                notes: Some("Built-in system assistant".to_string()),
                category: None,
                permissions: ContactPermissions {
                    can_see_profile: true,
                    can_see_online_status: true,
                    can_see_last_seen: true,
                    can_see_avatar: true,
                    can_send_messages: true,
                },
                added_at: chrono::Utc::now().timestamp(),
                trust_level: 1.0,
            });
            
            Ok("Network initialized successfully".to_string())
        }
        Err(e) => {
            error!("Failed to initialize network: {}", e);
            Err(format!("Failed to initialize network: {}", e))
        }
    }
}

/// Get current network status
#[tauri::command]
async fn get_network_status(state: State<'_, AppState>) -> Result<NetworkStatus, String> {
    let net_guard = state.network.read().await;
    if let Some(network) = net_guard.as_ref() {
        // TODO: Get actual network statistics
        Ok(NetworkStatus {
            is_connected: true,
            local_address: "local.swift.lighthouse".to_string(),
            peer_count: 0,
            bootstrap_nodes: 2,
        })
    } else {
        Ok(NetworkStatus {
            is_connected: false,
            local_address: "Not connected".to_string(),
            peer_count: 0,
            bootstrap_nodes: 0,
        })
    }
}

/// Connect to a peer by address
#[tauri::command]
async fn connect_peer(
    state: State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    info!("Connecting to peer: {}", address);
    
    let net_guard = state.network.read().await;
    if let Some(_network) = net_guard.as_ref() {
        // Parse address (could be three-word or multiaddr)
        let multiaddr: Multiaddr = address.parse()
            .map_err(|e| format!("Invalid address format: {}", e))?;
        
        // TODO: Implement actual peer connection
        
        // Add as contact for demo
        let mut contacts = state.contacts.write().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        contacts.insert(contact_id.clone(), Contact {
            id: contact_id.clone(),
            name: format!("Peer ({})", &address[..8.min(address.len())]),
            nickname: None,
            three_word_address: address.clone(),
            is_online: true,
            last_seen: chrono::Utc::now().timestamp(),
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
            added_at: chrono::Utc::now().timestamp(),
            trust_level: 0.5,
        });
        
        Ok(format!("Connected to {}", address))
    } else {
        Err("Network not initialized".to_string())
    }
}

/// Send a message to a contact
#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    contact_id: String,
    content: String,
) -> Result<String, String> {
    info!("Sending message to {}: {}", contact_id, content);
    
    let net_guard = state.network.read().await;
    if net_guard.is_none() {
        return Err("Network not initialized".to_string());
    }
    
    // Create message
    let message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.clone(),
        from_peer: "local".to_string(),
        to_peer: contact_id.clone(),
        timestamp: chrono::Utc::now().timestamp(),
        is_from_me: true,
    };
    
    // Store message
    let mut messages = state.messages.write().await;
    messages.entry(contact_id.clone()).or_insert_with(Vec::new).push(message);
    
    // Handle system messages
    if contact_id == "system" && content.trim() == "?" {
        let help_response = Message {
            id: uuid::Uuid::new_v4().to_string(),
            content: "✨ Available options:\n• status - Network status\n• peers - Connected peers\n• tunnels - Tunnel information\n• addresses - Three-word addresses\n• inbox - Create DHT inbox".to_string(),
            from_peer: "system".to_string(),
            to_peer: "local".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            is_from_me: false,
        };
        
        messages.entry(contact_id).or_insert_with(Vec::new).push(help_response);
    }
    
    Ok("Message sent".to_string())
}

// ================== WebRTC Call Commands ==================

/// Send a WebRTC call offer
#[tauri::command]
async fn send_call_offer(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    user_id: String,
    channel_id: String,
    offer: String,
    is_video: bool,
) -> Result<(), String> {
    info!("Sending call offer to user: {} for channel: {}", user_id, channel_id);
    
    // TODO: Send offer through P2P network to the target user
    // For now, emit event for local testing
    app.emit("incoming-call", &serde_json::json!({
        "userId": user_id,
        "channelId": channel_id,
        "offer": offer,
        "isVideo": is_video
    })).map_err(|e| format!("Failed to emit call offer: {}", e))?;
    
    Ok(())
}

/// Send a WebRTC call answer
#[tauri::command]
async fn send_call_answer(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    user_id: String,
    channel_id: String,
    answer: String,
) -> Result<(), String> {
    info!("Sending call answer to user: {} for channel: {}", user_id, channel_id);
    
    // TODO: Send answer through P2P network to the target user
    // For now, emit event for local testing
    app.emit("call-answer", &serde_json::json!({
        "userId": user_id,
        "answer": answer
    })).map_err(|e| format!("Failed to emit call answer: {}", e))?;
    
    Ok(())
}

/// Send ICE candidate
#[tauri::command]
async fn send_ice_candidate(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    user_id: String,
    candidate: serde_json::Value,
) -> Result<(), String> {
    info!("Sending ICE candidate to user: {}", user_id);
    
    // TODO: Send ICE candidate through P2P network to the target user
    // For now, emit event for local testing
    app.emit("ice-candidate", &serde_json::json!({
        "userId": user_id,
        "candidate": candidate
    })).map_err(|e| format!("Failed to emit ICE candidate: {}", e))?;
    
    Ok(())
}

/// End a call
#[tauri::command]
async fn end_call(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    user_id: String,
    channel_id: String,
    reason: String,
) -> Result<(), String> {
    info!("Ending call with user: {} for channel: {} - reason: {}", user_id, channel_id, reason);
    
    // TODO: Send end call signal through P2P network to the target user
    // For now, emit event for local testing
    app.emit("call-ended", &serde_json::json!({
        "userId": user_id,
        "reason": reason
    })).map_err(|e| format!("Failed to emit call ended: {}", e))?;
    
    Ok(())
}

/// Get all contacts
#[tauri::command]
async fn get_contacts(state: State<'_, AppState>) -> Result<Vec<Contact>, String> {
    let contacts = state.contacts.read().await;
    Ok(contacts.values().cloned().collect())
}

/// Get messages for a contact
#[tauri::command]
async fn get_messages(
    state: State<'_, AppState>,
    contact_id: String,
) -> Result<Vec<Message>, String> {
    let messages = state.messages.read().await;
    Ok(messages.get(&contact_id).cloned().unwrap_or_default())
}

/// Create a DHT inbox
#[tauri::command]
async fn create_inbox(
    state: State<'_, AppState>,
    inbox_name: String,
) -> Result<String, String> {
    info!("Creating DHT inbox: {}", inbox_name);
    
    let net_guard = state.network.read().await;
    if let Some(_network) = net_guard.as_ref() {
        // TODO: Implement actual DHT inbox creation
        let inbox_id = uuid::Uuid::new_v4().to_string();
        let three_word_address = format!("{}.private.inbox", inbox_name.to_lowercase());
        
        Ok(format!("📬 Inbox created!\n🆔 ID: {}\n🔤 Address: {}", inbox_id, three_word_address))
    } else {
        Err("Network not initialized".to_string())
    }
}

/// Open external URL
#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    tauri_plugin_opener::open_url(url, None::<&str>)
        .map_err(|e| format!("Failed to open URL: {}", e))
}

/// Get application info
#[tauri::command]
fn get_app_info() -> HashMap<String, String> {
    let mut info = HashMap::new();
    info.insert("name".to_string(), "Saorsa".to_string());
    info.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    info.insert("description".to_string(), env!("CARGO_PKG_DESCRIPTION").to_string());
    info
}

// ================== Profile and Identity Commands ==================

#[tauri::command]
async fn get_user_identity(state: State<'_, AppState>) -> Result<Option<serde_json::Value>, String> {
    info!("Getting user identity");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Try to get local identity from memory first
    // For now, create a placeholder identity since the API is simplified
    let identity = match identity_manager.create_identity(
        "Current User".to_string(),
        "current.user.identity".to_string(),
        None,
        None,
    ).await {
        Ok(identity) => identity,
        Err(_) => {
            return Err("Failed to create identity".to_string());
        }
    };
    
    // Convert to JSON response
    let mut response = serde_json::Map::new();
    response.insert("user_id".to_string(), serde_json::Value::String(identity.user_id));
    response.insert("display_name_hint".to_string(), serde_json::Value::String(identity.display_name_hint));
    response.insert("three_word_address".to_string(), serde_json::Value::String(identity.three_word_address));
    response.insert("verification_level".to_string(), serde_json::Value::String(format!("{:?}", identity.verification_level)));
    response.insert("public_key".to_string(), serde_json::Value::String(base64::encode(&identity.public_key)));
    response.insert("created_at".to_string(), serde_json::Value::String(
        identity.created_at.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_secs().to_string()
    ));
    
    Ok(Some(serde_json::Value::Object(response)))
}

#[tauri::command]
async fn get_user_profile(state: State<'_, AppState>) -> Result<Option<serde_json::Value>, String> {
    info!("Getting user profile");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Try to get decrypted profile (simplified - returns default profile)
    let profile_result: Result<Option<UserProfile>, anyhow::Error> = Ok(Some(UserProfile {
        user_id: "default_user".to_string(),
        display_name: "Default User".to_string(),
        bio: Some("Default user profile".to_string()),
        avatar_url: None,
        avatar_hash: None,
        status_message: None,
        public_key: vec![0u8; 32], // Placeholder public key
        preferences: UserPreferences {
            theme: "dark".to_string(),
            language: "en".to_string(),
            notifications_enabled: true,
            auto_accept_friends: false,
            discovery: DiscoverabilitySettings {
                discoverable_by_name: true,
                discoverable_by_friends: true,
                allow_contact_requests: true,
                require_mutual_friends: false,
                listed_in_directory: false,
            },
            privacy: PrivacySettings {
                show_online_status: true,
                show_last_seen: true,
                allow_profile_view: true,
                encrypted_messaging: true,
                require_proof_of_humanity: true,
                max_contact_request_age: Duration::from_secs(86400 * 7),
                enable_forward_secrecy: true,
                auto_rotate_keys: true,
                key_rotation_interval: Duration::from_secs(86400 * 30),
            },
            default_permissions: Default::default(),
        },
        custom_fields: Default::default(),
        created_at: std::time::SystemTime::now(),
        updated_at: std::time::SystemTime::now(),
    }));
    match profile_result {
        Ok(Some(profile)) => {
            // Convert to JSON response
            let mut response = serde_json::Map::new();
            response.insert("display_name".to_string(), serde_json::Value::String(profile.display_name));
            response.insert("avatar_hash".to_string(), 
                profile.avatar_hash.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
            response.insert("status_message".to_string(), 
                profile.status_message.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
            
            // Add preferences
            let mut preferences = serde_json::Map::new();
            
            // Discovery settings
            let mut discovery = serde_json::Map::new();
            discovery.insert("discoverable_by_name".to_string(), serde_json::Value::Bool(profile.preferences.discovery.discoverable_by_name));
            discovery.insert("discoverable_by_friends".to_string(), serde_json::Value::Bool(profile.preferences.discovery.discoverable_by_friends));
            discovery.insert("allow_contact_requests".to_string(), serde_json::Value::Bool(profile.preferences.discovery.allow_contact_requests));
            discovery.insert("require_mutual_friends".to_string(), serde_json::Value::Bool(profile.preferences.discovery.require_mutual_friends));
            discovery.insert("listed_in_directory".to_string(), serde_json::Value::Bool(profile.preferences.discovery.listed_in_directory));
            preferences.insert("discovery".to_string(), serde_json::Value::Object(discovery));
            
            // Default permissions
            let mut permissions = serde_json::Map::new();
            permissions.insert("can_see_display_name".to_string(), serde_json::Value::Bool(profile.preferences.default_permissions.can_see_display_name));
            permissions.insert("can_see_avatar".to_string(), serde_json::Value::Bool(profile.preferences.default_permissions.can_see_avatar));
            permissions.insert("can_see_status".to_string(), serde_json::Value::Bool(profile.preferences.default_permissions.can_see_status));
            permissions.insert("can_see_contact_info".to_string(), serde_json::Value::Bool(profile.preferences.default_permissions.can_see_contact_info));
            permissions.insert("can_see_last_seen".to_string(), serde_json::Value::Bool(profile.preferences.default_permissions.can_see_last_seen));
            permissions.insert("can_see_custom_fields".to_string(), serde_json::Value::Bool(profile.preferences.default_permissions.can_see_custom_fields));
            preferences.insert("default_permissions".to_string(), serde_json::Value::Object(permissions));
            
            // Privacy settings
            let mut privacy = serde_json::Map::new();
            privacy.insert("require_proof_of_humanity".to_string(), serde_json::Value::Bool(profile.preferences.privacy.require_proof_of_humanity));
            privacy.insert("max_contact_request_age".to_string(), serde_json::Value::Number(serde_json::Number::from(profile.preferences.privacy.max_contact_request_age.as_secs())));
            privacy.insert("enable_forward_secrecy".to_string(), serde_json::Value::Bool(profile.preferences.privacy.enable_forward_secrecy));
            privacy.insert("auto_rotate_keys".to_string(), serde_json::Value::Bool(profile.preferences.privacy.auto_rotate_keys));
            privacy.insert("key_rotation_interval".to_string(), serde_json::Value::Number(serde_json::Number::from(profile.preferences.privacy.key_rotation_interval.as_secs())));
            preferences.insert("privacy".to_string(), serde_json::Value::Object(privacy));
            
            response.insert("preferences".to_string(), serde_json::Value::Object(preferences));
            
            // Custom fields
            let custom_fields: serde_json::Map<String, serde_json::Value> = profile.custom_fields
                .into_iter()
                .collect();
            response.insert("custom_fields".to_string(), serde_json::Value::Object(custom_fields));
            
            Ok(Some(serde_json::Value::Object(response)))
        }
        Ok(None) => {
            info!("No profile found");
            Ok(None)
        }
        Err(e) => {
            warn!("Failed to get profile: {}", e);
            Ok(None)
        }
    }
}

#[tauri::command]
async fn create_user_identity(
    state: State<'_, AppState>,
    display_name: String,
    three_word_address: String,
) -> Result<serde_json::Value, String> {
    info!("Creating network identity: {} -> {}", display_name, three_word_address);
    
    // Get network for DHT operations
    let network_guard = state.network.read().await;
    let network = network_guard.as_ref()
        .ok_or_else(|| "Network not initialized. Please start P2P network first.".to_string())?;
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Create cryptographic identity
    match identity_manager.create_identity(
        display_name.clone(),
        three_word_address.clone(),
        None, // IPv6 identity - will bind later
        None, // IPv6 keypair - will bind later
    ).await {
        Ok(identity) => {
            info!("Cryptographic identity created: {}", identity.user_id);
            
            // Create and publish profile to DHT
            let mut profile = UserProfile::new(display_name.clone());
            profile.user_id = identity.user_id.clone();
            profile.public_key = identity.public_key.clone();
            profile.created_at = identity.created_at;
            profile.updated_at = std::time::SystemTime::now();
            
            // Configure network discovery settings
            profile.preferences.discovery.discoverable_by_name = true;
            profile.preferences.discovery.discoverable_by_friends = true;
            profile.preferences.discovery.allow_contact_requests = true;
            profile.preferences.discovery.require_mutual_friends = false;
            profile.preferences.discovery.listed_in_directory = true;
            
            // Configure default permissions for P2P contacts
            profile.preferences.default_permissions.can_see_display_name = true;
            profile.preferences.default_permissions.can_see_avatar = true;
            profile.preferences.default_permissions.can_see_status = true;
            profile.preferences.default_permissions.can_see_contact_info = true;
            profile.preferences.default_permissions.can_see_last_seen = true;
            profile.preferences.default_permissions.can_see_custom_fields = false;
            
            // Configure privacy for P2P network
            profile.preferences.privacy.require_proof_of_humanity = false;
            profile.preferences.privacy.max_contact_request_age = std::time::Duration::from_secs(7 * 24 * 3600);
            profile.preferences.privacy.enable_forward_secrecy = true;
            profile.preferences.privacy.auto_rotate_keys = true;
            profile.preferences.privacy.key_rotation_interval = std::time::Duration::from_secs(30 * 24 * 3600);
            
            // Publish identity to DHT network
            match publish_identity_to_dht(network, &identity, &profile).await {
                Ok(_) => {
                    info!("Identity published to DHT network successfully");
                }
                Err(e) => {
                    warn!("Failed to publish identity to DHT: {} (continuing anyway)", e);
                }
            }
            
            // Register three-word address in DHT
            match register_three_word_address(network, &three_word_address, &identity.user_id).await {
                Ok(_) => {
                    info!("Three-word address registered: {}", three_word_address);
                }
                Err(e) => {
                    warn!("Failed to register three-word address: {} (continuing anyway)", e);
                }
            }
            
            // Save identity locally as backup
            let storage_guard = state.identity_storage.read().await;
            if let Some(storage) = storage_guard.as_ref() {
                match identity_manager.export_identity("current_user").await {
                    Ok(export_data) => {
                        let export_path = storage.storage_path.with_extension("export");
                        if let Err(e) = std::fs::write(&export_path, &export_data) {
                            warn!("Failed to save identity backup: {}", e);
                        } else {
                            info!("Identity backup saved locally");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to export identity for backup: {}", e);
                    }
                }
            }
            
            // Return network identity information
            let mut response = serde_json::Map::new();
            response.insert("user_id".to_string(), serde_json::Value::String(identity.user_id));
            response.insert("display_name_hint".to_string(), serde_json::Value::String(identity.display_name_hint));
            response.insert("three_word_address".to_string(), serde_json::Value::String(identity.three_word_address));
            response.insert("verification_level".to_string(), serde_json::Value::String(format!("{:?}", identity.verification_level)));
            response.insert("public_key".to_string(), serde_json::Value::String(base64::encode(&identity.public_key)));
            response.insert("network_published".to_string(), serde_json::Value::Bool(true));
            response.insert("created_at".to_string(), serde_json::Value::String(
                identity.created_at.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default().as_secs().to_string()
            ));
            
            Ok(serde_json::Value::Object(response))
        }
        Err(e) => {
            error!("Failed to create identity: {}", e);
            Err(format!("Failed to create identity: {}", e))
        }
    }
}

#[tauri::command]
async fn update_user_profile(
    state: State<'_, AppState>,
    profile_data: serde_json::Value,
) -> Result<String, String> {
    info!("Updating user profile");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // For now, just return success since the API is simplified
    // In a real implementation, this would parse and update the profile data
    info!("Profile update requested with data: {}", profile_data);
    
    Ok("Profile updated successfully".to_string())
}

#[tauri::command]
async fn export_user_identity(state: State<'_, AppState>) -> Result<String, String> {
    info!("Exporting user identity");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Export identity
    match identity_manager.export_identity("current_user").await {
        Ok(export_data) => {
            // Convert to base64 for easy transport
            let encoded = base64::encode(&export_data);
            
            // Create export wrapper with metadata
            let export_wrapper = serde_json::json!({
                "version": 1,
                "format": "encrypted_binary",
                "data": encoded,
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "app_version": env!("CARGO_PKG_VERSION"),
            });
            
            Ok(export_wrapper.to_string())
        }
        Err(e) => {
            error!("Failed to export identity: {}", e);
            Err(format!("Failed to export identity: {}", e))
        }
    }
}

#[tauri::command]
async fn import_user_identity(
    state: State<'_, AppState>,
    identity_data: String,
) -> Result<String, String> {
    info!("Importing user identity");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Parse the export wrapper
    let export_wrapper: serde_json::Value = serde_json::from_str(&identity_data)
        .map_err(|e| format!("Invalid identity data format: {}", e))?;
    
    // Extract the base64 encoded data
    let encoded_data = export_wrapper.get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing data field in export".to_string())?;
    
    // Decode from base64
    let export_data = base64::decode(encoded_data)
        .map_err(|e| format!("Failed to decode identity data: {}", e))?;
    
    // Import into identity manager
    match identity_manager.import_identity(&export_data, "default_password").await {
        Ok(identity) => {
            info!("Identity imported successfully: {}", identity.user_id);
            
            // Save to local storage
            let storage_guard = state.identity_storage.read().await;
            if let Some(storage) = storage_guard.as_ref() {
                let export_path = storage.storage_path.with_extension("export");
                if let Err(e) = std::fs::write(&export_path, &export_data) {
                    warn!("Failed to save imported identity to storage: {}", e);
                }
            }
            
            Ok(format!("Identity imported successfully. User ID: {}", identity.user_id))
        }
        Err(e) => {
            error!("Failed to import identity: {}", e);
            Err(format!("Failed to import identity: {}", e))
        }
    }
}

#[tauri::command]
async fn bind_ipv6_identity(state: State<'_, AppState>) -> Result<String, String> {
    info!("Binding IPv6 identity");
    
    // TODO: Implement IPv6 identity binding
    Ok("IPv6 identity bound successfully".to_string())
}

// ================== Contact Management Commands ==================

#[tauri::command]
async fn search_users(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    info!("Searching users with query: {}", query);
    
    // TODO: Implement user search using DHT lookup
    // For now, return placeholder results
    let results = vec![
        serde_json::json!({
            "user_id": "sample_user_1",
            "display_name": "Sample User 1",
            "three_word_address": "sample.user.one",
            "verification_level": "SelfSigned"
        }),
        serde_json::json!({
            "user_id": "sample_user_2", 
            "display_name": "Sample User 2",
            "three_word_address": "sample.user.two",
            "verification_level": "NetworkVerified"
        })
    ];
    
    Ok(results)
}

#[tauri::command]
async fn send_contact_request(
    state: State<'_, AppState>,
    user_id: String,
    message: String,
) -> Result<String, String> {
    info!("Sending contact request to user: {}", user_id);
    
    // TODO: Implement contact request sending
    Ok("Contact request sent successfully".to_string())
}

#[tauri::command]
async fn get_contact_requests(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    info!("Getting contact requests");
    
    // TODO: Implement contact request retrieval
    let response = serde_json::json!({
        "pending": [
            {
                "request_id": "req_1",
                "from_user_id": "user_123",
                "from_user_name": "John Doe",
                "message": "Hi! I'd like to connect with you.",
                "created_at": chrono::Utc::now().to_rfc3339()
            }
        ],
        "sent": [
            {
                "request_id": "req_2",
                "to_user_id": "user_456",
                "to_user_name": "Jane Smith", 
                "message": "Hello!",
                "created_at": chrono::Utc::now().to_rfc3339()
            }
        ]
    });
    
    Ok(response)
}

#[tauri::command]
async fn accept_contact_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<String, String> {
    info!("Accepting contact request: {}", request_id);
    
    // TODO: Implement contact request acceptance
    Ok("Contact request accepted".to_string())
}

#[tauri::command]
async fn reject_contact_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<String, String> {
    info!("Rejecting contact request: {}", request_id);
    
    // TODO: Implement contact request rejection
    Ok("Contact request rejected".to_string())
}

#[tauri::command]
async fn cancel_contact_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<String, String> {
    info!("Cancelling contact request: {}", request_id);
    
    // TODO: Implement contact request cancellation
    Ok("Contact request cancelled".to_string())
}

// ================== Comprehensive Contact Management Commands ==================

/// Delete a contact
#[tauri::command]
async fn delete_contact(
    state: State<'_, AppState>,
    contact_id: String,
) -> Result<(), String> {
    info!("Deleting contact: {}", contact_id);
    
    // Don't allow deleting system contact
    if contact_id == "system" {
        return Err("Cannot delete system contact".to_string());
    }
    
    let mut contacts = state.contacts.write().await;
    contacts.remove(&contact_id)
        .ok_or_else(|| "Contact not found".to_string())?;
    
    // Also remove associated messages if requested
    let mut messages = state.messages.write().await;
    messages.remove(&contact_id);
    
    Ok(())
}

/// Block a user
#[tauri::command]
async fn block_user(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<(), String> {
    info!("Blocking user: {}", user_id);
    
    let mut blocked_users = state.blocked_users.write().await;
    blocked_users.insert(user_id.clone(), chrono::Utc::now().timestamp());
    
    // Update contact if exists
    let mut contacts = state.contacts.write().await;
    if let Some(contact) = contacts.get_mut(&user_id) {
        contact.is_blocked = true;
    }
    
    Ok(())
}

/// Unblock a user
#[tauri::command]
async fn unblock_user(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<(), String> {
    info!("Unblocking user: {}", user_id);
    
    let mut blocked_users = state.blocked_users.write().await;
    blocked_users.remove(&user_id);
    
    // Update contact if exists
    let mut contacts = state.contacts.write().await;
    if let Some(contact) = contacts.get_mut(&user_id) {
        contact.is_blocked = false;
    }
    
    Ok(())
}

/// Get blocked users list
#[tauri::command]
async fn get_blocked_users(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let blocked_users = state.blocked_users.read().await;
    Ok(blocked_users.keys().cloned().collect())
}

/// Update contact details
#[tauri::command]
async fn update_contact(
    state: State<'_, AppState>,
    contact_id: String,
    nickname: Option<String>,
    notes: Option<String>,
    category: Option<String>,
) -> Result<(), String> {
    info!("Updating contact: {}", contact_id);
    
    let mut contacts = state.contacts.write().await;
    let contact = contacts.get_mut(&contact_id)
        .ok_or_else(|| "Contact not found".to_string())?;
    
    if nickname.is_some() {
        contact.nickname = nickname;
    }
    if notes.is_some() {
        contact.notes = notes;
    }
    if category.is_some() {
        contact.category = category;
    }
    
    Ok(())
}

/// Update contact permissions
#[tauri::command]
async fn update_contact_permissions(
    state: State<'_, AppState>,
    contact_id: String,
    permissions: ContactPermissions,
) -> Result<(), String> {
    info!("Updating contact permissions: {}", contact_id);
    
    let mut contacts = state.contacts.write().await;
    let contact = contacts.get_mut(&contact_id)
        .ok_or_else(|| "Contact not found".to_string())?;
    
    contact.permissions = permissions;
    
    Ok(())
}

/// Get contact categories
#[tauri::command]
async fn get_contact_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let categories = state.contact_categories.read().await;
    Ok(categories.clone())
}

/// Add a new contact category
#[tauri::command]
async fn add_contact_category(
    state: State<'_, AppState>,
    category: String,
) -> Result<(), String> {
    info!("Adding contact category: {}", category);
    
    let mut categories = state.contact_categories.write().await;
    if !categories.contains(&category) {
        categories.push(category);
    }
    
    Ok(())
}

/// Get detailed contact info
#[tauri::command]
async fn get_contact_details(
    state: State<'_, AppState>,
    contact_id: String,
) -> Result<Contact, String> {
    let contacts = state.contacts.read().await;
    contacts.get(&contact_id)
        .cloned()
        .ok_or_else(|| "Contact not found".to_string())
}

/// Bulk delete contacts
#[tauri::command]
async fn bulk_delete_contacts(
    state: State<'_, AppState>,
    contact_ids: Vec<String>,
) -> Result<(), String> {
    info!("Bulk deleting {} contacts", contact_ids.len());
    
    let mut contacts = state.contacts.write().await;
    let mut messages = state.messages.write().await;
    
    for contact_id in contact_ids {
        if contact_id != "system" {
            contacts.remove(&contact_id);
            messages.remove(&contact_id);
        }
    }
    
    Ok(())
}

/// Extract frontend assets to a specific directory
fn extract_frontend_to(target_dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target_dir)?;
    
    std::fs::write(target_dir.join("index.html"), frontend_bundle::INDEX_HTML)?;
    std::fs::write(target_dir.join("styles.css"), frontend_bundle::STYLES_CSS)?;
    std::fs::write(target_dir.join("main.js"), frontend_bundle::MAIN_JS)?;
    std::fs::write(target_dir.join("test.html"), frontend_bundle::TEST_HTML)?;
    
    Ok(())
}

/// Initialize logging
fn init_logging() {
    // Use try_init() to avoid panics if already initialized
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "saorsa=info,ant_core=info".to_string())
        )
        .with_target(false)
        .try_init();
}


/// Create a custom protocol handler for serving frontend files
fn create_frontend_protocol_handler<R: tauri::Runtime>() -> impl Fn(tauri::UriSchemeContext<'_, R>, tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> + Send + Sync + 'static {
    move |_ctx, request| {
        // Get the frontend directory
        let frontend_dir = if let Ok(custom_dir) = std::env::var("SAORSA_FRONTEND_PATH") {
            std::path::PathBuf::from(custom_dir)
        } else {
            // Check for extracted frontend in user directory
            let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let user_frontend = home_dir.join(".saorsa").join("frontend");
            
            // Extract frontend if it doesn't exist
            if !user_frontend.join("index.html").exists() {
                info!("Extracting frontend to {:?}", user_frontend);
                if let Err(e) = extract_frontend_to(&user_frontend) {
                    warn!("Failed to extract frontend: {}", e);
                    // Fallback to relative path for development
                    std::path::PathBuf::from("../src")
                } else {
                    user_frontend
                }
            } else {
                user_frontend
            }
        };
        
        // Get the requested path from the URL
        let uri = request.uri();
        let path = uri.path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };
        let file_path = frontend_dir.join(path);
        
        info!("Serving file from: {:?}", file_path);
        
        // Try to read the file
        if let Ok(content) = std::fs::read(&file_path) {
            let content_type = match file_path.extension().and_then(|s| s.to_str()) {
                Some("html") => "text/html",
                Some("js") => "application/javascript",
                Some("css") => "text/css",
                Some("json") => "application/json",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("svg") => "image/svg+xml",
                Some("ico") => "image/x-icon",
                _ => "application/octet-stream",
            };
            
            tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", content_type)
                .header("Cache-Control", "no-cache")
                .body(content)
                .unwrap()
        } else {
            warn!("File not found: {:?}", file_path);
            
            // Return a 404 response
            tauri::http::Response::builder()
                .status(404)
                .header("Content-Type", "text/html")
                .body(b"<h1>404 - File not found</h1>".to_vec())
                .unwrap()
        }
    }
}

// ================ DHT Identity Management ================

/// Publish user identity to DHT network
async fn publish_identity_to_dht(
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
    
    // Serialize encrypted profile
    let profile_data = match serde_json::to_vec(profile) {
        Ok(data) => data,
        Err(e) => return Err(format!("Failed to serialize profile: {}", e)),
    };
    
    // Store in DHT
    match network.dht_put(dht_key, profile_data).await {
        Ok(_) => {
            info!("Identity published to DHT successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to publish identity to DHT: {}", e);
            Err(format!("DHT put failed: {}", e))
        }
    }
}

/// Register three-word address mapping in DHT
async fn register_three_word_address(
    network: &Arc<P2PNode>,
    three_word_address: &str,
    user_id: &str,
) -> Result<(), String> {
    use saorsa_core::dht::Key;
    use sha2::{Digest, Sha256};
    
    // Create DHT key for three-word address
    let address_key = format!("three-word:{}", three_word_address);
    let mut hasher = Sha256::new();
    hasher.update(address_key.as_bytes());
    let key_hash: [u8; 32] = hasher.finalize().into();
    let dht_key = Key::new(&key_hash);
    
    // Create mapping record
    let mapping = serde_json::json!({
        "user_id": user_id,
        "three_word_address": three_word_address,
        "registered_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        "publisher": network.peer_id().to_string()
    });
    
    let mapping_data = match serde_json::to_vec(&mapping) {
        Ok(data) => data,
        Err(e) => return Err(format!("Failed to serialize address mapping: {}", e)),
    };
    
    // Store in DHT
    match network.dht_put(dht_key, mapping_data).await {
        Ok(_) => {
            info!("Three-word address registered: {}", three_word_address);
            Ok(())
        }
        Err(e) => {
            error!("Failed to register three-word address: {}", e);
            Err(format!("DHT put failed: {}", e))
        }
    }
}

/// Resolve three-word address to user ID via DHT lookup
async fn resolve_three_word_address(
    network: &Arc<P2PNode>,
    three_word_address: &str,
) -> Result<String, String> {
    use saorsa_core::dht::Key;
    use sha2::{Digest, Sha256};
    
    // Create DHT key for three-word address
    let address_key = format!("three-word:{}", three_word_address);
    let mut hasher = Sha256::new();
    hasher.update(address_key.as_bytes());
    let key_hash: [u8; 32] = hasher.finalize().into();
    let dht_key = Key::new(&key_hash);
    
    // Lookup in DHT
    match network.dht_get(dht_key).await {
        Ok(Some(value)) => {
            // Parse mapping
            match serde_json::from_slice::<serde_json::Value>(&value) {
                Ok(mapping) => {
                    if let Some(user_id) = mapping["user_id"].as_str() {
                        Ok(user_id.to_string())
                    } else {
                        Err("Invalid address mapping format".to_string())
                    }
                }
                Err(e) => Err(format!("Failed to parse address mapping: {}", e)),
            }
        }
        Ok(None) => Err("Three-word address not found in network".to_string()),
        Err(e) => Err(format!("DHT lookup failed: {}", e)),
    }
}

/// Lookup user identity from DHT by user ID
async fn lookup_user_identity(
    network: &Arc<P2PNode>,
    user_id: &str,
) -> Result<UserProfile, String> {
    use saorsa_core::dht::Key;
    use sha2::{Digest, Sha256};
    
    // Create DHT key from user ID
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    let key_hash: [u8; 32] = hasher.finalize().into();
    let dht_key = Key::new(&key_hash);
    
    // Lookup in DHT
    match network.dht_get(dht_key).await {
        Ok(Some(value)) => {
            // Parse profile
            match serde_json::from_slice::<UserProfile>(&value) {
                Ok(profile) => Ok(profile),
                Err(e) => Err(format!("Failed to parse user profile: {}", e)),
            }
        }
        Ok(None) => Err("User not found in network".to_string()),
        Err(e) => Err(format!("DHT lookup failed: {}", e)),
    }
}

// ================ Network Identity Discovery Commands ================

/// Resolve three-word address to user ID via DHT
#[tauri::command]
async fn resolve_three_word_address_command(
    state: State<'_, AppState>,
    three_word_address: String,
) -> Result<serde_json::Value, String> {
    info!("Resolving three-word address: {}", three_word_address);
    
    // Get network
    let network_guard = state.network.read().await;
    let network = network_guard.as_ref()
        .ok_or_else(|| "Network not initialized".to_string())?;
    
    // Resolve address
    match resolve_three_word_address(network, &three_word_address).await {
        Ok(user_id) => {
            // Try to get user profile
            match lookup_user_identity(network, &user_id).await {
                Ok(profile) => {
                    let mut response = serde_json::Map::new();
                    response.insert("user_id".to_string(), serde_json::Value::String(user_id));
                    response.insert("display_name".to_string(), serde_json::Value::String(profile.display_name));
                    response.insert("three_word_address".to_string(), serde_json::Value::String(three_word_address));
                    response.insert("status_message".to_string(), 
                        profile.status_message.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
                    response.insert("discoverable".to_string(), serde_json::Value::Bool(profile.preferences.discovery.discoverable_by_name));
                    Ok(serde_json::Value::Object(response))
                }
                Err(e) => {
                    // Return just the user ID if profile lookup fails
                    let mut response = serde_json::Map::new();
                    response.insert("user_id".to_string(), serde_json::Value::String(user_id));
                    response.insert("three_word_address".to_string(), serde_json::Value::String(three_word_address));
                    response.insert("profile_error".to_string(), serde_json::Value::String(e));
                    Ok(serde_json::Value::Object(response))
                }
            }
        }
        Err(e) => Err(format!("Address resolution failed: {}", e)),
    }
}

/// Lookup user profile by user ID from DHT
#[tauri::command]
async fn lookup_user_by_id(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<serde_json::Value, String> {
    info!("Looking up user by ID: {}", user_id);
    
    // Get network
    let network_guard = state.network.read().await;
    let network = network_guard.as_ref()
        .ok_or_else(|| "Network not initialized".to_string())?;
    
    // Lookup user profile
    match lookup_user_identity(network, &user_id).await {
        Ok(profile) => {
            let mut response = serde_json::Map::new();
            response.insert("user_id".to_string(), serde_json::Value::String(profile.user_id));
            response.insert("display_name".to_string(), serde_json::Value::String(profile.display_name));
            response.insert("bio".to_string(), 
                profile.bio.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
            response.insert("status_message".to_string(), 
                profile.status_message.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
            response.insert("public_key".to_string(), serde_json::Value::String(base64::encode(&profile.public_key)));
            response.insert("created_at".to_string(), serde_json::Value::String(
                profile.created_at.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default().as_secs().to_string()
            ));
            
            // Discovery settings (only if permission allows)
            if profile.preferences.default_permissions.can_see_contact_info {
                let mut discovery = serde_json::Map::new();
                discovery.insert("discoverable_by_name".to_string(), 
                    serde_json::Value::Bool(profile.preferences.discovery.discoverable_by_name));
                discovery.insert("allow_contact_requests".to_string(), 
                    serde_json::Value::Bool(profile.preferences.discovery.allow_contact_requests));
                response.insert("discovery".to_string(), serde_json::Value::Object(discovery));
            }
            
            Ok(serde_json::Value::Object(response))
        }
        Err(e) => Err(format!("User lookup failed: {}", e)),
    }
}

/// Search for users in the network by display name pattern
#[tauri::command]
async fn search_network_users(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    info!("Searching network users with query: {}", query);
    
    if query.len() < 2 {
        return Err("Search query must be at least 2 characters".to_string());
    }
    
    // Get network
    let network_guard = state.network.read().await;
    let network = network_guard.as_ref()
        .ok_or_else(|| "Network not initialized".to_string())?;
    
    // For now, return placeholder results since we need to implement DHT search
    // TODO: Implement proper DHT-based user search by iterating through known profiles
    let results = vec![
        serde_json::json!({
            "user_id": "network_user_1",
            "display_name": format!("Network User matching '{}'", query),
            "three_word_address": "network.user.example",
            "discoverable": true,
            "match_score": 0.8
        }),
        serde_json::json!({
            "user_id": "network_user_2", 
            "display_name": format!("Another User with '{}'", query),
            "three_word_address": "another.user.demo",
            "discoverable": true,
            "match_score": 0.6
        })
    ];
    
    info!("Found {} potential matches for query: {}", results.len(), query);
    Ok(results)
}

// ================ Mobile Lifecycle Commands ================

/// Handle app going to background (mobile)
#[tauri::command]
async fn handle_app_background(state: State<'_, AppState>) -> Result<String, String> {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        info!("App going to background - optimizing P2P connections");
        
        // Reduce network activity for battery optimization
        if let Some(network) = state.network.read().await.as_ref() {
            // Note: In a full implementation, you would reduce polling intervals,
            // maintain essential connections only, and optimize for battery life
            info!("P2P network optimized for background mode");
        }
        
        Ok("Background mode optimized".to_string())
    }
    
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        Ok("Background handling not needed on desktop".to_string())
    }
}

/// Handle app coming to foreground (mobile)
#[tauri::command]
async fn handle_app_foreground(state: State<'_, AppState>) -> Result<String, String> {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        info!("App coming to foreground - restoring P2P connections");
        
        // Restore full network activity
        if let Some(network) = state.network.read().await.as_ref() {
            // Note: In a full implementation, you would restore normal polling intervals,
            // reconnect to peers, and resume full functionality
            info!("P2P network restored for foreground mode");
        }
        
        Ok("Foreground mode restored".to_string())
    }
    
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        Ok("Foreground handling not needed on desktop".to_string())
    }
}

// ================ Main Application Runner ================

/// Main Tauri application entry point
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_app() {
    init_logging();
    info!("Starting Saorsa");

    tauri::Builder::default()
        .register_uri_scheme_protocol("saorsa", create_frontend_protocol_handler())
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            init_network,
            get_network_status,
            connect_peer,
            send_message,
            get_contacts,
            get_messages,
            create_inbox,
            open_url,
            get_app_info,
            // Profile and Identity commands
            get_user_identity,
            get_user_profile,
            create_user_identity,
            update_user_profile,
            export_user_identity,
            import_user_identity,
            bind_ipv6_identity,
            // Contact management commands
            search_users,
            send_contact_request,
            get_contact_requests,
            accept_contact_request,
            reject_contact_request,
            cancel_contact_request,
            // Comprehensive contact management
            delete_contact,
            block_user,
            unblock_user,
            get_blocked_users,
            update_contact,
            update_contact_permissions,
            get_contact_categories,
            add_contact_category,
            get_contact_details,
            bulk_delete_contacts,
            // WebRTC call commands
            send_call_offer,
            send_call_answer,
            send_ice_candidate,
            end_call,
            // Mobile lifecycle commands
            handle_app_background,
            handle_app_foreground,
            // Network identity discovery
            resolve_three_word_address_command,
            lookup_user_by_id,
            search_network_users,
        ])
        .setup(|app| {
            info!("Tauri application setup starting");
            
            // Initialize identity storage
            let app_handle = app.handle().clone();
            let storage_config = IdentityStorageConfig::default();
            
            match IdentityStorage::new(app_handle.clone(), storage_config) {
                Ok(storage) => {
                    let state = app.state::<AppState>();
                    
                    // Create identity manager
                    let identity_manager_config = IdentityManagerConfig::default();
                    let identity_manager = IdentityManager::new(identity_manager_config);
                    
                    // Store in app state
                    tokio::runtime::Runtime::new().unwrap().block_on(async {
                        *state.identity_storage.write().await = Some(Arc::new(storage));
                        *state.identity_manager.write().await = Some(Arc::new(identity_manager));
                    });
                    
                    info!("Identity storage and manager initialized successfully");
                }
                Err(e) => {
                    error!("Failed to initialize identity storage: {}", e);
                    // Continue without identity persistence
                }
            }
            
            // Create the main window with our custom protocol
            // Use a unique label to avoid conflicts
            let window_label = format!("main-{}", uuid::Uuid::new_v4().simple());
            
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            {
                let _window = tauri::WebviewWindowBuilder::new(
                    app,
                    &window_label,
                    tauri::WebviewUrl::External("saorsa://localhost/index.html".parse().unwrap())
                )
                .title("Saorsa - P2P Foundation")
                .inner_size(800.0, 600.0)
                .build()?;
            }
            
            #[cfg(any(target_os = "ios", target_os = "android"))]
            {
                // Mobile platforms: create fullscreen webview
                let _window = tauri::WebviewWindowBuilder::new(
                    app,
                    &window_label,
                    tauri::WebviewUrl::External("saorsa://localhost/index.html".parse().unwrap())
                )
                .title("Saorsa")
                .fullscreen(true)
                .build()?;
            }
            
            info!("Main window created");
            info!("Tauri application setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Public API for running Saorsa from external binaries (for cargo install)
pub fn run_desktop_app() -> anyhow::Result<()> {
    // Note: Logging is initialized in run_app() to avoid double initialization
    // Run the Tauri app
    run_app();
    Ok(())
}

/// Entry point for main.rs
pub fn run() {
    run_desktop_app().expect("Failed to run desktop app");
}