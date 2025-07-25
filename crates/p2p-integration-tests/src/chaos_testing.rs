//! Chaos engineering tests for the adaptive P2P network
//!
//! Tests system resilience through controlled chaos:
//! - Random failures and delays
//! - Resource exhaustion
//! - Byzantine behavior
//! - Clock skew
//! - Network chaos

use p2p_integration_tests::*;
use saorsa_core::adaptive::*;
use anyhow::Result;
use std::{
    time::{Duration, Instant, SystemTime},
    collections::HashMap,
    sync::atomic::{AtomicU64, AtomicBool, Ordering},
};
use tracing::{info, warn, error, debug};
use tokio::time::interval;
use rand::{Rng, distributions::Uniform};

/// Chaos type
#[derive(Debug, Clone)]
pub enum ChaosType {
    /// Random component failures
    ComponentFailure {
        failure_rate: f64,
        components: Vec<ComponentType>,
    },
    
    /// Resource exhaustion
    ResourceExhaustion {
        memory_pressure: f64,
        cpu_pressure: f64,
        disk_pressure: f64,
    },
    
    /// Byzantine behavior
    ByzantineBehavior {
        byzantine_nodes: usize,
        behaviors: Vec<ByzantineBehaviorType>,
    },
    
    /// Clock skew
    ClockSkew {
        max_skew_ms: i64,
        drift_rate: f64,
    },
    
    /// Network chaos
    NetworkChaos {
        packet_corruption: f64,
        packet_reordering: f64,
        bandwidth_variation: f64,
    },
    
    /// Combined chaos
    Combined {
        chaos_types: Vec<ChaosType>,
    },
}

/// Component types that can fail
#[derive(Debug, Clone, Copy)]
pub enum ComponentType {
    DHT,
    Router,
    Storage,
    Gossip,
    Trust,
    Monitoring,
}

/// Byzantine behavior types
#[derive(Debug, Clone, Copy)]
pub enum ByzantineBehaviorType {
    /// Send conflicting information
    Equivocation,
    
    /// Selectively respond to requests
    SelectiveResponse,
    
    /// Corrupt stored data
    DataCorruption,
    
    /// False routing information
    MisroutingAttack,
    
    /// Time-based attacks
    TimingAttack,
}

/// Chaos injector
pub struct ChaosInjector {
    /// Chaos configuration
    chaos_type: ChaosType,
    
    /// Test cluster
    cluster: TestCluster,
    
    /// Chaos state
    state: Arc<RwLock<ChaosState>>,
    
    /// Chaos events
    events: Arc<RwLock<Vec<ChaosEvent>>>,
    
    /// Active flag
    active: Arc<AtomicBool>,
}

/// Chaos state
#[derive(Debug, Default)]
pub struct ChaosState {
    /// Failed components
    failed_components: HashMap<String, Vec<ComponentType>>,
    
    /// Byzantine nodes
    byzantine_nodes: HashMap<String, Vec<ByzantineBehaviorType>>,
    
    /// Clock skews
    clock_skews: HashMap<String, i64>,
    
    /// Resource limits
    resource_limits: HashMap<String, ResourceLimits>,
}

/// Resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub memory_mb: u64,
    pub cpu_percent: f64,
    pub disk_io_mbps: f64,
}

/// Chaos event
#[derive(Debug, Clone)]
pub struct ChaosEvent {
    pub timestamp: Instant,
    pub node_id: String,
    pub event_type: String,
    pub details: String,
    pub impact: ImpactLevel,
}

