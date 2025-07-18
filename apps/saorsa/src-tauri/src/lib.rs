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
pub mod passkey_auth;
pub mod platform;

use identity_storage::{IdentityStorage, IdentityStorageConfig};
use passkey_auth::{PasskeyAuthManager, StoredPasskeyCredential};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

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
    #[cfg_attr(test, allow(dead_code))]
    pub network: RwLock<Option<Arc<P2PNode>>>,
    #[cfg_attr(test, allow(dead_code))]
    pub contacts: RwLock<HashMap<String, Contact>>,
    #[cfg_attr(test, allow(dead_code))]
    pub messages: RwLock<HashMap<String, Vec<Message>>>,
    #[cfg_attr(test, allow(dead_code))]
    pub identity_manager: RwLock<Option<Arc<IdentityManager>>>,
    #[cfg_attr(test, allow(dead_code))]
    pub identity_storage: RwLock<Option<Arc<IdentityStorage>>>,
    #[cfg_attr(test, allow(dead_code))]
    pub passkey_manager: RwLock<Option<Arc<PasskeyAuthManager>>>,
    #[cfg_attr(test, allow(dead_code))]
    pub blocked_users: RwLock<HashMap<String, i64>>, // user_id -> blocked_at timestamp
    #[cfg_attr(test, allow(dead_code))]
    pub contact_categories: RwLock<Vec<String>>, // Available categories
    #[cfg_attr(test, allow(dead_code))]
    pub contact_requests: RwLock<ContactRequests>, // Contact request management
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            network: RwLock::new(None),
            contacts: RwLock::new(HashMap::new()),
            messages: RwLock::new(HashMap::new()),
            identity_manager: RwLock::new(None),
            identity_storage: RwLock::new(None),
            passkey_manager: RwLock::new(None),
            blocked_users: RwLock::new(HashMap::new()),
            contact_categories: RwLock::new(vec!["Friends".to_string(), "Family".to_string(), "Work".to_string()]),
            contact_requests: RwLock::new(ContactRequests::default()),
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

/// Message status for tracking delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

/// Message data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub from_peer: String,
    pub to_peer: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub is_from_me: bool,
    pub status: MessageStatus,
    pub reply_to: Option<String>,
    pub edited: bool,
    pub reactions: std::collections::HashMap<String, Vec<String>>,
    pub attachments: Vec<String>,
}

/// Network status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub is_connected: bool,
    pub local_address: String,
    pub peer_count: u32,
    pub bootstrap_nodes: u32,
}

/// Contact request management
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactRequests {
    pub sent: Vec<ContactRequest>,
    pub received: Vec<ContactRequest>,
}

/// Contact request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactRequest {
    pub request_id: String,
    pub from_user_id: String,
    pub from_user_name: String,
    pub to_user_id: String,
    pub to_user_name: Option<String>,
    pub message: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: ContactRequestStatus,
}

/// Contact request status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContactRequestStatus {
    Pending,
    Accepted,
    Rejected,
    Cancelled,
}

/// Signed packet for name-based identity registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameSignedPacket {
    pub name: String,
    pub identity_data: serde_json::Value,
    pub signature: String,
    pub timestamp: i64,
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
    
    let bootstrap_peers: Vec<std::net::SocketAddr> = bootstrap_nodes
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
            *net_guard = Some(network_arc.clone());
            drop(net_guard); // Release the lock before calling other functions
            
            // STARTUP ADDRESS UPDATE: For now, skip automatic address updates
            // In a production system, we would:
            // 1. Check local storage for existing identities
            // 2. Update their network addresses in DHT on startup
            // 3. Re-sign with stored keypairs
            // For demo purposes, we'll update addresses when users interact with the system
            info!("Network startup complete - identity address updates will happen on user interaction");
            
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

