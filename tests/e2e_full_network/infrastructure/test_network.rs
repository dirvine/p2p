
//! Test network infrastructure for distributed testing

use anyhow::{Context, Result};
use p2p_core::{
    identity::{EnhancedIdentity, IdentityManager},
    network::{NetworkManager, P2PNode},
    storage::StorageManager,
    dht::DHT,
    transport::Transport,
    tunneling::{TunnelManager, TunnelConfig, TunnelProtocol},
    quantum_crypto::QuantumCrypto,
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use super::test_reporter::{TestReporter, TestEvent, TestEventType};

/// Configuration for distributed test network
#[derive(Clone, Debug)]
pub struct DistributedTestConfig {
    pub local_node_count: usize,
    pub remote_endpoints: Vec<String>,
    pub output_dir: PathBuf,
    pub timeout_secs: u64,
    pub verbose: bool,
    pub ipv6_only: bool,
}

impl Default for DistributedTestConfig {
    fn default() -> Self {
        Self {
            local_node_count: 8,
            remote_endpoints: Vec::new(),
            output_dir: PathBuf::from("./test-reports"),
            timeout_secs: 3600,
            verbose: false,
            ipv6_only: true,
        }
    }
}

/// A distributed test network with multiple nodes
pub struct DistributedTestNetwork {
    pub local_nodes: Vec<TestNode>,
    pub remote_nodes: HashMap<String, RemoteNodeHandle>,
    pub coordinator: NetworkCoordinator,
    pub reporter: TestReporter,
    pub config: DistributedTestConfig,
    start_time: Instant,
}

impl DistributedTestNetwork {
    /// Create a new distributed test network
    pub async fn new(config: DistributedTestConfig) -> Result<Self> {
        let reporter = TestReporter::new(config.verbose).await?;
        let coordinator = NetworkCoordinator::new();
        
        Ok(Self {
            local_nodes: Vec::new(),
            remote_nodes: HashMap::new(),
            coordinator,
            reporter,
            config,
            start_time: Instant::now(),
        })
    }
    
    /// Start all local nodes
    pub async fn start_all_nodes(&mut self) -> Result<()> {
        self.start_local_nodes(self.config.local_node_count).await?;
        
        if !self.config.remote_endpoints.is_empty() {
            self.connect_remote_nodes(self.config.remote_endpoints.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Start local nodes
    pub async fn start_local_nodes(&mut self, count: usize) -> Result<()> {
        for i in 0..count {
            let node = self.create_local_node(i).await
                .with_context(|| format!("Failed to create local node {}", i))?;
            
            self.local_nodes.push(node);
            
            self.reporter.report_progress(TestEvent {
                timestamp: std::time::SystemTime::now(),
                node_id: format!("node_{}", i),
                event_type: TestEventType::NodeStarted,
                details: HashMap::new(),
                success: true,
            }).await;
        }
        
        // Wait for nodes to stabilize
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Connect nodes in mesh topology
        self.establish_mesh_topology().await?;
        
        Ok(())
    }
    
    /// Create a local test node
    async fn create_local_node(&self, index: usize) -> Result<TestNode> {
        // Generate IPv6 address for the node
        let base_port = 9000 + (index * 10);
        let addr = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, index as u16 + 1)),
            base_port as u16,
        );
        
        // Create identity
        let identity = IdentityManager::new()
            .create_identity(Some(format!("TestNode{}", index)))
            .await?;
        
        // Create storage
        let storage_path = self.config.output_dir.join(format!("node_{}", index));
        tokio::fs::create_dir_all(&storage_path).await?;
        let storage = StorageManager::new(storage_path).await?;
        
        // Create DHT
        let dht = DHT::new(identity.clone()).await?;
        
        // Create transport
        let transport = Transport::new_ipv6_only(addr).await?;
        
        // Create tunnel manager
        let tunnel_manager = TunnelManager::new(transport.clone()).await?;
        
        // Create network manager
        let network_manager = NetworkManager::new(
            identity.clone(),
            transport.clone(),
            dht.clone(),
            tunnel_manager.clone(),
        ).await?;
        
        // Create P2P node
        let node = P2PNode::new(
            identity.clone(),
            network_manager.clone(),
            storage.clone(),
            dht.clone(),
        ).await?;
        
        // Start the node
        node.start().await?;
        
        Ok(TestNode {
            node,
            identity,
            storage,
            network_manager,
            tunnel_manager,
            dht,
            transport,
            metrics: Arc::new(Mutex::new(NodeMetrics::default())),
            test_data: Arc::new(RwLock::new(TestDataCollector::new())),
        })
    }
    
    /// Establish mesh topology between local nodes
    async fn establish_mesh_topology(&mut self) -> Result<()> {
        let node_count = self.local_nodes.len();
        
        for i in 0..node_count {
            for j in (i + 1)..node_count {
                let addr_j = self.local_nodes[j].transport.local_addr()?;
                
                self.local_nodes[i].node.connect_peer(addr_j).await
                    .with_context(|| format!("Failed to connect node {} to node {}", i, j))?;
                
                self.reporter.report_progress(TestEvent {
                    timestamp: std::time::SystemTime::now(),
                    node_id: format!("node_{}", i),
                    event_type: TestEventType::ConnectionEstablished,
                    details: {
                        let mut details = HashMap::new();
                        details.insert("peer".to_string(), serde_json::json!(format!("node_{}", j)));
                        details.insert("address".to_string(), serde_json::json!(addr_j.to_string()));
                        details
                    },
                    success: true,
                }).await;
            }
        }
        
        Ok(())
    }
    
    /// Connect to remote test nodes
    pub async fn connect_remote_nodes(&mut self, endpoints: Vec<String>) -> Result<()> {
        for endpoint in endpoints {
            let addr: SocketAddr = endpoint.parse()
                .with_context(|| format!("Invalid endpoint: {}", endpoint))?;
            
            // Ensure IPv6
            if !addr.is_ipv6() {
                return Err(anyhow::anyhow!("Remote endpoint must be IPv6: {}", endpoint));
            }
            
            // Connect first local node to remote
            let remote_info = self.local_nodes[0].node.connect_peer(addr).await
                .with_context(|| format!("Failed to connect to remote node: {}", endpoint))?;
            
            let handle = RemoteNodeHandle {
                endpoint: endpoint.clone(),
                peer_id: remote_info.peer_id,
                three_word_address: remote_info.three_word_address,
                capabilities: remote_info.capabilities,
                status: RemoteNodeStatus::Connected,
                last_seen: Instant::now(),
            };
            
            self.remote_nodes.insert(endpoint.clone(), handle);
            
            self.reporter.report_progress(TestEvent {
                timestamp: std::time::SystemTime::now(),
                node_id: "coordinator".to_string(),
                event_type: TestEventType::ConnectionEstablished,
                details: {
                    let mut details = HashMap::new();
                    details.insert("remote_endpoint".to_string(), serde_json::json!(endpoint));
                    details.insert("peer_id".to_string(), serde_json::json!(remote_info.peer_id));
                    details
                },
                success: true,
            }).await;
        }
        
        Ok(())
    }
    
    /// Verify full mesh connectivity
    pub async fn verify_connectivity(&self) -> Result<ConnectivityReport> {
        let mut report = ConnectivityReport {
            total_nodes: self.local_nodes.len() + self.remote_nodes.len(),
            connected_pairs: 0,
            failed_pairs: Vec::new(),
            average_latency: Duration::default(),
            connectivity_matrix: HashMap::new(),
        };
        
        // Check local node connectivity
        for (i, node_i) in self.local_nodes.iter().enumerate() {
            for (j, node_j) in self.local_nodes.iter().enumerate() {
                if i >= j {
                    continue;
                }
                
                let connected = node_i.network_manager
                    .is_connected(&node_j.identity.base_identity.peer_id)
                    .await;
                
                if connected {
                    report.connected_pairs += 1;
                } else {
                    report.failed_pairs.push((
                        format!("node_{}", i),
                        format!("node_{}", j),
                    ));
                }
                
                report.connectivity_matrix.insert(
                    (format!("node_{}", i), format!("node_{}", j)),
                    connected,
                );
            }
        }
        
        // Check remote connectivity
        for (endpoint, handle) in &self.remote_nodes {
            let connected = matches!(handle.status, RemoteNodeStatus::Connected);
            
            if connected {
                report.connected_pairs += 1;
            } else {
                report.failed_pairs.push((
                    "local_network".to_string(),
                    endpoint.clone(),
                ));
            }
        }
        
        Ok(report)
    }
    
    /// Get current network topology
    pub async fn get_topology(&self) -> NetworkTopology {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        
        // Add local nodes
        for (i, node) in self.local_nodes.iter().enumerate() {
            nodes.push(TopologyNode {
                id: format!("node_{}", i),
                three_word_address: node.identity.base_identity.three_word_address.clone(),
                node_type: NodeType::Local,
                status: NodeStatus::Active,
            });
            
            // Get connections
            let peers = node.network_manager.get_connected_peers().await;
            for peer in peers {
                edges.push(TopologyEdge {
                    from: format!("node_{}", i),
                    to: peer.to_string(),
                    edge_type: EdgeType::Direct,
                    latency: None,
                });
            }
        }
        
        // Add remote nodes
        for (endpoint, handle) in &self.remote_nodes {
            nodes.push(TopologyNode {
                id: endpoint.clone(),
                three_word_address: handle.three_word_address.clone(),
                node_type: NodeType::Remote,
                status: match handle.status {
                    RemoteNodeStatus::Connected => NodeStatus::Active,
                    RemoteNodeStatus::Disconnected => NodeStatus::Inactive,
                    RemoteNodeStatus::Failed => NodeStatus::Failed,
                },
            });
        }
        
        NetworkTopology { nodes, edges }
    }
    
    /// Get network status
    pub fn get_network_status(&self) -> NetworkStatus {
        NetworkStatus {
            local_nodes: self.local_nodes.len(),
            remote_nodes: self.remote_nodes.len(),
            total_connections: self.coordinator.get_total_connections(),
            uptime: self.start_time.elapsed(),
            test_phase: self.coordinator.current_phase.clone(),
        }
    }
    
    /// Stop all nodes
    pub async fn stop_all_nodes(&mut self) -> Result<()> {
        for (i, mut node) in self.local_nodes.drain(..).enumerate() {
            node.node.stop().await
                .with_context(|| format!("Failed to stop node {}", i))?;
        }
        
        Ok(())
    }
    
    /// Get total node count
    pub fn total_nodes(&self) -> usize {
        self.local_nodes.len() + self.remote_nodes.len()
    }
    
    /// Get test duration
    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Join a coordinator for distributed testing
    pub async fn join_coordinator(&mut self, coordinator_addr: SocketAddr, name: Option<String>) -> Result<()> {
        self.coordinator.join_remote(coordinator_addr, name).await
    }
    
    /// Run as a remote node in distributed test
    pub async fn run_as_remote(&mut self) -> Result<()> {
        self.coordinator.run_as_remote().await
    }
}

/// Individual test node wrapper
pub struct TestNode {
    pub node: P2PNode,
    pub identity: Arc<EnhancedIdentity>,
    pub storage: Arc<StorageManager>,
    pub network_manager: Arc<NetworkManager>,
    pub tunnel_manager: Arc<TunnelManager>,
    pub dht: Arc<DHT>,
    pub transport: Arc<Transport>,
    pub metrics: Arc<Mutex<NodeMetrics>>,
    pub test_data: Arc<RwLock<TestDataCollector>>,
}

impl TestNode {
    /// Get node's three-word address
    pub fn three_word_address(&self) -> &str {
        &self.identity.base_identity.three_word_address
    }
    
    /// Update node metrics
    pub async fn update_metrics(&self) {
        let mut metrics = self.metrics.lock().await;
        metrics.messages_sent += 1;
        metrics.last_activity = Instant::now();
    }
}

/// Remote node handle
pub struct RemoteNodeHandle {
    pub endpoint: String,
    pub peer_id: String,
    pub three_word_address: String,
    pub capabilities: HashMap<String, String>,
    pub status: RemoteNodeStatus,
    pub last_seen: Instant,
}

#[derive(Debug, Clone)]
pub enum RemoteNodeStatus {
    Connected,
    Disconnected,
    Failed,
}

/// Network coordinator for distributed tests
pub struct NetworkCoordinator {
    pub current_phase: String,
    total_connections: Arc<Mutex<usize>>,
}

impl NetworkCoordinator {
    pub fn new() -> Self {
        Self {
            current_phase: "initialization".to_string(),
            total_connections: Arc::new(Mutex::new(0)),
        }
    }
    
    pub fn get_total_connections(&self) -> usize {
        self.total_connections.try_lock().map(|g| *g).unwrap_or(0)
    }
    
    pub async fn join_remote(&mut self, _addr: SocketAddr, _name: Option<String>) -> Result<()> {
        // Implementation for joining remote coordinator
        Ok(())
    }
    
    pub async fn run_as_remote(&mut self) -> Result<()> {
        // Implementation for running as remote node
        Ok(())
    }
}

/// Node metrics
#[derive(Default)]
pub struct NodeMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connections: usize,
    pub last_activity: Instant,
}

/// Test data collector
pub struct TestDataCollector {
    pub events: Vec<TestEvent>,
    pub performance_samples: Vec<PerformanceSample>,
}

impl TestDataCollector {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            performance_samples: Vec::new(),
        }
    }
}

/// Performance sample
pub struct PerformanceSample {
    pub timestamp: Instant,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub network_bandwidth: (u64, u64), // (upload, download)
}

/// Connectivity report
pub struct ConnectivityReport {
    pub total_nodes: usize,
    pub connected_pairs: usize,
    pub failed_pairs: Vec<(String, String)>,
    pub average_latency: Duration,
    pub connectivity_matrix: HashMap<(String, String), bool>,
}

/// Network topology representation
pub struct NetworkTopology {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

/// Topology node
pub struct TopologyNode {
    pub id: String,
    pub three_word_address: String,
    pub node_type: NodeType,
    pub status: NodeStatus,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Active,
    Inactive,
    Failed,
}

/// Topology edge
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub latency: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum EdgeType {
    Direct,
    Tunneled,
}

/// Network status
pub struct NetworkStatus {
    pub local_nodes: usize,
    pub remote_nodes: usize,
    pub total_connections: usize,
    pub uptime: Duration,
    pub test_phase: String,
}