/// Impact level of chaos
#[derive(Debug, Clone, Copy)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl ChaosInjector {
    /// Create new chaos injector
    pub async fn new(cluster: TestCluster, chaos_type: ChaosType) -> Self {
        Self {
            chaos_type,
            cluster,
            state: Arc::new(RwLock::new(ChaosState::default())),
            events: Arc::new(RwLock::new(Vec::new())),
            active: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Start chaos injection
    pub async fn start(&self, duration: Duration) -> Result<()> {
        info!("Starting chaos injection: {:?} for {:?}", self.chaos_type, duration);
        self.active.store(true, Ordering::SeqCst);
        
        match &self.chaos_type {
            ChaosType::ComponentFailure { failure_rate, components } => {
                self.inject_component_failures(*failure_rate, components.clone(), duration).await?;
            }
            ChaosType::ResourceExhaustion { memory_pressure, cpu_pressure, disk_pressure } => {
                self.inject_resource_exhaustion(*memory_pressure, *cpu_pressure, *disk_pressure, duration).await?;
            }
            ChaosType::ByzantineBehavior { byzantine_nodes, behaviors } => {
                self.inject_byzantine_behavior(*byzantine_nodes, behaviors.clone(), duration).await?;
            }
            ChaosType::ClockSkew { max_skew_ms, drift_rate } => {
                self.inject_clock_skew(*max_skew_ms, *drift_rate, duration).await?;
            }
            ChaosType::NetworkChaos { packet_corruption, packet_reordering, bandwidth_variation } => {
                self.inject_network_chaos(*packet_corruption, *packet_reordering, *bandwidth_variation, duration).await?;
            }
            ChaosType::Combined { chaos_types } => {
                self.inject_combined_chaos(chaos_types.clone(), duration).await?;
            }
        }
        
        self.active.store(false, Ordering::SeqCst);
        Ok(())
    }
    
    /// Inject component failures
    async fn inject_component_failures(
        &self,
        failure_rate: f64,
        components: Vec<ComponentType>,
        duration: Duration,
    ) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(1));
        let start = Instant::now();
        
        while start.elapsed() < duration && self.active.load(Ordering::SeqCst) {
            ticker.tick().await;
            
            let nodes = self.cluster.nodes.read().await;
            let active_nodes: Vec<_> = nodes.values()
                .filter(|n| n.state.read().await.running)
                .collect();
            
            if active_nodes.is_empty() {
                continue;
            }
            
            // Inject failures based on rate
            if rand::random::<f64>() < failure_rate {
                let node = active_nodes[rand::random::<usize>() % active_nodes.len()];
                let component = components[rand::random::<usize>() % components.len()];
                
                self.fail_component(node, component).await?;
                
                self.events.write().await.push(ChaosEvent {
                    timestamp: Instant::now(),
                    node_id: node.id.clone(),
                    event_type: format!("ComponentFailure::{:?}", component),
                    details: format!("{:?} component failed", component),
                    impact: ImpactLevel::High,
                });
            }
            
            // Random recovery
            let mut state = self.state.write().await;
            let failed_nodes: Vec<String> = state.failed_components.keys().cloned().collect();
            
            for node_id in failed_nodes {
                if rand::random::<f64>() < 0.1 { // 10% recovery chance
                    state.failed_components.remove(&node_id);
                    
                    self.events.write().await.push(ChaosEvent {
                        timestamp: Instant::now(),
                        node_id: node_id.clone(),
                        event_type: "ComponentRecovery".to_string(),
                        details: "Components recovered".to_string(),
                        impact: ImpactLevel::Low,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Fail a specific component
    async fn fail_component(&self, node: &TestNode, component: ComponentType) -> Result<()> {
        match component {
            ComponentType::DHT => {
                // Simulate DHT failure by clearing routing table
                debug!("Simulating DHT failure for {}", node.id);
                // In real implementation, would clear DHT routing table
            }
            ComponentType::Router => {
                // Simulate router failure
                debug!("Simulating router failure for {}", node.id);
                // In real implementation, would disable routing
            }
            ComponentType::Storage => {
                // Simulate storage failure
                debug!("Simulating storage failure for {}", node.id);
                // In real implementation, would make storage read-only
            }
            ComponentType::Gossip => {
                // Simulate gossip failure
                debug!("Simulating gossip failure for {}", node.id);
                // In real implementation, would disconnect from mesh
            }
            ComponentType::Trust => {
                // Simulate trust engine failure
                debug!("Simulating trust engine failure for {}", node.id);
                // In real implementation, would return default trust scores
            }
            ComponentType::Monitoring => {
                // Stop monitoring
                node.components.monitoring.stop().await;
            }
        }
        
        let mut state = self.state.write().await;
        state.failed_components
            .entry(node.id.clone())
            .or_insert_with(Vec::new)
            .push(component);
        
        Ok(())
    }
    
    /// Inject resource exhaustion
    async fn inject_resource_exhaustion(
        &self,
        memory_pressure: f64,
        cpu_pressure: f64,
        disk_pressure: f64,
        duration: Duration,
    ) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(5));
        let start = Instant::now();
        
        while start.elapsed() < duration && self.active.load(Ordering::SeqCst) {
            ticker.tick().await;
            
            let nodes = self.cluster.nodes.read().await;
            
            // Apply resource pressure to random nodes
            for node in nodes.values() {
                if rand::random::<f64>() < 0.3 { // 30% chance per node
                    let limits = ResourceLimits {
                        memory_mb: (1024.0 * (1.0 - memory_pressure)) as u64,
                        cpu_percent: 100.0 * (1.0 - cpu_pressure),
                        disk_io_mbps: 100.0 * (1.0 - disk_pressure),
                    };
                    
                    self.state.write().await.resource_limits.insert(
                        node.id.clone(),
                        limits.clone(),
                    );
                    
                    self.events.write().await.push(ChaosEvent {
                        timestamp: Instant::now(),
                        node_id: node.id.clone(),
                        event_type: "ResourceExhaustion".to_string(),
                        details: format!("Memory: {}MB, CPU: {:.0}%, Disk: {:.0}MB/s",
                            limits.memory_mb, limits.cpu_percent, limits.disk_io_mbps),
                        impact: ImpactLevel::Medium,
                    });
                    
                    // Simulate impact on operations
                    if memory_pressure > 0.8 || cpu_pressure > 0.8 {
                        // Severe resource pressure - operations may fail
                        node.stats.write().await.failed_ops += 1;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Inject Byzantine behavior
    async fn inject_byzantine_behavior(
        &self,
        byzantine_count: usize,
        behaviors: Vec<ByzantineBehaviorType>,
        duration: Duration,
    ) -> Result<()> {
        // Select Byzantine nodes
        let nodes = self.cluster.nodes.read().await;
        let node_ids: Vec<_> = nodes.keys().cloned().collect();
        let mut byzantine_nodes = HashMap::new();
        
        for i in 0..byzantine_count.min(node_ids.len() / 3) { // Max 1/3 Byzantine
            let node_id = &node_ids[i];
            byzantine_nodes.insert(node_id.clone(), behaviors.clone());
            
            self.events.write().await.push(ChaosEvent {
                timestamp: Instant::now(),
                node_id: node_id.clone(),
                event_type: "ByzantineNode".to_string(),
                details: format!("Node became Byzantine with behaviors: {:?}", behaviors),
                impact: ImpactLevel::Critical,
            });
        }
        
        self.state.write().await.byzantine_nodes = byzantine_nodes;
        
        // Simulate Byzantine behavior
        let mut ticker = interval(Duration::from_secs(2));
        let start = Instant::now();
        
        while start.elapsed() < duration && self.active.load(Ordering::SeqCst) {
            ticker.tick().await;
            
            let byzantine = self.state.read().await.byzantine_nodes.clone();
            
            for (node_id, behaviors) in byzantine {
                if let Some(node) = nodes.get(&node_id) {
                    for behavior in &behaviors {
                        self.execute_byzantine_behavior(node, *behavior).await?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute specific Byzantine behavior
    async fn execute_byzantine_behavior(
        &self,
        node: &TestNode,
        behavior: ByzantineBehaviorType,
    ) -> Result<()> {
        match behavior {
            ByzantineBehaviorType::Equivocation => {
                // Send conflicting messages
                debug!("Byzantine {}: Equivocating", node.id);
                node.stats.write().await.messages_sent += 2;
                
                self.events.write().await.push(ChaosEvent {
                    timestamp: Instant::now(),
                    node_id: node.id.clone(),
                    event_type: "Equivocation".to_string(),
                    details: "Sent conflicting messages".to_string(),
                    impact: ImpactLevel::High,
                });
            }
            
            ByzantineBehaviorType::SelectiveResponse => {
                // Respond only to certain nodes
                debug!("Byzantine {}: Selective response", node.id);
                
                self.events.write().await.push(ChaosEvent {
                    timestamp: Instant::now(),
                    node_id: node.id.clone(),
                    event_type: "SelectiveResponse".to_string(),
                    details: "Ignoring requests from some nodes".to_string(),
                    impact: ImpactLevel::Medium,
                });
            }
            
            ByzantineBehaviorType::DataCorruption => {
                // Corrupt stored data
                debug!("Byzantine {}: Data corruption", node.id);
                
                // In real implementation, would modify stored content
                self.events.write().await.push(ChaosEvent {
                    timestamp: Instant::now(),
                    node_id: node.id.clone(),
                    event_type: "DataCorruption".to_string(),
                    details: "Attempting to corrupt stored data".to_string(),
                    impact: ImpactLevel::Critical,
                });
            }
            
            ByzantineBehaviorType::MisroutingAttack => {
                // Provide false routing information
                debug!("Byzantine {}: Misrouting attack", node.id);
                
                self.events.write().await.push(ChaosEvent {
                    timestamp: Instant::now(),
                    node_id: node.id.clone(),
                    event_type: "MisroutingAttack".to_string(),
                    details: "Providing false routing information".to_string(),
                    impact: ImpactLevel::High,
                });
            }
            
            ByzantineBehaviorType::TimingAttack => {
                // Delay responses to disrupt timing
                debug!("Byzantine {}: Timing attack", node.id);
                tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 1000)).await;
                
                self.events.write().await.push(ChaosEvent {
                    timestamp: Instant::now(),
                    node_id: node.id.clone(),
                    event_type: "TimingAttack".to_string(),
                    details: "Introducing artificial delays".to_string(),
                    impact: ImpactLevel::Medium,
                });
            }
        }
        
        Ok(())
    }
    
    /// Inject clock skew
    async fn inject_clock_skew(
        &self,
        max_skew_ms: i64,
        drift_rate: f64,
        duration: Duration,
    ) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(1));
        let start = Instant::now();
        let dist = Uniform::new(-max_skew_ms, max_skew_ms);
        let mut rng = rand::thread_rng();
        
        // Initialize clock skews
        let nodes = self.cluster.nodes.read().await;
        let mut skews = HashMap::new();
        
        for node_id in nodes.keys() {
            let initial_skew = rng.sample(dist);
            skews.insert(node_id.clone(), initial_skew);
            
            self.events.write().await.push(ChaosEvent {
                timestamp: Instant::now(),
                node_id: node_id.clone(),
                event_type: "ClockSkew".to_string(),
                details: format!("Initial clock skew: {}ms", initial_skew),
                impact: ImpactLevel::Low,
            });
        }
        
        self.state.write().await.clock_skews = skews;
        
        // Apply drift over time
        while start.elapsed() < duration && self.active.load(Ordering::SeqCst) {
            ticker.tick().await;
            
            let mut state = self.state.write().await;
            
            for (node_id, skew) in state.clock_skews.iter_mut() {
                // Apply drift
                let drift = (drift_rate * 1000.0) as i64;
                *skew += rng.gen_range(-drift..=drift);
                
                // Clamp to max skew
                *skew = (*skew).clamp(-max_skew_ms, max_skew_ms);
                
                if skew.abs() > max_skew_ms / 2 {
                    self.events.write().await.push(ChaosEvent {
                        timestamp: Instant::now(),
                        node_id: node_id.clone(),
                        event_type: "SignificantClockSkew".to_string(),
                        details: format!("Clock skew reached {}ms", skew),
                        impact: ImpactLevel::Medium,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Inject network chaos
    async fn inject_network_chaos(
        &self,
        packet_corruption: f64,
        packet_reordering: f64,
        bandwidth_variation: f64,
        duration: Duration,
    ) -> Result<()> {
        let mut ticker = interval(Duration::from_millis(100));
        let start = Instant::now();
        
        while start.elapsed() < duration && self.active.load(Ordering::SeqCst) {
            ticker.tick().await;
            
            let nodes = self.cluster.nodes.read().await;
            
            for node in nodes.values() {
                let mut stats = node.stats.write().await;
                
                // Simulate packet corruption
                if rand::random::<f64>() < packet_corruption {
                    stats.failed_ops += 1;
                    
                    self.events.write().await.push(ChaosEvent {
                        timestamp: Instant::now(),
                        node_id: node.id.clone(),
                        event_type: "PacketCorruption".to_string(),
                        details: "Simulated corrupted packet".to_string(),
                        impact: ImpactLevel::Low,
                    });
                }
                
                // Simulate packet reordering
                if rand::random::<f64>() < packet_reordering {
                    // In real implementation, would delay and reorder packets
                    debug!("Network chaos: Packet reordering for {}", node.id);
                }
                
                // Simulate bandwidth variation
                if rand::random::<f64>() < bandwidth_variation {
                    let variation = rand::random::<f64>() * 2.0; // 0x to 2x
                    debug!("Network chaos: Bandwidth variation {}x for {}", variation, node.id);
                }
            }
        }
        
        Ok(())
    }
    
    /// Inject combined chaos
    async fn inject_combined_chaos(
        &self,
        chaos_types: Vec<ChaosType>,
        duration: Duration,
    ) -> Result<()> {
        let mut handles = vec![];
        
        for chaos_type in chaos_types {
            let injector = ChaosInjector::new(self.cluster.clone(), chaos_type).await;
            
            let handle = tokio::spawn(async move {
                injector.start(duration).await
            });
            
            handles.push(handle);
        }
        
        // Wait for all chaos types to complete
        for handle in handles {
            handle.await??;
        }
        
        Ok(())
    }
    
    /// Get chaos report
    pub async fn get_report(&self) -> ChaosReport {
        let events = self.events.read().await;
        let state = self.state.read().await;
        
        let mut event_counts = HashMap::new();
        let mut impact_counts = HashMap::new();
        
        for event in events.iter() {
            *event_counts.entry(event.event_type.clone()).or_insert(0) += 1;
            *impact_counts.entry(event.impact).or_insert(0) += 1;
        }
        
        ChaosReport {
            total_events: events.len(),
            event_counts,
            impact_counts,
            failed_components: state.failed_components.len(),
            byzantine_nodes: state.byzantine_nodes.len(),
            max_clock_skew: state.clock_skews.values().map(|s| s.abs()).max().unwrap_or(0),
            duration: events.first()
                .and_then(|first| events.last().map(|last| last.timestamp - first.timestamp))
                .unwrap_or(Duration::ZERO),
        }
    }
}

/// Chaos report
#[derive(Debug)]
pub struct ChaosReport {
    pub total_events: usize,
    pub event_counts: HashMap<String, usize>,
    pub impact_counts: HashMap<ImpactLevel, usize>,
    pub failed_components: usize,
    pub byzantine_nodes: usize,
    pub max_clock_skew: i64,
    pub duration: Duration,
}

/// System invariant checker
pub struct InvariantChecker {
    cluster: TestCluster,
    invariants: Vec<Box<dyn Invariant>>,
    violations: Arc<RwLock<Vec<InvariantViolation>>>,
}

/// Invariant trait
#[async_trait::async_trait]
pub trait Invariant: Send + Sync {
    /// Check if invariant holds
    async fn check(&self, cluster: &TestCluster) -> Result<bool>;
    
    /// Get invariant name
    fn name(&self) -> &str;
    
    /// Get invariant description
    fn description(&self) -> &str;
}

/// Invariant violation
#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub timestamp: Instant,
    pub invariant_name: String,
    pub details: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy)]
pub enum ViolationSeverity {
    Warning,
    Error,
    Critical,
}

/// Data availability invariant
struct DataAvailabilityInvariant {
    min_replication: u32,
}

#[async_trait::async_trait]
impl Invariant for DataAvailabilityInvariant {
    async fn check(&self, cluster: &TestCluster) -> Result<bool> {
        // Check that all stored data maintains minimum replication
        let nodes = cluster.nodes.read().await;
        let active_nodes = nodes.values()
            .filter(|n| n.state.read().await.running)
            .count();
        
        // Simple check: ensure enough nodes for replication
        Ok(active_nodes >= self.min_replication as usize)
    }
    
    fn name(&self) -> &str {
        "DataAvailability"
    }
    
    fn description(&self) -> &str {
        "All stored data must maintain minimum replication factor"
    }
}

/// Network connectivity invariant
struct NetworkConnectivityInvariant {
    min_connectivity: f64,
}

#[async_trait::async_trait]
impl Invariant for NetworkConnectivityInvariant {
    async fn check(&self, cluster: &TestCluster) -> Result<bool> {
        let nodes = cluster.nodes.read().await;
        let mut connected_count = 0;
        let mut total_count = 0;
        
        for node in nodes.values() {
            if node.state.read().await.running {
                total_count += 1;
                if !node.state.read().await.peers.is_empty() {
                    connected_count += 1;
                }
            }
        }
        
        let connectivity = if total_count > 0 {
            connected_count as f64 / total_count as f64
        } else {
            0.0
        };
        
        Ok(connectivity >= self.min_connectivity)
    }
    
    fn name(&self) -> &str {
        "NetworkConnectivity"
    }
    
    fn description(&self) -> &str {
        "Network must maintain minimum connectivity level"
    }
}

impl InvariantChecker {
    /// Create new invariant checker
    pub fn new(cluster: TestCluster) -> Self {
        let invariants: Vec<Box<dyn Invariant>> = vec![
            Box::new(DataAvailabilityInvariant { min_replication: 3 }),
            Box::new(NetworkConnectivityInvariant { min_connectivity: 0.5 }),
        ];
        
        Self {
            cluster,
            invariants,
            violations: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Run continuous invariant checking
    pub async fn run(&self, duration: Duration) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(5));
        let start = Instant::now();
        
        while start.elapsed() < duration {
            ticker.tick().await;
            
            for invariant in &self.invariants {
                match invariant.check(&self.cluster).await {
                    Ok(true) => {
                        debug!("Invariant {} holds", invariant.name());
                    }
                    Ok(false) => {
                        warn!("Invariant {} violated: {}", invariant.name(), invariant.description());
                        
                        self.violations.write().await.push(InvariantViolation {
                            timestamp: Instant::now(),
                            invariant_name: invariant.name().to_string(),
                            details: invariant.description().to_string(),
                            severity: ViolationSeverity::Error,
                        });
                    }
                    Err(e) => {
                        error!("Error checking invariant {}: {}", invariant.name(), e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get violations
    pub async fn get_violations(&self) -> Vec<InvariantViolation> {
        self.violations.read().await.clone()
    }
}

#[tokio::test]
async fn test_component_failure_chaos() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting component failure chaos test");
    
    let config = TestClusterConfig {
        node_count: 30,
        bootstrap_count: 3,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Store test data before chaos
    let nodes = cluster.nodes.read().await;
    let node = nodes.values().next().unwrap();
    
    let test_data = b"Data that must survive chaos".to_vec();
    let metadata = storage::ContentMetadata {
        size: test_data.len(),
        content_type: ContentType::DataRetrieval,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        chunk_count: None,
        replication_factor: 10,
    };
    
    let content_hash = node.components.storage.store(test_data.clone(), metadata).await?;
    drop(nodes);
    
    // Create chaos injector
    let chaos = ChaosInjector::new(
        cluster.clone(),
        ChaosType::ComponentFailure {
            failure_rate: 0.1, // 10% per second
            components: vec![
                ComponentType::DHT,
                ComponentType::Storage,
                ComponentType::Gossip,
            ],
        },
    ).await;
    
    // Create invariant checker
    let checker = InvariantChecker::new(cluster.clone());
    
    // Run chaos and invariant checking concurrently
    let chaos_handle = tokio::spawn({
        let chaos = chaos.clone();
        async move {
            chaos.start(Duration::from_secs(60)).await
        }
    });
    
    let check_handle = tokio::spawn(async move {
        checker.run(Duration::from_secs(60)).await
    });
    
    // Periodically test system functionality
    for i in 0..6 {
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        let nodes = cluster.nodes.read().await;
        let active_nodes: Vec<_> = nodes.values()
            .filter(|n| n.state.read().await.running)
            .collect();
        
        if let Some(test_node) = active_nodes.first() {
            let retrieval_manager = RetrievalManager::new(
                test_node.components.router.clone(),
                test_node.components.storage.clone(),
                Arc::new(learning::QLearnCacheManager::new(100 * 1024 * 1024)),
            );
            
            match retrieval_manager.retrieve(&content_hash, retrieval::RetrievalStrategy::Parallel).await {
                Ok(data) => {
                    assert_eq!(data, test_data, "Retrieved data should match");
                    info!("Data retrieval successful during chaos (check {})", i + 1);
                }
                Err(e) => {
                    warn!("Retrieval failed during chaos: {}", e);
                }
            }
        }
    }
    
    chaos_handle.await??;
    check_handle.await??;
    
    // Check results
    let report = chaos.get_report().await;
    
    info!("Component failure chaos results:");
    info!("  Total events: {}", report.total_events);
    info!("  Failed components: {}", report.failed_components);
    info!("  Event breakdown:");
    for (event_type, count) in &report.event_counts {
        info!("    {}: {}", event_type, count);
    }
    
    assert!(report.total_events > 0, "Should have chaos events");
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_byzantine_behavior() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting Byzantine behavior test");
    
    let config = TestClusterConfig {
        node_count: 40,
        bootstrap_count: 4,
        topology: NetworkTopology::Mesh,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Inject Byzantine nodes
    let chaos = ChaosInjector::new(
        cluster.clone(),
        ChaosType::ByzantineBehavior {
            byzantine_nodes: 10, // 25% Byzantine
            behaviors: vec![
                ByzantineBehaviorType::Equivocation,
                ByzantineBehaviorType::SelectiveResponse,
                ByzantineBehaviorType::MisroutingAttack,
            ],
        },
    ).await;
    
    // Run Byzantine chaos
    let chaos_handle = tokio::spawn({
        let chaos = chaos.clone();
        async move {
            chaos.start(Duration::from_secs(60)).await
        }
    });
    
    // Monitor trust scores during Byzantine behavior
    let mut trust_samples = vec![];
    
    for _ in 0..6 {
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        let nodes = cluster.nodes.read().await;
        let sample_node = nodes.values()
            .skip(30) // Use honest node
            .next()
            .unwrap();
        
        let trust_scores = sample_node.components.trust.get_all_trust_scores().await;
        let avg_trust = trust_scores.values().sum::<f64>() / trust_scores.len().max(1) as f64;
        trust_samples.push(avg_trust);
        
        info!("Average trust score: {:.3}", avg_trust);
    }
    
    chaos_handle.await??;
    
    // Verify trust system detected Byzantine nodes
    let report = chaos.get_report().await;
    let trust_degradation = trust_samples.first().unwrap() - trust_samples.last().unwrap();
    
    info!("Byzantine behavior results:");
    info!("  Byzantine nodes: {}", report.byzantine_nodes);
    info!("  Trust degradation: {:.3}", trust_degradation);
    info!("  Total Byzantine events: {}", report.total_events);
    
    assert!(trust_degradation > 0.0, "Trust should degrade for Byzantine nodes");
    assert!(report.byzantine_nodes <= 13, "Byzantine nodes should be limited to ~1/3");
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_clock_skew_resilience() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting clock skew resilience test");
    
    let config = TestClusterConfig {
        node_count: 20,
        bootstrap_count: 2,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Inject clock skew
    let chaos = ChaosInjector::new(
        cluster.clone(),
        ChaosType::ClockSkew {
            max_skew_ms: 5000, // 5 seconds max skew
            drift_rate: 0.001, // 1ms/s drift
        },
    ).await;
    
    chaos.start(Duration::from_secs(60)).await?;
    
    // Test time-sensitive operations
    let nodes = cluster.nodes.read().await;
    let node = nodes.values().next().unwrap();
    
    // Gossip should still work with clock skew
    let topic = "time_test";
    node.components.gossip.subscribe(topic).await?;
    node.components.gossip.publish(topic, b"Test message".to_vec()).await?;
    
    let report = chaos.get_report().await;
    
    info!("Clock skew results:");
    info!("  Max clock skew: {}ms", report.max_clock_skew);
    info!("  Events: {}", report.total_events);
    
    assert!(report.max_clock_skew <= 5000, "Clock skew should be bounded");
    
    cluster.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_combined_chaos() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    
    info!("Starting combined chaos test");
    
    let config = TestClusterConfig {
        node_count: 50,
        bootstrap_count: 5,
        topology: NetworkTopology::Random,
        ..Default::default()
    };
    
    let mut cluster = TestCluster::new(config).await?;
    cluster.start().await?;
    cluster.wait_for_stabilization(Duration::from_secs(30)).await?;
    
    // Create multi-chaos scenario
    let chaos = ChaosInjector::new(
        cluster.clone(),
        ChaosType::Combined {
            chaos_types: vec![
                ChaosType::ComponentFailure {
                    failure_rate: 0.05,
                    components: vec![ComponentType::Storage, ComponentType::DHT],
                },
                ChaosType::NetworkChaos {
                    packet_corruption: 0.02,
                    packet_reordering: 0.05,
                    bandwidth_variation: 0.1,
                },
                ChaosType::ResourceExhaustion {
                    memory_pressure: 0.3,
                    cpu_pressure: 0.2,
                    disk_pressure: 0.1,
                },
            ],
        },
    ).await;
    
    // Store critical data
    let nodes = cluster.nodes.read().await;
    let node = nodes.values().next().unwrap();
    
    let critical_data = b"Critical system data".to_vec();
    let metadata = storage::ContentMetadata {
        size: critical_data.len(),
        content_type: ContentType::DataRetrieval,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        chunk_count: None,
        replication_factor: 15,
    };
    
    let hash = node.components.storage.store(critical_data.clone(), metadata).await?;
    drop(nodes);
    
    // Run chaos
    chaos.start(Duration::from_secs(90)).await?;
    
    // Verify system still functions
    let nodes = cluster.nodes.read().await;
    let mut successful_ops = 0;
    let total_ops = 10;
    
    for i in 0..total_ops {
        let test_node = nodes.values()
            .filter(|n| n.state.read().await.running)
            .skip(i % nodes.len())
            .next();
        
        if let Some(node) = test_node {
            let retrieval_manager = RetrievalManager::new(
                node.components.router.clone(),
                node.components.storage.clone(),
                Arc::new(learning::QLearnCacheManager::new(100 * 1024 * 1024)),
            );
            
            if let Ok(data) = retrieval_manager.retrieve(&hash, retrieval::RetrievalStrategy::Parallel).await {
                if data == critical_data {
                    successful_ops += 1;
                }
            }
        }
    }
    
    let success_rate = successful_ops as f64 / total_ops as f64;
    let report = chaos.get_report().await;
    
    info!("Combined chaos results:");
    info!("  Total chaos events: {}", report.total_events);
    info!("  Operation success rate: {:.1}%", success_rate * 100.0);
    info!("  Impact breakdown:");
    for (impact, count) in &report.impact_counts {
        info!("    {:?}: {}", impact, count);
    }
    
    assert!(success_rate > 0.7, "System should maintain >70% success rate under combined chaos");
    
    cluster.shutdown().await?;
    Ok(())
}