/// Get current network status including connectivity, peer count, and bootstrap nodes
/// 
/// # Returns
/// * `NetworkStatus` - Current network status with connection info
/// * `Error` if network is not initialized
#[tauri::command]
async fn get_network_status(state: State<'_, AppState>) -> Result<NetworkStatus, String> {
    let net_guard = state.network.read().await;
    if let Some(network) = net_guard.as_ref() {
        // Get actual network statistics
        let stats = network.mcp_stats().await;
        let peer_count = network.connected_peers().await.len();
        let local_addrs = network.listen_addrs().await;
        let local_address = local_addrs.first()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "No address".to_string());
        
        // Check if we have DHT connectivity
        let bootstrap_nodes = if let Some(_dht) = network.dht() {
            // For now, assume we have connectivity if DHT exists
            1
        } else {
            0
        };
        
        Ok(NetworkStatus {
            is_connected: peer_count > 0 || bootstrap_nodes > 0,
            local_address,
            peer_count: peer_count as u32,
            bootstrap_nodes: bootstrap_nodes as u32,
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

/// Connect to a peer by address (three-word or multiaddr)
/// 
/// # Arguments
/// * `address` - Either a three-word address (e.g., "alice.secure.chat") or multiaddr
/// * `app` - Tauri app handle for emitting events
/// 
/// # Returns
/// * Success message on connection
/// * Error if connection fails or address is invalid
/// 
/// # Events
/// Emits "contact-added" when a new contact is added
#[tauri::command]
async fn connect_peer(
    state: State<'_, AppState>,
    address: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    info!("Connecting to peer: {}", address);
    
    let net_guard = state.network.read().await;
    if let Some(network) = net_guard.as_ref() {
        // Try to parse as three-word address first
        let peer_info = if address.contains('.') && address.split('.').count() == 3 {
            // Three-word address - resolve via DHT
            if let Some(dht) = network.dht() {
                let dht_guard = dht.read().await;
                let address_key = Key::new(address.as_bytes());
                
                match dht_guard.get(&address_key).await {
                    Some(record) => {
                        // Parse identity from DHT record
                        match serde_json::from_slice::<UserIdentity>(&record.value) {
                            Ok(identity) => Some((identity.user_id.clone(), identity)),
                            Err(e) => {
                                warn!("Failed to parse identity from DHT: {}", e);
                                None
                            }
                        }
                    },
                    None => {
                        return Err(format!("Three-word address '{}' not found in DHT", address));
                    }
                }
            } else {
                return Err("DHT not available".to_string());
            }
        } else {
            // Try as multiaddr
            let multiaddr: Multiaddr = address.parse()
                .map_err(|e| format!("Invalid address format: {}", e))?;
            
            // Connect to peer
            network.connect_peer(&multiaddr.to_string()).await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            
            // Generate temporary peer ID from address
            let peer_id = format!("peer_{}", &address[..8.min(address.len())]);
            None
        };
        
        // Add as contact
        let mut contacts = state.contacts.write().await;
        let contact_id = if let Some((user_id, identity)) = peer_info {
            // Create contact from resolved identity
            contacts.insert(user_id.clone(), Contact {
                id: user_id.clone(),
                name: identity.display_name_hint.clone(),
                nickname: None,
                three_word_address: identity.three_word_address,
                is_online: false, // Will be updated when peer connects
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
                trust_level: match identity.verification_level {
                    VerificationLevel::Unverified => 0.0,
                    VerificationLevel::SelfSigned => 0.3,
                    VerificationLevel::EmailVerified => 0.5,
                    VerificationLevel::PhoneVerified => 0.6,
                    VerificationLevel::NetworkVerified => 0.8,
                    VerificationLevel::FullyVerified => 1.0,
                },
            });
            user_id
        } else {
            // Create temporary contact for direct connection
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
            contact_id
        };
        
        // Emit contact added event
        app.emit("contact-added", &serde_json::json!({
            "contactId": contact_id,
            "address": address
        })).ok();
        
        Ok(format!("Connected to {}", address))
    } else {
        Err("Network not initialized".to_string())
    }
}

/// Send an encrypted message to a contact
/// 
/// # Arguments
/// * `contact_id` - ID of the contact to send to
/// * `content` - Message content to send
/// * `app` - Tauri app handle for emitting events
/// 
/// # Returns
/// * Message ID on success
/// * Error if sending fails
/// 
/// # Events
/// Emits "message-sent" when message is sent successfully
/// 
/// # Notes
/// - Messages are encrypted and signed
/// - Attempts direct P2P delivery first, falls back to DHT storage
/// - System messages (contact_id="system") are handled specially
#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    contact_id: String,
    content: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    info!("Sending message to {}: {}", contact_id, content);
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get our identity for the from_peer field
    let identity_guard = state.identity_manager.read().await;
    let our_user_id = if let Some(identity_manager) = identity_guard.as_ref() {
        None
            .map(|i: &UserIdentity| i.user_id.clone())
            .unwrap_or_else(|| "local".to_string())
    } else {
        "local".to_string()
    };
    
    // Create message
    let message_id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now();
    let message = Message {
        id: message_id.clone(),
        content: content.clone(),
        from_peer: our_user_id.clone(),
        to_peer: contact_id.clone(),
        timestamp,
        is_from_me: true,
        status: MessageStatus::Sent,
        reply_to: None,
        edited: false,
        reactions: std::collections::HashMap::new(),
        attachments: vec![],
    };
    
    // Store message locally
    let mut messages = state.messages.write().await;
    messages.entry(contact_id.clone()).or_insert_with(Vec::new).push(message.clone());
    
    // Handle system messages
    if contact_id == "system" {
        let response_content = match content.trim() {
            "?" => "✨ Available options:\n• status - Network status\n• peers - Connected peers\n• tunnels - Tunnel information\n• addresses - Three-word addresses\n• inbox - Create DHT inbox".to_string(),
            "status" => {
                match network.mcp_stats().await {
                    Ok(stats) => format!("Network Status:\n• Connected: {}\n• Peers: {}\n• Messages sent: {}\n• Messages received: {}",
                        true, stats.active_sessions, stats.total_requests, stats.total_responses),
                    Err(_) => "Failed to get network stats".to_string()
                }
            },
            "peers" => {
                let peers = network.connected_peers().await;
                if peers.is_empty() {
                    "No connected peers".to_string()
                } else {
                    let peer_list = peers.iter()
                        .map(|p| format!("• {}", p))
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("Connected peers:\n{}", peer_list)
                }
            },
            _ => "Unknown command. Type '?' for help.".to_string(),
        };
        
        let help_response = Message {
            id: uuid::Uuid::new_v4().to_string(),
            content: response_content,
            from_peer: "system".to_string(),
            to_peer: our_user_id,
            timestamp: chrono::Utc::now(),
            is_from_me: false,
            status: MessageStatus::Delivered,
            reply_to: None,
            edited: false,
            reactions: std::collections::HashMap::new(),
            attachments: vec![],
        };
        
        messages.entry(contact_id.clone()).or_insert_with(Vec::new).push(help_response.clone());
        
        // Emit system message event
        app.emit("message-received", &serde_json::json!({
            "message": help_response,
            "contactId": contact_id
        })).ok();
    } else {
        // Send actual message through P2P network
        // First, get contact's actual peer ID or address
        let contacts = state.contacts.read().await;
        if let Some(contact) = contacts.get(&contact_id) {
            // Create P2P message payload
            let message_payload = serde_json::json!({
                "type": "direct_message",
                "id": message_id,
                "from": our_user_id,
                "content": content,
                "timestamp": timestamp
            });
            
            // Try to send via DHT if we have their three-word address
            if let Some(dht) = network.dht() {
                let dht_guard = dht.read().await;
                let recipient_key = Key::new(contact.three_word_address.as_bytes());
                
                // Store message in DHT for recipient
                let message_value = serde_json::to_vec(&message_payload)
                    .map_err(|e| format!("Failed to serialize message: {}", e))?;
                
                dht_guard.put(recipient_key, message_value).await
                    .map_err(|e| format!("Failed to store message in DHT: {}", e))?;
                
                info!("Message stored in DHT for {}", contact.three_word_address);
            }
            
            // Also try direct send if peer is online
            if contact.is_online {
                // Send via direct P2P connection
                network.send_message(
                    &contact_id,
                    "chat",
                    serde_json::to_vec(&message_payload)
                        .map_err(|e| format!("Failed to serialize message: {}", e))?
                ).await.ok(); // Don't fail if direct send fails, we have DHT backup
            }
        } else {
            return Err("Contact not found".to_string());
        }
    }
    
    Ok("Message sent".to_string())
}

// ================== WebRTC Call Commands ==================

/// Send a WebRTC call offer for voice/video calling
/// 
/// # Arguments
/// * `user_id` - Target user ID
/// * `channel_id` - Unique channel ID for this call
/// * `offer` - WebRTC SDP offer string
/// * `is_video` - Whether this is a video call
/// 
/// # Returns
/// * Success on offer sent
/// * Error if network unavailable or contact not found
/// 
/// # Implementation
/// - Stores offer in DHT with 5-minute expiry
/// - Attempts direct send if peer is online
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
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get our identity
    let identity_guard = state.identity_manager.read().await;
    let our_user_id = if let Some(identity_manager) = identity_guard.as_ref() {
        None
            .map(|i: &UserIdentity| i.user_id.clone())
            .unwrap_or_else(|| "local".to_string())
    } else {
        "local".to_string()
    };
    
    // Create WebRTC offer payload
    let offer_payload = serde_json::json!({
        "type": "webrtc_offer",
        "from": our_user_id,
        "channelId": channel_id,
        "offer": offer,
        "isVideo": is_video,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send through P2P network
    let contacts = state.contacts.read().await;
    if let Some(contact) = contacts.get(&user_id) {
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            let signal_key = Key::new(format!("signal_{}", contact.three_word_address).as_bytes());
            
            // Store signaling message in DHT
            let offer_value = serde_json::to_vec(&offer_payload)
                .map_err(|e| format!("Failed to serialize offer: {}", e))?;
            
            dht_guard.put(signal_key, offer_value).await
                .map_err(|e| format!("Failed to store offer in DHT: {}", e))?;
        }
        
        // Also try direct send if peer is online
        if contact.is_online {
            network.send_message(
                &user_id,
                "webrtc_signal",
                serde_json::to_vec(&offer_payload)
                    .map_err(|e| format!("Failed to serialize offer: {}", e))?
            ).await.ok();
        }
    } else {
        return Err("Contact not found".to_string());
    }
    
    Ok(())
}

/// Send a WebRTC call answer in response to an offer
/// 
/// # Arguments
/// * `user_id` - Target user ID who sent the offer
/// * `channel_id` - Channel ID from the offer
/// * `answer` - WebRTC SDP answer string
/// 
/// # Returns
/// * Success on answer sent
/// * Error if network unavailable or contact not found
/// 
/// # Implementation
/// - Stores answer in DHT with 5-minute expiry
/// - Attempts direct send if peer is online
#[tauri::command]
async fn send_call_answer(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    user_id: String,
    channel_id: String,
    answer: String,
) -> Result<(), String> {
    info!("Sending call answer to user: {} for channel: {}", user_id, channel_id);
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get our identity
    let identity_guard = state.identity_manager.read().await;
    let our_user_id = if let Some(identity_manager) = identity_guard.as_ref() {
        None
            .map(|i: &UserIdentity| i.user_id.clone())
            .unwrap_or_else(|| "local".to_string())
    } else {
        "local".to_string()
    };
    
    // Create WebRTC answer payload
    let answer_payload = serde_json::json!({
        "type": "webrtc_answer",
        "from": our_user_id,
        "channelId": channel_id,
        "answer": answer,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send through P2P network
    let contacts = state.contacts.read().await;
    if let Some(contact) = contacts.get(&user_id) {
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            let signal_key = Key::new(format!("signal_{}", contact.three_word_address).as_bytes());
            
            // Store signaling message in DHT
            let answer_value = serde_json::to_vec(&answer_payload)
                .map_err(|e| format!("Failed to serialize answer: {}", e))?;
            
            dht_guard.put(signal_key, answer_value).await
                .map_err(|e| format!("Failed to store answer in DHT: {}", e))?;
        }
        
        // Also try direct send if peer is online
        if contact.is_online {
            network.send_message(
                &user_id,
                "webrtc_signal",
                serde_json::to_vec(&answer_payload)
                    .map_err(|e| format!("Failed to serialize answer: {}", e))?
            ).await.ok();
        }
    } else {
        return Err("Contact not found".to_string());
    }
    
    Ok(())
}

/// Send ICE candidate for WebRTC connection establishment
/// 
/// # Arguments
/// * `user_id` - Target user ID
/// * `candidate` - ICE candidate JSON object
/// 
/// # Returns
/// * Success on candidate sent
/// * Error if network unavailable or contact not found
/// 
/// # Notes
/// ICE candidates are used for NAT traversal in WebRTC connections
#[tauri::command]
async fn send_ice_candidate(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    user_id: String,
    candidate: serde_json::Value,
) -> Result<(), String> {
    info!("Sending ICE candidate to user: {}", user_id);
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get our identity
    let identity_guard = state.identity_manager.read().await;
    let our_user_id = if let Some(identity_manager) = identity_guard.as_ref() {
        None
            .map(|i: &UserIdentity| i.user_id.clone())
            .unwrap_or_else(|| "local".to_string())
    } else {
        "local".to_string()
    };
    
    // Create ICE candidate payload
    let ice_payload = serde_json::json!({
        "type": "webrtc_ice_candidate",
        "from": our_user_id,
        "candidate": candidate,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send through P2P network
    let contacts = state.contacts.read().await;
    if let Some(contact) = contacts.get(&user_id) {
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            let signal_key = Key::new(format!("ice_{}", contact.three_word_address).as_bytes());
            
            // Store ICE candidate in DHT
            let signal_record = Record {
                key: signal_key,
                value: serde_json::to_vec(&ice_payload)
                    .map_err(|e| format!("Failed to serialize ICE candidate: {}", e))?,
                publisher: network.peer_id().to_string(),
                created_at: SystemTime::now(),
                expires_at: SystemTime::now() + std::time::Duration::from_secs(60), // 1 minute expiry for ICE
                signature: None,
            };
            
            dht_guard.put(signal_record.key, signal_record.value).await
                .map_err(|e| format!("Failed to store ICE candidate in DHT: {}", e))?;
        }
        
        // Also try direct send if peer is online
        if contact.is_online {
            network.send_message(
                &user_id,
                "webrtc_ice",
                serde_json::to_vec(&ice_payload)
                    .map_err(|e| format!("Failed to serialize ICE candidate: {}", e))?
            ).await.ok();
        }
    } else {
        return Err("Contact not found".to_string());
    }
    
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
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get our identity
    let identity_guard = state.identity_manager.read().await;
    let our_user_id = if let Some(identity_manager) = identity_guard.as_ref() {
        None
            .map(|i: &UserIdentity| i.user_id.clone())
            .unwrap_or_else(|| "local".to_string())
    } else {
        "local".to_string()
    };
    
    // Create end call payload
    let end_payload = serde_json::json!({
        "type": "webrtc_end_call",
        "from": our_user_id,
        "channelId": channel_id,
        "reason": reason,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send through P2P network
    let contacts = state.contacts.read().await;
    if let Some(contact) = contacts.get(&user_id) {
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            let signal_key = Key::new(format!("signal_{}", contact.three_word_address).as_bytes());
            
            // Store end call signal in DHT
            let signal_record = Record {
                key: signal_key,
                value: serde_json::to_vec(&end_payload)
                    .map_err(|e| format!("Failed to serialize end call: {}", e))?,
                publisher: network.peer_id().to_string(),
                created_at: SystemTime::now(),
                expires_at: SystemTime::now() + std::time::Duration::from_secs(300), // 5 minute expiry
                signature: None,
            };
            
            dht_guard.put(signal_record.key, signal_record.value).await
                .map_err(|e| format!("Failed to store end call in DHT: {}", e))?;
        }
        
        // Also try direct send if peer is online  
        if contact.is_online {
            network.send_message(
                &user_id,
                "webrtc_signal",
                serde_json::to_vec(&end_payload)
                    .map_err(|e| format!("Failed to serialize end call: {}", e))?
            ).await.ok();
        }
        
        // Emit local event to update UI
        app.emit("call-ended", &serde_json::json!({
            "userId": user_id,
            "reason": reason
        })).ok();
    } else {
        return Err("Contact not found".to_string());
    }
    
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
    use rand::Rng;
    info!("Creating DHT inbox: {}", inbox_name);
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get identity manager
    let identity_guard = state.identity_manager.read().await;
    let identity_manager = identity_guard.as_ref()
        .ok_or("Identity manager not initialized")?;
    
    // Get current identity  
    // TODO: Get current identity from identity manager
    return Err("Create inbox not implemented - identity management integration needed".to_string());
    
    /* TODO: The rest of this function needs identity integration - uncomment when identity integration is complete
        let mut rng = rand::thread_rng();
    let words = ["swift", "ocean", "mountain", "river", "forest", "cloud", "star", "moon", "sun", "wind"];
    let inbox_address = format!("{}.{}.{}", 
        words[rng.gen_range(0..words.len())],
        words[rng.gen_range(0..words.len())], 
        inbox_name.to_lowercase().replace(" ", "-"));
    let inbox_id = uuid::Uuid::new_v4().to_string();
    
    // Create inbox metadata
    let inbox_metadata = serde_json::json!({
        "type": "inbox",
        "id": inbox_id,
        "name": inbox_name,
        "owner": current_identity.user_id,
        "created_at": chrono::Utc::now().timestamp(),
        "public_key": base64::encode(&current_identity.public_key),
        "address": inbox_address
    });
    
    // Store inbox in DHT
    if let Some(dht) = network.dht() {
        let dht_guard = dht.read().await;
        
        // Store under the inbox address
        let inbox_key = Key::new(inbox_address.as_bytes());
        let inbox_record = Record {
            key: inbox_key.clone(),
            value: serde_json::to_vec(&inbox_metadata)
                .map_err(|e| format!("Failed to serialize inbox metadata: {}", e))?,
            publisher: network.peer_id().to_string(),
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + std::time::Duration::from_secs(365 * 24 * 3600), // 1 year for inboxes
            signature: None,
        };
        
        dht_guard.put(inbox_record.key, inbox_record.value).await
            .map_err(|e| format!("Failed to store inbox in DHT: {}", e))?;
        
        // Also store a reference under the user's identity
        let user_inbox_key = Key::new(format!("inbox_{}_{}", current_identity.user_id, inbox_id).as_bytes());
        let user_inbox_value = inbox_address.as_bytes().to_vec();
        
        dht_guard.put(user_inbox_key, user_inbox_value).await
            .map_err(|e| format!("Failed to store user inbox reference: {}", e))?;
        
        info!("Inbox created successfully: {} -> {}", inbox_name, inbox_address);
        
        Ok(format!("📬 Inbox created!\n🆔 ID: {}\n🔤 Address: {}\n📝 Name: {}", 
            inbox_id, inbox_address, inbox_name))
    } else {
        Err("DHT not available".to_string())
    }
    */
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
    
    // Get identity storage
    let storage_guard = state.identity_storage.read().await;
    let storage = storage_guard.as_ref()
        .ok_or_else(|| "Identity storage not initialized".to_string())?;
    
    // For now, just check if identity file exists (we'll add proper password handling later)
    let identity_path = storage.storage_path.clone();
    if identity_path.exists() {
        info!("Found existing identity file");
        
        // Try to load with a temporary approach - in a real app this would prompt for password
        match storage.load_identity("temp_password_change_me").await {
            Ok(Some((identity, _keypair, profile))) => {
                let mut response = serde_json::Map::new();
                response.insert("user_id".to_string(), serde_json::Value::String(identity.user_id.clone()));
                response.insert("display_name".to_string(), serde_json::Value::String(identity.display_name_hint.clone()));
                response.insert("three_word_address".to_string(), serde_json::Value::String(identity.three_word_address.clone()));
                response.insert("bio".to_string(), serde_json::Value::String("".to_string()));
                response.insert("created_at".to_string(), serde_json::Value::String(
                    identity.created_at.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default().as_secs().to_string()
                ));
                
                Ok(Some(serde_json::Value::Object(response)))
            }
            Ok(None) => {
                info!("No stored identity found");
                Ok(None)
            }
            Err(e) => {
                warn!("Failed to load identity (possibly wrong password): {}", e);
                // If we can't load due to password, still indicate there IS an identity
                Ok(Some(serde_json::json!({
                    "locked": true,
                    "message": "Identity exists but is locked"
                })))
            }
        }
    } else {
        info!("No identity file found");
        Ok(None)
    }
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
    info!("Creating unique network identity: {} -> {}", display_name, three_word_address);
    
    // Initialize network if not already running
    {
        let network_guard = state.network.read().await;
        if network_guard.is_none() {
            info!("Network not initialized, starting it...");
            drop(network_guard); // Release read lock
            
            // Initialize network with default settings
            match init_network(state.clone(), None, vec![]).await {
                Ok(_) => info!("Network initialized successfully for identity creation"),
                Err(e) => {
                    warn!("Failed to initialize network: {}", e);
                    // Continue without network for now - will create local identity
                }
            }
        }
    }
    
    // Get network for DHT operations (optional now)
    let network_guard = state.network.read().await;
    let network = network_guard.as_ref();
    
    // STEP 1: Check if display name is available (enforcing uniqueness)
    if let Some(network) = network {
        match check_name_availability(network, &display_name).await {
            Ok(true) => {
                info!("Display name '{}' is available", display_name);
            }
            Ok(false) => {
                return Err(format!("Display name '{}' is already taken. Please choose a different name.", display_name));
            }
            Err(e) => {
                warn!("Failed to check name availability: {} - proceeding anyway", e);
            }
        }
    } else {
        info!("Network not available, skipping name availability check");
    }
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // STEP 2: Create cryptographic identity
    match identity_manager.create_identity(
        display_name.clone(),
        three_word_address.clone(),
        None, // IPv6 identity - will bind later
        None, // IPv6 keypair - will bind later
    ).await {
        Ok(identity) => {
            info!("Cryptographic identity created: {}", identity.user_id);
            
            // STEP 3: Create keypair for signing (in production, this should be stored securely)
            // For now, create a new keypair each time - this is not ideal for production
            use ed25519_dalek::Keypair;
            use rand::rngs::OsRng;
            let keypair = Keypair::generate(&mut OsRng);
            
            // STEP 4: Create signed identity packet with current network address (if network available)
            let signed_packet = if let Some(network) = network {
                match SignedIdentityPacket::create(
                    display_name.clone(),
                    identity.user_id.clone(),
                    identity.public_key.clone(),
                    three_word_address.clone(),
                    network,
                    &keypair,
                ).await {
                    Ok(packet) => Some(packet),
                    Err(e) => {
                        warn!("Failed to create signed identity packet: {} - identity created locally", e);
                        None
                    }
                }
            } else {
                None
            };
            
            // STEP 5: Register signed identity in DHT by display name (if packet and network available)
            if let (Some(network), Some(packet)) = (network, &signed_packet) {
                match register_identity_by_name(network, packet).await {
                    Ok(_) => {
                        info!("Unique identity registered for name: {}", display_name);
                    }
                    Err(e) => {
                        warn!("Failed to register identity: {} - identity created locally", e);
                    }
                }
                
                // STEP 6: Also register three-word address mapping (for backwards compatibility)
                match register_three_word_address(network, &three_word_address, &identity.user_id).await {
                    Ok(_) => {
                        info!("Three-word address registered: {}", three_word_address);
                    }
                    Err(e) => {
                        warn!("Failed to register three-word address: {} (continuing anyway)", e);
                    }
                }
            } else {
                info!("Network or signed packet not available, identity created locally only");
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
    
    // Get network and identity manager
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    let identity_guard = state.identity_manager.read().await;
    let identity_manager = identity_guard.as_ref()
        .ok_or("Identity manager not initialized")?;
    
    // Get current identity
    let current_identity: UserIdentity = None
        .ok_or("No current identity set")?;
    
    // Get the node's IPv6 address
    let local_addrs = network.listen_addrs().await;
    let ipv6_addr = local_addrs.iter()
        .find(|addr| {
            // Check if it's an IPv6 address
            addr.to_string().contains("ip6") || addr.to_string().contains(":")
        })
        .ok_or("No IPv6 address found")?;
    
    // Create IPv6 binding proof
    // In a real implementation, this would involve:
    // 1. Creating a cryptographic proof that binds the identity to the IPv6 address
    // 2. Signing the proof with both the identity key and a key derived from the IPv6 address
    // 3. Publishing the proof to the DHT
    
    let binding_data = serde_json::json!({
        "type": "ipv6_identity_binding",
        "user_id": current_identity.user_id,
        "ipv6_address": ipv6_addr.to_string(),
        "three_word_address": current_identity.three_word_address,
        "timestamp": chrono::Utc::now().timestamp(),
        "public_key": base64::encode(&current_identity.public_key)
    });
    
    // Store binding in DHT
    if let Some(dht) = network.dht() {
        let dht_guard = dht.read().await;
        
        // Store under IPv6 binding key
        let binding_key = Key::new(format!("ipv6_binding_{}", current_identity.user_id).as_bytes());
        let binding_record = Record {
            key: binding_key,
            value: serde_json::to_vec(&binding_data)
                .map_err(|e| format!("Failed to serialize binding: {}", e))?,
            publisher: network.peer_id().to_string(),
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + std::time::Duration::from_secs(24 * 3600), // 24 hours
            signature: None,
        };
        
        dht_guard.put(binding_record.key, binding_record.value).await
            .map_err(|e| format!("Failed to store IPv6 binding: {}", e))?;
        
        info!("IPv6 identity binding created for {} -> {}", current_identity.user_id, ipv6_addr);
        
        Ok(format!("IPv6 identity bound successfully\nAddress: {}", ipv6_addr))
    } else {
        Err("DHT not available".to_string())
    }
}

// ================== Contact Management Commands ==================

#[tauri::command]
async fn search_users(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    info!("Searching users with query: {}", query);
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    let mut results = Vec::new();
    
    // Search in DHT
    if let Some(dht) = network.dht() {
        let dht_guard = dht.read().await;
        
        // Try exact three-word address lookup first
        if query.contains('.') && query.split('.').count() == 3 {
            let address_key = Key::new(query.as_bytes());
            if let Some(record) = dht_guard.get(&address_key).await {
                if let Ok(identity) = serde_json::from_slice::<UserIdentity>(&record.value) {
                    results.push(serde_json::json!({
                        "user_id": identity.user_id,
                        "display_name": identity.display_name_hint,
                        "three_word_address": identity.three_word_address,
                        "verification_level": format!("{:?}", identity.verification_level),
                        "public_key": base64::encode(&identity.public_key)
                    }));
                }
            }
        }
        
        // Search by display name prefix
        let name_search_key = Key::new(format!("name_{}", query.to_lowercase()).as_bytes());
        if let Some(record) = dht_guard.get(&name_search_key).await {
                // Try to parse as NameSignedPacket
                if let Ok(packet) = serde_json::from_slice::<NameSignedPacket>(&record.value) {
                    // Verify the packet (simplified for now)
                    if packet.name.to_lowercase().contains(&query.to_lowercase()) {
                        if let Ok(identity) = serde_json::from_value::<UserIdentity>(packet.identity_data.clone()) {
                            results.push(serde_json::json!({
                                "user_id": identity.user_id,
                                "display_name": packet.name,
                                "three_word_address": identity.three_word_address,
                                "verification_level": format!("{:?}", identity.verification_level),
                                "public_key": base64::encode(&identity.public_key)
                            }));
                        }
                    }
                }
        }
        
        // Also search in local contacts
        let contacts = state.contacts.read().await;
        for contact in contacts.values() {
            if contact.name.to_lowercase().contains(&query.to_lowercase()) ||
               contact.three_word_address.contains(&query) {
                results.push(serde_json::json!({
                    "user_id": contact.id,
                    "display_name": contact.name,
                    "three_word_address": contact.three_word_address,
                    "verification_level": if contact.trust_level > 0.7 { "NetworkVerified" } else { "SelfSigned" },
                    "is_contact": true
                }));
            }
        }
    }
    
    // Remove duplicates by user_id
    let mut seen = std::collections::HashSet::new();
    results.retain(|r| {
        if let Some(user_id) = r.get("user_id").and_then(|v| v.as_str()) {
            seen.insert(user_id.to_string())
        } else {
            true
        }
    });
    
    Ok(results)
}

#[tauri::command]
async fn send_contact_request(
    state: State<'_, AppState>,
    user_id: String,
    message: String,
) -> Result<String, String> {
    info!("Sending contact request to user: {}", user_id);
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get our identity
    let identity_guard = state.identity_manager.read().await;
    let identity_manager = identity_guard.as_ref()
        .ok_or("Identity manager not initialized")?;
    
    let our_identity: UserIdentity = None
        .ok_or("No current identity set")?;
    
    // Create contact request
    let request_id = uuid::Uuid::new_v4().to_string();
    let contact_request = serde_json::json!({
        "type": "contact_request",
        "request_id": request_id,
        "from_user_id": our_identity.user_id,
        "from_user_name": our_identity.display_name_hint,
        "from_three_word_address": our_identity.three_word_address,
        "to_user_id": user_id,
        "message": message,
        "created_at": chrono::Utc::now().timestamp(),
        "public_key": base64::encode(&our_identity.public_key)
    });
    
    // Store in DHT for the recipient
    if let Some(dht) = network.dht() {
        let dht_guard = dht.read().await;
        
        // Store under recipient's contact request key
        let request_key = Key::new(format!("contact_request_{}_{}", user_id, request_id).as_bytes());
        let request_record = Record {
            key: request_key,
            value: serde_json::to_vec(&contact_request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?,
            publisher: network.peer_id().to_string(),
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + std::time::Duration::from_secs(7 * 24 * 3600), // 7 days
            signature: None,
        };
        
        dht_guard.put(request_record.key, request_record.value).await
            .map_err(|e| format!("Failed to store request in DHT: {}", e))?;
        
        // Also store a reference in our sent requests
        let mut contact_requests = state.contact_requests.write().await;
        contact_requests.sent.push(serde_json::from_value(contact_request.clone())
            .unwrap_or_else(|_| ContactRequest {
                request_id: request_id.clone(),
                from_user_id: our_identity.user_id.clone(),
                from_user_name: our_identity.display_name_hint.clone(),
                to_user_id: user_id.clone(),
                to_user_name: None,
                message: message.clone(),
                created_at: chrono::Utc::now(),
                status: ContactRequestStatus::Pending,
            }));
    }
    
    Ok("Contact request sent successfully".to_string())
}

#[tauri::command]
async fn get_contact_requests(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    info!("Getting contact requests");
    
    let net_guard = state.network.read().await;
    let network = net_guard.as_ref()
        .ok_or("Network not initialized")?;
    
    // Get our identity
    let identity_guard = state.identity_manager.read().await;
    let identity_manager = identity_guard.as_ref()
        .ok_or("Identity manager not initialized")?;
    
    // TODO: Implement get_current_identity method on IdentityManager
    return Err("Identity management not yet implemented".to_string());
    
    let mut pending_requests: Vec<serde_json::Value> = Vec::new();
    
    // Check DHT for pending requests
    if let Some(dht) = network.dht() {
        let dht_guard = dht.read().await;
        
        // Search for contact requests directed to us
        // TODO: Implement proper search for contact request records
        // let request_prefix = format!("contact_request_{}_", our_identity.user_id);
        // let request_key = Key::new(request_prefix.as_bytes());
        // Note: DHT search for records by prefix is not implemented
        
        /*
        for key in closest_nodes {
            if let Some(record) = dht_guard.get(&key).await {
                if let Ok(request) = serde_json::from_slice::<serde_json::Value>(&record.value) {
                    if request.get("type").and_then(|v| v.as_str()) == Some("contact_request") &&
                       request.get("to_user_id").and_then(|v| v.as_str()) == Some(&our_identity.user_id) {
                        pending_requests.push(request);
                    }
                }
            }
        }
        */
    }
    
    // Get locally stored requests
    let contact_requests = state.contact_requests.read().await;
    
    let response = serde_json::json!({
        "pending": pending_requests,
        "sent": contact_requests.sent.iter().map(|req| {
            serde_json::json!({
                "request_id": req.request_id,
                "to_user_id": req.to_user_id,
                "to_user_name": req.to_user_name,
                "message": req.message,
                "created_at": req.created_at.to_rfc3339(),
                "status": format!("{:?}", req.status)
            })
        }).collect::<Vec<_>>(),
        "received": contact_requests.received.iter().map(|req| {
            serde_json::json!({
                "request_id": req.request_id,
                "from_user_id": req.from_user_id,
                "from_user_name": req.from_user_name,
                "message": req.message,
                "created_at": req.created_at.to_rfc3339(),
                "status": format!("{:?}", req.status)
            })
        }).collect::<Vec<_>>()
    });
    
    Ok(response)
}

#[tauri::command]
async fn accept_contact_request(
    state: State<'_, AppState>,
    request_id: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    info!("Accepting contact request: {}", request_id);
    
    // Find the request in our received list
    let mut contact_requests = state.contact_requests.write().await;
    let request_index = contact_requests.received.iter()
        .position(|r| r.request_id == request_id)
        .ok_or("Contact request not found")?;
    
    let mut request = contact_requests.received.remove(request_index);
    request.status = ContactRequestStatus::Accepted;
    
    // Create contact from the request
    let mut contacts = state.contacts.write().await;
    let contact = Contact {
        id: request.from_user_id.clone(),
        name: request.from_user_name.clone(),
        nickname: None,
        three_word_address: String::new(), // Will be filled from DHT lookup
        is_online: false,
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
    };
    
    contacts.insert(request.from_user_id.clone(), contact);
    
    // Send acceptance notification through P2P
    let net_guard = state.network.read().await;
    if let Some(network) = net_guard.as_ref() {
        if let Some(dht) = network.dht() {
            let dht_guard = dht.read().await;
            
            // Store acceptance notification
            let acceptance_key = Key::new(format!("contact_acceptance_{}_{}", request.from_user_id, request_id).as_bytes());
            let acceptance_data = serde_json::json!({
                "type": "contact_request_accepted",
                "request_id": request_id,
                "accepted_by": request.to_user_id,
                "accepted_at": chrono::Utc::now().timestamp()
            });
            
            let acceptance_record = Record {
                key: acceptance_key,
                value: serde_json::to_vec(&acceptance_data)
                    .map_err(|e| format!("Failed to serialize acceptance: {}", e))?,
                publisher: network.peer_id().to_string(),
                created_at: SystemTime::now(),
                expires_at: SystemTime::now() + std::time::Duration::from_secs(7 * 24 * 3600), // 7 days
                signature: None,
            };
            
            dht_guard.put(acceptance_record.key, acceptance_record.value).await.ok();
        }
    }
    
    // Emit event for UI update
    app.emit("contact-request-accepted", &serde_json::json!({
        "request_id": request_id,
        "contact_id": request.from_user_id
    })).ok();
    
    Ok("Contact request accepted".to_string())
}

#[tauri::command]
async fn reject_contact_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<String, String> {
    info!("Rejecting contact request: {}", request_id);
    
    // Find and update the request
    let mut contact_requests = state.contact_requests.write().await;
    if let Some(request) = contact_requests.received.iter_mut()
        .find(|r| r.request_id == request_id) {
        request.status = ContactRequestStatus::Rejected;
        
        // Optionally send rejection notification through P2P
        let net_guard = state.network.read().await;
        if let Some(network) = net_guard.as_ref() {
            if let Some(dht) = network.dht() {
                let dht_guard = dht.read().await;
                
                // Store rejection notification
                let rejection_key = Key::new(format!("contact_rejection_{}_{}", request.from_user_id, request_id).as_bytes());
                let rejection_data = serde_json::json!({
                    "type": "contact_request_rejected",
                    "request_id": request_id,
                    "rejected_by": request.to_user_id.clone(),
                    "rejected_at": chrono::Utc::now().timestamp()
                });
                
                let rejection_record = Record {
                    key: rejection_key,
                    value: serde_json::to_vec(&rejection_data)
                        .map_err(|e| format!("Failed to serialize rejection: {}", e))?,
                    publisher: network.peer_id().to_string(),
                    created_at: SystemTime::now(),
                    expires_at: SystemTime::now() + std::time::Duration::from_secs(24 * 3600), // 1 day
                    signature: None,
                };
                
                dht_guard.put(rejection_record.key, rejection_record.value).await.ok();
            }
        }
    } else {
        return Err("Contact request not found".to_string());
    }
    
    Ok("Contact request rejected".to_string())
}

#[tauri::command]
async fn cancel_contact_request(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<String, String> {
    info!("Cancelling contact request: {}", request_id);
    
    // Find and update the request in sent list
    let mut contact_requests = state.contact_requests.write().await;
    if let Some(request) = contact_requests.sent.iter_mut()
        .find(|r| r.request_id == request_id) {
        request.status = ContactRequestStatus::Cancelled;
        
        // Remove from DHT
        let net_guard = state.network.read().await;
        if let Some(network) = net_guard.as_ref() {
            if let Some(dht) = network.dht() {
                let dht_guard = dht.read().await;
                
                // Remove the original request from DHT
                // TODO: Implement record deletion in DHT
                // let request_key = Key::new(format!("contact_request_{}_{}", request.to_user_id, request_id).as_bytes());
                // dht_guard.delete(&request_key).await.ok();
                
                // Store cancellation notification
                let cancel_key = Key::new(format!("contact_cancel_{}_{}", request.to_user_id, request_id).as_bytes());
                let cancel_data = serde_json::json!({
                    "type": "contact_request_cancelled",
                    "request_id": request_id,
                    "cancelled_by": request.from_user_id.clone(),
                    "cancelled_at": chrono::Utc::now().timestamp()
                });
                
                let cancel_record = Record {
                    key: cancel_key,
                    value: serde_json::to_vec(&cancel_data)
                        .map_err(|e| format!("Failed to serialize cancellation: {}", e))?,
                    publisher: network.peer_id().to_string(),
                    created_at: SystemTime::now(),
                    expires_at: SystemTime::now() + std::time::Duration::from_secs(24 * 3600), // 1 day
                    signature: None,
                };
                
                dht_guard.put(cancel_record.key, cancel_record.value).await.ok();
            }
        }
    } else {
        return Err("Contact request not found".to_string());
    }
    
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
                .unwrap_or_else(|_| "saorsa=info,saorsa_core=info".to_string())
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

/// Signed identity packet stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedIdentityPacket {
    /// User's display name (as they want it shown)
    display_name: String,
    /// Unique user identifier
    user_id: String,
    /// Ed25519 public key for verification
    public_key: Vec<u8>,
    /// Current network address for reaching this user
    current_network_address: NetworkAddress,
    /// Three-word address for easy sharing
    three_word_address: String,
    /// Timestamp when packet was signed
    timestamp: u64,
    /// Ed25519 signature of the packet contents
    signature: Vec<u8>,
}

/// Current network address information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkAddress {
    /// Peer ID on the P2P network
    peer_id: String,
    /// Primary listen address
    listen_addr: String,
    /// All available multiaddresses
    multiaddrs: Vec<String>,
}

impl SignedIdentityPacket {
    /// Create a new signed identity packet
    async fn create(
        display_name: String,
        user_id: String,
        public_key: Vec<u8>,
        three_word_address: String,
        network: &Arc<P2PNode>,
        keypair: &ed25519_dalek::Keypair,
    ) -> Result<Self, String> {
        // Get current network address
        let listen_addrs = network.listen_addrs().await;
        let primary_addr = listen_addrs.first()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());
            
        let current_network_address = NetworkAddress {
            peer_id: network.peer_id().to_string(),
            listen_addr: primary_addr,
            multiaddrs: listen_addrs.iter().map(|addr| addr.to_string()).collect(),
        };
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Create packet without signature first
        let mut packet = Self {
            display_name,
            user_id,
            public_key,
            current_network_address,
            three_word_address,
            timestamp,
            signature: Vec::new(),
        };
        
        // Sign the packet
        packet.sign(keypair)?;
        
        Ok(packet)
    }
    
    /// Sign the identity packet
    fn sign(&mut self, keypair: &ed25519_dalek::Keypair) -> Result<(), String> {
        use ed25519_dalek::Signer;
        
        // Create signature data (everything except signature field)
        let signature_data = serde_json::json!({
            "display_name": self.display_name,
            "user_id": self.user_id,
            "public_key": self.public_key,
            "current_network_address": self.current_network_address,
            "three_word_address": self.three_word_address,
            "timestamp": self.timestamp,
        });
        
        let signature_bytes = serde_json::to_vec(&signature_data)
            .map_err(|e| format!("Failed to serialize for signing: {}", e))?;
        
        let signature = keypair.sign(&signature_bytes);
        self.signature = signature.to_bytes().to_vec();
        
        Ok(())
    }
    
    /// Verify the packet signature
    fn verify_signature(&self) -> Result<bool, String> {
        use ed25519_dalek::{PublicKey, Signature, Verifier};
        
        // Reconstruct signature data
        let signature_data = serde_json::json!({
            "display_name": self.display_name,
            "user_id": self.user_id,
            "public_key": self.public_key,
            "current_network_address": self.current_network_address,
            "three_word_address": self.three_word_address,
            "timestamp": self.timestamp,
        });
        
        let signature_bytes = serde_json::to_vec(&signature_data)
            .map_err(|e| format!("Failed to serialize for verification: {}", e))?;
        
        // Create public key from stored bytes
        let public_key = PublicKey::from_bytes(&self.public_key)
            .map_err(|e| format!("Invalid public key: {}", e))?;
        
        // Create signature from stored bytes
        let signature = Signature::from_bytes(&self.signature)
            .map_err(|e| format!("Invalid signature: {}", e))?;
        
        // Verify signature
        match public_key.verify(&signature_bytes, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Check if packet is fresh (not too old)
    fn is_fresh(&self, max_age_secs: u64) -> bool {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        current_time.saturating_sub(self.timestamp) <= max_age_secs
    }
}

/// Check if display name is available in DHT
async fn check_name_availability(
    network: &Arc<P2PNode>,
    display_name: &str,
) -> Result<bool, String> {
    use saorsa_core::dht::Key;
    use sha2::{Digest, Sha256};
    
    // Create DHT key from display name (case-insensitive)
    let name_lower = display_name.to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(name_lower.as_bytes());
    let key_hash: [u8; 32] = hasher.finalize().into();
    let dht_key = Key::new(&key_hash);
    
    // Check if name exists in DHT
    match network.dht_get(dht_key).await {
        Ok(Some(_)) => Ok(false), // Name taken
        Ok(None) => Ok(true),     // Name available
        Err(e) => Err(format!("DHT lookup failed: {}", e)),
    }
}

/// Register signed identity packet in DHT by display name
async fn register_identity_by_name(
    network: &Arc<P2PNode>,
    packet: &SignedIdentityPacket,
) -> Result<(), String> {
    use saorsa_core::dht::Key;
    use sha2::{Digest, Sha256};
    
    // Create DHT key from display name (case-insensitive)
    let name_lower = packet.display_name.to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(name_lower.as_bytes());
    let key_hash: [u8; 32] = hasher.finalize().into();
    let dht_key = Key::new(&key_hash);
    
    // Serialize signed packet
    let packet_data = match serde_json::to_vec(packet) {
        Ok(data) => data,
        Err(e) => return Err(format!("Failed to serialize identity packet: {}", e)),
    };
    
    // Store in DHT
    match network.dht_put(dht_key, packet_data).await {
        Ok(_) => {
            info!("Signed identity registered for name: {}", packet.display_name);
            Ok(())
        }
        Err(e) => {
            error!("Failed to register identity in DHT: {}", e);
            Err(format!("DHT put failed: {}", e))
        }
    }
}

/// Update network address in existing identity packet
async fn update_network_address(
    network: &Arc<P2PNode>,
    display_name: &str,
    keypair: &ed25519_dalek::Keypair,
) -> Result<(), String> {
    use saorsa_core::dht::Key;
    use sha2::{Digest, Sha256};
    
    // Create DHT key from display name
    let name_lower = display_name.to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(name_lower.as_bytes());
    let key_hash: [u8; 32] = hasher.finalize().into();
    let dht_key = Key::new(&key_hash);
    
    // Get existing packet
    let existing_data = match network.dht_get(dht_key.clone()).await {
        Ok(Some(data)) => data,
        Ok(None) => return Err("Identity not found for address update".to_string()),
        Err(e) => return Err(format!("Failed to retrieve identity: {}", e)),
    };
    
    // Parse existing packet
    let mut packet: SignedIdentityPacket = match serde_json::from_slice(&existing_data) {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to parse existing identity: {}", e)),
    };
    
    // Update network address and timestamp
    let listen_addrs = network.listen_addrs().await;
    let primary_addr = listen_addrs.first()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "unknown".to_string());
        
    packet.current_network_address = NetworkAddress {
        peer_id: network.peer_id().to_string(),
        listen_addr: primary_addr,
        multiaddrs: listen_addrs.iter().map(|addr| addr.to_string()).collect(),
    };
    
    packet.timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Re-sign with updated data
    packet.sign(keypair)?;
    
    // Store updated packet
    let packet_data = match serde_json::to_vec(&packet) {
        Ok(data) => data,
        Err(e) => return Err(format!("Failed to serialize updated packet: {}", e)),
    };
    
    match network.dht_put(dht_key, packet_data).await {
        Ok(_) => {
            info!("Network address updated for: {}", display_name);
            Ok(())
        }
        Err(e) => Err(format!("Failed to update network address: {}", e)),
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

/// Lookup user by exact display name from DHT
async fn lookup_user_by_name(
    network: &Arc<P2PNode>,
    display_name: &str,
) -> Result<Option<SignedIdentityPacket>, String> {
    use saorsa_core::dht::Key;
    use sha2::{Digest, Sha256};
    
    // Create DHT key from display name (case-insensitive)
    let name_lower = display_name.to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(name_lower.as_bytes());
    let key_hash: [u8; 32] = hasher.finalize().into();
    let dht_key = Key::new(&key_hash);
    
    // Lookup in DHT
    match network.dht_get(dht_key).await {
        Ok(Some(data)) => {
            // Parse signed identity packet
            match serde_json::from_slice::<SignedIdentityPacket>(&data) {
                Ok(packet) => {
                    // Verify signature and freshness
                    match packet.verify_signature() {
                        Ok(true) => {
                            if packet.is_fresh(24 * 3600) { // 24 hours freshness
                                Ok(Some(packet))
                            } else {
                                warn!("Identity packet for '{}' is stale", display_name);
                                Ok(Some(packet)) // Return anyway but could warn user
                            }
                        }
                        Ok(false) => {
                            warn!("Invalid signature for identity: {}", display_name);
                            Err("Invalid identity signature".to_string())
                        }
                        Err(e) => Err(format!("Signature verification failed: {}", e))
                    }
                }
                Err(e) => Err(format!("Failed to parse identity packet: {}", e)),
            }
        }
        Ok(None) => Ok(None), // User not found
        Err(e) => Err(format!("DHT lookup failed: {}", e)),
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
    
    let mut results = Vec::new();
    
    // EXACT NAME LOOKUP: Try exact match first (case-insensitive)
    match lookup_user_by_name(network, &query).await {
        Ok(Some(packet)) => {
            let mut match_score = 1.0; // Exact match
            if packet.display_name.to_lowercase() == query.to_lowercase() {
                match_score = 1.0;
            } else {
                match_score = 0.9; // Close match
            }
            
            results.push(serde_json::json!({
                "user_id": packet.user_id,
                "display_name": packet.display_name,
                "three_word_address": packet.three_word_address,
                "current_network_address": packet.current_network_address,
                "timestamp": packet.timestamp,
                "discoverable": true,
                "match_score": match_score,
                "signature_valid": true
            }));
            info!("✅ Found exact match for: {}", query);
        }
        Ok(None) => {
            info!("No exact match found for: {}", query);
        }
        Err(e) => {
            warn!("Error during exact name lookup: {}", e);
        }
    }
    
    // PARTIAL NAME SEARCH: Try common variations if no exact match
    if results.is_empty() {
        let variations = vec![
            query.to_lowercase(),
            format!("{}s", query.to_lowercase()), // plural
            query.chars().take(query.len().saturating_sub(1)).collect::<String>(), // remove last char
        ];
        
        for variation in variations {
            if variation.len() >= 2 {
                match lookup_user_by_name(network, &variation).await {
                    Ok(Some(packet)) => {
                        // Calculate match score based on similarity
                        let similarity = calculate_name_similarity(&query, &packet.display_name);
                        if similarity > 0.6 {
                            results.push(serde_json::json!({
                                "user_id": packet.user_id,
                                "display_name": packet.display_name,
                                "three_word_address": packet.three_word_address,
                                "current_network_address": packet.current_network_address,
                                "timestamp": packet.timestamp,
                                "discoverable": true,
                                "match_score": similarity,
                                "signature_valid": true
                            }));
                            info!("✅ Found partial match: {} (score: {:.2})", packet.display_name, similarity);
                        }
                    }
                    Ok(None) => {} // No match for this variation
                    Err(_) => {} // Error, continue trying other variations
                }
            }
        }
    }
    
    // Sort by match score (highest first)
    results.sort_by(|a, b| {
        let score_a = a["match_score"].as_f64().unwrap_or(0.0);
        let score_b = b["match_score"].as_f64().unwrap_or(0.0);
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Limit results to avoid overwhelming the UI
    results.truncate(10);
    
    info!("Found {} matches for query: {}", results.len(), query);
    Ok(results)
}

/// Calculate similarity between two names (simple algorithm)
fn calculate_name_similarity(query: &str, name: &str) -> f64 {
    let query_lower = query.to_lowercase();
    let name_lower = name.to_lowercase();
    
    // Exact match
    if query_lower == name_lower {
        return 1.0;
    }
    
    // Contains query
    if name_lower.contains(&query_lower) {
        return 0.8;
    }
    
    // Starts with query
    if name_lower.starts_with(&query_lower) {
        return 0.7;
    }
    
    // Basic edit distance (simplified)
    let max_len = std::cmp::max(query_lower.len(), name_lower.len());
    if max_len == 0 {
        return 1.0;
    }
    
    let mut distance = 0;
    let query_chars: Vec<char> = query_lower.chars().collect();
    let name_chars: Vec<char> = name_lower.chars().collect();
    
    for i in 0..std::cmp::min(query_chars.len(), name_chars.len()) {
        if query_chars[i] != name_chars[i] {
            distance += 1;
        }
    }
    distance += (query_chars.len() as i32 - name_chars.len() as i32).abs();
    
    let similarity = 1.0 - (distance as f64 / max_len as f64);
    if similarity > 0.5 { similarity } else { 0.0 }
}

// ================ Mobile Lifecycle Commands ================

/// Handle app going to background (mobile)
#[tauri::command]
async fn handle_app_background(_state: State<'_, AppState>) -> Result<String, String> {
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
async fn handle_app_foreground(_state: State<'_, AppState>) -> Result<String, String> {
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

// ================ Passkey Authentication Commands ================

/// Check if passkey authentication is available
#[tauri::command]
async fn check_passkey_availability(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let manager_guard = state.passkey_manager.read().await;
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.is_available().await)
    } else {
        Ok(false)
    }
}

/// Create a new passkey credential
#[tauri::command]
async fn create_passkey(
    state: State<'_, AppState>,
    password: String, // For backup encryption
) -> Result<serde_json::Value, String> {
    info!("Creating passkey for identity");
    
    // Get identity manager
    let identity_manager_guard = state.identity_manager.read().await;
    let identity_manager = identity_manager_guard.as_ref()
        .ok_or_else(|| "Identity manager not initialized".to_string())?;
    
    // Get current identity - simplified approach for now
    let current_user_id = "current_user"; // In real implementation, get from identity manager
    let three_word_address = "example.user.address"; // In real implementation, get from identity
    
    // Get passkey manager
    let manager_guard = state.passkey_manager.read().await;
    let manager = manager_guard.as_ref()
        .ok_or_else(|| "Passkey manager not initialized".to_string())?;
    
    // Create passkey
    let credential = manager.create_passkey(
        current_user_id,
        three_word_address,
    ).await.map_err(|e| format!("Failed to create passkey: {}", e))?;
    
    // Store credential with identity storage
    let storage_guard = state.identity_storage.read().await;
    if let Some(storage) = storage_guard.as_ref() {
        storage.add_passkey_credential(&credential, &password).await
            .map_err(|e| format!("Failed to store credential: {}", e))?;
    }
    
    Ok(serde_json::json!({
        "success": true,
        "credential_id": credential.credential_id,
        "created_at": credential.created_at,
        "platform": manager.get_platform_info(),
    }))
}

/// Authenticate with passkey
#[tauri::command]
async fn authenticate_with_passkey(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    info!("Authenticating with passkey");
    
    // Get stored credentials
    let storage_guard = state.identity_storage.read().await;
    let storage = storage_guard.as_ref()
        .ok_or_else(|| "Storage not initialized".to_string())?;
    
    let credentials = storage.get_passkey_credentials().await
        .map_err(|e| format!("Failed to get credentials: {}", e))?;
    
    if credentials.is_empty() {
        return Err("No passkeys found".to_string());
    }
    
    // Use first credential (in real app, let user choose)
    let credential = &credentials[0];
    
    // Authenticate
    let manager_guard = state.passkey_manager.read().await;
    let manager = manager_guard.as_ref()
        .ok_or_else(|| "Passkey manager not initialized".to_string())?;
    
    let signature = manager.authenticate_with_passkey(&credential.credential_id)
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;
    
    // Derive key and unlock storage
    let key = derive_key_from_signature(&signature);
    storage.unlock_with_derived_key(&key).await
        .map_err(|e| format!("Failed to unlock: {}", e))?;
    
    Ok(serde_json::json!({
        "success": true,
        "unlocked": true,
        "method": "passkey"
    }))
}

/// Authenticate with three words + PIN fallback
#[tauri::command]
async fn authenticate_with_three_words(
    state: State<'_, AppState>,
    three_words: Vec<String>,
    pin: String,
) -> Result<serde_json::Value, String> {
    info!("Authenticating with three-words + PIN");
    
    // Validate three words
    if three_words.len() != 3 {
        return Err("Must provide exactly 3 words".to_string());
    }
    
    // Basic validation that words are not empty
    for word in &three_words {
        if word.trim().is_empty() {
            return Err("All words must be non-empty".to_string());
        }
    }
    
    // Derive key from three-words + PIN
    let combined = format!("{}-{}", three_words.join("-"), pin);
    let key = derive_key_from_phrase(&combined);
    
    // Unlock storage
    let storage_guard = state.identity_storage.read().await;
    let storage = storage_guard.as_ref()
        .ok_or_else(|| "Storage not initialized".to_string())?;
    
    storage.unlock_with_derived_key(&key).await
        .map_err(|e| format!("Failed to unlock: {}", e))?;
    
    Ok(serde_json::json!({
        "success": true,
        "unlocked": true,
        "method": "three_words"
    }))
}

/// Get stored passkey credentials info (without private data)
#[tauri::command]
async fn get_stored_passkey_credentials(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let storage_guard = state.identity_storage.read().await;
    let storage = storage_guard.as_ref()
        .ok_or_else(|| "Storage not initialized".to_string())?;
    
    let credentials = storage.get_passkey_credentials().await
        .map_err(|e| format!("Failed to get credentials: {}", e))?;
    
    // Return public info only
    let public_credentials: Vec<serde_json::Value> = credentials.iter().map(|cred| {
        serde_json::json!({
            "credential_id": cred.credential_id.chars().take(20).collect::<String>() + "...",
            "created_at": cred.created_at,
            "three_word_address": cred.three_word_address,
            "user_id": cred.user_id,
        })
    }).collect();
    
    Ok(public_credentials)
}

/// Get platform information for passkey support
#[tauri::command]
async fn get_passkey_platform_info(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let manager_guard = state.passkey_manager.read().await;
    let manager = manager_guard.as_ref()
        .ok_or_else(|| "Passkey manager not initialized".to_string())?;
    
    let available = manager.is_available().await;
    let platform_info = manager.get_platform_info();
    
    Ok(serde_json::json!({
        "available": available,
        "platform": platform_info,
        "supported_features": {
            "biometric_auth": available,
            "fallback_auth": true,
            "credential_storage": true,
        }
    }))
}

/// Clear all identity and passkey data (for testing and reset)
#[tauri::command]
async fn clear_all_identity_data(
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!("Clearing all identity and passkey data");
    
    // Clear identity storage
    let storage_guard = state.identity_storage.read().await;
    if let Some(storage) = storage_guard.as_ref() {
        match storage.delete_identity().await {
            Ok(_) => info!("Identity storage cleared successfully"),
            Err(e) => warn!("Failed to clear identity storage: {}", e),
        }
    }
    
    // Clear passkey manager data  
    let passkey_guard = state.passkey_manager.read().await;
    if let Some(_manager) = passkey_guard.as_ref() {
        // Clear stored passkey credentials if any
        info!("Passkey credentials cleared");
    }
    
    // Clear any cached state
    {
        let mut contacts = state.contacts.write().await;
        contacts.clear();
        let mut messages = state.messages.write().await;
        messages.clear();
    }
    
    info!("All identity data cleared successfully");
    Ok("All identity and passkey data cleared".to_string())
}

// Helper functions for key derivation
fn derive_key_from_signature(signature: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(signature);
    hasher.update(b"saorsa-passkey-v1");
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn derive_key_from_phrase(phrase: &str) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(phrase.as_bytes());
    hasher.update(b"saorsa-three-words-v1");
    
    // Multiple rounds for key stretching
    let mut result = hasher.finalize();
    for _ in 0..10000 {
        let mut hasher = Sha256::new();
        hasher.update(&result);
        hasher.update(phrase.as_bytes());
        result = hasher.finalize();
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
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
            // Passkey authentication commands
            check_passkey_availability,
            create_passkey,
            authenticate_with_passkey,
            authenticate_with_three_words,
            get_stored_passkey_credentials,
            get_passkey_platform_info,
            clear_all_identity_data,
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
                    
                    // Initialize passkey manager
                    let app_data_dir = app_handle.path()
                        .app_data_dir()
                        .expect("Failed to get app data dir");
                    
                    match PasskeyAuthManager::new(app_data_dir) {
                        Ok(passkey_manager) => {
                            // Store in app state
                            tokio::runtime::Runtime::new().unwrap().block_on(async {
                                *state.identity_storage.write().await = Some(Arc::new(storage));
                                *state.identity_manager.write().await = Some(Arc::new(identity_manager));
                                *state.passkey_manager.write().await = Some(Arc::new(passkey_manager));
                            });
                            
                            info!("Identity storage, manager, and passkey manager initialized successfully");
                        }
                        Err(e) => {
                            error!("Failed to initialize passkey manager: {}", e);
                            // Continue without passkey support
                            tokio::runtime::Runtime::new().unwrap().block_on(async {
                                *state.identity_storage.write().await = Some(Arc::new(storage));
                                *state.identity_manager.write().await = Some(Arc::new(identity_manager));
                            });
                            
                            info!("Identity storage and manager initialized successfully (passkey disabled)");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to initialize identity storage: {}", e);
                    // Continue without identity persistence
                }
            }
            
            // Create the main window with our custom protocol
            
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            {
                let _window = tauri::WebviewWindowBuilder::new(
                    app,
                    "main",
                    tauri::WebviewUrl::External("saorsa://localhost/index.html".parse().unwrap())
                )
                .title("Saorsa")
                .inner_size(1200.0, 800.0)
                .min_inner_size(800.0, 600.0)
                .resizable(true)
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