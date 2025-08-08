// Communitas - P2P Collaboration Platform v2.0
#![allow(dead_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bootstrap;
mod contact_commands;
mod contacts;
mod files;
mod groups;
mod identity;

use contact_commands::init_contact_manager;
use contacts::ContactManager;
use files::FileManager;
use groups::GroupManager;
use identity::IdentityManager;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// Real P2P node integration with saorsa-core
use saorsa_core::{NodeConfig, P2PNode, PeerId};
use std::collections::HashMap;
use std::net::SocketAddr;

// Application state management
#[derive(Debug)]
pub struct AppState {
    pub identity_manager: Arc<RwLock<IdentityManager>>,
    pub contact_manager: Arc<RwLock<ContactManager>>,
    pub group_manager: Arc<RwLock<GroupManager>>,
    pub file_manager: Arc<RwLock<FileManager>>,
    pub p2p_node: Option<Arc<RwLock<RealP2PNode>>>,
}

pub struct RealP2PNode {
    /// The actual saorsa-core P2P node
    node: Arc<P2PNode>,
    /// Our peer ID in the network
    peer_id: PeerId,
    /// Connected peers for tracking
    peers: HashMap<PeerId, String>,
    /// Bootstrap nodes for initial connection
    bootstrap_peers: Vec<SocketAddr>,
}

impl RealP2PNode {
    pub async fn new(config: NodeConfig) -> anyhow::Result<Self> {
        let node = P2PNode::new(config).await?;
        let peer_id = node.peer_id().to_string();

        Ok(Self {
            node: Arc::new(node),
            peer_id,
            peers: HashMap::new(),
            bootstrap_peers: vec!["bootstrap.communitas.app:8888".parse()?],
        })
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        info!("Starting P2P node with peer ID: {}", self.peer_id);

        // Connect to bootstrap nodes
        for addr in &self.bootstrap_peers {
            match self.node.connect_peer(&addr.to_string()).await {
                Ok(_) => info!("Connected to bootstrap node: {}", addr),
                Err(e) => warn!("Failed to connect to bootstrap node {}: {}", addr, e),
            }
        }

        self.node.start().await?;
        info!("P2P node started successfully");
        Ok(())
    }

    pub async fn get_peer_count(&self) -> usize {
        self.node.peer_count().await
    }

    pub fn local_peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }
}

impl fmt::Debug for RealP2PNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RealP2PNode")
            .field("peer_id", &self.peer_id)
            .field("peers_len", &self.peers.len())
            .field("bootstrap_peers", &self.bootstrap_peers)
            .finish()
    }
}

// Tauri commands
#[derive(serde::Serialize, serde::Deserialize)]
struct NetworkHealthResponse {
    status: String,
    peer_count: u32,
    nat_type: String,
    bandwidth_kbps: f64,
    avg_latency_ms: f64,
}

