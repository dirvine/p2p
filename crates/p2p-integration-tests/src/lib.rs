//! Integration test framework for the adaptive P2P network
//!
//! This module provides comprehensive integration testing capabilities including:
//! - Multi-node test clusters
//! - Network condition simulation
//! - Performance benchmarking
//! - Attack scenario testing
//! - Chaos engineering tools

use saorsa_core::adaptive::*;
use saorsa_core::adaptive::TrustProvider;
use anyhow::{Result, Context};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
    net::SocketAddr,
};
use tokio::sync::{RwLock, mpsc, Barrier};
use futures::future::join_all;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Test cluster configuration
#[derive(Debug, Clone)]
pub struct TestClusterConfig {
    /// Number of nodes in the cluster
    pub node_count: usize,
    
    /// Bootstrap node count
    pub bootstrap_count: usize,
    
    /// Network topology type
    pub topology: NetworkTopology,
    
    /// Simulated network conditions
    pub conditions: NetworkConditions,
    
    /// Enable detailed logging
    pub verbose: bool,
    
    /// Test timeout
    pub timeout: Duration,
}

impl Default for TestClusterConfig {
    fn default() -> Self {
        Self {
            node_count: 10,
            bootstrap_count: 3,
            topology: NetworkTopology::Random,
            conditions: NetworkConditions::default(),
            verbose: false,
            timeout: Duration::from_secs(300),
        }
    }
}

/// Network topology types for testing
#[derive(Debug, Clone, Copy)]
pub enum NetworkTopology {
    /// Random connections
    Random,
    
    /// Ring topology
    Ring,
    
    /// Star topology
    Star,
    
    /// Mesh topology
    Mesh,
    
    /// Hierarchical topology
    Hierarchical,
}

/// Simulated network conditions
#[derive(Debug, Clone)]
pub struct NetworkConditions {
    /// Packet loss rate (0.0 to 1.0)
    pub packet_loss: f64,
    
    /// Latency in milliseconds
    pub latency_ms: u64,
    
    /// Latency variance (jitter)
    pub jitter_ms: u64,
    
    /// Bandwidth limit in Mbps
    pub bandwidth_mbps: u64,
    
    /// Node failure rate per hour
    pub failure_rate: f64,
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self {
            packet_loss: 0.0,
            latency_ms: 10,
            jitter_ms: 2,
            bandwidth_mbps: 100,
            failure_rate: 0.0,
        }
    }
}

/// Test node wrapper
pub struct TestNode {
    /// Node ID
    pub id: String,
    
    /// Node identity
    pub identity: NodeIdentity,
    
    /// Network components
    pub components: NodeComponents,
    
    /// Node address
    pub address: SocketAddr,
    
    /// Node state
    pub state: Arc<RwLock<NodeState>>,
    
    /// Statistics collector
    pub stats: Arc<RwLock<NodeStats>>,
}

/// Node components for testing
pub struct NodeComponents {
    /// DHT instance
    pub dht: Arc<AdaptiveDHT>,
    
    /// Router
    pub router: Arc<AdaptiveRouter>,
    
    /// Gossip protocol
    pub gossip: Arc<AdaptiveGossipSub>,
    
    /// Storage
    pub storage: Arc<ContentStore>,
    
    /// Trust engine
    pub trust: Arc<EigenTrustEngine>,
    
    /// Churn handler
    pub churn: Arc<ChurnHandler>,
    
    /// Monitoring system
    pub monitoring: Arc<MonitoringSystem>,
}

/// Node state for testing
#[derive(Debug, Clone)]
pub struct NodeState {
    /// Is node running
    pub running: bool,
    
    /// Connected peers
    pub peers: Vec<String>,
    
    /// Last heartbeat time
    pub last_heartbeat: Instant,
    
    /// Failure scheduled
    pub scheduled_failure: Option<Instant>,
}

/// Node statistics
#[derive(Debug, Clone, Default)]
pub struct NodeStats {
    /// Messages sent
    pub messages_sent: u64,
    
    /// Messages received
    pub messages_received: u64,
    
    /// Bytes sent
    pub bytes_sent: u64,
    
    /// Bytes received
    pub bytes_received: u64,
    
    /// Storage operations
    pub storage_ops: u64,
    
    /// Retrieval operations
    pub retrieval_ops: u64,
    
    /// Failed operations
    pub failed_ops: u64,
}

