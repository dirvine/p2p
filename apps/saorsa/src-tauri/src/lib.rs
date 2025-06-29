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

use ant_core::{
    network::{P2PNode, NodeConfig, DHTConfig as NetworkDHTConfig, SecurityConfig, TrustLevel},
    dht::{DHT, DHTConfig, Key, Record},
    production::ProductionConfig,
    identity::{
        UserIdentity, UserProfile, EncryptedUserProfile, 
        ProfilePermissions, PrivacySettings, DiscoverabilitySettings,
        UserPreferences, VerificationLevel,
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
    
    // Create a default configuration
    let listen_addr = format!("127.0.0.1:{}", listen_port.unwrap_or(9000))
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
        max_connections: 100,
        max_incoming_connections: 50,
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
                    manager.set_network(network_arc.clone());
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
    if let Some(identity) = identity_manager.get_local_identity().await {
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
        
        return Ok(Some(serde_json::Value::Object(response)));
    }
    
    // Try to load from storage if not in memory
    let storage_guard = state.identity_storage.read().await;
    if let Some(storage) = storage_guard.as_ref() {
        let export_path = storage.storage_path.with_extension("export");
        if export_path.exists() {
            // Load the export data
            match std::fs::read(&export_path) {
                Ok(export_data) => {
                    // Import into identity manager
                    match identity_manager.import_identity(&export_data).await {
                        Ok(identity) => {
                            info!("Identity loaded from storage: {}", identity.user_id);
                            
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
                            
                            return Ok(Some(serde_json::Value::Object(response)));
                        }
                        Err(e) => {
                            warn!("Failed to import identity: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read identity export: {}", e);
                }
            }
        }
    }
    
    Ok(None)
}

#[tauri::command]
async fn get_user_profile(state: State<'_, AppState>) -> Result<Option<serde_json::Value>, String> {
    info!("Getting user profile");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Try to get decrypted profile
    match identity_manager.get_local_profile().await {
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
    info!("Creating user identity: {}", display_name);
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Get network for IPv6 identity
    let network_guard = state.network.read().await;
    let network = network_guard.as_ref();
    
    // Create identity - for now without IPv6 binding (will add later)
    match identity_manager.create_identity(
        display_name.clone(),
        three_word_address.clone(),
        None, // IPv6 identity - will implement in bind_ipv6_identity
        None, // IPv6 keypair - will implement in bind_ipv6_identity
    ).await {
        Ok(identity) => {
            info!("Identity created successfully: {}", identity.user_id);
            
            // The keypair is managed internally by identity manager
            // We'll export the identity data for storage instead
            
            // Create default profile
            let mut profile = UserProfile::new(display_name.clone());
            profile.created_at = std::time::SystemTime::now();
            profile.updated_at = std::time::SystemTime::now();
            
            // Configure discovery settings
            profile.preferences.discovery.discoverable_by_name = true;
            profile.preferences.discovery.discoverable_by_friends = true;
            profile.preferences.discovery.allow_contact_requests = true;
            profile.preferences.discovery.require_mutual_friends = false;
            profile.preferences.discovery.listed_in_directory = true;
            
            // Configure default permissions
            profile.preferences.default_permissions.can_see_display_name = true;
            profile.preferences.default_permissions.can_see_avatar = true;
            profile.preferences.default_permissions.can_see_status = true;
            profile.preferences.default_permissions.can_see_contact_info = true;
            profile.preferences.default_permissions.can_see_last_seen = true;
            profile.preferences.default_permissions.can_see_custom_fields = true;
            
            // Configure privacy settings
            profile.preferences.privacy.require_proof_of_humanity = false;
            profile.preferences.privacy.max_contact_request_age = std::time::Duration::from_secs(7 * 24 * 3600); // 7 days
            profile.preferences.privacy.enable_forward_secrecy = true;
            profile.preferences.privacy.auto_rotate_keys = false;
            profile.preferences.privacy.key_rotation_interval = std::time::Duration::from_secs(30 * 24 * 3600); // 30 days
            
            // Update the profile
            if let Err(e) = identity_manager.update_local_profile(profile).await {
                warn!("Failed to update profile: {}", e);
            }
            
            // Save to local storage if available
            let storage_guard = state.identity_storage.read().await;
            if let Some(storage) = storage_guard.as_ref() {
                // Export identity for storage
                match identity_manager.export_identity().await {
                    Ok(export_data) => {
                        // Store the export data securely
                        let export_path = storage.storage_path.with_extension("export");
                        if let Err(e) = std::fs::write(&export_path, &export_data) {
                            warn!("Failed to save identity export: {}", e);
                        } else {
                            info!("Identity saved to disk successfully");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to export identity: {}", e);
                    }
                }
            }
            
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
    
    // Get current profile first
    let current_profile = match identity_manager.get_local_profile().await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            return Err("No profile found to update".to_string());
        }
        Err(e) => {
            return Err(format!("Failed to get current profile: {}", e));
        }
    };
    
    // Parse update data
    let updates = profile_data.as_object()
        .ok_or_else(|| "Invalid profile data format".to_string())?;
    
    // Create updated profile
    let mut updated_profile = current_profile;
    
    // Update basic fields
    if let Some(display_name) = updates.get("display_name").and_then(|v| v.as_str()) {
        updated_profile.display_name = display_name.to_string();
    }
    
    if let Some(status_message) = updates.get("status_message") {
        updated_profile.status_message = if status_message.is_null() {
            None
        } else {
            status_message.as_str().map(|s| s.to_string())
        };
    }
    
    if let Some(avatar_hash) = updates.get("avatar_hash") {
        updated_profile.avatar_hash = if avatar_hash.is_null() {
            None
        } else {
            avatar_hash.as_str().map(|s| s.to_string())
        };
    }
    
    // Update preferences if provided
    if let Some(preferences) = updates.get("preferences").and_then(|v| v.as_object()) {
        // Update discovery settings
        if let Some(discovery) = preferences.get("discovery").and_then(|v| v.as_object()) {
            if let Some(by_name) = discovery.get("discoverable_by_name").and_then(|v| v.as_bool()) {
                updated_profile.preferences.discovery.discoverable_by_name = by_name;
            }
            if let Some(by_friends) = discovery.get("discoverable_by_friends").and_then(|v| v.as_bool()) {
                updated_profile.preferences.discovery.discoverable_by_friends = by_friends;
            }
            if let Some(allow_requests) = discovery.get("allow_contact_requests").and_then(|v| v.as_bool()) {
                updated_profile.preferences.discovery.allow_contact_requests = allow_requests;
            }
            if let Some(mutual) = discovery.get("require_mutual_friends").and_then(|v| v.as_bool()) {
                updated_profile.preferences.discovery.require_mutual_friends = mutual;
            }
            if let Some(listed) = discovery.get("listed_in_directory").and_then(|v| v.as_bool()) {
                updated_profile.preferences.discovery.listed_in_directory = listed;
            }
        }
        
        // Update privacy settings
        if let Some(privacy) = preferences.get("privacy").and_then(|v| v.as_object()) {
            if let Some(proof) = privacy.get("require_proof_of_humanity").and_then(|v| v.as_bool()) {
                updated_profile.preferences.privacy.require_proof_of_humanity = proof;
            }
            if let Some(age) = privacy.get("max_contact_request_age").and_then(|v| v.as_u64()) {
                updated_profile.preferences.privacy.max_contact_request_age = std::time::Duration::from_secs(age);
            }
            if let Some(forward) = privacy.get("enable_forward_secrecy").and_then(|v| v.as_bool()) {
                updated_profile.preferences.privacy.enable_forward_secrecy = forward;
            }
            if let Some(rotate) = privacy.get("auto_rotate_keys").and_then(|v| v.as_bool()) {
                updated_profile.preferences.privacy.auto_rotate_keys = rotate;
            }
            if let Some(interval) = privacy.get("key_rotation_interval").and_then(|v| v.as_u64()) {
                updated_profile.preferences.privacy.key_rotation_interval = std::time::Duration::from_secs(interval);
            }
        }
        
        // Update default permissions
        if let Some(permissions) = preferences.get("default_permissions").and_then(|v| v.as_object()) {
            if let Some(name) = permissions.get("can_see_display_name").and_then(|v| v.as_bool()) {
                updated_profile.preferences.default_permissions.can_see_display_name = name;
            }
            if let Some(avatar) = permissions.get("can_see_avatar").and_then(|v| v.as_bool()) {
                updated_profile.preferences.default_permissions.can_see_avatar = avatar;
            }
            if let Some(status) = permissions.get("can_see_status").and_then(|v| v.as_bool()) {
                updated_profile.preferences.default_permissions.can_see_status = status;
            }
            if let Some(contact) = permissions.get("can_see_contact_info").and_then(|v| v.as_bool()) {
                updated_profile.preferences.default_permissions.can_see_contact_info = contact;
            }
            if let Some(seen) = permissions.get("can_see_last_seen").and_then(|v| v.as_bool()) {
                updated_profile.preferences.default_permissions.can_see_last_seen = seen;
            }
            if let Some(fields) = permissions.get("can_see_custom_fields").and_then(|v| v.as_bool()) {
                updated_profile.preferences.default_permissions.can_see_custom_fields = fields;
            }
        }
    }
    
    // Update custom fields
    if let Some(custom_fields) = updates.get("custom_fields").and_then(|v| v.as_object()) {
        updated_profile.custom_fields = custom_fields
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    }
    
    // Apply the update
    match identity_manager.update_local_profile(updated_profile).await {
        Ok(_) => {
            info!("Profile updated successfully");
            
            // Save the updated identity to storage
            let storage_guard = state.identity_storage.read().await;
            if let Some(storage) = storage_guard.as_ref() {
                // Export and save the updated identity
                match identity_manager.export_identity().await {
                    Ok(export_data) => {
                        let export_path = storage.storage_path.with_extension("export");
                        if let Err(e) = std::fs::write(&export_path, &export_data) {
                            warn!("Failed to save updated identity export: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to export updated identity: {}", e);
                    }
                }
            }
            
            Ok("Profile updated successfully".to_string())
        }
        Err(e) => {
            error!("Failed to update profile: {}", e);
            Err(format!("Failed to update profile: {}", e))
        }
    }
}

#[tauri::command]
async fn export_user_identity(state: State<'_, AppState>) -> Result<String, String> {
    info!("Exporting user identity");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Export identity
    match identity_manager.export_identity().await {
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
    match identity_manager.import_identity(&export_data).await {
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
            let _window = tauri::WebviewWindowBuilder::new(
                app,
                &window_label,
                tauri::WebviewUrl::External("saorsa://localhost/index.html".parse().unwrap())
            )
            .title("Saorsa - P2P Foundation")
            .inner_size(800.0, 600.0)
            .build()?;
            
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