#[tauri::command]
async fn get_network_health(
    app_state: tauri::State<'_, Arc<RwLock<AppState>>>,
) -> Result<NetworkHealthResponse, String> {
    let state = app_state.inner().read().await;
    let peer_count = if let Some(p2p) = &state.p2p_node {
        let node = p2p.read().await;
        node.get_peer_count().await as u32
    } else {
        0
    };

    Ok(NetworkHealthResponse {
        status: if peer_count > 0 { "connected".into() } else { "disconnected".into() },
        peer_count,
        nat_type: "unknown".into(),
        bandwidth_kbps: 0.0,
        avg_latency_ms: 0.0,
    })
}
#[tauri::command]
async fn get_dht_status(
    app_state: tauri::State<'_, Arc<RwLock<AppState>>>,
) -> Result<Option<serde_json::Value>, String> {
    let state = app_state.inner().read().await;
    if let Some(p2p) = &state.p2p_node {
        let node = p2p.read().await;
        let peer_count = node.get_peer_count().await as u64;
        let uptime = 0u64;
        let status = serde_json::json!({
            "node_id": node.local_peer_id(),
            "peer_count": peer_count,
            "stored_items": 0,
            "network_health": 0.7,
            "uptime": uptime,
            "performance": {
                "avg_lookup_latency": 0,
                "avg_store_latency": 0,
                "operation_success_rate": 0.9,
                "throughput": 0.0,
                "bandwidth_utilization": 0.1,
                "memory_usage": 0
            }
        });
        Ok(Some(status))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn initialize_messaging() -> Result<String, String> {
    Ok("messaging_initialized".to_string())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct MessageRequest {
    recipient: String,
    content: String,
    message_type: String,
    group_id: Option<String>,
}

#[tauri::command]
async fn get_messages(
    _user_id: Option<String>,
    _group_id: Option<String>,
    _limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[tauri::command]
async fn send_group_message(
    request: MessageRequest,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let _ = request;
    Ok(id)
}

#[tauri::command]
async fn create_group(
    name: String,
    description: Option<String>,
    app_state: tauri::State<'_, Arc<RwLock<AppState>>>,
) -> Result<String, String> {
    let state = app_state.inner().write().await;
    let group_id = {
        let mgr = state.group_manager.read().await;
        drop(mgr);
        uuid::Uuid::new_v4().to_string()
    };
    let _ = (name, description);
    Ok(group_id)
}

// Tauri commands
#[tauri::command]
async fn get_node_info(
    app_state: tauri::State<'_, Arc<RwLock<AppState>>>,
) -> Result<serde_json::Value, String> {
    let state = app_state.inner().read().await;

    let peer_count = if let Some(p2p_node) = &state.p2p_node {
        let node = p2p_node.read().await;
        node.get_peer_count().await
    } else {
        0
    };

    Ok(serde_json::json!({
        "peer_count": peer_count,
        "status": "connected",
        "version": "2.0.0"
    }))
}

#[tauri::command]
async fn initialize_p2p_node(
    app_state: tauri::State<'_, Arc<RwLock<AppState>>>,
) -> Result<String, String> {
    info!("Initializing P2P node...");

    // Create node configuration
    let config = NodeConfig {
        listen_addr: "0.0.0.0:0"
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?,
        bootstrap_peers: vec!["bootstrap.communitas.app:8888"
            .parse()
            .map_err(|e| format!("Invalid bootstrap address: {}", e))?],
        ..Default::default()
    };

    // Create and start P2P node
    let mut p2p_node = RealP2PNode::new(config)
        .await
        .map_err(|e| format!("Failed to create P2P node: {}", e))?;

    p2p_node
        .start()
        .await
        .map_err(|e| format!("Failed to start P2P node: {}", e))?;

    let peer_id = p2p_node.local_peer_id().to_string();

    // Store in application state
    let mut state = app_state.inner().write().await;
    state.p2p_node = Some(Arc::new(RwLock::new(p2p_node)));

    info!("P2P node initialized with peer ID: {}", peer_id);
    Ok(peer_id)
}

async fn setup_application_state() -> anyhow::Result<AppState> {
    info!("Setting up application state...");

    // Use a workspace-local data dir for now; in full Tauri app use app.handle().path().app_data_dir()
    let app_data_dir = std::path::PathBuf::from(".communitas-data");
    let _ = tokio::fs::create_dir_all(&app_data_dir).await;

    // Initialize managers
    let _identity_storage = app_data_dir.join("identity");
    let identity_manager = Arc::new(RwLock::new(IdentityManager::new()));
    let contact_manager = Arc::new(RwLock::new(
        init_contact_manager(app_data_dir.clone()).await?,
    ));
    let group_manager = Arc::new(RwLock::new(GroupManager::new()));
    let file_manager = Arc::new(RwLock::new(FileManager::new()));

    info!("Application state setup complete");

    Ok(AppState {
        identity_manager,
        contact_manager,
        group_manager,
        file_manager,
        p2p_node: None,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,communitas=debug,saorsa_core=debug")
        .init();

    info!("Starting Communitas v2.0...");

    // Setup application state
    let app_state = Arc::new(RwLock::new(setup_application_state().await?));

    // Build and run Tauri application
    tauri::Builder::default()
        .manage(app_state.clone())
        .manage({
            let state = app_state.read().await;
            state.contact_manager.clone()
        })
        .invoke_handler(tauri::generate_handler![
            // Node management
            get_node_info,
            initialize_p2p_node,
            // DHT and messaging
            get_dht_status,
            get_network_health,
            initialize_messaging,
            get_messages,
            send_group_message,
            create_group,
            // Contact management commands
            contact_commands::add_contact,
            contact_commands::get_contact,
            contact_commands::get_contact_by_address,
            contact_commands::list_contacts,
            contact_commands::search_contacts,
            contact_commands::create_invitation,
            contact_commands::accept_invitation,
            contact_commands::update_contact_status,
            contact_commands::get_contact_file_system_path,
            contact_commands::generate_four_word_address,
            contact_commands::four_word_encode_address,
            contact_commands::four_word_decode_address,
        ])
        .setup(|_app| {
            info!("Communitas application setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