/// Test cluster manager
pub struct TestCluster {
    /// Cluster configuration
    pub config: TestClusterConfig,
    
    /// Test nodes
    pub nodes: Arc<RwLock<HashMap<String, Arc<TestNode>>>>,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    
    /// Test start time
    pub start_time: Instant,
    
    /// Shutdown signal
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Arc<RwLock<mpsc::Receiver<()>>>,
}

impl TestCluster {
    /// Create a new test cluster
    pub async fn new(config: TestClusterConfig) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        Ok(Self {
            config,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes: Vec::new(),
            start_time: Instant::now(),
            shutdown_tx,
            shutdown_rx: Arc::new(RwLock::new(shutdown_rx)),
        })
    }
    
    /// Start the test cluster
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting test cluster with {} nodes", self.config.node_count);
        
        // Create bootstrap nodes first
        for i in 0..self.config.bootstrap_count {
            let node = self.create_node(i, true).await?;
            self.bootstrap_nodes.push(node.id.clone());
            self.nodes.write().await.insert(node.id.clone(), Arc::new(node));
        }
        
        // Create remaining nodes
        let barrier = Arc::new(Barrier::new(self.config.node_count - self.config.bootstrap_count));
        let mut handles = vec![];
        
        for i in self.config.bootstrap_count..self.config.node_count {
            let bootstrap = self.bootstrap_nodes.clone();
            let barrier = barrier.clone();
            let nodes = self.nodes.clone();
            
            let handle = tokio::spawn(async move {
                let node = Self::create_node_static(i, false, bootstrap).await.unwrap();
                nodes.write().await.insert(node.id.clone(), Arc::new(node));
                barrier.wait().await;
            });
            
            handles.push(handle);
        }
        
        // Wait for all nodes to be created
        join_all(handles).await;
        
        // Connect nodes according to topology
        self.connect_topology().await?;
        
        info!("Test cluster started successfully");
        Ok(())
    }
    
    /// Create a test node
    async fn create_node(&self, index: usize, is_bootstrap: bool) -> Result<TestNode> {
        Self::create_node_static(index, is_bootstrap, self.bootstrap_nodes.clone()).await
    }
    
    /// Static helper to create nodes
    async fn create_node_static(
        index: usize,
        is_bootstrap: bool,
        bootstrap_nodes: Vec<String>,
    ) -> Result<TestNode> {
        let id = format!("node_{:03}", index);
        let port = 4000 + index as u16;
        let address: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
        
        // Create node identity
        let identity = NodeIdentity::generate().await?;
        
        // Create trust provider  
        let trust_provider: Arc<dyn TrustProvider> = Arc::new(trust::MockTrustProvider::new());
        
        // Create network components
        let hyperbolic = Arc::new(hyperbolic::HyperbolicSpace::new());
        let som = Arc::new(som::SelfOrganizingMap::new(10, 10, 4));
        let router = Arc::new(AdaptiveRouter::new(
            trust_provider.clone(),
            hyperbolic,
            som,
        ));
        
        let node_id = identity.node_id.clone();
        let gossip = Arc::new(AdaptiveGossipSub::new(node_id.clone(), trust_provider.clone()));
        
        let storage_config = StorageConfig::default();
        let storage = Arc::new(ContentStore::new(storage_config).await?);
        
        let dht = Arc::new(AdaptiveDHT::new(
            identity.clone(),
            trust_provider.clone(),
        ));
        
        let trust = Arc::new(EigenTrustEngine::new(vec![node_id.clone()]));
        
        let churn_predictor = Arc::new(learning::ChurnPredictor::new());
        let replication_manager = Arc::new(ReplicationManager::new(
            Default::default(),
            trust_provider.clone(),
            churn_predictor.clone(),
            router.clone(),
        ));
        
        let churn = Arc::new(ChurnHandler::new(
            churn_predictor,
            trust_provider.clone(),
            replication_manager,
            router.clone(),
            gossip.clone(),
            Default::default(),
        ));
        
        let cache_manager = Arc::new(learning::QLearnCacheManager::new(
            storage.get_config().cache_size,
        ));
        
        let monitoring = Arc::new(MonitoringSystem::new(
            monitoring::MonitoredComponents {
                router: router.clone(),
                churn_handler: churn.clone(),
                gossip: gossip.clone(),
                storage: storage.clone(),
                replication: replication_manager,
                thompson: Arc::new(learning::ThompsonSampling::new()),
                cache: cache_manager,
            },
            Default::default(),
        )?);
        
        let components = NodeComponents {
            dht,
            router,
            gossip,
            storage,
            trust,
            churn,
            monitoring,
        };
        
        let state = Arc::new(RwLock::new(NodeState {
            running: true,
            peers: if is_bootstrap { vec![] } else { bootstrap_nodes },
            last_heartbeat: Instant::now(),
            scheduled_failure: None,
        }));
        
        Ok(TestNode {
            id,
            identity,
            components,
            address,
            state,
            stats: Arc::new(RwLock::new(NodeStats::default())),
        })
    }
    
    /// Connect nodes according to topology
    async fn connect_topology(&self) -> Result<()> {
        let nodes = self.nodes.read().await;
        let node_ids: Vec<String> = nodes.keys().cloned().collect();
        
        match self.config.topology {
            NetworkTopology::Random => {
                // Connect each node to 3-5 random peers
                for node_id in &node_ids {
                    let peer_count = rand::random::<usize>() % 3 + 3;
                    let mut peers = vec![];
                    
                    while peers.len() < peer_count {
                        let peer_idx = rand::random::<usize>() % node_ids.len();
                        let peer_id = &node_ids[peer_idx];
                        
                        if peer_id != node_id && !peers.contains(peer_id) {
                            peers.push(peer_id.clone());
                        }
                    }
                    
                    if let Some(node) = nodes.get(node_id) {
                        node.state.write().await.peers = peers;
                    }
                }
            }
            
            NetworkTopology::Ring => {
                // Connect each node to its neighbors in a ring
                for (i, node_id) in node_ids.iter().enumerate() {
                    let prev = &node_ids[(i + node_ids.len() - 1) % node_ids.len()];
                    let next = &node_ids[(i + 1) % node_ids.len()];
                    
                    if let Some(node) = nodes.get(node_id) {
                        node.state.write().await.peers = vec![prev.clone(), next.clone()];
                    }
                }
            }
            
            NetworkTopology::Star => {
                // Connect all nodes to bootstrap nodes
                let hub_nodes = &self.bootstrap_nodes;
                
                for node_id in &node_ids {
                    if !hub_nodes.contains(node_id) {
                        if let Some(node) = nodes.get(node_id) {
                            node.state.write().await.peers = hub_nodes.clone();
                        }
                    }
                }
            }
            
            NetworkTopology::Mesh => {
                // Fully connected mesh
                for node_id in &node_ids {
                    let peers: Vec<String> = node_ids.iter()
                        .filter(|id| *id != node_id)
                        .cloned()
                        .collect();
                    
                    if let Some(node) = nodes.get(node_id) {
                        node.state.write().await.peers = peers;
                    }
                }
            }
            
            NetworkTopology::Hierarchical => {
                // Tree-like hierarchy with bootstrap nodes as roots
                let level_size = 3;
                let mut current_level = self.bootstrap_nodes.clone();
                let mut next_level = vec![];
                let mut assigned = self.bootstrap_nodes.clone();
                
                for node_id in &node_ids {
                    if !assigned.contains(node_id) {
                        // Assign to a parent from current level
                        let parent_idx = next_level.len() % current_level.len();
                        let parent = &current_level[parent_idx];
                        
                        if let Some(node) = nodes.get(node_id) {
                            node.state.write().await.peers = vec![parent.clone()];
                        }
                        
                        next_level.push(node_id.clone());
                        assigned.push(node_id.clone());
                        
                        // Move to next level when current is full
                        if next_level.len() >= current_level.len() * level_size {
                            current_level = next_level.clone();
                            next_level.clear();
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Simulate network conditions
    pub async fn apply_network_conditions(&self) -> Result<()> {
        // In a real implementation, this would use network emulation tools
        // For now, we'll simulate conditions in the application layer
        
        if self.config.conditions.packet_loss > 0.0 {
            debug!("Applying {}% packet loss", self.config.conditions.packet_loss * 100.0);
        }
        
        if self.config.conditions.latency_ms > 0 {
            debug!("Applying {}ms latency", self.config.conditions.latency_ms);
        }
        
        Ok(())
    }
    
    /// Get cluster statistics
    pub async fn get_stats(&self) -> ClusterStats {
        let nodes = self.nodes.read().await;
        let mut total_stats = ClusterStats::default();
        
        for node in nodes.values() {
            let node_stats = node.stats.read().await;
            total_stats.total_messages += node_stats.messages_sent + node_stats.messages_received;
            total_stats.total_bytes += node_stats.bytes_sent + node_stats.bytes_received;
            total_stats.total_operations += node_stats.storage_ops + node_stats.retrieval_ops;
            total_stats.failed_operations += node_stats.failed_ops;
        }
        
        total_stats.node_count = nodes.len();
        total_stats.running_time = self.start_time.elapsed();
        
        total_stats
    }
    
    /// Shutdown the cluster
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down test cluster");
        
        // Send shutdown signal
        let _ = self.shutdown_tx.send(()).await;
        
        // Stop all nodes
        let nodes = self.nodes.read().await;
        for node in nodes.values() {
            node.state.write().await.running = false;
            node.components.monitoring.stop().await;
            node.components.churn.stop_monitoring().await;
        }
        
        Ok(())
    }
    
    /// Wait for cluster to stabilize
    pub async fn wait_for_stabilization(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        
        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Cluster stabilization timeout"));
            }
            
            // Check if all nodes have discovered each other
            let nodes = self.nodes.read().await;
            let mut all_connected = true;
            
            for node in nodes.values() {
                let health = node.components.monitoring.get_health().await;
                if health.score < 0.8 {
                    all_connected = false;
                    break;
                }
            }
            
            if all_connected {
                info!("Cluster stabilized after {:?}", start.elapsed());
                return Ok(());
            }
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// Cluster-wide statistics
#[derive(Debug, Clone, Default)]
pub struct ClusterStats {
    /// Number of nodes
    pub node_count: usize,
    
    /// Total messages exchanged
    pub total_messages: u64,
    
    /// Total bytes transferred
    pub total_bytes: u64,
    
    /// Total operations performed
    pub total_operations: u64,
    
    /// Failed operations
    pub failed_operations: u64,
    
    /// Running time
    pub running_time: Duration,
}

impl ClusterStats {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            1.0
        } else {
            1.0 - (self.failed_operations as f64 / self.total_operations as f64)
        }
    }
    
    /// Calculate messages per second
    pub fn messages_per_second(&self) -> f64 {
        if self.running_time.as_secs() == 0 {
            0.0
        } else {
            self.total_messages as f64 / self.running_time.as_secs_f64()
        }
    }
    
    /// Calculate throughput in MB/s
    pub fn throughput_mbps(&self) -> f64 {
        if self.running_time.as_secs() == 0 {
            0.0
        } else {
            (self.total_bytes as f64 / 1024.0 / 1024.0) / self.running_time.as_secs_f64()
        }
    }
}

/// Test utilities
pub mod utils {
    use super::*;
    
    /// Generate random content of specified size
    pub fn generate_content(size: usize) -> Vec<u8> {
        use rand::RngCore;
        let mut content = vec![0u8; size];
        rand::thread_rng().fill_bytes(&mut content);
        content
    }
    
    /// Measure operation latency
    pub async fn measure_latency<F, Fut, T>(operation: F) -> (T, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        (result, duration)
    }
    
    /// Run operations in parallel
    pub async fn parallel_operations<F, Fut, T>(
        count: usize,
        operation: F,
    ) -> Vec<Result<T>>
    where
        F: Fn(usize) -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send + 'static,
    {
        let mut handles = vec![];
        
        for i in 0..count {
            let op = operation.clone();
            let handle = tokio::spawn(async move {
                op(i).await
            });
            handles.push(handle);
        }
        
        let mut results = vec![];
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(anyhow::anyhow!("Task failed: {}", e))),
            }
        }
        
        results
    }
    
    /// Calculate statistics from latency measurements
    pub fn calculate_latency_stats(latencies: &[Duration]) -> LatencyStats {
        if latencies.is_empty() {
            return LatencyStats::default();
        }
        
        let mut sorted = latencies.to_vec();
        sorted.sort();
        
        let sum: Duration = sorted.iter().sum();
        let avg = sum / sorted.len() as u32;
        
        let p50_idx = sorted.len() / 2;
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        
        LatencyStats {
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            avg,
            p50: sorted[p50_idx],
            p95: sorted[p95_idx.min(sorted.len() - 1)],
            p99: sorted[p99_idx.min(sorted.len() - 1)],
        }
    }
}

/// Latency statistics
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}